//! El puente entre el nucleo y la interfaz.
//!
//! Aqui **no se toma ninguna decision musical**. Se traducen llamadas y se reenvian datos.
//! Si en este archivo aparece un `if` sobre notas, tempos o manos, esa logica pertenece a
//! `piano-core`, donde si esta cubierta por pruebas.

use serde::Serialize;
use std::sync::Mutex;
use tauri::ipc::Channel;

/// Lo que el nucleo empuja hacia la interfaz.
///
/// **Un solo canal** para todo, discriminado por etiqueta: asi el orden entre las teclas y
/// las anclas queda garantizado por construccion. Con dos canales no lo estaria.
#[derive(Clone, Debug, Serialize)]
#[serde(tag = "tipo", rename_all = "camelCase")]
pub enum MensajeAlFrontend {
    /// Una tecla se pulso o se solto.
    Tecla { key: u8, pulsada: bool },
    /// Cambio el regimen del cursor. **No se envia sesenta veces por segundo**: la
    /// interfaz interpola entre anclas, y eso es lo que mantiene el puente casi vacio.
    Ancla {
        posicion_us: u64,
        instante_us: u64,
        num: u32,
        den: u32,
        tope_us: Option<u64>,
    },
    /// El cursor espera a que se toque esta nota.
    Esperando { key: u8 },
    /// La cancion llego a su fin.
    Terminada,
    /// El teclado desaparecio a mitad de practica.
    DispositivoPerdido,
}

/// Estado compartido de la aplicacion.
#[derive(Default)]
pub struct Estado {
    canal: Mutex<Option<Channel<MensajeAlFrontend>>>,
}

impl Estado {
    /// Empuja un mensaje si hay canal registrado.
    ///
    /// **Nunca se llama desde el hilo de tiempo real.** `send` cuesta hasta 13 ms en el
    /// peor caso, y eso dentro de la ruta critica arruinaria el presupuesto del Principio
    /// IV que la feature 002 dejo cerrado. Quien llama aqui es el hilo reenviador.
    pub fn enviar(&self, mensaje: MensajeAlFrontend) {
        let guarda = match self.canal.lock() {
            Ok(g) => g,
            Err(envenenado) => envenenado.into_inner(),
        };
        if let Some(canal) = guarda.as_ref() {
            let _ = canal.send(mensaje);
        }
    }
}

/// La interfaz registra su canal al arrancar la practica.
#[tauri::command]
pub fn registrar_canal(estado: tauri::State<'_, Estado>, canal: Channel<MensajeAlFrontend>) {
    let mut guarda = match estado.canal.lock() {
        Ok(g) => g,
        Err(envenenado) => envenenado.into_inner(),
    };
    *guarda = Some(canal);
}
