//! T071-T082 — el modo espera: la canción aguarda a que el alumno acierte.

mod fixtures;
use fixtures::SmfBuilder;
use piano_core::clock::{Clock, VirtualClock};
use piano_core::load_smf;
use piano_core::practica::{Avance, Cursor, Mano, MascaraTeclas, Velocidad};
use piano_core::time::Micros;
use piano_core::Song;

/// Un tick es un milisegundo.
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

fn teclas(ks: &[u8]) -> MascaraTeclas {
    let mut m = MascaraTeclas::VACIA;
    for k in ks {
        m.poner(*k);
    }
    m
}

/// Un cursor en modo espera, en marcha, con todas las notas de la derecha.
fn en_espera(song: &Song) -> (Cursor, VirtualClock) {
    let manos = vec![Mano::Derecha; song.notes().len()];
    let mut c = Cursor::nuevo_con_puertas(song, &manos, None);
    let reloj = VirtualClock::new();
    c.cambiar_avance(Avance::PorAcierto, reloj.now());
    c.poner_en_marcha(reloj.now());
    (c, reloj)
}

// ---------------------------------------------------------------- T071

#[test]
fn el_cursor_avanza_a_tempo_entre_notas_y_se_para_en_la_pendiente() {
    // FR-018 y FR-018a. **El tiempo entre notas transcurre de verdad**: es lo que deja al
    // alumno percibir si una nota es una redonda o una semicorchea aunque la canción le
    // espere. Saltar de nota en nota destruiría la figura rítmica, que es justo lo que se
    // está aprendiendo.
    let song = cancion(&[(0, 60, 500), (2_000, 64, 500)]);
    let (mut c, mut reloj) = en_espera(&song);

    // La primera puerta está en 0: hasta que no se acierte, no se mueve.
    reloj.set(Micros(500_000));
    c.avanzar_con(reloj.now(), MascaraTeclas::VACIA);
    assert_eq!(c.posicion(), Micros::ZERO, "espera en la primera nota");

    // Se acierta: a partir de aquí el reloj gobierna hasta la puerta siguiente.
    c.avanzar_con(reloj.now(), teclas(&[60]));
    reloj.set(Micros(1_000_000));
    c.avanzar_con(reloj.now(), teclas(&[60]));
    assert_eq!(c.posicion(), Micros(500_000), "medio segundo real, medio de canción");

    reloj.set(Micros(2_000_000));
    c.avanzar_con(reloj.now(), MascaraTeclas::VACIA);
    assert_eq!(c.posicion(), Micros(1_500_000), "sigue a tempo, sin saltarse el silencio");

    // Y se detiene exactamente en la puerta siguiente, no antes ni después.
    reloj.set(Micros(9_000_000));
    c.avanzar_con(reloj.now(), MascaraTeclas::VACIA);
    assert_eq!(c.posicion(), Micros(2_000_000), "espera en la segunda nota");
}

#[test]
fn la_duda_del_alumno_no_se_convierte_en_avance_de_cancion() {
    // Al abrirse la puerta, el ancla tiene que rebasarse en el instante del ACIERTO. Con el
    // de llegada a la puerta, los treinta segundos que el alumno tardó en decidirse se
    // convertirían en treinta segundos de canción de golpe.
    // La segunda nota dura de sobra: si la canción acabase antes, el techo del final
    // recortaría la posición y la prueba mediría eso en vez de lo que quiere medir.
    let song = cancion(&[(0, 60, 500), (1_000, 64, 4_000)]);
    let (mut c, mut reloj) = en_espera(&song);
    c.avanzar_con(reloj.now(), teclas(&[60]));

    reloj.set(Micros(1_000_000));
    c.avanzar_con(reloj.now(), MascaraTeclas::VACIA);
    assert_eq!(c.posicion(), Micros(1_000_000), "llegó a la segunda puerta");

    // Treinta segundos de duda.
    reloj.set(Micros(31_000_000));
    c.avanzar_con(reloj.now(), MascaraTeclas::VACIA);
    assert_eq!(c.posicion(), Micros(1_000_000), "sigue esperando, sin moverse");

    // Acierta por fin, y un segundo después solo ha avanzado un segundo.
    c.avanzar_con(reloj.now(), teclas(&[64]));
    reloj.set(Micros(32_000_000));
    c.avanzar_con(reloj.now(), teclas(&[64]));
    assert_eq!(c.posicion(), Micros(2_000_000), "un segundo real, un segundo de canción");
}

// ---------------------------------------------------------------- T074

#[test]
fn una_nota_equivocada_no_hace_avanzar_el_cursor() {
    // FR-019. Y se comunica sin interrumpir: la práctica sigue, el cursor no.
    let song = cancion(&[(0, 60, 500), (1_000, 64, 500)]);
    let (mut c, mut reloj) = en_espera(&song);

    reloj.set(Micros(500_000));
    let paso = c.avanzar_con(reloj.now(), teclas(&[61, 62, 71]));
    assert_eq!(c.posicion(), Micros::ZERO, "ninguna de las tres era la buena");
    assert!(paso.esperando, "sigue esperando");
    assert!(!paso.terminada, "y la práctica no se interrumpe");
}

#[test]
fn acertar_la_nota_con_otras_de_mas_tambien_abre_la_puerta() {
    // Tocar de más es un error del alumno, pero la nota pedida está pulsada: la puerta
    // exige que estén TODAS las suyas, no que no haya ninguna otra.
    let song = cancion(&[(0, 60, 500), (1_000, 64, 500)]);
    let (mut c, mut reloj) = en_espera(&song);
    reloj.set(Micros(500_000));
    c.avanzar_con(reloj.now(), teclas(&[60, 61]));
    reloj.set(Micros(700_000));
    c.avanzar_con(reloj.now(), teclas(&[60, 61]));
    assert!(c.posicion() > Micros::ZERO, "avanzó pese al dedo de más");
}

// ---------------------------------------------------------------- T075

#[test]
fn un_acorde_avanza_solo_con_todas_sus_notas_a_la_vez() {
    // FR-022. Un acorde es un gesto simultáneo; aceptarlo por partes enseñaría un hábito
    // que después hay que corregir.
    let song = cancion(&[(0, 60, 1_000), (0, 64, 1_000), (0, 67, 1_000), (2_000, 72, 500)]);
    let (mut c, mut reloj) = en_espera(&song);

    reloj.set(Micros(100_000));
    c.avanzar_con(reloj.now(), teclas(&[60, 64]));
    assert_eq!(c.posicion(), Micros::ZERO, "faltaba el sol");

    reloj.set(Micros(200_000));
    c.avanzar_con(reloj.now(), teclas(&[60, 64, 67]));
    reloj.set(Micros(300_000));
    c.avanzar_con(reloj.now(), teclas(&[60, 64, 67]));
    assert!(c.posicion() > Micros::ZERO, "con las tres a la vez sí");
}

#[test]
fn acertar_las_notas_de_un_acorde_una_tras_otra_no_basta() {
    // La otra mitad de FR-022, y la que importa: soltando entre medias **nunca** llegan a
    // coincidir, así que la puerta no debe abrirse por acumulación.
    let song = cancion(&[(0, 60, 1_000), (0, 64, 1_000), (0, 67, 1_000)]);
    let (mut c, mut reloj) = en_espera(&song);

    for (i, k) in [60u8, 64, 67].iter().enumerate() {
        reloj.set(Micros(100_000 * (i as u64 + 1)));
        c.avanzar_con(reloj.now(), teclas(&[*k])); // solo una, soltando la anterior
    }
    reloj.set(Micros(900_000));
    c.avanzar_con(reloj.now(), MascaraTeclas::VACIA);
    assert_eq!(c.posicion(), Micros::ZERO, "una tras otra no es un acorde");
}

#[test]
fn no_se_exige_precision_de_milisegundos_dentro_del_acorde() {
    // FR-022a: basta con que coincidan pulsadas en algún instante, aunque se hayan ido
    // pulsando poco a poco.
    let song = cancion(&[(0, 60, 2_000), (0, 64, 2_000), (2_000, 72, 500)]);
    let (mut c, mut reloj) = en_espera(&song);

    reloj.set(Micros(100_000));
    c.avanzar_con(reloj.now(), teclas(&[60]));
    reloj.set(Micros(800_000)); // 700 ms después, sin soltar la primera
    c.avanzar_con(reloj.now(), teclas(&[60, 64]));
    reloj.set(Micros(900_000));
    c.avanzar_con(reloj.now(), teclas(&[60, 64]));
    assert!(c.posicion() > Micros::ZERO, "coincidieron pulsadas, y con eso basta");
}

// ---------------------------------------------------------------- T077

#[test]
fn con_una_mano_elegida_las_notas_de_la_otra_no_abren_la_puerta() {
    // SC-012.
    let song = cancion(&[(0, 40, 1_000), (0, 72, 1_000), (2_000, 74, 500)]);
    let manos = vec![Mano::Izquierda, Mano::Derecha, Mano::Derecha];
    let mut c = Cursor::nuevo_con_puertas(&song, &manos, Some(Mano::Izquierda));
    let mut reloj = VirtualClock::new();
    c.cambiar_avance(Avance::PorAcierto, reloj.now());
    c.poner_en_marcha(reloj.now());

    reloj.set(Micros(500_000));
    c.avanzar_con(reloj.now(), teclas(&[72])); // la de la derecha
    assert_eq!(c.posicion(), Micros::ZERO, "esa nota no es de la mano practicada");

    reloj.set(Micros(600_000));
    c.avanzar_con(reloj.now(), teclas(&[40]));
    reloj.set(Micros(700_000));
    c.avanzar_con(reloj.now(), teclas(&[40]));
    assert!(c.posicion() > Micros::ZERO, "la suya sí");
}

#[test]
fn practicando_una_mano_las_puertas_de_la_otra_no_existen() {
    // No es que se abran solas: es que no están. Si estuvieran, el cursor pararía en ellas
    // esperando algo que el alumno no tiene que tocar.
    let song = cancion(&[(0, 40, 500), (1_000, 72, 500), (2_000, 43, 500)]);
    let manos = vec![Mano::Izquierda, Mano::Derecha, Mano::Izquierda];
    let mut c = Cursor::nuevo_con_puertas(&song, &manos, Some(Mano::Izquierda));
    let mut reloj = VirtualClock::new();
    c.cambiar_avance(Avance::PorAcierto, reloj.now());
    c.poner_en_marcha(reloj.now());

    c.avanzar_con(reloj.now(), teclas(&[40]));
    reloj.set(Micros(1_500_000));
    c.avanzar_con(reloj.now(), MascaraTeclas::VACIA);
    // Pasa de largo por la nota de la derecha, que está en 1.000.000.
    assert_eq!(c.posicion(), Micros(1_500_000), "no para en una puerta que no es suya");
}

// ---------------------------------------------------------------- T079

#[test]
fn cambiar_de_modo_a_mitad_conserva_la_posicion() {
    // FR-021.
    // Igual que arriba: la pieza tiene que llegar más allá de los 5 s que se comprueban.
    let song = cancion(&[(0, 60, 500), (3_000, 64, 4_000)]);
    let (mut c, mut reloj) = en_espera(&song);
    c.avanzar_con(reloj.now(), teclas(&[60]));
    reloj.set(Micros(1_200_000));
    c.avanzar_con(reloj.now(), MascaraTeclas::VACIA);
    let antes = c.posicion();
    assert_eq!(antes, Micros(1_200_000));

    c.cambiar_avance(Avance::PorReloj, reloj.now());
    assert_eq!(c.posicion(), antes, "el cambio de modo no mueve el cursor");

    // Y ahora el reloj gobierna sin detenerse en las puertas.
    reloj.set(Micros(5_000_000));
    c.avanzar_con(reloj.now(), MascaraTeclas::VACIA);
    assert_eq!(c.posicion(), Micros(5_000_000), "por reloj ya no espera a nadie");
}

#[test]
fn volver_a_modo_espera_a_mitad_tampoco_mueve_el_cursor() {
    let song = cancion(&[(0, 60, 500), (3_000, 64, 500)]);
    let manos = vec![Mano::Derecha; song.notes().len()];
    let mut c = Cursor::nuevo_con_puertas(&song, &manos, None);
    let mut reloj = VirtualClock::new();
    c.poner_en_marcha(reloj.now());
    reloj.set(Micros(2_000_000));
    c.avanzar_con(reloj.now(), MascaraTeclas::VACIA);
    let antes = c.posicion();

    c.cambiar_avance(Avance::PorAcierto, reloj.now());
    assert_eq!(c.posicion(), antes, "conserva la posición");
    // Y la puerta pendiente es la siguiente que queda por delante, no una ya pasada.
    reloj.set(Micros(9_000_000));
    c.avanzar_con(reloj.now(), MascaraTeclas::VACIA);
    assert_eq!(c.posicion(), Micros(3_000_000), "espera en la nota de después");
}

// ---------------------------------------------------------------- T081

#[test]
fn hay_salida_cuando_el_modo_espera_no_puede_satisfacerse() {
    // FR-020: si la canción pide una nota que el teclado del alumno no tiene, el modo
    // espera no puede quedarse atascado para siempre.
    let song = cancion(&[(0, 21, 1_000), (2_000, 60, 500)]);
    let (mut c, mut reloj) = en_espera(&song);
    reloj.set(Micros(5_000_000));
    c.avanzar_con(reloj.now(), MascaraTeclas::VACIA);
    assert_eq!(c.posicion(), Micros::ZERO, "atascado en una nota que no puede tocar");

    c.saltar_puerta(reloj.now());
    reloj.set(Micros(6_000_000));
    c.avanzar_con(reloj.now(), MascaraTeclas::VACIA);
    assert_eq!(c.posicion(), Micros(1_000_000), "sigue a tempo desde donde estaba");
}

#[test]
fn la_salida_no_salta_mas_de_una_puerta() {
    let song = cancion(&[(0, 21, 500), (1_000, 22, 500), (2_000, 60, 500)]);
    let (mut c, mut reloj) = en_espera(&song);
    c.saltar_puerta(reloj.now());
    reloj.set(Micros(5_000_000));
    c.avanzar_con(reloj.now(), MascaraTeclas::VACIA);
    assert_eq!(c.posicion(), Micros(1_000_000), "se detiene en la puerta siguiente");
}

#[test]
fn sin_puertas_pendientes_el_modo_espera_se_comporta_como_el_reloj() {
    // Una canción sin notas de la mano practicada no puede dejar el cursor clavado.
    let song = cancion(&[(0, 72, 500)]);
    let manos = vec![Mano::Derecha];
    let mut c = Cursor::nuevo_con_puertas(&song, &manos, Some(Mano::Izquierda));
    let mut reloj = VirtualClock::new();
    c.cambiar_avance(Avance::PorAcierto, reloj.now());
    c.poner_en_marcha(reloj.now());
    reloj.set(Micros(400_000));
    c.avanzar_con(reloj.now(), MascaraTeclas::VACIA);
    assert_eq!(c.posicion(), Micros(400_000), "avanza como si no hubiera modo espera");
}

#[test]
fn la_velocidad_sigue_mandando_entre_puertas() {
    let song = cancion(&[(0, 60, 500), (4_000, 64, 500)]);
    let (mut c, mut reloj) = en_espera(&song);
    c.cambiar_velocidad(Velocidad::nueva(1, 2).expect("válida"), reloj.now());
    c.avanzar_con(reloj.now(), teclas(&[60]));
    reloj.set(Micros(2_000_000));
    c.avanzar_con(reloj.now(), teclas(&[60]));
    assert_eq!(c.posicion(), Micros(1_000_000), "a mitad de velocidad, mitad de canción");
}

#[test]
fn la_percusion_no_genera_puertas() {
    // El comentario de `ProgramaDePuertas::nuevo` afirma que la percusión no genera puertas.
    // No lo hacía: solo filtraba por mano practicada. Un archivo con batería en el canal 9
    // producía una puerta por cada golpe, y en modo espera la práctica se quedaba atascada
    // para siempre esperando una caja que no se toca con las manos.
    //
    // `is_on_88_keys()` no salva del problema: una caja en la tecla 38 está dentro de las 88.
    let raw = SmfBuilder::new(1000)
        .track(|t| t.tempo(0, 1_000_000).note(0, 60, 90, 500).note(2_000, 64, 90, 500))
        .track(|t| {
            // Canal 9: percusión.
            let mut t = t;
            for i in 0..8u64 {
                t = t.raw(i * 250, &[0x99, 38, 100]).raw(i * 250 + 100, &[0x89, 38, 0]);
            }
            t
        })
        .build();
    let song = load_smf(&raw).expect("valida");
    let manos = vec![Mano::Derecha; song.notes().len()];
    let mut c = Cursor::nuevo_con_puertas(&song, &manos, None);
    let mut reloj = VirtualClock::new();
    c.cambiar_avance(Avance::PorAcierto, reloj.now());
    c.poner_en_marcha(reloj.now());

    // Se acierta la primera nota de piano; el cursor debe llegar hasta la segunda (2 s) sin
    // detenerse en ninguna puerta de percusión intermedia.
    c.avanzar_con(reloj.now(), teclas(&[60]));
    reloj.set(Micros(1_500_000));
    c.avanzar_con(reloj.now(), MascaraTeclas::VACIA);
    assert_eq!(
        c.posicion(),
        Micros(1_500_000),
        "la batería no debe detener el cursor"
    );
}

#[test]
fn las_puertas_y_el_evaluador_coinciden_en_que_es_evaluable() {
    // T021. El criterio vive en un solo sitio; esta prueba lo comprueba **nota por nota**
    // sobre una pieza con percusión, notas fuera de las 88 y dos manos. Si algún día alguien
    // vuelve a poner un filtro propio en las puertas, aquí se ve.
    use piano_core::evaluacion::es_evaluable;

    let raw = SmfBuilder::new(1000)
        .track(|t| {
            t.tempo(0, 1_000_000)
                .note(0, 60, 90, 500)    // normal
                .note(500, 20, 90, 500)  // por debajo del piano
                .note(1_000, 109, 90, 500) // por encima
                .note(1_500, 40, 90, 500)
        })
        .track(|t| t.raw(0, &[0x99, 38, 100]).raw(100, &[0x89, 38, 0]))
        .build();
    let song = load_smf(&raw).expect("valida");
    let manos = vec![Mano::Derecha; song.notes().len()];

    for practicada in [None, Some(Mano::Derecha), Some(Mano::Izquierda)] {
        let p = piano_core::practica::ProgramaDePuertas::nuevo(&song, &manos, practicada);
        let esperadas: Vec<u8> = song
            .notes()
            .iter()
            .zip(&manos)
            .filter(|(n, m)| es_evaluable(n.channel, n.key, **m, practicada))
            .map(|(n, _)| n.key)
            .collect();
        let en_puertas: usize = (0..p.len())
            .filter_map(|i| p.get(i))
            .map(|g| g.teclas.cuenta() as usize)
            .sum();
        assert_eq!(
            en_puertas,
            esperadas.len(),
            "con mano {practicada:?}: las puertas y el evaluador no coinciden"
        );
    }
}
