//! T027-T035 — emparejar ataques con sueltas, y cerrar lo que quede abierto.

use piano_core::capture::{Cierre, Emparejador, EventoCrudo, TipoEvento};
use piano_core::time::Micros;

fn ev(seq: u32, us: u64, key: u8, kind: TipoEvento) -> EventoCrudo {
    EventoCrudo {
        at: Micros(us),
        seq,
        key,
        velocity: if kind == TipoEvento::Ataque { 100 } else { 0 },
        kind,
        channel: 0,
    }
}
fn ataque(seq: u32, us: u64, key: u8) -> EventoCrudo { ev(seq, us, key, TipoEvento::Ataque) }
fn suelta(seq: u32, us: u64, key: u8) -> EventoCrudo { ev(seq, us, key, TipoEvento::Suelta) }

#[test]
fn un_ataque_y_su_suelta_producen_una_pulsacion() {
    let mut e = Emparejador::nuevo();
    assert_eq!(e.consumir(ataque(0, 1_000, 60)), None, "un ataque solo no cierra nada");
    let p = e.consumir(suelta(1, 3_000, 60)).expect("la suelta cierra la nota");
    assert_eq!((p.onset, p.end, p.key, p.velocity), (Micros(1_000), Micros(3_000), 60, 100));
    assert_eq!(p.closure, Cierre::PorSuelta);
    assert!(!p.duracion_censurada);
    assert!(e.informe().esta_limpio());
}

#[test]
fn la_misma_altura_en_canales_distintos_son_notas_independientes() {
    let mut e = Emparejador::nuevo();
    let mut a = ataque(0, 0, 60); a.channel = 0;
    let mut b = ataque(1, 0, 60); b.channel = 1;
    assert_eq!(e.consumir(a), None);
    assert_eq!(e.consumir(b), None, "otro canal, otra voz: no cierra la anterior");
    let mut off_b = suelta(2, 500, 60); off_b.channel = 1;
    let p = e.consumir(off_b).expect("cierra la del canal 1");
    assert_eq!(p.channel, 1);
    assert_eq!(e.informe().repulsaciones, 0);
}

#[test]
fn repulsar_una_tecla_ya_hundida_cierra_la_anterior() {
    // El caso canonico. **Aqui divergimos a proposito de la feature 001**, y el motivo
    // esta en el codigo: la carga de un archivo ve la pieza entera y puede emparejar en
    // FIFO mirando hacia adelante; la captura en vivo no puede ver el futuro sin
    // retrasar la entrega, y retrasar la entrega es justo lo que prohibe el presupuesto
    // de 30 ms. Cuando una tecla ya hundida recibe otro ataque, la anterior termina ahi.
    let mut e = Emparejador::nuevo();
    assert_eq!(e.consumir(ataque(0, 0, 60)), None);
    let primera = e.consumir(ataque(1, 10, 60)).expect("el segundo ataque cierra el primero");
    assert_eq!((primera.onset, primera.end), (Micros(0), Micros(10)));
    assert_eq!(primera.closure, Cierre::PorRepulsacion);

    let segunda = e.consumir(suelta(2, 20, 60)).expect("la suelta cierra la segunda");
    assert_eq!((segunda.onset, segunda.end), (Micros(10), Micros(20)));
    assert_eq!(segunda.closure, Cierre::PorSuelta);

    assert_eq!(e.consumir(suelta(3, 30, 60)), None, "la suelta sobrante es huerfana");
    assert_eq!(e.informe().repulsaciones, 1);
    assert_eq!(e.informe().sueltas_sin_pulsacion, 1);
}

#[test]
fn una_suelta_sin_ataque_previo_se_ignora_y_se_cuenta() {
    let mut e = Emparejador::nuevo();
    assert_eq!(e.consumir(suelta(0, 100, 60)), None);
    assert_eq!(e.informe().sueltas_sin_pulsacion, 1);
}

#[test]
fn cerrar_por_parada_cierra_las_teclas_hundidas_y_las_etiqueta() {
    // FR-015: ni se descartan ni reciben una duracion inventada.
    let mut e = Emparejador::nuevo();
    e.consumir(ataque(0, 0, 60));
    e.consumir(ataque(1, 100, 64));
    e.consumir(ataque(2, 200, 67));
    e.consumir(suelta(3, 300, 64)); // esta si cierra sola
    let cerradas = e.cerrar(Micros(5_000), Cierre::PorParada);
    assert_eq!(cerradas.len(), 2, "las dos que seguian hundidas");
    assert!(cerradas.iter().all(|p| p.closure == Cierre::PorParada));
    assert!(cerradas.iter().all(|p| p.end == Micros(5_000)));
    assert!(cerradas.iter().all(|p| !p.duracion_censurada), "la parada si sabe cuando fue");
    assert_eq!(cerradas.iter().map(|p| p.key).collect::<Vec<_>>(), vec![60, 67], "orden estable");
    assert_eq!(e.informe().cerradas_por_parada, 2);
}

#[test]
fn cerrar_por_perdida_marca_la_duracion_como_censurada() {
    // Sabemos cuando empezo, no cuando termino: el sello es el ultimo evento recibido.
    let mut e = Emparejador::nuevo();
    e.consumir(ataque(0, 1_000, 60));
    let cerradas = e.cerrar(Micros(1_500), Cierre::PorPerdidaDeDispositivo);
    assert_eq!(cerradas.len(), 1);
    assert_eq!(cerradas[0].closure, Cierre::PorPerdidaDeDispositivo);
    assert!(cerradas[0].duracion_censurada, "no sabemos cuanto duro de verdad");
    assert_eq!(e.informe().cerradas_por_perdida, 1);
}

#[test]
fn cerrar_dos_veces_no_duplica_nada() {
    let mut e = Emparejador::nuevo();
    e.consumir(ataque(0, 0, 60));
    assert_eq!(e.cerrar(Micros(100), Cierre::PorParada).len(), 1);
    assert_eq!(e.cerrar(Micros(200), Cierre::PorParada).len(), 0, "ya no queda nada abierto");
}

#[test]
fn una_nota_nunca_dura_cero() {
    let mut e = Emparejador::nuevo();
    e.consumir(ataque(0, 1_000, 60));
    let p = e.consumir(suelta(1, 1_000, 60)).expect("cierra");
    assert!(p.end > p.onset, "un ataque sin duracion seria un ataque sin sonido");
}

#[test]
fn cuenta_la_percusion_y_lo_que_cae_fuera_de_las_88_teclas() {
    let mut e = Emparejador::nuevo();
    let mut perc = ataque(0, 0, 38); perc.channel = 9;
    e.consumir(perc);
    e.consumir(ataque(1, 0, 12));  // por debajo del piano
    e.consumir(ataque(2, 0, 60));  // normal
    e.cerrar(Micros(100), Cierre::PorParada);
    assert_eq!(e.informe().percusion, 1);
    assert_eq!(e.informe().fuera_de_88_teclas, 1, "solo la 12: la 38 si esta dentro de 21..=108");
    assert!(!e.informe().esta_limpio());
}

#[test]
fn el_emparejador_es_determinista() {
    let ejecutar = || {
        let mut e = Emparejador::nuevo();
        let mut salida = Vec::new();
        for (i, (us, key, kind)) in [
            (0u64, 67u8, TipoEvento::Ataque), (0, 60, TipoEvento::Ataque), (0, 64, TipoEvento::Ataque),
            (500, 60, TipoEvento::Suelta), (500, 64, TipoEvento::Suelta), (900, 67, TipoEvento::Ataque),
        ].into_iter().enumerate() {
            if let Some(p) = e.consumir(ev(i as u32, us, key, kind)) { salida.push(p); }
        }
        salida.extend(e.cerrar(Micros(2_000), Cierre::PorParada));
        (salida, *e.informe())
    };
    let referencia = ejecutar();
    for _ in 0..50 { assert_eq!(ejecutar(), referencia); }
}
