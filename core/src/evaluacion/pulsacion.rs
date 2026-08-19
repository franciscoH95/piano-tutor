//! Lo que el alumno toco, con sus dos extremos casados.
//!
//! La captura entrega **ataques y sueltas por separado** —`Observacion { at, key, velocity,
//! kind, channel }`—, no duraciones. Casarlos es trabajo de aqui, y trae dos casos que se
//! deciden en este archivo y no se descubren despues.

use crate::capture::{Observacion, TipoEvento};
use crate::time::Micros;

/// Una tecla que el alumno pulso.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Pulsacion {
    /// Altura MIDI.
    pub key: u8,
    /// Instante del ataque, sellado con el reloj de sesion.
    pub ataque_us: Micros,
    /// Instante de la suelta.
    ///
    /// `None` si la tecla **seguia hundida** al cerrar. Desconocido, que **no es cero**:
    /// cero significaria que la solto en el mismo instante en que la pulso, y eso seria
    /// afirmar algo que nadie observo.
    pub final_us: Option<Micros>,
    /// Intensidad del ataque. La de la suelta se descarta.
    pub velocity: u8,
}

/// Casa ataques con sueltas segun van llegando.
///
/// **No asigna por evento**: la tabla de teclas abiertas es de tamaño fijo —128, las del
/// protocolo— y el vector de pulsaciones crece amortizado. Puede vivir en la ruta critica
/// sin tocar el presupuesto del Principio IV.
pub struct Pulsaciones {
    /// Por tecla, la pulsacion abierta que le corresponde.
    abiertas: [Option<usize>; 128],
    cerradas: Vec<Pulsacion>,
}

impl Default for Pulsaciones {
    fn default() -> Self {
        Self::nuevas()
    }
}

impl Pulsaciones {
    /// Sin nada tocado todavia.
    #[must_use]
    pub const fn nuevas() -> Self {
        Self { abiertas: [None; 128], cerradas: Vec::new() }
    }

    /// Un ataque o una suelta.
    pub fn observar(&mut self, obs: Observacion) {
        let k = usize::from(obs.key & 0x7F);
        match obs.kind {
            TipoEvento::Ataque => {
                // Un teclado real puede repetir el ataque de una tecla mantenida. La
                // primera pulsacion no puede desaparecer: el alumno la toco.
                self.cerradas.push(Pulsacion {
                    key: obs.key,
                    ataque_us: obs.at,
                    final_us: None,
                    velocity: obs.velocity,
                });
                if let Some(slot) = self.abiertas.get_mut(k) {
                    *slot = Some(self.cerradas.len().saturating_sub(1));
                }
            }
            TipoEvento::Suelta => {
                // Una suelta sin ataque previo se descarta: pasa de verdad si la aplicacion
                // arranca con una tecla ya hundida.
                let Some(slot) = self.abiertas.get_mut(k) else {
                    return;
                };
                let Some(i) = slot.take() else {
                    return;
                };
                if let Some(p) = self.cerradas.get_mut(i) {
                    p.final_us = Some(obs.at);
                }
            }
        }
    }

    /// Las pulsaciones vistas hasta ahora, en orden de ataque.
    ///
    /// Se puede consultar sin consumir: el evaluador las empareja segun llegan y necesita
    /// que las teclas todavia hundidas sigan abiertas.
    #[must_use]
    pub fn vistas(&self) -> &[Pulsacion] {
        &self.cerradas
    }

    /// Cierra y devuelve las pulsaciones, **ordenadas por ataque**.
    ///
    /// El orden canonico de salida es lo que impide que el resultado dependa de en que orden
    /// llegaron dos observaciones del mismo instante (SC-008).
    #[must_use]
    pub fn cerrar(mut self) -> Vec<Pulsacion> {
        self.cerradas.sort_by_key(|p| (p.ataque_us.0, p.key));
        self.cerradas
    }
}
