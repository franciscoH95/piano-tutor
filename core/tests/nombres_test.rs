//! T017, T018, T020 — cómo se llama cada nota.

mod fixtures;
use fixtures::SmfBuilder;
use piano_core::load_smf;
use piano_core::practica::{Alteracion, Base, MapaDeArmaduras, NombreDeNota};
use piano_core::time::Ticks;

/// Meta de armadura: `FF 59 02 sf mi`.
fn armadura(sf: i8) -> Vec<u8> {
    vec![0xFF, 0x59, 0x02, sf as u8, 0x00]
}

#[test]
fn con_armadura_de_sostenidos_la_tecla_61_es_do_sostenido() {
    let m = MapaDeArmaduras::desde(&[(0, 2)]); // Re mayor, dos sostenidos
    let n = m.nombre(Ticks(0), 61);
    assert_eq!((n.base, n.alteracion), (Base::Do, Alteracion::Sostenido));
}

#[test]
fn con_armadura_de_bemoles_la_tecla_61_es_re_bemol() {
    let m = MapaDeArmaduras::desde(&[(0, -3)]); // Mi bemol mayor
    let n = m.nombre(Ticks(0), 61);
    assert_eq!((n.base, n.alteracion), (Base::Re, Alteracion::Bemol));
}

#[test]
fn sin_armadura_declarada_se_usan_sostenidos() {
    let m = MapaDeArmaduras::desde(&[]);
    assert_eq!(m.nombre(Ticks(0), 61).alteracion, Alteracion::Sostenido);
    assert_eq!(m.nombre(Ticks(0), 61).base, Base::Do);
}

#[test]
fn una_tecla_blanca_nunca_lleva_alteracion() {
    // Simplificación declarada: no hay Mi♯ ni Do♭, aunque la teoría los contemple.
    for sf in [-7i8, -3, 0, 3, 7] {
        let m = MapaDeArmaduras::desde(&[(0, sf)]);
        for key in [60u8, 62, 64, 65, 67, 69, 71] {
            let n = m.nombre(Ticks(0), key);
            assert_eq!(n.alteracion, Alteracion::Ninguna, "tecla {key} con sf {sf}");
        }
    }
}

#[test]
fn las_doce_alturas_tienen_nombre_en_ambas_tablas() {
    for sf in [-1i8, 1] {
        let m = MapaDeArmaduras::desde(&[(0, sf)]);
        for key in 60u8..72 {
            let _ = m.nombre(Ticks(0), key); // no debe entrar en pánico
        }
    }
}

#[test]
fn toma_la_ultima_armadura_con_tick_menor_o_igual() {
    // T018: la misma forma que el mapa de tempo que ya existe.
    let m = MapaDeArmaduras::desde(&[(0, 2), (1_000, -3), (2_000, 0)]);
    assert_eq!(m.nombre(Ticks(0), 61).alteracion, Alteracion::Sostenido);
    assert_eq!(m.nombre(Ticks(999), 61).alteracion, Alteracion::Sostenido);
    assert_eq!(m.nombre(Ticks(1_000), 61).alteracion, Alteracion::Bemol);
    assert_eq!(m.nombre(Ticks(1_999), 61).alteracion, Alteracion::Bemol);
    assert_eq!(m.nombre(Ticks(2_000), 61).alteracion, Alteracion::Sostenido);
}

#[test]
fn varias_armaduras_en_el_mismo_tick_gana_la_ultima() {
    let m = MapaDeArmaduras::desde(&[(0, 2), (0, -3)]);
    assert_eq!(m.nombre(Ticks(0), 61).alteracion, Alteracion::Bemol);
}

#[test]
fn una_armadura_posterior_al_inicio_no_afecta_a_lo_anterior() {
    let m = MapaDeArmaduras::desde(&[(5_000, -3)]);
    assert_eq!(m.nombre(Ticks(0), 61).alteracion, Alteracion::Sostenido, "antes, el defecto");
    assert_eq!(m.nombre(Ticks(5_000), 61).alteracion, Alteracion::Bemol);
}

#[test]
fn la_cancion_cargada_expone_su_mapa_de_armaduras() {
    // El cargador de la feature 001 descartaba este meta-evento; ahora lo conserva.
    let raw = SmfBuilder::new(480)
        .track(|t| t.raw(0, &armadura(-3)).raw(960, &armadura(2)))
        .track(|t| t.note(0, 61, 90, 240).note(960, 61, 90, 240))
        .build();
    let song = load_smf(&raw).expect("valida");
    let m = song.armaduras();
    assert_eq!(m.nombre(Ticks(0), 61).alteracion, Alteracion::Bemol);
    assert_eq!(m.nombre(Ticks(960), 61).alteracion, Alteracion::Sostenido);
}

#[test]
fn una_cancion_sin_armadura_declarada_sigue_dando_nombres() {
    let raw = SmfBuilder::new(480).track(|t| t.note(0, 61, 90, 240)).build();
    let song = load_smf(&raw).expect("valida");
    assert_eq!(song.armaduras().nombre(Ticks(0), 61).alteracion, Alteracion::Sostenido);
}

#[test]
fn el_nombre_es_simbolico_y_no_una_cadena() {
    // El formateo pertenece a quien pinta: el núcleo no sabe de textos ni de idiomas.
    let m = MapaDeArmaduras::desde(&[]);
    let n: NombreDeNota = m.nombre(Ticks(0), 60);
    assert_eq!(n.base, Base::Do);
    assert_eq!(n.alteracion, Alteracion::Ninguna);
}
