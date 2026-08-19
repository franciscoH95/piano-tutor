//! T005: constructor de Standard MIDI Files sinteticos para las pruebas.
//!
//! Los fixtures se construyen como bytes en codigo, nunca como archivos `.mid` en el
//! repositorio. Dos razones: la Constitucion prohibe distribuir obras de terceros, y un
//! fixture binario es imposible de revisar en un pull request. Aqui la intencion de cada
//! prueba esta a la vista.


#![allow(dead_code)] // cada binario de prueba usa solo una parte del constructor

pub mod interpretaciones;

/// Codifica un entero como *variable-length quantity*, el formato de delta del estandar MIDI.
fn vlq(mut value: u64) -> Vec<u8> {
    let mut out = vec![(value & 0x7F) as u8];
    value >>= 7;
    while value > 0 {
        out.push(((value & 0x7F) as u8) | 0x80);
        value >>= 7;
    }
    out.reverse();
    out
}

/// Constructor de una pista. Los eventos se declaran en tiempo **absoluto**; el paso a
/// deltas lo hace `build`.
pub struct TrackBuilder {
    /// `(tick_absoluto, orden_de_declaracion, bytes_del_evento)`
    events: Vec<(u64, u32, Vec<u8>)>,
    seq: u32,
}

impl TrackBuilder {
    fn new() -> Self {
        Self { events: Vec::new(), seq: 0 }
    }

    fn push(mut self, tick: u64, bytes: Vec<u8>) -> Self {
        self.events.push((tick, self.seq, bytes));
        self.seq += 1;
        self
    }

    /// Evento crudo en un tick dado. Escotilla para construir casos sucios byte a byte.
    pub fn raw(self, tick: u64, bytes: &[u8]) -> Self {
        self.push(tick, bytes.to_vec())
    }

    /// Cambio de tempo: meta `Set Tempo` con microsegundos por negra.
    pub fn tempo(self, tick: u64, us_per_qn: u32) -> Self {
        let b = us_per_qn.to_be_bytes();
        self.push(tick, vec![0xFF, 0x51, 0x03, b[1], b[2], b[3]])
    }

    /// Note-on explicito.
    pub fn note_on(self, tick: u64, key: u8, vel: u8) -> Self {
        self.push(tick, vec![0x90, key, vel])
    }

    /// Note-off explicito (status `0x80`, velocity de release cero).
    pub fn note_off(self, tick: u64, key: u8) -> Self {
        self.push(tick, vec![0x80, key, 0])
    }

    /// Note-on con velocity cero: el estandar lo define como equivalente a note-off.
    pub fn note_on_vel0(self, tick: u64, key: u8) -> Self {
        self.push(tick, vec![0x90, key, 0])
    }

    /// Una nota completa: ataque en `tick`, cierre en `tick + dur`.
    pub fn note(self, tick: u64, key: u8, vel: u8, dur: u64) -> Self {
        self.note_on(tick, key, vel).note_off(tick + dur, key)
    }

    /// Varias notas simultaneas. Los ataques salen en el orden dado, todos en `tick`.
    pub fn chord(mut self, tick: u64, keys: &[u8], vel: u8, dur: u64) -> Self {
        for &k in keys {
            self = self.note_on(tick, k, vel);
        }
        for &k in keys {
            self = self.note_off(tick + dur, k);
        }
        self
    }

    /// Serializa la pista a un chunk `MTrk` completo, con su meta `End of Track`.
    fn build(mut self) -> Vec<u8> {
        // Orden estable por (tick, orden de declaracion): el fixture es determinista y el
        // autor de la prueba controla el orden dentro de un mismo tick.
        self.events.sort_by_key(|(tick, seq, _)| (*tick, *seq));

        let mut body = Vec::new();
        let mut last_tick = 0u64;
        for (tick, _, bytes) in &self.events {
            body.extend_from_slice(&vlq(tick - last_tick));
            body.extend_from_slice(bytes);
            last_tick = *tick;
        }
        body.extend_from_slice(&[0x00, 0xFF, 0x2F, 0x00]); // End of Track

        let mut chunk = Vec::with_capacity(body.len() + 8);
        chunk.extend_from_slice(b"MTrk");
        chunk.extend_from_slice(&(body.len() as u32).to_be_bytes());
        chunk.extend_from_slice(&body);
        chunk
    }
}

/// Constructor de un Standard MIDI File completo, siempre de formato 1 con timing PPQ.
pub struct SmfBuilder {
    ppq: u16,
    format: u16,
    tracks: Vec<Vec<u8>>,
}

impl SmfBuilder {
    pub fn new(ppq: u16) -> Self {
        Self { ppq, format: 1, tracks: Vec::new() }
    }

    /// Fuerza un formato distinto de 1. Para probar el rechazo del formato 2.
    pub fn format(mut self, format: u16) -> Self {
        self.format = format;
        self
    }

    pub fn track(mut self, f: impl FnOnce(TrackBuilder) -> TrackBuilder) -> Self {
        self.tracks.push(f(TrackBuilder::new()).build());
        self
    }

    pub fn build(self) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(b"MThd");
        out.extend_from_slice(&6u32.to_be_bytes());
        out.extend_from_slice(&self.format.to_be_bytes());
        out.extend_from_slice(&(self.tracks.len() as u16).to_be_bytes());
        out.extend_from_slice(&self.ppq.to_be_bytes());
        for t in &self.tracks {
            out.extend_from_slice(t);
        }
        out
    }

    /// Construye con un campo `division` arbitrario, saltandose el PPQ. Para probar el
    /// rechazo de timing SMPTE (bit 15 a 1) y de `division == 0`.
    pub fn build_with_division(self, division: u16) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(b"MThd");
        out.extend_from_slice(&6u32.to_be_bytes());
        out.extend_from_slice(&self.format.to_be_bytes());
        out.extend_from_slice(&(self.tracks.len() as u16).to_be_bytes());
        out.extend_from_slice(&division.to_be_bytes());
        for t in &self.tracks {
            out.extend_from_slice(t);
        }
        out
    }
}
