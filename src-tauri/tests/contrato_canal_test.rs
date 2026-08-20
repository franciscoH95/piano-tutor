//! La forma exacta del JSON que cruza el puente.
//!
//! Existe porque `#[serde(tag = "tipo", rename_all = "camelCase")]` sobre un enum renombra
//! **las variantes**, no los campos de dentro. Es un detalle facil de dar por supuesto y
//! que no falla ruidosamente: los tipos de TypeScript compilarian igual y el campo llegaria
//! como `undefined` en tiempo de ejecucion. Aqui queda escrito lo que realmente se envia.

use piano_tutor_lib::comandos::MensajeAlFrontend;

fn json(m: &MensajeAlFrontend) -> String {
    serde_json::to_string(m).expect("serializa")
}

#[test]
fn la_tecla_lleva_su_etiqueta_y_sus_campos() {
    let s = json(&MensajeAlFrontend::Tecla { key: 60, pulsada: true });
    assert!(s.contains("\"tipo\":\"tecla\""), "etiqueta de variante: {s}");
    assert!(s.contains("\"key\":60"), "{s}");
    assert!(s.contains("\"pulsada\":true"), "{s}");
}

#[test]
fn el_ancla_declara_como_se_llaman_sus_campos() {
    let s = json(&MensajeAlFrontend::Ancla {
        posicion_us: 1,
        instante_us: 2,
        num: 3,
        den: 4,
        tope_us: Some(5),
    });
    assert!(s.contains("\"tipo\":\"ancla\""), "{s}");
    // Camello, **igual que `AnclaPlana`**. Las dos rutas del puente llevan el mismo dato y
    // tienen que nombrarlo igual; con nombres distintos, uno de los dos lados leeria
    // `undefined` sin que nada fallase ruidosamente.
    assert!(s.contains("\"posicionUs\":1"), "{s}");
    assert!(s.contains("\"instanteUs\":2"), "{s}");
    assert!(s.contains("\"topeUs\":5"), "{s}");
    assert!(!s.contains("posicion_us"), "nada en serpiente: {s}");
}

#[test]
fn el_tope_ausente_viaja_como_nulo() {
    let s = json(&MensajeAlFrontend::Ancla {
        posicion_us: 0,
        instante_us: 0,
        num: 1,
        den: 1,
        tope_us: None,
    });
    assert!(s.contains("null"), "sin tope viaja como null, no se omite: {s}");
}

#[test]
fn las_variantes_sin_campos_llevan_su_etiqueta() {
    assert!(json(&MensajeAlFrontend::Terminada).contains("\"tipo\":\"terminada\""));
    let perdido = json(&MensajeAlFrontend::DispositivoPerdido);
    assert!(
        perdido.contains("\"tipo\":\"dispositivoPerdido\""),
        "la variante compuesta se renombra a camello: {perdido}"
    );
    let esperando = json(&MensajeAlFrontend::Esperando { teclas: vec![60, 64, 67] });
    assert!(esperando.contains("\"tipo\":\"esperando\""), "{esperando}");
    // Un acorde entero, no una sola tecla: con una sola el alumno no ve que le falta.
    assert!(esperando.contains("[60,64,67]"), "{esperando}");
}

#[test]
fn muestra_el_json_real() {
    println!("  TECLA:    {}", json(&MensajeAlFrontend::Tecla { key: 60, pulsada: true }));
    println!(
        "  ANCLA:    {}",
        json(&MensajeAlFrontend::Ancla {
            posicion_us: 1,
            instante_us: 2,
            num: 3,
            den: 4,
            tope_us: Some(5)
        })
    );
    println!("  PERDIDO:  {}", json(&MensajeAlFrontend::DispositivoPerdido));
}

// ---------------------------------------------------------------- T042: el resultado

use piano_tutor_lib::comandos::{PorMano, RecuentoPlano, ResultadoPlano};

fn recuento() -> RecuentoPlano {
    RecuentoPlano { acertadas: 0, fuera_de_tiempo: 0, omitidas: 0 }
}

#[test]
fn el_resultado_viaja_en_camello_como_el_resto_del_puente() {
    // La 003 dejó fijado que los dos caminos del puente nombran igual el mismo dato. Aquí
    // se mantiene: con nombres distintos, un lado leería `undefined` sin que nada fallase.
    let r = ResultadoPlano {
        acertadas: 3,
        fuera_de_tiempo: 1,
        omitidas: 2,
        de_mas: 1,
        dedos_escapados: 1,
        fuera_de_alcance: 0,
        no_intentadas: 0,
        intentadas: 6,
        desfase_mediana_us: Some(-40_000),
        desfase_dispersion_us: Some(5_000),
        sin_tocar: false,
        parcial: true,
        por_mano: PorMano { izquierda: recuento(), derecha: recuento() },
    };
    let s = serde_json::to_string(&r).expect("serializa");
    assert!(s.contains("\"fueraDeTiempo\":1"), "{s}");
    assert!(s.contains("\"dedosEscapados\":1"), "{s}");
    assert!(s.contains("\"desfaseMedianaUs\":-40000"), "el signo viaja: {s}");
    assert!(s.contains("\"parcial\":true"), "{s}");
    assert!(!s.contains("fuera_de_tiempo"), "nada en serpiente: {s}");
}

#[test]
fn sin_desfase_los_campos_viajan_como_nulos() {
    // No se omiten: la interfaz distingue «no hay desfase» de «el campo no llegó».
    let r = ResultadoPlano {
        acertadas: 0,
        fuera_de_tiempo: 0,
        omitidas: 0,
        de_mas: 0,
        dedos_escapados: 0,
        fuera_de_alcance: 0,
        no_intentadas: 0,
        intentadas: 0,
        desfase_mediana_us: None,
        desfase_dispersion_us: None,
        sin_tocar: true,
        parcial: false,
        por_mano: PorMano { izquierda: recuento(), derecha: recuento() },
    };
    let s = serde_json::to_string(&r).expect("serializa");
    assert!(s.contains("\"desfaseMedianaUs\":null"), "{s}");
}

#[test]
fn no_poder_abrir_es_una_variante_propia_y_lleva_su_motivo() {
    // Antes este caso viajaba como `dispositivoPerdido`, asi que la interfaz decia «se
    // perdio la conexion» justo despues de decir «Conectado». Aqui queda escrito que son
    // dos etiquetas distintas y que el motivo cruza el puente.
    let s = json(&MensajeAlFrontend::NoSePudoAbrir {
        motivo: "no se pudo abrir «Piano X»: el sistema respondio 0x80070005".into(),
    });
    assert!(s.contains("\"tipo\":\"noSePudoAbrir\""), "etiqueta de variante: {s}");
    assert!(s.contains("0x80070005"), "el codigo tiene que llegar entero: {s}");
    assert!(!s.contains("dispositivoPerdido"), "no es una perdida: {s}");
}

#[test]
fn perder_el_teclado_sigue_siendo_una_etiqueta_distinta() {
    let s = json(&MensajeAlFrontend::DispositivoPerdido);
    assert!(s.contains("\"tipo\":\"dispositivoPerdido\""), "{s}");
}
