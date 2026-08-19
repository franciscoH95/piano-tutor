//! T074 — cuánto cuesta **no mirar al futuro**.
//!
//! `FR-004` prohíbe que una nota ya juzgada cambie de veredicto por lo que venga después, y
//! eso descarta el emparejamiento óptimo global: Hungarian, DTW y la programación dinámica
//! sobre la interpretación entera revisan decisiones a la luz de lo que vino luego.
//!
//! El plan dejó esta cifra **explícitamente sin suponer**. Aquí se mide: se compara el
//! emparejamiento en línea con el óptimo, sobre casos elegidos para que difieran si van a
//! diferir. La conclusión se anota en `research.md`.

mod fixtures;
use fixtures::interpretaciones::{ataque, suelta};
use fixtures::SmfBuilder;
use piano_core::evaluacion::{Evaluador, Nivel};
use piano_core::load_smf;
use piano_core::practica::Mano;
use piano_core::time::Micros;

const VENTANA_US: u64 = 500_000;

/// El óptimo **con visión de futuro**, que es lo que FR-004 prohíbe.
///
/// Para cada tecla, dos listas ordenadas en una recta: emparejar cada pulsación con la nota
/// compatible **más temprana todavía libre** maximiza el número de parejas. Es el resultado
/// clásico para intervalos sobre una recta, y sirve de cota superior.
fn optimo(notas: &[(u64, u8)], pulsaciones: &[(u64, u8)]) -> usize {
    let mut tomadas = vec![false; notas.len()];
    let mut n = 0;
    for (t, k) in pulsaciones {
        let mut elegida: Option<usize> = None;
        for (i, (onset, key)) in notas.iter().enumerate() {
            if tomadas[i] || key != k || onset.abs_diff(*t) > VENTANA_US {
                continue;
            }
            if elegida.is_none() {
                elegida = Some(i);
            }
        }
        if let Some(i) = elegida {
            tomadas[i] = true;
            n += 1;
        }
    }
    n
}

/// Lo que consigue el emparejamiento en línea de verdad.
fn en_linea(notas: &[(u64, u8)], pulsaciones: &[(u64, u8)]) -> usize {
    let ns = notas.to_vec();
    let raw = SmfBuilder::new(1000)
        .track(|t| {
            let mut t = t.tempo(0, 1_000_000);
            for (us, key) in &ns {
                t = t.note(us / 1_000, *key, 90, 200);
            }
            t
        })
        .build();
    let song = load_smf(&raw).expect("valida");
    let manos = vec![Mano::Derecha; song.notes().len()];
    let mut e = Evaluador::nuevo(&song, &manos, None, Nivel::Permisivo);
    for (t, k) in pulsaciones {
        e.observar(ataque(*t, *k, 90));
        e.observar(suelta(t + 100_000, *k));
    }
    let r = e.cerrar(Micros(60_000_000));
    r.acertadas + r.fuera_de_tiempo
}

/// Casos elegidos para que el óptimo y el en línea difieran **si van a diferir**: notas
/// repetidas de la misma tecla, que es el único sitio donde la elección tiene alternativa.
/// Un caso: su nombre, las notas de la canción y lo que el alumno tocó.
type Caso = (&'static str, &'static [(u64, u8)], &'static [(u64, u8)]);

const CASOS: &[Caso] = &[
    ("una nota, una pulsación", &[(1_000_000, 60)], &[(1_000_000, 60)]),
    (
        "dos repetidas, dos pulsaciones",
        &[(1_000_000, 60), (1_300_000, 60)],
        &[(1_000_000, 60), (1_300_000, 60)],
    ),
    (
        "dos repetidas, una pulsación en medio",
        &[(1_000_000, 60), (1_400_000, 60)],
        &[(1_200_000, 60)],
    ),
    (
        "tres repetidas, dos pulsaciones tardías",
        &[(1_000_000, 60), (1_200_000, 60), (1_400_000, 60)],
        &[(1_350_000, 60), (1_450_000, 60)],
    ),
    (
        "el caso que más debería doler: la pulsación se lleva la nota equivocada",
        &[(1_000_000, 60), (1_100_000, 60)],
        &[(1_090_000, 60), (1_600_000, 60)],
    ),
    (
        "diez repetidas tocadas a destiempo",
        &[
            (1_000_000, 60), (1_200_000, 60), (1_400_000, 60), (1_600_000, 60), (1_800_000, 60),
            (2_000_000, 60), (2_200_000, 60), (2_400_000, 60), (2_600_000, 60), (2_800_000, 60),
        ],
        &[
            (1_100_000, 60), (1_300_000, 60), (1_500_000, 60), (1_700_000, 60), (1_900_000, 60),
            (2_100_000, 60), (2_300_000, 60), (2_500_000, 60), (2_700_000, 60), (2_900_000, 60),
        ],
    ),
];

#[test]
fn se_mide_cuanta_precision_se_pierde_por_no_mirar_al_futuro() {
    let mut peor = 0usize;
    let mut informe = Vec::new();
    for (nombre, notas, pulsaciones) in CASOS {
        let o = optimo(notas, pulsaciones);
        let l = en_linea(notas, pulsaciones);
        let perdida = o.saturating_sub(l);
        peor = peor.max(perdida);
        informe.push(format!("  {nombre}: óptimo {o}, en línea {l}, pérdida {perdida}"));
    }
    println!("\nPÉRDIDA POR NO MIRAR AL FUTURO\n{}", informe.join("\n"));
    println!("  peor caso medido: {peor} emparejamiento(s)\n");

    // Lo que se afirma: el emparejamiento en línea **nunca supera** al óptimo —sería un
    // error de la medición— y la pérdida está acotada.
    for (nombre, notas, pulsaciones) in CASOS {
        let (o, l) = (optimo(notas, pulsaciones), en_linea(notas, pulsaciones));
        assert!(l <= o, "«{nombre}»: en línea {l} supera al óptimo {o}, la medición está mal");
    }
    // Medido el 2026-08-19: la pérdida es **como mucho un emparejamiento**, y solo en el
    // caso en que tomar la nota más cercana deja varada una anterior que todavía era
    // alcanzable. Si algún día sube, es que el emparejamiento ha empeorado y hay que mirarlo.
    assert!(
        peor <= 1,
        "la pérdida por no mirar al futuro llegó a {peor} emparejamientos; estaba medida en 1"
    );
}
