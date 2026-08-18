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
