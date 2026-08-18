//! Nucleo de dominio de Piano Tutor.
//!
//! Convierte un Standard MIDI File en una linea temporal de notas programadas y emite
//! avisos (*cues*) anticipados de que nota toca a continuacion.
//!
//! Este crate es **headless**: no depende de Tauri, de la interfaz ni de ningun
//! dispositivo. Se ejerce por completo desde `cargo test`, sin ventana y sin teclado
//! MIDI conectado (Constitucion, Principio III).
//!
//! # Reglas transversales
//!
//! - **Prohibido el punto flotante.** Toda magnitud temporal es entero de 64 bits. Es lo
//!   que hace posible el determinismo bit a bit entre ejecuciones y entre plataformas.
//! - **Sin panicos.** Ninguna funcion publica entra en panico, sea cual sea su entrada.
//! - **Sin efectos.** No se abre ningun archivo ni ningun socket: la carga recibe bytes.

#![forbid(unsafe_code)]
#![deny(clippy::float_arithmetic)]
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![deny(clippy::indexing_slicing)]
#![warn(missing_docs)]

pub mod clock;
pub mod time;
