//! T065, T068 — el vigia de dispositivos, contra fuentes virtuales reales.
#![cfg(target_os = "macos")]

use coremidi::Client;
use piano_core::capture::{EstadoSesion, SesionDeCaptura};
use piano_midi_io::vigia::{Presencia, Vigia};
use piano_midi_io::dispositivos;
use std::time::{Duration, Instant};

/// Estas pruebas no manipulan un objeto nuestro: manipulan el **grafo MIDI del sistema**,
/// que es un recurso global y compartido con el resto de la maquina. Ejecutarlas en
/// paralelo hacia que CoreMIDI rechazase la creacion de clientes con el codigo -304, y las
/// nueve fallaban de golpe en 0,03 s. No era una carrera de tiempos: era un limite de
/// recursos del sistema.
///
/// Serializarlas es la unica forma honesta de probarlas. `cargo test` paraleliza dentro de
/// cada binario, asi que hace falta este cerrojo explicito.
fn en_serie() -> std::sync::MutexGuard<'static, ()> {
    static SERIE: std::sync::Mutex<()> = std::sync::Mutex::new(());
    // Si una prueba anterior entro en panico, el cerrojo queda envenenado. Seguir es lo
    // correcto: ese fallo ya lo reporta su propia prueba, y bloquear las demas solo
    // esconderia informacion.
    SERIE.lock().unwrap_or_else(|e| e.into_inner())
}

fn esperar<F: Fn(&mut Vigia) -> bool>(v: &mut Vigia, ms: u64, cond: F) -> bool {
    let limite = Instant::now() + Duration::from_millis(ms);
    while Instant::now() < limite {
        if cond(v) {
            return true;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    false
}

#[test]
fn el_vigia_ve_presente_un_dispositivo_que_existe() {
    let _serie = en_serie();
    let cliente = Client::new("PianoTutorVigiaTest").expect("cliente");
    let vs = cliente.virtual_source("PianoTutorTest-vigia-presente").expect("fuente");
    std::thread::sleep(Duration::from_millis(100));
    let d = dispositivos()
        .expect("enumerar")
        .into_iter()
        .find(|d| d.nombre == "PianoTutorTest-vigia-presente")
        .expect("enumerada");

    let mut v = Vigia::con_intervalo(d, Duration::from_millis(50));
    assert!(
        esperar(&mut v, 2_000, |v| v.novedad() == Some(Presencia::Presente)),
        "el vigia no vio el dispositivo"
    );
    drop(vs);
}

#[test]
fn el_vigia_ve_ausente_un_dispositivo_que_desaparece() {
    let _serie = en_serie();
    let cliente = Client::new("PianoTutorVigiaTest").expect("cliente");
    let vs = cliente.virtual_source("PianoTutorTest-vigia-ausente").expect("fuente");
    std::thread::sleep(Duration::from_millis(100));
    let d = dispositivos()
        .expect("enumerar")
        .into_iter()
        .find(|d| d.nombre == "PianoTutorTest-vigia-ausente")
        .expect("enumerada");

    let mut v = Vigia::con_intervalo(d, Duration::from_millis(50));
    assert!(esperar(&mut v, 2_000, |v| v.novedad() == Some(Presencia::Presente)));

    drop(vs); // el teclado se desenchufa
    assert!(
        esperar(&mut v, 3_000, |v| v.novedad() == Some(Presencia::Ausente)),
        "el vigia no detecto la desaparicion"
    );
}

#[test]
fn el_vigia_y_la_sesion_juntos_declaran_la_perdida_con_doble_confirmacion() {
    let _serie = en_serie();
    // El vigia informa de hechos; la sesion decide. Este es el circuito completo, y es lo
    // que cumple SC-007 sin declarar una perdida por un parpadeo del sistema.
    let cliente = Client::new("PianoTutorVigiaTest").expect("cliente");
    let vs = cliente.virtual_source("PianoTutorTest-vigia-circuito").expect("fuente");
    std::thread::sleep(Duration::from_millis(100));
    let d = dispositivos()
        .expect("enumerar")
        .into_iter()
        .find(|d| d.nombre == "PianoTutorTest-vigia-circuito")
        .expect("enumerada");

    let mut sesion = SesionDeCaptura::nueva();
    sesion.abriendo();
    sesion.capturando();

    let mut v = Vigia::con_intervalo(d, Duration::from_millis(50));
    assert!(esperar(&mut v, 2_000, |v| v.novedad() == Some(Presencia::Presente)));
    drop(vs);

    let limite = Instant::now() + Duration::from_secs(3);
    let mut perdida = false;
    while Instant::now() < limite && !perdida {
        if let Some(p) = v.novedad() {
            perdida = match p {
                Presencia::Presente => {
                    sesion.notificar_presencia();
                    false
                }
                Presencia::Ausente => sesion.notificar_ausencia(),
            };
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    assert!(perdida, "el circuito completo no declaro la perdida");
    assert_eq!(sesion.estado(), EstadoSesion::Perdida);
}
