//! T076 — la unica preferencia que toca el disco.

use piano_tutor_lib::preferencias::{cargar, guardar, TecladoRecordado};
use std::path::PathBuf;

fn ruta(nombre: &str) -> PathBuf {
    std::env::temp_dir().join(format!("piano-tutor-test-{nombre}.json"))
}

#[test]
fn lo_guardado_se_recupera_igual() {
    let p = ruta("ida-vuelta");
    let _ = std::fs::remove_file(&p);
    let t = TecladoRecordado { id_sistema: Some(4242), nombre: "Piano X".into(), posicion: 1 };
    guardar(&p, &t).expect("guardar");
    assert_eq!(cargar(&p), Some(t));
    let _ = std::fs::remove_file(&p);
}

#[test]
fn sin_archivo_no_hay_nada_recordado() {
    assert_eq!(cargar(&ruta("no-existe-jamas")), None);
}

#[test]
fn un_archivo_corrupto_se_trata_como_si_no_hubiera_nada() {
    // Volver a preguntar es el comportamiento seguro: nunca se abre un dispositivo a
    // partir de datos dudosos.
    let p = ruta("corrupto");
    std::fs::write(&p, "{ esto no es json valido").expect("escribir");
    assert_eq!(cargar(&p), None);
    let _ = std::fs::remove_file(&p);
}

#[test]
fn la_ida_y_vuelta_a_dispositivo_conserva_las_dos_identidades() {
    use piano_core::capture::{DeviceId, Dispositivo};
    let d = Dispositivo { id_sistema: Some(DeviceId(99)), nombre: "Teclado".into(), posicion: 2 };
    let t = TecladoRecordado::from(&d);
    assert_eq!(Dispositivo::from(&t), d);
}
