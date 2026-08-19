//! El adaptador para las plataformas cuyo backend todavia no existe.
//!
//! Hoy solo hay backend de CoreMIDI. Sin este modulo, `piano_midi_io::dispositivos` y
//! `piano_midi_io::abrir` **no existen** fuera de macOS y la aplicacion entera deja de
//! compilar alli: es exactamente lo que ocurria, y no se noto porque no hay ninguna maquina
//! Windows a mano para probarlo.
//!
//! La alternativa era llenar `src-tauri` de `#[cfg]`. Se descarta: la capa de plataforma
//! existe precisamente para absorber esto, y el puente no debe saber en que sistema corre.
//!
//! **No finge funcionar.** Informa de que no hay teclados, que es la verdad, y la
//! aplicacion sigue siendo util: la cancion se ve y se reproduce igual, y la falta de
//! teclado se comunica sin bloquear nada (FR-015). Cuando exista el backend de Windows,
//! este modulo desaparece.

use piano_core::capture::{canal, Dispositivo, ErrorDeEntrada, Receptor};
use piano_core::clock::Clock;

/// Una captura en curso. En esta plataforma **nunca llega a existir**: `abrir` siempre
/// falla. El tipo se declara para que la firma sea la misma en todas partes.
pub struct Captura {
    receptor: Receptor,
}

impl Captura {
    /// El extremo de lectura.
    pub fn receptor(&mut self) -> &mut Receptor {
        &mut self.receptor
    }

    /// Libera el dispositivo.
    pub fn cerrar(self) {
        drop(self);
    }

    /// Sin backend no hay actividad que confirmar.
    pub fn confirmar_actividad(&mut self, _ventana: std::time::Duration) -> bool {
        false
    }
}

/// Enumera los teclados. Sin backend, ninguno.
///
/// Devuelve la lista vacia y no un error: no hay ningun fallo que comunicar, simplemente
/// esta plataforma todavia no sabe mirar.
pub fn dispositivos() -> Result<Vec<Dispositivo>, ErrorDeEntrada> {
    Ok(Vec::new())
}

/// Abre un dispositivo. Sin backend, siempre falla.
pub fn abrir<C>(_dispositivo: &Dispositivo, _clock: C) -> Result<Captura, ErrorDeEntrada>
where
    C: Clock + Send + 'static,
{
    // Se construye el canal y se descarta a proposito: mantiene el tipo `Captura` habitado
    // y demuestra que la firma es la misma que la del backend real.
    let (_emisor, _receptor) = canal(1);
    Err(ErrorDeEntrada::SinDispositivos)
}

/// Reabre un dispositivo tras perderlo. Sin backend, siempre falla.
pub fn reabrir<C>(dispositivo: &Dispositivo, clock: C) -> Result<Captura, ErrorDeEntrada>
where
    C: Clock + Send + 'static,
{
    abrir(dispositivo, clock)
}
