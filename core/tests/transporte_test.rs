//! T007, T009, T009a, T011, T013, T013a, T015, T016a — el transporte entre el callback
//! del sistema operativo y el consumidor.

use piano_core::capture::{canal, EventoCrudo, Observacion, TipoEvento};
use piano_core::time::Micros;
use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;

// --- asignador instrumentado ---
//
// El contador es POR HILO, no global. `#[global_allocator]` es unico para todo el
// binario de pruebas y `cargo test` ejecuta las pruebas en paralelo: con un contador
// global, esta prueba contaba las asignaciones de las demas y fallaba de forma
// intermitente. Contar solo lo que asigna este hilo la vuelve fiable.
struct Contador;
thread_local! {
    static VIGILANDO: Cell<bool> = const { Cell::new(false) };
    static ASIGNACIONES: Cell<usize> = const { Cell::new(0) };
}

unsafe impl GlobalAlloc for Contador {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        // `try_with` porque durante la destruccion del hilo el local ya no existe, y
        // acceder a el volveria a asignar: recursion infinita.
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

fn obs(us: u64, key: u8) -> Observacion {
    Observacion { at: Micros(us), key, velocity: 100, kind: TipoEvento::Ataque, channel: 0 }
}

#[test]
fn el_evento_ocupa_exactamente_dieciseis_bytes() {
    // Importa: determina cuantos caben en el espacio acotado.
    assert_eq!(std::mem::size_of::<EventoCrudo>(), 16);
    assert_eq!(std::mem::align_of::<EventoCrudo>(), 8);
}

#[test]
fn lo_emitido_se_recoge_identico_y_en_orden() {
    let (mut tx, mut rx) = canal(64);
    for i in 0..5u64 {
        tx.emitir(obs(i * 1000, 60 + i as u8));
    }
    let mut v = Vec::new();
    assert_eq!(rx.recoger(&mut v), 5);
    assert_eq!(v.iter().map(|e| e.key).collect::<Vec<_>>(), vec![60, 61, 62, 63, 64]);
    assert_eq!(v.iter().map(|e| e.at.0).collect::<Vec<_>>(), vec![0, 1000, 2000, 3000, 4000]);
}

#[test]
fn recoger_de_una_cola_vacia_devuelve_cero() {
    let (_tx, mut rx) = canal(8);
    let mut v = Vec::new();
    assert_eq!(rx.recoger(&mut v), 0);
    assert!(v.is_empty());
}

#[test]
fn dos_eventos_del_mismo_instante_salen_en_orden_definido_y_estable() {
    // T009a — el caso real: un acorde que llega en un solo paquete comparte instante.
    // El desempate es el orden de llegada, materializado en `seq`.
    let ejecutar = || {
        let (mut tx, mut rx) = canal(64);
        for k in [67u8, 60, 64] {
            tx.emitir(obs(5_000, k));
        }
        let mut v = Vec::new();
        rx.recoger(&mut v);
        v.iter().map(|e| (e.at.0, e.seq, e.key)).collect::<Vec<_>>()
    };
    let referencia = ejecutar();
    assert_eq!(referencia, vec![(5000, 0, 67), (5000, 1, 60), (5000, 2, 64)]);
    assert!(referencia.windows(2).all(|w| w[0].1 < w[1].1), "seq estrictamente creciente");
    for _ in 0..100 {
        assert_eq!(ejecutar(), referencia, "el orden no es estable");
    }
}

#[test]
fn al_llenarse_se_descarta_lo_entrante_y_se_cuenta() {
    let (mut tx, mut rx) = canal(4);
    for i in 0..4u64 {
        tx.emitir(obs(i, 60 + i as u8)); // llena
    }
    for i in 4..10u64 {
        tx.emitir(obs(i, 60 + i as u8)); // se descartan
    }
    assert_eq!(tx.descartados(), 6, "seis entrantes descartados");
    let mut v = Vec::new();
    rx.recoger(&mut v);
    assert_eq!(
        v.iter().map(|e| e.key).collect::<Vec<_>>(),
        vec![60, 61, 62, 63],
        "lo ya almacenado sigue intacto: nunca se descarta lo viejo"
    );
}

#[test]
fn el_hueco_en_la_secuencia_localiza_donde_se_perdio() {
    let (mut tx, mut rx) = canal(4);
    for i in 0..4u64 {
        tx.emitir(obs(i, 60));
    }
    let mut v = Vec::new();
    rx.recoger(&mut v); // vacia
    for i in 4..7u64 {
        tx.emitir(obs(i, 61)); // caben
    }
    v.clear();
    rx.recoger(&mut v);
    // seq 0..3 salieron antes; ahora 4,5,6: sin hueco porque no hubo descarte
    assert_eq!(v.iter().map(|e| e.seq).collect::<Vec<_>>(), vec![4, 5, 6]);
}

#[test]
fn emitir_no_asigna_memoria_ni_en_el_primer_evento() {
    let (mut tx, mut rx) = canal(256);
    let mut v = Vec::with_capacity(512); // reservado fuera de la medicion
    ASIGNACIONES.with(|a| a.set(0));
    VIGILANDO.with(|x| x.set(true));
    for i in 0..200u64 {
        tx.emitir(obs(i, 60));
    }
    VIGILANDO.with(|x| x.set(false));
    let n = ASIGNACIONES.with(Cell::get);
    assert_eq!(n, 0, "emitir asigno memoria {n} veces");
    rx.recoger(&mut v);
}

#[test]
fn emitir_con_la_cola_llena_no_bloquea() {
    // T013a — FR-020 prohibe esperas bloqueantes, no solo asignaciones.
    // No hay consumidor ninguno: si `emitir` esperase sitio, esta prueba colgaria.
    // Que termine ES la demostracion.
    let (mut tx, _rx) = canal(8);
    let inicio = std::time::Instant::now();
    for i in 0..100_000u64 {
        tx.emitir(obs(i, 60));
    }
    let transcurrido = inicio.elapsed();
    assert_eq!(tx.descartados(), 100_000 - 8);
    assert!(
        transcurrido.as_millis() < 500,
        "100.000 emisiones con la cola llena tardaron {transcurrido:?}: algo esta esperando"
    );
}

#[test]
fn el_consumidor_duerme_y_lo_despierta_el_productor() {
    let (mut tx, mut rx) = canal(64);
    let hilo = std::thread::spawn(move || {
        let mut v = Vec::new();
        rx.esperar(); // duerme hasta que llegue algo
        rx.recoger(&mut v);
        v.len()
    });
    std::thread::sleep(std::time::Duration::from_millis(50));
    tx.emitir(obs(1, 60));
    assert_eq!(hilo.join().expect("el hilo consumidor colgo"), 1);
}

#[test]
fn un_aviso_que_llega_antes_de_dormirse_no_se_pierde() {
    // El testigo de unpark es pegajoso: si el aviso precede al park, park retorna ya.
    let (mut tx, mut rx) = canal(64);
    tx.emitir(obs(1, 60)); // el aviso llega ANTES de que nadie duerma
    let hilo = std::thread::spawn(move || {
        let mut v = Vec::new();
        rx.esperar(); // no debe quedarse dormido para siempre
        rx.recoger(&mut v);
        v.len()
    });
    assert_eq!(hilo.join().expect("el consumidor se quedo dormido"), 1);
}

#[test]
fn un_instante_que_retrocede_se_sujeta_al_ultimo_y_se_cuenta() {
    // T016a — FR-013: los instantes entregados nunca decrecen.
    let (mut tx, mut rx) = canal(64);
    tx.emitir(obs(1_000, 60));
    tx.emitir(obs(500, 61)); // retrocede
    tx.emitir(obs(2_000, 62));
    let mut v = Vec::new();
    rx.recoger(&mut v);
    assert_eq!(
        v.iter().map(|e| e.at.0).collect::<Vec<_>>(),
        vec![1_000, 1_000, 2_000],
        "el instante que retrocedia se sujeta al ultimo"
    );
    assert_eq!(tx.retrocesos(), 1);
    assert!(v.windows(2).all(|w| w[0].at <= w[1].at), "nunca decrecen");
}
