# Tasks: Harness feedforward del núcleo

**Feature**: `001-midi-feedforward-harness` | **Fecha**: 2026-08-17
**Entrada**: [spec.md](./spec.md), [plan.md](./plan.md), [research.md](./research.md),
[data-model.md](./data-model.md), [contracts/core-api.md](./contracts/core-api.md)

## Nota sobre el orden de las tareas

La Constitución impone TDD estricto como principio no negociable. Por eso **cada tarea de
implementación va inmediatamente precedida por la tarea de su prueba**, en lugar de agrupar
todas las pruebas al principio de cada fase. El ciclo Red-Green-Refactor debe quedar visible en
el historial: la prueba se escribe, se ejecuta y **falla**, y solo entonces se implementa.

Una tarea de prueba no está completa si la prueba pasa nada más escribirla.

---

## Phase 1: Setup

- [X] T001 Crear el crate `core` con `cargo new --lib core` y añadirlo a `members` en `Cargo.toml` (raíz)
- [X] T002 Declarar la dependencia `midi_file = "0.2.0"` en `core/Cargo.toml`
- [X] T003 Declarar los lints de crate en `core/src/lib.rs`: `#![forbid(unsafe_code)]`, `#![deny(clippy::float_arithmetic)]`, `#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::indexing_slicing)]`

**Checkpoint**: `cargo check -p core` compila y `cargo tree -p core` muestra únicamente `core` y `midi_file`.

---

## Phase 2: Foundational

**Bloquea todas las historias.** Sin fixtures ni tipos de tiempo no se puede escribir ninguna prueba.

- [X] T004 Prueba del constructor de fixtures en `core/tests/fixtures.rs`: construir un SMF mínimo y verificar byte a byte la cabecera `MThd` y el chunk `MTrk`
- [X] T005 Implementar `SmfBuilder` en `core/tests/fixtures.rs` con API encadenable: `new(ppq)`, `.track(|t| ...)`, `.tempo(tick, us_per_qn)`, `.note(tick, key, vel, dur)`, `.chord(tick, &[keys], vel, dur)`, `.raw_event(bytes)`, `.build()`
- [X] T006 [P] Prueba de los newtypes en `core/src/time.rs`: `Ticks` y `Micros` comparan y ordenan; `saturating_sub` satura en cero; verificar que **no** existe `impl Sub` ni `From<Ticks> for Micros` (prueba de compilación negativa con `trybuild` o comentario documentado)
- [X] T007 Implementar `Ticks(u64)` y `Micros(u64)` en `core/src/time.rs` con `#[repr(transparent)]` y derives `Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Hash`
- [X] T008 [P] Prueba del reloj en `core/tests/clock_test.rs`: `VirtualClock` empieza en cero, avanza solo cuando se le ordena y nunca retrocede; `MonotonicClock` es no decreciente en 1.000 lecturas consecutivas
- [X] T009 Implementar el trait `Clock` y las structs `VirtualClock` y `MonotonicClock` en `core/src/clock.rs`

**Checkpoint**: `cargo test -p core` pasa con las pruebas de fundamentos.

---

## Phase 3: User Story 1 — Convertir una canción en una lección (P1) 🎯 MVP

**Objetivo**: un archivo MIDI se convierte en una secuencia ordenada y determinista de notas.

**Prueba independiente**: `cargo test -p core --test timeline_test` verde, sin ventana ni teclado.

### Errores y contadores

- [X] T010 [US1] Prueba en `core/tests/dirty_input_test.rs`: `load_smf(&[])` y `load_smf(b"basura")` devuelven `Err` y **no** entran en pánico
- [X] T011 [US1] Implementar `LoadError` (`#[non_exhaustive]`, 12 variantes de research.md Decisión 8) y `LoadReport` en `core/src/error.rs`

### Mapa de tempo

- [X] T012 [US1] Prueba en `core/src/tempo.rs`: `tick_to_us` con el ejemplo numérico trabajado de research.md Decisión 2 (PPQ 480, tempo 500.000 en tick 0 y 250.000 en tick 960); verificar truncado hacia abajo
- [X] T013 [US1] Implementar `TempoMap` y `TempoSegment` en `core/src/tempo.rs` con `anchor_scaled` en µs×PPQ y una **única** división por consulta, según la fórmula canónica de research.md Decisión 2
- [X] T014 [US1] Prueba en `core/src/tempo.rs`: sin eventos de tempo se antepone el tramo sintético de 500.000 µs/negra (FR-005); varios tempos en el mismo tick colapsan al último; `us_per_qn == 0` devuelve `LoadError::InvalidTempo`
- [X] T015 [US1] Implementar la normalización de tramos en `core/src/tempo.rs`: colapso por tick, tramo sintético inicial, rechazo de `us_per_qn == 0`, sin tramos de longitud cero
- [X] T016 [US1] Prueba de propiedad en `core/src/tempo.rs`: `us_to_tick(tick_to_us(t)) <= t` para todo `t`, y `start_us` es estrictamente creciente
- [X] T017 [US1] Implementar `us_to_tick` en `core/src/tempo.rs` con `u128` en el producto para blindar el desbordamiento

### Frontera del parser

- [X] T018 [US1] Prueba en `core/tests/dirty_input_test.rs`: cabecera con `division == 0x8000` (SMPTE) devuelve `TimingSmpteNoSoportado`; `division == 0` devuelve `DivisionCero`; formato 2 devuelve `FormatoNoSoportado`
- [X] T019 [US1] Implementar `core/src/midi/loader.rs`: pre-validación de la cabecera de 14 bytes, invocación de `midi_file::MidiFile::read` sobre `Cursor`, y traducción a eventos propios. **Este archivo es el único del crate autorizado a nombrar tipos de `midi_file`**
- [X] T020 [US1] Prueba en `core/tests/dirty_input_test.rs`: note-on con velocity 0 se trata como note-off; un SMF con running status y velocity 0 en el mismo tramo produce las notas correctas
- [X] T021 [US1] Implementar la normalización R0 en `core/src/midi/loader.rs`: reescritura de note-on velocity 0 a note-off, descarte de mensajes no-nota

### Línea temporal

- [X] T022 [US1] Prueba en `core/tests/timeline_test.rs`: cinco notas consecutivas producen cinco `ScheduledNote` con altura, inicio, duración e intensidad correctos; un acorde de tres notas comparte `onset_tick`
- [X] T023 [US1] Implementar la fusión de pistas en `core/src/timeline.rs` con la clave total `(tick_abs, track_idx, event_idx)`, acumulación con `checked_add` y techo `u32::MAX` (`LoadError::TickOverflow`)
- [X] T024 [US1] Prueba en `core/tests/dirty_input_test.rs`: el caso canónico `on(60)@0, on(60)@10, off(60)@20, off(60)@30` produce las notas `[0,20]` y `[10,30]` (política FIFO, no LIFO)
- [X] T025 [US1] Implementar el emparejamiento FIFO por `VoiceKey { track, channel, key }` en `core/src/timeline.rs` con una cola por voz
- [X] T026 [US1] Prueba en `core/tests/dirty_input_test.rs`: nota colgada se cierra al final de **su** pista y marca `Closure::HangingClosedAtTrackEnd` con `report.hanging_notes == 1`; note-off huérfano se ignora con `report.orphan_note_offs == 1`; solapamiento del mismo pitch acorta la primera nota y marca `truncated`
- [X] T027 [US1] Implementar las reglas R5, R6 y R7 en `core/src/timeline.rs` (cierre de colgadas, descarte de huérfanos, acortado de solapamiento) alimentando `LoadReport`
- [X] T028 [US1] Prueba en `core/tests/timeline_test.rs`: un cambio de tempo a mitad altera `onset_us` pero deja `onset_tick` intacto (FR-003); la misma altura repetida sin silencio produce dos notas distintas
- [X] T029 [US1] Implementar el orden total `(onset_tick, key, track, channel, seq)` con `sort_unstable_by_key` en `core/src/timeline.rs`, y la conversión masiva a microsegundos con cursor sobre los tramos de tempo
- [X] T030 [US1] Implementar `Song`, `ScheduledNote`, `Closure` y la función pública `load_smf(&[u8]) -> Result<Song, LoadError>` en `core/src/lib.rs`, conforme a `contracts/core-api.md`

**Checkpoint**: US1 completa e independientemente verificable. `cargo test -p core` verde.

---

## Phase 4: User Story 2 — Saber qué nota viene (P2)

**Objetivo**: cada nota se anuncia antes de tocarse, con antelación en tiempo musical.

**Prueba independiente**: `cargo test -p core --test feedforward_test` verde.

- [X] T031 [US2] Prueba en `core/tests/feedforward_test.rs`: con `lead_ticks` dado, cada nota genera exactamente un `Cue` y `cue_at <= onset_at` siempre (FR-010, FR-012)
- [X] T032 [US2] Implementar `Cue` (32 bytes exactos, verificado con `assert_eq!(size_of::<Cue>(), 32)`) y `CueSchedule::build(song, lead_ticks)` en `core/src/feedforward.rs`, con `cue_tick = onset_tick.saturating_sub(lead_ticks)`
- [X] T033 [US2] Prueba en `core/tests/feedforward_test.rs`: los cues de un acorde salen contiguos y ordenados de grave a agudo (FR-013); el orden total `(cue_at, cue_tick, note_index)` no admite empates
- [X] T034 [US2] Implementar el orden total de cues en `core/src/feedforward.rs`
- [X] T035 [US2] Prueba en `core/tests/feedforward_test.rs`: `advance_to` emite cada cue una sola vez; un salto de tiempo que cruza varias notas las emite todas (FR-015); una antelación mayor que la canción emite todo al inicio; una canción sin notas no emite nada y termina limpiamente
- [X] T036 [US2] Implementar `Playback::advance_to` en `core/src/feedforward.rs` devolviendo un **subslice** (cero asignaciones), con las cinco invariantes de research.md Decisión 5 en `debug_assert!`
- [X] T037 [US2] Prueba en `core/tests/feedforward_test.rs`: `advance_to` con un instante anterior devuelve `Err(Rewind)` y **no altera** el estado (FR-020); `seek` sí recoloca el cursor
- [X] T038 [US2] Implementar `Rewind`, `Playback::seek` y `Playback::is_finished` (FR-016) en `core/src/feedforward.rs`
- [X] T039 [US2] Prueba en `core/tests/feedforward_test.rs`: con un cambio de tempo, el aviso mantiene la misma distancia **musical** a la nota aunque el margen en segundos cambie (FR-011)
- [X] T040 [US2] Implementar `Cue::remaining_at` con `saturating_sub` en `core/src/feedforward.rs` (FR-014)

**Checkpoint**: US2 completa. El camino feedforward funciona de extremo a extremo.

---

## Phase 5: User Story 3 — Comportamiento reproducible (P3)

**Objetivo**: la misma entrada produce siempre el mismo resultado, sin esperar tiempo real.

**Prueba independiente**: `cargo test -p core --test determinism_test` verde en menos de un segundo.

- [X] T041 [US3] Prueba en `core/tests/determinism_test.rs`: cargar el mismo SMF 100 veces produce `Song` idénticas (comparación estructural completa, incluido el orden dentro de los acordes)
- [X] T042 [US3] Prueba en `core/tests/determinism_test.rs`: ejecutar la misma canción dos veces con la misma secuencia de avances de `VirtualClock` produce secuencias de cues idénticas (SC-003)
- [X] T043 [US3] Prueba en `core/tests/determinism_test.rs`: una pieza de 10 minutos con 5.000 notas se recorre entera bajo `VirtualClock` sin esperar tiempo real (FR-019)
- [X] T044 [US3] Ajustar lo que haga falta en `core/src/` para que las tres pruebas anteriores pasen sin relajarlas: eliminar cualquier dependencia de `HashMap` en rutas que afecten al orden, y cualquier uso de reloj de sistema fuera de `MonotonicClock`

**Checkpoint**: las tres historias completas y verificadas.

---

## Phase 6: Polish

- [X] T045 Prueba de invariante de coste en `core/tests/cost_invariant_test.rs`: instrumentar el contador de comparaciones de `advance_to` y verificar que emitir `k` cues cuesta `k+1` comparaciones en una canción de 10 notas y en otra de 10.000 (SC-006). Contar comparaciones, **no** medir tiempo de reloj
- [X] T046 [P] Prueba de rendimiento en `core/tests/perf_test.rs`: 1.000 notas se convierten en línea temporal en menos de 100 ms (SC-001), con margen amplio para no volverse intermitente en CI
- [X] T047 [P] Documentar con rustdoc toda la API pública de `core/src/lib.rs` conforme a `contracts/core-api.md`, incluido el contrato de truncado de `tick_to_us`
- [X] T048 [P] Verificar `cargo clippy -p core -- -D warnings` limpio, sin `#[allow]` nuevos sin justificar en comentario
- [X] T049 Verificar que la suite completa (`cargo test -p core`) termina en menos de 1 segundo (SC-002) y dejar el dato registrado en `specs/001-midi-feedforward-harness/quickstart.md`
- [X] T050 Verificar `cargo tree -p core` y confirmar que no aparece `tauri` ni ninguna dependencia de interfaz (Principio III)

---

## Dependencias

```
Phase 1 (Setup)  ──►  Phase 2 (Foundational)  ──►  Phase 3 (US1)  ──►  Phase 4 (US2)  ──►  Phase 5 (US3)
                                                          │                                      │
                                                          └──────────────────────────────────────┴──►  Phase 6 (Polish)
```

- **US2 depende de US1**: no se pueden construir avisos sin línea temporal.
- **US3 depende de US1 y US2**: verifica el determinismo de ambas.
- Dentro de cada fase, la tarea de prueba **siempre** precede a la de implementación que la
  satisface. Ese orden no es negociable (Constitución, Principio II).

## Oportunidades de paralelismo

Marcadas con `[P]`. Son pocas a propósito: TDD estricto serializa casi todo, porque cada
implementación depende de que su prueba exista y falle antes.

- **Phase 2**: T006/T007 (tipos de tiempo) y T008/T009 (reloj) son módulos independientes.
- **Phase 6**: T046, T047 y T048 tocan archivos distintos y no dependen entre sí.

## Estrategia de entrega

- **MVP = Phase 1 + Phase 2 + Phase 3 (US1)**. En ese punto una canción ya se convierte en una
  lección estructurada y verificable, que es el cimiento de todo lo demás.
- Phase 4 añade el valor propio del feedforward.
- Phase 5 y 6 endurecen lo construido.

## Resumen

| Fase | Tareas | De ellas, pruebas |
| --- | --- | --- |
| 1. Setup | T001–T003 (3) | 0 |
| 2. Foundational | T004–T009 (6) | 3 |
| 3. US1 (P1) | T010–T030 (21) | 10 |
| 4. US2 (P2) | T031–T040 (10) | 5 |
| 5. US3 (P3) | T041–T044 (4) | 3 |
| 6. Polish | T045–T050 (6) | 3 |
| **Total** | **50** | **24** |

---

## Desviaciones respecto al plan, y por que

Se registran aqui en lugar de dejarlas implicitas en el codigo.

1. **El crate se llama `piano-core` (biblioteca `piano_core`), no `core`.** El directorio
   sigue siendo `core/` como decia el plan, pero el nombre del paquete colisionaba con el
   crate `core` de la biblioteca estandar de Rust, que ya expone un modulo `time`: los
   `use core::time::...` de las pruebas quedaban ambiguos y no compilaban. Se detecto al
   ejecutar la primera prueba de T006.

2. **Los fixtures viven en `core/tests/fixtures/mod.rs`**, no en `core/tests/fixtures.rs`.
   Un archivo suelto en `tests/` se compila como su propio binario de prueba y no se puede
   importar desde los demas; un subdirectorio si. Las pruebas del propio constructor estan
   en `core/tests/fixtures_test.rs` (T004).

3. **Las pruebas del mapa de tempo son de integracion** (`core/tests/tempo_test.rs`), no
   unitarias dentro de `core/src/tempo.rs` como decia T012/T014/T016. `TempoMap` es API
   publica: probarla desde fuera verifica el contrato que realmente se ofrece.

4. **T006 no usa `trybuild`** para comprobar por compilacion negativa que no existen
   `impl Sub` ni `From<Ticks> for Micros`. Anadir esa dependencia habria multiplicado el
   tiempo de compilacion de la suite, en contra de SC-002. La ausencia esta documentada en
   el modulo y garantizada por revision de codigo.

5. **T029 convierte a microsegundos con busqueda binaria por nota, no con cursor.** Una
   pieza real tiene menos de diez tramos de tempo, asi que la busqueda binaria son unas
   tres comparaciones: el cursor no habria ahorrado nada medible y anadia estado mutable.
   El requisito de coste que si importa (SC-006) es el del scheduler, y ese si esta
   implementado con cursor y verificado contando comparaciones (T045).

6. **`LoadError` tiene una variante mas de las 12 del diseno: `CuerpoIlegible`.** Cubre los
   fallos que detecta el parseador despues de una cabecera valida (chunk truncado, evento
   incoherente). Sin ella habria que mentir sobre que pista fallo.

7. **Se anadio un suelo de tempo (`MIN_US_PER_QN = 1000`) que el diseno no preveia.** El
   parseador satura un tempo de cero a 1 microsegundo por negra **en silencio**, o sea 60
   millones de pulsaciones por minuto. Aceptarlo entregaria una leccion en la que todo
   ocurre a la vez, justo lo que FR-007 prohibe. Se descubrio leyendo el codigo fuente de
   la dependencia al implementar T019.

8. **`Playback` lleva un contador de comparaciones** (`last_advance_comparisons`) que el
   diseno no mencionaba. Es lo que permite verificar SC-006 **contando** en vez de
   cronometrando; una prueba que mide milisegundos es intermitente en una maquina cargada
   y ademas no demuestra la propiedad estructural.

## Resultado de la verificacion final

| Comprobacion | Resultado |
| --- | --- |
| `cargo test -p piano-core` | **79 pruebas, todas verdes** |
| Tiempo de ejecucion de la suite (SC-002 < 1 s) | **60 ms** |
| `cargo clippy -p piano-core --all-targets -- -D warnings` | **limpio** |
| `cargo doc -p piano-core --no-deps` | **sin avisos** |
| `cargo tree -p piano-core` (Principio III) | **solo `piano-core` y `midi_file`** |
| 1.000 notas a leccion (SC-001 < 100 ms) | **verificado en `perf_test.rs`** |
| Coste de emision independiente del tamano (SC-006) | **verificado contando en `cost_invariant_test.rs`** |
