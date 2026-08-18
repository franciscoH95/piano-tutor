//! T012, T014, T016: el mapa de tempo.
//!
//! Es la pieza de la que depende toda la precision musical: si aqui hay deriva de
//! redondeo, las notas de una pieza larga se desplazan y el alumno practica contra una
//! referencia falsa.

use piano_core::error::LoadError;
use piano_core::tempo::TempoMap;
use piano_core::time::{Micros, Ticks};

/// El ejemplo numerico trabajado de research.md, Decision 2.
/// PPQ 480, 120 negras/min desde el tick 0, 240 negras/min desde el tick 960.
fn mapa_de_ejemplo() -> TempoMap {
    TempoMap::new(480, &[(0, 500_000), (960, 250_000)]).expect("mapa valido")
}

#[test]
fn convierte_ticks_a_microsegundos_dentro_del_primer_tramo() {
    let m = mapa_de_ejemplo();
    assert_eq!(m.tick_to_us(Ticks(0)), Micros(0));
    // Una negra a 120 negras/min = 500 ms.
    assert_eq!(m.tick_to_us(Ticks(480)), Micros(500_000));
    // Dos negras = 1 s, justo el borde del cambio de tempo.
    assert_eq!(m.tick_to_us(Ticks(960)), Micros(1_000_000));
}

#[test]
fn el_cambio_de_tempo_solo_afecta_al_tramo_posterior() {
    let m = mapa_de_ejemplo();
    // 1 s del primer tramo + una negra a 240 negras/min (250 ms) = 1,25 s.
    assert_eq!(m.tick_to_us(Ticks(1_440)), Micros(1_250_000));
    // Dos negras mas del segundo tramo.
    assert_eq!(m.tick_to_us(Ticks(1_920)), Micros(1_500_000));
}

#[test]
fn sin_eventos_de_tempo_asume_120_negras_por_minuto() {
    // FR-005: 500.000 microsegundos por negra es el defecto del estandar.
    let m = TempoMap::new(480, &[]).expect("un mapa sin tempos es valido");
    assert_eq!(m.segments().len(), 1);
    assert_eq!(m.tick_to_us(Ticks(480)), Micros(500_000));
}

#[test]
fn un_primer_tempo_en_un_tick_posterior_a_cero_antepone_el_tramo_por_defecto() {
    let m = TempoMap::new(480, &[(960, 250_000)]).expect("valido");
    assert_eq!(m.segments().len(), 2, "se antepone el tramo por defecto");
    assert_eq!(m.tick_to_us(Ticks(480)), Micros(500_000), "antes del cambio, 120 negras/min");
    assert_eq!(m.tick_to_us(Ticks(1_440)), Micros(1_250_000), "despues, 240");
}

#[test]
fn varios_tempos_en_el_mismo_tick_ganan_el_ultimo() {
    let m = TempoMap::new(480, &[(0, 500_000), (0, 250_000)]).expect("valido");
    assert_eq!(m.segments().len(), 1, "no se dejan tramos de longitud cero");
    assert_eq!(m.tick_to_us(Ticks(480)), Micros(250_000), "gana el ultimo declarado");
}

#[test]
fn un_tempo_de_cero_se_rechaza_en_vez_de_corregirse_en_silencio() {
    // Corregirlo en silencio entregaria una leccion falsa; ademas es division por cero.
    assert!(matches!(
        TempoMap::new(480, &[(0, 0)]),
        Err(LoadError::InvalidTempo { tick: 0, us_per_qn: 0 })
    ));
}

#[test]
fn una_division_de_cero_se_rechaza() {
    assert!(matches!(TempoMap::new(0, &[]), Err(LoadError::DivisionCero)));
}

#[test]
fn el_redondeo_trunca_hacia_abajo_y_eso_es_contrato_publico() {
    // PPQ 3: un tick es 500000/3 = 166666,67 microsegundos.
    let m = TempoMap::new(3, &[(0, 500_000)]).expect("valido");
    assert_eq!(m.tick_to_us(Ticks(1)), Micros(166_666), "floor, no round");
    assert_eq!(m.tick_to_us(Ticks(2)), Micros(333_333));
    assert_eq!(m.tick_to_us(Ticks(3)), Micros(500_000), "el tercer tick cuadra exacto");
}

#[test]
fn no_hay_deriva_acumulada_en_una_pieza_larga() {
    // La clave de la Decision 2: acumular en microsegundos x PPQ y dividir una sola vez.
    // Con PPQ 3 y un tempo que no divide exacto, sumar tick a tick derivaria.
    let m = TempoMap::new(3, &[(0, 500_000)]).expect("valido");
    // 3.000.000 de ticks = 1.000.000 de negras = 500.000.000.000 microsegundos exactos.
    assert_eq!(m.tick_to_us(Ticks(3_000_000)), Micros(500_000_000_000));
}

#[test]
fn la_inversa_devuelve_el_mayor_tick_que_cae_en_ese_microsegundo() {
    // La primera version de esta prueba afirmaba `vuelta <= t`, y era incorrecta:
    // con truncado, varios ticks consecutivos caen en el mismo microsegundo, y el
    // contrato de `us_to_tick` es devolver el MAYOR de ellos. La ida y vuelta puede
    // por tanto avanzar dentro de ese grupo, nunca salir de el.
    let m = mapa_de_ejemplo();
    for t in [0u64, 1, 479, 480, 959, 960, 961, 1_440, 100_000] {
        let ida = m.tick_to_us(Ticks(t));
        let vuelta = m.us_to_tick(ida);
        assert!(vuelta.0 >= t, "us_to_tick({ida:?}) = {vuelta:?} retrocedio por debajo de {t}");
        assert_eq!(m.tick_to_us(vuelta), ida, "el tick devuelto cae en el mismo microsegundo");
        assert!(
            m.tick_to_us(Ticks(vuelta.0 + 1)) > ida,
            "el tick siguiente ya se pasa: {vuelta:?} es realmente el mayor"
        );
    }
}

#[test]
fn la_inversa_no_se_pasa_del_instante_pedido() {
    let m = mapa_de_ejemplo();
    for us in [0u64, 1, 1_040, 1_041, 499_999, 500_000, 1_000_000, 1_249_999, 9_999_999] {
        let tick = m.us_to_tick(Micros(us));
        assert!(
            m.tick_to_us(tick).0 <= us,
            "us_to_tick({us}) = {tick:?} cae en {:?}, despues del instante pedido",
            m.tick_to_us(tick)
        );
    }
}

#[test]
fn los_tramos_tienen_tiempo_real_estrictamente_creciente() {
    // Es la invariante que legitima la busqueda binaria en la direccion inversa.
    let m = TempoMap::new(96, &[(0, 500_000), (96, 100_000), (192, 900_000), (500, 1)])
        .expect("valido");
    let segs = m.segments();
    for w in segs.windows(2) {
        assert!(w[0].start_tick < w[1].start_tick, "ticks estrictamente crecientes");
        assert!(w[0].start_us < w[1].start_us, "tiempo real estrictamente creciente");
    }
}

#[test]
fn el_mapa_es_determinista() {
    let a = mapa_de_ejemplo();
    let b = mapa_de_ejemplo();
    assert_eq!(a.segments(), b.segments());
}
