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
    assert!(json(&MensajeAlFrontend::Esperando { key: 72 }).contains("\"tipo\":\"esperando\""));
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
