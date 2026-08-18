//! El ciclo de vida de una captura.
//!
//! Su regla mas importante es negativa: **la perdida del dispositivo nunca se infiere del
//! silencio.** Que no lleguen eventos significa que el alumno no esta tocando, no que se
//! haya desenchufado nada. Solo la declaran una notificacion explicita del sistema o la
//! ausencia del dispositivo en **dos** enumeraciones consecutivas.

use crate::capture::emparejador::{Cierre, Emparejador, InformeDeCaptura, PulsacionCapturada};
use crate::capture::evento::EventoCrudo;
use crate::time::Micros;

/// Ausencias consecutivas necesarias para declarar la perdida.
///
/// Una sola puede ser un parpadeo del sistema al reenumerar. Con el sondeo de un segundo,
/// dos ausencias cumplen el requisito de dos segundos sin falsos positivos.
const AUSENCIAS_PARA_PERDIDA: u8 = 2;

/// En que punto esta la captura.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum EstadoSesion {
    /// Sin dispositivo abierto.
    Inactiva,
    /// Abriendo el dispositivo elegido.
    Abriendo,
    /// Capturando.
    Capturando,
    /// El dispositivo desaparecio.
    Perdida,
    /// No se pudo abrir. Se comunica y la aplicacion sigue.
    Error,
}

/// Una sesion de captura sobre un dispositivo.
pub struct SesionDeCaptura {
    estado: EstadoSesion,
    emparejador: Emparejador,
    /// Instante del ultimo evento **recibido**. Es lo que se usa para sellar las notas
    /// que quedan colgando cuando el dispositivo desaparece: la deteccion llega mas
    /// tarde, y datarlas ahi inventaria duracion.
    ultimo_evento: Micros,
    ausencias: u8,
}

impl Default for SesionDeCaptura {
    fn default() -> Self {
        Self::nueva()
    }
}

impl SesionDeCaptura {
    /// Una sesion sin dispositivo abierto.
    #[must_use]
    pub fn nueva() -> Self {
        Self {
            estado: EstadoSesion::Inactiva,
            emparejador: Emparejador::nuevo(),
            ultimo_evento: Micros::ZERO,
            ausencias: 0,
        }
    }

    /// En que punto esta.
    #[must_use]
    pub const fn estado(&self) -> EstadoSesion {
        self.estado
    }

    /// Que se toleró durante la sesion.
    #[must_use]
    pub const fn informe(&self) -> &InformeDeCaptura {
        self.emparejador.informe()
    }

    /// Se esta abriendo el dispositivo.
    pub fn abriendo(&mut self) {
        self.estado = EstadoSesion::Abriendo;
        self.ausencias = 0;
    }

    /// El dispositivo esta abierto y entregando.
    pub fn capturando(&mut self) {
        self.estado = EstadoSesion::Capturando;
        self.ausencias = 0;
    }

    /// No se pudo abrir. La aplicacion sigue viva.
    pub fn fallo(&mut self) {
        self.estado = EstadoSesion::Error;
    }

    /// Consume un evento y devuelve una nota si alguna queda cerrada.
    pub fn consumir(&mut self, ev: EventoCrudo) -> Option<PulsacionCapturada> {
        self.ultimo_evento = ev.at;
        self.ausencias = 0;
        self.emparejador.consumir(ev)
    }

    /// El dispositivo sigue presente. Reinicia la cuenta de ausencias.
    pub fn notificar_presencia(&mut self) {
        self.ausencias = 0;
    }

    /// El dispositivo no aparece en la enumeracion.
    ///
    /// Devuelve `true` cuando se alcanza la doble confirmacion y la sesion pasa a
    /// [`EstadoSesion::Perdida`]. Quien llama debe entonces recoger las notas colgantes
    /// con [`declarar_perdida`](Self::declarar_perdida).
    pub fn notificar_ausencia(&mut self) -> bool {
        if self.estado != EstadoSesion::Capturando {
            return false;
        }
        self.ausencias = self.ausencias.saturating_add(1);
        if self.ausencias >= AUSENCIAS_PARA_PERDIDA {
            self.estado = EstadoSesion::Perdida;
            true
        } else {
            false
        }
    }

    /// Declara la perdida del dispositivo y cierra lo que quedase hundido.
    ///
    /// Las notas se sellan en el instante del **ultimo evento recibido** y se marcan con
    /// duracion censurada: sabemos cuando empezaron, no cuando terminaron.
    pub fn declarar_perdida(&mut self) -> Vec<PulsacionCapturada> {
        self.estado = EstadoSesion::Perdida;
        self.emparejador.cerrar(self.ultimo_evento, Cierre::PorPerdidaDeDispositivo)
    }

    /// El usuario detiene la captura. Cierra las teclas aun hundidas en `at`.
    pub fn detener(&mut self, at: Micros) -> Vec<PulsacionCapturada> {
        self.estado = EstadoSesion::Inactiva;
        self.emparejador.cerrar(at, Cierre::PorParada)
    }
}
