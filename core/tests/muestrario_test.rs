//! Muestrario para revisión humana. No afirma nada: imprime para que un músico juzgue.
mod fixtures;
use fixtures::SmfBuilder;
use piano_core::digitacion::digitar;
use piano_core::load_smf;
use piano_core::practica::Mano;

fn digitar_escala(keys: &[u8], mano: Mano) -> String {
    let mut b = SmfBuilder::new(480);
    b = b.track(|t| {
        let mut t = t;
        for (i, k) in keys.iter().enumerate() {
            t = t.note(i as u64 * 480, *k, 90, 240);
        }
        t
    });
    let song = load_smf(&b.build()).expect("valida");
    let d = digitar(&song, &vec![mano; keys.len()]);
    (0..keys.len())
        .map(|i| d.dedo(i).numero().to_string())
        .collect::<Vec<_>>()
        .join("-")
}

#[test]
fn muestrario() {
    let escalas: [(&str, [u8; 8]); 6] = [
        ("Do  mayor  (sin alteraciones)", [60, 62, 64, 65, 67, 69, 71, 72]),
        ("Sol mayor  (fa#)            ", [67, 69, 71, 72, 74, 76, 78, 79]),
        ("Fa  mayor  (sib)            ", [65, 67, 69, 70, 72, 74, 76, 77]),
        ("Re  mayor  (fa# do#)        ", [62, 64, 66, 67, 69, 71, 73, 74]),
        ("Si  mayor  (5 sostenidos)   ", [71, 73, 75, 76, 78, 80, 82, 83]),
        ("Reb mayor  (5 bemoles)      ", [61, 63, 65, 66, 68, 70, 72, 73]),
    ];
    println!("\n  ESCALA                          DERECHA           IZQUIERDA");
    println!("  ─────────────────────────────────────────────────────────────");
    for (nombre, keys) in escalas {
        println!(
            "  {}    {}   {}",
            nombre,
            digitar_escala(&keys, Mano::Derecha),
            digitar_escala(&keys, Mano::Izquierda)
        );
    }
    println!("\n  ACORDES (mano derecha)");
    println!("  ─────────────────────────────────────────────────────────────");
    for (nombre, keys) in [
        ("Do mayor           60-64-67", vec![60u8, 64, 67]),
        ("Do mayor 1a inv    64-67-72", vec![64u8, 67, 72]),
        ("Do septima         60-64-67-70", vec![60u8, 64, 67, 70]),
        ("Fa# mayor (negras) 66-70-73", vec![66u8, 70, 73]),
    ] {
        let mut b = SmfBuilder::new(480);
        let ks = keys.clone();
        b = b.track(|t| {
            let mut t = t;
            for k in &ks {
                t = t.note(0, *k, 90, 480);
            }
            t
        });
        let song = load_smf(&b.build()).expect("valida");
        let d = digitar(&song, &vec![Mano::Derecha; keys.len()]);
        let dedos: Vec<String> = (0..keys.len()).map(|i| d.dedo(i).numero().to_string()).collect();
        println!("  {}    {}", nombre, dedos.join("-"));
    }
    println!();
}
