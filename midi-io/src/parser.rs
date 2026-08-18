//! De bytes MIDI a observaciones.
//!
//! # Por que este archivo existe
//!
//! La biblioteca obvia para leer MIDI en Rust, `midir`, quedo descartada porque su bucle
//! de paquetes rebana un corte cuya longitud deduce **solo del byte de estado**, sin
//! comprobar cuantos bytes quedan de verdad. Ante un paquete truncado desborda el corte,
//! y como ocurre dentro de un callback del sistema —cruzando una frontera FFI— el panico
//! no es capturable: **aborta el proceso**. Es su issue #160, abierto desde 2024 y sin
//! corregir. Un solo teclado que emita basura al encenderse mataria la aplicacion.
//!
//! Aqui esa clase de fallo no puede ni escribirse: el crate declara
//! `deny(clippy::indexing_slicing)`, asi que todo acceso pasa por `get(..)` y un paquete
//! corto se traduce en "no hay mas que leer", no en un panico.

use piano_core::capture::{Observacion, TipoEvento};
use piano_core::time::Micros;

/// Analiza un flujo de bytes MIDI, conservando el estado en carrera entre paquetes.
#[derive(Clone, Copy, Debug, Default)]
pub struct Parser {
    /// Ultimo byte de estado visto. El estandar permite omitirlo en mensajes sucesivos
    /// del mismo tipo, y es justo el truco que acompana a la velocity cero.
    estado: Option<u8>,
}

impl Parser {
    /// Un analizador sin estado previo.
    #[must_use]
    pub const fn nuevo() -> Self {
        Self { estado: None }
    }

    /// Recorre un paquete y entrega una observacion por cada nota reconocida.
    ///
    /// Todas las notas del paquete comparten `at`: el reloj se lee **una vez por
    /// paquete**, no una por mensaje, de modo que un acorde tiene un instante unico por
    /// construccion.
    ///
    /// Lo que no sea nota se descarta en silencio (FR-014). Lo que este truncado detiene
    /// el recorrido sin ruido: nunca entra en panico, sea cual sea la entrada.
    pub fn consumir<F: FnMut(Observacion)>(&mut self, at: Micros, bytes: &[u8], mut emitir: F) {
        let mut i: usize = 0;
        while let Some(&b) = bytes.get(i) {
            // Tiempo real (0xF8..=0xFF): un byte, puede aparecer en cualquier hueco y
            // NO cancela el estado en carrera.
            if b >= 0xF8 {
                i = i.saturating_add(1);
                continue;
            }
            // Sysex: se salta hasta su terminador. Cancela el estado en carrera.
            if b == 0xF0 {
                self.estado = None;
                i = i.saturating_add(1);
                while let Some(&c) = bytes.get(i) {
                    i = i.saturating_add(1);
                    if c == 0xF7 {
                        break;
                    }
                }
                continue;
            }
            // Resto de mensajes de sistema comunes. Tambien cancelan el estado.
            if (0xF1..=0xF7).contains(&b) {
                self.estado = None;
                let datos = match b {
                    0xF2 => 2,
                    0xF1 | 0xF3 => 1,
                    _ => 0,
                };
                i = i.saturating_add(1).saturating_add(datos);
                continue;
            }

            // Byte de estado explicito, o estado en carrera.
            let (estado, inicio_datos) = if b >= 0x80 {
                self.estado = Some(b);
                (b, i.saturating_add(1))
            } else {
                match self.estado {
                    Some(s) => (s, i),
                    // Dato suelto sin ningun estado previo: no hay forma de interpretarlo.
                    None => {
                        i = i.saturating_add(1);
                        continue;
                    }
                }
            };

            let cuantos = match estado & 0xF0 {
                0xC0 | 0xD0 => 1,
                _ => 2,
            };
            let fin = inicio_datos.saturating_add(cuantos);
            // Aqui es donde `midir` desborda. `get` devuelve `None` y salimos.
            let Some(datos) = bytes.get(inicio_datos..fin) else {
                return;
            };
            i = fin;

            let canal = estado & 0x0F;
            // Los bytes de datos del estandar son de 7 bits. Un flujo malformado podria
            // colar aqui un valor >= 0x80; enmascarar lo mantiene dentro del rango real de
            // notas y evita que un byte basura se propague al resto del sistema.
            match (estado & 0xF0, datos.first(), datos.get(1)) {
                (0x80, Some(&key), Some(_)) => {
                    let key = key & 0x7F;
                    emitir(Observacion { at, key, velocity: 0, kind: TipoEvento::Suelta, channel: canal });
                }
                (0x90, Some(&key), Some(&velocity)) => {
                    let (key, velocity) = (key & 0x7F, velocity & 0x7F);
                    // Velocity cero equivale a soltar la tecla. Regla dura del estandar.
                    let kind = if velocity == 0 { TipoEvento::Suelta } else { TipoEvento::Ataque };
                    let velocity = if velocity == 0 { 0 } else { velocity };
                    emitir(Observacion { at, key, velocity, kind, channel: canal });
                }
                // Pedal, aftertouch, cambios de instrumento, rueda de tono: se descartan.
                _ => {}
            }
        }
    }
}
