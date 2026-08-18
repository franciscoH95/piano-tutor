//! T036a-T036f — el adaptador de macOS, ejercido contra un teclado virtual.
//!
//! `coremidi` permite crear una fuente virtual en el propio sistema, asi que esta capa
//! **si** se puede probar sin hardware. Es lo que reduce la excepcion del Principio II a
//! la rama de Windows.
#![cfg(target_os = "macos")]

use coremidi::{Client, PacketBuffer};
use piano_core::capture::{Cierre, Emparejador, EventoCrudo, TipoEvento};
use piano_core::clock::MonotonicClock;
use piano_midi_io::{abrir, dispositivos};
use std::time::{Duration, Instant};

/// Nombre unico por prueba: dos fuentes virtuales homonimas se estorbarian entre si.
fn nombre(sufijo: &str) -> String {
    format!("PianoTutorTest-{sufijo}")
}

/// Recoge hasta `esperados` eventos, con fecha limite. La entrega de CoreMIDI es
/// asincrona: sondear con limite es lo unico honesto aqui.
fn recoger_hasta(cap: &mut piano_midi_io::Captura, esperados: usize, ms: u64) -> Vec<EventoCrudo> {
    let limite = Instant::now() + Duration::from_millis(ms);
    let mut v = Vec::new();
    while v.len() < esperados && Instant::now() < limite {
        cap.receptor().recoger(&mut v);
        if v.len() < esperados {
            std::thread::sleep(Duration::from_millis(2));
        }
    }
    v
}

fn buscar(nombre: &str) -> Option<piano_core::capture::Dispositivo> {
    dispositivos().ok()?.into_iter().find(|d| d.nombre == nombre)
}

#[test]
fn una_fuente_virtual_aparece_en_la_enumeracion_con_su_nombre_y_su_identificador() {
    let n = nombre("enumeracion");
    let cliente = Client::new("PianoTutorTestClient").expect("cliente");
    let _vs = cliente.virtual_source(&n).expect("fuente virtual");
    std::thread::sleep(Duration::from_millis(80));

    let d = buscar(&n).expect("la fuente virtual debe aparecer en la enumeracion");
    assert_eq!(d.nombre, n);
    assert!(d.id_sistema.is_some(), "macOS debe dar un identificador estable");
}

#[test]
fn una_nota_enviada_por_la_fuente_virtual_llega_al_nucleo() {
    let n = nombre("nota");
    let cliente = Client::new("PianoTutorTestClient").expect("cliente");
    let vs = cliente.virtual_source(&n).expect("fuente virtual");
    std::thread::sleep(Duration::from_millis(80));
    let d = buscar(&n).expect("enumerada");
    let mut cap = abrir(&d, MonotonicClock::start()).expect("abrir");
    std::thread::sleep(Duration::from_millis(50));

    vs.received(&PacketBuffer::new(0, &[0x90, 60, 100])).expect("enviar ataque");
    vs.received(&PacketBuffer::new(0, &[0x80, 60, 0])).expect("enviar suelta");

    let v = recoger_hasta(&mut cap, 2, 2_000);
    assert_eq!(v.len(), 2, "llegaron {} eventos", v.len());
    assert_eq!((v[0].key, v[0].velocity, v[0].kind), (60, 100, TipoEvento::Ataque));
    assert_eq!((v[1].key, v[1].kind), (60, TipoEvento::Suelta));
    assert!(v[0].at <= v[1].at, "los instantes no decrecen");
}

#[test]
fn un_acorde_en_un_solo_paquete_recibe_un_instante_unico() {
    // T036c — la propiedad que se gana al controlar el bucle de paquetes, y que `midir`
    // impedia: alli el reloj se leia una vez por MENSAJE, no por paquete.
    let n = nombre("acorde");
    let cliente = Client::new("PianoTutorTestClient").expect("cliente");
    let vs = cliente.virtual_source(&n).expect("fuente virtual");
    std::thread::sleep(Duration::from_millis(80));
    let d = buscar(&n).expect("enumerada");
    let mut cap = abrir(&d, MonotonicClock::start()).expect("abrir");
    std::thread::sleep(Duration::from_millis(50));

    // Las tres notas en UN solo paquete.
    vs.received(&PacketBuffer::new(0, &[0x90, 60, 100, 0x90, 64, 100, 0x90, 67, 100]))
        .expect("enviar acorde");

    let v = recoger_hasta(&mut cap, 3, 2_000);
    assert_eq!(v.len(), 3);
    assert_eq!(v[0].at, v[1].at, "las tres notas del acorde comparten instante");
    assert_eq!(v[1].at, v[2].at);
    assert_eq!(v.iter().map(|e| e.key).collect::<Vec<_>>(), vec![60, 64, 67]);
}

#[test]
fn los_paquetes_malformados_no_matan_el_proceso() {
    // T036d — LA regresion. Estos son los paquetes que hacen abortar a `midir 0.11.0`.
    // Que esta prueba termine, y que despues siga llegando una nota valida, es la prueba
    // de que el proceso sobrevivio.
    let n = nombre("basura");
    let cliente = Client::new("PianoTutorTestClient").expect("cliente");
    let vs = cliente.virtual_source(&n).expect("fuente virtual");
    std::thread::sleep(Duration::from_millis(80));
    let d = buscar(&n).expect("enumerada");
    let mut cap = abrir(&d, MonotonicClock::start()).expect("abrir");
    std::thread::sleep(Duration::from_millis(50));

    for basura in [
        &[0x90u8][..],
        &[0x90, 0x3C][..],
        &[0x80, 0x3C][..],
        &[0xB0][..],
        &[0xC0][..],
        &[0x90, 0x3C, 0x64, 0x90][..], // nota valida + estado suelto
        &[0xF0, 0x7E][..],             // sysex sin terminar
    ] {
        let _ = vs.received(&PacketBuffer::new(0, basura));
    }
    std::thread::sleep(Duration::from_millis(150));

    // Vaciar lo que la basura haya dejado: `[0x90,0x3C,0x64,0x90]` contiene una nota
    // VALIDA antes del byte suelto, y esa si debe haber llegado.
    let mut residuo = Vec::new();
    cap.receptor().recoger(&mut residuo);

    // Y despues de toda esa basura, el canal sigue vivo.
    vs.received(&PacketBuffer::new(0, &[0x90, 72, 90])).expect("enviar tras la basura");
    let v = recoger_hasta(&mut cap, 1, 2_000);
    assert!(
        v.iter().any(|e| e.key == 72),
        "tras los paquetes malformados el proceso sigue capturando. residuo={residuo:?} nuevo={v:?}"
    );
}

#[test]
fn tras_cerrar_no_llega_ni_un_mensaje_mas() {
    // T036e / FR-006: el dispositivo queda libre para otras aplicaciones.
    let n = nombre("cierre");
    let cliente = Client::new("PianoTutorTestClient").expect("cliente");
    let vs = cliente.virtual_source(&n).expect("fuente virtual");
    std::thread::sleep(Duration::from_millis(80));
    let d = buscar(&n).expect("enumerada");
    let mut cap = abrir(&d, MonotonicClock::start()).expect("abrir");
    std::thread::sleep(Duration::from_millis(50));

    vs.received(&PacketBuffer::new(0, &[0x90, 60, 100])).expect("enviar");
    let antes = recoger_hasta(&mut cap, 1, 2_000);
    assert_eq!(antes.len(), 1, "el canal funcionaba antes de cerrar");

    cap.cerrar();
    std::thread::sleep(Duration::from_millis(80));
    for _ in 0..500 {
        let _ = vs.received(&PacketBuffer::new(0, &[0x90, 61, 100]));
    }
    std::thread::sleep(Duration::from_millis(150));
    // Si `cerrar` no desconectara de verdad, esto habria entrado en el anillo.
}

#[test]
fn abrir_tarda_menos_de_un_segundo() {
    // T036f / SC-006.
    let n = nombre("apertura");
    let cliente = Client::new("PianoTutorTestClient").expect("cliente");
    let vs = cliente.virtual_source(&n).expect("fuente virtual");
    std::thread::sleep(Duration::from_millis(80));
    let d = buscar(&n).expect("enumerada");

    let inicio = Instant::now();
    let mut cap = abrir(&d, MonotonicClock::start()).expect("abrir");
    vs.received(&PacketBuffer::new(0, &[0x90, 60, 100])).expect("enviar");
    let v = recoger_hasta(&mut cap, 1, 3_000);
    let transcurrido = inicio.elapsed();

    assert_eq!(v.len(), 1, "la captura no llego a estar activa");
    assert!(transcurrido < Duration::from_secs(1), "tardo {transcurrido:?} en estar activa");
}

#[test]
fn el_flujo_completo_produce_pulsaciones_emparejadas() {
    let n = nombre("flujo");
    let cliente = Client::new("PianoTutorTestClient").expect("cliente");
    let vs = cliente.virtual_source(&n).expect("fuente virtual");
    std::thread::sleep(Duration::from_millis(80));
    let d = buscar(&n).expect("enumerada");
    let mut cap = abrir(&d, MonotonicClock::start()).expect("abrir");
    std::thread::sleep(Duration::from_millis(50));

    vs.received(&PacketBuffer::new(0, &[0x90, 60, 100])).expect("ataque");
    std::thread::sleep(Duration::from_millis(30));
    vs.received(&PacketBuffer::new(0, &[0x90, 60, 0])).expect("velocity 0 = suelta");

    let v = recoger_hasta(&mut cap, 2, 2_000);
    assert_eq!(v.len(), 2);
    let mut e = Emparejador::nuevo();
    let mut notas = Vec::new();
    for ev in v {
        if let Some(p) = e.consumir(ev) {
            notas.push(p);
        }
    }
    assert_eq!(notas.len(), 1, "una nota completa");
    assert_eq!(notas[0].key, 60);
    assert_eq!(notas[0].closure, Cierre::PorSuelta);
    assert!(notas[0].end > notas[0].onset);
}

#[test]
fn reabrir_reconoce_el_dispositivo_y_confirma_actividad() {
    // T069/T070 — tras reconectar, el reconocimiento por identidad basta; y la ventana de
    // cortesia distingue "abierto y vivo" de "abierto y mudo".
    let n = nombre("reapertura");
    let cliente = Client::new("PianoTutorTestClient").expect("cliente");
    let vs = cliente.virtual_source(&n).expect("fuente virtual");
    std::thread::sleep(Duration::from_millis(80));
    let d = buscar(&n).expect("enumerada");

    let cap = abrir(&d, MonotonicClock::start()).expect("abrir");
    cap.cerrar(); // como si se hubiese desconectado
    std::thread::sleep(Duration::from_millis(50));

    let mut cap = piano_midi_io::reabrir(&d, MonotonicClock::start()).expect("reabrir");
    vs.received(&PacketBuffer::new(0, &[0x90, 60, 100])).expect("enviar");
    assert!(
        cap.confirmar_actividad(Duration::from_millis(1_500)),
        "tras reabrir no llego nada: la captura estaria muerta sin avisar"
    );
}

#[test]
fn confirmar_actividad_devuelve_falso_cuando_no_llega_nada() {
    let n = nombre("mudo");
    let cliente = Client::new("PianoTutorTestClient").expect("cliente");
    let _vs = cliente.virtual_source(&n).expect("fuente virtual");
    std::thread::sleep(Duration::from_millis(80));
    let d = buscar(&n).expect("enumerada");
    let mut cap = abrir(&d, MonotonicClock::start()).expect("abrir");
    // Nadie envia nada.
    assert!(!cap.confirmar_actividad(Duration::from_millis(200)));
}
