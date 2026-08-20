//! M2 — que cada fallo del sistema se cuente como lo que es. Rama de Windows.
//!
//! Gemela de `traduccion_test.rs`, que cubre la de macOS. Esta no existia: `traducir` en
//! Windows no tenia ni una sola prueba, aunque es una funcion pura y no necesita hardware
//! para ejercerse. La corre el runner de Windows en cada integracion continua.
#![cfg(windows)]

use piano_core::capture::ErrorDeEntrada;
use piano_midi_io::traducir;

/// E_ACCESSDENIED.
const ACCESO_DENEGADO: i32 = -2_147_024_891;
/// E_INVALIDARG.
const ARGUMENTO_INVALIDO: i32 = -2_147_024_809;
/// HRESULT_FROM_WIN32(ERROR_NOT_FOUND).
const NO_ENCONTRADO: i32 = -2_147_023_728;
/// E_POINTER. La firma tipica de `MidiInPort::FromIdAsync` completando con exito pero
/// devolviendo null: windows-rs rechaza la interfaz nula y este es el codigo que sale.
const PUNTERO_NULO: i32 = -2_147_467_261;

#[test]
fn el_puerto_tomado_por_otra_aplicacion_se_distingue() {
    // En Windows un puerto MIDI de entrada es exclusivo; en macOS CoreMIDI reparte la misma
    // fuente entre todos los clientes. Por eso esta variante solo puede alcanzarse aqui, y
    // decirla bien es lo que evita mandar al alumno a revisar un cable que esta perfecto.
    assert_eq!(
        traducir(ACCESO_DENEGADO, "Piano X"),
        ErrorDeEntrada::EnUsoPorOtraAplicacion { nombre: "Piano X".into() }
    );
}

#[test]
fn un_dispositivo_que_desaparece_entre_enumerar_y_abrir_se_distingue() {
    let esperado = ErrorDeEntrada::DesaparecioAlAbrir { nombre: "Piano X".into() };
    assert_eq!(traducir(ARGUMENTO_INVALIDO, "Piano X"), esperado);
    assert_eq!(traducir(NO_ENCONTRADO, "Piano X"), esperado);
}

#[test]
fn un_codigo_desconocido_llega_con_su_numero_y_no_se_pierde() {
    // Esta es la prueba que hace util la primera ejecucion en una maquina Windows real. La
    // tabla de traduccion se dedujo leyendo documentacion, no midiendo: nadie ha visto aun
    // que devuelve WinRT con el puerto ocupado. Si un codigo no contemplado se convirtiese
    // en `NoSePudoAbrir`, el numero se perderia y no habria con que corregir la tabla.
    for c in [PUNTERO_NULO, -2_147_467_262, -2_147_467_259, 42] {
        match traducir(c, "Piano X") {
            ErrorDeEntrada::FalloDelSistema { codigo: Some(v), .. } => assert_eq!(v, c),
            otro => panic!("el codigo {c} se perdio por el camino: {otro:?}"),
        }
    }
}

#[test]
fn el_codigo_se_muestra_en_hexadecimal() {
    // En decimal, -2147467261 no encuentra absolutamente nada. En hexadecimal, 0x80004003
    // es la primera respuesta de cualquier buscador. El formato ES el diagnostico.
    let texto = traducir(PUNTERO_NULO, "Piano X").to_string();
    assert!(texto.contains("0x80004003"), "el texto fue «{texto}»");
    assert!(texto.contains("Piano X"), "el texto fue «{texto}»");
}

#[test]
fn ningun_codigo_se_traduce_a_permiso_denegado() {
    // `PermisoDenegado` es exclusiva de macOS, que pide consentimiento explicito para el
    // acceso a MIDI. Windows no lo pide: mandar ahi al alumno seria mandarlo a una pantalla
    // de ajustes donde no hay nada que conceder. Impide que alguien la reintroduzca aqui.
    for c in [ACCESO_DENEGADO, ARGUMENTO_INVALIDO, NO_ENCONTRADO, PUNTERO_NULO, -10_844, 0] {
        assert_ne!(traducir(c, "X"), ErrorDeEntrada::PermisoDenegado, "codigo {c}");
    }
}
