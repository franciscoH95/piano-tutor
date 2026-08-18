//! M2 — que cada fallo del sistema se cuente como lo que es.
#![cfg(target_os = "macos")]

use piano_core::capture::ErrorDeEntrada;
use piano_midi_io::traducir;

#[test]
fn el_permiso_denegado_no_se_confunde_con_un_fallo_de_apertura() {
    // Mandar a alguien a revisar el cable cuando lo que falta es un permiso del sistema es
    // hacerle perder la tarde.
    assert_eq!(traducir(-10844, "Piano X"), ErrorDeEntrada::PermisoDenegado);
}

#[test]
fn un_dispositivo_que_desaparece_entre_enumerar_y_abrir_se_distingue() {
    let esperado = ErrorDeEntrada::DesaparecioAlAbrir { nombre: "Piano X".into() };
    assert_eq!(traducir(-10834, "Piano X"), esperado);
    assert_eq!(traducir(-10842, "Piano X"), esperado);
}

#[test]
fn cualquier_otro_codigo_cae_en_no_se_pudo_abrir() {
    for c in [-10830, -10831, -10832, -10833, -10845, 0, 42] {
        assert_eq!(
            traducir(c, "Piano X"),
            ErrorDeEntrada::NoSePudoAbrir { nombre: "Piano X".into() },
            "codigo {c}"
        );
    }
}

#[test]
fn ningun_codigo_se_traduce_a_en_uso_por_otra_aplicacion() {
    // CoreMIDI reparte la misma fuente entre todos los clientes: comprobado ejecutando dos
    // clientes a la vez, ambos conectan. Esa variante es exclusiva de Windows, y esta
    // prueba impide que alguien la reintroduzca aqui por descuido.
    for c in -10850..=10 {
        assert!(
            !matches!(traducir(c, "X"), ErrorDeEntrada::EnUsoPorOtraAplicacion { .. }),
            "el codigo {c} se tradujo a un fallo que en macOS no puede ocurrir"
        );
    }
}
