//! T054, T055 — SC-004: la misma secuencia controlada produce siempre lo mismo.

use piano_core::capture::{
    Cierre, Emparejador, FuenteDeEventos, FuenteGuionizada, Observacion, TipoEvento,
};
use piano_core::time::Micros;

fn o(us: u64, key: u8, canal: u8, kind: TipoEvento) -> Observacion {
    Observacion {
        at: Micros(us),
        key,
        velocity: if kind == TipoEvento::Ataque { 90 } else { 0 },
        kind,
        channel: canal,
    }
}

/// Un guion limpio: escalas y acordes bien formados.
fn guion_limpio() -> Vec<Observacion> {
    let mut v = Vec::new();
    for i in 0..60u64 {
        let key = 48 + (i % 36) as u8;
        v.push(o(i * 1_000, key, 0, TipoEvento::Ataque));
        v.push(o(i * 1_000 + 800, key, 0, TipoEvento::Suelta));
    }
    v
}

/// Un guion sucio a proposito: sueltas huerfanas, repulsaciones, acordes, varios canales,
/// instantes que retroceden y teclas que quedan hundidas al final.
fn guion_sucio() -> Vec<Observacion> {
    vec![
        o(0, 60, 0, TipoEvento::Suelta),      // huerfana
        o(100, 60, 0, TipoEvento::Ataque),
        o(150, 60, 0, TipoEvento::Ataque),    // repulsacion
        o(120, 64, 0, TipoEvento::Ataque),    // instante que retrocede
        o(200, 67, 1, TipoEvento::Ataque),    // otro canal
        o(200, 67, 0, TipoEvento::Ataque),    // misma altura, canal distinto
        o(300, 60, 0, TipoEvento::Suelta),
        o(300, 12, 0, TipoEvento::Ataque),    // fuera de las 88 teclas
        o(300, 38, 9, TipoEvento::Ataque),    // percusion
        o(400, 99, 0, TipoEvento::Suelta),    // huerfana
        o(500, 64, 0, TipoEvento::Ataque),    // repulsacion de la 64
        // y varias quedan hundidas al terminar
    ]
}

fn ejecutar(guion: Vec<Observacion>) -> (Vec<piano_core::capture::PulsacionCapturada>, piano_core::capture::InformeDeCaptura) {
    let mut f = FuenteGuionizada::nueva(guion);
    let mut e = Emparejador::nuevo();
    let mut buf = Vec::new();
    let mut notas = Vec::new();
    f.recoger(&mut buf);
    for ev in buf {
        if let Some(p) = e.consumir(ev) {
            notas.push(p);
        }
    }
    notas.extend(e.cerrar(Micros(10_000), Cierre::PorParada));
    (notas, *e.informe())
}

#[test]
fn cien_ejecuciones_del_mismo_guion_limpio_dan_lo_mismo() {
    let referencia = ejecutar(guion_limpio());
    assert!(!referencia.0.is_empty());
    for i in 0..100 {
        assert_eq!(ejecutar(guion_limpio()), referencia, "la ejecucion {i} difiere");
    }
}

#[test]
fn cien_ejecuciones_del_guion_sucio_tambien_dan_lo_mismo() {
    // T055 — el determinismo tiene que aguantar tambien los casos feos, que son donde
    // suele colarse un `HashMap` o un orden accidental.
    let referencia = ejecutar(guion_sucio());
    assert!(!referencia.0.is_empty());
    assert!(!referencia.1.esta_limpio(), "el guion sucio debe ensuciar el informe");
    for i in 0..100 {
        assert_eq!(ejecutar(guion_sucio()), referencia, "la ejecucion {i} difiere");
    }
}

#[test]
fn el_orden_de_las_notas_cerradas_al_parar_es_estable() {
    // Se cierran recorriendo la tabla de voces en orden de (canal, altura), no en el
    // orden en que se pulsaron: eso es lo que lo hace reproducible.
    let (notas, _) = ejecutar(guion_sucio());
    let cerradas: Vec<(u8, u8)> = notas
        .iter()
        .filter(|p| p.closure == Cierre::PorParada)
        .map(|p| (p.channel, p.key))
        .collect();
    let mut ordenadas = cerradas.clone();
    ordenadas.sort_unstable();
    assert_eq!(cerradas, ordenadas, "las cerradas al parar salen en orden canonico");
}

#[test]
fn ninguna_nota_queda_con_duracion_cero_ni_instantes_al_reves() {
    for guion in [guion_limpio(), guion_sucio()] {
        let (notas, _) = ejecutar(guion);
        for p in &notas {
            assert!(p.end > p.onset, "nota de duracion cero: {p:?}");
        }
    }
}
