//! Capturar lo que el alumno toca.
//!
//! Aqui vive **toda** la logica: el transporte entre el hilo del sistema operativo y el
//! consumidor, el emparejamiento de pulsacion con suelta, los cierres y los contadores.
//! Lo que habla con el hardware vive fuera, en `piano-midi-io`, y no toma ninguna
//! decision (Constitucion v1.1.0, Principio II, excepcion acotada).

mod dispositivo;
mod emparejador;
mod sesion;
mod evento;
mod fuente;
mod transporte;

pub use sesion::{EstadoSesion, SesionDeCaptura};
pub use emparejador::{Cierre, Emparejador, InformeDeCaptura, PulsacionCapturada};
pub use dispositivo::{catalogar, reconocer, DeviceId, Dispositivo, ErrorDeEntrada, Reconocimiento};
pub use evento::{EventoCrudo, Observacion, TipoEvento};
pub use fuente::{FuenteDeEventos, FuenteGuionizada};
pub use transporte::{canal, Emisor, Receptor};
