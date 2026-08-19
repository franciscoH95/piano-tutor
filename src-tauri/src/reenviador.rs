//! El hilo que lleva las pulsaciones del anillo al puente.
//!
//! # Por que existe un hilo solo para esto
//!
//! Enviar por el canal de Tauri cuesta, medido, un percentil 95 de entre 0,5 y 1,6 ms, y
//! llega a 13 ms en el peor caso. Hacerlo desde el consumidor de tiempo real —o peor, desde
//! el callback del sistema— arruinaria el presupuesto de 30 ms que la feature 002 dejo
//! cerrado y verificado.
//!
//! Este hilo duerme (`Receptor::esperar`) en lugar de sondear, asi que no gasta CPU cuando
//! el alumno no toca.

use crate::comandos::{Estado, MensajeAlFrontend};
use piano_core::capture::{EventoCrudo, FuenteDeEventos, TipoEvento};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// Arranca el reenviador. Se detiene al poner la bandera.
pub fn arrancar<F>(mut fuente: F, estado: Arc<Estado>, parar: Arc<AtomicBool>)
where
    F: FuenteDeEventos + Send + 'static,
{
    std::thread::spawn(move || bucle(&mut fuente, &estado, &parar));
}

/// El bucle en si, sobre una fuente **prestada**.
///
/// Existe separado de `arrancar` porque `Captura` retiene el puerto y el cliente de
/// CoreMIDI, que no son `Send`: no se pueden mandar a otro hilo. Lo que si es `Send` es el
/// `Receptor`, pero solo se puede tomar prestado de la `Captura`. La salida es que **el
/// hilo abra el dispositivo el mismo** y llame aqui con el prestamo, de modo que nada que
/// no pueda cruzar cruza.
pub fn bucle<F>(fuente: &mut F, estado: &Arc<Estado>, parar: &Arc<AtomicBool>)
where
    F: FuenteDeEventos + ?Sized,
{
    // Prioridad elevada: la cola de latencia la pone el planificador, no el codigo.
    // Si el sistema no lo permite se sigue igual, solo sin ese seguro.
    let _ = piano_midi_io::prioridad::elevar_hilo_actual();

    let mut buffer: Vec<EventoCrudo> = Vec::with_capacity(1_024);
    while !parar.load(Ordering::Relaxed) {
        fuente.esperar();
        buffer.clear();
        if fuente.recoger(&mut buffer) == 0 {
            continue;
        }
        for ev in buffer.drain(..) {
            estado.enviar(MensajeAlFrontend::Tecla {
                key: ev.key,
                pulsada: ev.kind == TipoEvento::Ataque,
            });
        }
    }
}
