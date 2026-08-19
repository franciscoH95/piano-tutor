//! T039a y T044 — preparar una canción para practicarla.
//!
//! Dos comportamientos que la especificación pide por separado pero que comparten estado:
//! cargar otra canción no debe arrastrar nada de la anterior (FR-005), y mover el punto de
//! corte debe recalcular manos **y digitación**, no sólo el color (FR-003c).

mod fixtures;
use fixtures::SmfBuilder;
use piano_core::load_smf;
use piano_core::practica::{Avance, Mano, MascaraTeclas, Preparacion, Reparto};
use piano_core::time::{Micros, Ticks};
use piano_core::Song;

/// Dos voces claramente separadas: agudos en una pista, graves en otra.
fn cancion_a() -> Song {
    let raw = SmfBuilder::new(480)
        .track(|t| {
            let mut t = t;
            for i in 0..20u64 {
                t = t.note(i * 480, 72 + (i % 12) as u8, 90, 240);
            }
            t
        })
        .track(|t| {
            let mut t = t;
            for i in 0..20u64 {
                t = t
                    .raw(i * 480, &[0x91, 40 + (i % 12) as u8, 90])
                    .raw(i * 480 + 240, &[0x81, 40 + (i % 12) as u8, 0]);
            }
            t
        })
        .build();
    load_smf(&raw).expect("cancion_a debe cargar")
}

/// Otra canción, deliberadamente más corta y de una sola voz.
fn cancion_b() -> Song {
    let raw = SmfBuilder::new(480)
        .track(|t| t.note(0, 60, 90, 240).note(240, 62, 90, 240))
        .build();
    load_smf(&raw).expect("cancion_b debe cargar")
}

fn una_voz() -> Song {
    let raw = SmfBuilder::new(480)
        .track(|t| t.note(0, 60, 90, 240).note(240, 64, 90, 240).note(480, 67, 90, 240))
        .build();
    load_smf(&raw).expect("debe cargar")
}

fn manos_de(p: &Preparacion) -> Vec<Mano> {
    (0..p.reparto().len()).map(|i| p.reparto().mano(i)).collect()
}

#[test]
fn cargar_otra_cancion_no_arrastra_nada_de_la_anterior() {
    // FR-005. Se ensucia el estado a conciencia antes de cargar la otra: se mueve el corte
    // y se avanza por la pieza.
    let mut p = Preparacion::nueva(cancion_a());
    p.ajustar_corte(84);
    p.avanzar_a(1_000_000);
    assert_ne!(p.corte(), Preparacion::CORTE_POR_DEFECTO, "el estado está sucio");
    assert_ne!(p.posicion(), 0, "el cursor está avanzado");

    p.cargar(cancion_b());
    let limpia = Preparacion::nueva(cancion_b());

    assert_eq!(p.corte(), limpia.corte(), "el corte vuelve al valor de partida");
    assert_eq!(p.posicion(), limpia.posicion(), "el cursor vuelve al principio");
    assert_eq!(p.reparto().len(), limpia.reparto().len(), "reparto rehecho");
    assert_eq!(p.digitacion().len(), limpia.digitacion().len(), "digitación rehecha");
    for i in 0..limpia.reparto().len() {
        assert_eq!(p.reparto().mano(i), limpia.reparto().mano(i), "nota {i}");
        assert_eq!(p.digitacion().dedo(i), limpia.digitacion().dedo(i), "nota {i}");
    }
}

#[test]
fn cargar_otra_cancion_no_deja_notas_de_la_anterior() {
    // La comprobación más directa de que no queda residuo: B tiene muchas menos notas que
    // A, así que si algo se arrastrase, sobraría.
    let mut p = Preparacion::nueva(cancion_a());
    assert_eq!(p.digitacion().len(), 40, "A trae 40 notas");
    p.cargar(cancion_b());
    assert_eq!(p.cancion().notes().len(), 2, "exactamente las notas de B");
    assert_eq!(p.digitacion().len(), 2, "la digitación no sobrevive a la carga");
    assert_eq!(p.reparto().len(), 2, "el reparto tampoco");
}

#[test]
fn mover_el_corte_recalcula_manos_y_tambien_digitacion() {
    // FR-003c. Es la parte que se olvida: mover el corte cambia de qué mano es cada nota,
    // y una nota que cambia de mano se digita con la reflexión contraria, así que su dedo
    // puede cambiar. Recalcular sólo el color dejaría digitaciones de la otra mano.
    let mut p = Preparacion::nueva(una_voz());
    assert_eq!(p.reparto().origen(), Reparto::CortePorAltura, "una sola voz");
    let antes = manos_de(&p);

    p.ajustar_corte(96); // por encima de todo: las tres notas pasan a la izquierda
    assert_ne!(antes, manos_de(&p), "el corte cambió el reparto");
    assert!(manos_de(&p).iter().all(|m| *m == Mano::Izquierda), "todas a la izquierda");

    // Y la digitación se rehízo con la mano nueva: debe coincidir con la de una preparación
    // que naciera ya con ese corte, no con la que tenía antes de moverlo.
    let mut referencia = Preparacion::nueva(una_voz());
    referencia.ajustar_corte(96);
    for i in 0..p.digitacion().len() {
        assert_eq!(
            p.digitacion().dedo(i),
            referencia.digitacion().dedo(i),
            "la digitación de la nota {i} corresponde a la mano nueva"
        );
    }
}

#[test]
fn el_corte_no_manda_cuando_el_archivo_trae_las_voces() {
    // El control sigue visible —eso es cosa de la interfaz—, pero el reparto del archivo
    // manda sobre él.
    let mut p = Preparacion::nueva(cancion_a());
    assert_eq!(p.reparto().origen(), Reparto::VocesDelArchivo, "dos voces");
    let antes = manos_de(&p);
    p.ajustar_corte(96);
    assert_eq!(antes, manos_de(&p), "las voces del archivo mandan sobre el corte");
}

#[test]
fn una_cancion_sin_notas_se_prepara_sin_reventar() {
    let vacia = load_smf(&SmfBuilder::new(480).track(|t| t).build()).expect("cargable");
    let mut p = Preparacion::nueva(vacia);
    assert_eq!(p.digitacion().len(), 0);
    p.ajustar_corte(70);
    p.avanzar_a(5_000_000);
    assert_eq!(p.reparto().len(), 0);
}

// ------------------------------------------------- detalle para pintar

#[test]
fn el_detalle_cruza_nota_mano_dedo_y_nombre() {
    // El puente no debe cruzar nada por su cuenta: `NotaVisible` sólo trae el índice, y
    // unir ese índice con la mano, el dedo y el nombre es una decisión del dominio. Se
    // hace aquí, donde hay pruebas, y no en `src-tauri`, donde no las hay.
    let mut p = Preparacion::nueva(una_voz());
    let mut out = Vec::new();
    p.detallar(0, 2_000_000, &mut out);

    assert_eq!(out.len(), 3, "las tres notas caen en la ventana");
    for (i, d) in out.iter().enumerate() {
        assert_eq!(d.indice, i, "el índice se conserva");
        assert_eq!(d.key, p.cancion().notes()[i].key, "misma altura");
        assert_eq!(d.mano, p.reparto().mano(i), "la mano que dice el reparto");
        assert_eq!(d.dedo, p.digitacion().dedo(i), "el dedo que dice la digitación");
    }
    assert_eq!(out[0].nombre, p.cancion().armaduras().nombre(Ticks(0), 60), "Do");
}

#[test]
fn el_detalle_solo_trae_lo_que_cae_en_la_ventana() {
    let mut p = Preparacion::nueva(una_voz());
    let mut out = Vec::new();
    p.detallar(0, 1, &mut out);
    assert_eq!(out.len(), 1, "sólo la primera nota toca el instante 0");
}

#[test]
fn mover_el_corte_se_ve_en_el_detalle() {
    // Es lo que hace visible FR-003c: el color y el dedo que se pintan cambian juntos.
    let mut p = Preparacion::nueva(una_voz());
    let mut antes = Vec::new();
    p.detallar(0, 2_000_000, &mut antes);
    p.ajustar_corte(96);
    let mut despues = Vec::new();
    p.detallar(0, 2_000_000, &mut despues);
    assert!(antes.iter().any(|d| d.mano == Mano::Derecha), "antes había derecha");
    assert!(despues.iter().all(|d| d.mano == Mano::Izquierda), "ahora todo izquierda");
}

// ------------------------------------------------- el transporte, ya con cursor

#[test]
fn reproducir_mueve_la_posicion_y_la_vista_la_sigue() {
    let mut p = Preparacion::nueva(una_voz());
    p.poner_en_marcha(Micros::ZERO);
    let paso = p.avanzar(Micros(400_000));
    assert_eq!(paso.posicion, Micros(400_000));
    assert_eq!(p.posicion(), 400_000, "la preparación queda donde dice el cursor");
}

#[test]
fn cargar_otra_cancion_reinicia_tambien_el_transporte() {
    // FR-005 otra vez, ahora con la parte que antes no existía: si el cursor sobreviviera
    // a la carga, la canción nueva empezaría por la mitad y ya en marcha.
    let mut p = Preparacion::nueva(cancion_a());
    p.poner_en_marcha(Micros::ZERO);
    p.avanzar(Micros(3_000_000));
    assert_ne!(p.posicion(), 0, "el cursor está avanzado");

    p.cargar(cancion_b());
    assert_eq!(p.posicion(), 0, "la canción nueva empieza por el principio");
    // Y parada: avanzar el reloj no la mueve mientras no se ponga en marcha.
    p.avanzar(Micros(9_000_000));
    assert_eq!(p.posicion(), 0, "y parada, no heredando la marcha de la anterior");
}

#[test]
fn el_final_de_la_cancion_se_avisa_una_sola_vez_desde_la_preparacion() {
    let mut p = Preparacion::nueva(cancion_b());
    p.poner_en_marcha(Micros::ZERO);
    let fin = p.cancion().duration_us();
    let mut avisos = 0;
    for i in 1..=50u64 {
        if p.avanzar(Micros(i * fin.0 / 10)).terminada {
            avisos += 1;
        }
    }
    assert_eq!(avisos, 1);
}

#[test]
fn saltar_hacia_atras_recoloca_la_vista() {
    // La parte que enlaza el cursor con la vista: sin recolocar, tras saltar atrás el
    // cursor monótono de la vista se queda por delante y no devuelve nada.
    let mut p = Preparacion::nueva(una_voz());
    p.poner_en_marcha(Micros::ZERO);
    p.avanzar(Micros(700_000));
    let mut lejos = Vec::new();
    p.detallar(p.posicion(), p.posicion() + 1, &mut lejos);

    p.saltar_a(Micros::ZERO, Micros(700_000));
    assert_eq!(p.posicion(), 0);
    let mut cerca = Vec::new();
    p.detallar(0, 1, &mut cerca);
    assert!(!cerca.is_empty(), "tras volver al principio se ve la primera nota");
    assert_ne!(cerca.first().map(|n| n.indice), lejos.first().map(|n| n.indice));
}

// ------------------------------------------------- modo espera

#[test]
fn la_preparacion_espera_en_la_primera_nota_y_avanza_al_acertarla() {
    let mut p = Preparacion::nueva(una_voz());
    p.cambiar_avance(Avance::PorAcierto, Micros::ZERO);
    p.poner_en_marcha(Micros::ZERO);

    let paso = p.avanzar_con(Micros(900_000), MascaraTeclas::VACIA);
    assert_eq!(paso.posicion, Micros::ZERO, "espera en la primera");
    assert!(paso.esperando);

    let mut m = MascaraTeclas::VACIA;
    m.poner(60);
    p.avanzar_con(Micros(900_000), m);
    let paso = p.avanzar_con(Micros(1_000_000), m);
    assert!(paso.posicion > Micros::ZERO, "acertada, avanza");
}

#[test]
fn mover_el_corte_rehace_tambien_las_puertas() {
    // El corte cambia de qué mano es cada nota, y con una mano practicada eso cambia qué
    // puertas existen. Sin rehacerlas, el alumno esperaría en notas que ya no son suyas.
    let mut p = Preparacion::nueva(una_voz());
    p.practicar_mano(Some(Mano::Izquierda), Micros::ZERO);
    p.cambiar_avance(Avance::PorAcierto, Micros::ZERO);
    p.poner_en_marcha(Micros::ZERO);
    // Con el corte por defecto (60) las tres notas (60, 64, 67) son de la derecha, así que
    // practicando la izquierda no hay ninguna puerta y el cursor corre libre.
    let suelto = p.avanzar_con(Micros(500_000), MascaraTeclas::VACIA).posicion;
    assert_eq!(suelto, Micros(500_000), "sin puertas de la izquierda, avanza");

    // Se sube el corte: ahora las tres son de la izquierda y sí hay puertas.
    p.saltar_a(Micros::ZERO, Micros(500_000));
    p.ajustar_corte(96);
    let parado = p.avanzar_con(Micros(1_500_000), MascaraTeclas::VACIA).posicion;
    assert_eq!(parado, Micros::ZERO, "ahora sí espera: las puertas se rehicieron");
}

#[test]
fn cargar_otra_cancion_reinicia_el_modo_y_las_puertas() {
    let mut p = Preparacion::nueva(una_voz());
    p.cambiar_avance(Avance::PorAcierto, Micros::ZERO);
    p.cargar(cancion_b());
    assert_eq!(p.avance(), Avance::PorReloj, "la canción nueva empieza por reloj");
}

// ------------------------------------------------- evaluación

#[test]
fn pausar_cierra_la_interpretacion_y_reanudar_abre_otra() {
    // T035a (FR-014a). Es la misma frontera que el cursor ya usa para cambiar de régimen,
    // así que se comprueba contra ella y no contra un concepto nuevo.
    let mut p = Preparacion::nueva(una_voz());
    p.poner_en_marcha(Micros::ZERO);
    assert!(p.resultado().is_none(), "en marcha todavía no hay resultado");

    p.pausar(Micros(500_000));
    assert!(p.resultado().is_some(), "pausar cierra la interpretación");

    p.poner_en_marcha(Micros(600_000));
    assert!(p.resultado().is_none(), "reanudar abre otra, y todavía no ha cerrado");
}

#[test]
fn saltar_cierra_la_interpretacion() {
    let mut p = Preparacion::nueva(una_voz());
    p.poner_en_marcha(Micros::ZERO);
    p.saltar_a(Micros::ZERO, Micros(500_000));
    assert!(p.resultado().is_some(), "saltar también la cierra");
}

#[test]
fn una_interpretacion_que_no_llega_al_final_se_evalua_igual() {
    // T035b (FR-014b). Exigir un recorrido completo dejaría sin retorno al principiante,
    // que casi nunca termina.
    let mut p = Preparacion::nueva(una_voz());
    p.poner_en_marcha(Micros::ZERO);
    let mut m = MascaraTeclas::VACIA;
    m.poner(60);
    p.observar_tecla(60, true, Micros(10_000));
    p.avanzar_con(Micros(300_000), m);
    p.pausar(Micros(400_000)); // se para a mitad

    let r = p.resultado().expect("hay resultado");
    assert!(!r.sin_tocar, "sí tocó");
    assert_eq!(r.acertadas, 1, "se evalúa el tramo recorrido");
}

#[test]
fn cargar_otra_cancion_tira_el_resultado_anterior() {
    // FR-005 otra vez: el resultado de la canción anterior no puede sobrevivir a la carga.
    let mut p = Preparacion::nueva(una_voz());
    p.poner_en_marcha(Micros::ZERO);
    p.pausar(Micros(500_000));
    assert!(p.resultado().is_some());
    p.cargar(cancion_b());
    assert!(p.resultado().is_none(), "la canción nueva empieza sin resultado");
}
