//! T060-T064 — la sesion de captura: estados, perdida del dispositivo y parada.

use piano_core::capture::{Cierre, EstadoSesion, EventoCrudo, SesionDeCaptura, TipoEvento};
use piano_core::time::Micros;

fn ev(seq: u32, us: u64, key: u8, kind: TipoEvento) -> EventoCrudo {
    EventoCrudo { at: Micros(us), seq, key, velocity: 100, kind, channel: 0 }
}

#[test]
fn la_sesion_empieza_inactiva_y_recorre_sus_estados() {
    let mut s = SesionDeCaptura::nueva();
    assert_eq!(s.estado(), EstadoSesion::Inactiva);
    s.abriendo();
    assert_eq!(s.estado(), EstadoSesion::Abriendo);
    s.capturando();
    assert_eq!(s.estado(), EstadoSesion::Capturando);
}

#[test]
fn al_perder_el_dispositivo_las_teclas_hundidas_se_sellan_en_el_ultimo_evento() {
    // T060 — no en el instante de la deteccion, que llega hasta un segundo mas tarde:
    // datarlas ahi inventaria duracion que no ocurrio.
    let mut s = SesionDeCaptura::nueva();
    s.abriendo();
    s.capturando();
    s.consumir(ev(0, 1_000, 60, TipoEvento::Ataque));
    s.consumir(ev(1, 2_500, 64, TipoEvento::Ataque)); // ultimo evento recibido

    let cerradas = s.declarar_perdida();
    assert_eq!(s.estado(), EstadoSesion::Perdida);
    assert_eq!(cerradas.len(), 2);
    // Selladas en el ultimo evento recibido... salvo la que empezo justo en ese mismo
    // instante: esa recibe el minimo de un microsegundo, porque una nota de duracion cero
    // seria un ataque sin sonido. Es la interseccion de dos reglas, y conviene que este
    // escrita: el sellado no puede pasarse del ultimo evento por mas de ese minimo.
    for p in &cerradas {
        assert!(
            p.end.0 <= 2_500.max(p.onset.0 + 1),
            "la nota {p:?} se sello despues del ultimo evento recibido"
        );
        assert!(p.end > p.onset, "ninguna nota dura cero");
    }
    assert!(
        cerradas.iter().any(|p| p.end == Micros(2_500)),
        "la que empezo antes si se sella exactamente en el ultimo evento"
    );
    assert!(cerradas.iter().all(|p| p.closure == Cierre::PorPerdidaDeDispositivo));
    assert!(cerradas.iter().all(|p| p.duracion_censurada), "no sabemos cuanto duraron");
}

#[test]
fn lo_capturado_antes_de_la_perdida_se_conserva_integro() {
    // SC-007.
    let mut s = SesionDeCaptura::nueva();
    s.abriendo();
    s.capturando();
    let mut completas = Vec::new();
    for i in 0..10u64 {
        if let Some(p) = s.consumir(ev(i as u32 * 2, i * 1_000, 60 + i as u8, TipoEvento::Ataque)) {
            completas.push(p);
        }
        if let Some(p) =
            s.consumir(ev(i as u32 * 2 + 1, i * 1_000 + 500, 60 + i as u8, TipoEvento::Suelta))
        {
            completas.push(p);
        }
    }
    assert_eq!(completas.len(), 10, "las diez notas completas antes de la perdida");
    let cerradas = s.declarar_perdida();
    assert_eq!(cerradas.len(), 0, "no quedaba ninguna hundida");
    assert_eq!(completas.len(), 10, "y las anteriores siguen intactas");
}

#[test]
fn detener_cierra_las_hundidas_con_su_propia_etiqueta() {
    let mut s = SesionDeCaptura::nueva();
    s.abriendo();
    s.capturando();
    s.consumir(ev(0, 1_000, 60, TipoEvento::Ataque));
    let cerradas = s.detener(Micros(9_000));
    assert_eq!(s.estado(), EstadoSesion::Inactiva);
    assert_eq!(cerradas.len(), 1);
    assert_eq!(cerradas[0].closure, Cierre::PorParada);
    assert_eq!(cerradas[0].end, Micros(9_000), "la parada si sabe cuando fue");
    assert!(!cerradas[0].duracion_censurada);
}

#[test]
fn la_perdida_nunca_se_infiere_del_silencio() {
    // T063 — solo la declara una notificacion explicita o la doble ausencia. Que no
    // lleguen eventos significa que el alumno no esta tocando, no que se haya
    // desenchufado nada.
    let mut s = SesionDeCaptura::nueva();
    s.abriendo();
    s.capturando();
    // Muchos ciclos sin un solo evento.
    for _ in 0..1_000 {
        assert_eq!(s.estado(), EstadoSesion::Capturando, "el silencio no es una perdida");
    }
}

#[test]
fn hace_falta_una_doble_ausencia_para_declarar_la_perdida() {
    // T064 — una sola enumeracion vacia puede ser un parpadeo del sistema.
    let mut s = SesionDeCaptura::nueva();
    s.abriendo();
    s.capturando();
    assert!(!s.notificar_ausencia(), "una ausencia sola no basta");
    assert_eq!(s.estado(), EstadoSesion::Capturando);
    assert!(s.notificar_ausencia(), "la segunda consecutiva si");
    assert_eq!(s.estado(), EstadoSesion::Perdida);
}

#[test]
fn una_presencia_intermedia_reinicia_la_cuenta_de_ausencias() {
    let mut s = SesionDeCaptura::nueva();
    s.abriendo();
    s.capturando();
    assert!(!s.notificar_ausencia());
    s.notificar_presencia();
    assert!(!s.notificar_ausencia(), "la cuenta se reinicio: esta es la primera otra vez");
    assert_eq!(s.estado(), EstadoSesion::Capturando);
}

#[test]
fn tras_la_perdida_se_puede_reabrir_sin_reiniciar_nada() {
    // FR-024.
    let mut s = SesionDeCaptura::nueva();
    s.abriendo();
    s.capturando();
    s.consumir(ev(0, 100, 60, TipoEvento::Ataque));
    s.declarar_perdida();
    assert_eq!(s.estado(), EstadoSesion::Perdida);

    s.abriendo();
    s.capturando();
    assert_eq!(s.estado(), EstadoSesion::Capturando);
    // Y el emparejador arranca limpio: la nota de antes ya se cerro.
    let cerradas = s.detener(Micros(1_000));
    assert!(cerradas.is_empty(), "no arrastra teclas hundidas de la sesion anterior");
}

#[test]
fn declarar_la_perdida_dos_veces_no_duplica_nada() {
    let mut s = SesionDeCaptura::nueva();
    s.abriendo();
    s.capturando();
    s.consumir(ev(0, 100, 60, TipoEvento::Ataque));
    assert_eq!(s.declarar_perdida().len(), 1);
    assert_eq!(s.declarar_perdida().len(), 0);
}
