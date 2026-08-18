//! La tabla de vanos de Parncutt et al. (1997), como DATOS.
//!
//! Van aqui, separadas de las reglas, para que ajustar un umbral sea cambiar un numero y
//! no tocar un condicional. Los valores son de la Tabla 1 del articulo original, en
//! semitonos y para la mano derecha; la izquierda usa la misma tabla sobre alturas
//! reflejadas.

/// Los seis umbrales de un par de dedos, en semitonos.
#[derive(Clone, Copy, Debug)]
pub struct Umbrales {
    /// Por debajo, el par es impracticable.
    pub min_prac: i32,
    /// Por debajo, incomodo.
    pub min_comf: i32,
    /// Extremo inferior del rango relajado.
    pub min_rel: i32,
    /// Extremo superior del rango relajado.
    pub max_rel: i32,
    /// Por encima, incomodo.
    pub max_comf: i32,
    /// Por encima, impracticable.
    pub max_prac: i32,
}

/// Todos los umbrales a cero: el mismo dedo consigo mismo.
const CERO: Umbrales =
    Umbrales { min_prac: 0, min_comf: 0, min_rel: 0, max_rel: 0, max_comf: 0, max_prac: 0 };

#[rustfmt::skip]
const TABLA: [(u8, u8, Umbrales); 10] = [
    (1, 2, Umbrales { min_prac: -5, min_comf: -3, min_rel: 1, max_rel:  5, max_comf:  8, max_prac: 10 }),
    (1, 3, Umbrales { min_prac: -4, min_comf: -2, min_rel: 3, max_rel:  7, max_comf: 10, max_prac: 12 }),
    (1, 4, Umbrales { min_prac: -3, min_comf: -1, min_rel: 5, max_rel:  9, max_comf: 12, max_prac: 14 }),
    (1, 5, Umbrales { min_prac: -1, min_comf:  1, min_rel: 7, max_rel: 10, max_comf: 13, max_prac: 15 }),
    (2, 3, Umbrales { min_prac:  1, min_comf:  1, min_rel: 1, max_rel:  2, max_comf:  3, max_prac:  5 }),
    (2, 4, Umbrales { min_prac:  1, min_comf:  1, min_rel: 3, max_rel:  4, max_comf:  5, max_prac:  7 }),
    (2, 5, Umbrales { min_prac:  2, min_comf:  2, min_rel: 5, max_rel:  6, max_comf:  8, max_prac: 10 }),
    (3, 4, Umbrales { min_prac:  1, min_comf:  1, min_rel: 1, max_rel:  2, max_comf:  2, max_prac:  4 }),
    (3, 5, Umbrales { min_prac:  1, min_comf:  1, min_rel: 3, max_rel:  4, max_comf:  5, max_prac:  7 }),
    (4, 5, Umbrales { min_prac:  1, min_comf:  1, min_rel: 1, max_rel:  2, max_comf:  3, max_prac:  5 }),
];

/// Los umbrales de un par de dedos, en orden canonico (menor primero).
#[must_use]
pub fn umbrales(menor: u8, mayor: u8) -> Umbrales {
    if menor == mayor {
        return CERO;
    }
    let (a, b) = if menor < mayor { (menor, mayor) } else { (mayor, menor) };
    let mut i = 0;
    while i < TABLA.len() {
        match TABLA.get(i) {
            Some((x, y, u)) if *x == a && *y == b => return *u,
            _ => i += 1,
        }
    }
    CERO
}

/// `true` si esa altura MIDI cae en una tecla negra.
///
/// Se consulta siempre sobre la altura **real**, nunca sobre la reflejada: reflejar sirve
/// para la geometria de la mano, no para el color del teclado.
#[must_use]
pub const fn es_negra(key: u8) -> bool {
    matches!(key % 12, 1 | 3 | 6 | 8 | 10)
}
