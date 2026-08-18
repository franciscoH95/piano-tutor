//! Prioridad del hilo que consume las pulsaciones.
//!
//! # Por que hace falta
//!
//! La cola de latencia no la pone el transporte: la pone el planificador del sistema. Con
//! el consumidor a prioridad normal, el percentil 99,9 medido se dispara a milisegundos no
//! porque el codigo tarde, sino porque el hilo espera su turno detras de trabajo que no
//! tiene ninguna prisa. Elevarlo es lo que mantiene la cola de la distribucion corta.
//!
//! # Por que vive en este crate y no en el nucleo
//!
//! La tarea original decia ponerlo en `core/src/capture/transporte.rs`, y **eso habria
//! roto el Principio III**: cambiar la prioridad de un hilo es una llamada al sistema, y la
//! crate que la envuelve arrastra `libc` y `log`, dos de las dependencias que la puerta de
//! `cargo tree -p piano-core` prohibe expresamente. Aqui, en la capa de plataforma, no
//! molesta a nadie.
//!
//! # Que NO hace
//!
//! No pide prioridad de **tiempo real**. `ThreadPriority::Max` sube al maximo dentro de la
//! politica de planificacion vigente, que es la normal: un fallo nuestro puede volver la
//! aplicacion lenta, pero no puede dejar la maquina inservible. Para una aplicacion de
//! escritorio que el usuario instala, esa diferencia importa.

use thread_priority::{set_current_thread_priority, ThreadPriority};

/// Por que no se pudo elevar la prioridad.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct NoSePudoElevar;

impl core::fmt::Display for NoSePudoElevar {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "el sistema no permitio elevar la prioridad de este hilo")
    }
}

impl std::error::Error for NoSePudoElevar {}

/// Eleva la prioridad del hilo **actual**, que debe ser el que consume las pulsaciones.
///
/// # Fallar aqui no es grave, y por eso no entra en panico
///
/// Muchos sistemas no dejan subir la prioridad sin privilegios, y un runner de integracion
/// continua casi nunca lo permite. Si falla, la captura sigue funcionando exactamente
/// igual: solo pierde el seguro contra la cola de la distribucion. Quien llama decide si
/// merece la pena decirselo al usuario; lo que no debe es tratarlo como un error fatal.
///
/// # Errores
///
/// [`NoSePudoElevar`] si el sistema lo rechaza.
pub fn elevar_hilo_actual() -> Result<(), NoSePudoElevar> {
    set_current_thread_priority(ThreadPriority::Max).map_err(|_| NoSePudoElevar)
}
