//! T046 — SC-001: mil notas se convierten en leccion en menos de 100 ms.
//!
//! El margen real es de dos ordenes de magnitud, asi que la prueba no es intermitente
//! aunque la maquina este cargada.

mod fixtures;
use fixtures::SmfBuilder;
use piano_core::feedforward::CueSchedule;
use piano_core::load_smf;

#[test]
fn mil_notas_se_convierten_en_leccion_en_menos_de_cien_milisegundos() {
    let raw = SmfBuilder::new(480)
        .track(|t| {
            let mut t = t.tempo(0, 500_000);
            for i in 0..50u64 {
                t = t.tempo(i * 960 + 480, 300_000 + (i as u32 % 7) * 25_000);
            }
            t
        })
        .track(|t| {
            let mut t = t;
            for i in 0..1_000u64 {
                t = t.note(i * 120, 21 + (i % 88) as u8, 64, 110);
            }
            t
        })
        .build();

    let arranque = std::time::Instant::now();
    let song = load_smf(&raw).expect("valida");
    let schedule = CueSchedule::build(&song, 240);
    let transcurrido = arranque.elapsed();

    assert_eq!(song.notes().len(), 1_000);
    assert_eq!(schedule.len(), 1_000);
    assert!(
        transcurrido.as_millis() < 100,
        "tardo {} ms en convertir 1.000 notas",
        transcurrido.as_millis()
    );
}
