//! El puente entre el nucleo y la interfaz.
//!
//! Aqui **no se toma ninguna decision musical**. Se traducen llamadas y se reenvian datos.
//! Si en este archivo aparece un `if` sobre notas, tempos o manos, esa logica pertenece a
//! `piano-core`, donde si esta cubierta por pruebas.

use piano_core::practica::{Alteracion, Base, NotaDetallada, Preparacion, Reparto};
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

/// Lo que la interfaz necesita saber de una cancion recien abierta.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResumenCancion {
    /// Numero de notas.
    pub notas: usize,
    /// Duracion total en microsegundos.
    pub duracion_us: u64,
    /// Si el archivo traia las dos manos separadas. La interfaz lo usa para rotular el
    /// control del corte, que sigue visible en ambos casos.
    pub voces_del_archivo: bool,
    /// Punto de corte vigente.
    pub corte: u8,
}

/// Una nota lista para dibujar, aplanada para cruzar el puente.
///
/// El nombre viaja ya separado en base y alteracion, que es como lo produce el nucleo. No
/// se formatea aqui: el texto es cosa de quien pinta.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NotaVisiblePlana {
    pub indice: usize,
    pub key: u8,
    pub onset_us: u64,
    pub end_us: u64,
    /// `true` si es de la mano derecha.
    pub derecha: bool,
    /// Dedo propuesto, de 1 a 5.
    pub dedo: u8,
    /// Letra de la nota: 0 = Do, 1 = Re, ... 6 = Si.
    pub base: u8,
    /// -1 bemol, 0 natural, 1 sostenido.
    pub alteracion: i8,
    /// Situacion respecto a lo tocado.
    pub estado: &'static str,
}

impl From<&NotaDetallada> for NotaVisiblePlana {
    fn from(d: &NotaDetallada) -> Self {
        Self {
            indice: d.indice,
            key: d.key,
            onset_us: d.onset_us,
            end_us: d.end_us,
            derecha: d.mano == piano_core::practica::Mano::Derecha,
            dedo: d.dedo.numero(),
            // Mapeo explicito y no `as`: un `as` sobre el enum ata este contrato al
            // ORDEN de las variantes, y reordenarlas cambiaria en silencio lo que ve la
            // interfaz. Con `Bemol` en tercera posicion, `as i8` daba 2 en vez de -1.
            base: match d.nombre.base {
                Base::Do => 0,
                Base::Re => 1,
                Base::Mi => 2,
                Base::Fa => 3,
                Base::Sol => 4,
                Base::La => 5,
                Base::Si => 6,
            },
            alteracion: match d.nombre.alteracion {
                Alteracion::Bemol => -1,
                Alteracion::Ninguna => 0,
                Alteracion::Sostenido => 1,
            },
            estado: match d.estado {
                piano_core::practica::EstadoNota::Pendiente => "pendiente",
                piano_core::practica::EstadoNota::Sonando => "sonando",
                piano_core::practica::EstadoNota::Acertada => "acertada",
                piano_core::practica::EstadoNota::Omitida => "omitida",
            },
        }
    }
}

/// Estado compartido de la aplicacion.
#[derive(Default)]
pub struct Estado {
    canal: Mutex<Option<Channel<MensajeAlFrontend>>>,
    preparacion: Mutex<Option<Preparacion>>,
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

/// Abre una cancion del disco.
///
/// **Leer del disco es cosa de esta capa**, no del nucleo, que sigue recibiendo `&[u8]` y
/// por eso se puede probar sin tocar el sistema de archivos (Principio III).
///
/// El motivo del fallo se devuelve tal cual lo da el nucleo: la interfaz lo muestra sin
/// interpretarlo (FR-004).
#[tauri::command]
pub fn abrir_cancion(
    estado: tauri::State<'_, std::sync::Arc<Estado>>,
    ruta: String,
) -> Result<ResumenCancion, String> {
    let bytes = std::fs::read(&ruta).map_err(|e| format!("No se pudo leer el archivo: {e}"))?;
    let cancion = piano_core::load_smf(&bytes).map_err(|e| e.to_string())?;
    let preparacion = Preparacion::nueva(cancion);
    let resumen = ResumenCancion {
        notas: preparacion.cancion().notes().len(),
        duracion_us: preparacion.cancion().duration_us().0,
        voces_del_archivo: preparacion.reparto().origen() == Reparto::VocesDelArchivo,
        corte: preparacion.corte(),
    };
    let mut guarda = match estado.preparacion.lock() {
        Ok(g) => g,
        Err(envenenado) => envenenado.into_inner(),
    };
    // Se sustituye entera: cargar otra cancion no arrastra nada de la anterior (FR-005).
    *guarda = Some(preparacion);
    Ok(resumen)
}

/// Mueve el punto de corte entre manos. Rehace manos **y** digitacion (FR-003c).
#[tauri::command]
pub fn ajustar_corte(estado: tauri::State<'_, std::sync::Arc<Estado>>, corte: u8) {
    let mut guarda = match estado.preparacion.lock() {
        Ok(g) => g,
        Err(envenenado) => envenenado.into_inner(),
    };
    if let Some(p) = guarda.as_mut() {
        p.ajustar_corte(corte);
    }
}

/// Las notas que caen en la ventana pedida, listas para dibujar.
#[tauri::command]
pub fn vista_actual(
    estado: tauri::State<'_, std::sync::Arc<Estado>>,
    desde_us: u64,
    hasta_us: u64,
) -> Vec<NotaVisiblePlana> {
    let mut guarda = match estado.preparacion.lock() {
        Ok(g) => g,
        Err(envenenado) => envenenado.into_inner(),
    };
    let Some(p) = guarda.as_mut() else {
        return Vec::new();
    };
    let mut detalle = Vec::new();
    p.detallar(desde_us, hasta_us, &mut detalle);
    detalle.iter().map(NotaVisiblePlana::from).collect()
}
