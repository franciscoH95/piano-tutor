//! T004: prueba del propio constructor de fixtures.
//!
//! Un constructor de fixtures roto invalida en silencio todas las demas pruebas, asi que
//! el constructor se prueba a si mismo antes de que nadie lo use.

mod fixtures;
use fixtures::SmfBuilder;

#[test]
fn cabecera_mthd_correcta() {
    let raw = SmfBuilder::new(480).track(|t| t).build();
    assert_eq!(&raw[0..4], b"MThd", "magic de cabecera");
    assert_eq!(&raw[4..8], &6u32.to_be_bytes(), "longitud de cabecera siempre 6");
    assert_eq!(&raw[8..10], &1u16.to_be_bytes(), "formato 1");
    assert_eq!(&raw[10..12], &1u16.to_be_bytes(), "una pista declarada");
    assert_eq!(&raw[12..14], &480u16.to_be_bytes(), "PPQ");
}

#[test]
fn el_numero_de_pistas_declarado_coincide_con_las_reales() {
    let raw = SmfBuilder::new(96).track(|t| t).track(|t| t).track(|t| t).build();
    assert_eq!(&raw[10..12], &3u16.to_be_bytes());
    assert_eq!(raw.windows(4).filter(|w| *w == b"MTrk").count(), 3);
}

#[test]
fn una_nota_produce_note_on_y_note_off_con_deltas_correctos() {
    // Nota en el tick 0, duracion 480.
    let raw = SmfBuilder::new(480).track(|t| t.note(0, 60, 100, 480)).build();
    let trk = raw.windows(4).position(|w| w == b"MTrk").expect("debe haber MTrk");
    let body = &raw[trk + 8..];
    // delta 0, note-on canal 0, nota 60, vel 100
    assert_eq!(&body[0..4], &[0x00, 0x90, 60, 100], "note-on");
    // delta 480 (VLQ: 0x83 0x60), note-off canal 0, nota 60, vel 0
    assert_eq!(&body[4..9], &[0x83, 0x60, 0x80, 60, 0], "note-off tras 480 ticks");
    // meta End of Track
    assert_eq!(&body[9..13], &[0x00, 0xFF, 0x2F, 0x00], "end of track");
}

#[test]
fn un_acorde_produce_notas_simultaneas() {
    let raw = SmfBuilder::new(480).track(|t| t.chord(0, &[60, 64, 67], 100, 480)).build();
    let trk = raw.windows(4).position(|w| w == b"MTrk").expect("debe haber MTrk");
    let body = &raw[trk + 8..];
    // tres note-on con delta 0 consecutivos
    assert_eq!(&body[0..4], &[0x00, 0x90, 60, 100]);
    assert_eq!(&body[4..8], &[0x00, 0x90, 64, 100]);
    assert_eq!(&body[8..12], &[0x00, 0x90, 67, 100]);
}

#[test]
fn el_tempo_se_codifica_como_meta_set_tempo_de_tres_bytes() {
    let raw = SmfBuilder::new(480).track(|t| t.tempo(0, 500_000)).build();
    let trk = raw.windows(4).position(|w| w == b"MTrk").expect("debe haber MTrk");
    let body = &raw[trk + 8..];
    assert_eq!(&body[0..7], &[0x00, 0xFF, 0x51, 0x03, 0x07, 0xA1, 0x20]);
}

#[test]
fn los_deltas_usan_vlq_para_valores_grandes() {
    // 0x0FFFFF = 1048575 -> VLQ de 3 bytes: 0xBF 0xFF 0x7F
    let raw = SmfBuilder::new(480).track(|t| t.note(0x0F_FFFF, 60, 100, 1)).build();
    let trk = raw.windows(4).position(|w| w == b"MTrk").expect("debe haber MTrk");
    let body = &raw[trk + 8..];
    assert_eq!(&body[0..3], &[0xBF, 0xFF, 0x7F], "delta VLQ de 3 bytes");
}

#[test]
fn la_longitud_declarada_del_chunk_coincide_con_su_contenido() {
    let raw = SmfBuilder::new(480).track(|t| t.note(0, 60, 100, 480)).build();
    let trk = raw.windows(4).position(|w| w == b"MTrk").expect("debe haber MTrk");
    let len = u32::from_be_bytes([raw[trk + 4], raw[trk + 5], raw[trk + 6], raw[trk + 7]]) as usize;
    assert_eq!(raw.len(), trk + 8 + len, "el chunk llega justo hasta el final");
}
