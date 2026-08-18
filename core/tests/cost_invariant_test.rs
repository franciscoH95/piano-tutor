//! T045 — SC-006: el coste de emitir avisos no depende del tamano de la cancion.
//!
//! Se cuentan **comparaciones**, no milisegundos. Medir tiempo de reloj haria la prueba
//! intermitente en una maquina cargada y no demostraria la propiedad estructural.

mod fixtures;
use fixtures::SmfBuilder;
use piano_core::feedforward::{CueSchedule, Playback};
use piano_core::load_smf;
use piano_core::time::Micros;
use std::sync::Arc;

/// Una cancion con `n` notas espaciadas media negra a PPQ 480 (250 ms cada una).
fn cancion_de(n: u64) -> piano_core::Song {
    let raw = SmfBuilder::new(480)
        .track(|t| t.tempo(0, 500_000))
        .track(|t| {
            let mut t = t;
            for i in 0..n {
                t = t.note(i * 240, 21 + (i % 88) as u8, 64, 120);
            }
            t
        })
        .build();
    load_smf(&raw).expect("valida")
}

fn reproduccion(song: &piano_core::Song) -> Playback {
    Playback::new(Arc::new(CueSchedule::build(song, 0)))
}

#[test]
fn emitir_los_mismos_avisos_cuesta_lo_mismo_en_una_cancion_corta_que_en_una_larga() {
    let corta = cancion_de(10);
    let larga = cancion_de(10_000);
    assert_eq!(corta.notes().len(), 10);
    assert_eq!(larga.notes().len(), 10_000);

    // Instante que cubre exactamente tres avisos en ambas: 0, 250 ms y 500 ms.
    let hasta = Micros(500_000);

    let mut a = reproduccion(&corta);
    let mut b = reproduccion(&larga);
    assert_eq!(a.advance_to(hasta).expect("valido").len(), 3);
    assert_eq!(b.advance_to(hasta).expect("valido").len(), 3);

    assert_eq!(
        a.last_advance_comparisons(),
        b.last_advance_comparisons(),
        "una cancion 1.000 veces mas larga costo un numero distinto de comparaciones"
    );
}

#[test]
fn el_coste_es_exactamente_los_avisos_emitidos_mas_uno() {
    let song = cancion_de(10_000);
    let mut pb = reproduccion(&song);
    for hasta in [0u64, 250_000, 1_000_000, 5_000_000] {
        let k = pb.advance_to(Micros(hasta)).expect("valido").len();
        assert_eq!(
            pb.last_advance_comparisons(),
            k + 1,
            "emitir {k} avisos debe costar {} comparaciones",
            k + 1
        );
    }
}

#[test]
fn agotar_la_cancion_no_recorre_lo_ya_emitido() {
    let song = cancion_de(5_000);
    let mut pb = reproduccion(&song);
    let mut total_comparaciones = 0usize;
    let mut total_emitidos = 0usize;
    for paso in 1..=100u64 {
        total_emitidos += pb.advance_to(Micros(paso * 25_000)).expect("valido").len();
        total_comparaciones += pb.last_advance_comparisons();
    }
    // Si cada avance reescaneara desde el principio, el coste seria cuadratico.
    assert!(
        total_comparaciones <= total_emitidos + 200,
        "{total_comparaciones} comparaciones para {total_emitidos} avisos: se esta reescaneando"
    );
}
