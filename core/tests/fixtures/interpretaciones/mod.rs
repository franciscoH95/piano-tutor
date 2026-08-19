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

use piano_core::evaluacion::Nivel;
use piano_core::practica::Mano;

/// Lo que se espera de una interpretación de referencia.
///
/// **Escrito a mano**, nunca volcado de la implementación: un resultado volcado copia el
/// fallo de quien lo generó, y la prueba pasa a confirmar el error en vez de detectarlo.
pub struct Esperado {
    pub acertadas: usize,
    pub fuera_de_tiempo: usize,
    pub omitidas: usize,
    pub de_mas: usize,
    pub dedos_escapados: usize,
    pub fuera_de_alcance: usize,
    pub no_intentadas: usize,
    pub intentadas: usize,
    /// La mediana del desfase, o `None` si no hay desfase sistemático.
    pub desfase_us: Option<i64>,
    pub sin_tocar: bool,
    pub parcial: bool,
}

impl Esperado {
    /// Todo a cero: cada caso rellena solo lo que le importa.
    pub const fn nada() -> Self {
        Self {
            acertadas: 0,
            fuera_de_tiempo: 0,
            omitidas: 0,
            de_mas: 0,
            dedos_escapados: 0,
            fuera_de_alcance: 0,
            no_intentadas: 0,
            intentadas: 0,
            desfase_us: None,
            sin_tocar: false,
            parcial: false,
        }
    }
}

/// Una interpretación de referencia completa.
pub struct Caso {
    pub nombre: &'static str,
    /// Notas de la canción: `(tick, tecla, duración, canal)`. Un tick es un milisegundo.
    pub notas: &'static [(u64, u8, u64, u8)],
    /// De qué mano es cada nota. Vacío = todas de la derecha.
    pub manos: &'static [Mano],
    /// Qué mano se practica.
    pub practicada: Option<Mano>,
    pub nivel: Nivel,
    /// Lo que el alumno tocó: `(instante_us, tecla, duración_us)`.
    pub tocado: &'static [(u64, u8, u64)],
    pub esperado: Esperado,
    /// Por qué existe este caso. Si no se puede decir, el caso no aporta nada.
    pub porque: &'static str,
}
