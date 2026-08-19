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
            // Un literal es una tirada de dígitos y guiones bajos que **no está pegada a una
            // letra**: sin esa condición, `is_on_88_keys` se leería como el literal `_88_`.
            let bytes: Vec<char> = codigo.chars().collect();
            let mut i = 0;
            while i < bytes.len() {
                if !bytes[i].is_ascii_digit() {
                    i += 1;
                    continue;
                }
                let antes_es_letra = i > 0 && (bytes[i - 1].is_alphabetic() || bytes[i - 1] == '_');
                let inicio = i;
                while i < bytes.len() && (bytes[i].is_ascii_digit() || bytes[i] == '_') {
                    i += 1;
                }
                let despues_es_letra = i < bytes.len() && bytes[i].is_alphabetic();
                if antes_es_letra || despues_es_letra {
                    continue;
                }
                let literal: String = bytes[inicio..i].iter().collect();
                let limpio = literal.replace('_', "");
                let grande = limpio.parse::<u64>().is_ok_and(|v| v > 1_000);
                if literal.contains('_') || grande {
                    culpables.push(format!("{}:{} → {literal}", ruta.display(), n + 1));
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

// ---------------------------------------------------------------- el evaluador

use fixtures::interpretaciones::{ataque, suelta};
use fixtures::SmfBuilder;
use piano_core::evaluacion::{Evaluador, Veredicto};
use piano_core::load_smf;
use piano_core::practica::Mano;
use piano_core::time::Micros;
use piano_core::Song;

/// Un tick es un milisegundo: las cifras se leen en microsegundos ×1000.
fn cancion(notas: &[(u64, u8, u64)]) -> Song {
    let ns = notas.to_vec();
    let raw = SmfBuilder::new(1000)
        .track(|t| {
            let mut t = t.tempo(0, 1_000_000);
            for (tick, key, dur) in &ns {
                t = t.note(*tick, *key, 90, *dur);
            }
            t
        })
        .build();
    load_smf(&raw).expect("valida")
}

/// Un evaluador a tempo normal sobre toda la canción, con las dos manos.
fn evaluador(song: &Song, nivel: Nivel) -> Evaluador {
    let manos = vec![Mano::Derecha; song.notes().len()];
    Evaluador::nuevo(song, &manos, None, nivel)
}

#[test]
fn una_interpretacion_perfecta_acierta_todo_en_los_tres_niveles() {
    // SC-001 (T034).
    let song = cancion(&[(0, 60, 400), (500, 62, 400), (1_000, 64, 400)]);
    for nivel in NIVELES {
        let mut e = evaluador(&song, nivel);
        for (t, k) in [(0u64, 60u8), (500_000, 62), (1_000_000, 64)] {
            e.observar(ataque(t, k, 90));
            e.observar(suelta(t + 400_000, k));
        }
        let r = e.cerrar(Micros(2_000_000));
        assert_eq!(r.acertadas, 3, "nivel {nivel:?}");
        assert_eq!(r.omitidas, 0, "nivel {nivel:?}");
        assert_eq!(r.de_mas, 0, "nivel {nivel:?}");
    }
}

#[test]
fn no_tocar_nada_no_es_lo_mismo_que_tocar_mal() {
    // SC-002 (T035). Son cosas distintas y el alumno las lee distinto.
    let song = cancion(&[(0, 60, 400), (500, 62, 400)]);
    let e = evaluador(&song, Nivel::Intermedio);
    let r = e.cerrar(Micros(2_000_000));
    assert!(r.sin_tocar, "se comunica como «no se tocó nada»");
    assert_eq!(r.acertadas, 0);
}

#[test]
fn tocar_mal_no_se_comunica_como_no_haber_tocado() {
    // La contraparte: si se tocó y todo estuvo mal, NO es «no se tocó nada».
    let song = cancion(&[(0, 60, 400)]);
    let mut e = evaluador(&song, Nivel::Exigente);
    e.observar(ataque(0, 71, 90));
    e.observar(suelta(100_000, 71));
    let r = e.cerrar(Micros(2_000_000));
    assert!(!r.sin_tocar, "sí tocó, aunque mal");
    assert_eq!(r.de_mas, 1);
    assert_eq!(r.omitidas, 1);
}

#[test]
fn el_emparejamiento_es_uno_a_uno() {
    // T025 (FR-002). Biyección parcial: ninguna nota recibe dos pulsaciones y ninguna
    // pulsación va a dos notas. Con repeticiones de la misma tecla, que es donde falla.
    let song = cancion(&[(0, 60, 200), (300, 60, 200), (600, 60, 200)]);
    let mut e = evaluador(&song, Nivel::Permisivo);
    for t in [0u64, 300_000, 600_000] {
        e.observar(ataque(t, 60, 90));
        e.observar(suelta(t + 200_000, 60));
    }
    let r = e.cerrar(Micros(1_000_000));
    assert_eq!(r.acertadas, 3);
    assert_eq!(r.de_mas, 0, "ninguna pulsación se quedó sin nota");
    assert_eq!(r.omitidas, 0, "ninguna nota se quedó sin pulsación");
}

#[test]
fn una_sola_pulsacion_no_puede_acertar_dos_notas() {
    // La canción pide la misma tecla dos veces y el alumno la toca UNA. Una de las dos
    // queda omitida: no se puede cubrir dos notas con una pulsación.
    let song = cancion(&[(0, 60, 200), (300, 60, 200)]);
    let mut e = evaluador(&song, Nivel::Permisivo);
    e.observar(ataque(0, 60, 90));
    e.observar(suelta(200_000, 60));
    let r = e.cerrar(Micros(1_000_000));
    assert_eq!(r.acertadas, 1);
    assert_eq!(r.omitidas, 1);
    assert_eq!(r.de_mas, 0);
}

#[test]
fn una_nota_ya_juzgada_no_cambia_por_lo_que_venga_despues() {
    // T026 (FR-004). Se toca la primera nota, se cierra su ventana, y después se toca algo
    // que con visión de futuro habría cambiado el emparejamiento. El veredicto de la
    // primera no se mueve.
    let song = cancion(&[(0, 60, 200), (2_000, 60, 200)]);
    let mut e = evaluador(&song, Nivel::Exigente);
    e.observar(ataque(0, 60, 90));
    e.observar(suelta(150_000, 60));
    e.avanzar(Micros(1_500_000));
    let tras_la_primera = e.veredicto_de(0);
    assert_eq!(tras_la_primera, Veredicto::Acertada);

    e.observar(ataque(2_000_000, 60, 90));
    e.observar(suelta(2_150_000, 60));
    e.avanzar(Micros(3_000_000));
    assert_eq!(e.veredicto_de(0), tras_la_primera, "no se revisó");
}

// ---------------------------------------------------------------- las medidas

#[test]
fn se_registran_las_tres_medidas_con_su_signo() {
    // T030a (FR-005, FR-006, FR-007). El signo del desfase ES la información.
    let song = cancion(&[(1_000, 60, 500)]); // 1.000.000 → 1.500.000
    let mut e = evaluador(&song, Nivel::Permisivo);
    e.observar(ataque(1_040_000, 60, 77)); // 40 ms tarde
    e.observar(suelta(1_400_000, 60)); // sostenida 360 ms frente a los 500 escritos
    e.avanzar(Micros(2_000_000));
    let m = e.medida_de(0).expect("emparejada");
    assert_eq!(m.desfase_us, 40_000, "positivo: se atrasó");
    // Diferencia de DURACIÓN, no de instante final: 360 ms tocados menos 500 escritos.
    assert_eq!(m.duracion_us, Some(-140_000), "negativa: la sostuvo menos de lo escrito");
    assert_eq!(m.velocity, 77);
}

#[test]
fn adelantarse_da_desfase_negativo() {
    let song = cancion(&[(1_000, 60, 500)]);
    let mut e = evaluador(&song, Nivel::Permisivo);
    e.observar(ataque(960_000, 60, 90)); // 40 ms antes
    e.observar(suelta(1_400_000, 60));
    e.avanzar(Micros(2_000_000));
    assert_eq!(e.medida_de(0).expect("emparejada").desfase_us, -40_000);
}

#[test]
fn la_duracion_y_la_intensidad_no_alteran_el_veredicto() {
    // T030b (FR-006). Una nota con el ataque perfecto y soltada enseguida sigue acertada.
    let song = cancion(&[(1_000, 60, 2_000)]); // dura 2 segundos
    let mut e = evaluador(&song, Nivel::Exigente);
    e.observar(ataque(1_000_000, 60, 1)); // ataque exacto, intensidad mínima
    e.observar(suelta(1_010_000, 60)); // soltada a los 10 ms
    e.avanzar(Micros(4_000_000));
    assert_eq!(e.veredicto_de(0), Veredicto::Acertada, "el veredicto lo decide el ataque");
    let m = e.medida_de(0).expect("emparejada");
    assert!(m.duracion_us.is_some_and(|d| d < 0), "y la diferencia se comunica aparte");
}

#[test]
fn una_tecla_hundida_al_cerrar_deja_la_duracion_desconocida() {
    // T030c. Desconocida, que no es cero: cero significaría que la sostuvo exactamente lo
    // escrito, y eso sería mentir.
    let song = cancion(&[(1_000, 60, 500)]);
    let mut e = evaluador(&song, Nivel::Permisivo);
    e.observar(ataque(1_000_000, 60, 90)); // nunca se suelta
    e.avanzar(Micros(2_000_000));
    let m = e.medida_de(0).expect("emparejada");
    assert_eq!(m.duracion_us, None, "desconocida, no cero");
    assert_eq!(e.veredicto_de(0), Veredicto::Acertada, "y sigue siendo acierto");
}

// ---------------------------------------------------------------- T031, T031a-c

#[test]
fn las_seis_clases_de_veredicto_se_distinguen() {
    // T031.
    let raw = SmfBuilder::new(1000)
        .track(|t| {
            t.tempo(0, 1_000_000)
                .note(0, 60, 90, 300)      // se acierta
                .note(1_000, 62, 90, 300)  // se toca fuera de tiempo
                .note(2_000, 64, 90, 300)  // se omite
                .note(3_000, 109, 90, 300) // fuera de las 88
        })
        .build();
    let song = load_smf(&raw).expect("valida");
    let mut e = evaluador(&song, Nivel::Exigente);
    e.observar(ataque(0, 60, 90));
    e.observar(suelta(200_000, 60));
    e.observar(ataque(1_100_000, 62, 90)); // 100 ms tarde: fuera de la ventana exigente
    e.observar(suelta(1_300_000, 62));
    e.observar(ataque(5_000_000, 71, 90)); // nota de más, lejos de todo
    e.observar(suelta(5_100_000, 71));
    let r = e.cerrar(Micros(6_000_000));

    assert_eq!(r.acertadas, 1);
    assert_eq!(r.fuera_de_tiempo, 1, "emparejada pero fuera de la ventana de ataque");
    assert_eq!(r.omitidas, 1);
    assert_eq!(r.fuera_de_alcance, 1, "la que no puede tocar, fuera del denominador");
    assert_eq!(r.de_mas, 1);
    assert_eq!(r.intentadas(), 3, "el denominador honesto: sin la de fuera de alcance");
}

#[test]
fn rozar_la_tecla_de_al_lado_es_un_dedo_que_se_escapa() {
    // T031a (SC-013). El error más frecuente de un principiante. Contarlo igual que tocar
    // un pasaje entero equivocado castiga dos veces el mismo tropiezo.
    let song = cancion(&[(1_000, 64, 300)]);
    let mut e = evaluador(&song, Nivel::Permisivo);
    e.observar(ataque(990_000, 65, 60)); // roza el Fa
    e.observar(suelta(1_000_000, 65));
    e.observar(ataque(1_000_000, 64, 90)); // y toca el Mi
    e.observar(suelta(1_300_000, 64));
    let r = e.cerrar(Micros(3_000_000));

    assert_eq!(r.acertadas, 1, "el acierto sigue contando");
    assert_eq!(r.dedos_escapados, 1, "y la contigua se cuenta aparte");
    assert_eq!(r.de_mas, 0, "no como una nota de más cualquiera");
}

#[test]
fn la_cercania_del_dedo_no_se_traga_notas_legitimas() {
    // T031b. Si la canción pide la contigua en ese instante, es una nota de verdad y hay
    // que emparejarla, no clasificarla como dedo que resbala.
    let song = cancion(&[(1_000, 64, 300), (1_000, 65, 300)]);
    let mut e = evaluador(&song, Nivel::Permisivo);
    e.observar(ataque(1_000_000, 64, 90));
    e.observar(suelta(1_300_000, 64));
    e.observar(ataque(1_000_000, 65, 90));
    e.observar(suelta(1_300_000, 65));
    let r = e.cerrar(Micros(3_000_000));

    assert_eq!(r.acertadas, 2, "las dos son notas de la pieza");
    assert_eq!(r.dedos_escapados, 0);
    assert_eq!(r.de_mas, 0);
}

#[test]
fn tocar_lejos_de_todo_es_una_nota_de_mas_y_no_un_dedo_escapado() {
    let song = cancion(&[(1_000, 64, 300)]);
    let mut e = evaluador(&song, Nivel::Permisivo);
    e.observar(ataque(1_000_000, 64, 90));
    e.observar(suelta(1_300_000, 64));
    e.observar(ataque(1_010_000, 71, 90)); // a siete semitonos: no es un dedo que resbala
    e.observar(suelta(1_100_000, 71));
    let r = e.cerrar(Micros(3_000_000));
    assert_eq!(r.de_mas, 1);
    assert_eq!(r.dedos_escapados, 0);
}

// ---------------------------------------------------------------- T032a, T032b

#[test]
fn un_pasaje_saltado_no_cuenta_como_fallado() {
    // T032a (FR-013).
    let song = cancion(&[(0, 60, 300), (1_000, 62, 300), (2_000, 64, 300), (3_000, 65, 300)]);
    let mut e = evaluador(&song, Nivel::Permisivo);
    e.observar(ataque(0, 60, 90));
    e.observar(suelta(300_000, 60));
    e.saltar(Micros(900_000), Micros(2_500_000)); // se salta la 62 y la 64
    e.observar(ataque(3_000_000, 65, 90));
    e.observar(suelta(3_300_000, 65));
    let r = e.cerrar(Micros(4_000_000));

    assert_eq!(r.no_intentadas, 2);
    assert_eq!(r.omitidas, 0, "saltar no es fallar");
    assert_eq!(r.acertadas, 2);
}

#[test]
fn el_porcentaje_se_calcula_sobre_lo_que_se_intento() {
    // T032b (SC-009). Con 4 notas de las que 2 se saltaron, acertar las otras 2 es el 100 %,
    // no el 50 %. Meter lo no intentado en el denominador convertiría en fallo del alumno
    // algo que ni siquiera le pidieron.
    let song = cancion(&[(0, 60, 300), (1_000, 62, 300), (2_000, 64, 300), (3_000, 65, 300)]);
    let mut e = evaluador(&song, Nivel::Permisivo);
    e.observar(ataque(0, 60, 90));
    e.observar(suelta(300_000, 60));
    e.saltar(Micros(900_000), Micros(2_500_000));
    e.observar(ataque(3_000_000, 65, 90));
    e.observar(suelta(3_300_000, 65));
    let r = e.cerrar(Micros(4_000_000));

    assert_eq!(r.fraccion_de_aciertos(), Some((2, 2)), "dos de dos, no dos de cuatro");
}

// ---------------------------------------------------------------- T037-T041

/// Una escala de `n` notas, una cada 500 ms.
fn escala(n: u64) -> Song {
    let notas: Vec<(u64, u8, u64)> = (0..n).map(|i| (i * 500, 60 + (i % 12) as u8, 300)).collect();
    cancion(&notas)
}

/// La toca entera con un desplazamiento fijo.
fn tocar_desplazada(e: &mut Evaluador, song: &Song, desplazamiento: i64) {
    for n in song.notes() {
        let t = n.onset_us.0.saturating_add_signed(desplazamiento);
        e.observar(ataque(t, n.key, 90));
        e.observar(suelta(t + 200_000, n.key));
    }
}

#[test]
fn un_retraso_uniforme_dentro_de_tolerancia_acierta_todo_y_avisa() {
    // SC-003 (T037). Veinte notas 40 ms tarde: las veinte son acierto en el nivel permisivo,
    // **y** se comunica que va sistemáticamente atrasado.
    let song = escala(20);
    let mut e = evaluador(&song, Nivel::Permisivo);
    tocar_desplazada(&mut e, &song, 40_000);
    let r = e.cerrar(Micros(20_000_000));
    assert_eq!(r.acertadas, 20);
    let d = r.desfase.expect("hay desfase sistemático");
    assert_eq!(d.mediana_us, 40_000, "y dice cuánto y hacia dónde");
    assert_eq!(d.dispersion_us, 0, "todas igual de desviadas");
}

#[test]
fn un_retraso_uniforme_fuera_de_tolerancia_avisa_en_vez_de_dar_veinte_fallos() {
    // SC-004 (T038). Es la diferencia entre decirle «vas tarde» y decirle «fallaste veinte
    // veces». Lo primero se corrige; lo segundo desanima.
    let song = escala(20);
    let mut e = evaluador(&song, Nivel::Exigente);
    tocar_desplazada(&mut e, &song, 120_000);
    let r = e.cerrar(Micros(20_000_000));
    assert_eq!(r.acertadas, 0, "fuera de la ventana exigente");
    assert_eq!(r.fuera_de_tiempo, 20, "pero emparejadas, no omitidas");
    assert_eq!(r.omitidas, 0, "y desde luego no veinte fallos independientes");
    assert_eq!(r.desfase.expect("hay desfase").mediana_us, 120_000);
}

#[test]
fn con_pocas_notas_no_se_afirma_que_haya_desfase_sistematico() {
    // T039. Con dos notas la mediana existe y no significa nada.
    let song = escala(3);
    let mut e = evaluador(&song, Nivel::Permisivo);
    tocar_desplazada(&mut e, &song, 40_000);
    let r = e.cerrar(Micros(5_000_000));
    assert_eq!(r.acertadas, 3);
    assert!(r.desfase.is_none(), "tres notas no bastan para hablar de sistemático");
}

#[test]
fn tocar_irregular_no_es_ir_sistematicamente_tarde() {
    // La dispersión es lo que los separa: si la mitad central de los desfases está muy
    // repartida, el alumno va irregular, no tarde.
    let song = escala(20);
    let mut e = evaluador(&song, Nivel::Permisivo);
    for (i, n) in song.notes().iter().enumerate() {
        // Alterna muy pronto y muy tarde: mediana cerca de cero, dispersión enorme.
        let d: i64 = if i % 2 == 0 { -100_000 } else { 100_000 };
        let t = n.onset_us.0.saturating_add_signed(d);
        e.observar(ataque(t, n.key, 90));
        e.observar(suelta(t + 200_000, n.key));
    }
    let r = e.cerrar(Micros(20_000_000));
    assert!(r.desfase.is_none(), "irregular no es sistemático");
}

#[test]
fn la_velocidad_no_regala_tolerancia() {
    // SC-012 (T041). La tolerancia es absoluta en milisegundos y no escala con el tempo.
    // Si a mitad de velocidad salieran más aciertos, FR-008a estaría roto.
    //
    // Se compara la misma pieza a tempo normal contra la misma pieza con las notas al doble
    // de separación —que es lo que el alumno oye al practicar a la mitad—, con el mismo
    // desfase absoluto en los dos casos.
    let normal = escala(20);
    let lenta = {
        let notas: Vec<(u64, u8, u64)> =
            (0..20u64).map(|i| (i * 1_000, 60 + (i % 12) as u8, 600)).collect();
        cancion(&notas)
    };
    let aciertos = |song: &Song| {
        let mut e = evaluador(song, Nivel::Intermedio);
        tocar_desplazada(&mut e, song, 80_000); // 80 ms tarde en los dos casos
        e.cerrar(Micros(40_000_000)).acertadas
    };
    assert_eq!(
        aciertos(&normal),
        aciertos(&lenta),
        "bajar la velocidad da tiempo para acertar, no tolerancia regalada"
    );
    assert_eq!(aciertos(&normal), 0, "80 ms está fuera de la ventana intermedia de 60");
}

// ---------------------------------------------------------------- T041a-c: determinismo

#[test]
fn cien_evaluaciones_de_lo_mismo_dan_lo_mismo() {
    // SC-005 (T041a). El Principio I lo exige como MUST.
    let song = escala(20);
    let referencia = {
        let mut e = evaluador(&song, Nivel::Intermedio);
        tocar_desplazada(&mut e, &song, 25_000);
        e.cerrar(Micros(20_000_000))
    };
    for i in 0..100 {
        let mut e = evaluador(&song, Nivel::Intermedio);
        tocar_desplazada(&mut e, &song, 25_000);
        assert_eq!(e.cerrar(Micros(20_000_000)), referencia, "ejecución {i}");
    }
}

#[test]
fn el_orden_de_las_simultaneas_no_altera_el_resultado() {
    // SC-008 (T041b). Tres notas en el mismo instante, entregadas de las seis formas
    // posibles: los seis resultados tienen que coincidir.
    let song = cancion(&[(1_000, 60, 300), (1_000, 64, 300), (1_000, 67, 300)]);
    let ordenes: [[u8; 3]; 6] = [
        [60, 64, 67], [60, 67, 64], [64, 60, 67],
        [64, 67, 60], [67, 60, 64], [67, 64, 60],
    ];
    let mut resultados = Vec::new();
    for orden in ordenes {
        let mut e = evaluador(&song, Nivel::Intermedio);
        for k in orden {
            e.observar(ataque(1_010_000, k, 90));
        }
        for k in orden {
            e.observar(suelta(1_300_000, k));
        }
        resultados.push(e.cerrar(Micros(3_000_000)));
    }
    for (i, r) in resultados.iter().enumerate() {
        assert_eq!(r, &resultados[0], "el orden {i} dio otra cosa");
    }
}

#[test]
fn el_resultado_no_depende_del_perfil_de_compilacion() {
    // T041c (FR-003, FR-021). Los valores que pasan por `u128` y `try_from` son los que
    // podrían diferir entre debug y release si alguien metiera un `as`. Se fijan aquí con
    // cifras escritas a mano: si el número cambia, cambia en la prueba y no en producción,
    // que es donde nadie lo vería.
    let song = escala(20);
    let mut e = evaluador(&song, Nivel::Permisivo);
    tocar_desplazada(&mut e, &song, -37_000);
    let r = e.cerrar(Micros(20_000_000));
    assert_eq!(r.acertadas, 20);
    assert_eq!(r.desfase.expect("hay desfase").mediana_us, -37_000, "adelantado, con signo");
}

// ---------------------------------------------------------------- T031d, T031e

#[test]
fn en_modo_espera_se_evaluan_las_notas_pero_no_los_tiempos() {
    // T031d (FR-009a). No se puede llegar tarde a algo que te espera: publicar un desfase
    // en modo espera sería inventarlo.
    let song = escala(20);

    let por_reloj = {
        let mut e = evaluador(&song, Nivel::Exigente);
        tocar_desplazada(&mut e, &song, 120_000); // muy tarde
        e.cerrar(Micros(20_000_000))
    };
    let esperando = {
        let mut e = evaluador(&song, Nivel::Exigente);
        e.evaluar_tiempos(false);
        tocar_desplazada(&mut e, &song, 120_000);
        e.cerrar(Micros(20_000_000))
    };

    // Las notas: las mismas se emparejan en los dos casos.
    assert_eq!(
        esperando.acertadas + esperando.fuera_de_tiempo,
        por_reloj.acertadas + por_reloj.fuera_de_tiempo,
        "se emparejan las mismas"
    );
    assert_eq!(esperando.omitidas, por_reloj.omitidas);
    // Los tiempos: en modo espera no se juzgan ni se resumen.
    assert_eq!(esperando.acertadas, 20, "en espera, llegar tarde no es un fallo de tiempo");
    assert_eq!(esperando.fuera_de_tiempo, 0);
    assert!(esperando.desfase.is_none(), "no se publica un desfase inventado");
    assert!(esperando.parcial, "y se DECLARA que el resultado es parcial");
    assert!(!por_reloj.parcial, "por reloj el resultado es completo");
}

#[test]
fn cambiar_de_modo_a_mitad_evalua_cada_nota_segun_su_regimen() {
    // T031e. Cada nota se juzga según el régimen vigente **cuando se emparejó**, no según
    // un único indicador del intento entero. Es lo que FR-004 obliga: una nota ya juzgada
    // no se recalcula.
    let song = escala(20);
    let mut e = evaluador(&song, Nivel::Exigente);
    e.evaluar_tiempos(false); // empieza en modo espera
    for (i, n) in song.notes().iter().enumerate() {
        if i == 10 {
            e.evaluar_tiempos(true); // a mitad pasa a tempo
        }
        let t = n.onset_us.0 + 120_000; // siempre 120 ms tarde
        e.observar(ataque(t, n.key, 90));
        e.observar(suelta(t + 100_000, n.key));
        e.avanzar(Micros(t + 150_000));
    }
    let r = e.cerrar(Micros(20_000_000));

    assert_eq!(r.acertadas, 10, "las diez de modo espera: el tiempo no se juzgó");
    assert_eq!(r.fuera_de_tiempo, 10, "las diez de tempo: 120 ms es fuera de la ventana");
    assert!(r.parcial, "hubo notas sin tiempo evaluado, así que el resultado es parcial");
}

// ---------------------------------------------------------------- T048, T049

#[test]
fn cada_veredicto_queda_situado_en_su_nota() {
    // T048 (FR-017). Un «80 %» no cambia lo que el alumno hace mañana; saber que los fallos
    // están en la segunda mitad, sí.
    let song = escala(20);
    let mut e = evaluador(&song, Nivel::Intermedio);
    // Toca bien las diez primeras y no toca las diez últimas.
    for n in song.notes().iter().take(10) {
        e.observar(ataque(n.onset_us.0, n.key, 90));
        e.observar(suelta(n.onset_us.0 + 200_000, n.key));
    }
    let r = e.cerrar(Micros(20_000_000));

    assert_eq!(r.por_nota.len(), 20, "una entrada por nota");
    let aciertos: Vec<usize> = r
        .por_nota
        .iter()
        .filter(|(_, v)| *v == Veredicto::Acertada)
        .map(|(i, _)| *i)
        .collect();
    assert_eq!(aciertos, (0..10).collect::<Vec<_>>(), "los aciertos, en la primera mitad");
    let fallos: Vec<usize> = r
        .por_nota
        .iter()
        .filter(|(_, v)| *v == Veredicto::Omitida)
        .map(|(i, _)| *i)
        .collect();
    assert_eq!(fallos, (10..20).collect::<Vec<_>>(), "y los fallos, en la segunda");
}

#[test]
fn el_resultado_separa_una_mano_de_la_otra() {
    // T049 (FR-018). Con la izquierda bien y la derecha mal, decirlo junto sería inútil.
    let song = cancion(&[(0, 40, 300), (0, 72, 300), (500, 43, 300), (500, 74, 300)]);
    let manos = vec![Mano::Izquierda, Mano::Derecha, Mano::Izquierda, Mano::Derecha];
    let mut e = Evaluador::nuevo(&song, &manos, None, Nivel::Intermedio);
    // Toca solo las de la izquierda.
    for (t, k) in [(0u64, 40u8), (500_000, 43)] {
        e.observar(ataque(t, k, 90));
        e.observar(suelta(t + 200_000, k));
    }
    let r = e.cerrar(Micros(3_000_000));

    let izq = r.por_mano[0];
    let der = r.por_mano[1];
    assert_eq!(izq.acertadas, 2, "la izquierda, perfecta");
    assert_eq!(izq.omitidas, 0);
    assert_eq!(der.acertadas, 0, "la derecha, sin tocar");
    assert_eq!(der.omitidas, 2);
}

#[test]
fn con_una_sola_mano_la_otra_no_aparece_como_fallada() {
    // Practicando la izquierda, las notas de la derecha no se le piden: no pueden contar
    // como omitidas.
    let song = cancion(&[(0, 40, 300), (0, 72, 300)]);
    let manos = vec![Mano::Izquierda, Mano::Derecha];
    let mut e = Evaluador::nuevo(&song, &manos, Some(Mano::Izquierda), Nivel::Intermedio);
    e.observar(ataque(0, 40, 90));
    e.observar(suelta(200_000, 40));
    let r = e.cerrar(Micros(2_000_000));

    assert_eq!(r.acertadas, 1);
    assert_eq!(r.omitidas, 0, "la de la derecha no se le pidió");
    assert_eq!(r.intentadas(), 1, "y no está en el denominador");
}

// ---------------------------------------------------------------- T055, T056, T057

#[test]
fn la_misma_nota_acierta_en_el_permisivo_y_no_en_el_exigente() {
    // T055. Un principiante y alguien con años no se miden con la misma vara.
    let song = cancion(&[(1_000, 60, 300)]);
    let veredicto = |nivel| {
        let mut e = evaluador(&song, nivel);
        e.observar(ataque(1_060_000, 60, 90)); // 60 ms tarde
        e.observar(suelta(1_200_000, 60));
        e.cerrar(Micros(3_000_000))
    };
    assert_eq!(veredicto(Nivel::Permisivo).acertadas, 1, "120 ms de ventana: entra");
    assert_eq!(veredicto(Nivel::Intermedio).acertadas, 1, "60 ms justos: entra");
    assert_eq!(veredicto(Nivel::Exigente).acertadas, 0, "30 ms: no entra");
    assert_eq!(veredicto(Nivel::Exigente).fuera_de_tiempo, 1, "pero se emparejó igual");
}

#[test]
fn el_permisivo_nunca_da_menos_aciertos_que_el_exigente() {
    // T056 (SC-006). Con las dos ventanas separadas esto es aritmética: mismo
    // emparejamiento y ventanas anidadas. Pero se comprueba por si alguien las vuelve a
    // juntar, que es justo el cambio que rompería la propiedad sin romper nada más.
    //
    // Cincuenta interpretaciones deterministas, con desfases de −200 a +200 ms.
    let song = escala(20);
    for caso in 0..50u64 {
        let desplazamiento = (caso as i64 - 25) * 8_000;
        let aciertos = |nivel| {
            let mut e = evaluador(&song, nivel);
            tocar_desplazada(&mut e, &song, desplazamiento);
            e.cerrar(Micros(20_000_000)).acertadas
        };
        let (p, i, x) = (
            aciertos(Nivel::Permisivo),
            aciertos(Nivel::Intermedio),
            aciertos(Nivel::Exigente),
        );
        assert!(p >= i, "caso {caso} ({desplazamiento} µs): permisivo {p} < intermedio {i}");
        assert!(i >= x, "caso {caso} ({desplazamiento} µs): intermedio {i} < exigente {x}");
    }
}

#[test]
fn cambiar_de_nivel_no_cambia_que_se_empareja_con_que() {
    // T057. **La razón de ser de las dos ventanas.** Si el emparejamiento dependiera del
    // nivel, una nota podría quedar acertada en el exigente y SIN PAREJA en el permisivo,
    // y entonces SC-006 dejaría de cumplirse sin que ninguna otra prueba lo notase.
    let song = escala(20);
    let emparejadas = |nivel| {
        let mut e = evaluador(&song, nivel);
        tocar_desplazada(&mut e, &song, 95_000); // entre la ventana exigente y la permisiva
        e.avanzar(Micros(20_000_000)); // el emparejamiento ocurre al avanzar
        // Qué notas recibieron pulsación, con independencia del veredicto.
        let con_medida: Vec<usize> =
            (0..song.notes().len()).filter(|i| e.medida_de(*i).is_some()).collect();
        let _ = e.cerrar(Micros(20_000_000));
        con_medida
    };
    let p = emparejadas(Nivel::Permisivo);
    let x = emparejadas(Nivel::Exigente);
    assert_eq!(p, x, "el emparejamiento no puede depender del nivel");
    assert_eq!(p.len(), 20, "y las veinte se emparejaron en los dos");
}

// ---------------------------------------------------------------- T062, T063, T064

use core::cmp::Ordering;
use piano_core::evaluacion::comparar;

/// Una interpretación de la escala con `fallos` notas sin tocar y el resto desplazadas.
fn intento(song: &Song, fallos: usize, desplazamiento: i64) -> piano_core::evaluacion::Resultado {
    let mut e = evaluador(song, Nivel::Permisivo);
    for n in song.notes().iter().skip(fallos) {
        let t = n.onset_us.0.saturating_add_signed(desplazamiento);
        e.observar(ataque(t, n.key, 90));
        e.observar(suelta(t + 200_000, n.key));
    }
    e.cerrar(Micros(30_000_000))
}

#[test]
fn de_dos_intentos_se_señala_el_mejor() {
    // T062 (SC-010).
    let song = escala(20);
    let malo = intento(&song, 10, 0);
    let bueno = intento(&song, 5, 0);
    assert_eq!(comparar(&bueno, &malo), Ordering::Greater);
    assert_eq!(comparar(&malo, &bueno), Ordering::Less);
    assert_eq!(comparar(&bueno, &bueno), Ordering::Equal);
}

#[test]
fn el_orden_es_total_y_transitivo() {
    // T063 (FR-020a). «No se puede saber» no es una respuesta admisible: es justo cuando el
    // alumno más quiere saberlo.
    let song = escala(20);
    let intentos: Vec<_> = (0..10)
        .map(|i| intento(&song, i, (i as i64 - 5) * 20_000))
        .collect();

    // Total: cualquier par se ordena, y de forma antisimétrica.
    for a in &intentos {
        for b in &intentos {
            let ab = comparar(a, b);
            let ba = comparar(b, a);
            assert_eq!(ab, ba.reverse(), "antisimetría");
        }
    }
    // Transitivo.
    for a in &intentos {
        for b in &intentos {
            for c in &intentos {
                if comparar(a, b) == Ordering::Greater && comparar(b, c) == Ordering::Greater {
                    assert_eq!(comparar(a, c), Ordering::Greater, "transitividad");
                }
            }
        }
    }
}

#[test]
fn el_orden_es_lexico_y_no_una_puntuacion_con_pesos() {
    // T064. Una interpretación con **un acierto más** y un ritmo mucho peor gana igual. Si
    // se pudieran compensar, no sería léxico, y unos pesos arbitrarios reordenarían en
    // silencio interpretaciones ya juzgadas cada vez que se ajustasen.
    let song = escala(20);
    let mas_aciertos_peor_ritmo = intento(&song, 4, 110_000); // 16 aciertos, muy desplazado
    let menos_aciertos_ritmo_perfecto = intento(&song, 5, 0); // 15 aciertos, clavado

    assert_eq!(mas_aciertos_peor_ritmo.acertadas, 16);
    assert_eq!(menos_aciertos_ritmo_perfecto.acertadas, 15);
    assert_eq!(
        comparar(&mas_aciertos_peor_ritmo, &menos_aciertos_ritmo_perfecto),
        Ordering::Greater,
        "manda el número de notas; el ritmo solo desempata"
    );
}

#[test]
fn con_los_mismos_aciertos_decide_el_ritmo() {
    let song = escala(20);
    let clavado = intento(&song, 5, 0);
    let desplazado = intento(&song, 5, 100_000);
    assert_eq!(clavado.acertadas, desplazado.acertadas, "empatan en notas");
    assert_eq!(comparar(&clavado, &desplazado), Ordering::Greater, "y decide el ritmo");
}

#[test]
fn no_tener_desfase_sistematico_es_mejor_que_tenerlo() {
    // Ir a tempo es lo mejor que puede pasarte, así que la ausencia de desfase gana.
    let song = escala(20);
    let a_tempo = intento(&song, 0, 0);
    let tarde = intento(&song, 0, 100_000);
    assert!(a_tempo.desfase.is_none());
    assert!(tarde.desfase.is_some());
    assert_eq!(comparar(&a_tempo, &tarde), Ordering::Greater);
}
