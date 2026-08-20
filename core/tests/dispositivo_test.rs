//! T019-T022b — identidad del dispositivo y modos de fallo al abrir.

use piano_core::capture::{catalogar, reconocer, Dispositivo, DeviceId, ErrorDeEntrada, Reconocimiento};

fn dev(id: Option<u64>, nombre: &str, pos: u16) -> Dispositivo {
    Dispositivo { id_sistema: id.map(DeviceId), nombre: nombre.to_string(), posicion: pos }
}

// ---------------------------------------------------------------- catalogar

#[test]
fn dos_dispositivos_con_el_mismo_nombre_se_distinguen_por_su_posicion() {
    let cat = catalogar(&[(Some(1), "Piano X"), (Some(2), "Piano X"), (Some(3), "Otro")]);
    assert_eq!(cat.len(), 3);
    assert_eq!((cat[0].nombre.as_str(), cat[0].posicion), ("Piano X", 0));
    assert_eq!((cat[1].nombre.as_str(), cat[1].posicion), ("Piano X", 1));
    assert_eq!((cat[2].nombre.as_str(), cat[2].posicion), ("Otro", 0), "otro nombre, cuenta aparte");
}

#[test]
fn un_nombre_vacio_recibe_una_etiqueta_generada_y_sigue_siendo_elegible() {
    let cat = catalogar(&[(Some(7), "")]);
    assert!(!cat[0].nombre.is_empty(), "un nombre vacio no se puede mostrar al usuario");
    assert!(cat[0].nombre.contains('7'), "la etiqueta identifica al dispositivo: {}", cat[0].nombre);
}

#[test]
fn catalogar_es_determinista() {
    let entrada = [(Some(1), "A"), (None, "A"), (Some(2), "B")];
    let a = catalogar(&entrada);
    for _ in 0..20 {
        assert_eq!(catalogar(&entrada), a);
    }
}

// ---------------------------------------------------------------- reconocer

#[test]
fn el_identificador_del_sistema_manda_sobre_el_nombre() {
    // FR-004b. El teclado cambio de conector: mismo id, otro nombre y otra posicion.
    let recordado = dev(Some(42), "Piano X", 1);
    let disponibles = vec![dev(Some(9), "Otro", 0), dev(Some(42), "Piano X (USB 2)", 0)];
    assert_eq!(reconocer(&recordado, &disponibles), Reconocimiento::Encontrado(1));
}

#[test]
fn sin_identificador_se_recurre_al_nombre_y_la_posicion() {
    let recordado = dev(None, "Piano X", 1);
    let disponibles = vec![dev(None, "Piano X", 0), dev(None, "Piano X", 1)];
    assert_eq!(reconocer(&recordado, &disponibles), Reconocimiento::Encontrado(1));
}

#[test]
fn si_el_identificador_no_casa_se_prueba_la_pareja_de_reserva() {
    // El sistema reasigno el id, pero el nombre y la posicion siguen igual.
    let recordado = dev(Some(42), "Piano X", 0);
    let disponibles = vec![dev(Some(99), "Piano X", 0)];
    assert_eq!(reconocer(&recordado, &disponibles), Reconocimiento::Encontrado(0));
}

#[test]
fn si_no_casa_ninguno_se_pide_al_usuario_y_nunca_se_abre_otro() {
    // FR-004c. Capturar del aparato equivocado sin avisar es peor que no capturar.
    let recordado = dev(Some(42), "Piano X", 0);
    let disponibles = vec![dev(Some(99), "Teclado Distinto", 0), dev(Some(7), "Otro Mas", 0)];
    assert_eq!(reconocer(&recordado, &disponibles), Reconocimiento::PedirAlUsuario);
}

#[test]
fn sin_ningun_dispositivo_disponible_se_pide_al_usuario() {
    assert_eq!(reconocer(&dev(Some(1), "X", 0), &[]), Reconocimiento::PedirAlUsuario);
}

#[test]
fn solo_se_reconoce_el_elegido_y_nunca_se_fusionan_dispositivos() {
    // T021a / FR-004: ni autoseleccion ni fusion. `reconocer` devuelve UN indice, jamas
    // una lista, asi que el tipo mismo impide fusionar.
    let recordado = dev(Some(2), "B", 0);
    let disponibles = vec![dev(Some(1), "A", 0), dev(Some(2), "B", 0), dev(Some(3), "C", 0)];
    match reconocer(&recordado, &disponibles) {
        Reconocimiento::Encontrado(i) => {
            assert_eq!(i, 1);
            assert_eq!(disponibles[i].nombre, "B");
        }
        otro => panic!("esperaba encontrar exactamente uno, no {otro:?}"),
    }
}

// ---------------------------------------------------------------- modos de fallo

#[test]
fn los_tres_modos_de_fallo_al_abrir_son_distinguibles() {
    // T022a / FR-005: cada uno se comunica de forma distinta, y ninguno interrumpe la app.
    let a = ErrorDeEntrada::SinDispositivos;
    let b = ErrorDeEntrada::NoSePudoAbrir { nombre: "Piano X".into() };
    let c = ErrorDeEntrada::EnUsoPorOtraAplicacion { nombre: "Piano X".into() };
    assert_ne!(a, b);
    assert_ne!(b, c);
    assert_ne!(a, c);
    for e in [&a, &b, &c] {
        let texto = e.to_string();
        assert!(!texto.is_empty(), "todo error debe poder mostrarse al usuario");
    }
    // Es un Error de verdad, no un String suelto.
    let _: &dyn std::error::Error = &a;
}

#[test]
fn un_fallo_del_sistema_conserva_su_codigo() {
    // El backend de Windows se escribio sin ninguna maquina Windows delante. El dia que
    // falle alli, el unico dato util sera el numero que devolvio el sistema. Hoy ese numero
    // se descarta, y «no se pudo abrir» acaba siendo indistinguible de «no hay teclado»:
    // dos causas opuestas, el mismo cartel, y nada que buscar.
    let con_codigo = ErrorDeEntrada::FalloDelSistema {
        operacion: "enumerar los teclados".into(),
        codigo: Some(-2_147_024_891),
    };
    let sin_respuesta = ErrorDeEntrada::FalloDelSistema {
        operacion: "abrir «Piano X»".into(),
        codigo: None,
    };
    assert_ne!(con_codigo, sin_respuesta);

    // En hexadecimal, que es como lo publica el fabricante y como se busca. En decimal,
    // -2147024891 no encuentra absolutamente nada; 0x80070005 encuentra E_ACCESSDENIED.
    let texto = con_codigo.to_string();
    assert!(texto.contains("0x80070005"), "el texto fue «{texto}»");
    assert!(texto.contains("enumerar los teclados"), "el texto fue «{texto}»");

    // Y cuando el sistema no contesta no hay codigo que dar: decirlo es mejor que inventar
    // un cero, que se leeria como exito.
    let texto = sin_respuesta.to_string();
    assert!(!texto.contains("0x"), "sin respuesta no hay codigo que mostrar: «{texto}»");
    assert!(texto.contains("abrir «Piano X»"), "el texto fue «{texto}»");
}
