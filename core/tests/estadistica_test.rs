//! Pruebas de `core/src/evaluacion/estadistica.rs`.

mod fixtures;
use piano_core::evaluacion::{cuartiles, mediana};

#[test]
fn la_mediana_de_un_numero_impar_es_el_de_en_medio() {
    assert_eq!(mediana(&[5]), Some(5));
    assert_eq!(mediana(&[1, 2, 3]), Some(2));
    assert_eq!(mediana(&[3, 1, 2]), Some(2), "no depende del orden de entrada");
}

#[test]
fn la_mediana_de_un_numero_par_redondea_hacia_abajo_tambien_con_negativos() {
    // **La trampa.** La división entera de Rust trunca hacia CERO, no hacia abajo:
    // `(-3 + -2) / 2` da `-2`, no `-3`. Si no se corrige, la mediana de dos desfases
    // negativos sale sesgada hacia cero, es decir, hacia «va menos adelantado de lo que va».
    assert_eq!(mediana(&[2, 3]), Some(2), "positivos: hacia abajo");
    assert_eq!(mediana(&[-3, -2]), Some(-3), "negativos: TAMBIÉN hacia abajo, no hacia cero");
    assert_eq!(mediana(&[-1, 1]), Some(0));
    assert_eq!(mediana(&[-2, 1]), Some(-1), "no 0, que es lo que daría truncar hacia cero");
}

#[test]
fn una_lista_vacia_no_tiene_mediana() {
    assert_eq!(mediana(&[]), None, "None y no cero: cero sería un desfase perfecto");
}

#[test]
fn los_cuartiles_con_pocas_muestras() {
    // Con 1 y 2 elementos el recorrido intercuartílico no significa nada, pero no puede
    // reventar ni devolver basura.
    assert!(cuartiles(&[]).is_none());
    let (q1, q3) = cuartiles(&[7]).expect("un elemento");
    assert_eq!((q1, q3), (7, 7), "con una muestra los tres cuartiles coinciden");
    assert!(cuartiles(&[1, 9]).is_some());
}

#[test]
fn el_recorrido_intercuartilico_mide_la_dispersion() {
    // Cien valores idénticos: dispersión cero. Cien repartidos: dispersión grande.
    let iguales = [40_i64; 100];
    let (q1, q3) = cuartiles(&iguales).expect("cien elementos");
    assert_eq!(q3 - q1, 0, "todos iguales, ninguna dispersión");

    let repartidos: Vec<i64> = (0..100).map(|i| i * 10).collect();
    let (q1, q3) = cuartiles(&repartidos).expect("cien elementos");
    assert!(q3 - q1 > 300, "repartidos, dispersión grande: {}", q3 - q1);
}

#[test]
fn el_orden_de_entrada_no_altera_ningun_resultado() {
    // SC-008: ninguna medida puede depender de en qué orden llegaron las observaciones.
    let base: Vec<i64> = vec![-40, 12, 0, 99, -7, 33, 5, -1, 61, 20];
    let (m, c) = (mediana(&base), cuartiles(&base));
    // Diez permutaciones deterministas, girando la lista.
    for giro in 1..10usize {
        let mut otra = base.clone();
        otra.rotate_left(giro);
        assert_eq!(mediana(&otra), m, "giro {giro}");
        assert_eq!(cuartiles(&otra), c, "giro {giro}");
    }
    let mut alreves = base.clone();
    alreves.reverse();
    assert_eq!(mediana(&alreves), m);
    assert_eq!(cuartiles(&alreves), c);
}

#[test]
fn no_desborda_con_valores_extremos() {
    // Los desfases son microsegundos con signo. Sumar dos para promediar puede desbordar
    // si se hace ingenuamente.
    let extremos = [i64::MIN / 2, i64::MAX / 2];
    assert!(mediana(&extremos).is_some(), "sin pánico");
    let grandes = [i64::MAX - 1, i64::MAX];
    assert!(mediana(&grandes).is_some(), "sin pánico");
}
