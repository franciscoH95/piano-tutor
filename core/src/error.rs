//! Fallos de carga y contadores de datos sucios.
//!
//! La distincion entre ambos es normativa y no se difumina:
//!
//! - **Fallo estructural** ([`LoadError`]): la carga falla. El archivo no describe una
//!   cancion que se pueda tocar.
//! - **Dato sucio** ([`LoadReport`]): la carga tiene exito, el problema se corrige segun
//!   una regla documentada y se **cuenta**. Una cancion con notas colgadas se practica
//!   igual; una con tempo cero, no, porque implicaria dividir por cero y entregar una
//!   leccion falsa.

use std::fmt;

/// Un fallo estructural que impide construir la cancion.
///
/// Es `#[non_exhaustive]`: pueden aparecer variantes nuevas sin romper compatibilidad,
/// asi que quien lo consuma necesita un brazo `_`.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LoadError {
    /// El archivo es mas corto que la cabecera de 14 bytes.
    CabeceraTruncada,
    /// Los primeros cuatro bytes no son `MThd`.
    MagicInvalido,
    /// La cabecera declara una longitud distinta de 6.
    CabeceraInvalida {
        /// Longitud declarada.
        len: u32,
    },
    /// El campo de formato no es 0, 1 ni 2.
    FormatoInvalido {
        /// Formato declarado.
        format: u16,
    },
    /// Formato 2: pistas con ejes temporales independientes. Fuera de alcance.
    FormatoNoSoportado {
        /// Formato declarado.
        format: u16,
    },
    /// Timing por fotogramas SMPTE (bit 15 del campo `division`). Fuera de alcance.
    TimingSmpteNoSoportado,
    /// El campo `division` es cero: no hay resolucion temporal que aplicar.
    DivisionCero,
    /// Un chunk de pista termina antes de lo que declara.
    ChunkTruncado {
        /// Indice de la pista.
        track: u16,
    },
    /// Un evento meta declara una longitud que no cuadra con su contenido.
    MetaMalformado {
        /// Indice de la pista.
        track: u16,
        /// Tick absoluto del evento.
        tick: u64,
        /// Byte de tipo del meta.
        tipo: u8,
    },
    /// Un evento de tempo con cero microsegundos por negra.
    InvalidTempo {
        /// Tick absoluto del evento.
        tick: u64,
        /// Valor declarado.
        us_per_qn: u32,
    },
    /// El tick absoluto acumulado supera el techo de `u32::MAX`.
    TickOverflow {
        /// Indice de la pista.
        track: u16,
    },
    /// El cuerpo del archivo no se pudo leer: chunk truncado, evento malformado o
    /// estructura incoherente. La cabecera si era valida.
    CuerpoIlegible,
    /// La pieza dura mas de lo representable (mas de 24 horas).
    DuracionExcesiva {
        /// Duracion calculada, en microsegundos.
        us: u64,
    },
}

impl fmt::Display for LoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CabeceraTruncada => write!(f, "el archivo es mas corto que la cabecera MIDI"),
            Self::MagicInvalido => write!(f, "no es un archivo MIDI: falta la marca MThd"),
            Self::CabeceraInvalida { len } => {
                write!(f, "cabecera invalida: declara {len} bytes en vez de 6")
            }
            Self::FormatoInvalido { format } => write!(f, "formato MIDI desconocido: {format}"),
            Self::FormatoNoSoportado { format } => {
                write!(f, "el formato {format} no esta soportado: sus pistas no comparten eje temporal")
            }
            Self::TimingSmpteNoSoportado => {
                write!(f, "el archivo usa timing SMPTE, que no esta soportado")
            }
            Self::DivisionCero => write!(f, "el archivo declara resolucion temporal cero"),
            Self::ChunkTruncado { track } => write!(f, "la pista {track} termina antes de tiempo"),
            Self::MetaMalformado { track, tick, tipo } => {
                write!(f, "evento meta 0x{tipo:02X} malformado en la pista {track}, tick {tick}")
            }
            Self::InvalidTempo { tick, us_per_qn } => {
                write!(f, "tempo invalido en el tick {tick}: {us_per_qn} microsegundos por negra")
            }
            Self::TickOverflow { track } => {
                write!(f, "la pista {track} acumula mas ticks de los representables")
            }
            Self::CuerpoIlegible => write!(f, "el contenido del archivo MIDI no se pudo leer"),
            Self::DuracionExcesiva { us } => {
                write!(f, "la pieza dura {us} microsegundos, mas del maximo admitido")
            }
        }
    }
}

impl std::error::Error for LoadError {}

/// Datos sucios que se toleraron durante la carga.
///
/// Todos los contadores estan a cero en una cancion limpia. No hay registro en disco ni
/// en consola: son exactamente los valores sobre los que assertan las pruebas de casos
/// limite, lo que convierte cada caso del spec en una asercion concreta.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct LoadReport {
    /// Note-on sin note-off, cerradas al final de su pista.
    pub hanging_notes: u32,
    /// Note-off sin ningun note-on abierto que les corresponda.
    pub orphan_note_offs: u32,
    /// Dos note-on de la misma voz en el mismo tick.
    pub duplicate_note_ons: u32,
    /// Notas acortadas por solapamiento de la misma altura.
    pub truncated_overlaps: u32,
    /// Notas del canal de percusion (indice 9).
    pub percussion_notes: u32,
    /// Notas fuera del rango del piano de 88 teclas (21..=108).
    pub notes_out_of_88_range: u32,
    /// Eventos de tempo hallados fuera de la pista 0. Solo informativo.
    pub tempo_events_outside_track0: u32,
}

impl LoadReport {
    /// `true` si no se toco ni corrigio nada durante la carga.
    #[must_use]
    pub fn is_clean(&self) -> bool {
        *self == Self::default()
    }
}
