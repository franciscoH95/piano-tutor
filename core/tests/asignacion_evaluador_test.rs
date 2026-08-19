//! T029a — `Evaluador::observar` no asigna memoria por evento.
//!
//! Vive en su propio binario de pruebas porque `#[global_allocator]` es único para todo el
//! binario: mezclarlo con las demás pruebas del evaluador instrumentaría también las suyas.
//!
//! El contador es **por hilo**, no global. `cargo test` ejecuta en paralelo, y con un
//! contador global esta prueba contaría las asignaciones de las otras y fallaría de forma
//! intermitente. Ese fallo exacto ya hubo que corregirlo en la feature 002.

mod fixtures;
use fixtures::interpretaciones::{ataque, suelta};
use fixtures::SmfBuilder;
use piano_core::evaluacion::{Evaluador, Nivel};
use piano_core::load_smf;
use piano_core::practica::Mano;
use piano_core::time::Micros;
use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;

struct Contador;
thread_local! {
    static VIGILANDO: Cell<bool> = const { Cell::new(false) };
    static ASIGNACIONES: Cell<usize> = const { Cell::new(0) };
}

unsafe impl GlobalAlloc for Contador {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        // `try_with` porque durante la destrucción del hilo el local ya no existe, y
        // acceder a él volvería a asignar: recursión infinita.
        let _ = VIGILANDO.try_with(|v| {
            if v.get() {
                let _ = ASIGNACIONES.try_with(|a| a.set(a.get() + 1));
            }
        });
        unsafe { System.alloc(l) }
    }
    unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
        unsafe { System.dealloc(p, l) }
    }
}
#[global_allocator]
static A: Contador = Contador;

#[test]
fn observar_no_asigna_por_evento() {
    // La ruta crítica del Principio IV: llega una tecla y hay que decidir en menos de 30 ms.
    // Una asignación por evento mete al asignador en ese camino, y el asignador puede
    // bloquear.
    let raw = SmfBuilder::new(1000)
        .track(|t| {
            let mut t = t.tempo(0, 1_000_000);
            for i in 0..500u64 {
                t = t.note(i * 100, 60 + (i % 12) as u8, 90, 80);
            }
            t
        })
        .build();
    let song = load_smf(&raw).expect("valida");
    let manos = vec![Mano::Derecha; song.notes().len()];
    let mut e = Evaluador::nuevo(&song, &manos, None, Nivel::Intermedio);

    // Se calienta: el vector de pulsaciones crece de forma amortizada, y esas asignaciones
    // son las de la estructura, no las del evento. Lo que se mide es el estado estacionario.
    for i in 0..200u64 {
        e.observar(ataque(i * 100_000, 60, 90));
        e.observar(suelta(i * 100_000 + 50_000, 60));
    }

    ASIGNACIONES.with(|a| a.set(0));
    VIGILANDO.with(|v| v.set(true));
    for i in 200..400u64 {
        e.observar(ataque(i * 100_000, 62, 90));
        e.observar(suelta(i * 100_000 + 50_000, 62));
    }
    VIGILANDO.with(|v| v.set(false));
    let n = ASIGNACIONES.with(Cell::get);

    // 400 eventos. Se permite el crecimiento amortizado del vector —unas pocas
    // reasignaciones—, pero no una por evento.
    assert!(n < 10, "400 eventos provocaron {n} asignaciones");
    e.avanzar(Micros(40_000_000));
}
