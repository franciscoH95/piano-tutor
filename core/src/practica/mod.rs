//! Practicar una cancion: que se ve, donde estamos y si lo tocado coincide.
//!
//! Aqui vive **todo lo que decide**. La capa que pinta recibe el resultado y no toma
//! ninguna decision propia (Constitucion, Principio III, y la excepcion acotada del
//! Principio II en la v1.1.0).

mod cursor;
mod manos;
mod preparacion;
mod puertas;
mod sonando;
mod nombres;
mod vista;

pub use manos::{repartir, Mano, Reparto, RepartoDeManos};
pub use cursor::{posicion_en, Ancla, Avance, Cursor, Paso, Velocidad};
pub use preparacion::{Comparacion, NotaDetallada, Preparacion};
pub use nombres::{Alteracion, Base, MapaDeArmaduras, NombreDeNota};
pub use puertas::{ProgramaDePuertas, Puerta};
pub use sonando::{ConjuntoSonando, MascaraTeclas};
pub use vista::{vista, EstadoNota, NotaVisible, Vista};
