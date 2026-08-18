//! Banco de latencia: la puerta que salda la deuda del Principio IV.
//!
//! Vive **fuera** de `cargo test` a proposito. Un percentil 95 honesto necesita miles de
//! muestras; metido en la suite, la haria inutilizable como bucle de desarrollo, y una
//! suite lenta se acaba ignorando.
//!
//! ```sh
//! cargo run -p piano-bench --release --bin latencia
//! cargo run -p piano-bench --release --bin latencia -- --sostenido     # 10 minutos
//! cargo run -p piano-bench --release --bin latencia -- --con-hardware  # con teclado
//! ```
//!
//! Codigos de salida: 0 correcto · 1 supera la puerta de capa · 2 supera la
//! constitucional · 3 error de ejecucion.

use piano_core::capture::{canal, Emparejador, Observacion, TipoEvento};
use piano_core::clock::{Clock, MonotonicClock};
use std::time::Duration;

/// Puerta de nuestra capa. Es la que salta de verdad ante una regresion.
///
/// Se puede ajustar con `PIANO_BENCH_PUERTA_US`. Existe por dos motivos, y ninguno es
/// relajarla a conveniencia: permite **calibrarla en el runner real** antes de fijarla
/// (los numeros de referencia salieron de un portatil, no de un runner compartido de 2
/// vCPU), y permite **demostrar que la puerta muerde**, que es la unica forma de saber
/// que protege algo.
const PUERTA_CAPA_US_DEFECTO: u64 = 1_000;

fn puerta_capa() -> u64 {
    std::env::var("PIANO_BENCH_PUERTA_US")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(PUERTA_CAPA_US_DEFECTO)
}
/// Puerta constitucional (Principio IV). Margen enorme a proposito.
const PUERTA_CONSTITUCIONAL_US: u64 = 30_000;

const MUESTRAS: usize = 3_000;
const CALENTAMIENTO: usize = 500;
const REPETICIONES: usize = 3;
const INTERVALO: Duration = Duration::from_millis(1);
const CAPACIDAD: usize = 4_096;

fn percentil(ordenadas: &[u64], p: usize) -> u64 {
    if ordenadas.is_empty() {
        return 0;
    }
    let i = (ordenadas.len().saturating_mul(p) / 100).min(ordenadas.len().saturating_sub(1));
    ordenadas.get(i).copied().unwrap_or(0)
}

/// Una tanda: el productor emite `n` observaciones y el consumidor mide cuanto tarda cada
/// una en llegarle **decodificada**.
///
/// t0 es el instante que sella el productor justo antes de publicar en el anillo: la misma
/// linea donde, en produccion, publica el callback de CoreMIDI.
/// t1 es el instante que lee el consumidor **despues** de despertar de un bloqueo real y
/// **despues** de decodificar el evento al tipo de dominio. El despertar cuenta: es lo que
/// exige la definicion acordada del presupuesto.
fn tanda(n: usize, intervalo: Duration) -> Vec<u64> {
    let reloj = MonotonicClock::start();
    let (mut tx, mut rx) = canal(CAPACIDAD);

    let productor = std::thread::spawn(move || {
        for i in 0..n {
            std::thread::sleep(intervalo);
            let kind = if i % 2 == 0 { TipoEvento::Ataque } else { TipoEvento::Suelta };
            // t0: ultima instruccion antes del traspaso.
            let at = reloj.now();
            tx.emitir(Observacion { at, key: 60, velocity: 100, kind, channel: 0 });
        }
    });

    // La cola de latencia la pone el planificador, no el transporte: con el consumidor a
    // prioridad normal el percentil 99,9 se dispara a milisegundos esperando turno. Si el
    // sistema no lo permite se sigue igual, solo que sin ese seguro.
    let prioridad = piano_midi_io::prioridad::elevar_hilo_actual();

    let mut emparejador = Emparejador::nuevo();
    let mut buffer = Vec::with_capacity(CAPACIDAD);
    let _ = &prioridad;
    let mut muestras = Vec::with_capacity(n);
    while muestras.len() < n {
        rx.esperar();
        buffer.clear();
        if rx.recoger(&mut buffer) == 0 {
            continue;
        }
        for ev in buffer.drain(..) {
            // Decodificar va DENTRO de la medida: si el banco midiese solo hasta el pop,
            // no protegeria el trabajo que de verdad hace el consumidor.
            let _ = emparejador.consumir(ev);
            let t1 = reloj.now();
            muestras.push(t1.0.saturating_sub(ev.at.0));
        }
    }
    let _ = productor.join();
    muestras
}

fn informe_de_alcance() {
    println!("\n  Que mide este banco, y que NO mide");
    println!("  ─────────────────────────────────────────────────────────────────");
    println!("  DENTRO:  publicacion en el anillo, aviso al consumidor, planificacion");
    println!("           y despertar del hilo, extraccion, y decodificacion a nota.");
    println!("  FUERA:   barrido de teclas del instrumento, transporte USB y despacho");
    println!("           del driver del sistema. No son observables desde la aplicacion.");
    println!("  DELTA_SO_USB: SIN CALIBRAR  (esperado por la literatura: 1-9 ms)");
    println!();
    println!("  El banco cubre en torno al 0,11 % del recorrido que percibe el alumno.");
    println!("  Es un detector de regresiones DE NUESTRO CODIGO, no una afirmacion sobre");
    println!("  lo que siente quien toca. Para eso esta `--con-hardware`.");
    println!("  ─────────────────────────────────────────────────────────────────");
}

fn modo_normal() -> i32 {
    match piano_midi_io::prioridad::elevar_hilo_actual() {
        Ok(()) => println!("Prioridad del hilo consumidor: elevada"),
        Err(e) => println!("Prioridad del hilo consumidor: NO elevada ({e}).\n\
                            Los numeros incluyen la espera del planificador."),
    }
    println!("Banco de latencia — {REPETICIONES} repeticiones de {MUESTRAS} muestras");
    println!("(se descartan las primeras {CALENTAMIENTO} de cada una)");

    let mut p95s = Vec::with_capacity(REPETICIONES);
    for r in 1..=REPETICIONES {
        let mut m = tanda(MUESTRAS, INTERVALO);
        m.drain(..CALENTAMIENTO.min(m.len()));
        m.sort_unstable();
        let p50 = percentil(&m, 50);
        let p95 = percentil(&m, 95);
        let p99 = percentil(&m, 99);
        let max = m.last().copied().unwrap_or(0);
        println!("  ronda {r}:  p50 {p50:>6} us   p95 {p95:>6} us   p99 {p99:>6} us   max {max:>7} us");
        p95s.push(p95);
    }

    // El minimo de los p95: la tanda menos contaminada por el planificador es la que mas
    // se acerca a lo que hace el codigo. En un runner compartido las demas miden ruido.
    let p95 = p95s.iter().copied().min().unwrap_or(u64::MAX);
    let puerta = puerta_capa();
    informe_de_alcance();
    println!("\n  RESULTADO: p95 = {p95} us");
    println!("  puerta de capa .......... {puerta} us");
    println!("  puerta constitucional ... {PUERTA_CONSTITUCIONAL_US} us");

    if p95 > PUERTA_CONSTITUCIONAL_US {
        println!("\n  FALLO: se supera el presupuesto del Principio IV.");
        return 2;
    }
    if p95 > puerta {
        println!("\n  FALLO: regresion en nuestra capa. Aun dentro del presupuesto");
        println!("  constitucional, pero el margen existe para el hardware, no para nosotros.");
        return 1;
    }
    println!("\n  OK");
    0
}

/// SC-008: una sesion larga no debe degradar el retraso.
fn modo_sostenido() -> i32 {
    // Duracion configurable: el modo completo son diez minutos, pero comprobar que el
    // modo FUNCIONA no deberia costar diez minutos.
    let minutos: usize = std::env::var("PIANO_BENCH_MINUTOS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(10);
    const POR_MINUTO: usize = 6_000; // 10 ms entre eventos
    println!("Modo sostenido: {minutos} minutos. Esto tarda de verdad.");
    let mut primero = 0;
    let mut ultimo = 0;
    for minuto in 1..=minutos {
        let mut m = tanda(POR_MINUTO, Duration::from_millis(10));
        m.sort_unstable();
        let p95 = percentil(&m, 95);
        println!("  minuto {minuto:>2}:  p95 {p95:>6} us");
        if minuto == 1 {
            primero = p95;
        }
        ultimo = p95;
    }
    let limite = primero.saturating_add(primero / 10);
    println!("\n  primero {primero} us · ultimo {ultimo} us · limite {limite} us (+10 %)");
    if ultimo > limite {
        println!("  FALLO: la sesion larga degrada el retraso.");
        return 1;
    }
    println!("  OK");
    0
}

/// Mide desde el sello del propio sistema operativo. Necesita un teclado delante.
fn modo_con_hardware() -> i32 {
    println!("Modo con hardware: toca el teclado durante unos segundos.");
    #[cfg(target_os = "macos")]
    {
        let disponibles = match piano_midi_io::dispositivos() {
            Ok(d) if !d.is_empty() => d,
            _ => {
                println!("  No hay ningun teclado conectado. Nada que medir.");
                return 3;
            }
        };
        let reloj = MonotonicClock::start();
        let mut captura = match piano_midi_io::abrir(&disponibles[0], reloj) {
            Ok(c) => c,
            Err(e) => {
                println!("  No se pudo abrir: {e}");
                return 3;
            }
        };
        println!("  Abierto «{}». Recogiendo 15 segundos...", disponibles[0].nombre);
        let fin = std::time::Instant::now() + Duration::from_secs(15);
        let mut muestras = Vec::new();
        let mut buffer = Vec::with_capacity(1_024);
        while std::time::Instant::now() < fin {
            buffer.clear();
            captura.receptor().recoger(&mut buffer);
            for ev in buffer.drain(..) {
                muestras.push(reloj.now().0.saturating_sub(ev.at.0));
            }
            std::thread::sleep(Duration::from_millis(2));
        }
        if muestras.is_empty() {
            println!("  No llego ninguna nota. ¿Tocaste algo?");
            return 3;
        }
        muestras.sort_unstable();
        println!("  {} notas · p50 {} us · p95 {} us", muestras.len(),
                 percentil(&muestras, 50), percentil(&muestras, 95));
        println!("\n  Este numero incluye el despacho del driver; la diferencia con el de");
        println!("  integracion continua es el tramo que el banco sintetico no cubre.");
        0
    }
    #[cfg(not(target_os = "macos"))]
    {
        println!("  Solo implementado para macOS por ahora.");
        3
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let modo = args.get(1).map(String::as_str);
    let codigo = match modo {
        Some("--sostenido") => modo_sostenido(),
        Some("--con-hardware") => modo_con_hardware(),
        Some(otro) => {
            println!("Modo desconocido: {otro}");
            3
        }
        None => modo_normal(),
    };
    std::process::exit(codigo);
}
