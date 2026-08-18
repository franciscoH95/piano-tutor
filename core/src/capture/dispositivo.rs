//! Quien es cada teclado, y como se le vuelve a reconocer la proxima sesion.
//!
//! El indice de puerto **no se usa jamas**: se renumera al conectar o desconectar
//! cualquier otro aparato, y abrir el dispositivo equivocado sin avisar es peor que no
//! abrir ninguno. La identidad es el identificador que asigna el sistema operativo, con
//! la pareja (nombre, posicion entre homonimos) como reserva.

use std::fmt;

/// Identificador que asigna el sistema operativo.
///
/// En macOS es el `unique_id` de CoreMIDI; en Windows, la ruta de interfaz del
/// dispositivo. Ambos sobreviven a renumeraciones y a mover el teclado de conector.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
pub struct DeviceId(pub u64);

/// Un teclado disponible.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Dispositivo {
    /// Identidad primaria, si la plataforma la ofrece.
    pub id_sistema: Option<DeviceId>,
    /// Nombre legible. Nunca vacio: si el sistema no da ninguno, se genera una etiqueta.
    pub nombre: String,
    /// Posicion entre los dispositivos que anuncian **este mismo** nombre.
    pub posicion: u16,
}

/// El resultado de buscar el teclado recordado entre los disponibles.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Reconocimiento {
    /// Es el que ocupa esta posicion en la lista de disponibles.
    Encontrado(usize),
    /// No hay forma segura de saber cual es. Hay que preguntar.
    PedirAlUsuario,
}

/// Por que no se pudo empezar a capturar.
///
/// Ninguna de estas situaciones interrumpe la aplicacion: todas se comunican y se sigue.
#[non_exhaustive]
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum ErrorDeEntrada {
    /// No hay ningun teclado conectado.
    SinDispositivos,
    /// El dispositivo existe pero el sistema no dejo abrirlo.
    NoSePudoAbrir {
        /// Nombre del dispositivo.
        nombre: String,
    },
    /// Otra aplicacion lo tiene tomado.
    EnUsoPorOtraAplicacion {
        /// Nombre del dispositivo.
        nombre: String,
    },
}

impl fmt::Display for ErrorDeEntrada {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SinDispositivos => write!(f, "no hay ningun teclado MIDI conectado"),
            Self::NoSePudoAbrir { nombre } => write!(f, "no se pudo abrir «{nombre}»"),
            Self::EnUsoPorOtraAplicacion { nombre } => {
                write!(f, "«{nombre}» lo esta usando otra aplicacion")
            }
        }
    }
}

impl std::error::Error for ErrorDeEntrada {}

/// Convierte lo que entrega la plataforma en dispositivos con identidad completa.
///
/// Asigna la posicion de cada uno entre los que comparten su mismo nombre, y sustituye
/// los nombres vacios por una etiqueta legible: un dispositivo sin nombre seguiria siendo
/// elegible por el usuario, pero no seria distinguible en una lista.
#[must_use]
pub fn catalogar(crudos: &[(Option<u64>, &str)]) -> Vec<Dispositivo> {
    let mut vistos: Vec<(String, u16)> = Vec::new();
    let mut salida = Vec::with_capacity(crudos.len());
    for (i, (id, nombre)) in crudos.iter().enumerate() {
        let nombre = if nombre.trim().is_empty() {
            match id {
                Some(v) => format!("Teclado MIDI {v}"),
                None => format!("Teclado MIDI en la posicion {i}"),
            }
        } else {
            (*nombre).to_string()
        };
        let posicion = match vistos.iter_mut().find(|(n, _)| *n == nombre) {
            Some((_, cuenta)) => {
                *cuenta = cuenta.saturating_add(1);
                *cuenta
            }
            None => {
                vistos.push((nombre.clone(), 0));
                0
            }
        };
        salida.push(Dispositivo { id_sistema: id.map(DeviceId), nombre, posicion });
    }
    salida
}

/// Busca el teclado recordado entre los disponibles.
///
/// El orden es normativo (FR-004b): primero el identificador del sistema, que sobrevive a
/// renumeraciones y a cambiar de conector; despues la pareja (nombre, posicion). Si
/// ninguno casa, **se pregunta**: nunca se abre el que mas se parezca.
#[must_use]
pub fn reconocer(recordado: &Dispositivo, disponibles: &[Dispositivo]) -> Reconocimiento {
    if let Some(id) = recordado.id_sistema {
        if let Some(i) = disponibles.iter().position(|d| d.id_sistema == Some(id)) {
            return Reconocimiento::Encontrado(i);
        }
    }
    match disponibles
        .iter()
        .position(|d| d.nombre == recordado.nombre && d.posicion == recordado.posicion)
    {
        Some(i) => Reconocimiento::Encontrado(i),
        None => Reconocimiento::PedirAlUsuario,
    }
}
