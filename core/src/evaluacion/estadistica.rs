//! Mediana y cuartiles, en aritmetica entera.
//!
//! Se eligieron frente a la media y la desviacion tipica por una razon concreta: el nucleo
//! prohibe la coma flotante, y una desviacion tipica exigiria una raiz cuadrada. Con ella
//! haria falta una excepcion al Principio III o una aproximacion, y una aproximacion haria
//! el resultado dependiente de la implementacion, rompiendo SC-005.
//!
//! # La trampa del signo
//!
//! **La division entera de Rust trunca hacia CERO, no hacia abajo.** `(-3 + -2) / 2` da
//! `-2`, no `-3`. Sobre desfases —que son negativos cuando el alumno se adelanta— eso
//! sesgaria la mediana hacia cero, es decir, hacia «va menos adelantado de lo que va». Aqui
//! se corrige explicitamente.

/// Media de dos enteros con signo, **redondeando hacia abajo** y sin desbordar.
///
/// Sin desbordar porque `a + b` puede pasarse de `i64` con desfases extremos; se suma la
/// mitad de la diferencia al menor, que siempre cabe.
/// `i64::midpoint` no sirve: redondea hacia cero, que es justo el sesgo que hay que evitar.
const fn media_hacia_abajo(a: i64, b: i64) -> i64 {
    let (menor, mayor) = if a < b { (a, b) } else { (b, a) };
    // Los dos `as` de aqui son seguros y no se pueden evitar sin una dependencia: la
    // diferencia de dos `i64` no cabe en `i64` pero SI en `u64` como patron de bits, que es
    // exactamente lo que produce `wrapping_sub`. Su mitad siempre cabe de vuelta en `i64`
    // porque es como mucho la mitad del rango, y sumarla al menor no puede pasarse del
    // mayor. Es el unico sitio del modulo donde aparece un `as`, y por eso lleva esta nota.
    #[allow(clippy::cast_possible_wrap, clippy::cast_sign_loss)]
    let mitad = ((mayor.wrapping_sub(menor) as u64) / 2) as i64;
    menor.wrapping_add(mitad)
}

/// El valor que ocupa la posicion `k` de la lista ordenada.
fn en_posicion(ordenados: &[i64], k: usize) -> Option<i64> {
    ordenados.get(k).copied()
}

/// La mediana. `None` si no hay muestras.
///
/// Con un numero par de elementos devuelve la media de los dos centrales, **redondeada
/// hacia abajo tambien con negativos**.
#[must_use]
pub fn mediana(muestras: &[i64]) -> Option<i64> {
    if muestras.is_empty() {
        return None;
    }
    let mut v = muestras.to_vec();
    // `sort_unstable` sobre enteros es un orden total sin empates ambiguos: dos valores
    // iguales son indistinguibles, asi que el resultado no depende del orden de entrada.
    v.sort_unstable();
    let n = v.len();
    if n % 2 == 1 {
        en_posicion(&v, n / 2)
    } else {
        let a = en_posicion(&v, n / 2 - 1)?;
        let b = en_posicion(&v, n / 2)?;
        Some(media_hacia_abajo(a, b))
    }
}

/// El primer y el tercer cuartil. `None` si no hay muestras.
///
/// Con una sola muestra los dos coinciden con ella: la dispersion es cero, que es la
/// respuesta correcta y no un error.
#[must_use]
pub fn cuartiles(muestras: &[i64]) -> Option<(i64, i64)> {
    if muestras.is_empty() {
        return None;
    }
    let mut v = muestras.to_vec();
    v.sort_unstable();
    let n = v.len();
    // Metodo del orden, el mas simple y totalmente determinista: la posicion sale de una
    // division entera, sin interpolar. Interpolar exigiria fracciones, y aqui no las hay.
    let q1 = en_posicion(&v, n / 4)?;
    let q3 = en_posicion(&v, ((n * 3) / 4).min(n - 1))?;
    Some((q1, q3))
}
