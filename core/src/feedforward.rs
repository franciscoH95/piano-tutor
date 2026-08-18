//! Avisar de cada nota antes de que haya que tocarla.
//!
//! # Por que todo se calcula en la carga
//!
//! Reproducir una cancion es la ruta que un dia estara entre la tecla del alumno y lo
//! que ve en pantalla, con un presupuesto de 30 milisegundos. Por eso aqui no se calcula
//! nada: los avisos se materializan ordenados durante la carga, y reproducir es avanzar
//! un indice comparando enteros. Sin divisiones, sin asignaciones, sin acceso a disco.
//!
//! # Por que la antelacion se mide en pulsos
//!
//! Un margen fijo en milisegundos se queda corto al practicar despacio e inunda la
//! pantalla al practicar rapido. Medido en pulsos, el margen real se estira y encoge con
//! el tempo por si solo, que es como piensa quien toca: "dos tiempos antes".

use crate::time::{Micros, Ticks};
use crate::Song;
use std::fmt;
use std::sync::Arc;

/// El aviso anticipado de una nota.
///
/// Ocupa 32 bytes exactos y es `Copy`. No repite la altura ni la pista: se obtienen de
/// la cancion a traves de [`Cue::note_index`]. Mantenerlo pequeno importa porque la
/// reproduccion devuelve cortes del array de avisos.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Cue {
    /// Instante en que se anuncia la nota.
    pub cue_at: Micros,
    /// Instante en que hay que pulsar la tecla.
    pub onset_at: Micros,
    /// Tiempo musical del aviso. Desempata el orden y ayuda a depurar.
    pub cue_tick: u64,
    /// Posicion de la nota en [`Song::notes`].
    pub note_index: u32,
}

impl Cue {
    /// Cuanto falta para pulsar la tecla.
    ///
    /// Satura en cero si `now` ya paso el ataque, que es lo que ocurre cuando un salto
    /// grande de tiempo cruza la nota entera.
    #[inline]
    #[must_use]
    pub const fn remaining_at(&self, now: Micros) -> Micros {
        self.onset_at.saturating_sub(now)
    }
}

/// Todos los avisos de una cancion, ya ordenados.
///
/// Inmutable y compartible: varias reproducciones pueden apuntar al mismo programa sin
/// copiarlo.
#[derive(Clone, Debug)]
pub struct CueSchedule {
    cues: Box<[Cue]>,
}

impl CueSchedule {
    /// Construye el programa de avisos con una antelacion en pulsos.
    ///
    /// `lead_ticks` es la antelacion en tiempo musical. Una antelacion de cero es valida:
    /// el aviso coincide con el ataque. Si la antelacion es mayor que la posicion de la
    /// nota, el aviso satura en el instante cero en lugar de irse a negativo.
    #[must_use]
    pub fn build(song: &Song, lead_ticks: u64) -> Self {
        let tempo = song.tempo_map();
        let mut cues: Vec<Cue> = song
            .notes()
            .iter()
            .enumerate()
            .map(|(i, n)| {
                let cue_tick = n.onset_tick.0.saturating_sub(lead_ticks);
                Cue {
                    cue_at: tempo.tick_to_us(Ticks(cue_tick)),
                    onset_at: n.onset_us,
                    cue_tick,
                    note_index: u32::try_from(i).unwrap_or(u32::MAX),
                }
            })
            .collect();

        // Orden total sin empates posibles: `note_index` es unico. Al derivar de una
        // linea temporal ya ordenada, hereda el desempate musical, de modo que un acorde
        // sale de grave a agudo sin necesidad de repetir la altura dentro del aviso.
        // `cue_tick` va antes que `note_index` para que dos avisos de distinto tiempo
        // musical que caen en el mismo microsegundo queden en orden musical.
        cues.sort_unstable_by_key(|c| (c.cue_at, c.cue_tick, c.note_index));

        debug_assert!(cues.windows(2).all(|w| matches!(w, [a, b] if a.cue_at <= b.cue_at)));
        Self { cues: cues.into_boxed_slice() }
    }

    /// Los avisos, ordenados de forma no decreciente por instante de aviso.
    #[must_use]
    pub fn cues(&self) -> &[Cue] {
        &self.cues
    }

    /// Cuantos avisos hay. Es igual al numero de notas de la cancion.
    #[must_use]
    pub fn len(&self) -> usize {
        self.cues.len()
    }

    /// `true` si la cancion no tiene ninguna nota.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.cues.is_empty()
    }
}

/// Se intento retroceder el tiempo dentro de una misma reproduccion.
///
/// Retroceder en silencio reemitiria avisos ya emitidos. El unico camino legal hacia
/// atras es [`Playback::seek`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rewind {
    /// Ultimo instante alcanzado.
    pub last: Micros,
    /// Instante solicitado, anterior al ultimo.
    pub requested: Micros,
}

impl fmt::Display for Rewind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "no se puede retroceder de {} a {} microsegundos: usa seek",
            self.last.0, self.requested.0
        )
    }
}

impl std::error::Error for Rewind {}

/// El recorrido de una cancion a lo largo del tiempo.
#[derive(Clone, Debug)]
pub struct Playback {
    schedule: Arc<CueSchedule>,
    next: usize,
    last: Micros,
    /// Candidatos examinados en el ultimo `advance_to`.
    ///
    /// Es diagnostico, no logica: una suma entera junto a una comparacion que ya se
    /// estaba haciendo. Existe porque la garantia de coste de SC-006 merece verificarse
    /// contando, no cronometrando: una prueba que mide milisegundos es intermitente en
    /// una maquina cargada y no demuestra la propiedad estructural.
    comparisons: usize,
}

impl Playback {
    /// Empieza a reproducir desde el instante cero.
    #[must_use]
    pub fn new(schedule: Arc<CueSchedule>) -> Self {
        Self { schedule, next: 0, last: Micros::ZERO, comparisons: 0 }
    }

    /// Avanza hasta `now` y devuelve los avisos que quedan cubiertos.
    ///
    /// El corte devuelto es una vista del programa: no copia ni asigna memoria.
    ///
    /// # Coste
    ///
    /// Exactamente `k + 1` comparaciones, donde `k` es el numero de avisos devueltos.
    /// **No depende del tamano de la cancion.** Esa propiedad es lo que mantiene la
    /// reproduccion dentro del presupuesto de latencia cuando la pieza es larga.
    ///
    /// # Errores
    ///
    /// [`Rewind`] si `now` es anterior al ultimo instante alcanzado. En ese caso el
    /// estado **no se modifica**.
    pub fn advance_to(&mut self, now: Micros) -> Result<&[Cue], Rewind> {
        if now < self.last {
            return Err(Rewind { last: self.last, requested: now });
        }
        self.last = now;

        let inicio = self.next;
        let cues = self.schedule.cues();
        let mut i = inicio;
        let mut examinados = 0usize;
        while let Some(c) = cues.get(i) {
            examinados = examinados.saturating_add(1);
            if c.cue_at > now {
                break;
            }
            i = i.saturating_add(1);
        }
        self.next = i;
        self.comparisons = examinados;

        debug_assert!(cues.get(..i).is_some_and(|previos| previos.iter().all(|c| c.cue_at <= now)));
        debug_assert!(cues.get(i..).is_some_and(|siguientes| siguientes.iter().all(|c| c.cue_at > now)));

        Ok(cues.get(inicio..i).unwrap_or(&[]))
    }

    /// Recoloca la reproduccion en `to`. Es el unico camino legal hacia atras.
    ///
    /// Coste `O(log n)`. No esta pensado para la ruta critica.
    pub fn seek(&mut self, to: Micros) {
        self.next = self.schedule.cues().partition_point(|c| c.cue_at <= to);
        self.last = to;
    }

    /// `true` cuando no queda ningun aviso por emitir.
    #[must_use]
    pub fn is_finished(&self) -> bool {
        self.next >= self.schedule.len()
    }

    /// Ultimo instante alcanzado.
    #[must_use]
    pub const fn position(&self) -> Micros {
        self.last
    }

    /// Cuantos avisos examino el ultimo [`advance_to`](Self::advance_to).
    ///
    /// Es `k + 1` cuando quedan avisos por delante y `k` cuando la cancion se agota,
    /// siendo `k` los avisos emitidos. **Nunca depende del tamano de la cancion**: esa
    /// es la garantia de SC-006, y este contador es como se verifica.
    #[must_use]
    pub const fn last_advance_comparisons(&self) -> usize {
        self.comparisons
    }

    /// Cuantos avisos quedan por emitir.
    #[must_use]
    pub fn remaining(&self) -> usize {
        self.schedule.len().saturating_sub(self.next)
    }
}
