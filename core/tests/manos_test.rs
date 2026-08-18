//! T021, T022, T024, T026, T044 — de qué mano es cada nota.

mod fixtures;
use fixtures::SmfBuilder;
use piano_core::load_smf;
use piano_core::practica::{repartir, Mano, Reparto};

/// Dos voces en pistas distintas: la aguda arriba, la grave abajo.
fn dos_manos(n: u64) -> piano_core::Song {
    let raw = SmfBuilder::new(480)
        .track(|t| {
            let mut t = t;
            for i in 0..n { t = t.note(i * 480, 72 + (i % 12) as u8, 90, 240); }
            t
        })
        .track(|t| {
            let mut t = t;
            for i in 0..n { t = t.raw(i * 480, &[0x90, 40 + (i % 12) as u8, 90])
                                 .raw(i * 480 + 240, &[0x80, 40 + (i % 12) as u8, 0]); }
            t
        })
        .build();
    load_smf(&raw).expect("valida")
}

#[test]
fn con_dos_voces_separadas_se_usan_las_del_archivo() {
    let song = dos_manos(20);
    let r = repartir(&song, 60);
    assert_eq!(r.origen(), Reparto::VocesDelArchivo);
    let agudas: Vec<Mano> = song.notes().iter().enumerate()
        .filter(|(_, n)| n.key >= 72).map(|(i, _)| r.mano(i)).collect();
    assert!(agudas.iter().all(|m| *m == Mano::Derecha), "las agudas son la derecha");
}

#[test]
fn la_derecha_es_la_voz_de_mediana_mas_alta_aunque_este_en_la_pista_1() {
    // T022. Aquí la pista 0 lleva las notas GRAVES: si el reparto fuese por índice de
    // pista se equivocaría, y ése es justo el error que la regla evita.
    let raw = SmfBuilder::new(480)
        .track(|t| { let mut t = t; for i in 0..20u64 { t = t.note(i*480, 40, 90, 240); } t })
        .track(|t| { let mut t = t; for i in 0..20u64 {
            t = t.raw(i*480, &[0x91, 76, 90]).raw(i*480+240, &[0x81, 76, 0]); } t })
        .build();
    let song = load_smf(&raw).expect("valida");
    let r = repartir(&song, 60);
    assert_eq!(r.origen(), Reparto::VocesDelArchivo);
    for (i, n) in song.notes().iter().enumerate() {
        let esperada = if n.key == 76 { Mano::Derecha } else { Mano::Izquierda };
        assert_eq!(r.mano(i), esperada, "nota {} (tecla {})", i, n.key);
    }
}

#[test]
fn con_una_sola_voz_se_reparte_por_altura() {
    let raw = SmfBuilder::new(480)
        .track(|t| { let mut t = t;
            for (i, k) in [40u8, 72, 45, 80, 50, 67].into_iter().enumerate() {
                t = t.note(i as u64 * 480, k, 90, 240); } t })
        .build();
    let song = load_smf(&raw).expect("valida");
    let r = repartir(&song, 60);
    assert_eq!(r.origen(), Reparto::CortePorAltura);
    for (i, n) in song.notes().iter().enumerate() {
        let esperada = if n.key >= 60 { Mano::Derecha } else { Mano::Izquierda };
        assert_eq!(r.mano(i), esperada, "tecla {}", n.key);
    }
}

#[test]
fn mover_el_corte_reasigna_solo_las_notas_afectadas() {
    // T024 y T044: mover el corte cambia el reparto, y con él lo que dependa de él.
    let raw = SmfBuilder::new(480)
        .track(|t| { let mut t = t;
            for (i, k) in [55u8, 62, 68].into_iter().enumerate() {
                t = t.note(i as u64 * 480, k, 90, 240); } t })
        .build();
    let song = load_smf(&raw).expect("valida");
    let bajo = repartir(&song, 60);
    let alto = repartir(&song, 65);
    let manos_bajo: Vec<Mano> = (0..3).map(|i| bajo.mano(i)).collect();
    let manos_alto: Vec<Mano> = (0..3).map(|i| alto.mano(i)).collect();
    assert_eq!(manos_bajo, vec![Mano::Izquierda, Mano::Derecha, Mano::Derecha]);
    assert_eq!(manos_alto, vec![Mano::Izquierda, Mano::Izquierda, Mano::Derecha],
               "solo la del 62 cambia de mano");
}

#[test]
fn tres_voces_no_cuentan_como_manos_separadas() {
    // G1: exactamente dos voces con notas.
    let raw = SmfBuilder::new(480)
        .track(|t| { let mut t = t; for i in 0..10u64 { t = t.note(i*480, 40, 90, 240); } t })
        .track(|t| { let mut t = t; for i in 0..10u64 {
            t = t.raw(i*480, &[0x91, 60, 90]).raw(i*480+240, &[0x81, 60, 0]); } t })
        .track(|t| { let mut t = t; for i in 0..10u64 {
            t = t.raw(i*480, &[0x92, 80, 90]).raw(i*480+240, &[0x82, 80, 0]); } t })
        .build();
    let song = load_smf(&raw).expect("valida");
    assert_eq!(repartir(&song, 60).origen(), Reparto::CortePorAltura);
}

#[test]
fn una_voz_marginal_no_cuenta_como_mano() {
    // G3, primera mitad: cada voz necesita al menos el 5 % de las notas.
    let raw = SmfBuilder::new(480)
        .track(|t| { let mut t = t; for i in 0..100u64 { t = t.note(i*480, 72, 90, 240); } t })
        .track(|t| t.raw(0, &[0x91, 40, 90]).raw(240, &[0x81, 40, 0]))  // una sola nota
        .build();
    let song = load_smf(&raw).expect("valida");
    assert_eq!(repartir(&song, 60).origen(), Reparto::CortePorAltura,
               "una voz con el 1 % de las notas no es una mano");
}

#[test]
fn dos_voces_de_la_misma_altura_no_cuentan_como_manos() {
    // G3, segunda mitad: las medianas deben diferir al menos tres semitonos.
    let raw = SmfBuilder::new(480)
        .track(|t| { let mut t = t; for i in 0..20u64 { t = t.note(i*480, 60, 90, 240); } t })
        .track(|t| { let mut t = t; for i in 0..20u64 {
            t = t.raw(i*480, &[0x91, 61, 90]).raw(i*480+240, &[0x81, 61, 0]); } t })
        .build();
    let song = load_smf(&raw).expect("valida");
    assert_eq!(repartir(&song, 60).origen(), Reparto::CortePorAltura,
               "un semitono de diferencia no separa dos manos");
}

#[test]
fn la_percusion_no_cuenta_como_voz() {
    let raw = SmfBuilder::new(480)
        .track(|t| { let mut t = t; for i in 0..20u64 { t = t.note(i*480, 72, 90, 240); } t })
        .track(|t| { let mut t = t; for i in 0..20u64 {
            t = t.raw(i*480, &[0x99, 38, 90]).raw(i*480+240, &[0x89, 38, 0]); } t })
        .build();
    let song = load_smf(&raw).expect("valida");
    assert_eq!(repartir(&song, 60).origen(), Reparto::CortePorAltura,
               "con la percusion descartada solo queda una voz");
}

#[test]
fn el_reparto_es_determinista() {
    let song = dos_manos(30);
    let referencia: Vec<Mano> = (0..song.notes().len()).map(|i| repartir(&song, 60).mano(i)).collect();
    for _ in 0..50 {
        let r = repartir(&song, 60);
        let ahora: Vec<Mano> = (0..song.notes().len()).map(|i| r.mano(i)).collect();
        assert_eq!(ahora, referencia);
    }
}

#[test]
fn una_cancion_sin_notas_no_revienta() {
    let raw = SmfBuilder::new(480).track(|t| t.tempo(0, 500_000)).build();
    let song = load_smf(&raw).expect("valida");
    let r = repartir(&song, 60);
    assert_eq!(r.origen(), Reparto::CortePorAltura);
}
