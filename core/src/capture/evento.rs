//! Lo que cruza la cola.

use crate::time::Micros;

/// Ataque o suelta. No hay mas: el pedal y los demas mensajes se descartan en la
/// frontera, antes de llegar aqui.
#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum TipoEvento {
    /// La tecla se pulso.
    Ataque = 0,
    /// La tecla se solto.
    Suelta = 1,
}

/// Lo que observa el adaptador: una pulsacion ya sellada con el reloj de sesion, pero
/// todavia sin identidad propia.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Observacion {
    /// Instante sellado por el reloj de sesion.
    pub at: Micros,
    /// Altura MIDI.
    pub key: u8,
    /// Intensidad del ataque. Cero en las sueltas.
    pub velocity: u8,
    /// Ataque o suelta.
    pub kind: TipoEvento,
    /// Canal MIDI de origen.
    pub channel: u8,
}

/// El evento tal y como viaja por la cola. **Exactamente 16 bytes.**
///
/// El tamano importa: determina cuantos caben en el espacio acotado, y este es el unico
/// tipo que recorre la ruta critica.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct EventoCrudo {
    /// Instante, ya sujeto a monotonia.
    pub at: Micros,
    /// Contador monotono asignado por el transporte.
    ///
    /// Un salto en la secuencia le dice al consumidor **exactamente donde** se perdio
    /// algo, sin gastar una ranura ni trabajo del productor.
    pub seq: u32,
    /// Altura MIDI.
    pub key: u8,
    /// Intensidad.
    pub velocity: u8,
    /// Ataque o suelta.
    pub kind: TipoEvento,
    /// Canal MIDI.
    pub channel: u8,
}
