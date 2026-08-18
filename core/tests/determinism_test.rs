//! T041, T042, T043 — Historia 3: comportamiento reproducible bajo un reloj controlado.

mod fixtures;
use fixtures::SmfBuilder;
use piano_core::clock::{Clock, VirtualClock};
use piano_core::feedforward::{Cue, CueSchedule, Playback};
use piano_core::load_smf;
use piano_core::time::Micros;
use std::sync::Arc;

/// Una pieza con de todo: varias pistas, cambios de tempo, acordes, solapamientos y una
/// nota colgada. Cuanto mas sucia, mas exige al determinismo.
fn pieza_dificil() -> Vec<u8> {
    SmfBuilder::new(960)
        .track(|t| t.tempo(0, 500_000).tempo(1920, 300_000).tempo(3840, 750_000))
        .track(|t| {
            let mut t = t.chord(0, &[67, 60, 64], 100, 960);
            for i in 0..40u64 {
                t = t.note(960 + i * 240, 60 + (i % 24) as u8, 80 + (i % 40) as u8, 480);
            }
            t.note_on(20_000, 72, 100) // colgada a proposito
        })
        .track(|t| {
            let mut t = t;
            for i in 0..40u64 {
                t = t.raw(i * 480, &[0x91, 48 + (i % 12) as u8, 90])
                    .raw(i * 480 + 300, &[0x81, 48 + (i % 12) as u8, 0]);
            }
            t
        })
        .build()
}

#[test]
fn cargar_la_misma_cancion_cien_veces_da_exactamente_lo_mismo() {
    let raw = pieza_dificil();
    let referencia = load_smf(&raw).expect("valida");
    assert!(!referencia.notes().is_empty(), "la pieza tiene notas");
    for i in 0..100 {
        let otra = load_smf(&raw).expect("valida");
        assert_eq!(referencia, otra, "la carga numero {i} difiere de la primera");
    }
}

#[test]
fn el_orden_dentro_de_los_acordes_tambien_es_estable() {
    let raw = pieza_dificil();
    let referencia: Vec<(u64, u8, u16, u8)> = load_smf(&raw)
        .expect("valida")
        .notes()
        .iter()
        .map(|n| (n.onset_tick.0, n.key, n.track, n.channel))
        .collect();
    for _ in 0..25 {
        let otra: Vec<(u64, u8, u16, u8)> = load_smf(&raw)
            .expect("valida")
            .notes()
            .iter()
            .map(|n| (n.onset_tick.0, n.key, n.track, n.channel))
            .collect();
        assert_eq!(referencia, otra);
    }
}

#[test]
fn la_misma_secuencia_de_tiempo_produce_los_mismos_avisos() {
    // SC-003: cero variabilidad.
    let raw = pieza_dificil();
    let song = load_smf(&raw).expect("valida");
    let schedule = Arc::new(CueSchedule::build(&song, 480));

    let ejecutar = || {
        let mut reloj = VirtualClock::new();
        let mut pb = Playback::new(Arc::clone(&schedule));
        let mut salida: Vec<Cue> = Vec::new();
        for delta in [7_000u64, 13_000, 1, 250_000, 3, 999_999, 40_000, 5_000_000] {
            reloj.advance(Micros(delta));
            salida.extend(pb.advance_to(reloj.now()).expect("avance valido").iter().copied());
        }
        salida
    };

    let a = ejecutar();
    assert!(!a.is_empty(), "la ejecucion emite avisos");
    for i in 0..50 {
        assert_eq!(a, ejecutar(), "la ejecucion numero {i} difiere");
    }
}

#[test]
fn todos_los_avisos_se_emiten_exactamente_una_vez_en_una_pieza_sucia() {
    let raw = pieza_dificil();
    let song = load_smf(&raw).expect("valida");
    let schedule = Arc::new(CueSchedule::build(&song, 480));
    let mut pb = Playback::new(Arc::clone(&schedule));
    let mut reloj = VirtualClock::new();

    let mut emitidos: Vec<u32> = Vec::new();
    while !pb.is_finished() {
        reloj.advance(Micros(11_111));
        emitidos.extend(pb.advance_to(reloj.now()).expect("valido").iter().map(|c| c.note_index));
    }
    emitidos.sort_unstable();
    let esperados: Vec<u32> = (0..song.notes().len() as u32).collect();
    assert_eq!(emitidos, esperados, "cada nota, una sola vez (FR-012)");
}

#[test]
fn una_pieza_de_diez_minutos_se_recorre_sin_esperar_diez_minutos() {
    // FR-019. Si esta prueba tarda, es que algo esta mirando el reloj real.
    let mut b = SmfBuilder::new(480).track(|t| t.tempo(0, 500_000));
    b = b.track(|t| {
        let mut t = t;
        for i in 0..5_000u64 {
            t = t.note(i * 120, 21 + (i % 88) as u8, 64, 100);
        }
        t
    });
    let raw = b.build();

    let arranque = std::time::Instant::now();
    let song = load_smf(&raw).expect("valida");
    assert_eq!(song.notes().len(), 5_000);
    assert!(song.duration_us().0 > 600_000_000, "la pieza dura mas de 10 minutos");

    let schedule = Arc::new(CueSchedule::build(&song, 240));
    let mut pb = Playback::new(schedule);
    let mut reloj = VirtualClock::new();
    let mut total = 0usize;
    while !pb.is_finished() {
        reloj.advance(Micros(1_000_000)); // un segundo virtual por vuelta
        total += pb.advance_to(reloj.now()).expect("valido").len();
    }
    assert_eq!(total, 5_000);
    assert!(
        arranque.elapsed().as_millis() < 500,
        "recorrer 10 minutos virtuales tardo {} ms de tiempo real",
        arranque.elapsed().as_millis()
    );
}
