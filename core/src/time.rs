//! Magnitudes temporales del nucleo.
//!
//! Hay dos, y no son intercambiables:
//!
//! - [`Ticks`] es tiempo **musical**. No cambia si cambia el tempo.
//! - [`Micros`] es tiempo **real** transcurrido. Si cambia el tempo, cambia.
//!
//! Deliberadamente **no** existe `impl Sub` para ninguno de los dos, ni conversion
//! directa entre ellos. Restar es siempre una decision consciente sobre que ocurre por
//! debajo de cero ([`saturating_sub`](Ticks::saturating_sub) o
//! [`checked_sub`](Ticks::checked_sub)), y convertir requiere pasar por
//! [`TempoMap`](crate::tempo::TempoMap), que es el unico que conoce la relacion entre
//! ambos. El compilador impide el resto.

/// Tiempo musical, en pulsos. Invariante al tempo.
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Hash, Default)]
pub struct Ticks(pub u64);

/// Tiempo real transcurrido, en microsegundos.
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Hash, Default)]
pub struct Micros(pub u64);

macro_rules! impl_time {
    ($t:ident) => {
        impl $t {
            /// El valor cero.
            pub const ZERO: Self = Self(0);

            /// Resta saturando en cero en lugar de desbordar.
            #[inline]
            #[must_use]
            pub const fn saturating_sub(self, other: Self) -> Self {
                Self(self.0.saturating_sub(other.0))
            }

            /// Resta, o `None` si el resultado seria negativo.
            #[inline]
            #[must_use]
            pub const fn checked_sub(self, other: Self) -> Option<Self> {
                match self.0.checked_sub(other.0) {
                    Some(v) => Some(Self(v)),
                    None => None,
                }
            }

            /// Suma, o `None` si desborda.
            #[inline]
            #[must_use]
            pub const fn checked_add(self, other: Self) -> Option<Self> {
                match self.0.checked_add(other.0) {
                    Some(v) => Some(Self(v)),
                    None => None,
                }
            }
        }
    };
}

impl_time!(Ticks);
impl_time!(Micros);
