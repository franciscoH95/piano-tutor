//! Practicar una cancion: que se ve, donde estamos y si lo tocado coincide.
//!
//! Aqui vive **todo lo que decide**. La capa que pinta recibe el resultado y no toma
//! ninguna decision propia (Constitucion, Principio III, y la excepcion acotada del
//! Principio II en la v1.1.0).

mod vista;

pub use vista::{vista, EstadoNota, NotaVisible, Vista};
