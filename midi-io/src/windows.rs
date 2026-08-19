//! Captura MIDI de entrada en Windows, mediante **WinRT** (`Windows.Devices.Midi`).
//!
//! # Por que WinRT y no WinMM
//!
//! `research.md` de la feature 002 decidio WinMM. **Esta implementacion lo enmienda**, y no
//! en silencio: la enmienda esta escrita en ese mismo documento con su motivo.
//!
//! El motivo es una restriccion que no se comprobo entonces: **toda** la API WinMM del
//! crate `windows` es `pub unsafe fn`, y el callback exige un `unsafe extern "system" fn`
//! que recupera su estado desreferenciando un puntero crudo. Este crate tiene
//! `#![forbid(unsafe_code)]`, y `forbid` **no se levanta con un `allow` local**: es la
//! diferencia con `deny`. Usar WinMM obligaria a retirar esa garantia de todo el crate,
//! puesta ahi deliberadamente despues de descartar `midir` por un fallo de memoria.
//!
//! WinRT no la necesita: enumeracion, apertura, callback y cierre se escriben sin una sola
//! linea insegura. Verificado compilando contra `x86_64-pc-windows-msvc` con los cuatro
//! `deny` de este crate y `-D warnings`.
//!
//! Las dos razones que la decision original dio para descartar WinRT tampoco sobreviven:
//! una era que `midir` avisa de que su backend WinRT esta peor probado —no usamos midir—, y
//! la otra unos problemas de enumeracion documentados en `microsoft/MIDI` que resultan ser
//! de la ruta **WinMM**, es decir del rival.
//!
//! # Lo que esto cuesta, y no esta medido
//!
//! WinRT entrega cada mensaje como un **objeto COM**: hay un `AddRef`/`Release` y un
//! `QueryInterface` por nota, dentro del callback. CoreMIDI entrega un slice y no asigna
//! nada. Eso cae en la ruta critica del Principio IV y **no hay ninguna medicion todavia**:
//! es la primera cifra que hay que tomar en una maquina Windows real. Si resultara
//! inaceptable, la alternativa es WinMM con `unsafe` acotado, y esa conversacion habria que
//! volver a tenerla.
//!
//! # Lo que WinRT se lleva por delante
//!
//! `parser.rs` **no se usa en esta plataforma**: WinRT ya entrega el mensaje analizado. Ese
//! archivo existe porque `midir` rebanaba un paquete truncado y abortaba el proceso; aqui
//! ese riesgo lo asume el sistema operativo. Deja de ser cierto, por tanto, que todo el
//! analisis del proyecto viva bajo `deny(clippy::indexing_slicing)`.

use piano_core::capture::{
    canal, catalogar, reconocer, Dispositivo, ErrorDeEntrada, Receptor, Reconocimiento,
};
use piano_core::capture::{Observacion, TipoEvento};
use piano_core::clock::Clock;
use piano_core::time::Micros;
use std::cell::RefCell;
use std::sync::mpsc;
use std::time::Duration;
use windows::core::{Interface, Ref, HSTRING};
use windows::Devices::Enumeration::DeviceInformation;
use windows::Devices::Midi::{
    MidiInPort, MidiMessageReceivedEventArgs, MidiMessageType, MidiNoteOffMessage,
    MidiNoteOnMessage,
};
use windows::Foundation::TypedEventHandler;

const CAPACIDAD: usize = 4_096;
/// Espera maxima por una operacion asincrona de WinRT.
const ESPERA_WINRT: Duration = Duration::from_secs(5);

/// FNV-1a de 64 bits sobre las unidades UTF-16 de la ruta de interfaz.
/// Escrito a mano a proposito: `DefaultHasher` no promete estabilidad entre versiones
/// de Rust y este identificador se PERSISTE en disco.
fn huella(id: &HSTRING) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for u in id.iter() {
        for b in u.to_le_bytes() {
            h ^= u64::from(b);
            h = h.wrapping_mul(0x1000_0000_01b3);
        }
    }
    h
}

/// Espera acotada por una operacion asincrona, SIN `join()`.
///
/// `IAsyncOperation::join()` termina en `Waiter::drop` -> `WaitForSingleObject(INFINITE)`
/// sin bombeo de mensajes: si la operacion no completa nunca, el hilo se cuelga para
/// siempre. Con `when` + `recv_timeout` el peor caso es un error, no un cuelgue.
fn esperar<T, R>(registrar: R) -> Result<T, ErrorDeEntrada>
where
    T: Send + 'static,
    R: FnOnce(Box<dyn FnOnce(windows::core::Result<T>) + Send + 'static>) -> windows::core::Result<()>,
{
    let (tx, rx) = mpsc::sync_channel(1);
    let cb: Box<dyn FnOnce(windows::core::Result<T>) + Send + 'static> = Box::new(move |r| {
        let _ = tx.send(r);
    });
    registrar(cb).map_err(|_| ErrorDeEntrada::SinDispositivos)?;
    match rx.recv_timeout(ESPERA_WINRT) {
        Ok(Ok(v)) => Ok(v),
        Ok(Err(_)) | Err(_) => Err(ErrorDeEntrada::SinDispositivos),
    }
}

/// Una instantanea de la enumeracion: ruta de interfaz + nombre legible, en orden.
fn enumerar() -> Result<Vec<(HSTRING, String)>, ErrorDeEntrada> {
    let selector =
        MidiInPort::GetDeviceSelector().map_err(|_| ErrorDeEntrada::SinDispositivos)?;
    let op = DeviceInformation::FindAllAsyncAqsFilter(&selector)
        .map_err(|_| ErrorDeEntrada::SinDispositivos)?;
    let coleccion = esperar(|cb| op.when(cb))?;
    let n = usize::try_from(coleccion.Size().unwrap_or(0)).unwrap_or(0);
    let mut salida = Vec::with_capacity(n);
    for info in &coleccion {
        let (Ok(id), Ok(nombre)) = (info.Id(), info.Name()) else { continue };
        salida.push((id, nombre.to_string_lossy()));
    }
    Ok(salida)
}

/// Enumera los teclados disponibles, con nombre legible e identidad estable.
pub fn dispositivos() -> Result<Vec<Dispositivo>, ErrorDeEntrada> {
    let crudos = enumerar()?;
    let refs: Vec<(Option<u64>, &str)> =
        crudos.iter().map(|(id, n)| (Some(huella(id)), n.as_str())).collect();
    Ok(catalogar(&refs))
}

/// Traduce un `HRESULT` al error que el usuario va a leer.
pub fn traducir(codigo: i32, nombre: &str) -> ErrorDeEntrada {
    match codigo {
        // E_ACCESSDENIED
        -2147024891 => ErrorDeEntrada::EnUsoPorOtraAplicacion { nombre: nombre.to_string() },
        // E_INVALIDARG / ERROR_NOT_FOUND (HRESULT_FROM_WIN32(1168))
        -2147024809 | -2147023728 => {
            ErrorDeEntrada::DesaparecioAlAbrir { nombre: nombre.to_string() }
        }
        _ => ErrorDeEntrada::NoSePudoAbrir { nombre: nombre.to_string() },
    }
}

/// Una captura en curso. Al soltarla se libera el dispositivo.
pub struct Captura {
    puerto: MidiInPort,
    testigo: i64,
    receptor: Receptor,
}

impl Captura {
    /// El extremo de lectura. Implementa `FuenteDeEventos`.
    pub fn receptor(&mut self) -> &mut Receptor {
        &mut self.receptor
    }

    /// Libera el dispositivo para otras aplicaciones (FR-006).
    pub fn cerrar(self) {
        drop(self);
    }

    /// Espera hasta `ventana` a que llegue algo, y dice si llego.
    pub fn confirmar_actividad(&mut self, ventana: Duration) -> bool {
        let limite = std::time::Instant::now() + ventana;
        let mut buffer = Vec::with_capacity(64);
        while std::time::Instant::now() < limite {
            if piano_core::capture::FuenteDeEventos::recoger(&mut self.receptor, &mut buffer) > 0 {
                return true;
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        false
    }
}

impl Drop for Captura {
    fn drop(&mut self) {
        let _ = self.puerto.RemoveMessageReceived(self.testigo);
        let _ = self.puerto.Close();
    }
}

fn empujar(
    em: &mut piano_core::capture::Emisor,
    at: Micros,
    canal_n: u8,
    nota: u8,
    vel: u8,
    on: bool,
) {
    let kind = if on && vel > 0 { TipoEvento::Ataque } else { TipoEvento::Suelta };
    let velocity = if kind == TipoEvento::Ataque { vel } else { 0 };
    em.emitir(Observacion { at, key: nota, velocity, kind, channel: canal_n });
}

/// Abre un dispositivo y empieza a capturar.
///
/// **Debe llamarse desde un hilo trabajador**, nunca desde el hilo principal de Tauri:
/// espera a una operacion asincrona de WinRT.
pub fn abrir<C>(dispositivo: &Dispositivo, clock: C) -> Result<Captura, ErrorDeEntrada>
where
    C: Clock + Send + 'static,
{
    // UNA sola instantanea: el indice que devuelve `reconocer` solo es valido dentro de
    // ella, y la ruta de interfaz se saca de la MISMA lista.
    let crudos = enumerar()?;
    if crudos.is_empty() {
        return Err(ErrorDeEntrada::SinDispositivos);
    }
    let refs: Vec<(Option<u64>, &str)> =
        crudos.iter().map(|(id, n)| (Some(huella(id)), n.as_str())).collect();
    let disponibles = catalogar(&refs);
    let indice = match reconocer(dispositivo, &disponibles) {
        Reconocimiento::Encontrado(i) => i,
        Reconocimiento::PedirAlUsuario => {
            return Err(ErrorDeEntrada::NoSePudoAbrir { nombre: dispositivo.nombre.clone() })
        }
    };
    let (id, _) = crudos
        .get(indice)
        .ok_or_else(|| ErrorDeEntrada::NoSePudoAbrir { nombre: dispositivo.nombre.clone() })?;

    let op = MidiInPort::FromIdAsync(id)
        .map_err(|e| traducir(e.code().0, &dispositivo.nombre))?;
    let puerto = esperar(|cb| op.when(cb))
        .map_err(|_| ErrorDeEntrada::NoSePudoAbrir { nombre: dispositivo.nombre.clone() })?;

    let (emisor, receptor) = canal(CAPACIDAD);
    let estado = RefCell::new(emisor);
    let testigo = puerto
        .MessageReceived(&TypedEventHandler::new(
            move |_e: Ref<'_, MidiInPort>, args: Ref<'_, MidiMessageReceivedEventArgs>| {
                // FR-012a: PRIMERA linea, reloj de sesion.
                let at = clock.now();
                let Some(args) = args.as_ref() else { return Ok(()) };
                let m = args.Message()?;
                let Ok(mut em) = estado.try_borrow_mut() else { return Ok(()) };
                match m.Type()? {
                    MidiMessageType::NoteOn => {
                        let n: MidiNoteOnMessage = m.cast()?;
                        empujar(&mut em, at, n.Channel()?, n.Note()?, n.Velocity()?, true);
                    }
                    MidiMessageType::NoteOff => {
                        let n: MidiNoteOffMessage = m.cast()?;
                        empujar(&mut em, at, n.Channel()?, n.Note()?, n.Velocity()?, false);
                    }
                    _ => {}
                }
                Ok(())
            },
        ))
        .map_err(|e| traducir(e.code().0, &dispositivo.nombre))?;

    Ok(Captura { puerto, testigo, receptor })
}

/// Reabre un dispositivo tras una reconexion.
pub fn reabrir<C>(dispositivo: &Dispositivo, clock: C) -> Result<Captura, ErrorDeEntrada>
where
    C: Clock + Send + 'static,
{
    abrir(dispositivo, clock)
}
