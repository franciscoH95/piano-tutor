//! Interpretaciones grabadas: la red de FR-022.
//!
//! Cada una lleva su cancion, la lista de observaciones con sus instantes, el nivel y **el
//! resultado esperado escrito a mano**. A mano y no volcado de la implementacion: un
//! resultado volcado copia el fallo de quien lo genero, y la prueba pasa a confirmar el
//! error en vez de detectarlo.

use piano_core::capture::{Observacion, TipoEvento};
use piano_core::time::Micros;

/// Un ataque en un instante dado.
#[must_use]
pub fn ataque(us: u64, key: u8, velocity: u8) -> Observacion {
    Observacion { at: Micros(us), key, velocity, kind: TipoEvento::Ataque, channel: 0 }
}

/// Una suelta en un instante dado.
#[must_use]
pub fn suelta(us: u64, key: u8) -> Observacion {
    Observacion { at: Micros(us), key, velocity: 0, kind: TipoEvento::Suelta, channel: 0 }
}

/// Una nota tocada: su ataque y su suelta, ya emparejados.
#[must_use]
pub fn tocada(desde: u64, hasta: u64, key: u8) -> Vec<Observacion> {
    vec![ataque(desde, key, 90), suelta(hasta, key)]
}
