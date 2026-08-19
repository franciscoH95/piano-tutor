//! Juzgar como toco el alumno.
//!
//! La feature 003 dejo la linea temporal, el cursor con su tempo de practica y el reparto de
//! manos. Aqui se anade **la medida y el veredicto**, y no se vuelve a construir la base.
//!
//! # Lo que gobierna este modulo
//!
//! - **Todas las tolerancias viven en `tolerancias.rs`**, y en ningun otro sitio. El
//!   Principio I lo exige textualmente: nunca constantes dispersas.
//! - **Dos ventanas, no una**: la de emparejamiento es igual en los tres niveles y la de
//!   ataque cambia. Asi el nivel decide el veredicto de una nota *ya emparejada* y nunca
//!   *que* se empareja con que, lo que hace que SC-006 se cumpla por aritmetica.
//! - **El instante esperado se sella al cruzar y no se recalcula nunca** (FR-004).

mod emparejar;
mod evaluador;
mod estadistica;
mod pulsacion;
mod resultado;
mod tolerancias;

pub use emparejar::instante_de;
pub use evaluador::Evaluador;
pub use resultado::{
    comparar, sistematico, Medida, Recuento, Resultado, Sistematico, Veredicto,
};
pub use pulsacion::{Pulsacion, Pulsaciones};
pub use estadistica::{cuartiles, mediana};
pub use tolerancias::{Nivel, Tolerancias};

use crate::practica::Mano;
use crate::timeline::{CANAL_PERCUSION, PIANO_MAX, PIANO_MIN};

/// Si el alumno puede y debe tocar esa nota.
///
/// **Un solo criterio, consumido por las puertas y por el evaluador.** Si vive en dos
/// sitios vuelven a divergir, y ya ocurrio: `ProgramaDePuertas` llevaba escrito que
/// filtraba la percusion y no lo hacia, asi que la practica se atascaba desde el primer
/// compas esperando una caja.
///
/// Tres motivos para dejar una nota fuera, y ninguno es culpa del alumno:
///
/// - **percusion**: no se toca con las manos en el teclado. Mirar la altura no basta, una
///   caja esta en la tecla 38, dentro del piano;
/// - **fuera de las 88 teclas**: no las tiene (FR-014);
/// - **la otra mano**: no se le esta pidiendo.
#[must_use]
pub fn es_evaluable(canal: u8, key: u8, mano: Mano, practicada: Option<Mano>) -> bool {
    canal != CANAL_PERCUSION
        && (PIANO_MIN..=PIANO_MAX).contains(&key)
        && practicada.is_none_or(|m| m == mano)
}
