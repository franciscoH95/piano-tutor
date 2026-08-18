//! La relacion entre tiempo musical y tiempo real.
//!
//! # Por que no se acumula dividiendo
//!
//! La conversion obvia seria calcular los microsegundos tramo a tramo y sumarlos. No
//! sirve: cada division trunca, y los truncados se acumulan. En una pieza larga con
//! cambios de tempo frecuentes esa deriva desplaza las notas y rompe el determinismo.
//!
//! Aqui cada tramo guarda su ancla en **microsegundos x PPQ**, que es exacta porque no
//! ha pasado por ninguna division. La unica division ocurre al final de cada consulta.
//!
//! # Redondeo
//!
//! [`TempoMap::tick_to_us`] **trunca hacia abajo**. Es contrato publico, esta cubierto
//! por pruebas, y cambiarlo seria un cambio incompatible.

use crate::error::LoadError;
use crate::time::{Micros, Ticks};

/// Microsegundos por negra que asume el estandar MIDI cuando el archivo no declara
/// ninguno: 120 negras por minuto.
pub const DEFAULT_US_PER_QN: u32 = 500_000;

/// Maximo valor representable en el meta `Set Tempo`, que son tres bytes.
const MAX_US_PER_QN: u32 = 0x00FF_FFFF;

/// Techo de ticks absolutos. Son ~621 horas a PPQ 960 y 120 negras/min.
const MAX_TICK: u64 = u32::MAX as u64;

/// Techo de duracion: 24 horas en microsegundos.
const MAX_DURATION_US: u64 = 24 * 60 * 60 * 1_000_000;

/// Un tramo de tempo constante.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TempoSegment {
    /// Tick absoluto en el que empieza el tramo.
    pub start_tick: u64,
    /// Microsegundos x PPQ acumulados hasta `start_tick`. Exacto, sin divisiones.
    pub anchor_scaled: u64,
    /// `anchor_scaled / ppq`, precalculado para la busqueda inversa.
    pub start_us: u64,
    /// Microsegundos por negra vigentes en el tramo.
    pub us_per_qn: u32,
}

/// El mapa de tempo de una cancion.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TempoMap {
    ppq: u32,
    segments: Vec<TempoSegment>,
}

impl TempoMap {
    /// Construye el mapa a partir de los eventos de tempo de la cancion.
    ///
    /// `tempos` son pares `(tick_absoluto, microsegundos_por_negra)`. No hace falta que
    /// vengan ordenados ni que haya alguno en el tick 0.
    ///
    /// # Errores
    ///
    /// - [`LoadError::DivisionCero`] si `ppq` es cero o mayor que 32767.
    /// - [`LoadError::InvalidTempo`] si algun tempo es cero o excede los tres bytes del
    ///   estandar. No se corrige en silencio: un tempo cero implica dividir por cero, y
    ///   un clamp callado entregaria una leccion falsa.
    /// - [`LoadError::TickOverflow`] si algun tick supera `u32::MAX`.
    /// - [`LoadError::DuracionExcesiva`] si la pieza pasa de 24 horas.
    pub fn new(ppq: u32, tempos: &[(u64, u32)]) -> Result<Self, LoadError> {
        if ppq == 0 || ppq > 32_767 {
            return Err(LoadError::DivisionCero);
        }
        for &(tick, us) in tempos {
            if us == 0 || us > MAX_US_PER_QN {
                return Err(LoadError::InvalidTempo { tick, us_per_qn: us });
            }
            if tick > MAX_TICK {
                return Err(LoadError::TickOverflow { track: 0 });
            }
        }

        // Orden estable por tick: dentro de un mismo tick se conserva el orden de
        // llegada, y el colapso posterior se queda con el ultimo. Es la regla del
        // estandar y la que hace la construccion reproducible.
        let mut pares: Vec<(u64, u32)> = tempos.to_vec();
        pares.sort_by_key(|(tick, _)| *tick);

        // Colapsar ticks repetidos quedandose con el ultimo. Nunca deben quedar tramos
        // de longitud cero: son lo que rompe la monotonia estricta de `start_us`, de la
        // que depende la busqueda binaria inversa.
        let mut plano: Vec<(u64, u32)> = Vec::with_capacity(pares.len() + 1);
        for (tick, us) in pares {
            match plano.last_mut() {
                Some(ultimo) if ultimo.0 == tick => ultimo.1 = us,
                _ => plano.push((tick, us)),
            }
        }

        // Garantizar un tramo que empiece en el tick 0 (FR-005).
        if !matches!(plano.first(), Some((0, _))) {
            plano.insert(0, (0, DEFAULT_US_PER_QN));
        }

        let mut segments: Vec<TempoSegment> = Vec::with_capacity(plano.len());
        let mut anchor_scaled: u64 = 0;
        let mut prev: Option<(u64, u32)> = None;
        for (tick, us_per_qn) in plano {
            if let Some((prev_tick, prev_us)) = prev {
                let dt = tick - prev_tick; // > 0 por el colapso previo
                let inc = dt
                    .checked_mul(u64::from(prev_us))
                    .ok_or(LoadError::DuracionExcesiva { us: u64::MAX })?;
                anchor_scaled = anchor_scaled
                    .checked_add(inc)
                    .ok_or(LoadError::DuracionExcesiva { us: u64::MAX })?;
            }
            let start_us = anchor_scaled / u64::from(ppq);
            if start_us > MAX_DURATION_US {
                return Err(LoadError::DuracionExcesiva { us: start_us });
            }
            segments.push(TempoSegment { start_tick: tick, anchor_scaled, start_us, us_per_qn });
            prev = Some((tick, us_per_qn));
        }

        debug_assert!(!segments.is_empty());
        debug_assert!(matches!(segments.first(), Some(s) if s.start_tick == 0));
        // Los patrones de corte evitan indexar: es la misma comprobacion sin poder panicar.
        debug_assert!(segments.windows(2).all(|w| matches!(w, [a, b] if a.start_tick < b.start_tick)));
        debug_assert!(segments.windows(2).all(|w| matches!(w, [a, b] if a.start_us < b.start_us)));

        Ok(Self { ppq, segments })
    }

    /// Pulsos por negra declarados por el archivo.
    #[must_use]
    pub const fn ppq(&self) -> u32 {
        self.ppq
    }

    /// Los tramos de tempo, en orden.
    #[must_use]
    pub fn segments(&self) -> &[TempoSegment] {
        &self.segments
    }

    /// Convierte tiempo musical en tiempo real. Trunca hacia abajo.
    ///
    /// Coste `O(log n)` sobre el numero de tramos. No asigna y no puede entrar en panico.
    #[must_use]
    pub fn tick_to_us(&self, tick: Ticks) -> Micros {
        let k = self.segments.partition_point(|s| s.start_tick <= tick.0).saturating_sub(1);
        let Some(seg) = self.segments.get(k) else {
            return Micros::ZERO; // inalcanzable: `segments` nunca esta vacio
        };
        let delta = tick.0.saturating_sub(seg.start_tick);
        let scaled = seg
            .anchor_scaled
            .saturating_add(delta.saturating_mul(u64::from(seg.us_per_qn)));
        Micros(scaled / u64::from(self.ppq))
    }

    /// Convierte tiempo real en tiempo musical: el mayor tick cuyo tiempo real no pasa
    /// de `us`.
    ///
    /// No esta en la ruta critica: solo se usa para reposicionar la reproduccion.
    #[must_use]
    pub fn us_to_tick(&self, us: Micros) -> Ticks {
        let k = self.segments.partition_point(|s| s.start_us <= us.0).saturating_sub(1);
        let Some(seg) = self.segments.get(k) else {
            return Ticks::ZERO;
        };
        // Buscamos el mayor tick con `floor(scaled / ppq) <= us`, que equivale a
        // `scaled <= (us + 1) * ppq - 1`. Sin ese `+1 ... -1` se devolveria el tick
        // anterior siempre que la division no cuadrase exacta, que es casi siempre.
        // u128 para que el producto no pueda desbordar antes de restar el ancla.
        let techo = (u128::from(us.0) + 1) * u128::from(self.ppq) - 1;
        let num = techo.saturating_sub(u128::from(seg.anchor_scaled));
        let avance = (num / u128::from(seg.us_per_qn)).min(u128::from(u64::MAX)) as u64;
        let tick = seg.start_tick.saturating_add(avance);

        // El resultado no puede invadir el tramo siguiente: `partition_point` eligio el
        // tramo cuyo `start_us` no pasa de `us`, asi que el siguiente empieza despues.
        // El clamp es defensa en profundidad, no deberia llegar a actuar nunca.
        match self.segments.get(k + 1) {
            Some(siguiente) => Ticks(tick.min(siguiente.start_tick.saturating_sub(1))),
            None => Ticks(tick),
        }
    }
}
