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
pub mod error;
mod midi;
pub mod tempo;
pub mod time;
pub mod timeline;

use crate::error::{LoadError, LoadReport};
use crate::midi::loader::RawKind;
use crate::tempo::TempoMap;
use crate::time::{Micros, Ticks};
use crate::timeline::ScheduledNote;

/// Una cancion lista para practicar.
///
/// Inmutable: todo el trabajo caro (parsear, construir el mapa de tempo, emparejar notas,
/// calcular tiempos reales) ocurre una sola vez, en la carga. Reproducir no vuelve a
/// tocar nada de esto.
#[derive(Clone, Debug)]
pub struct Song {
    tempo_map: TempoMap,
    notes: Box<[ScheduledNote]>,
    report: LoadReport,
}

impl Song {
    /// Las notas, ordenadas por instante de ataque.
    ///
    /// El orden es total y canonico: dentro de un acorde las notas salen de grave a
    /// agudo, y la posicion de cada nota en este corte es su identidad estable.
    #[must_use]
    pub fn notes(&self) -> &[ScheduledNote] {
        &self.notes
    }

    /// La relacion entre tiempo musical y tiempo real de esta cancion.
    #[must_use]
    pub const fn tempo_map(&self) -> &TempoMap {
        &self.tempo_map
    }

    /// Que datos sucios se toleraron al cargar.
    #[must_use]
    pub const fn report(&self) -> &LoadReport {
        &self.report
    }

    /// Tiempo musical del final de la ultima nota. Cero si la cancion no tiene notas.
    #[must_use]
    pub fn duration_ticks(&self) -> Ticks {
        self.notes.iter().map(|n| n.end_tick).max().unwrap_or(Ticks::ZERO)
    }

    /// Tiempo real del final de la ultima nota. Cero si la cancion no tiene notas.
    #[must_use]
    pub fn duration_us(&self) -> Micros {
        self.notes.iter().map(|n| n.end_us).max().unwrap_or(Micros::ZERO)
    }
}

/// Convierte los bytes de un Standard MIDI File en una cancion lista para practicar.
///
/// Acepta formatos 0 y 1 con timing por pulsos. El formato 2 y el timing SMPTE se
/// rechazan con un error tipado, porque sus pistas no comparten eje temporal.
///
/// Recibe **bytes**, no una ruta: quien lee del disco es la capa de aplicacion. Asi el
/// nucleo es incapaz por construccion de tocar el sistema de archivos o la red, y toda
/// la funcionalidad se puede ejercer desde una prueba sin ficheros de apoyo.
///
/// # Garantias
///
/// - **Determinista**: los mismos bytes producen la misma cancion, bit a bit.
/// - **Total**: para cualquier entrada devuelve `Ok` o `Err`. Nunca entra en panico.
///
/// # Errores
///
/// Ver [`LoadError`]. Un fallo estructural aborta la carga; los datos sucios pero
/// tocables se corrigen segun regla documentada y se cuentan en [`Song::report`].
pub fn load_smf(raw: &[u8]) -> Result<Song, LoadError> {
    let parsed = midi::loader::parse(raw)?;
    let mut report = LoadReport::default();

    // El tempo no pertenece a ninguna pista: el estandar lo situa en la pista 0 pero
    // aplica a todas. Se recogen de donde esten y se avisa si aparecen fuera.
    let mut tempos: Vec<(u64, u32)> = Vec::new();
    for ev in &parsed.events {
        if let RawKind::Tempo { us_per_qn } = ev.kind {
            if ev.track != 0 {
                report.tempo_events_outside_track0 =
                    report.tempo_events_outside_track0.saturating_add(1);
            }
            tempos.push((ev.tick, us_per_qn));
        }
    }

    let tempo_map = TempoMap::new(parsed.ppq, &tempos)?;
    let notes = timeline::build(&parsed, &tempo_map, &mut report)?;

    Ok(Song { tempo_map, notes: notes.into_boxed_slice(), report })
}
