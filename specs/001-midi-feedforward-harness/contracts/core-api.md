# Contrato: API pública del crate `core`

**Feature**: `001-midi-feedforward-harness` | **Fecha**: 2026-08-17

Este es el contrato que `core` ofrece a sus consumidores. En esta entrega el único consumidor son
las pruebas; en la siguiente lo será `src-tauri`. Todo lo que no aparezca aquí es privado.

## Principio de frontera

`core` **no toca el sistema de archivos, ni la red, ni el reloj de pared salvo a través de
`Clock`**. Recibe bytes y devuelve datos. Quien lee el archivo del disco es la capa de
aplicación, no el núcleo. Esto es lo que hace que toda la funcionalidad se pueda ejercer desde
`cargo test` sin ventana, sin teclado y sin ficheros de apoyo (FR-021, SC-007).

## Carga

```rust
/// Convierte los bytes de un Standard MIDI File en una canción lista para practicar.
///
/// Acepta SMF de formato 0 y 1 con timing PPQ. El formato 2 y el timing SMPTE se
/// rechazan con un error tipado.
///
/// Nunca entra en pánico, sea cual sea el contenido de `raw`.
pub fn load_smf(raw: &[u8]) -> Result<Song, LoadError>;
```

**Garantías**:

- Determinista: los mismos bytes producen la misma `Song`, bit a bit, en cualquier plataforma
  y en cualquier ejecución (FR-008, SC-003).
- Total: para cualquier `&[u8]` devuelve `Ok` o `Err`, nunca panic, nunca bucle infinito
  (FR-007, SC-005).
- Sin efectos: no abre archivos, no abre sockets, no escribe nada (FR-023).

## Consulta de la canción

```rust
impl Song {
    pub fn notes(&self) -> &[ScheduledNote];
    pub fn tempo_map(&self) -> &TempoMap;
    pub fn report(&self) -> &LoadReport;
    /// Tiempo musical del final de la última nota. `Ticks(0)` si no hay notas.
    pub fn duration_ticks(&self) -> Ticks;
    /// Tiempo real del final de la última nota. `Micros(0)` si no hay notas.
    pub fn duration_us(&self) -> Micros;
}

impl TempoMap {
    pub fn ppq(&self) -> u32;
    pub fn segments(&self) -> &[TempoSegment];
    /// Conversión exacta, truncada hacia abajo. Coste O(log n).
    pub fn tick_to_us(&self, tick: Ticks) -> Micros;
    /// Inversa: mayor tick cuyo tiempo real sea <= `us`. No está en la ruta crítica.
    pub fn us_to_tick(&self, us: Micros) -> Ticks;
}
```

**Contrato de redondeo**: `tick_to_us` trunca hacia abajo (floor). Es parte del contrato público
y está cubierto por pruebas: cambiarlo es un cambio incompatible.

## Programa de avisos

```rust
impl CueSchedule {
    /// Construye el programa de avisos con una antelación en tiempo musical.
    ///
    /// `lead_ticks` es la antelación en pulsos. Al ser musical, el margen real se
    /// estira y encoge con el tempo (FR-011). `lead_ticks == 0` es válido: el aviso
    /// coincide con el ataque.
    pub fn build(song: &Song, lead_ticks: u64) -> Self;
    pub fn cues(&self) -> &[Cue];
    pub fn len(&self) -> usize;
    pub fn is_empty(&self) -> bool;
}

impl Cue {
    /// Cuánto falta para pulsar la tecla. Satura a cero si `now` ya pasó el ataque.
    pub fn remaining_at(&self, now: Micros) -> Micros;
}
```

**Garantías**:

- Hay exactamente un `Cue` por cada nota de `song.notes()` (FR-012).
- `cues()` está ordenado de forma no decreciente por `cue_at`.
- Los cues de un acorde salen **contiguos** y en orden de grave a agudo (FR-013).

## Reproducción

```rust
impl Playback {
    pub fn new(schedule: Arc<CueSchedule>) -> Self;

    /// Avanza hasta `now` y devuelve los avisos que quedan cubiertos.
    ///
    /// El slice devuelto no asigna memoria: es una vista del programa.
    /// Coste: exactamente `k + 1` comparaciones, con `k` = avisos devueltos.
    /// NO depende del tamaño de la canción (SC-006).
    pub fn advance_to(&mut self, now: Micros) -> Result<&[Cue], Rewind>;

    /// Único camino legal hacia atrás. Recoloca el cursor. Coste O(log n).
    pub fn seek(&mut self, to: Micros);

    /// `true` cuando no queda ningún aviso por emitir (FR-016).
    pub fn is_finished(&self) -> bool;
}

/// Se intentó retroceder el tiempo dentro de una misma reproducción (FR-020).
pub struct Rewind { pub last: Micros, pub requested: Micros }
```

**Garantía dura**: `advance_to` no asigna memoria, no hace E/S, no bloquea y no puede entrar en
pánico. Es la única función de este contrato pensada para vivir dentro del presupuesto de
latencia del Principio IV.

## Reloj

```rust
pub trait Clock {
    /// Microsegundos transcurridos desde el inicio de la sesión. No decreciente.
    fn now(&self) -> Micros;
}

/// Reloj de pruebas: avanza solo cuando se le ordena.
pub struct VirtualClock { /* ... */ }
impl VirtualClock {
    pub fn new() -> Self;
    pub fn advance(&mut self, delta: Micros);
    pub fn set(&mut self, at: Micros);   // pánico en debug si retrocede
}

/// Reloj de la aplicación: monótono, basado en `std::time::Instant`.
pub struct MonotonicClock { /* ... */ }
impl MonotonicClock { pub fn start() -> Self; }
```

El reloj se inyecta **por genérico** (`fn run<C: Clock>(clock: &C, ...)`), no por `dyn Clock`,
para no pagar despacho dinámico en la ruta crítica.

**Contrato de `Clock`**: `now()` nunca decrece. `VirtualClock` lo garantiza por construcción;
`MonotonicClock` se apoya en la monotonía de `Instant`.

## Compatibilidad

- `LoadError` es `#[non_exhaustive]`: añadir variantes no es un cambio incompatible, y quien lo
  consuma debe tener un brazo `_`.
- Los newtypes de tiempo son `#[repr(transparent)]`: cambiar su representación interna **sí**
  sería incompatible.
- El contrato de truncado de `tick_to_us` y el orden total de notas y cues forman parte de la
  API observable: están cubiertos por pruebas y cambiarlos requiere una versión mayor.

## Lo que este contrato NO ofrece (deliberadamente)

- No hay captura de lo que toca el alumno, ni comparación, ni puntuación (FR-022).
- No hay filtrado por mano o por pista. Los datos para hacerlo (`track`, `channel`) están
  presentes en cada nota, pero la selección es una feature posterior (FR-009).
- No hay pedal de sustain, ni cambios de compás, ni armadura aplicados al comportamiento.
- No hay persistencia ni configuración en disco.
