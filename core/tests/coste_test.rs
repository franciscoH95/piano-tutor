//! T031 — las reglas del modelo de coste, **una por una**.
//!
//! Cada regla se comprueba con el caso que la dispara y con el que no. Esta prueba existe
//! porque su ausencia dejó pasar dos reglas enteras sin implementar (recolocación de mano
//! y paso del pulgar): sólo se comprobaba el resultado global de la escala, y ese resultado
//! puede salir mal por muchas causas distintas sin decir cuál.
//!
//! Se puntúa con `coste_de`, la función objetivo que minimiza `digitar`. Las reglas del
//! modelo son el subconjunto de Parncutt et al. (1997) que gobierna escalas y pasajes; los
//! acordes de más de cinco notas y los saltos extremos quedan fuera y se declara que quedan
//! fuera.

use piano_core::digitacion::coste_de;
use piano_core::practica::Mano;

const D: Mano = Mano::Derecha;

// ------------------------------------------------- Regla 6: dedo débil

#[test]
fn el_anular_y_el_menique_cuestan_y_los_demas_no() {
    // Parncutt los pesa **igual**. Ponderar el 4 por encima del 5 expulsaba el final
    // 3-4-5 de las escalas, que es justo la digitación de manual.
    assert_eq!(coste_de(&[60], &[4], D), 1, "el anular cuesta");
    assert_eq!(coste_de(&[60], &[5], D), 1, "el meñique cuesta lo mismo");
    for dedo in [1, 2, 3] {
        assert_eq!(coste_de(&[60], &[dedo], D), 0, "el dedo {dedo} no es débil");
    }
}

// ------------------------------------------------- pulgar y meñique en tecla negra

#[test]
fn el_pulgar_en_negra_cuesta_y_en_blanca_no() {
    assert_eq!(coste_de(&[61], &[1], D), 8, "pulgar en negra");
    assert_eq!(coste_de(&[60], &[1], D), 0, "pulgar en blanca");
}

#[test]
fn el_pulgar_en_negra_tambien_encarece_la_transicion() {
    // Doce por la transición más ocho por la nota: el gesto es incómodo de las dos formas.
    let en_negra = coste_de(&[61, 65], &[1, 3], D);
    let en_blanca = coste_de(&[60, 65], &[1, 3], D);
    assert_eq!(en_negra - en_blanca, 20, "la negra bajo el pulgar penaliza");
}

#[test]
fn el_menique_en_negra_encarece_la_transicion() {
    // Menos grave que el pulgar, pero se evita. Se mide por diferencia porque el vano de
    // los dos casos es igual de grande y no se quiere medir el estiramiento.
    let en_negra = coste_de(&[66, 61], &[2, 5], D);
    let en_blanca = coste_de(&[65, 60], &[2, 5], D);
    assert_eq!(en_negra - en_blanca, 4, "el meñique en negra penaliza");
}

// ------------------------------------------------- repetición de dedo

#[test]
fn repetir_dedo_en_notas_distintas_cuesta_y_en_la_misma_no() {
    assert_eq!(coste_de(&[60, 62], &[3, 3], D), 200, "obliga a saltar");
    assert_eq!(coste_de(&[60, 60], &[3, 3], D), 0, "una nota repetida sí");
}

// ------------------------------------------------- Regla 12: paso del pulgar

#[test]
fn cruzar_el_pulgar_cuesta_y_no_cruzarlo_no() {
    // Sin esta regla el paso del pulgar sale gratis salvo el estiramiento, y el óptimo
    // prefiere pasar dos veces antes que usar el anular y el meñique.
    assert!(coste_de(&[64, 65], &[3, 1], D) > 0, "3→1 ascendente cruza");
    assert_eq!(coste_de(&[64, 65], &[2, 3], D), 0, "2→3 no cruza");
    assert_eq!(coste_de(&[60, 62], &[1, 2], D), 0, "1→2 ascendente no cruza");
}

#[test]
fn cruzar_el_pulgar_bajo_una_negra_no_penaliza() {
    // Es el gesto normal de toda escala con sostenidos: en Re mayor el pulgar pasa bajo el
    // fa♯. Penalizarlo expulsaba esas escalas de la solución óptima. Lo incómodo es que el
    // pulgar **aterrice** en negra, y eso lo cobran otras reglas.
    let bajo_negra = coste_de(&[66, 67], &[3, 1], D);
    let bajo_blanca = coste_de(&[64, 65], &[3, 1], D);
    assert_eq!(bajo_negra, bajo_blanca, "pasar bajo una negra no es el problema");
}

#[test]
fn que_el_pulgar_aterrice_en_negra_si_penaliza() {
    let aterriza_en_negra = coste_de(&[65, 66], &[3, 1], D);
    let aterriza_en_blanca = coste_de(&[64, 65], &[3, 1], D);
    assert!(
        aterriza_en_negra > aterriza_en_blanca,
        "el pulgar en negra sí se evita ({aterriza_en_negra} > {aterriza_en_blanca})"
    );
}

// ------------------------------------------------- Regla 4: recolocación de mano

#[test]
fn dos_recolocaciones_cuestan_mas_que_una() {
    // Es la magnitud que separa la digitación canónica de sus alternativas. Sin este
    // término las dos empataban y el desempate lo decidía el orden de iteración, que no
    // es un criterio musical.
    let escala = [60u8, 62, 64, 65, 67, 69, 71, 72];
    let una = coste_de(&escala, &[1, 2, 3, 1, 2, 3, 4, 5], D);
    let dos = coste_de(&escala, &[1, 2, 3, 1, 2, 1, 2, 3], D);
    assert!(una < dos, "una recolocación ({una}) debe costar menos que dos ({dos})");
}

// ------------------------------------------------- Regla 7: tres-cuatro-cinco

#[test]
fn el_terceto_tres_cuatro_cinco_cuesta_uno_en_ambos_sentidos() {
    // Peso 1, el del artículo. Es torpe, pero es el final normal de una escala:
    // penalizarlo más lo expulsaba de la solución óptima.
    let notas = [60u8, 62, 64];
    let debiles = coste_de(&notas, &[3, 4, 5], D) - coste_de(&notas, &[1, 2, 3], D);
    assert_eq!(debiles, 3, "dos dedos débiles (1+1) más el terceto (1)");

    let bajando = [64u8, 62, 60];
    let inverso = coste_de(&bajando, &[5, 4, 3], D) - coste_de(&bajando, &[3, 2, 1], D);
    assert_eq!(inverso, 3, "5-4-3 penaliza igual que 3-4-5");
}

#[test]
fn un_terceto_cualquiera_no_penaliza() {
    let notas = [60u8, 62, 64];
    assert_eq!(coste_de(&notas, &[1, 2, 3], D), 0, "1-2-3 es cómodo");
}

// ------------------------------------------------- estiramiento

#[test]
fn el_estiramiento_crece_con_el_vano_y_lo_impracticable_sigue_siendo_finito() {
    // SC-009: el sistema debe proponer siempre algo, aunque sea lo menos malo. Un coste
    // infinito dejaría notas sin dedo.
    let comodo = coste_de(&[60, 64], &[1, 3], D);
    let forzado = coste_de(&[60, 71], &[1, 3], D);
    let imposible = coste_de(&[60, 100], &[1, 2], D);
    assert!(comodo < forzado, "más vano, más coste ({comodo} < {forzado})");
    assert!(forzado < imposible, "lo impracticable cuesta mucho más");
    assert!(imposible < i32::MAX, "pero es finito: {imposible}");
}
