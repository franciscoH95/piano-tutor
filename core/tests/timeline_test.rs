//! T022, T028 — Historia 1: una cancion se convierte en una leccion con estructura temporal.

mod fixtures;
use fixtures::SmfBuilder;
use piano_core::load_smf;
use piano_core::time::{Micros, Ticks};

#[test]
fn cinco_notas_consecutivas_producen_cinco_notas_ordenadas() {
    let raw = SmfBuilder::new(480)
        .track(|t| {
            t.note(0, 60, 100, 240)
                .note(480, 62, 90, 240)
                .note(960, 64, 80, 240)
                .note(1440, 65, 70, 240)
                .note(1920, 67, 60, 240)
        })
        .build();
    let song = load_smf(&raw).expect("cancion valida");
    let notas = song.notes();
    assert_eq!(notas.len(), 5);
    assert_eq!(notas.iter().map(|n| n.key).collect::<Vec<_>>(), vec![60, 62, 64, 65, 67]);
    assert_eq!(notas.iter().map(|n| n.velocity).collect::<Vec<_>>(), vec![100, 90, 80, 70, 60]);
    assert_eq!(
        notas.iter().map(|n| n.onset_tick).collect::<Vec<_>>(),
        vec![Ticks(0), Ticks(480), Ticks(960), Ticks(1440), Ticks(1920)]
    );
    for n in notas {
        assert_eq!(n.end_tick.0 - n.onset_tick.0, 240, "duracion de cada nota");
    }
    assert!(song.report().is_clean(), "una cancion limpia no reporta nada: {:?}", song.report());
}

#[test]
fn un_acorde_comparte_instante_de_ataque() {
    let raw = SmfBuilder::new(480).track(|t| t.chord(0, &[60, 64, 67], 100, 480)).build();
    let song = load_smf(&raw).expect("valida");
    assert_eq!(song.notes().len(), 3);
    assert!(song.notes().iter().all(|n| n.onset_tick == Ticks(0)), "mismo ataque");
    assert!(song.notes().iter().all(|n| n.onset_us == Micros(0)));
    // Orden total: de grave a agudo dentro del acorde.
    assert_eq!(song.notes().iter().map(|n| n.key).collect::<Vec<_>>(), vec![60, 64, 67]);
}

#[test]
fn un_acorde_declarado_de_agudo_a_grave_sale_igualmente_ordenado() {
    // El orden total no depende del orden del archivo: es lo que hace la carga
    // reproducible (FR-008).
    let raw = SmfBuilder::new(480).track(|t| t.chord(0, &[67, 60, 64], 100, 480)).build();
    let song = load_smf(&raw).expect("valida");
    assert_eq!(song.notes().iter().map(|n| n.key).collect::<Vec<_>>(), vec![60, 64, 67]);
}

#[test]
fn un_cambio_de_tempo_altera_el_tiempo_real_pero_no_el_musical() {
    let notas = |t: fixtures::TrackBuilder| t.note(960, 60, 100, 480);
    let sin_cambio = SmfBuilder::new(480)
        .track(|t| t.tempo(0, 500_000))
        .track(notas)
        .build();
    let con_cambio = SmfBuilder::new(480)
        .track(|t| t.tempo(0, 500_000).tempo(480, 250_000))
        .track(notas)
        .build();

    let a = load_smf(&sin_cambio).expect("valida");
    let b = load_smf(&con_cambio).expect("valida");

    let na = a.notes().first().expect("una nota");
    let nb = b.notes().first().expect("una nota");

    assert_eq!(na.onset_tick, nb.onset_tick, "el tiempo musical no cambia (FR-003)");
    assert_eq!(na.onset_us, Micros(1_000_000), "sin cambio: dos negras a 120 = 1 s");
    assert_eq!(nb.onset_us, Micros(750_000), "con cambio: 0,5 s + 0,25 s");
    assert!(nb.onset_us < na.onset_us, "acelerar adelanta la nota en tiempo real");
}

#[test]
fn la_misma_altura_repetida_sin_silencio_produce_dos_notas() {
    let raw = SmfBuilder::new(480)
        .track(|t| t.note(0, 60, 100, 480).note(480, 60, 100, 480))
        .build();
    let song = load_smf(&raw).expect("valida");
    assert_eq!(song.notes().len(), 2, "dos notas, no una alargada");
    assert_eq!(song.notes()[0].onset_tick, Ticks(0));
    assert_eq!(song.notes()[1].onset_tick, Ticks(480));
}

#[test]
fn las_notas_conservan_su_pista_y_canal_de_origen() {
    // FR-006: es lo que permitira filtrar por mano sin rehacer la carga.
    let raw = SmfBuilder::new(480)
        .track(|t| t.note(0, 60, 100, 480))
        .track(|t| t.raw(0, &[0x91, 72, 100]).raw(480, &[0x81, 72, 0]))
        .build();
    let song = load_smf(&raw).expect("valida");
    assert_eq!(song.notes().len(), 2);
    let grave = song.notes().iter().find(|n| n.key == 60).expect("nota 60");
    let agudo = song.notes().iter().find(|n| n.key == 72).expect("nota 72");
    assert_eq!((grave.track, grave.channel), (0, 0));
    assert_eq!((agudo.track, agudo.channel), (1, 1));
}

#[test]
fn una_cancion_sin_notas_se_carga_sin_error() {
    let raw = SmfBuilder::new(480).track(|t| t.tempo(0, 500_000)).build();
    let song = load_smf(&raw).expect("una cancion vacia es valida, no un error");
    assert!(song.notes().is_empty());
    assert_eq!(song.duration_ticks(), Ticks(0));
    assert_eq!(song.duration_us(), Micros(0));
}

#[test]
fn la_duracion_es_el_final_de_la_ultima_nota() {
    let raw = SmfBuilder::new(480)
        .track(|t| t.note(0, 60, 100, 480).note(240, 64, 100, 960))
        .build();
    let song = load_smf(&raw).expect("valida");
    assert_eq!(song.duration_ticks(), Ticks(1200), "240 + 960, no 0 + 480");
    assert_eq!(song.duration_us(), Micros(1_250_000));
}

#[test]
fn los_eventos_de_tempo_de_la_pista_0_aplican_a_todas_las_pistas() {
    let raw = SmfBuilder::new(480)
        .track(|t| t.tempo(0, 250_000))
        .track(|t| t.note(480, 60, 100, 240))
        .build();
    let song = load_smf(&raw).expect("valida");
    assert_eq!(song.notes()[0].onset_us, Micros(250_000), "una negra a 240 = 250 ms");
}
