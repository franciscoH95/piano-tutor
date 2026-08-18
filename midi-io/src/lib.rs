//! Adaptador de entrada MIDI: la unica capa de Piano Tutor que habla con el sistema
//! operativo.
//!
//! Aqui no se toma **ninguna** decision de dominio. Se abre un puerto, se recorre el
//! paquete, se filtra a notas, se sella con el reloj de sesion y se empuja al anillo.
//! Nada mas. Si algo de este crate empieza a necesitar logica, esa logica pertenece a
//! `piano-core`, donde si esta cubierta por pruebas (Constitucion v1.1.0, Principio II,
//! excepcion acotada para adaptadores de plataforma).
//!
//! `deny(clippy::indexing_slicing)` no es decorativo: la biblioteca obvia para esto,
//! `midir`, quedo descartada porque rebana un slice con una longitud deducida del byte de
//! estado sin comprobar el paquete real, y ante un paquete truncado **aborta el proceso**.
//! Bajo este lint esa clase de fallo es imposible de escribir.

#![forbid(unsafe_code)]
#![deny(clippy::float_arithmetic)]
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![deny(clippy::indexing_slicing)]
#![warn(missing_docs)]

pub mod parser;
pub mod prioridad;

#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
pub mod vigia;
#[cfg(target_os = "macos")]
pub use macos::{abrir, dispositivos, reabrir, traducir, Captura};
