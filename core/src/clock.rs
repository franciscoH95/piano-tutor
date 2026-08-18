//! La fuente de tiempo, sustituible.
//!
//! El nucleo nunca lee el reloj del sistema directamente: lo recibe. En las pruebas se
//! inyecta un [`VirtualClock`] que avanza solo cuando se le ordena, lo que permite
//! recorrer una pieza de diez minutos en microsegundos y sin una sola prueba
//! intermitente. En la aplicacion se inyecta un [`MonotonicClock`].
//!
//! Se inyecta por generico (`fn f<C: Clock>(clock: &C)`), no como `dyn Clock`, para no
//! pagar despacho dinamico en la ruta critica de tiempo real.

use crate::time::Micros;
use std::time::Instant;

/// Una fuente de tiempo.
///
/// # Contrato
///
/// `now()` **nunca decrece** entre llamadas sucesivas sobre la misma instancia.
pub trait Clock {
    /// Microsegundos transcurridos desde el inicio de la sesion.
    fn now(&self) -> Micros;
}

/// Reloj de pruebas: no avanza solo.
///
/// Es lo que hace deterministas las pruebas de reproduccion. Leerlo no lo mueve.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct VirtualClock {
    now: Micros,
}

impl VirtualClock {
    /// Un reloj detenido en cero.
    #[must_use]
    pub const fn new() -> Self {
        Self { now: Micros::ZERO }
    }

    /// Adelanta el reloj `delta` microsegundos. Satura en lugar de desbordar.
    pub const fn advance(&mut self, delta: Micros) {
        self.now = Micros(self.now.0.saturating_add(delta.0));
    }

    /// Coloca el reloj en un instante concreto.
    ///
    /// Retroceder es un error de la prueba, no un caso de uso: se detecta con
    /// `debug_assert!` para que salte en desarrollo sin coste en produccion.
    pub fn set(&mut self, at: Micros) {
        debug_assert!(at >= self.now, "el reloj virtual no puede retroceder: {:?} -> {:?}", self.now, at);
        self.now = at;
    }
}

impl Clock for VirtualClock {
    #[inline]
    fn now(&self) -> Micros {
        self.now
    }
}

/// Reloj de la aplicacion: monotono, medido desde el inicio de la sesion.
#[derive(Clone, Copy, Debug)]
pub struct MonotonicClock {
    start: Instant,
}

impl MonotonicClock {
    /// Arranca el reloj. El instante de esta llamada es el cero.
    #[must_use]
    pub fn start() -> Self {
        Self { start: Instant::now() }
    }
}

impl Clock for MonotonicClock {
    #[inline]
    fn now(&self) -> Micros {
        // `Instant` ya garantiza monotonia. La saturacion solo se alcanzaria tras
        // ~584.000 anos de sesion.
        Micros(u64::try_from(self.start.elapsed().as_micros()).unwrap_or(u64::MAX))
    }
}
