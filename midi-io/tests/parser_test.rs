//! T023-T025 — el analizador de mensajes MIDI.
//!
//! La prueba que de verdad importa es `ningun_paquete_malformado_provoca_panico`: es el
//! fallo exacto por el que se descarto `midir`, que ante un paquete truncado aborta el
//! proceso entero.

use piano_core::capture::{Observacion, TipoEvento};
use piano_core::time::Micros;
use piano_midi_io::parser::Parser;

fn analizar(p: &mut Parser, bytes: &[u8]) -> Vec<Observacion> {
    let mut v = Vec::new();
    p.consumir(Micros(1_000), bytes, |o| v.push(o));
    v
}

#[test]
fn un_note_on_produce_un_ataque() {
    let v = analizar(&mut Parser::nuevo(), &[0x90, 60, 100]);
    assert_eq!(v.len(), 1);
    assert_eq!((v[0].key, v[0].velocity, v[0].kind, v[0].channel), (60, 100, TipoEvento::Ataque, 0));
    assert_eq!(v[0].at, Micros(1_000), "todas las notas del paquete comparten instante");
}

#[test]
fn un_note_off_produce_una_suelta() {
    let v = analizar(&mut Parser::nuevo(), &[0x85, 64, 40]);
    assert_eq!(v.len(), 1);
    assert_eq!((v[0].key, v[0].kind, v[0].channel), (64, TipoEvento::Suelta, 5));
}

#[test]
fn un_note_on_con_velocity_cero_es_una_suelta() {
    // FR-009. El estandar lo define asi y ningun parseador lo aplica por nosotros.
    let v = analizar(&mut Parser::nuevo(), &[0x90, 60, 0]);
    assert_eq!(v.len(), 1);
    assert_eq!(v[0].kind, TipoEvento::Suelta);
}

#[test]
fn un_acorde_en_un_solo_paquete_comparte_instante() {
    let v = analizar(&mut Parser::nuevo(), &[0x90, 60, 100, 0x90, 64, 100, 0x90, 67, 100]);
    assert_eq!(v.len(), 3);
    assert!(v.iter().all(|o| o.at == Micros(1_000)), "un paquete, un instante");
    assert_eq!(v.iter().map(|o| o.key).collect::<Vec<_>>(), vec![60, 64, 67]);
}

#[test]
fn el_estado_en_carrera_se_interpreta_bien() {
    let mut p = Parser::nuevo();
    // Un solo byte de estado y tres notas colgando de el.
    let v = analizar(&mut p, &[0x90, 60, 100, 64, 100, 67, 0]);
    assert_eq!(v.len(), 3);
    assert_eq!(v[0].kind, TipoEvento::Ataque);
    assert_eq!(v[1].kind, TipoEvento::Ataque);
    assert_eq!(v[2].kind, TipoEvento::Suelta, "velocity 0 tambien en estado en carrera");
}

#[test]
fn el_estado_en_carrera_persiste_entre_paquetes() {
    let mut p = Parser::nuevo();
    assert_eq!(analizar(&mut p, &[0x90, 60, 100]).len(), 1);
    let v = analizar(&mut p, &[64, 100]); // sin byte de estado
    assert_eq!(v.len(), 1);
    assert_eq!(v[0].key, 64);
}

#[test]
fn el_pedal_y_los_demas_mensajes_se_descartan_sin_estorbar_a_las_notas() {
    // FR-014, y las notas entremezcladas deben sobrevivir.
    let mut p = Parser::nuevo();
    let v = analizar(&mut p, &[
        0xB0, 64, 127,      // pedal de resonancia
        0x90, 60, 100,      // nota
        0xA0, 60, 80,       // aftertouch polifonico
        0xC0, 5,            // cambio de instrumento
        0xE0, 0x00, 0x40,   // rueda de tono
        0xD0, 90,           // presion de canal
        0x90, 64, 100,      // nota
    ]);
    assert_eq!(v.len(), 2, "solo las dos notas");
    assert_eq!(v.iter().map(|o| o.key).collect::<Vec<_>>(), vec![60, 64]);
}

#[test]
fn los_mensajes_de_tiempo_real_no_rompen_el_estado_en_carrera() {
    let mut p = Parser::nuevo();
    // 0xF8 (reloj) puede aparecer en cualquier punto y no cancela el estado en carrera.
    let v = analizar(&mut p, &[0x90, 60, 100, 0xF8, 64, 100]);
    assert_eq!(v.len(), 2);
    assert_eq!(v[1].key, 64);
}

#[test]
fn un_sysex_se_salta_entero() {
    let mut p = Parser::nuevo();
    let v = analizar(&mut p, &[0xF0, 0x7E, 0x00, 0x06, 0x01, 0xF7, 0x90, 60, 100]);
    assert_eq!(v.len(), 1);
    assert_eq!(v[0].key, 60);
}

#[test]
fn ningun_paquete_malformado_provoca_panico() {
    // T024 — LA prueba. Estos son los casos exactos que hacen abortar el proceso con
    // `midir 0.11.0` (issue #160). Aqui no pueden ni escribirse: el analizador vive bajo
    // `deny(clippy::indexing_slicing)`.
    let casos: &[&[u8]] = &[
        &[],
        &[0x90],
        &[0x90, 0x3C],
        &[0x80, 0x3C],
        &[0xB0],
        &[0xC0],
        &[0xE0, 0x00],
        &[0x90, 0x3C, 0x64, 0x90],       // nota valida + byte de estado suelto
        &[0x90, 0x3C, 0x64, 0x90, 0x3C], // nota valida + nota truncada
        &[0xF0],                          // sysex sin terminar
        &[0xF7],                          // fin de sysex huerfano
        &[0x3C, 0x64],                    // datos sin ningun estado previo
        &[0xFF],
        &[0x00],
    ];
    for caso in casos {
        let mut p = Parser::nuevo();
        let _ = analizar(&mut p, caso); // no debe entrar en panico
    }
}

#[test]
fn una_nota_valida_seguida_de_basura_entrega_la_nota() {
    // No basta con no panicar: lo que si es valido debe llegar.
    let v = analizar(&mut Parser::nuevo(), &[0x90, 60, 100, 0x90, 60]);
    assert_eq!(v.len(), 1, "la nota completa se entrega; la truncada se descarta");
    assert_eq!(v[0].key, 60);
}

#[test]
fn barrer_todos_los_bytes_posibles_no_provoca_panico() {
    // Fuzz exhaustivo de un byte y de dos, mas paquetes largos aleatorios deterministas.
    for a in 0u16..=255 {
        let mut p = Parser::nuevo();
        let _ = analizar(&mut p, &[a as u8]);
        for b in 0u16..=255 {
            let mut p = Parser::nuevo();
            let _ = analizar(&mut p, &[a as u8, b as u8]);
        }
    }
    // Secuencia pseudoaleatoria determinista (LCG), sin depender de ninguna semilla externa.
    let mut x: u32 = 0x1234_5678;
    let mut buf = Vec::with_capacity(64);
    for _ in 0..2_000 {
        buf.clear();
        let n = (x >> 24) % 40;
        for _ in 0..n {
            x = x.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            buf.push((x >> 16) as u8);
        }
        let mut p = Parser::nuevo();
        let _ = analizar(&mut p, &buf);
        x = x.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
    }
}

#[test]
fn los_bytes_de_datos_se_enmascaran_a_siete_bits() {
    // Un flujo malformado podria colar un valor >= 0x80 donde iba un dato. Enmascarar lo
    // mantiene dentro del rango real de notas del estandar.
    let mut w = Vec::new();
    Parser::nuevo().consumir(Micros(0), &[0x90, 200, 200], |o| w.push(o));
    assert_eq!(w.len(), 1);
    assert_eq!(w[0].key, 200 & 0x7F, "altura enmascarada a siete bits");
    assert_eq!(w[0].velocity, 200 & 0x7F, "intensidad enmascarada a siete bits");
}
