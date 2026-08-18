//! Adaptador de macOS: CoreMIDI.
//!
//! Esta capa no toma **ninguna** decision de dominio. Abre el puerto, recorre el paquete,
//! sella con el reloj de sesion, deja que el analizador filtre a notas y empuja al anillo.
//! (Constitucion v1.1.0, Principio II, excepcion acotada para adaptadores de plataforma.)

use crate::parser::Parser;
use coremidi::{Client, InputPort, PacketList, Source, Sources};
use piano_core::capture::{
    canal, catalogar, reconocer, Dispositivo, ErrorDeEntrada, Receptor, Reconocimiento,
};
use piano_core::clock::Clock;

/// Capacidad del anillo: 4.096 eventos de 16 bytes = 64 KiB.
///
/// Dos ordenes de magnitud por encima de la rafaga humana mas densa (~50 eventos/s):
/// cubre mas de ochenta segundos sin que el consumidor toque nada.
const CAPACIDAD: usize = 4_096;

/// Enumera los teclados disponibles, con nombre legible e identidad estable.
pub fn dispositivos() -> Result<Vec<Dispositivo>, ErrorDeEntrada> {
    let n = Sources::count();
    let mut crudos: Vec<(Option<u64>, String)> = Vec::with_capacity(n);
    for i in 0..n {
        let Some(s) = Source::from_index(i) else { continue };
        crudos.push((s.unique_id().map(u64::from), s.display_name().unwrap_or_default()));
    }
    let refs: Vec<(Option<u64>, &str)> = crudos.iter().map(|(id, n)| (*id, n.as_str())).collect();
    Ok(catalogar(&refs))
}

/// Una captura en curso. Al soltarla se libera el dispositivo.
pub struct Captura {
    // El orden de los campos importa: el puerto debe destruirse antes que el cliente.
    _puerto: InputPort,
    _cliente: Client,
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

    /// Espera hasta `ventana` a que llegue **algo**, y dice si llego.
    ///
    /// Existe por un fallo documentado de Windows (`microsoft/MIDI` #906): tras
    /// reconectar, el puerto se abre con exito y **nunca entrega un solo mensaje**. Sin
    /// esta comprobacion, la aplicacion mostraria una captura activa que en realidad esta
    /// muerta, y el alumno tocaria contra el vacio sin saber por que.
    ///
    /// Devolver `false` **no** significa que el dispositivo este roto: puede que
    /// simplemente no se este tocando. Quien llama decide que contar al usuario; lo que no
    /// puede es dar por hecho que funciona.
    pub fn confirmar_actividad(&mut self, ventana: std::time::Duration) -> bool {
        let limite = std::time::Instant::now() + ventana;
        let mut buffer = Vec::with_capacity(64);
        while std::time::Instant::now() < limite {
            if self.receptor.recoger(&mut buffer) > 0 {
                return true;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        false
    }
}

/// Reabre un dispositivo tras una reconexion, reconociendolo de nuevo por su identidad.
///
/// Es exactamente [`abrir`]: el reconocimiento (identificador del sistema primero, pareja
/// (nombre, posicion) despues) ya cubre el caso de que el sistema haya renumerado los
/// puertos al reconectar. No hace falta nada especial, y esa es justamente la ventaja de
/// no haber usado nunca el indice de puerto como identidad.
pub fn reabrir<C>(dispositivo: &Dispositivo, clock: C) -> Result<Captura, ErrorDeEntrada>
where
    C: Clock + Send + 'static,
{
    abrir(dispositivo, clock)
}

/// Abre un dispositivo y empieza a capturar.
///
/// `clock` es el reloj **de sesion**, el mismo que usa la reproduccion: por eso los
/// instantes de lo tocado y los de lo esperado son directamente comparables, sin
/// conversion ni correccion de por medio (FR-012a).
pub fn abrir<C>(dispositivo: &Dispositivo, clock: C) -> Result<Captura, ErrorDeEntrada>
where
    C: Clock + Send + 'static,
{
    let disponibles = dispositivos()?;
    if disponibles.is_empty() {
        return Err(ErrorDeEntrada::SinDispositivos);
    }
    let indice = match reconocer(dispositivo, &disponibles) {
        Reconocimiento::Encontrado(i) => i,
        Reconocimiento::PedirAlUsuario => {
            return Err(ErrorDeEntrada::NoSePudoAbrir { nombre: dispositivo.nombre.clone() })
        }
    };
    // Resolver la fuente por su IDENTIFICADOR, no por el indice de enumeracion. El indice
    // es valido solo en el instante en que se enumero: si otro aparato aparece o
    // desaparece entre medias, apunta a un dispositivo distinto. Es exactamente el fallo
    // que la identidad estable existe para evitar, y usarlo aqui lo reintroducia por la
    // puerta de atras.
    let elegido = disponibles
        .get(indice)
        .ok_or_else(|| ErrorDeEntrada::NoSePudoAbrir { nombre: dispositivo.nombre.clone() })?;
    let fuente = match elegido.id_sistema {
        Some(id) => u32::try_from(id.0).ok().and_then(Source::from_unique_id),
        None => Source::from_index(indice),
    }
    .ok_or_else(|| ErrorDeEntrada::NoSePudoAbrir { nombre: dispositivo.nombre.clone() })?;

    let cliente = Client::new("Piano Tutor")
        .map_err(|_| ErrorDeEntrada::NoSePudoAbrir { nombre: dispositivo.nombre.clone() })?;

    let (mut emisor, receptor) = canal(CAPACIDAD);
    let mut parser = Parser::nuevo();

    let puerto = cliente
        .input_port("Piano Tutor entrada", move |paquetes: &PacketList| {
            for paquete in paquetes.iter() {
                // El reloj se lee UNA vez por paquete, no una por mensaje: asi un acorde
                // que llega junto recibe un instante unico, por construccion.
                let at = clock.now();
                parser.consumir(at, paquete.data(), |o| emisor.emitir(o));
            }
        })
        .map_err(|_| ErrorDeEntrada::NoSePudoAbrir { nombre: dispositivo.nombre.clone() })?;

    puerto.connect_source(&fuente).map_err(|_| ErrorDeEntrada::EnUsoPorOtraAplicacion {
        nombre: dispositivo.nombre.clone(),
    })?;

    Ok(Captura { _puerto: puerto, _cliente: cliente, receptor })
}
