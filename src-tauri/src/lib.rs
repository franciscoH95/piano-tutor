//! Aplicacion de escritorio de Piano Tutor.

pub mod comandos;
pub mod preferencias;
pub mod reenviador;

use piano_core::clock::MonotonicClock;

/// El reloj de sesion.
///
/// Se crea **una sola vez** y lo comparten la captura y la reproduccion. Es lo que hace
/// que los instantes de lo que toca el alumno y los de lo que deberia tocar sean
/// directamente comparables, sin conversion ni correccion (FR-012a).
///
/// Dos relojes arrancados por separado empezarian ambos en cero pero en momentos
/// distintos, y la diferencia constante entre ellos apareceria mas tarde como "el alumno
/// siempre llega tarde". Ese error compilaria y pasaria todas las pruebas de cada lado por
/// separado.
pub struct RelojDeSesion(pub MonotonicClock);

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // NO tocar MIDI antes de esta linea: el reloj de sesion debe existir antes que
    // cualquier captura o reproduccion, porque las dos tienen que recibir EL MISMO.
    let reloj = RelojDeSesion(MonotonicClock::start());

    tauri::Builder::default()
        .manage(reloj)
        .manage(std::sync::Arc::new(comandos::Estado::default()))
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            greet,
            comandos::registrar_canal,
            comandos::abrir_cancion,
            comandos::ajustar_corte,
            comandos::vista_actual,
            comandos::transporte_marcha,
            comandos::transporte_pausa,
            comandos::transporte_saltar,
            comandos::transporte_velocidad,
            comandos::transporte_posicion,
            comandos::conectar_teclado
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
