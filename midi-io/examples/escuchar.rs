//! T042 y T071 — el arnes manual de la capa de entrada. Necesita hardware delante.
//!
//! ```sh
//! cargo run -p piano-midi-io --example escuchar        # el primer teclado
//! cargo run -p piano-midi-io --example escuchar -- 2   # el de la posicion 2
//! ```
//!
//! **No lleva ni un `#[cfg]`.** Antes tenia el cuerpo entero dentro de
//! `#[cfg(target_os = "macos")]`, asi que en Windows imprimia «solo esta implementado para
//! macOS» y salia con codigo 0: el unico programa capaz de ejercer un teclado real no
//! ejercia el backend que hacia falta validar. Como `lib.rs` exporta los mismos nombres en
//! las tres plataformas, la forma correcta de escribirlo es sin ninguna condicion; asi no
//! puede volver a pasar por descuido.
//!
//! Recorre la lista de T042 en orden y da un veredicto por paso, de modo que quien esta
//! delante de la maquina sepa en un vistazo hasta donde llego:
//!
//! 1. enumerar,
//! 2. abrir,
//! 3. recibir notas,
//! 4. enterarse de la retirada del teclado,
//! 5. **cerrar el puerto sin colgarse** (Microsoft KB4460006 documenta un cuelgue
//!    irrecuperable justo en esa ruta),
//! 6. reabrir tras volver a enchufarlo (T071).

use piano_core::capture::{reconocer, Cierre, Dispositivo, Emparejador, Reconocimiento};
use piano_core::clock::{Clock, MonotonicClock};
use piano_core::time::Micros;
use piano_midi_io::vigia::{Presencia, Vigia};
use std::io::Write;
use std::time::{Duration, Instant};

/// Cuanto escucha antes de rendirse, si no pasa nada.
const SESION: Duration = Duration::from_secs(300);
/// Cada cuanto mira el anillo. Ver la nota sobre la cifra de latencia en [`escuchar`].
const SONDEO: Duration = Duration::from_millis(1);
/// Cuanto espera a que el teclado vuelva a aparecer despues de retirarlo.
const ESPERA_REENCHUFE: Duration = Duration::from_secs(120);
/// Ausencias consecutivas para dar por perdido el teclado. Es la regla de la sesion.
const AUSENCIAS_PARA_PERDIDA: u8 = 2;

fn paso(n: u8, que: &str) {
    print!("{n}) {que} ... ");
    let _ = std::io::stdout().flush();
}

fn main() {
    println!("== Arnes manual de entrada MIDI (T042 / T071) ==");
    println!("   plataforma: {}\n", std::env::consts::OS);

    // Un unico reloj de sesion, como en la aplicacion real: se le pasa una copia a la
    // captura y se conserva otra para medir. `MonotonicClock` es `Copy` y ambas comparten
    // origen, asi que los instantes son comparables entre si.
    let reloj = MonotonicClock::start();

    paso(1, "Enumerar");
    let disponibles = match piano_midi_io::dispositivos() {
        Ok(d) => {
            println!("bien, {} teclado(s)", d.len());
            d
        }
        Err(e) => {
            println!("FALLO");
            println!("   {e}");
            println!();
            println!("   Ojo: esto NO significa «no hay teclado». La enumeracion fallo. El");
            println!("   codigo de arriba es lo que devolvio el sistema; buscalo tal cual.");
            return;
        }
    };
    if disponibles.is_empty() {
        println!("   La enumeracion funciono y no hay ningun teclado conectado.");
        println!("   En Parallels: Dispositivos > USB y Bluetooth > conecta el teclado a Windows.");
        return;
    }
    for (i, d) in disponibles.iter().enumerate() {
        let id = d.id_sistema.map_or("sin id".to_owned(), |v| format!("id {}", v.0));
        println!("   [{i}] {} (posicion {}, {id})", d.nombre, d.posicion);
    }

    let indice: usize = std::env::args().nth(1).and_then(|a| a.parse().ok()).unwrap_or(0);
    let Some(elegido) = disponibles.get(indice) else {
        println!("\n   No hay ningun teclado en la posicion {indice}.");
        return;
    };

    paso(2, &format!("Abrir «{}»", elegido.nombre));
    let mut captura = match piano_midi_io::abrir(elegido, reloj) {
        Ok(c) => {
            println!("bien");
            c
        }
        Err(e) => {
            println!("FALLO");
            println!("   {e}");
            return;
        }
    };

    println!("\n3) Toca algo. Desenchufa el teclado cuando quieras seguir con el paso 4.");
    println!("   (Ctrl-C para salir; se rinde solo a los {} s)\n", SESION.as_secs());

    let mut vigia = Vigia::nuevo(elegido.clone());
    let mut emparejador = Emparejador::nuevo();
    let desenlace = escuchar(&mut captura, &mut vigia, reloj, &mut emparejador);

    let Desenlace::Perdido { ultimo, ausente_desde } = desenlace else {
        println!("\n   Se acabo el tiempo sin retirar el teclado. Los pasos 4 a 6 no se probaron.");
        return;
    };

    println!("\n4) Retirada detectada en {} ms.", ausente_desde.as_millis());
    println!("   FR-014 pide menos de 2000 ms.");

    let colgantes = emparejador.cerrar(ultimo, Cierre::PorPerdidaDeDispositivo);
    if !colgantes.is_empty() {
        println!("   {} nota(s) quedaban hundidas; se sellaron en el ultimo evento", colgantes.len());
        println!("   recibido, no en el momento de detectarlo: fechar ahi inventaria duracion.");
    }

    // El paso que puede colgarse. Se anuncia ANTES y con la salida vaciada a proposito: si
    // el proceso se queda aqui, en la pantalla queda escrito exactamente donde.
    paso(5, "Cerrar el puerto tras la retirada");
    let t0 = Instant::now();
    captura.cerrar();
    println!("bien, {} ms (sin cuelgue)", t0.elapsed().as_millis());

    paso(6, "Vuelve a enchufar el teclado; esperando");
    let Some(vuelto) = esperar_reaparicion(elegido) else {
        println!("no aparecio en {} s", ESPERA_REENCHUFE.as_secs());
        return;
    };
    match piano_midi_io::reabrir(&vuelto, reloj) {
        Ok(mut otra) => {
            println!("reabierto");
            println!("\n   Toca de nuevo para confirmar que la captura se reanudo:\n");
            let mut emparejador = Emparejador::nuevo();
            let mut v = Vigia::nuevo(vuelto);
            let _ = escuchar(&mut otra, &mut v, reloj, &mut emparejador);
        }
        Err(e) => println!("FALLO al reabrir\n   {e}"),
    }
}

/// Como termino el bucle de escucha.
enum Desenlace {
    /// Se agoto [`SESION`] sin novedad.
    Tiempo,
    /// El teclado desaparecio de dos enumeraciones consecutivas.
    Perdido {
        /// Instante del ultimo evento recibido, para sellar lo que quedase hundido.
        ultimo: Micros,
        /// Cuanto paso entre la primera ausencia y la confirmacion.
        ausente_desde: Duration,
    },
}

/// Escucha hasta que el teclado desaparezca o se agote el tiempo.
///
/// **Sobre la cifra de latencia**: lo que se mide aqui es el trayecto desde que el sistema
/// sella el evento hasta que este hilo lo saca del anillo, e **incluye hasta [`SONDEO`] de
/// espera del propio arnes**, y es un **peor caso**, no una distribucion. Para la cifra que
/// se anota en un informe esta `cargo run -p piano-bench --release --bin latencia -- --con-hardware`,
/// que da p50 y p95 sobre quince segundos de muestras; sondea igual, pero con cientos de
/// muestras el sondeo se diluye en vez de fijar el maximo. Ninguna de las dos es la latencia
/// que siente el alumno: eso incluye el viaje por USB, que aqui ademas pasa por el paso a
/// traves del hipervisor.
fn escuchar(
    captura: &mut piano_midi_io::Captura,
    vigia: &mut Vigia,
    reloj: MonotonicClock,
    emparejador: &mut Emparejador,
) -> Desenlace {
    let mut buffer = Vec::with_capacity(256);
    let mut ultimo = Micros::ZERO;
    let mut peor_us: u64 = 0;
    let mut notas: u32 = 0;
    let mut ausencias: u8 = 0;
    let mut primera_ausencia: Option<Instant> = None;
    let inicio = Instant::now();

    while inicio.elapsed() < SESION {
        buffer.clear();
        let hubo = captura.receptor().recoger(&mut buffer);
        // Justo despues de sacarlos, para que la marca no incluya el trabajo de imprimir.
        let ahora = reloj.now();
        for ev in buffer.drain(..) {
            ultimo = ev.at;
            peor_us = peor_us.max(ahora.0.saturating_sub(ev.at.0));
            if let Some(p) = emparejador.consumir(ev) {
                notas = notas.saturating_add(1);
                let marca = match p.closure {
                    Cierre::PorSuelta => "",
                    Cierre::PorRepulsacion => "  (repulsada)",
                    Cierre::PorParada => "  (parada)",
                    Cierre::PorPerdidaDeDispositivo => "  (cerrada sin suelta)",
                };
                println!(
                    "   nota {:>3}  intensidad {:>3}  duracion {:>6} ms{marca}",
                    p.key,
                    p.velocity,
                    p.end.0.saturating_sub(p.onset.0) / 1_000
                );
            }
        }

        match vigia.novedad() {
            Some(Presencia::Ausente) => {
                primera_ausencia.get_or_insert_with(Instant::now);
                ausencias = ausencias.saturating_add(1);
                if ausencias >= AUSENCIAS_PARA_PERDIDA {
                    let ausente_desde =
                        primera_ausencia.map_or(Duration::ZERO, |t| t.elapsed());
                    resumen(notas, peor_us, captura.receptor().descartados());
                    return Desenlace::Perdido { ultimo, ausente_desde };
                }
            }
            Some(Presencia::Presente) => {
                ausencias = 0;
                primera_ausencia = None;
            }
            None => {}
        }

        if hubo == 0 {
            std::thread::sleep(SONDEO);
        }
    }
    resumen(notas, peor_us, captura.receptor().descartados());
    Desenlace::Tiempo
}

fn resumen(notas: u32, peor_us: u64, descartados: u32) {
    println!("\n   {notas} nota(s) completas.");
    println!("   Peor trayecto sistema -> anillo: {peor_us} us (incluye el sondeo del arnes).");
    if descartados > 0 {
        println!("   ATENCION: {descartados} observacion(es) descartadas por desbordamiento.");
    }
}

/// Sondea hasta que el teclado recordado vuelva a estar en la lista.
fn esperar_reaparicion(objetivo: &Dispositivo) -> Option<Dispositivo> {
    let limite = Instant::now() + ESPERA_REENCHUFE;
    while Instant::now() < limite {
        if let Ok(lista) = piano_midi_io::dispositivos() {
            if let Reconocimiento::Encontrado(i) = reconocer(objetivo, &lista) {
                return lista.get(i).cloned();
            }
        }
        std::thread::sleep(Duration::from_millis(500));
    }
    None
}
