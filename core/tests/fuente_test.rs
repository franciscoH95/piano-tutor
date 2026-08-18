//! T017 — la fuente controlada, que es lo que hace verificable todo lo demas sin hardware.

use piano_core::capture::{FuenteDeEventos, FuenteGuionizada, Observacion, TipoEvento};
use piano_core::time::Micros;

fn ataque(us: u64, key: u8) -> Observacion {
    Observacion { at: Micros(us), key, velocity: 90, kind: TipoEvento::Ataque, channel: 0 }
}
fn suelta(us: u64, key: u8) -> Observacion {
    Observacion { at: Micros(us), key, velocity: 0, kind: TipoEvento::Suelta, channel: 0 }
}

#[test]
fn entrega_el_guion_en_orden_y_con_sus_instantes() {
    let mut f = FuenteGuionizada::nueva(vec![
        ataque(0, 60),
        ataque(1_000, 64),
        suelta(2_000, 60),
        suelta(3_000, 64),
    ]);
    let mut v = Vec::new();
    assert_eq!(f.recoger(&mut v), 4);
    assert_eq!(v.iter().map(|e| e.at.0).collect::<Vec<_>>(), vec![0, 1_000, 2_000, 3_000]);
    assert_eq!(v.iter().map(|e| e.key).collect::<Vec<_>>(), vec![60, 64, 60, 64]);
    assert_eq!(v.iter().map(|e| e.seq).collect::<Vec<_>>(), vec![0, 1, 2, 3]);
}

#[test]
fn se_agota_al_terminar() {
    let mut f = FuenteGuionizada::nueva(vec![ataque(0, 60)]);
    assert!(!f.agotada());
    let mut v = Vec::new();
    f.recoger(&mut v);
    assert!(f.agotada());
    assert_eq!(f.recoger(&mut v), 0, "una fuente agotada no entrega mas");
}

#[test]
fn una_fuente_vacia_esta_agotada_desde_el_principio() {
    let mut f = FuenteGuionizada::nueva(Vec::new());
    assert!(f.agotada());
    let mut v = Vec::new();
    assert_eq!(f.recoger(&mut v), 0);
    f.esperar(); // no debe colgarse
}

#[test]
fn esperar_no_se_cuelga_cuando_hay_material() {
    let mut f = FuenteGuionizada::nueva(vec![ataque(0, 60)]);
    f.esperar(); // si bloquease, la prueba nunca terminaria
    let mut v = Vec::new();
    assert_eq!(f.recoger(&mut v), 1);
}

#[test]
fn el_mismo_guion_produce_siempre_lo_mismo() {
    let guion = || vec![ataque(0, 67), ataque(0, 60), ataque(0, 64), suelta(500, 60)];
    let ejecutar = || {
        let mut f = FuenteGuionizada::nueva(guion());
        let mut v = Vec::new();
        f.recoger(&mut v);
        v
    };
    let referencia = ejecutar();
    for _ in 0..50 {
        assert_eq!(ejecutar(), referencia);
    }
}

#[test]
fn sujeta_los_instantes_que_retroceden_igual_que_el_transporte() {
    let mut f = FuenteGuionizada::nueva(vec![ataque(1_000, 60), ataque(500, 61), ataque(2_000, 62)]);
    let mut v = Vec::new();
    f.recoger(&mut v);
    assert_eq!(v.iter().map(|e| e.at.0).collect::<Vec<_>>(), vec![1_000, 1_000, 2_000]);
}
