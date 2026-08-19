//! Que pulsacion del alumno corresponde a que nota de la cancion.

use crate::practica::Ancla;
use crate::time::Micros;

/// El primer instante del reloj en que el cursor alcanza `posicion`, o `None` si no la
/// alcanza nunca.
///
/// **Es la inversa exacta de `posicion_en`**, y esa exactitud no es cosmetica: si no lo
/// fuera, el desfase medido no seria el desfase real y todo lo que se construye encima
/// estaria mal desde la base.
///
/// # Por que el techo y no el suelo
///
/// `posicion_en` calcula `p₀ + ⌊(t − t₀)·num/den⌋`. Invertir un suelo da un **techo**:
/// `⌊Δ·num/den⌋ ≥ D ⟺ Δ ≥ ⌈D·den/num⌉`. Con el suelo, el instante esperado saldria
/// sistematicamente pronto y **todos los alumnos parecerian ir tarde**.
///
/// # Por que se calcula aqui y no con `posicion_en`
///
/// Llevar la pulsacion al eje de cancion en vez de la nota al eje del reloj seria mas
/// corto y estaria mal: `posicion_en` **recorta por el tope**, asi que una nota tocada mas
/// alla del final del archivo se proyectaria al final y su tardanza se truncaria en
/// silencio.
///
/// # Por que ni un solo `as`
///
/// El producto `Δ·den` puede llegar a 3,7·10²⁰ con los valores que `Velocidad::nueva`
/// admite y una cancion de 24 horas: cabe en `u128` y no en `u64`. Con `as` eso seria
/// panico en debug y un valor silencioso en release, es decir **dos salidas para la misma
/// entrada segun como se compile**, que es la violacion mas callada del Principio I.
#[must_use]
pub fn instante_de(ancla: &Ancla, posicion: Micros) -> Option<Micros> {
    // El techo del cursor impide llegar mas alla: devolver un instante seria prometer algo
    // que no va a ocurrir.
    if ancla.tope_us.is_some_and(|t| posicion.0 > t.0) {
        return None;
    }
    // El cursor no retrocede: una posicion ya pasada no vuelve a alcanzarse.
    let falta = posicion.0.checked_sub(ancla.posicion_us.0)?;
    if falta == 0 {
        return Some(ancla.instante_us);
    }
    // En pausa la posicion no avanza, asi que ningun instante la alcanza.
    if ancla.num == 0 {
        return None;
    }
    // `div_ceil` dice en su nombre lo que aqui importa: es la division CON TECHO, que es
    // la inversa del suelo que aplica `posicion_en`.
    let dt = u64::try_from(
        (u128::from(falta) * u128::from(ancla.den)).div_ceil(u128::from(ancla.num)),
    )
    .ok()?;
    ancla.instante_us.0.checked_add(dt).map(Micros)
}
