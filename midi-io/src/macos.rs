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
    let fuente = Source::from_index(indice)
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
