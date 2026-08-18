//! T010, T018, T020, T024, T026 — casos limite: entrada corrupta y datos sucios.
//!
//! La regla que se prueba aqui: un fallo **estructural** aborta la carga con un error
//! tipado; un dato **sucio** pero tocable la completa, se corrige segun regla documentada
//! y se cuenta. Nunca hay panico, sea cual sea la entrada.

mod fixtures;
use fixtures::SmfBuilder;
use piano_core::error::LoadError;
use piano_core::load_smf;
use piano_core::timeline::Closure;
use piano_core::time::Ticks;

// ---------------------------------------------------------------- estructural

#[test]
fn una_entrada_vacia_se_rechaza_sin_panico() {
    assert!(load_smf(&[]).is_err());
}

#[test]
fn una_entrada_que_no_es_midi_se_rechaza_sin_panico() {
    assert_eq!(load_smf(b"esto no es un midi").unwrap_err(), LoadError::MagicInvalido);
    assert_eq!(load_smf(b"MTh").unwrap_err(), LoadError::CabeceraTruncada);
}

#[test]
fn el_timing_smpte_se_rechaza() {
    // Bit 15 a 1. Es exactamente la entrada que hace panicar a midly (research, Decision 1).
    let raw = SmfBuilder::new(480).track(|t| t).build_with_division(0x8000);
    assert_eq!(load_smf(&raw).unwrap_err(), LoadError::TimingSmpteNoSoportado);
}

#[test]
fn una_division_de_cero_se_rechaza() {
    let raw = SmfBuilder::new(480).track(|t| t).build_with_division(0);
    assert_eq!(load_smf(&raw).unwrap_err(), LoadError::DivisionCero);
}

#[test]
fn el_formato_2_se_rechaza_porque_sus_pistas_no_comparten_eje_temporal() {
    let raw = SmfBuilder::new(480).format(2).track(|t| t.note(0, 60, 100, 480)).build();
    assert_eq!(load_smf(&raw).unwrap_err(), LoadError::FormatoNoSoportado { format: 2 });
}

#[test]
fn un_tempo_absurdo_se_rechaza_en_vez_de_entregar_una_leccion_falsa() {
    // El parser satura un tempo de 0 a 1 microsegundo por negra sin avisar: 60 millones
    // de pulsaciones por minuto. Aceptarlo daria una leccion en la que todo ocurre a la
    // vez. Por eso el nucleo impone un suelo musical propio.
    let cero = SmfBuilder::new(480)
        .track(|t| t.tempo(0, 0))
        .track(|t| t.note(0, 60, 100, 480))
        .build();
    assert!(matches!(load_smf(&cero), Err(LoadError::InvalidTempo { .. })), "tempo cero");

    let absurdo = SmfBuilder::new(480).track(|t| t.tempo(0, 10)).build();
    assert!(matches!(load_smf(&absurdo), Err(LoadError::InvalidTempo { .. })), "tempo absurdo");

    // Un tempo rapido pero real si se acepta: 400 pulsaciones por minuto.
    let rapido = SmfBuilder::new(480).track(|t| t.tempo(0, 150_000)).build();
    assert!(load_smf(&rapido).is_ok(), "400 pulsaciones por minuto es musica de verdad");
}

#[test]
fn ninguna_entrada_truncada_provoca_panico() {
    // SC-005: se recorta un fixture valido por todos los puntos posibles.
    let completo = SmfBuilder::new(480)
        .track(|t| t.tempo(0, 500_000))
        .track(|t| t.chord(0, &[60, 64, 67], 100, 480).note(480, 72, 90, 240))
        .build();
    for n in 0..completo.len() {
        let _ = load_smf(&completo[..n]); // no debe entrar en panico
    }
    assert!(load_smf(&completo).is_ok(), "el fixture completo si es valido");
}

#[test]
fn ningun_byte_alterado_provoca_panico() {
    let completo = SmfBuilder::new(96)
        .track(|t| t.tempo(0, 500_000).note(0, 60, 100, 96))
        .build();
    for i in 0..completo.len() {
        for v in [0x00u8, 0x7F, 0x80, 0xFF] {
            let mut m = completo.clone();
            m[i] = v;
            let _ = load_smf(&m); // no debe entrar en panico
        }
    }
}

// ---------------------------------------------------------------- datos sucios

#[test]
fn note_on_con_velocity_cero_equivale_a_note_off() {
    let raw = SmfBuilder::new(480)
        .track(|t| t.note_on(0, 60, 100).note_on_vel0(480, 60))
        .build();
    let song = load_smf(&raw).expect("valida");
    assert_eq!(song.notes().len(), 1);
    let n = &song.notes()[0];
    assert_eq!((n.onset_tick, n.end_tick), (Ticks(0), Ticks(480)));
    assert_eq!(n.closure, Closure::Normal, "cerro bien, no quedo colgada");
    assert_eq!(song.report().hanging_notes, 0);
}

#[test]
fn el_running_status_se_interpreta_junto_con_velocity_cero() {
    // Ese es justo el patron que el truco de velocity cero existe para permitir: una
    // rafaga de notas sin repetir el byte de estado.
    let raw = SmfBuilder::new(480)
        .track(|t| {
            t.raw(0, &[0x90, 60, 100]) // status explicito
                .raw(0, &[64, 100]) // running status: note-on
                .raw(480, &[60, 0]) // running status: note-on vel 0 = note-off
                .raw(480, &[64, 0])
        })
        .build();
    let song = load_smf(&raw).expect("valida");
    assert_eq!(song.notes().len(), 2, "dos notas completas");
    assert!(song.notes().iter().all(|n| n.end_tick == Ticks(480)));
    assert_eq!(song.report().hanging_notes, 0);
}

#[test]
fn el_emparejamiento_es_fifo_y_no_lifo() {
    // Caso canonico. FIFO: el note-off mas antiguo cierra el note-on mas antiguo.
    //   on(60)@0, on(60)@10, off(60)@20, off(60)@30
    // FIFO empareja [0,20] y [10,30]; LIFO emparejaria [10,20] y [0,30].
    // Tras el acortado por solapamiento queda [0,10] y [10,30] con FIFO,
    // y [0,10] y [10,20] con LIFO: el final de la segunda nota los distingue.
    let raw = SmfBuilder::new(480)
        .track(|t| t.note_on(0, 60, 100).note_on(10, 60, 90).note_off(20, 60).note_off(30, 60))
        .build();
    let song = load_smf(&raw).expect("valida");
    assert_eq!(song.notes().len(), 2);
    assert_eq!(song.notes()[0].onset_tick, Ticks(0));
    assert_eq!(song.notes()[0].end_tick, Ticks(10), "acortada por el solapamiento");
    assert!(song.notes()[0].truncated);
    assert_eq!(song.notes()[1].onset_tick, Ticks(10));
    assert_eq!(song.notes()[1].end_tick, Ticks(30), "FIFO: cierra con el note-off tardio");
    assert_eq!(song.report().truncated_overlaps, 1);
}

#[test]
fn una_nota_colgada_se_cierra_al_final_de_su_pista_y_se_cuenta() {
    let raw = SmfBuilder::new(480)
        .track(|t| t.note_on(0, 60, 100).note(240, 64, 100, 240))
        .build();
    let song = load_smf(&raw).expect("una nota colgada no invalida la cancion");
    let colgada = song.notes().iter().find(|n| n.key == 60).expect("la nota 60");
    assert_eq!(colgada.closure, Closure::HangingClosedAtTrackEnd);
    assert_eq!(colgada.end_tick, Ticks(480), "cerrada en el End of Track de su pista");
    assert_eq!(song.report().hanging_notes, 1);
}

#[test]
fn un_note_off_huerfano_se_ignora_y_se_cuenta() {
    let raw = SmfBuilder::new(480)
        .track(|t| t.note_off(0, 60).note(240, 64, 100, 240))
        .build();
    let song = load_smf(&raw).expect("valida");
    assert_eq!(song.notes().len(), 1, "el huerfano no genera nota");
    assert_eq!(song.notes()[0].key, 64);
    assert_eq!(song.report().orphan_note_offs, 1);
}

#[test]
fn la_misma_altura_en_canales_distintos_son_notas_independientes() {
    let raw = SmfBuilder::new(480)
        .track(|t| {
            t.raw(0, &[0x90, 60, 100]) // canal 0
                .raw(0, &[0x91, 60, 100]) // canal 1
                .raw(480, &[0x80, 60, 0])
                .raw(960, &[0x81, 60, 0])
        })
        .build();
    let song = load_smf(&raw).expect("valida");
    assert_eq!(song.notes().len(), 2);
    let c0 = song.notes().iter().find(|n| n.channel == 0).expect("canal 0");
    let c1 = song.notes().iter().find(|n| n.channel == 1).expect("canal 1");
    assert_eq!(c0.end_tick, Ticks(480), "cada canal cierra con el suyo");
    assert_eq!(c1.end_tick, Ticks(960));
    assert_eq!(song.report().truncated_overlaps, 0, "canales distintos no se solapan entre si");
}

#[test]
fn las_notas_de_percusion_y_fuera_del_piano_se_conservan_pero_se_cuentan() {
    let raw = SmfBuilder::new(480)
        .track(|t| {
            t.raw(0, &[0x99, 38, 100]).raw(240, &[0x89, 38, 0]) // canal 10 (indice 9)
                .note(0, 12, 100, 240) // por debajo de las 88 teclas
                .note(0, 60, 100, 240) // normal
        })
        .build();
    let song = load_smf(&raw).expect("valida");
    assert_eq!(song.notes().len(), 3, "no se filtra nada en esta entrega");
    assert_eq!(song.report().percussion_notes, 1);
    // La 38 SI cae dentro de 21..=108: la unica fuera de rango es la 12.
    assert_eq!(song.report().notes_out_of_88_range, 1, "solo la nota 12 esta fuera del piano");
}
