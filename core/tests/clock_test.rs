//! T008: la fuente de tiempo sustituible.

use piano_core::clock::{Clock, MonotonicClock, VirtualClock};
use piano_core::time::Micros;

#[test]
fn el_reloj_virtual_empieza_en_cero() {
    assert_eq!(VirtualClock::new().now(), Micros(0));
}

#[test]
fn el_reloj_virtual_solo_avanza_cuando_se_le_ordena() {
    let mut c = VirtualClock::new();
    assert_eq!(c.now(), Micros(0));
    assert_eq!(c.now(), Micros(0), "leerlo no lo mueve");
    c.advance(Micros(500));
    assert_eq!(c.now(), Micros(500));
    c.advance(Micros(500));
    assert_eq!(c.now(), Micros(1_000));
}

#[test]
fn el_reloj_virtual_se_puede_colocar_en_un_instante_concreto() {
    let mut c = VirtualClock::new();
    c.set(Micros(42_000));
    assert_eq!(c.now(), Micros(42_000));
}

#[test]
fn el_reloj_virtual_es_reproducible() {
    let seq = [10u64, 250, 3, 10_000, 1];
    let run = || {
        let mut c = VirtualClock::new();
        seq.iter().map(|d| { c.advance(Micros(*d)); c.now() }).collect::<Vec<_>>()
    };
    assert_eq!(run(), run(), "la misma secuencia produce los mismos instantes");
}

#[test]
fn el_reloj_monotono_nunca_retrocede() {
    let c = MonotonicClock::start();
    let mut prev = c.now();
    for _ in 0..1_000 {
        let now = c.now();
        assert!(now >= prev, "{now:?} < {prev:?}: el reloj retrocedio");
        prev = now;
    }
}

#[test]
fn el_reloj_monotono_empieza_cerca_de_cero() {
    // Mide desde el inicio de la sesion, no desde el epoch.
    assert!(MonotonicClock::start().now() < Micros(1_000_000));
}
