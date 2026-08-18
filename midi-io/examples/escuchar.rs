//! T041 — el unico modo de ejercer esta capa con un teclado de verdad.
//!
//! ```sh
//! cargo run -p piano-midi-io --example escuchar
//! ```
//!
//! No forma parte de la verificacion automatica: necesita hardware delante.

fn main() {
    #[cfg(target_os = "macos")]
    {
        use piano_core::capture::{Cierre, Emparejador};
        use piano_core::clock::MonotonicClock;
        use piano_core::time::Micros;

        let disponibles = match piano_midi_io::dispositivos() {
            Ok(d) if !d.is_empty() => d,
            Ok(_) => {
                println!("No hay ningun teclado MIDI conectado.");
                return;
            }
            Err(e) => {
                println!("No se pudieron enumerar los dispositivos: {e}");
                return;
            }
        };

        println!("Teclados disponibles:");
        for (i, d) in disponibles.iter().enumerate() {
            let id = d.id_sistema.map_or("sin id".to_string(), |v| format!("id {}", v.0));
            println!("  [{i}] {} (posicion {}, {id})", d.nombre, d.posicion);
        }

        let elegido = &disponibles[0];
        println!("\nAbriendo «{}». Toca algo. Ctrl-C para salir.\n", elegido.nombre);

        // El reloj de sesion. En la aplicacion real lo crea `src-tauri` UNA sola vez y se
        // lo pasa tanto a la captura como a la reproduccion.
        let reloj = MonotonicClock::start();
        let mut captura = match piano_midi_io::abrir(elegido, reloj) {
            Ok(c) => c,
            Err(e) => {
                println!("No se pudo abrir: {e}");
                return;
            }
        };

        let mut emparejador = Emparejador::nuevo();
        let mut buffer = Vec::with_capacity(256);
        let inicio = std::time::Instant::now();
        loop {
            captura.receptor().esperar();
            buffer.clear();
            captura.receptor().recoger(&mut buffer);
            for ev in buffer.drain(..) {
                if let Some(p) = emparejador.consumir(ev) {
                    let marca = match p.closure {
                        Cierre::PorSuelta => "",
                        Cierre::PorRepulsacion => "  (repulsada)",
                        _ => "  (cerrada sin suelta)",
                    };
                    println!(
                        "nota {:>3}  intensidad {:>3}  duracion {:>6} ms{marca}",
                        p.key,
                        p.velocity,
                        (p.end.0 - p.onset.0) / 1_000
                    );
                }
            }
            if inicio.elapsed().as_secs() > 300 {
                break;
            }
            let _ = Micros(0);
        }
    }

    #[cfg(not(target_os = "macos"))]
    println!("Este ejemplo solo esta implementado para macOS por ahora.");
}
