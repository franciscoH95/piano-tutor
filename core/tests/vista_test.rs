//! T005, T007 — qué notas hay que dibujar en un instante dado.

mod fixtures;
use fixtures::SmfBuilder;
use piano_core::load_smf;
use piano_core::practica::{vista, EstadoNota, Vista};
use piano_core::time::Micros;

/// Notas consecutivas de una negra a 120 pulsaciones por minuto: una cada 500 ms.
fn cancion(n: u64) -> piano_core::Song {
    let raw = SmfBuilder::new(480)
        .track(|t| t.tempo(0, 500_000))
        .track(|t| {
            let mut t = t;
            for i in 0..n {
                t = t.note(i * 480, 21 + (i % 88) as u8, 90, 400);
            }
            t
        })
        .build();
    load_smf(&raw).expect("valida")
}

#[test]
fn solo_devuelve_las_notas_que_se_solapan_con_la_ventana() {
    let song = cancion(10);
    let mut v = Vista::nueva();
    let mut out = Vec::new();
    // Ventana [1,0 s, 2,0 s): debe traer las notas que empiezan en 1,0 y 1,5.
    vista(&song, &mut v, Micros(1_000_000), Micros(1_000_000), Micros(2_000_000), &mut out);
    let inicios: Vec<u64> = out.iter().map(|n| n.onset_us.0).collect();
    assert_eq!(inicios, vec![1_000_000, 1_500_000], "solo las que caen dentro");
}

#[test]
fn incluye_una_nota_que_empezo_antes_y_sigue_sonando() {
    // Una nota larga que empieza fuera de la ventana pero la atraviesa: hay que dibujarla.
    let raw = SmfBuilder::new(480)
        .track(|t| t.tempo(0, 500_000))
        .track(|t| t.note(0, 60, 90, 4_800)) // 10 negras = 5 s
        .build();
    let song = load_smf(&raw).expect("valida");
    let mut v = Vista::nueva();
    let mut out = Vec::new();
    vista(&song, &mut v, Micros(2_000_000), Micros(2_000_000), Micros(3_000_000), &mut out);
    assert_eq!(out.len(), 1, "la nota larga atraviesa la ventana y debe dibujarse");
    assert_eq!(out[0].onset_us, Micros(0));
}

#[test]
fn una_ventana_vacia_no_devuelve_nada() {
    let song = cancion(3);
    let mut v = Vista::nueva();
    let mut out = Vec::new();
    // Muy despues del final de la pieza.
    vista(&song, &mut v, Micros(60_000_000), Micros(60_000_000), Micros(61_000_000), &mut out);
    assert!(out.is_empty());
}

#[test]
fn marca_como_sonando_la_nota_que_suena_en_la_posicion_actual() {
    let song = cancion(4);
    let mut v = Vista::nueva();
    let mut out = Vec::new();
    // La segunda nota empieza en 0,5 s y dura 400 ticks (~416 ms).
    vista(&song, &mut v, Micros(600_000), Micros(0), Micros(3_000_000), &mut out);
    let sonando: Vec<u64> = out
        .iter()
        .filter(|n| n.estado == EstadoNota::Sonando)
        .map(|n| n.onset_us.0)
        .collect();
    assert_eq!(sonando, vec![500_000], "solo la que suena en ese instante");
    assert!(out.iter().filter(|n| n.onset_us.0 != 500_000).all(|n| n.estado == EstadoNota::Pendiente));
}

#[test]
fn el_indice_permite_recuperar_la_nota_original() {
    // La vista no copia las anotaciones por nota (mano, dedo, nombre): las deja
    // localizables por indice, porque son constantes de la cancion y no datos de fotograma.
    let song = cancion(5);
    let mut v = Vista::nueva();
    let mut out = Vec::new();
    vista(&song, &mut v, Micros(0), Micros(0), Micros(3_000_000), &mut out);
    for n in &out {
        let original = &song.notes()[n.indice as usize];
        assert_eq!(original.key, n.key);
        assert_eq!(original.onset_us, n.onset_us);
    }
}

#[test]
fn el_coste_no_crece_con_el_tamano_de_la_cancion() {
    // T007. Se cuentan notas EXAMINADAS, no milisegundos: un cronometro haria la prueba
    // intermitente y no demostraria la propiedad estructural.
    let corta = cancion(50);
    let larga = cancion(20_000);
    let ventana = (Micros(10_000_000), Micros(10_000_000), Micros(11_000_000));

    let mut va = Vista::nueva();
    let mut vb = Vista::nueva();
    let mut out = Vec::new();
    vista(&corta, &mut va, ventana.0, ventana.1, ventana.2, &mut out);
    let n_corta = out.len();
    let examinadas_corta = va.examinadas();
    out.clear();
    vista(&larga, &mut vb, ventana.0, ventana.1, ventana.2, &mut out);
    assert_eq!(out.len(), n_corta, "la misma ventana muestra las mismas notas");
    assert_eq!(
        vb.examinadas(),
        examinadas_corta,
        "una cancion 400 veces mas larga examino un numero distinto de notas"
    );
}

#[test]
fn avanzar_por_la_cancion_no_reexamina_lo_ya_pasado() {
    // El cursor es monotono: recorrer la pieza entera examina cada nota un numero
    // acotado de veces, no una vez por fotograma.
    let song = cancion(2_000);
    let mut v = Vista::nueva();
    let mut out = Vec::new();
    let mut total = 0usize;
    for paso in 0..1_000u64 {
        let pos = Micros(paso * 1_000_000);
        out.clear();
        vista(&song, &mut v, pos, pos, Micros(pos.0 + 1_000_000), &mut out);
        total += out.len();
    }
    assert!(total > 0);
    assert!(
        v.examinadas() < 20_000,
        "se examinaron {} notas recorriendo 2.000: se esta reescaneando",
        v.examinadas()
    );
}

// ------------------------------------------------- regresión: saltar sobre una nota larga

/// ppq 1000 y 1.000.000 µs por negra hacen que **un tick sea un milisegundo**, así que las
/// cifras del archivo se leen directamente en microsegundos ×1000.
fn con_pedal() -> piano_core::Song {
    let raw = SmfBuilder::new(1000)
        .track(|t| {
            let mut t = t.tempo(0, 1_000_000);
            t = t.note(0, 60, 90, 1_000); // 0 → 1.000.000 µs
            // Un pedal largo que empieza mucho antes del salto y sigue sonando después.
            t = t.note(290_000, 36, 90, 30_000); // 290.000.000 → 320.000.000 µs
            for k in 0..9u64 {
                t = t.note(291_000 + k * 1_000, 72, 90, 500);
            }
            t.note(300_000, 74, 90, 500) // 300.000.000 → 300.500.000 µs
        })
        .build();
    load_smf(&raw).expect("valida")
}

#[test]
fn saltar_al_medio_de_una_nota_larga_la_deja_visible() {
    // El fallo: `Song::notes` está ordenado por ATAQUE, pero `reposicionar` buscaba con
    // `partition_point` sobre `end_us < objetivo`, que en esa ordenación **no es un
    // predicado monótono**. `partition_point` exige que lo sea; si no, devuelve un índice
    // cualquiera y el cursor se coloca por delante de una nota que todavía suena.
    //
    // El pedal (290 s → 320 s) sigue sonando en el segundo 300. Tras saltar allí, tiene
    // que verse. Con el fallo desaparece: el alumno salta a un compás y el bajo que
    // sostiene la armonía no está.
    let song = con_pedal();
    let pedal = song
        .notes()
        .iter()
        .position(|n| n.key == 36)
        .expect("el pedal está en la canción");

    let mut v = Vista::nueva();
    v.reposicionar(&song, Micros(300_000_000));

    let mut out = Vec::new();
    vista(
        &song,
        &mut v,
        Micros(300_000_000),
        Micros(300_000_000),
        Micros(301_000_000),
        &mut out,
    );

    assert!(
        out.iter().any(|n| n.indice as usize == pedal),
        "el pedal sigue sonando en el segundo 300 y debe verse; visibles: {:?}",
        out.iter().map(|n| n.key).collect::<Vec<_>>()
    );
}

#[test]
fn reposicionar_nunca_deja_atras_una_nota_que_aun_suena() {
    // La versión general, barriendo la pieza entera: en cualquier instante, toda nota que
    // esté sonando debe aparecer en la vista después de reposicionar ahí.
    let song = con_pedal();
    for us in (0..320_000_000u64).step_by(10_000_000) {
        let sonando: Vec<usize> = song
            .notes()
            .iter()
            .enumerate()
            .filter(|(_, n)| n.onset_us.0 <= us && us < n.end_us.0)
            .map(|(i, _)| i)
            .collect();

        let mut v = Vista::nueva();
        v.reposicionar(&song, Micros(us));
        let mut out = Vec::new();
        vista(&song, &mut v, Micros(us), Micros(us), Micros(us + 1), &mut out);

        for i in sonando {
            assert!(
                out.iter().any(|n| n.indice as usize == i),
                "la nota {i} sonaba en {us} y la vista no la trajo"
            );
        }
    }
}

#[test]
fn recolocar_en_una_cancion_de_diez_minutos_es_holgadamente_rapido() {
    // La corrección de `reposicionar` la volvió O(n). Eso hay que **demostrarlo** aceptable,
    // no afirmarlo: SC-008a da 100 ms para un salto en una pieza de diez minutos.
    let raw = SmfBuilder::new(1000)
        .track(|t| {
            let mut t = t.tempo(0, 1_000_000);
            // Diez minutos a diez notas por segundo: 6.000 notas.
            for i in 0..6_000u64 {
                t = t.note(i * 100, 60 + (i % 24) as u8, 90, 90);
            }
            t
        })
        .build();
    let song = load_smf(&raw).expect("valida");
    assert_eq!(song.notes().len(), 6_000);

    let inicio = std::time::Instant::now();
    let mut v = Vista::nueva();
    for us in (0..600_000_000u64).step_by(6_000_000) {
        v.reposicionar(&song, Micros(us));
    }
    let transcurrido = inicio.elapsed();
    // Cien saltos, cada uno con su presupuesto de 100 ms: sobra tres órdenes de magnitud.
    assert!(
        transcurrido < std::time::Duration::from_millis(100),
        "cien saltos tardaron {transcurrido:?}"
    );
}
