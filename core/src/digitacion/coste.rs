//! Cuanto "cuesta" un gesto de la mano.
//!
//! Las reglas vienen del modelo ergonomico de Parncutt et al. (1997). Aqui se implementan
//! las que gobiernan la digitacion de escalas y pasajes, que es lo que esta feature
//! necesita; las que tratan acordes de mas de cinco notas y saltos extremos quedan fuera y
//! se declara que quedan fuera.
//!
//! Todo en `i32`: el nucleo prohibe la coma flotante, y ademas la aritmetica entera es lo
//! que hace la propuesta **identica** en cada ejecucion, que es lo que exige SC-010.

use crate::digitacion::tablas::{es_negra, umbrales};

/// Penalizacion por un gesto impracticable. Alta, pero **finita**: el sistema debe
/// proponer siempre algo, aunque sea lo menos malo (SC-009).
pub const IMPRACTICABLE: i32 = 10_000;

/// Coste de recolocar la mano (Regla 4). Es el termino dominante en la digitacion de
/// escalas: lo que distingue una digitacion de manual de sus alternativas es que recoloca
/// **una vez por octava** en vez de dos o tres.
///
/// El valor esta calibrado contra las escalas canonicas de Do, Sol, Fa, Re y Si mayor en
/// las dos manos, que es como se validan estos modelos: Parncutt ajusta los suyos contra
/// digitaciones de expertos. No es una constante deducida, es una constante **ajustada**, y
/// las escalas de `digitacion_test.rs` son su banco de pruebas.
const RECOLOCACION: i32 = 6;

/// Mas alla de esto el pulgar no llega por debajo de la mano y el cruce no se puede hacer.
const CRUCE_MAX: i32 = 12;

/// Coste que depende **solo de la apertura** entre dos dedos. No recibe las teclas a
/// proposito: nada de lo que hay aqui puede depender de su color.
///
/// Existe separada porque en un acorde hay que evaluar **todos** los pares, y las reglas
/// que hablan de una nota —el pulgar en negra, el meñique en negra— no pueden cobrarse una
/// vez por par: en una septima el meñique aparece en tres pares y se cobraba tres veces.
#[must_use]
pub fn coste_vano(dedo_a: u8, dedo_b: u8, vano: i32) -> i32 {
    if dedo_a == dedo_b {
        return 0;
    }
    let mut c = 0;
    let u = umbrales(dedo_a.min(dedo_b), dedo_a.max(dedo_b));
    let cruce = (dedo_a == 1 || dedo_b == 1) && vano < 0;

    if cruce {
        // **La geometria es otra.** Durante el cruce la mano rota y el pulgar pasa por
        // debajo, asi que el termino cuadratico de incomodidad —que describe la apertura
        // de una mano en reposo— no aplica: era el que hacia costar 16 el paso `4->1`
        // desde el si bemol del Fa mayor, que es el gesto normal de esa escala.
        //
        // El termino lineal si se conserva, y no por inercia: en la tabla el alcance
        // relajado del pulgar es 1 con el indice, 3 con el corazon y 5 con el anular, que
        // es justo "pasar por debajo del anular cuesta mas que por debajo del corazon".
        // Eso es cierto de la mano, no de la postura, y es lo que decide que Do mayor
        // agrupe 3+5 mientras Fa mayor agrupa 4+4.
        //
        // El limite de lo practicable tambien vale aqui, y dice algo que ninguna otra
        // regla dice: para el par (1,5) es -1, es decir, **el pulgar no pasa por debajo
        // del meñique**. Es corto y esta por fuera; ese gesto no existe.
        if vano < u.min_prac || vano.saturating_abs() > CRUCE_MAX {
            c += IMPRACTICABLE;
        } else if vano < u.min_rel {
            c += u.min_rel - vano;
        }
        // Regla 12 (thumb-passing) y Regla 4 (position-change): cruzar es un gesto, y al
        // cruzar la mano no se estira, se **recoloca**.
        c += 1 + RECOLOCACION;
        // **No** se penaliza pasar por debajo de un dedo apoyado en negra. Es el gesto
        // normal de toda escala con sostenidos —en Re mayor el pulgar pasa bajo el fa♯—, y
        // cobrarlo expulsaba justo esas escalas de la solucion. Lo incomodo es que el
        // pulgar **aterrice** en negra, y de eso ya se ocupan `coste_nota` y la regla de
        // transicion, cada una una vez.
    } else {
        // Regla de estiramiento: fuera del rango comodo cuesta, y fuera del practicable
        // cuesta muchisimo.
        if vano < u.min_prac || vano > u.max_prac {
            c += IMPRACTICABLE;
        }
        if vano < u.min_comf {
            c += 2 * (u.min_comf - vano);
        } else if vano > u.max_comf {
            c += 2 * (vano - u.max_comf);
        }
        // Dentro de lo practicable pero fuera de lo relajado: molesto, no imposible.
        if vano < u.min_rel {
            c += u.min_rel - vano;
        } else if vano > u.max_rel {
            c += vano - u.max_rel;
        }
    }
    c
}

/// Coste de pasar de un dedo a otro entre dos notas **consecutivas**.
#[must_use]
pub fn coste_transicion(dedo_a: u8, key_a: u8, dedo_b: u8, key_b: u8, vano: i32) -> i32 {
    let mut c = coste_vano(dedo_a, dedo_b, vano);

    // Repetir dedo en dos notas distintas obliga a saltar: se penaliza fuerte, pero no se
    // prohibe (una nota repetida si puede llevar el mismo dedo).
    if dedo_a == dedo_b && key_a != key_b {
        c += 200;
    }

    // Paso del pulgar por debajo de una tecla negra: es el gesto mas incomodo del piano.
    if (dedo_a == 1 && es_negra(key_a)) || (dedo_b == 1 && es_negra(key_b)) {
        c += 12;
    }
    // Meniique en negra: menos grave, pero se evita.
    if (dedo_a == 5 && es_negra(key_a)) || (dedo_b == 5 && es_negra(key_b)) {
        c += 4;
    }
    c
}

/// Coste de usar un dedo en una nota de una **linea**, una detras de otra.
#[must_use]
pub fn coste_nota(dedo: u8, key: u8) -> i32 {
    let mut c = 0;
    // Regla 6 (weak-finger): el anular y el meniique tienen menos control independiente.
    // Parncutt los pesa **igual**; ponderar el 4 por encima del 5 hacia que el optimo
    // evitase el final 3-4-5 de las escalas, que es justo la digitacion de manual.
    if dedo == 4 || dedo == 5 {
        c += 1;
    }
    // El pulgar en negra se evita siempre que haya alternativa.
    if dedo == 1 && es_negra(key) {
        c += 8;
    }
    // El meñique en negra lo cobra `coste_transicion`, no aqui: en una linea melodica cada
    // nota participa en dos transiciones como mucho, asi que cobrarlo tambien por nota
    // seria contarlo dos veces.
    c
}

/// Coste de usar un dedo en una nota que **se sostiene a la vez** que otras.
///
/// La diferencia con `coste_nota` es que aqui **no se aplica la regla del dedo debil**. La
/// Regla 6 habla del control independiente al mover un dedo, que es un problema de pasajes;
/// en un acorde el meñique solo se apoya. Aplicarla dentro del acorde hacia que una triada
/// de Do mayor saliese 1-2-3 —la mano encogida— en vez de 1-3-5, porque el punto que cuesta
/// usar el meñique cancelaba exactamente el punto que cuesta forzar el par 2-3.
#[must_use]
pub fn coste_nota_simultanea(dedo: u8, key: u8) -> i32 {
    // El pulgar en negra se evita siempre que haya alternativa, suene solo o acompañado.
    if dedo == 1 && es_negra(key) {
        return 8;
    }
    // Meñique en negra: menos grave, pero se evita. Aqui **una vez por nota**, no una vez
    // por par, que es lo que hacia que una septima con si bemol arriba prefiriese el
    // anular al meñique.
    if dedo == 5 && es_negra(key) {
        return 4;
    }
    0
}

/// Coste de encadenar tres dedos. Captura lo que un par no ve.
#[must_use]
pub fn coste_terceto(a: u8, b: u8, c_: u8) -> i32 {
    // 3-4-5 seguidos es una secuencia debil y torpe.
    // Peso 1, el del articulo: 3-4-5 es torpe, pero es el final normal de una escala y
    // penalizarlo mas lo expulsaba de la solucion optima.
    if (a, b, c_) == (3, 4, 5) || (a, b, c_) == (5, 4, 3) {
        return 1;
    }
    0
}

/// Coste de la forma de la mano en un acorde, que se sostiene entero a la vez.
///
/// Las notas extremas van a los dedos extremos: asi la mano **abarca** el acorde en vez de
/// quedarse dentro de el. Un dedo que sobra por fuera es un dedo sin sitio donde ir.
///
/// Es lo que separa 1-3-5 de 1-2-4 en una triada: los dos reparten los mismos intervalos
/// dentro de lo relajado y por eso empataban a cero, pero el segundo deja el meñique
/// colgando por encima del acorde.
#[must_use]
pub fn coste_forma_acorde(dedos: &[u8]) -> i32 {
    let (Some(primero), Some(ultimo)) = (dedos.first(), dedos.last()) else {
        return 0;
    };
    if dedos.len() < 2 {
        return 0;
    }
    i32::from(*primero).saturating_sub(1) + 5_i32.saturating_sub(i32::from(*ultimo))
}
