# Quickstart: validar el harness feedforward

**Feature**: `001-midi-feedforward-harness` | **Fecha**: 2026-08-17

Cómo comprobar, desde cero, que esta funcionalidad hace lo que promete.

## Requisitos

- Rust 1.97.1 o superior (`rustc --version`).
- Nada más. **No hace falta teclado MIDI, ni ventana, ni archivos `.mid`**: los fixtures se
  construyen como bytes dentro de las propias pruebas.

## Validación completa

```sh
cd /Users/frankohiggins/Projects/teacher_learn_piano_songs_app
cargo test -p core
```

**Resultado esperado**: todas las pruebas en verde, y el conjunto completo por debajo de **1
segundo** de tiempo de ejecución (SC-002). Si tarda más, algo está esperando tiempo real: es un
fallo de diseño, no de rendimiento.

## Qué prueba cada archivo

| Archivo | Historia | Qué demuestra |
| --- | --- | --- |
| `core/tests/timeline_test.rs` | Historia 1 | Un MIDI se convierte en notas con altura, inicio, duración e intensidad correctos; los acordes comparten instante; los cambios de tempo desplazan el tiempo real pero no el musical. |
| `core/tests/feedforward_test.rs` | Historia 2 | Cada nota se anuncia una sola vez y siempre antes de su ataque; los acordes se anuncian juntos; un salto grande de tiempo no se salta ninguna nota. |
| `core/tests/determinism_test.rs` | Historia 3 | La misma canción con la misma secuencia temporal produce resultados idénticos; una pieza larga se ejecuta sin esperar su duración real. |
| `core/tests/dirty_input_test.rs` | Casos límite | Notas colgadas, note-off huérfanos, velocity cero, pitch solapado y archivos corruptos: ninguno provoca un pánico. |

## Comprobaciones puntuales

**Que no hay pánicos con entrada basura** (SC-005):

```sh
cargo test -p core dirty_input
```

**Que el coste de emitir avisos no depende del tamaño de la canción** (SC-006):

```sh
cargo test -p core cost_invariant -- --nocapture
```

Esta prueba cuenta comparaciones reales, no mide tiempo de reloj: por eso es fiable en una
máquina cargada y no genera intermitencias.

**Que el núcleo no arrastra la interfaz** (Principio III):

```sh
cargo tree -p core
```

**Resultado esperado**: solo `core` y `midi_file`. Si aparece `tauri` o cualquier cosa
relacionada con la ventana, el Principio III está roto y hay que detener la fusión.

## Cómo se ve un fixture

Los fixtures viven en `core/tests/fixtures.rs` como constructores de bytes, no como archivos
binarios. Un fixture se lee y se revisa como código:

```rust
let raw = SmfBuilder::new(480)          // PPQ
    .track(|t| t.tempo(0, 500_000)      // 120 negras/min en el tick 0
                .tempo(960, 250_000))   // 240 negras/min a partir del tick 960
    .track(|t| t.chord(0, &[60, 64, 67], 100, 480))
    .build();
```

Esto es deliberado: la constitución prohíbe distribuir obras de terceros, y además un fixture
binario es imposible de revisar en un pull request. Aquí la intención de la prueba está a la
vista.

## Lo que este quickstart todavía NO puede validar

- **Latencia real**: no hay ruta crítica que medir hasta que exista la entrada MIDI. El
  benchmark de 30 ms del Principio IV llega con esa feature. Ver Complexity Tracking en
  [plan.md](./plan.md).
- **La aplicación de escritorio**: `pnpm tauri dev` levanta la ventana del scaffold, pero todavía
  no consume `core`. Esta entrega no toca la interfaz.
