//! T071, T072 — **la tabla entera** de interpretaciones de referencia (FR-022).
//!
//! Existe como tabla y no como pruebas sueltas por la lección de la feature 003: un ajuste
//! de tolerancia que arregla un caso rompe otro, y eso **solo se ve comprobándolos juntos**.
//! Allí una escala se rompió en silencio porque tenía muestrario pero no aserción.
//!
//! Los resultados esperados están **calculados a mano** a partir de la especificación y de
//! los umbrales de `tolerancias.rs`, no volcados de la implementación. Volcarlos copiaría el
//! fallo de quien los generó y la prueba pasaría a confirmar el error en vez de detectarlo.
//!
//! Si un cambio de reglas altera alguno, FR-022 exige **declarar cuál cambia y por qué** en
//! el pull request. No se ajusta el número y se sigue.

mod fixtures;
use fixtures::interpretaciones::{ataque, suelta, Caso, Esperado};
use fixtures::SmfBuilder;
use piano_core::evaluacion::{Evaluador, Nivel};
use piano_core::load_smf;
use piano_core::practica::Mano;
use piano_core::time::Micros;

/// Diez notas, una cada 500 ms, para los casos de desfase sistemático.
///
/// Diez y no menos: el umbral mínimo para hablar de «sistemático» son ocho, y con menos la
/// mediana existe y no describe nada.
/// Empieza en el segundo 1 y no en cero: un adelanto de 50 ms sobre la primera nota se
/// iría por debajo de cero, y el instante de una pulsación no puede ser negativo.
const ESCALA: &[(u64, u8, u64, u8)] = &[
    (1_000, 60, 300, 0),
    (1_500, 62, 300, 0),
    (2_000, 64, 300, 0),
    (2_500, 65, 300, 0),
    (3_000, 67, 300, 0),
    (3_500, 69, 300, 0),
    (4_000, 71, 300, 0),
    (4_500, 72, 300, 0),
    (5_000, 74, 300, 0),
    (5_500, 76, 300, 0),
];

/// La escala tocada con un desplazamiento fijo.
macro_rules! desplazada {
    ($d:expr) => {
        &[
            (1_000_000_u64.wrapping_add_signed($d), 60_u8, 200_000_u64),
            (1_500_000_u64.wrapping_add_signed($d), 62, 200_000),
            (2_000_000_u64.wrapping_add_signed($d), 64, 200_000),
            (2_500_000_u64.wrapping_add_signed($d), 65, 200_000),
            (3_000_000_u64.wrapping_add_signed($d), 67, 200_000),
            (3_500_000_u64.wrapping_add_signed($d), 69, 200_000),
            (4_000_000_u64.wrapping_add_signed($d), 71, 200_000),
            (4_500_000_u64.wrapping_add_signed($d), 72, 200_000),
            (5_000_000_u64.wrapping_add_signed($d), 74, 200_000),
            (5_500_000_u64.wrapping_add_signed($d), 76, 200_000),
        ]
    };
}

const CASOS: &[Caso] = &[
    Caso {
        nombre: "perfecta",
        notas: ESCALA,
        manos: &[],
        practicada: None,
        nivel: Nivel::Intermedio,
        tocado: desplazada!(0),
        esperado: Esperado {
            acertadas: 10,
            intentadas: 10,
            // Desfase cero: por debajo del umbral de 30 ms, así que NO hay desfase
            // sistemático. Publicar «vas 0 ms tarde» sería ruido.
            desfase_us: None,
            ..Esperado::nada()
        },
        porque: "el caso base: si este falla, no hay nada que discutir",
    },
    Caso {
        nombre: "no se tocó nada",
        notas: ESCALA,
        manos: &[],
        practicada: None,
        nivel: Nivel::Intermedio,
        tocado: &[],
        esperado: Esperado {
            omitidas: 10,
            intentadas: 10,
            sin_tocar: true,
            ..Esperado::nada()
        },
        porque: "SC-002: no tocar nada no es tocarlo todo mal, y se comunica distinto",
    },
    Caso {
        nombre: "retraso uniforme dentro de tolerancia",
        notas: ESCALA,
        manos: &[],
        practicada: None,
        // Permisivo: ventana de 120 ms, así que 40 ms entra.
        nivel: Nivel::Permisivo,
        tocado: desplazada!(40_000),
        esperado: Esperado {
            acertadas: 10,
            intentadas: 10,
            // 40 ms ≥ el umbral de 30, y dispersión 0 ≤ 40: sí hay desfase sistemático.
            desfase_us: Some(40_000),
            ..Esperado::nada()
        },
        porque: "SC-003: acierta todo Y se le avisa de que va tarde. Las dos cosas",
    },
    Caso {
        nombre: "retraso uniforme fuera de tolerancia",
        notas: ESCALA,
        manos: &[],
        practicada: None,
        // Exigente: ventana de 30 ms, así que 100 ms se sale.
        nivel: Nivel::Exigente,
        tocado: desplazada!(100_000),
        esperado: Esperado {
            fuera_de_tiempo: 10,
            intentadas: 10,
            desfase_us: Some(100_000),
            ..Esperado::nada()
        },
        porque: "SC-004: se avisa del desfase EN VEZ de dar diez fallos sueltos. Y no son \
                 omitidas: se emparejaron, solo que tarde",
    },
    Caso {
        nombre: "adelanto uniforme",
        notas: ESCALA,
        manos: &[],
        practicada: None,
        nivel: Nivel::Intermedio,
        tocado: desplazada!(-50_000),
        esperado: Esperado {
            acertadas: 10,
            intentadas: 10,
            // El signo ES la información: sin él no se distingue de ir tarde.
            desfase_us: Some(-50_000),
            ..Esperado::nada()
        },
        porque: "50 ms cabe en la ventana intermedia de 60, y el desfase sale NEGATIVO",
    },
    Caso {
        nombre: "irregular",
        notas: ESCALA,
        manos: &[],
        practicada: None,
        nivel: Nivel::Permisivo,
        // Alterna 100 ms tarde y 100 ms pronto.
        tocado: &[
            (1_100_000, 60, 200_000),
            (1_400_000, 62, 200_000),
            (2_100_000, 64, 200_000),
            (2_400_000, 65, 200_000),
            (3_100_000, 67, 200_000),
            (3_400_000, 69, 200_000),
            (4_100_000, 71, 200_000),
            (4_400_000, 72, 200_000),
            (5_100_000, 74, 200_000),
            (5_400_000, 76, 200_000),
        ],
        esperado: Esperado {
            acertadas: 10,
            intentadas: 10,
            // Desfases alternos de ±100 ms: la mediana queda en el medio y la dispersión
            // se dispara. Ir irregular NO es ir sistemáticamente tarde.
            desfase_us: None,
            ..Esperado::nada()
        },
        porque: "la dispersión es lo que separa «va tarde» de «va irregular»",
    },
    Caso {
        nombre: "notas de más",
        notas: &[(0, 60, 300, 0), (1_000, 62, 300, 0), (2_000, 64, 300, 0)],
        manos: &[],
        practicada: None,
        nivel: Nivel::Permisivo,
        tocado: &[
            (0, 60, 200_000),
            (1_000_000, 62, 200_000),
            (2_000_000, 64, 200_000),
            (3_000_000, 80, 200_000),
            (3_500_000, 80, 200_000),
        ],
        esperado: Esperado {
            acertadas: 3,
            de_mas: 2,
            intentadas: 3,
            ..Esperado::nada()
        },
        porque: "tocar de más no reduce los aciertos ni entra en el denominador",
    },
    Caso {
        nombre: "dedo que se escapa",
        notas: &[(1_000, 64, 300, 0)],
        manos: &[],
        practicada: None,
        nivel: Nivel::Permisivo,
        tocado: &[(990_000, 65, 10_000), (1_000_000, 64, 300_000)],
        esperado: Esperado {
            acertadas: 1,
            dedos_escapados: 1,
            intentadas: 1,
            ..Esperado::nada()
        },
        porque: "el error más frecuente de un principiante: roza el Fa y toca el Mi. Se \
                 cuenta aparte, no como una nota de más cualquiera, y el acierto sigue \
                 contando",
    },
    Caso {
        nombre: "acorde a medias",
        notas: &[(1_000, 60, 300, 0), (1_000, 64, 300, 0), (1_000, 67, 300, 0)],
        manos: &[],
        practicada: None,
        nivel: Nivel::Permisivo,
        tocado: &[(1_000_000, 60, 200_000), (1_000_000, 67, 200_000)],
        esperado: Esperado {
            acertadas: 2,
            omitidas: 1,
            intentadas: 3,
            ..Esperado::nada()
        },
        porque: "de un acorde a medias se omite solo lo que faltó, no el acorde entero",
    },
    Caso {
        nombre: "nota repetida tocada una vez",
        notas: &[(0, 60, 200, 0), (300, 60, 200, 0)],
        manos: &[],
        practicada: None,
        nivel: Nivel::Permisivo,
        tocado: &[(0, 60, 150_000)],
        esperado: Esperado {
            acertadas: 1,
            omitidas: 1,
            intentadas: 2,
            ..Esperado::nada()
        },
        porque: "FR-002: una pulsación no puede cubrir dos notas. La más cercana se lleva \
                 la pulsación y la otra queda omitida",
    },
    Caso {
        nombre: "notas que el teclado no tiene",
        notas: &[(0, 60, 300, 0), (1_000, 62, 300, 0), (2_000, 109, 300, 0)],
        manos: &[],
        practicada: None,
        nivel: Nivel::Permisivo,
        tocado: &[(0, 60, 200_000), (1_000_000, 62, 200_000)],
        esperado: Esperado {
            acertadas: 2,
            fuera_de_alcance: 1,
            intentadas: 2,
            ..Esperado::nada()
        },
        porque: "FR-014 y SC-009: lo que no puede tocar queda FUERA del denominador. Dos \
                 de dos, no dos de tres",
    },
    Caso {
        nombre: "la percusión no se le pide",
        notas: &[(0, 60, 300, 0), (0, 38, 100, 9), (1_000, 62, 300, 0)],
        manos: &[],
        practicada: None,
        nivel: Nivel::Permisivo,
        tocado: &[(0, 60, 200_000), (1_000_000, 62, 200_000)],
        esperado: Esperado {
            acertadas: 2,
            intentadas: 2,
            ..Esperado::nada()
        },
        porque: "la batería no se toca con las manos en el teclado. Ni cuenta como omitida \
                 ni como fuera de alcance: no está",
    },
];

/// Construye la canción de un caso.
fn cancion_de(caso: &Caso) -> piano_core::Song {
    let notas = caso.notas.to_vec();
    let raw = SmfBuilder::new(1000)
        .track(|t| {
            let mut t = t.tempo(0, 1_000_000);
            for (tick, key, dur, canal) in &notas {
                if *canal == 0 {
                    t = t.note(*tick, *key, 90, *dur);
                } else {
                    let estado = 0x90 | canal;
                    t = t
                        .raw(*tick, &[estado, *key, 90])
                        .raw(tick + dur, &[0x80 | canal, *key, 0]);
                }
            }
            t
        })
        .build();
    load_smf(&raw).expect("el fixture debe cargar")
}

#[test]
fn la_tabla_entera_de_interpretaciones_de_referencia() {
    let mut fallos = Vec::new();
    for caso in CASOS {
        let song = cancion_de(caso);
        let manos: Vec<Mano> = if caso.manos.is_empty() {
            vec![Mano::Derecha; song.notes().len()]
        } else {
            caso.manos.to_vec()
        };
        let mut e = Evaluador::nuevo(&song, &manos, caso.practicada, caso.nivel);
        for (t, k, dur) in caso.tocado {
            e.observar(ataque(*t, *k, 90));
            e.observar(suelta(t + dur, *k));
        }
        let r = e.cerrar(Micros(60_000_000));
        let x = &caso.esperado;

        let mut comprobar = |campo: &str, obtenido: String, esperado: String| {
            if obtenido != esperado {
                fallos.push(format!(
                    "  «{}» · {campo}: se obtuvo {obtenido} y se esperaba {esperado}\n     ({})",
                    caso.nombre, caso.porque
                ));
            }
        };
        comprobar("acertadas", r.acertadas.to_string(), x.acertadas.to_string());
        comprobar("fuera_de_tiempo", r.fuera_de_tiempo.to_string(), x.fuera_de_tiempo.to_string());
        comprobar("omitidas", r.omitidas.to_string(), x.omitidas.to_string());
        comprobar("de_mas", r.de_mas.to_string(), x.de_mas.to_string());
        comprobar("dedos_escapados", r.dedos_escapados.to_string(), x.dedos_escapados.to_string());
        comprobar("fuera_de_alcance", r.fuera_de_alcance.to_string(), x.fuera_de_alcance.to_string());
        comprobar("no_intentadas", r.no_intentadas.to_string(), x.no_intentadas.to_string());
        comprobar("intentadas", r.intentadas().to_string(), x.intentadas.to_string());
        comprobar("sin_tocar", r.sin_tocar.to_string(), x.sin_tocar.to_string());
        comprobar("parcial", r.parcial.to_string(), x.parcial.to_string());
        comprobar(
            "desfase",
            format!("{:?}", r.desfase.map(|d| d.mediana_us)),
            format!("{:?}", x.desfase_us),
        );
    }
    assert!(
        fallos.is_empty(),
        "{} discrepancias con las interpretaciones de referencia.\n\
         FR-022 exige DECLARAR cuáles cambian y por qué, no ajustar el número y seguir:\n{}",
        fallos.len(),
        fallos.join("\n")
    );
}

#[test]
fn hay_suficientes_interpretaciones_de_referencia() {
    // T071 pide al menos diez. Menos no cubre las clases de comportamiento distintas, y la
    // tabla vale justo por cubrirlas todas a la vez.
    assert!(CASOS.len() >= 10, "solo hay {} casos", CASOS.len());
}

#[test]
fn cada_caso_dice_por_que_existe() {
    // Un caso que no puede explicarse no aporta nada, y con el tiempo nadie se atreve a
    // tocarlo porque nadie sabe qué protege.
    for c in CASOS {
        assert!(!c.porque.is_empty(), "«{}» no dice por qué existe", c.nombre);
    }
}
