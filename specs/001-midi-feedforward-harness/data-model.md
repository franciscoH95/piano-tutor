# Data Model: Harness feedforward del núcleo

**Feature**: `001-midi-feedforward-harness` | **Fecha**: 2026-08-17
**Fuente de las decisiones**: [research.md](./research.md)

Todos los tipos viven en el crate `core/`. Ninguno depende de Tauri, de la UI ni de hardware.

## Regla transversal: prohibido el punto flotante

`f32` y `f64` están **prohibidos en todo el crate**. Toda magnitud temporal es entero de 64 bits.
Es lo que hace posible el determinismo bit a bit que exige SC-003: dos ejecuciones de la misma
entrada producen exactamente los mismos bytes, en cualquier plataforma.

## Newtypes de tiempo

| Tipo | Representación | Significado |
| --- | --- | --- |
| `Ticks(u64)` | `#[repr(transparent)]` | Tiempo musical. Invariante al tempo. |
| `Micros(u64)` | `#[repr(transparent)]` | Tiempo real transcurrido, en microsegundos. |

Ambos derivan `Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Hash`.

**Reglas de tipo, deliberadas y restrictivas**:

- No existe `impl Sub` para ninguno de los dos. Solo `saturating_sub` y `checked_sub`, para que
  restar sea siempre una decisión consciente sobre qué pasa por debajo de cero.
- No existe `From<Ticks> for Micros` ni al revés. **Solo `TempoMap` sabe convertir.** Así el
  compilador impide que alguien convierta tiempo sin el mapa de tempo delante.

## Entidades

### `Song` — la canción cargada

Raíz agregada. Inmutable tras la carga.

| Campo | Tipo | Reglas |
| --- | --- | --- |
| `tempo_map` | `TempoMap` | Siempre presente; si el archivo no declara tempo, contiene el tramo sintético de 120 negras/min. |
| `notes` | `Box<[ScheduledNote]>` | Ordenado por la clave total de la Decisión 7. Puede estar vacío (canción sin notas: FR válido, no error). |
| `report` | `LoadReport` | Contadores de datos sucios tolerados. |

**Invariante**: `notes` está ordenado y ese orden es canónico. La posición de una nota en este
array es su `note_index`, referenciado desde los cues.

### `ScheduledNote` — una nota que debe tocarse

| Campo | Tipo | Reglas |
| --- | --- | --- |
| `onset_tick` | `Ticks` | Tiempo musical del ataque. |
| `end_tick` | `Ticks` | `> onset_tick` siempre (mínimo un tick). |
| `onset_us` | `Micros` | Derivado por `TempoMap`; precalculado en carga. |
| `end_us` | `Micros` | Derivado; `>= onset_us`. |
| `key` | `u8` | Altura MIDI, 0..=127. |
| `velocity` | `u8` | 1..=127. La velocity del note-**on**; la de release se descarta. |
| `track` | `u16` | Pista de origen. Conserva la identidad de voz que exige FR-006. |
| `channel` | `u8` | Canal MIDI 0..=15. |
| `closure` | `Closure` | Cómo se cerró la nota (ver más abajo). |
| `truncated` | `bool` | `true` si se acortó por solapamiento del mismo pitch (regla R7). |

`track` y `channel` son lo que permitirá filtrar por mano en una feature posterior **sin rehacer
la carga**, que es exactamente lo que decidiste en FR-009.

### `Closure` — procedencia del final de la nota

```
Normal                      // cerrada por su note-off
HangingClosedAtTrackEnd     // note-on sin note-off: cerrada al final de SU pista
```

Se conserva en el dato, no en un log, porque una feature futura querrá mostrárselo al alumno
("esta nota venía mal en el archivo") y porque los tests assertan sobre ello.

### `TempoMap` — la relación entre tiempo musical y tiempo real

| Campo | Tipo | Reglas |
| --- | --- | --- |
| `ppq` | `u32` | 1..=32767, validado en carga. `0` es error de carga. |
| `segments` | `Vec<TempoSegment>` | No vacío; `segments[0].start_tick == 0`. |

### `TempoSegment` — un tramo de tempo constante

| Campo | Tipo | Reglas |
| --- | --- | --- |
| `start_tick` | `u64` | Estrictamente creciente entre tramos. Nunca tramos de longitud cero. |
| `anchor_scaled` | `u64` | Microsegundos × PPQ acumulados hasta `start_tick`. Exacto, sin división. |
| `start_us` | `u64` | `anchor_scaled / ppq`, precalculado. Estrictamente creciente. |
| `us_per_qn` | `u32` | 1..=16_777_215. `0` es error de carga, no se corrige en silencio. |

**Por qué `anchor_scaled`**: acumular en "microsegundos × PPQ" difiere la única división hasta el
final de cada consulta. Si en cambio se dividiera tramo a tramo, cada truncado añadiría error y
la suma derivaría — rompiendo el determinismo y desplazando las notas de las piezas largas.

**Coste**: `tick → us` es `O(log n)` por búsqueda binaria (`partition_point`). En la carga
completa se usa un cursor que avanza con las notas ya ordenadas: `O(n_notas + n_tramos)`.

### `Cue` — el aviso anticipado de una nota

Exactamente 32 bytes, `Copy`.

| Campo | Tipo | Significado |
| --- | --- | --- |
| `cue_at` | `Micros` | Cuándo se anuncia. |
| `onset_at` | `Micros` | Cuándo hay que pulsar la tecla. |
| `cue_tick` | `u64` | Tiempo musical del aviso; desempate y depuración. |
| `note_index` | `u32` | Índice en `Song::notes`. |

`Cue` **no repite** pitch, track ni channel: se obtienen por `note_index`. Mantenerlo en 32 bytes
importa porque el scheduler devuelve subslices de este array en la ruta de reproducción.

`remaining_at(now) = onset_at.0.saturating_sub(now.0)` — satura a cero si un salto de tiempo
grande ya cruzó el ataque (FR-014).

### `CueSchedule` y `Playback` — la reproducción

| Tipo | Campos | Papel |
| --- | --- | --- |
| `CueSchedule` | `cues: Box<[Cue]>` | Inmutable y compartible. Ordenado por la clave de cue. |
| `Playback` | `schedule: Arc<CueSchedule>`, `next: usize`, `last: Micros` | Cursor de una reproducción. |

**Transiciones de estado de `Playback`**:

```
        advance_to(now >= last)  ──►  emite cues[next..i], next avanza, last = now
        advance_to(now <  last)  ──►  Err(Rewind), NADA cambia
        seek(to)                 ──►  next y last se recolocan (único camino hacia atrás)
```

**Invariantes** (con `debug_assert!` y prueba dedicada):

1. `cues` ordenado de forma no decreciente por `cue_at`.
2. `∀ i < next: cues[i].cue_at <= last`
3. `∀ i >= next: cues[i].cue_at > last`
4. `next` no decrece con `advance_to`; solo `seek` lo mueve atrás.
5. `last` no decrece con `advance_to`.

De (1)+(3) sale la garantía de coste de SC-006: `advance_to` hace exactamente `k+1`
comparaciones, con `k` = cues emitidos, **independientemente del tamaño de la canción**.
De (2)+(4) sale FR-012: cada cue se emite exactamente una vez.

### `LoadReport` — datos sucios tolerados

Contadores `u32`, todos a cero en una canción limpia: `hanging_notes`, `orphan_note_offs`,
`duplicate_note_ons`, `truncated_overlaps`, `percussion_notes`, `notes_out_of_88_range`,
`tempo_events_outside_track0`.

No hay logging ni E/S: son exactamente los valores sobre los que assertan los tests de casos
sucios, lo que convierte cada caso límite del spec en una aserción concreta.

### `LoadError` — fallos estructurales

`#[non_exhaustive]`. Variantes: `CabeceraTruncada`, `MagicInvalido`, `CabeceraInvalida`,
`FormatoInvalido`, `FormatoNoSoportado` (formato 2), `TimingSmpteNoSoportado`, `DivisionCero`,
`ChunkTruncado`, `MetaMalformado`, `InvalidTempo`, `TickOverflow`, `DuracionExcesiva`.

La distinción es normativa y no se difumina: **error estructural** aborta la carga (FR-007);
**dato sucio** la completa y se cuenta en `LoadReport`. Una canción con notas colgadas se carga
y se puede practicar; una con `us_per_qn == 0` no, porque implicaría una división por cero y
entregaría una lección falsa.

### `Clock` — la fuente de tiempo sustituible

Trait con dos implementaciones: `VirtualClock` (avance manual, para pruebas) y `MonotonicClock`
(`std::time::Instant`, para la app). Se inyecta por **genérico**, no por `dyn`, para no pagar
despacho dinámico en la ruta crítica.

`VirtualClock` es lo que permite ejecutar una pieza de diez minutos en microsegundos y sin una
sola prueba intermitente (SC-002, SC-003).

## Reglas de validación derivadas de los requisitos

| Regla | Requisito | Dónde se aplica |
| --- | --- | --- |
| Note-on con velocity 0 se reescribe a note-off | Caso límite del spec | Frontera del parser, antes de toda lógica |
| Identidad de nota abierta = `(track, channel, key)` | Caso límite | Emparejamiento |
| Emparejamiento FIFO por voz | Caso límite (pitch solapado) | Emparejamiento |
| Nota colgada se cierra al final de su pista | Caso límite | Post-emparejamiento |
| Note-off huérfano se ignora y se cuenta | Caso límite | Emparejamiento |
| Sin tempo declarado → 500.000 µs/negra | FR-005 | Construcción del mapa |
| Varios tempos en el mismo tick → gana el último | Caso límite | Construcción del mapa |
| `us_per_qn == 0` → error de carga | FR-007 | Construcción del mapa |
| Formato 2 → error de carga | Assumptions | Cabecera |
| Timing SMPTE → error de carga | Assumptions | Cabecera |
| Orden total `(onset_tick, key, track, channel, seq)` | FR-008 | Ordenación de notas |
| Orden total `(cue_at, cue_tick, note_index)` | FR-008, FR-013 | Ordenación de cues |
| Antelación en ticks, nunca en ms | FR-011 | Construcción de cues |
| Retroceder el tiempo → `Err(Rewind)` | FR-020 | `Playback::advance_to` |
