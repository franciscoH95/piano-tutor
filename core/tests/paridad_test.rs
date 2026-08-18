//! La posición del núcleo y la de la pantalla son **la misma función**.
//!
//! Esta prueba y `src/practica/paridad.test.ts` leen el mismo fichero de vectores. Si una
//! implementación cambia y la otra no, falla una de las dos. Sin esto, cada lado podría
//! estar internamente consistente y aun así pintar el cursor donde el núcleo no cree que
//! está: un desfase silencioso, que no rompe ninguna prueba de un solo lado.
//!
//! Se lee un CSV a mano y no JSON a propósito: el Principio III exige que
//! `cargo tree -p piano-core` dé exactamente tres líneas, así que el núcleo no puede tener
//! ni una dependencia de desarrollo más.

use piano_core::practica::{posicion_en, Ancla};
use piano_core::time::Micros;

const VECTORES: &str = include_str!("../../fixtures/paridad-cursor.csv");

#[test]
fn el_nucleo_cumple_los_vectores_de_paridad() {
    let mut comprobados = 0;
    for (n, linea) in VECTORES.lines().enumerate() {
        let linea = linea.trim();
        if linea.is_empty() || linea.starts_with('#') {
            continue;
        }
        let campos: Vec<&str> = linea.split(',').collect();
        assert_eq!(campos.len(), 7, "línea {} mal formada: {linea}", n + 1);

        let num = |i: usize| -> u64 {
            campos
                .get(i)
                .and_then(|c| c.parse::<u64>().ok())
                .unwrap_or_else(|| panic!("campo {i} de la línea {} no es un número", n + 1))
        };
        let tope = match campos.get(4) {
            Some(&"-") => None,
            _ => Some(Micros(num(4))),
        };
        let ancla = Ancla {
            posicion_us: Micros(num(0)),
            instante_us: Micros(num(1)),
            num: u32::try_from(num(2)).expect("num cabe en u32"),
            den: u32::try_from(num(3)).expect("den cabe en u32"),
            tope_us: tope,
        };
        let obtenido = posicion_en(&ancla, Micros(num(5)));
        assert_eq!(obtenido.0, num(6), "línea {}: {linea}", n + 1);
        comprobados += 1;
    }
    // Que el fichero no se quede vacío por un error de ruta y la prueba pase sola.
    assert!(comprobados >= 15, "solo se comprobaron {comprobados} vectores");
}
