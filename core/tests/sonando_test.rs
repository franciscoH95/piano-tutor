//! T060 y T064 — qué está sonando en la canción.
//!
//! Las pruebas del **juicio** —acierto, nota extra, omitida— se retiraron con la feature
//! 004: ese veredicto lo decide ahora `piano_core::evaluacion`, y sus pruebas viven en
//! `evaluacion_test.rs`. Tenerlas aquí también habría dejado dos sitios afirmando cosas
//! sobre lo mismo, y el día que discrepasen no sabríamos cuál manda.

mod fixtures;
use fixtures::SmfBuilder;
use piano_core::load_smf;
use piano_core::practica::{ConjuntoSonando, MascaraTeclas};
use piano_core::time::Micros;
use piano_core::Song;

/// Un tick es un milisegundo, así que las cifras se leen en microsegundos ×1000.
fn cancion(notas: &[(u64, u8, u64)]) -> Song {
    let ns = notas.to_vec();
    let raw = SmfBuilder::new(1000)
        .track(|t| {
            let mut t = t.tempo(0, 1_000_000);
            for (tick, key, dur) in &ns {
                t = t.note(*tick, *key, 90, *dur);
            }
            t
        })
        .build();
    load_smf(&raw).expect("valida")
}

// ---------------------------------------------------------------- máscara

#[test]
fn la_mascara_cubre_las_ciento_veintiocho_teclas() {
    let mut m = MascaraTeclas::VACIA;
    for k in 0..128u8 {
        assert!(!m.contiene(k), "la tecla {k} empieza suelta");
        m.poner(k);
        assert!(m.contiene(k), "la tecla {k} se puede poner");
    }
    assert_eq!(m.cuenta(), 128);
    for k in 0..128u8 {
        m.quitar(k);
    }
    assert_eq!(m.cuenta(), 0);
}

#[test]
fn poner_dos_veces_la_misma_tecla_no_la_duplica() {
    // Un teclado real repite el ataque si la tecla se mantiene, según el modelo.
    let mut m = MascaraTeclas::VACIA;
    m.poner(60);
    m.poner(60);
    assert_eq!(m.cuenta(), 1);
    m.quitar(60);
    assert!(!m.contiene(60), "y una sola soltada la libera");
}

#[test]
fn soltar_una_tecla_que_no_estaba_pulsada_no_rompe_nada() {
    let mut m = MascaraTeclas::VACIA;
    m.quitar(60);
    assert_eq!(m.cuenta(), 0);
}

// ---------------------------------------------------------------- qué suena

#[test]
fn una_nota_suena_entre_su_ataque_y_su_final_sin_ninguna_tolerancia() {
    // FR-014b. Los extremos: cerrado en el ataque, abierto en el final. Es el mismo
    // convenio que ya usa `vista.rs` para marcar una nota como sonando, y tiene que ser el
    // mismo o la misma nota estaría sonando para una parte del núcleo y no para otra.
    let song = cancion(&[(1_000, 60, 1_000)]); // 1.000.000 → 2.000.000 µs
    let mut c = ConjuntoSonando::nuevo(&song);

    for (pos, esperado) in [
        (999_999u64, false),
        (1_000_000, true),  // justo en el ataque: suena
        (1_500_000, true),
        (1_999_999, true),
        (2_000_000, false), // justo en el final: ya no
    ] {
        c.avanzar(&song, Micros(pos));
        assert_eq!(c.suena(60), esperado, "posición {pos}");
    }
}

#[test]
fn dos_notas_seguidas_de_la_misma_tecla_no_dejan_hueco_ni_se_solapan() {
    // El final de una es el ataque de la siguiente. Con los extremos al revés habría un
    // microsegundo de silencio, o uno en que la tecla sonaría "dos veces".
    let song = cancion(&[(0, 60, 1_000), (1_000, 60, 1_000)]);
    let mut c = ConjuntoSonando::nuevo(&song);
    for pos in [0u64, 999_999, 1_000_000, 1_000_001, 1_999_999] {
        c.avanzar(&song, Micros(pos));
        assert!(c.suena(60), "la tecla 60 suena sin interrupción en {pos}");
    }
    c.avanzar(&song, Micros(2_000_000));
    assert!(!c.suena(60), "y calla al terminar la segunda");
}

#[test]
fn un_pedal_largo_sigue_sonando_bajo_las_notas_posteriores() {
    // El mismo fallo estructural que tuvo `Vista::reposicionar`: las notas están ordenadas
    // por ATAQUE, así que una nota larga que empezó pronto termina después de otras que
    // empezaron más tarde. Buscar por el final rompe.
    let song = cancion(&[
        (0, 36, 30_000),   // pedal: 0 → 30.000.000
        (10_000, 72, 500), // 10.000.000 → 10.500.000
        (20_000, 74, 500),
    ]);
    let mut c = ConjuntoSonando::nuevo(&song);
    c.avanzar(&song, Micros(20_200_000));
    assert!(c.suena(36), "el pedal sigue sonando veinte segundos después");
    assert!(c.suena(74), "y la nota corta también");
    assert!(!c.suena(72), "la que ya terminó, no");
}

#[test]
fn recolocar_hacia_atras_vuelve_a_encontrar_el_pedal() {
    let song = cancion(&[(0, 36, 30_000), (10_000, 72, 500)]);
    let mut c = ConjuntoSonando::nuevo(&song);
    c.avanzar(&song, Micros(25_000_000));
    c.recolocar(&song, Micros(5_000_000));
    assert!(c.suena(36), "tras volver atrás, el pedal se ve otra vez");
}

// ---------------------------------------------------------------- las tres situaciones



// ---------------------------------------------------------------- coste

#[test]
fn el_coste_de_la_consulta_no_crece_con_el_tamano_de_la_cancion() {
    // T064. Se cuenta, no se cronometra: cronometrar sería intermitente y no demostraría
    // nada estructural.
    //
    // La ventana recorrida y la densidad de notas se mantienen FIJAS y solo crece la
    // longitud de la canción. Recorrer media pieza no serviría: pasar por la mitad de las
    // notas cuesta la mitad de las notas por definición, y esa prueba fallaría con
    // cualquier implementación, incluida la correcta.
    let examinadas_con = |total: u64| -> usize {
        let notas: Vec<(u64, u8, u64)> =
            (0..total).map(|i| (i * 100, 60 + (i % 12) as u8, 90)).collect();
        let song = cancion(&notas);
        let mut c = ConjuntoSonando::nuevo(&song);
        // Siempre el mismo segundo de música, sea la canción larga o corta.
        for paso in 0..100u64 {
            c.avanzar(&song, Micros(paso * 10_000));
        }
        c.examinadas()
    };
    let corta = examinadas_con(200); // 20 segundos
    let larga = examinadas_con(20_000); // 33 minutos, cien veces más
    assert_eq!(
        corta, larga,
        "cien veces más canción no puede costar más para recorrer el mismo segundo"
    );
}

// ---------------------------------------------------------------- la nota omitida







#[test]
fn una_sola_nota_larga_no_le_cobra_peaje_al_resto_de_la_pieza() {
    // La cota de duración es un recurso de la BÚSQUEDA BINARIA de `recolocar`, donde no se
    // tiene la nota en la mano y hace falta un predicado monótono. En el avance por
    // fotograma sí se tiene, así que rige el criterio exacto: se deja atrás una nota solo
    // cuando ha terminado de verdad.
    //
    // Con la cota, un pedal de 30 segundos al principio dejaba el cursor treinta segundos
    // retrasado **durante los diez minutos siguientes**, mucho después de que el pedal
    // hubiera terminado. Medido: 118 notas examinadas por fotograma en vez de 5.
    //
    // T064 no puede ver esto: con duraciones uniformes los dos criterios dan exactamente
    // el mismo número, y por eso hace falta esta prueba aparte, con una nota larga.
    let raw = SmfBuilder::new(1000)
        .track(|t| {
            let mut t = t.tempo(0, 1_000_000);
            t = t.note(0, 36, 90, 30_000); // pedal: 30 segundos
            for i in 0..2_400u64 {
                t = t.note(i * 250, 60 + (i % 12) as u8, 90, 200);
            }
            t
        })
        .build();
    let song = load_smf(&raw).expect("valida");
    let mut c = ConjuntoSonando::nuevo(&song);
    const FOTOGRAMAS: u64 = 36_000; // diez minutos a 60 Hz
    for f in 0..FOTOGRAMAS {
        c.avanzar(&song, Micros(f * 16_667));
    }
    let por_fotograma = c.examinadas() as u64 / FOTOGRAMAS;
    println!("  examinadas por fotograma: {por_fotograma} (total {})", c.examinadas());
    assert!(
        por_fotograma < 15,
        "se examinan {por_fotograma} notas por fotograma; el pedal le está cobrando peaje \
         a toda la pieza (total {})",
        c.examinadas()
    );
}


#[test]
fn retroceder_sin_recolocar_a_mano_sigue_dando_la_respuesta_correcta() {
    // El cursor solo avanza, así que llamar a `avanzar` con una posición anterior dejaría
    // el conjunto mirando por delante de notas que sí suenan, y devolvería `false` sin que
    // nada fallase. Que el llamante «tenga que acordarse» de recolocar es un pie de banco:
    // se detecta solo.
    let song = cancion(&[(0, 60, 1_000), (5_000, 64, 1_000)]);
    let mut c = ConjuntoSonando::nuevo(&song);

    c.avanzar(&song, Micros(5_500_000));
    assert!(c.suena(64));

    c.avanzar(&song, Micros(500_000)); // hacia atrás, sin recolocar a mano
    assert!(c.suena(60), "la primera nota vuelve a sonar");
    assert!(!c.suena(64), "y la segunda todavía no ha empezado");
}
