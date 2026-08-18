//! Bytes de un Standard MIDI File a eventos propios.
//!
//! **Este es el unico archivo del crate autorizado a nombrar tipos de `midi_file`.** La
//! frontera es deliberada: cambiar de parseador debe ser trabajo de un dia, no una
//! reescritura. Si algun otro modulo importa `midi_file`, la frontera se ha roto.
//!
//! La cabecera se valida aqui a mano, byte a byte, **antes** de entregarsela al
//! parseador. Es defensa en profundidad: asi los rechazos de formato 2, de timing SMPTE
//! y de resolucion cero son nuestros y estan cubiertos por pruebas propias, en lugar de
//! depender de lo que haga hoy una dependencia externa.

use crate::error::LoadError;
use midi_file::core::Message;
use midi_file::file::{Event, MetaEvent};
use midi_file::MidiFile;
use std::io::Cursor;

/// Tempo mas rapido que se admite, en microsegundos por negra.
///
/// 1.000 microsegundos por negra son 60.000 pulsaciones por minuto: eso no es una pieza
/// tocable. Ademas es como se manifiesta un tempo de cero, porque el parseador lo satura
/// a 1 en silencio en lugar de rechazarlo.
const MIN_US_PER_QN: u32 = 1_000;

/// Techo de ticks absolutos, heredado de [`crate::tempo`].
const MAX_TICK: u64 = u32::MAX as u64;

/// Un evento ya traducido a vocabulario propio.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RawKind {
    NoteOn { key: u8, velocity: u8 },
    NoteOff { key: u8 },
    Tempo { us_per_qn: u32 },
    /// Armadura de la pieza: `sf` de -7 (siete bemoles) a 7 (siete sostenidos).
    ///
    /// La feature 001 la descartaba porque nada la necesitaba. La 003 la usa para decidir
    /// si una tecla negra se llama sostenido o bemol.
    Armadura { sf: i8 },
}

/// Un evento con su posicion absoluta y su procedencia.
#[derive(Clone, Copy, Debug)]
pub(crate) struct RawEvent {
    pub tick: u64,
    pub track: u16,
    /// Posicion del evento dentro de su pista. Junto a `tick` y `track` forma la clave
    /// total que hace la fusion reproducible.
    pub index: u32,
    pub channel: u8,
    pub kind: RawKind,
}

/// El resultado de leer un archivo.
pub(crate) struct ParsedSmf {
    pub ppq: u32,
    pub events: Vec<RawEvent>,
    /// Tick del `End of Track` de cada pista. Es donde se cierran las notas colgadas.
    pub track_end_ticks: Vec<u64>,
}

fn be_u16(raw: &[u8], at: usize) -> Option<u16> {
    raw.get(at..at + 2)?.try_into().ok().map(u16::from_be_bytes)
}

fn be_u32(raw: &[u8], at: usize) -> Option<u32> {
    raw.get(at..at + 4)?.try_into().ok().map(u32::from_be_bytes)
}

/// Valida la cabecera de 14 bytes y devuelve los pulsos por negra.
fn validar_cabecera(raw: &[u8]) -> Result<u32, LoadError> {
    if raw.len() < 14 {
        return Err(LoadError::CabeceraTruncada);
    }
    if raw.get(..4) != Some(b"MThd".as_slice()) {
        return Err(LoadError::MagicInvalido);
    }
    let len = be_u32(raw, 4).ok_or(LoadError::CabeceraTruncada)?;
    if len != 6 {
        return Err(LoadError::CabeceraInvalida { len });
    }
    let format = be_u16(raw, 8).ok_or(LoadError::CabeceraTruncada)?;
    match format {
        0 | 1 => {}
        2 => return Err(LoadError::FormatoNoSoportado { format }),
        otro => return Err(LoadError::FormatoInvalido { format: otro }),
    }
    let division = be_u16(raw, 12).ok_or(LoadError::CabeceraTruncada)?;
    if division & 0x8000 != 0 {
        // Bit 15 a 1: timing por fotogramas SMPTE.
        return Err(LoadError::TimingSmpteNoSoportado);
    }
    if division == 0 {
        return Err(LoadError::DivisionCero);
    }
    Ok(u32::from(division))
}

/// Lee un Standard MIDI File. Nunca entra en panico, sea cual sea la entrada.
pub(crate) fn parse(raw: &[u8]) -> Result<ParsedSmf, LoadError> {
    let ppq = validar_cabecera(raw)?;

    let mf = MidiFile::read(Cursor::new(raw)).map_err(|_| LoadError::CuerpoIlegible)?;

    let mut events: Vec<RawEvent> = Vec::new();
    let mut track_end_ticks: Vec<u64> = Vec::new();

    for (track_idx, track) in mf.tracks().enumerate() {
        let track_u16 = u16::try_from(track_idx).unwrap_or(u16::MAX);
        let mut tick: u64 = 0;
        let mut ultimo_tick: u64 = 0;
        let mut fin: Option<u64> = None;

        for (index, ev) in track.events().enumerate() {
            tick = tick
                .checked_add(u64::from(ev.delta_time()))
                .ok_or(LoadError::TickOverflow { track: track_u16 })?;
            if tick > MAX_TICK {
                return Err(LoadError::TickOverflow { track: track_u16 });
            }
            ultimo_tick = tick;
            let index = u32::try_from(index).unwrap_or(u32::MAX);

            let kind = match ev.event() {
                Event::Midi(Message::NoteOn(nm)) => {
                    let velocity = nm.velocity().get();
                    let key = nm.note_number().get();
                    // Un note-on con velocity cero es un note-off. Es la regla del
                    // estandar y ningun parseador la aplica por nosotros.
                    if velocity == 0 {
                        Some((nm.channel().get(), RawKind::NoteOff { key }))
                    } else {
                        Some((nm.channel().get(), RawKind::NoteOn { key, velocity }))
                    }
                }
                Event::Midi(Message::NoteOff(nm)) => Some((
                    nm.channel().get(),
                    RawKind::NoteOff { key: nm.note_number().get() },
                )),
                Event::Meta(MetaEvent::SetTempo(us)) => {
                    let us_per_qn = us.get();
                    if us_per_qn < MIN_US_PER_QN {
                        return Err(LoadError::InvalidTempo { tick, us_per_qn });
                    }
                    Some((0, RawKind::Tempo { us_per_qn }))
                }
                Event::Meta(MetaEvent::KeySignature(k)) => {
                    Some((0, RawKind::Armadura { sf: k.accidentals().get() }))
                }
                Event::Meta(MetaEvent::EndOfTrack) => {
                    fin = Some(tick);
                    None
                }
                // Aftertouch, pedal, cambios de instrumento, compas, armadura: se leen y
                // se descartan. No afectan a que hay que tocar ni cuando.
                _ => None,
            };

            if let Some((channel, kind)) = kind {
                events.push(RawEvent { tick, track: track_u16, index, channel, kind });
            }
        }
        track_end_ticks.push(fin.unwrap_or(ultimo_tick));
    }

    // Clave total de fusion: (tick, pista, posicion en la pista). No solo el tick, porque
    // el tick empata constantemente y un empate deja el orden a merced del algoritmo de
    // ordenacion. Esta clave es lo que hace la carga reproducible bit a bit (FR-008).
    events.sort_by_key(|e| (e.tick, e.track, e.index));

    Ok(ParsedSmf { ppq, events, track_end_ticks })
}
