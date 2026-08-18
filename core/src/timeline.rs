//! De eventos sueltos a notas con principio y final.
//!
//! El emparejamiento de note-on con note-off es la fuente clasica de errores al leer
//! MIDI, porque los archivos reales estan llenos de casos sucios. Aqui la politica es
//! explicita y esta cubierta por pruebas, en dos fases separadas y en este orden:
//!
//! 1. **Emparejamiento FIFO** por voz: el note-off mas antiguo cierra el note-on mas
//!    antiguo de la misma voz. Es lo que hacen los secuenciadores reales.
//! 2. **Acortado del solapamiento residual**: si tras emparejar dos notas de la misma
//!    voz siguen pisandose, la primera se acorta hasta el ataque de la segunda y se
//!    marca. Una tecla fisica no puede sonar dos veces a la vez.

use crate::error::{LoadError, LoadReport};
use crate::midi::loader::{ParsedSmf, RawKind};
use crate::tempo::TempoMap;
use crate::time::{Micros, Ticks};
use std::collections::{BTreeMap, VecDeque};

/// Rango de las 88 teclas de un piano, en numeros de nota MIDI.
const PIANO_MIN: u8 = 21;
const PIANO_MAX: u8 = 108;

/// Indice del canal de percusion (el canal 10 del estandar, contado desde cero).
const CANAL_PERCUSION: u8 = 9;

/// Como llego a su fin una nota.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Closure {
    /// Se cerro con su propio note-off.
    Normal,
    /// No tenia note-off: se cerro al final de su pista.
    ///
    /// No se descarta ni se le da una duracion magica. Cerrarla al final de **su** pista
    /// mantiene la nota tocable y deja constancia de que el archivo venia mal.
    HangingClosedAtTrackEnd,
}

/// Una nota que el alumno debe tocar.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScheduledNote {
    /// Tiempo musical del ataque.
    pub onset_tick: Ticks,
    /// Tiempo musical del final. Siempre mayor que `onset_tick`.
    pub end_tick: Ticks,
    /// Tiempo real del ataque.
    pub onset_us: Micros,
    /// Tiempo real del final.
    pub end_us: Micros,
    /// Altura MIDI.
    pub key: u8,
    /// Intensidad del ataque. La de release se descarta.
    pub velocity: u8,
    /// Pista de origen. Conserva la identidad de voz (FR-006).
    pub track: u16,
    /// Canal MIDI de origen.
    pub channel: u8,
    /// Como se cerro.
    pub closure: Closure,
    /// `true` si se acorto por solaparse con otra nota de la misma voz y altura.
    pub truncated: bool,
}

impl ScheduledNote {
    /// Duracion en tiempo musical.
    #[must_use]
    pub const fn duration_ticks(&self) -> Ticks {
        Ticks(self.end_tick.0 - self.onset_tick.0)
    }

    /// `true` si la nota cae dentro de las 88 teclas de un piano.
    #[must_use]
    pub const fn is_on_88_keys(&self) -> bool {
        self.key >= PIANO_MIN && self.key <= PIANO_MAX
    }
}

/// Identidad de una nota abierta.
///
/// La tripleta importa: la misma altura en pistas o canales distintos son notas
/// independientes y no deben cerrarse entre si.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
struct VoiceKey {
    track: u16,
    channel: u8,
    key: u8,
}

/// Una nota abierta a la espera de su cierre.
#[derive(Clone, Copy)]
struct OpenNote {
    onset_tick: u64,
    velocity: u8,
    seq: u32,
}

/// Construye la linea temporal ordenada.
pub(crate) fn build(
    parsed: &ParsedSmf,
    tempo_map: &TempoMap,
    report: &mut LoadReport,
) -> Result<Vec<ScheduledNote>, LoadError> {
    let mut abiertas: BTreeMap<VoiceKey, VecDeque<OpenNote>> = BTreeMap::new();
    // `seq` es un contador monotono en el orden canonico de fusion. Es lo que garantiza
    // que la clave de ordenacion final no tenga empates.
    let mut seq: u32 = 0;
    let mut notas: Vec<(u32, ScheduledNote)> = Vec::new();

    for ev in &parsed.events {
        let voz = |key: u8| VoiceKey { track: ev.track, channel: ev.channel, key };
        match ev.kind {
            RawKind::Tempo { .. } => {}
            RawKind::NoteOn { key, velocity } => {
                abiertas.entry(voz(key)).or_default().push_back(OpenNote {
                    onset_tick: ev.tick,
                    velocity,
                    seq,
                });
                seq = seq.saturating_add(1);
            }
            RawKind::NoteOff { key } => {
                match abiertas.get_mut(&voz(key)).and_then(VecDeque::pop_front) {
                    Some(abierta) => {
                        // Una nota nunca dura cero: dejaria un ataque sin sonido y
                        // rompe la invariante `end > onset`.
                        let end = ev.tick.max(abierta.onset_tick.saturating_add(1));
                        notas.push((abierta.seq, nota(&abierta, ev.track, ev.channel, key, end, Closure::Normal)));
                    }
                    None => report.orphan_note_offs = report.orphan_note_offs.saturating_add(1),
                }
            }
        }
    }

    // Notas colgadas: se cierran al final de SU pista, no de la pieza.
    for (voz, cola) in &abiertas {
        for abierta in cola {
            let fin_pista = parsed
                .track_end_ticks
                .get(usize::from(voz.track))
                .copied()
                .unwrap_or(abierta.onset_tick);
            let end = fin_pista.max(abierta.onset_tick.saturating_add(1));
            notas.push((
                abierta.seq,
                nota(abierta, voz.track, voz.channel, voz.key, end, Closure::HangingClosedAtTrackEnd),
            ));
            report.hanging_notes = report.hanging_notes.saturating_add(1);
        }
    }

    // --- fase 2: acortado del solapamiento residual de la misma voz ---
    notas.sort_by_key(|(s, n)| (n.track, n.channel, n.key, n.onset_tick, *s));
    let mut recortes: Vec<(usize, u64)> = Vec::new();
    for i in 0..notas.len() {
        let (Some((_, a)), Some((_, b))) = (notas.get(i), notas.get(i + 1)) else {
            continue;
        };
        if (a.track, a.channel, a.key) != (b.track, b.channel, b.key) {
            continue;
        }
        if a.onset_tick == b.onset_tick {
            // Dos ataques identicos en el mismo instante: no se acorta a duracion cero,
            // se deja constancia y se sigue.
            report.duplicate_note_ons = report.duplicate_note_ons.saturating_add(1);
        } else if a.onset_tick < b.onset_tick && b.onset_tick < a.end_tick {
            recortes.push((i, b.onset_tick.0));
        }
    }
    for (i, nuevo_fin) in recortes {
        if let Some((_, n)) = notas.get_mut(i) {
            n.end_tick = Ticks(nuevo_fin);
            n.truncated = true;
            report.truncated_overlaps = report.truncated_overlaps.saturating_add(1);
        }
    }

    // --- tiempos reales y contadores ---
    for (_, n) in &mut notas {
        n.onset_us = tempo_map.tick_to_us(n.onset_tick);
        n.end_us = tempo_map.tick_to_us(n.end_tick);
        if n.channel == CANAL_PERCUSION {
            report.percussion_notes = report.percussion_notes.saturating_add(1);
        }
        if !n.is_on_88_keys() {
            report.notes_out_of_88_range = report.notes_out_of_88_range.saturating_add(1);
        }
    }

    // --- orden total y estable ---
    // La clave incluye `seq`, que es unico, asi que no hay empates posibles: el resultado
    // es identico entre ejecuciones y entre plataformas, sin depender de la estabilidad
    // del algoritmo de ordenacion.
    notas.sort_unstable_by_key(|(s, n)| (n.onset_tick, n.key, n.track, n.channel, *s));

    debug_assert!(notas.iter().all(|(_, n)| n.end_tick > n.onset_tick));
    debug_assert!(notas
        .windows(2)
        .all(|w| matches!(w, [(_, a), (_, b)] if a.onset_tick <= b.onset_tick)));

    Ok(notas.into_iter().map(|(_, n)| n).collect())
}

fn nota(abierta: &OpenNote, track: u16, channel: u8, key: u8, end: u64, closure: Closure) -> ScheduledNote {
    ScheduledNote {
        onset_tick: Ticks(abierta.onset_tick),
        end_tick: Ticks(end),
        onset_us: Micros::ZERO, // se rellena tras el acortado
        end_us: Micros::ZERO,
        key,
        velocity: abierta.velocity,
        track,
        channel,
        closure,
        truncated: false,
    }
}
