# Implementation Plan: Harness feedforward del núcleo

**Branch**: `001-midi-feedforward-harness` | **Date**: 2026-08-17 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `/specs/001-midi-feedforward-harness/spec.md`

## Summary

Un crate Rust nuevo, `core/`, que convierte un archivo MIDI en una lección con estructura
temporal y anuncia cada nota antes de que haya que tocarla, todo ello sin ventana, sin teclado
conectado y sin esperar tiempo real.

El diseño se apoya en tres decisiones que la investigación cerró con evidencia:

1. **Todo el trabajo caro ocurre en la carga.** Parsear, construir el mapa de tempo, emparejar
   notas, calcular los instantes reales y materializar los avisos ordenados se hace una sola vez.
   Reproducir es avanzar un índice sobre un array ordenado comparando enteros: cero divisiones,
   cero asignaciones, cero E/S. Es lo que hace alcanzable el presupuesto de latencia cuando
   entre el teclado real.
2. **Aritmética entera exacta, sin coma flotante en todo el crate.** El mapa de tempo acumula en
   "microsegundos × PPQ" y difiere la única división al final de cada consulta, en lugar de
   dividir tramo a tramo. Sin eso, el error de truncado se acumularía y desplazaría las notas de
   las piezas largas, rompiendo el determinismo que exige SC-003.
3. **El tiempo entra por un `Clock` inyectado como genérico.** Las pruebas usan un reloj virtual
   que avanza cuando se le ordena; la aplicación usará uno monótono. Es lo que permite ejecutar
   una pieza de diez minutos en microsegundos y sin una sola prueba intermitente.

## Technical Context

**Language/Version**: Rust 1.97.1, edition 2021. La UI (TypeScript 5.8 / React 19) queda fuera
del alcance de esta feature: no se toca ni una línea de `src/`.

**Primary Dependencies**: `midi_file 0.2.0` (MIT, cero dependencias transitivas, cero `unsafe`)
como único parseador de Standard MIDI Files. Nada más: el resto del crate es `std`.

Esta elección **contradice a propósito** la recomendación inicial de la investigación, que
proponía `midly 0.5.3`. La verificación adversarial la refutó compilando de verdad: `midly`
entra en pánico ante entrada malformada (desbordamiento al negar `-128i8` al leer el campo
`division`), lo que viola FR-007 de forma directa, y su issue está abierto sin respuesta. En un
barrido de los 65.536 valores posibles de `division` más 200.000 mutaciones aleatorias, `midly`
provocó 262 pánicos y `midi_file` ninguno. Verificado además de primera mano en este repositorio:
`midi_file` devuelve `Err` limpio exactamente con la entrada que hace panicar a `midly`.

El riesgo asumido es que `midi_file` es joven (primera publicación en febrero de 2026, ~12.500
descargas). Se mitiga estructuralmente: `core/src/midi/loader.rs` es el **único** archivo del
crate autorizado a nombrar tipos de `midi_file`. Cambiar de parseador es trabajo de un día.

**Storage**: N/A en esta entrega. No hay persistencia: el archivo MIDI lo aporta el usuario y la
línea temporal vive en memoria durante la sesión.

**Testing**: `cargo test` (pruebas unitarias por módulo + pruebas de integración en `core/tests/`),
con fixtures MIDI construidos en memoria como bytes, sin depender de archivos externos ni de
hardware.

**Target Platform**: macOS (arm64 y x86_64) y Windows 10+. El crate `core` es headless y no
depende de ninguna API de sistema más allá de `std`.

**Project Type**: desktop-app. Workspace Cargo con Tauri 2; esta feature entrega exclusivamente
un crate de biblioteca (`core`), sin comandos Tauri ni IPC.

**Performance Goals**:

- Convertir 1.000 notas en línea temporal en < 100 ms (SC-001).
- Suite completa de pruebas en < 1 s (SC-002).
- Emitir cues con coste proporcional al número de cues emitidos, no al tamaño de la canción
  (SC-006).

**Constraints**:

- Determinismo total: misma entrada y misma secuencia temporal producen resultados idénticos
  (SC-003). Prohibido cualquier uso de aleatoriedad, orden de hash no determinista o reloj del
  sistema dentro de la lógica.
- Headless: sin ventana y sin teclado conectado (SC-007, FR-021).
- Sin acceso a red (FR-023).
- Sin pánicos ante entrada malformada (FR-007, SC-005).
- Sin asignaciones no acotadas en el camino de emisión de cues (Constitución, Principio IV).

**Scale/Scope**: piezas de hasta ~10 minutos y ~10.000 notas. Un crate nuevo con unos 5 módulos.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principio | Estado | Cómo lo cumple este plan |
| --- | --- | --- |
| **I. Precisión Musical Primero** | PASA | El tiempo se deriva del contenido MIDI, nunca del reloj de la UI. Las tolerancias no aplican todavía (no hay evaluación), pero el mapa de tempo y la línea temporal se validan contra fixtures de referencia. |
| **II. TDD estricto** | PASA | Cada módulo se implementa después de su prueba. `tasks.md` ordenará explícitamente prueba antes que implementación, y el historial de commits lo reflejará. |
| **III. Núcleo desacoplado de la UI** | PASA | Es literalmente el objeto de esta feature: el crate `core` no depende de Tauri, de React ni de ningún dispositivo. Se ejerce entero desde `cargo test`. |
| **IV. Tiempo real con presupuesto** | PARCIAL — ver Complexity Tracking | La ruta crítica completa (tecla → evaluación → UI) todavía no existe: no hay entrada MIDI ni interfaz. El presupuesto de 30 ms no es medible aún. Este plan lo protege por diseño (coste de emisión proporcional a los cues emitidos, cero asignaciones por cue) y difiere el benchmark a la feature que introduzca la entrada real. |
| **V. Local primero y propiedad del usuario** | PASA | Sin red, sin telemetría, sin persistencia. El archivo MIDI se lee de disco local y no sale del dispositivo. |

**Restricción de contenido** (Constitución, sección Restricciones): los fixtures de prueba se
construyen sintéticamente en código, byte a byte. No se incluye ninguna obra musical de terceros
en el repositorio.

### Re-evaluación tras el diseño de la Fase 1

| Principio | Estado tras el diseño | Qué lo confirma |
| --- | --- | --- |
| **I. Precisión Musical** | PASA, reforzado | La prohibición de coma flotante en todo el crate y la acumulación en "µs × PPQ" eliminan la deriva de redondeo. El contrato de truncado es público y está cubierto por pruebas. |
| **II. TDD estricto** | PASA | El contrato y el modelo de datos fijan el comportamiento observable antes de existir una línea de implementación, que es la precondición para escribir la prueba primero. |
| **III. Núcleo desacoplado** | PASA, verificable | El quickstart incluye `cargo tree -p core` como comprobación: si aparece `tauri`, el principio está roto y se detiene la fusión. La dependencia es verificable, no confiada. |
| **IV. Tiempo real** | PARCIAL, sin cambios | Sigue en Complexity Tracking. El diseño protege el presupuesto: `advance_to` no asigna, no hace E/S y su coste es `k+1` comparaciones. |
| **V. Local primero** | PASA, reforzado | `load_smf` recibe `&[u8]`, no una ruta: el núcleo es incapaz por construcción de tocar el disco o la red. |

**Ningún gate nuevo se rompe con el diseño.** La única desviación sigue siendo el benchmark de
latencia, ya justificada y con deuda anotada.

## Project Structure

### Documentation (this feature)

```text
specs/001-midi-feedforward-harness/
├── plan.md              # Este archivo
├── spec.md              # Especificación (ya escrita)
├── research.md          # Fase 0
├── data-model.md        # Fase 1
├── quickstart.md        # Fase 1
├── contracts/           # Fase 1: contrato de la API pública del crate
├── checklists/
│   └── requirements.md  # Checklist de calidad de la spec (16/16)
└── tasks.md             # Fase 2 (/speckit-tasks, no lo crea este comando)
```

### Source Code (repository root)

```text
core/                          # NUEVO: crate de dominio, headless
├── Cargo.toml
├── src/
│   ├── lib.rs                 # API pública del crate
│   ├── clock.rs               # Fuente de tiempo sustituible (virtual y monótona)
│   ├── midi/
│   │   ├── mod.rs
│   │   └── loader.rs          # Bytes MIDI -> eventos crudos, con errores tipados
│   ├── tempo.rs               # Mapa de tempo: ticks <-> tiempo real
│   ├── timeline.rs            # Notas programadas, emparejamiento y orden total
│   └── feedforward.rs         # Reproducción y emisión de cues
└── tests/
    ├── fixtures.rs            # Constructores de SMF sintéticos en memoria
    ├── timeline_test.rs       # Historia 1: carga y línea temporal
    ├── feedforward_test.rs    # Historia 2: anticipación
    └── determinism_test.rs    # Historia 3: reproducibilidad

src-tauri/                     # EXISTE, no se toca en esta feature
src/                           # EXISTE (UI), no se toca en esta feature
Cargo.toml                     # Se modifica: añadir "core" a members
```

**Structure Decision**: el crate `core` vive en la raíz del workspace, hermano de `src-tauri`, no
dentro de él. Es una exigencia directa del Principio III: el dominio debe compilar y probarse sin
arrastrar Tauri. `src-tauri` dependerá de `core` en una feature posterior; en esta entrega no hay
ninguna arista entre ambos, lo que garantiza que el núcleo no puede haberse contaminado.

Los fixtures viven en `core/tests/fixtures.rs` como constructores de bytes, no como archivos
`.mid` binarios en el repositorio: así el fixture es legible en la revisión de código y su
intención queda explícita.

## Complexity Tracking

| Violation | Why Needed | Simpler Alternative Rejected Because |
|-----------|------------|-------------------------------------|
| Principio IV: no se entrega el benchmark de latencia de 30 ms exigido por la Constitución | La ruta crítica que ese presupuesto gobierna (tecla pulsada → evaluación → feedback) no existe en esta entrega: no hay captura MIDI ni interfaz. Medir 30 ms hoy sería medir un camino vacío y daría una falsa señal de verde en CI. | Escribir el benchmark ya, contra el harness feedforward, fue descartado porque mediría algo distinto de lo que el principio protege y quedaría obsoleto en cuanto entre el teclado real. En su lugar, este plan impone la propiedad estructural que hace alcanzable ese presupuesto (SC-006: coste proporcional a los cues emitidos) y la verifica con una prueba. **Deuda explícita**: la feature que introduzca la entrada MIDI real MUST entregar el benchmark. |
| No se entrega configuración de CI (la Constitución exige que la suite pase en Windows y macOS antes de fusionar) | El repositorio es local y todavía no tiene remoto configurado, por lo que no hay dónde ejecutar CI. | Añadir un workflow de CI a ciegas fue descartado porque no se puede verificar que funcione sin remoto. **Deuda explícita**: debe montarse en cuanto el repositorio tenga remoto, y antes de la primera fusión de terceros. |
