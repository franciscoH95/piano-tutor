//! Pruebas de `core/src/evaluacion/evaluacion.rs`.

mod fixtures;
use piano_core::evaluacion::{Nivel, Tolerancias};

const NIVELES: [Nivel; 3] = [Nivel::Permisivo, Nivel::Intermedio, Nivel::Exigente];

// ---------------------------------------------------------------- T006, T008

#[test]
fn las_ventanas_de_ataque_estan_anidadas() {
    // T006. Es lo que hace que SC-006 se cumpla por ARITMÉTICA y no por vigilancia: si el
    // permisivo contiene al intermedio y este al exigente, el permisivo no puede dar menos
    // aciertos. Si alguien ajusta un número y rompe el orden, esta prueba lo caza.
    let t: Vec<Tolerancias> = NIVELES.iter().map(|n| n.tolerancias()).collect();
    assert!(
        t[0].ventana_ataque_us > t[1].ventana_ataque_us,
        "permisivo ({}) debe ser más ancho que intermedio ({})",
        t[0].ventana_ataque_us,
        t[1].ventana_ataque_us
    );
    assert!(
        t[1].ventana_ataque_us > t[2].ventana_ataque_us,
        "intermedio ({}) debe ser más ancho que exigente ({})",
        t[1].ventana_ataque_us,
        t[2].ventana_ataque_us
    );
}

#[test]
fn la_ventana_de_emparejamiento_es_la_misma_en_los_tres_niveles() {
    // La otra mitad de la decisión de las dos ventanas: si el emparejamiento dependiera del
    // nivel, cambiar de nivel cambiaría QUÉ se empareja con qué, y una nota podría quedar
    // acertada en el exigente y sin pareja en el permisivo.
    let t: Vec<Tolerancias> = NIVELES.iter().map(|n| n.tolerancias()).collect();
    assert_eq!(t[0].ventana_emparejamiento_us, t[1].ventana_emparejamiento_us);
    assert_eq!(t[1].ventana_emparejamiento_us, t[2].ventana_emparejamiento_us);
}

#[test]
fn la_ventana_de_emparejamiento_contiene_a_la_de_ataque_mas_ancha() {
    // Si la de ataque fuese más ancha que la de emparejamiento, habría notas «dentro de
    // tolerancia» que nunca llegan a emparejarse. Sería un acierto imposible de conseguir.
    let t = Nivel::Permisivo.tolerancias();
    assert!(t.ventana_emparejamiento_us >= t.ventana_ataque_us);
}

#[test]
fn ningun_umbral_vive_fuera_de_tolerancias() {
    // T008. Criterio comprobable, no «literales sospechosos»: en `core/src/evaluacion/`,
    // fuera de `tolerancias.rs`, ningún literal entero mayor que 1.000 ni ninguno con
    // separador de millares. El Principio I lo exige textualmente y es la clase de regla
    // que se erosiona sola: alguien mete un 60_000 en el sitio equivocado y nadie lo nota.
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/evaluacion");
    let mut culpables = Vec::new();
    for entrada in std::fs::read_dir(&dir).expect("el módulo existe") {
        let ruta = entrada.expect("entrada legible").path();
        if ruta.file_name().is_some_and(|n| n == "tolerancias.rs") {
            continue;
        }
        let texto = std::fs::read_to_string(&ruta).expect("legible");
        for (n, linea) in texto.lines().enumerate() {
            let codigo = linea.split("//").next().unwrap_or("");
            for palabra in codigo.split(|c: char| !c.is_ascii_digit() && c != '_') {
                if palabra.contains('_') && palabra.chars().any(|c| c.is_ascii_digit()) {
                    culpables.push(format!("{}:{} → {palabra}", ruta.display(), n + 1));
                } else if let Ok(v) = palabra.parse::<u64>() {
                    if v > 1_000 {
                        culpables.push(format!("{}:{} → {v}", ruta.display(), n + 1));
                    }
                }
            }
        }
    }
    assert!(
        culpables.is_empty(),
        "estos umbrales deberían vivir en tolerancias.rs:\n  {}",
        culpables.join("\n  ")
    );
}

// ---------------------------------------------------------------- T019

#[test]
fn que_notas_puede_tocar_el_alumno() {
    // T019. Un solo criterio, consumido a la vez por las puertas y por el evaluador. Si
    // vive en dos sitios, vuelven a divergir: ya pasó con la percusión, donde el comentario
    // decía que se filtraba y el código no lo hacía.
    use piano_core::evaluacion::es_evaluable;
    use piano_core::practica::Mano;

    // Percusión: no se toca con las manos en el teclado.
    assert!(!es_evaluable(9, 38, Mano::Derecha, None), "canal 9 fuera");
    // Y no basta con mirar la altura: una caja está en la tecla 38, dentro del piano.
    assert!(es_evaluable(0, 38, Mano::Derecha, None), "la misma tecla en otro canal sí");

    // Fuera de las 88 teclas: el alumno no puede tocarlas (FR-014).
    assert!(!es_evaluable(0, 20, Mano::Derecha, None), "por debajo del la 0");
    assert!(!es_evaluable(0, 109, Mano::Derecha, None), "por encima del do 8");
    assert!(es_evaluable(0, 21, Mano::Derecha, None), "el la 0 sí");
    assert!(es_evaluable(0, 108, Mano::Derecha, None), "el do 8 sí");

    // La mano no practicada: no es que se falle, es que no se le pide.
    assert!(!es_evaluable(0, 60, Mano::Derecha, Some(Mano::Izquierda)));
    assert!(es_evaluable(0, 60, Mano::Derecha, Some(Mano::Derecha)));
    assert!(es_evaluable(0, 60, Mano::Derecha, None), "sin mano elegida, las dos");
}
