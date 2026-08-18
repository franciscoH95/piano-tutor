//! T009 — la única puerta de rendimiento de esta feature que puede bloquear un cambio.
//!
//! Las cinco cifras de SC-003 necesitan una ventana real en pantalla, y está medido que sin
//! ella el sistema no dibuja ni un fotograma. Esta prueba, en cambio, es determinista y
//! headless: mide lo que cuesta **decidir qué dibujar**, que es la parte que sí controlamos
//! y la única que una regresión nuestra puede empeorar.

mod fixtures;
use fixtures::SmfBuilder;
use piano_core::load_smf;
use piano_core::practica::{vista, NotaVisible, Vista};
use piano_core::time::Micros;

/// Pieza densa y larga: acordes de cinco notas cada 100 ms durante 10 minutos.
fn pieza_densa() -> piano_core::Song {
    let raw = SmfBuilder::new(480)
        .track(|t| t.tempo(0, 500_000))
        .track(|t| {
            let mut t = t;
            for i in 0..2_000u64 {
                let base = 40 + (i % 40) as u8;
                t = t.chord(i * 96, &[base, base + 4, base + 7, base + 12, base + 16], 90, 90);
            }
            t
        })
        .build();
    load_smf(&raw).expect("valida")
}

#[test]
fn producir_la_escena_de_un_fotograma_cabe_de_sobra_en_el_presupuesto() {
    let song = pieza_densa();
    assert_eq!(song.notes().len(), 10_000, "la pieza de prueba debe ser densa de verdad");

    let mut v = Vista::nueva();
    let mut out: Vec<NotaVisible> = Vec::with_capacity(1_024);

    // Recorrer diez minutos a sesenta fotogramas por segundo son 36.000 escenas.
    const FOTOGRAMAS: u64 = 36_000;
    const PASO_US: u64 = 16_667;
    const VENTANA_US: u64 = 2_000_000; // dos segundos de anticipación visible

    let inicio = std::time::Instant::now();
    let mut maximo_visible = 0usize;
    for f in 0..FOTOGRAMAS {
        let pos = Micros(f * PASO_US);
        out.clear();
        vista(&song, &mut v, pos, pos, Micros(pos.0 + VENTANA_US), &mut out);
        maximo_visible = maximo_visible.max(out.len());
    }
    let total = inicio.elapsed();
    // En nanosegundos: en microsegundos el resultado sale 0 y deja de servir como
    // referencia para detectar una regresión futura.
    let por_fotograma_ns = total.as_nanos() / u128::from(FOTOGRAMAS);

    assert!(maximo_visible > 20, "la ventana debe contener notas de verdad: {maximo_visible}");
    // El presupuesto de un fotograma entero son 16.700 µs, y de eso el dibujo se lleva la
    // mayor parte. Decidir qué dibujar debe costar dos órdenes de magnitud menos.
    assert!(
        por_fotograma_ns < 167_000,
        "producir la escena costó {por_fotograma_ns} ns por fotograma, más del 1 % del \
         presupuesto de 16.700.000 ns (máximo visible: {maximo_visible} notas)"
    );
    println!("  escena: {por_fotograma_ns} ns/fotograma · máx {maximo_visible} notas visibles");
}

#[test]
fn reposicionar_tras_un_salto_es_barato() {
    // SC-008a exige menos de 100 ms para saltar. Aquí se mide solo la parte de la vista.
    let song = pieza_densa();
    let mut v = Vista::nueva();
    let mut out = Vec::new();
    // Calentar el máximo de duración recorriendo un poco.
    vista(&song, &mut v, Micros(0), Micros(0), Micros(2_000_000), &mut out);

    let inicio = std::time::Instant::now();
    for salto in 0..1_000u64 {
        let pos = Micros((salto % 600) * 1_000_000);
        v.reposicionar(&song, pos);
        out.clear();
        vista(&song, &mut v, pos, pos, Micros(pos.0 + 2_000_000), &mut out);
    }
    let por_salto_ns = inicio.elapsed().as_nanos() / 1_000;
    assert!(
        por_salto_ns < 1_000_000,
        "reposicionar costó {por_salto_ns} ns, contra un presupuesto de 100.000.000 ns"
    );
    println!("  salto: {por_salto_ns} ns");
}
