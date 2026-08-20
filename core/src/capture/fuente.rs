//! De donde salen las pulsaciones.
//!
//! El nucleo define **que** es una fuente y no sabe **como** se obtienen los eventos. Eso
//! es lo que permite ejercer toda la logica sin ventana y sin teclado conectado, que es
//! lo que exige el Principio III.
//!
//! Se inyecta por **generico**, nunca como `dyn`: en la ruta critica no se paga despacho
//! dinamico. Es la misma decision que ya toma [`Clock`](crate::clock::Clock).

use crate::capture::evento::{EventoCrudo, Observacion};
use crate::capture::transporte::Receptor;
use crate::time::Micros;

/// Una fuente de pulsaciones.
pub trait FuenteDeEventos {
    /// Vacia lo disponible en `destino` y devuelve cuantos eventos habia. No bloquea.
    fn recoger(&mut self, destino: &mut Vec<EventoCrudo>) -> usize;

    /// Espera hasta que haya algo que recoger. Es donde el consumidor duerme.
    fn esperar(&mut self);

    /// Espera hasta que haya algo que recoger, **o hasta que pase `plazo`**.
    ///
    /// Es la unica forma de que un bucle de consumo pueda volver a mirar su bandera de
    /// parada. Con [`esperar`](Self::esperar) a secas, un consumidor que deja de recibir
    /// —porque desenchufaron el teclado— no despierta nunca. Se declara obligatorio y no
    /// con una implementacion por defecto que llame a `esperar`: esa por defecto seria
    /// precisamente el cuelgue, servido en silencio a quien no lo sobrescriba.
    fn esperar_hasta(&mut self, plazo: std::time::Duration);

    /// `true` si la fuente ya no puede entregar nada mas.
    fn agotada(&self) -> bool;
}

impl FuenteDeEventos for Receptor {
    fn recoger(&mut self, destino: &mut Vec<EventoCrudo>) -> usize {
        Receptor::recoger(self, destino)
    }
    fn esperar(&mut self) {
        Receptor::esperar(self);
    }
    fn esperar_hasta(&mut self, plazo: std::time::Duration) {
        Receptor::esperar_hasta(self, plazo);
    }
    fn agotada(&self) -> bool {
        false // un dispositivo abierto siempre puede entregar mas
    }
}

/// Fuente controlada: entrega un guion fijo, sin hardware y sin esperar tiempo real.
///
/// Es la implementacion que cumple FR-021 y FR-022, y la que hace que toda la suite pase
/// en una maquina sin ningun teclado conectado.
#[derive(Clone, Debug)]
pub struct FuenteGuionizada {
    eventos: Vec<EventoCrudo>,
    siguiente: usize,
}

impl FuenteGuionizada {
    /// Construye la fuente a partir de una secuencia de observaciones.
    ///
    /// Aplica las mismas reglas que el transporte real —numeracion monotona y sujecion de
    /// instantes que retroceden— para que lo que prueba esta fuente sea lo mismo que
    /// ocurre en produccion, y no una version simplificada.
    #[must_use]
    pub fn nueva(guion: Vec<Observacion>) -> Self {
        let mut ultimo = Micros::ZERO;
        let eventos = guion
            .into_iter()
            .enumerate()
            .map(|(i, o)| {
                let at = if o.at < ultimo { ultimo } else { o.at };
                ultimo = at;
                EventoCrudo {
                    at,
                    seq: u32::try_from(i).unwrap_or(u32::MAX),
                    key: o.key,
                    velocity: o.velocity,
                    kind: o.kind,
                    channel: o.channel,
                }
            })
            .collect();
        Self { eventos, siguiente: 0 }
    }
}

impl FuenteDeEventos for FuenteGuionizada {
    fn recoger(&mut self, destino: &mut Vec<EventoCrudo>) -> usize {
        let Some(pendientes) = self.eventos.get(self.siguiente..) else {
            return 0;
        };
        destino.extend_from_slice(pendientes);
        self.siguiente = self.eventos.len();
        pendientes.len()
    }

    fn esperar(&mut self) {
        // No hay nada que esperar: el guion entero esta disponible desde el principio.
        // Es justo lo que permite recorrer diez minutos de pieza en microsegundos.
    }

    fn esperar_hasta(&mut self, _plazo: std::time::Duration) {
        // Tampoco espera. Dormir el plazo aqui convertiria cada prueba en una espera real.
    }

    fn agotada(&self) -> bool {
        self.siguiente >= self.eventos.len()
    }
}
