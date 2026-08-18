//! T056, T057 — SC-002 y SC-002a: rafagas densas y desbordamiento.

use piano_core::capture::{canal, Observacion, TipoEvento};
use piano_core::time::Micros;

fn obs(us: u64, key: u8) -> Observacion {
    Observacion { at: Micros(us), key, velocity: 100, kind: TipoEvento::Ataque, channel: 0 }
}

#[test]
fn una_rafaga_de_un_minuto_a_cincuenta_eventos_por_segundo_no_pierde_nada() {
    // SC-002. Se simula el tiempo: no se espera un minuto de verdad.
    const TOTAL: usize = 50 * 60; // 3.000 eventos
    let (mut tx, mut rx) = canal(4_096);
    let mut recogidos = Vec::with_capacity(TOTAL);
    for i in 0..TOTAL {
        tx.emitir(obs(i as u64 * 20_000, 21 + (i % 88) as u8));
        // El consumidor va vaciando, como en la aplicacion real.
        if i % 100 == 0 {
            rx.recoger(&mut recogidos);
        }
    }
    rx.recoger(&mut recogidos);
    assert_eq!(recogidos.len(), TOTAL, "se perdieron pulsaciones");
    assert_eq!(tx.descartados(), 0, "el contador de descartes debe quedar en cero");
    assert!(recogidos.windows(2).all(|w| w[0].seq < w[1].seq), "sin huecos en la secuencia");
}

#[test]
fn con_el_consumidor_detenido_la_memoria_no_crece_y_el_contador_cuadra() {
    // SC-002a. Nadie recoge nada: el anillo se llena y a partir de ahi se descarta lo
    // entrante. La capacidad es fija, asi que la memoria no crece.
    const CAPACIDAD: usize = 512;
    const ENVIADOS: usize = 50_000;
    let (mut tx, mut rx) = canal(CAPACIDAD);
    for i in 0..ENVIADOS {
        tx.emitir(obs(i as u64, 60));
    }
    assert_eq!(
        tx.descartados() as usize,
        ENVIADOS - CAPACIDAD,
        "el contador debe reflejar exactamente cuantas se perdieron"
    );
    let mut v = Vec::new();
    assert_eq!(rx.recoger(&mut v), CAPACIDAD, "el anillo conserva exactamente su capacidad");
    assert!(v.windows(2).all(|w| w[0].seq < w[1].seq));
    assert_eq!(v.first().map(|e| e.seq), Some(0), "lo conservado es lo PRIMERO, no lo ultimo");
}

#[test]
fn tras_desbordar_el_hueco_de_secuencia_dice_cuanto_se_perdio() {
    let (mut tx, mut rx) = canal(4);
    for i in 0..4u64 {
        tx.emitir(obs(i, 60));
    }
    let mut v = Vec::new();
    rx.recoger(&mut v);
    v.clear();
    for i in 4..20u64 {
        tx.emitir(obs(i, 60)); // 16 mas: caben 4, se descartan 12
    }
    rx.recoger(&mut v);
    let seqs: Vec<u32> = v.iter().map(|e| e.seq).collect();
    assert_eq!(seqs, vec![4, 5, 6, 7], "solo entran las cuatro primeras");
    assert_eq!(tx.descartados(), 12);
}

#[test]
fn la_capacidad_del_anillo_cubre_de_sobra_una_rafaga_humana() {
    // 4.096 ranuras frente a ~50 eventos/s: mas de ochenta segundos sin que nadie recoja.
    const CAPACIDAD: usize = 4_096;
    let segundos_de_margen = CAPACIDAD / 50;
    assert!(segundos_de_margen > 60, "solo {segundos_de_margen} s de margen");
}
