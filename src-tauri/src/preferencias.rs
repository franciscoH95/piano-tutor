//! Lo unico de esta feature que toca el disco: que teclado eligio el usuario.
//!
//! Lo que el alumno toca **no** se guarda (FR-026). Aqui solo vive su eleccion, para que
//! elegir sea un tramite de una sola vez y no de cada sesion.

use piano_core::capture::{DeviceId, Dispositivo};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// El teclado recordado, con las dos identidades: la del sistema y la de reserva.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct TecladoRecordado {
    /// Identificador que asigna el sistema operativo. Es la identidad primaria.
    pub id_sistema: Option<u64>,
    /// Nombre legible del puerto.
    pub nombre: String,
    /// Posicion entre los homonimos. Identidad de reserva.
    pub posicion: u16,
}

impl From<&Dispositivo> for TecladoRecordado {
    fn from(d: &Dispositivo) -> Self {
        Self { id_sistema: d.id_sistema.map(|i| i.0), nombre: d.nombre.clone(), posicion: d.posicion }
    }
}

impl From<&TecladoRecordado> for Dispositivo {
    fn from(t: &TecladoRecordado) -> Self {
        Self { id_sistema: t.id_sistema.map(DeviceId), nombre: t.nombre.clone(), posicion: t.posicion }
    }
}

/// Guarda la eleccion. Un fallo al escribir no debe tumbar la aplicacion: como mucho, la
/// proxima vez habra que volver a elegir.
pub fn guardar(ruta: &Path, teclado: &TecladoRecordado) -> std::io::Result<()> {
    if let Some(dir) = ruta.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let json = serde_json::to_string_pretty(teclado)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    std::fs::write(ruta, json)
}

/// Lee la eleccion anterior, si la hay y es legible.
///
/// Un archivo corrupto se trata como "no hay nada recordado": se vuelve a preguntar, que
/// es el comportamiento seguro. Nunca se abre un dispositivo a partir de datos dudosos.
#[must_use]
pub fn cargar(ruta: &Path) -> Option<TecladoRecordado> {
    let texto = std::fs::read_to_string(ruta).ok()?;
    serde_json::from_str(&texto).ok()
}
