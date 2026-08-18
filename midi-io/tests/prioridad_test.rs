//! T052 — la prioridad del hilo consumidor.

use piano_midi_io::prioridad::elevar_hilo_actual;

#[test]
fn elevar_la_prioridad_nunca_entra_en_panico() {
    // Puede fallar por falta de privilegios; lo que no puede es tumbar nada. Se acepta
    // cualquiera de los dos resultados, y ese es justo el contrato.
    let r = elevar_hilo_actual();
    println!("resultado en esta maquina: {r:?}");
    if let Err(e) = r {
        assert!(!e.to_string().is_empty(), "todo error debe poder mostrarse");
    }
}

#[test]
fn es_idempotente() {
    let primera = elevar_hilo_actual();
    let segunda = elevar_hilo_actual();
    assert_eq!(primera.is_ok(), segunda.is_ok(), "llamarla dos veces no cambia el resultado");
}

#[test]
fn el_hilo_sigue_trabajando_despues() {
    // Si la elevacion dejase el hilo en un estado raro, esto no terminaria.
    let _ = elevar_hilo_actual();
    let mut suma: u64 = 0;
    for i in 0..1_000_000u64 {
        suma = suma.wrapping_add(i);
    }
    assert!(suma > 0);
}

#[test]
fn se_puede_elevar_un_hilo_secundario_sin_afectar_al_principal() {
    let hilo = std::thread::spawn(|| {
        let _ = elevar_hilo_actual();
        42
    });
    assert_eq!(hilo.join().expect("el hilo elevado debe terminar"), 42);
}
