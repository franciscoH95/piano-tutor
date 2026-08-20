//! El hilo que lleva las pulsaciones del anillo al puente.
//!
//! # Por que existe un hilo solo para esto
//!
//! Enviar por el canal de Tauri cuesta, medido, un percentil 95 de entre 0,5 y 1,6 ms, y
//! llega a 13 ms en el peor caso. Hacerlo desde el consumidor de tiempo real —o peor, desde
//! el callback del sistema— arruinaria el presupuesto de 30 ms que la feature 002 dejo
//! cerrado y verificado.
//!
//! # Por que el bucle tiene latido
//!
//! Hasta el 2026-08-19 dormia con `Receptor::esperar()`, que es `park()` sin plazo. Cuando
//! el teclado deja de mandar —que es exactamente lo que pasa al desenchufarlo— el hilo se
//! quedaba dormido para siempre: no volvia a mirar la bandera de parada, no terminaba, no
//! soltaba la `Captura` y por tanto **no se ejecutaba el `Drop` que cierra el puerto**. En
//! macOS pasaba desapercibido porque CoreMIDI reparte la misma fuente entre todos los
//! clientes; en Windows el puerto de entrada es exclusivo y el teclado quedaba inservible
//! hasta que muriese el proceso (FR-006).
//!
//! Con latido, el hilo despierta cada [`LATIDO`] aunque no llegue nada. No gasta CPU
//! mientras duerme: son diez despertares por segundo que no hacen nada y se vuelven a
//! dormir.

use crate::comandos::{Estado, MensajeAlFrontend};
use piano_core::capture::{EventoCrudo, FuenteDeEventos, SesionDeCaptura, TipoEvento};
use piano_midi_io::vigia::{Presencia, Vigia};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

/// Cada cuanto despierta el bucle aunque no llegue ninguna nota.
///
/// Tiene que ser **bastante menor** que [`piano_midi_io::vigia::INTERVALO_SONDEO`]: el
/// vigia colapsa los avisos pendientes en el ultimo, asi que si el bucle mirase mas despacio
/// que el vigia, dos ausencias seguidas se leerian como una sola y la doble confirmacion
/// necesitaria un sondeo de mas.
const LATIDO: Duration = Duration::from_millis(100);

/// Como termino el bucle.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Desenlace {
    /// Se levanto la bandera: se eligio otro teclado, o se cierra la aplicacion.
    Detenido,
    /// El teclado desaparecio de dos enumeraciones consecutivas.
    Perdido,
}

/// Quien dice si el teclado sigue ahi.
///
/// Existe como rasgo, y no como un [`Vigia`] a secas, para que la regla de perdida se pueda
/// probar sin hardware: la prueba le da un guion de presencias y comprueba que hacen falta
/// **dos** ausencias seguidas. Sin esto, el unico camino que encendia `DispositivoPerdido`
/// era que fallase la apertura inicial, y desenchufar a media practica no producia nada.
pub trait ObservadorDePresencia {
    /// Lo ultimo observado, si hay algo nuevo. No bloquea.
    fn novedad(&mut self) -> Option<Presencia>;
}

impl ObservadorDePresencia for Vigia {
    fn novedad(&mut self) -> Option<Presencia> {
        Vigia::novedad(self)
    }
}

/// Un observador que no observa. Para los casos en que no hay a quien vigilar.
pub struct SinVigilancia;

impl ObservadorDePresencia for SinVigilancia {
    fn novedad(&mut self) -> Option<Presencia> {
        None
    }
}

/// El bucle en si, sobre una fuente **prestada**.
///
/// Existe separado del hilo que lo llama porque `Captura` retiene el puerto y el cliente de
/// CoreMIDI, que no son `Send`: no se pueden mandar a otro hilo. Lo que si es `Send` es el
/// `Receptor`, pero solo se puede tomar prestado de la `Captura`. La salida es que **el
/// hilo abra el dispositivo el mismo** y llame aqui con el prestamo, de modo que nada que
/// no pueda cruzar cruza.
///
/// La regla de perdida no se decide aqui: la aplica [`SesionDeCaptura`], que ya esta
/// probada en el nucleo. Una sola ausencia puede ser un parpadeo del sistema al reenumerar,
/// y declarar la perdida por ella apagaria el teclado del alumno a mitad de compas.
pub fn bucle<F, P>(
    fuente: &mut F,
    estado: &Arc<Estado>,
    parar: &Arc<AtomicBool>,
    observador: &mut P,
) -> Desenlace
where
    F: FuenteDeEventos + ?Sized,
    P: ObservadorDePresencia,
{
    // Prioridad elevada: la cola de latencia la pone el planificador, no el codigo.
    // Si el sistema no lo permite se sigue igual, solo sin ese seguro.
    let _ = piano_midi_io::prioridad::elevar_hilo_actual();

    let mut sesion = SesionDeCaptura::nueva();
    sesion.capturando();
    let mut buffer: Vec<EventoCrudo> = Vec::with_capacity(1_024);

    while !parar.load(Ordering::Relaxed) {
        fuente.esperar_hasta(LATIDO);
        buffer.clear();
        if fuente.recoger(&mut buffer) > 0 {
            for ev in buffer.drain(..) {
                estado.enviar(MensajeAlFrontend::Tecla {
                    key: ev.key,
                    pulsada: ev.kind == TipoEvento::Ataque,
                });
            }
        }
        // Clippy propone plegar esto en un guard de `match`. No: el guard tendria que llamar
        // a `notificar_ausencia`, que **muta la cuenta**, y un efecto secundario dentro de un
        // guard se ejecuta aunque el brazo no llegue a entrar. Escrito asi se ve lo que pasa.
        if let Some(presencia) = observador.novedad() {
            let perdido = match presencia {
                Presencia::Presente => {
                    sesion.notificar_presencia();
                    false
                }
                Presencia::Ausente => sesion.notificar_ausencia(),
            };
            if perdido {
                return Desenlace::Perdido;
            }
        }
    }
    Desenlace::Detenido
}
