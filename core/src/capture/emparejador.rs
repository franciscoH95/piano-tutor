//! Emparejar ataques con sueltas, y cerrar lo que quede abierto.
//!
//! # Una divergencia deliberada respecto a la carga de canciones
//!
//! La feature 001 empareja en FIFO: ve el archivo entero, así que cuando una altura se
//! repite sin soltarse puede mirar hacia adelante y decidir qué suelta cierra qué ataque.
//!
//! Aquí no se puede. La captura es un flujo: cuando llega el segundo ataque de una tecla
//! que ya estaba hundida, hay que decidir **en ese instante** qué pasa con la primera, sin
//! saber qué vendrá después. Esperar a saberlo significaría retrasar la entrega, y
//! retrasar la entrega es exactamente lo que prohíbe el presupuesto de 30 ms.
//!
//! La regla, por tanto, es: **una tecla repulsada cierra la nota anterior en ese punto**.
//! Físicamente es lo que ocurre —una tecla solo puede estar hundida una vez—, y deja
//! constancia con [`Cierre::PorRepulsacion`] para que quien compare después sepa que ese
//! final no lo puso una suelta.

use crate::capture::evento::{EventoCrudo, TipoEvento};
use crate::time::Micros;

/// Canales MIDI por alturas: el tamaño exacto de la tabla de voces.
const RANURAS: usize = 16 * 128;

/// Rango de las 88 teclas de un piano.
const PIANO_MIN: u8 = 21;
const PIANO_MAX: u8 = 108;

/// Canal de percusión del estándar, contado desde cero.
use crate::timeline::CANAL_PERCUSION;

/// Cómo terminó una nota.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Cierre {
    /// La tecla se soltó de verdad.
    PorSuelta,
    /// El usuario detuvo la captura con la tecla aún hundida.
    PorParada,
    /// El teclado desapareció. La duración es censurada.
    PorPerdidaDeDispositivo,
    /// La misma tecla volvió a pulsarse sin haberse soltado.
    PorRepulsacion,
}

/// Una nota que el alumno tocó, ya con principio y final.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct PulsacionCapturada {
    /// Instante del ataque.
    pub onset: Micros,
    /// Instante del final. Siempre mayor que `onset`.
    pub end: Micros,
    /// Altura MIDI.
    pub key: u8,
    /// Intensidad del ataque.
    pub velocity: u8,
    /// Canal MIDI de origen.
    pub channel: u8,
    /// Cómo terminó.
    pub closure: Cierre,
    /// `true` cuando sabemos cuándo empezó pero no cuándo terminó de verdad.
    pub duracion_censurada: bool,
}

/// Lo que se toleró durante la sesión. Todo a cero en una captura limpia.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct InformeDeCaptura {
    /// Sueltas que no correspondían a ninguna tecla hundida.
    pub sueltas_sin_pulsacion: u32,
    /// Notas cerradas al detener la captura.
    pub cerradas_por_parada: u32,
    /// Notas cerradas al perderse el dispositivo.
    pub cerradas_por_perdida: u32,
    /// Teclas repulsadas sin haberse soltado.
    pub repulsaciones: u32,
    /// Notas del canal de percusión.
    pub percusion: u32,
    /// Notas fuera de las 88 teclas de un piano.
    pub fuera_de_88_teclas: u32,
}

impl InformeDeCaptura {
    /// `true` si no hubo que tolerar nada.
    #[must_use]
    pub fn esta_limpio(&self) -> bool {
        *self == Self::default()
    }
}

#[derive(Clone, Copy)]
struct Abierta {
    onset: Micros,
    velocity: u8,
}

/// Convierte el flujo de eventos en notas con principio y final.
pub struct Emparejador {
    /// Tabla plana indexada por (canal, altura). No asigna y su coste es constante, que
    /// es lo que la ruta crítica exige. Una tecla solo puede estar hundida una vez, así
    /// que una ranura por voz basta.
    abiertas: Vec<Option<Abierta>>,
    informe: InformeDeCaptura,
}

impl Default for Emparejador {
    fn default() -> Self {
        Self::nuevo()
    }
}

#[inline]
const fn ranura(channel: u8, key: u8) -> usize {
    // `channel` viene de un nibble y `key` está enmascarada a 7 bits en la frontera, así
    // que el índice cae siempre dentro de la tabla. Aun así el acceso se hace con `get`.
    ((channel & 0x0F) as usize) * 128 + ((key & 0x7F) as usize)
}

impl Emparejador {
    /// Un emparejador vacío. Reserva su tabla de una sola vez.
    #[must_use]
    pub fn nuevo() -> Self {
        Self { abiertas: vec![None; RANURAS], informe: InformeDeCaptura::default() }
    }

    /// Consume un evento. Devuelve una nota cuando alguna queda cerrada.
    ///
    /// Un ataque puede cerrar una nota: es el caso de la tecla repulsada.
    pub fn consumir(&mut self, ev: EventoCrudo) -> Option<PulsacionCapturada> {
        let idx = ranura(ev.channel, ev.key);
        let previa = self.abiertas.get_mut(idx)?;
        match ev.kind {
            TipoEvento::Ataque => {
                let cerrada = previa.map(|a| {
                    self.informe.repulsaciones = self.informe.repulsaciones.saturating_add(1);
                    nota(a, &ev, ev.at, Cierre::PorRepulsacion, false)
                });
                *previa = Some(Abierta { onset: ev.at, velocity: ev.velocity });
                self.contar(&ev);
                cerrada
            }
            TipoEvento::Suelta => match previa.take() {
                Some(a) => Some(nota(a, &ev, ev.at, Cierre::PorSuelta, false)),
                None => {
                    self.informe.sueltas_sin_pulsacion =
                        self.informe.sueltas_sin_pulsacion.saturating_add(1);
                    None
                }
            },
        }
    }

    /// Cierra todas las teclas que sigan hundidas.
    ///
    /// `at` es el instante que se les asigna como final. Para
    /// [`Cierre::PorPerdidaDeDispositivo`] debe ser el del **último evento recibido**, no
    /// el de la detección: la detección llega más tarde, y datarlas ahí inventaría
    /// duración que no ocurrió. Esas notas se marcan con `duracion_censurada`.
    pub fn cerrar(&mut self, at: Micros, motivo: Cierre) -> Vec<PulsacionCapturada> {
        let censurada = motivo == Cierre::PorPerdidaDeDispositivo;
        let mut salida = Vec::new();
        // Se recorre la tabla en orden de índice, que es (canal, altura) ascendente: el
        // resultado es estable y no depende del orden en que se pulsaron las teclas.
        for idx in 0..RANURAS {
            let Some(hueco) = self.abiertas.get_mut(idx) else { continue };
            let Some(a) = hueco.take() else { continue };
            let channel = u8::try_from(idx / 128).unwrap_or(0);
            let key = u8::try_from(idx % 128).unwrap_or(0);
            let ev = EventoCrudo {
                at,
                seq: 0,
                key,
                velocity: a.velocity,
                kind: TipoEvento::Suelta,
                channel,
            };
            salida.push(nota(a, &ev, at, motivo, censurada));
        }
        match motivo {
            Cierre::PorParada => {
                self.informe.cerradas_por_parada = self
                    .informe
                    .cerradas_por_parada
                    .saturating_add(u32::try_from(salida.len()).unwrap_or(u32::MAX));
            }
            Cierre::PorPerdidaDeDispositivo => {
                self.informe.cerradas_por_perdida = self
                    .informe
                    .cerradas_por_perdida
                    .saturating_add(u32::try_from(salida.len()).unwrap_or(u32::MAX));
            }
            _ => {}
        }
        salida
    }

    /// Qué se toleró durante la sesión.
    #[must_use]
    pub const fn informe(&self) -> &InformeDeCaptura {
        &self.informe
    }

    fn contar(&mut self, ev: &EventoCrudo) {
        if ev.channel == CANAL_PERCUSION {
            self.informe.percusion = self.informe.percusion.saturating_add(1);
        }
        if ev.key < PIANO_MIN || ev.key > PIANO_MAX {
            self.informe.fuera_de_88_teclas = self.informe.fuera_de_88_teclas.saturating_add(1);
        }
    }
}

/// Una nota nunca dura cero: un ataque sin duración sería un ataque sin sonido.
fn nota(a: Abierta, ev: &EventoCrudo, end: Micros, closure: Cierre, censurada: bool) -> PulsacionCapturada {
    PulsacionCapturada {
        onset: a.onset,
        end: Micros(end.0.max(a.onset.0.saturating_add(1))),
        key: ev.key,
        velocity: a.velocity,
        channel: ev.channel,
        closure,
        duracion_censurada: censurada,
    }
}
