//! Vigilar si el teclado sigue ahi.
//!
//! # Por que sondeo y no solo notificaciones
//!
//! CoreMIDI ofrece notificaciones de conexion y desconexion
//! (`Client::new_with_notifications`), pero exigen dos cosas incomodas: que el cliente de
//! notificaciones se cree **antes que cualquier otro cliente MIDI del proceso**, y que
//! corra en un hilo con un `CFRunLoop` activo —en Tauri, el principal—. Es una dependencia
//! de orden invisible que no da error si se rompe: simplemente deja de llegar nada.
//!
//! Por eso el mecanismo **de base** es el sondeo cada segundo, que funciona siempre y en
//! cualquier hilo. Las notificaciones, cuando la aplicacion pueda garantizar el orden de
//! arranque, son un acelerador que se suma; no la unica via.
//!
//! Este vigia informa de **hechos** —esta o no esta—, no de conclusiones. Quien decide si
//! eso es una perdida es [`SesionDeCaptura`](piano_core::capture::SesionDeCaptura), con su
//! regla de doble confirmacion. Separarlo asi es lo que permite probar la decision sin
//! hardware y el hecho con hardware virtual.

use piano_core::capture::{reconocer, Dispositivo, Reconocimiento};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::sync::Arc;
use std::time::Duration;

/// Intervalo de sondeo. Con la doble confirmacion de la sesion, dos sondeos cumplen el
/// requisito de detectar la perdida en menos de dos segundos.
pub const INTERVALO_SONDEO: Duration = Duration::from_millis(1_000);

/// Lo que el vigia observa en cada sondeo.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Presencia {
    /// El dispositivo aparece en la enumeracion.
    Presente,
    /// No aparece. **No** significa todavia que se haya perdido.
    Ausente,
}

/// Vigila un dispositivo concreto en segundo plano.
pub struct Vigia {
    rx: Receiver<Presencia>,
    parar: Arc<AtomicBool>,
}

impl Vigia {
    /// Empieza a vigilar. El hilo se detiene al soltar el vigia.
    #[must_use]
    pub fn nuevo(objetivo: Dispositivo) -> Self {
        Self::con_intervalo(objetivo, INTERVALO_SONDEO)
    }

    /// Igual, con un intervalo a medida. Existe para que las pruebas no tarden segundos.
    #[must_use]
    pub fn con_intervalo(objetivo: Dispositivo, intervalo: Duration) -> Self {
        let (tx, rx) = mpsc::channel();
        let parar = Arc::new(AtomicBool::new(false));
        let bandera = Arc::clone(&parar);
        std::thread::spawn(move || {
            while !bandera.load(Ordering::Relaxed) {
                let presencia = match crate::dispositivos() {
                    Ok(lista) => match reconocer(&objetivo, &lista) {
                        Reconocimiento::Encontrado(_) => Presencia::Presente,
                        Reconocimiento::PedirAlUsuario => Presencia::Ausente,
                    },
                    Err(_) => Presencia::Ausente,
                };
                if tx.send(presencia).is_err() {
                    return; // nadie escucha: el vigia se solto
                }
                std::thread::sleep(intervalo);
            }
        });
        Self { rx, parar }
    }

    /// Lo ultimo observado, si hay algo nuevo. No bloquea.
    pub fn novedad(&mut self) -> Option<Presencia> {
        let mut ultima = None;
        loop {
            match self.rx.try_recv() {
                Ok(p) => ultima = Some(p),
                Err(TryRecvError::Empty | TryRecvError::Disconnected) => return ultima,
            }
        }
    }
}

impl Drop for Vigia {
    fn drop(&mut self) {
        self.parar.store(true, Ordering::Relaxed);
    }
}
