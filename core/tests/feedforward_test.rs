//! T031, T033, T035, T037, T039 — Historia 2: saber que nota viene antes de tocarla.

mod fixtures;
use fixtures::SmfBuilder;
use piano_core::feedforward::{CueSchedule, Playback};
use piano_core::time::Micros;
use piano_core::{load_smf, Song};
use std::sync::Arc;

/// Cuatro negras consecutivas a 120 pulsaciones por minuto, PPQ 480.
/// Cada negra dura 500 ms.
fn cancion_simple() -> Song {
    let raw = SmfBuilder::new(480)
        .track(|t| {
            t.note(0, 60, 100, 480)
                .note(480, 62, 100, 480)
                .note(960, 64, 100, 480)
                .note(1440, 65, 100, 480)
        })
        .build();
    load_smf(&raw).expect("valida")
}

fn reproduccion(song: &Song, lead: u64) -> Playback {
    Playback::new(Arc::new(CueSchedule::build(song, lead)))
}

#[test]
fn el_cue_ocupa_exactamente_32_bytes() {
    // Importa: el scheduler devuelve cortes de este array en la ruta de reproduccion.
    assert_eq!(std::mem::size_of::<piano_core::feedforward::Cue>(), 32);
}

#[test]
fn hay_un_aviso_por_nota_y_siempre_antes_del_ataque() {
    let song = cancion_simple();
    let schedule = CueSchedule::build(&song, 240);
    assert_eq!(schedule.len(), song.notes().len(), "un aviso por nota (FR-012)");
    for c in schedule.cues() {
        assert!(c.cue_at <= c.onset_at, "el aviso nunca llega tarde (FR-010)");
    }
}

#[test]
fn la_antelacion_se_mide_en_pulsos_no_en_milisegundos() {
    let song = cancion_simple();
    let schedule = CueSchedule::build(&song, 240); // media negra
    let primero = schedule.cues().first().expect("un aviso");
    // La primera nota ataca en el tick 0: el aviso satura en cero, no va a negativo.
    assert_eq!(primero.cue_tick, 0);
    let segundo = schedule.cues().get(1).expect("dos avisos");
    assert_eq!(segundo.cue_tick, 240, "480 - 240 pulsos");
    assert_eq!(segundo.cue_at, Micros(250_000), "media negra a 120 = 250 ms");
}

#[test]
fn una_antelacion_de_cero_avisa_justo_en_el_ataque() {
    let song = cancion_simple();
    let schedule = CueSchedule::build(&song, 0);
    for (c, n) in schedule.cues().iter().zip(song.notes()) {
        assert_eq!(c.cue_at, n.onset_us);
        assert_eq!(c.cue_at, c.onset_at);
    }
}

#[test]
fn los_avisos_de_un_acorde_salen_juntos_y_de_grave_a_agudo() {
    let raw = SmfBuilder::new(480).track(|t| t.chord(480, &[67, 60, 64], 100, 480)).build();
    let song = load_smf(&raw).expect("valida");
    let schedule = CueSchedule::build(&song, 240);
    let cues = schedule.cues();
    assert_eq!(cues.len(), 3);
    assert!(cues.iter().all(|c| c.cue_at == cues[0].cue_at), "mismo instante de aviso");
    let alturas: Vec<u8> =
        cues.iter().map(|c| song.notes()[c.note_index as usize].key).collect();
    assert_eq!(alturas, vec![60, 64, 67], "de grave a agudo (FR-013)");
}

#[test]
fn los_avisos_estan_ordenados_y_sin_empates() {
    let song = cancion_simple();
    let schedule = CueSchedule::build(&song, 240);
    let cues = schedule.cues();
    for w in cues.windows(2) {
        assert!(w[0].cue_at <= w[1].cue_at, "orden no decreciente por instante");
        let a = (w[0].cue_at, w[0].cue_tick, w[0].note_index);
        let b = (w[1].cue_at, w[1].cue_tick, w[1].note_index);
        assert!(a < b, "la clave de orden es total: sin empates posibles");
    }
}

#[test]
fn cada_aviso_se_emite_exactamente_una_vez() {
    let song = cancion_simple();
    let mut pb = reproduccion(&song, 240);
    let mut vistos = Vec::new();
    for us in (0..2_500_000).step_by(50_000) {
        vistos.extend(pb.advance_to(Micros(us)).expect("avance valido").iter().copied());
    }
    assert_eq!(vistos.len(), 4, "cuatro avisos, ni uno mas");
    let mut indices: Vec<u32> = vistos.iter().map(|c| c.note_index).collect();
    indices.sort_unstable();
    assert_eq!(indices, vec![0, 1, 2, 3]);
}

#[test]
fn un_salto_grande_no_se_salta_ninguna_nota() {
    // FR-015: el avance cruza las cuatro notas de una vez.
    let song = cancion_simple();
    let mut pb = reproduccion(&song, 240);
    let emitidos = pb.advance_to(Micros(10_000_000)).expect("avance valido");
    assert_eq!(emitidos.len(), 4, "todas las notas cruzadas se anuncian");
    assert!(pb.is_finished());
}

#[test]
fn una_antelacion_mayor_que_la_cancion_lo_anuncia_todo_al_principio() {
    let song = cancion_simple();
    let mut pb = reproduccion(&song, 100_000); // muchisimo mas larga que la pieza
    let emitidos = pb.advance_to(Micros(0)).expect("avance valido");
    assert_eq!(emitidos.len(), 4, "todo cae en el instante cero");
}

#[test]
fn una_cancion_sin_notas_no_anuncia_nada_y_termina_limpiamente() {
    let raw = SmfBuilder::new(480).track(|t| t.tempo(0, 500_000)).build();
    let song = load_smf(&raw).expect("valida");
    let mut pb = reproduccion(&song, 240);
    assert!(pb.is_finished(), "sin notas, terminada desde el principio (FR-016)");
    assert!(pb.advance_to(Micros(1_000_000)).expect("avance valido").is_empty());
}

#[test]
fn retroceder_el_tiempo_se_rechaza_y_no_altera_el_estado() {
    let song = cancion_simple();
    let mut pb = reproduccion(&song, 240);
    let antes = pb.advance_to(Micros(1_000_000)).expect("avance valido").len();
    assert!(antes > 0);

    let err = pb.advance_to(Micros(500_000)).expect_err("retroceder es un error (FR-020)");
    assert_eq!(err.last, Micros(1_000_000));
    assert_eq!(err.requested, Micros(500_000));

    // El estado no se movio: avanzar al mismo instante no reemite nada.
    assert!(pb.advance_to(Micros(1_000_000)).expect("avance valido").is_empty());
}

#[test]
fn seek_es_el_unico_camino_hacia_atras() {
    let song = cancion_simple();
    let mut pb = reproduccion(&song, 0);
    assert_eq!(pb.advance_to(Micros(2_000_000)).expect("valido").len(), 4);
    assert!(pb.is_finished());

    pb.seek(Micros(0));
    assert!(!pb.is_finished(), "tras reposicionar quedan avisos por delante");
    assert_eq!(pb.advance_to(Micros(2_000_000)).expect("valido").len(), 3, "los tres restantes");
}

#[test]
fn el_aviso_dice_cuanto_falta_y_satura_en_cero() {
    let song = cancion_simple();
    let schedule = CueSchedule::build(&song, 480);
    let segundo = schedule.cues().get(1).expect("dos avisos");
    assert_eq!(segundo.onset_at, Micros(500_000));
    assert_eq!(segundo.remaining_at(Micros(0)), Micros(500_000));
    assert_eq!(segundo.remaining_at(Micros(250_000)), Micros(250_000));
    assert_eq!(segundo.remaining_at(Micros(900_000)), Micros(0), "satura, no da negativo");
}

#[test]
fn la_antelacion_conserva_su_distancia_musical_al_cambiar_el_tempo() {
    // FR-011: 240 pulsos siguen siendo 240 pulsos; lo que cambia es el margen real.
    let notas = |t: fixtures::TrackBuilder| t.note(960, 60, 100, 480);
    let lento = SmfBuilder::new(480).track(|t| t.tempo(0, 500_000)).track(notas).build();
    let rapido = SmfBuilder::new(480).track(|t| t.tempo(0, 250_000)).track(notas).build();

    let a = load_smf(&lento).expect("valida");
    let b = load_smf(&rapido).expect("valida");
    let ca = CueSchedule::build(&a, 240);
    let cb = CueSchedule::build(&b, 240);
    let (ca, cb) = (ca.cues()[0], cb.cues()[0]);

    assert_eq!(ca.cue_tick, 720, "960 - 240 pulsos");
    assert_eq!(cb.cue_tick, 720, "la distancia musical es la misma");

    let margen_a = ca.onset_at.0 - ca.cue_at.0;
    let margen_b = cb.onset_at.0 - cb.cue_at.0;
    assert_eq!(margen_a, 250_000, "media negra a 120 = 250 ms");
    assert_eq!(margen_b, 125_000, "media negra a 240 = 125 ms");
    assert!(margen_b < margen_a, "a tempo rapido el margen real se encoge");
}

#[test]
fn el_programa_de_avisos_se_puede_compartir_entre_reproducciones() {
    let song = cancion_simple();
    let schedule = Arc::new(CueSchedule::build(&song, 240));
    let mut a = Playback::new(Arc::clone(&schedule));
    let mut b = Playback::new(Arc::clone(&schedule));
    assert_eq!(a.advance_to(Micros(600_000)).expect("valido").len(), 2);
    assert_eq!(b.advance_to(Micros(600_000)).expect("valido").len(), 2, "independientes");
}
