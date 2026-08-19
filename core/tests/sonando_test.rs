//! T060, T062 y T064 — qué está sonando y qué toca el alumno.

mod fixtures;
use fixtures::SmfBuilder;
use piano_core::load_smf;
use piano_core::practica::{ConjuntoSonando, MascaraTeclas, Situacion};
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

#[test]
fn se_distingue_el_acierto_de_la_nota_extra() {
    // FR-014a, primeras dos situaciones.
    let song = cancion(&[(0, 60, 1_000)]);
    let mut c = ConjuntoSonando::nuevo(&song);
    c.avanzar(&song, Micros(500_000));
    assert_eq!(c.clasificar(60), Situacion::Acierto);
    assert_eq!(c.clasificar(61), Situacion::Extra);
    assert_eq!(c.clasificar(59), Situacion::Extra);
}

#[test]
fn barriendo_las_ciento_veintiocho_teclas_todas_se_clasifican_bien() {
    // SC-005a. Se barren **las 128**, no solo las que la canción usa: una implementación
    // que devolviera siempre "extra" pasaría una prueba que solo mirase las ausentes, y
    // una que devolviera siempre "acierto" pasaría la que solo mirase las presentes.
    let song = cancion(&[
        (0, 60, 2_000),
        (500, 64, 2_000),
        (1_000, 67, 3_000),
        (3_000, 72, 1_000),
    ]);
    let mut c = ConjuntoSonando::nuevo(&song);

    for pos_ms in [0u64, 400, 700, 1_200, 2_100, 2_600, 3_500, 4_500] {
        let pos = pos_ms * 1_000;
        c.avanzar(&song, Micros(pos));
        // Verdad calculada aparte, a fuerza bruta sobre la canción entera.
        let esperadas: Vec<u8> = song
            .notes()
            .iter()
            .filter(|n| n.onset_us.0 <= pos && pos < n.end_us.0)
            .map(|n| n.key)
            .collect();

        let mut aciertos = 0;
        let mut extras = 0;
        for k in 0..128u8 {
            match c.clasificar(k) {
                Situacion::Acierto => {
                    assert!(esperadas.contains(&k), "en {pos} la tecla {k} no sonaba");
                    aciertos += 1;
                }
                Situacion::Extra => {
                    assert!(!esperadas.contains(&k), "en {pos} la tecla {k} sí sonaba");
                    extras += 1;
                }
            }
        }
        assert_eq!(aciertos + extras, 128, "las 128 se clasifican, ninguna se queda fuera");
        assert_eq!(aciertos, esperadas.len(), "y son exactamente las que suenan en {pos}");
    }
}

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
fn una_nota_que_nadie_toco_se_declara_omitida_al_terminar() {
    // FR-014a, tercera situación. No se puede afirmar en el ataque: solo cuando su duración
    // ha pasado entera.
    let song = cancion(&[(0, 60, 1_000)]); // 0 → 1.000.000
    let mut c = ConjuntoSonando::nuevo(&song);
    let mut out = Vec::new();

    c.avanzar(&song, Micros(500_000));
    c.registrar(&song, MascaraTeclas::VACIA, Micros(500_000));
    c.omitidas(&song, Micros(500_000), &mut out);
    assert!(out.is_empty(), "a mitad de la nota todavía no se sabe");

    c.avanzar(&song, Micros(1_000_000));
    c.registrar(&song, MascaraTeclas::VACIA, Micros(1_000_000));
    c.omitidas(&song, Micros(1_000_000), &mut out);
    assert_eq!(out, vec![0], "terminó sin que nadie la tocara");
}

#[test]
fn una_tecla_mantenida_cuenta_aunque_no_llegue_ningun_ataque_nuevo() {
    // **La trampa de esta funcionalidad.** En un teclado real, una tecla que se mantiene
    // pulsada no genera eventos nuevos. Una implementación que solo mirase los ataques
    // declararía omitida una nota que el alumno está tocando en ese preciso momento.
    let song = cancion(&[(1_000, 60, 1_000)]); // 1.000.000 → 2.000.000
    let mut c = ConjuntoSonando::nuevo(&song);
    let mut pulsadas = MascaraTeclas::VACIA;
    pulsadas.poner(60); // pulsada ANTES del ataque, y nunca se vuelve a pulsar

    for pos in [500_000u64, 1_000_000, 1_500_000, 2_000_000] {
        c.avanzar(&song, Micros(pos));
        c.registrar(&song, pulsadas, Micros(pos));
    }
    let mut out = Vec::new();
    c.omitidas(&song, Micros(2_000_000), &mut out);
    assert!(out.is_empty(), "el alumno la estaba tocando: no es omitida");
}

#[test]
fn tocarla_en_el_ultimo_microsegundo_basta() {
    let song = cancion(&[(0, 60, 1_000)]);
    let mut c = ConjuntoSonando::nuevo(&song);
    let mut pulsadas = MascaraTeclas::VACIA;
    pulsadas.poner(60);

    c.avanzar(&song, Micros(999_999));
    c.registrar(&song, pulsadas, Micros(999_999));
    c.avanzar(&song, Micros(1_000_000));
    c.registrar(&song, MascaraTeclas::VACIA, Micros(1_000_000));

    let mut out = Vec::new();
    c.omitidas(&song, Micros(1_000_000), &mut out);
    assert!(out.is_empty(), "sonaba y estaba pulsada, aunque fuese al final");
}

#[test]
fn la_omision_se_comunica_una_sola_vez() {
    // Como el final de la canción: si se repitiera, el puente llevaría sesenta avisos por
    // segundo de una nota que ya pasó.
    let song = cancion(&[(0, 60, 1_000)]);
    let mut c = ConjuntoSonando::nuevo(&song);
    let mut total = 0;
    for i in 1..=60u64 {
        let pos = 1_000_000 + i * 16_667;
        c.avanzar(&song, Micros(pos));
        c.registrar(&song, MascaraTeclas::VACIA, Micros(pos));
        let mut out = Vec::new();
        c.omitidas(&song, Micros(pos), &mut out);
        total += out.len();
    }
    assert_eq!(total, 1, "una sola vez en sesenta consultas");
}

#[test]
fn tocar_otra_tecla_no_salva_a_la_nota() {
    let song = cancion(&[(0, 60, 1_000)]);
    let mut c = ConjuntoSonando::nuevo(&song);
    let mut pulsadas = MascaraTeclas::VACIA;
    pulsadas.poner(61); // la de al lado

    for pos in [200_000u64, 600_000, 1_000_000] {
        c.avanzar(&song, Micros(pos));
        c.registrar(&song, pulsadas, Micros(pos));
    }
    let mut out = Vec::new();
    c.omitidas(&song, Micros(1_000_000), &mut out);
    assert_eq!(out, vec![0], "tocar al lado no cuenta");
}

#[test]
fn de_un_acorde_a_medias_solo_se_omite_lo_que_falto() {
    let song = cancion(&[(0, 60, 1_000), (0, 64, 1_000), (0, 67, 1_000)]);
    let mut c = ConjuntoSonando::nuevo(&song);
    let mut pulsadas = MascaraTeclas::VACIA;
    pulsadas.poner(60);
    pulsadas.poner(67);

    c.avanzar(&song, Micros(500_000));
    c.registrar(&song, pulsadas, Micros(500_000));
    c.avanzar(&song, Micros(1_000_000));
    c.registrar(&song, pulsadas, Micros(1_000_000));

    let mut out = Vec::new();
    c.omitidas(&song, Micros(1_000_000), &mut out);
    let teclas: Vec<u8> = out.iter().filter_map(|i| song.notes().get(*i)).map(|n| n.key).collect();
    assert_eq!(teclas, vec![64], "solo la que no se tocó");
}

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
fn una_pulsacion_satisface_a_todas_las_notas_de_esa_tecla_que_suenan() {
    // La misma tecla puede sonar en DOS notas a la vez: una melodía doblada en dos pistas
    // sobrevive solapada, porque el cargador solo acorta cuando coinciden pista, canal y
    // tecla. Como el alumno no puede pulsar dos veces la misma tecla a la vez, una sola
    // pulsación tiene que satisfacer a las dos; si no, en modo espera la puerta no abriría
    // jamás.
    let raw = SmfBuilder::new(1000)
        .track(|t| t.tempo(0, 1_000_000).note(0, 60, 90, 4_000)) // pedal 0 → 4 s
        .track(|t| {
            t.raw(2_000, &[0x91, 60, 90]).raw(3_000, &[0x81, 60, 0]) // 2 s → 3 s
        })
        .build();
    let song = load_smf(&raw).expect("valida");
    assert_eq!(song.notes().len(), 2, "las dos sobreviven sin acortarse");

    let mut c = ConjuntoSonando::nuevo(&song);
    let mut pulsadas = MascaraTeclas::VACIA;
    pulsadas.poner(60);

    for pos in [2_400_000u64, 2_500_000, 3_000_000, 4_000_000] {
        c.avanzar(&song, Micros(pos));
        c.registrar(&song, pulsadas, Micros(pos));
    }
    let mut out = Vec::new();
    c.omitidas(&song, Micros(4_000_000), &mut out);
    assert!(out.is_empty(), "una pulsación vale para las dos; omitidas: {out:?}");
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
