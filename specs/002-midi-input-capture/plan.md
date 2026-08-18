# Implementation Plan: Captura MIDI del teclado

**Branch**: `002-midi-input-capture` | **Date**: 2026-08-17 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `/specs/002-midi-input-capture/spec.md`

## Summary

Una capa de entrada que lee lo que el alumno toca en un teclado MIDI físico y lo entrega al núcleo
con margen de sobra, más el banco de medición que convierte el presupuesto de 30 ms en una puerta
real en lugar de una aspiración.

Tres decisiones sostienen el diseño, y las tres salieron de verificar en vez de suponer:

1. **No se usa `midir`, la biblioteca obvia.** Aborta el proceso ante un paquete truncado, en la
   ruta exacta de note-on, y el pánico cruza una frontera FFI: `catch_unwind` no lo contiene. Se
   habla directamente con CoreMIDI y con WinMM, y el análisis de mensajes es propio, bajo el lint
   que hace **imposible por construcción** esa clase de fallo. El detonante fue el pánico, pero lo
   que inclinó la balanza es que `midir` tampoco notifica desconexiones: había que escribir código
   por plataforma de todas formas.
2. **El instante lo pone nuestro reloj, no el del sistema operativo.** Los sellos del sistema no
   son portables: en macOS cuentan desde el arranque de la máquina en microsegundos, en Windows
   desde `midiInStart` en milisegundos. Usarlos habría exigido una fórmula de alineación por
   plataforma con deriva. Al sellar con el reloj de sesión compartido, **el desfase con la
   reproducción es cero por construcción**: no hay nada que alinear.
3. **El consumidor duerme y lo despiertan, no sondea.** El despertar cuesta 37,5 µs en el
   percentil 95: el 0,125 % del presupuesto. Un sondeo cada 16 ms se habría comido la mitad del
   presupuesto sin tocar nada.

## Technical Context

**Language/Version**: Rust 1.97.1, edition 2021.

**Primary Dependencies**:

| Crate | Dónde | Para qué |
| --- | --- | --- |
| `coremidi 0.9.2` (MIT) | `midi-io/`, solo macOS | Enumerar, abrir, recibir paquetes y **notificaciones de conexión** |
| `windows` (MIT/Apache-2.0) | `midi-io/`, solo Windows | WinMM y `CM_Register_Notification` |
| `rtrb 0.3.4` (MIT/Apache-2.0) | `core/` | Cola acotada sin bloqueo entre el callback y el consumidor |

El núcleo suma **una** dependencia (`rtrb`) y ninguna de sistema. `midir` quedó descartado con
evidencia reproducida: ver research.md, Decisión 1.

**Storage**: una única preferencia en disco: el teclado elegido la última vez, como la pareja
(nombre del puerto, posición entre homónimos) que fija FR-004a. Nada más. Lo capturado vive en
memoria durante la sesión, conforme a las Assumptions de la spec.

**Testing**: `cargo test` con una fuente de eventos sustituible, más un banco de latencia separado
que no forma parte de la suite (ver Constitution Check).

**Target Platform**: macOS (arm64 y x86_64) y Windows 10+.

**Project Type**: desktop-app. Esta feature añade la capa de entrada de hardware y su banco de
medición.

**Performance Goals**:

- Entrega de una pulsación al consumidor en < 30 ms p95, despertar del consumidor incluido
  (SC-001).
- Cero pérdidas en ráfagas de 50 eventos/s durante un minuto (SC-002).
- Detección de desconexión en < 2 s (SC-007).
- Sin degradación en sesiones de 10 minutos: el p95 del último minuto no supera al del primero en
  más de un 10 % (SC-008).

**Constraints**:

- La ruta crítica MUST NOT asignar memoria, hacer E/S, ni bloquearse.
- Determinismo ante una fuente controlada (SC-004).
- La suite completa MUST pasar en una máquina sin ningún teclado conectado (SC-005).
- Sin red, sin telemetría.

**Scale/Scope**: un solo alumno, un solo dispositivo abierto a la vez, sesiones de hasta unas
horas.

## Alcance de la interfaz: qué entrega esta feature y qué no

La spec habla de que "el usuario consulta los dispositivos" y "elige uno". Eso describe una
**capacidad**, no necesariamente una pantalla. Este plan la resuelve así:

- **Dentro de alcance**: la capacidad completa —enumerar dispositivos con nombre legible,
  identificarlos por la pareja (nombre, posición), abrir el elegido, capturar, detectar la
  desconexión— expuesta como API, más los comandos que la hacen accesible desde la aplicación.
- **Fuera de alcance**: la pantalla de selección propiamente dicha. Llega con la feature de
  visualización, que es la que introduce interfaz de verdad. Construir aquí una pantalla suelta,
  sin el resto de la aplicación alrededor, produciría algo que habría que rehacer.

La consecuencia práctica: esta feature se valida por completo desde pruebas, sin abrir ventana.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principio | Estado | Cómo lo cumple este plan |
| --- | --- | --- |
| **I. Precisión Musical Primero** | PASA | Los instantes de captura se derivan del reloj de sesión compartido, nunca del reloj de la interfaz. El emparejamiento de pulsación con suelta reutiliza la política ya fijada en la feature 001 (FIFO por voz), de modo que lo tocado y lo esperado son comparables por construcción. |
| **II. TDD estricto** | PASA | Toda la lógica se ejerce a través de una fuente de eventos sustituible, así que cada comportamiento tiene prueba antes que implementación. La única parte no cubierta por pruebas automáticas es la que habla con el sistema operativo, deliberadamente reducida al mínimo (ver Complexity Tracking). |
| **III. Núcleo desacoplado de la UI** | PASA, con tensión resuelta en el diseño | Es la decisión central de esta feature. El núcleo define **qué** es una fuente de eventos y no sabe **cómo** se obtienen; la implementación que habla con el hardware vive fuera de él. La comprobación es mecánica: `cargo tree` del núcleo no puede mostrar ninguna dependencia de sistema. |
| **IV. Tiempo real con presupuesto** | **PASA — la deuda se salda aquí** | La feature 001 dejó anotado que el benchmark de 30 ms no se podía entregar porque la ruta crítica no existía. Aquí existe. Este plan entrega la medición, su umbral y el mecanismo que bloquea la fusión al superarlo. |
| **V. Local primero** | PASA | Sin red, sin telemetría. El único dato que toca el disco es qué teclado se eligió la última vez. Lo que el alumno toca no se guarda ni sale del dispositivo. |

### La deuda del Principio IV, y qué cuenta como saldarla

La feature 001 registró en su `Complexity Tracking` que no entregaba el benchmark exigido, con el
compromiso explícito de que **la feature que introdujera la entrada real debía entregarlo**. Es
ésta. Para considerarla saldada, este plan exige tres cosas, no una:

1. Que exista una medición automatizada del recorrido completo definido en FR-017.
2. Que se ejecute sin intervención manual y **sin teclado conectado**, porque si no, no correrá en
   integración continua y volverá a ser una promesa.
3. Que **falle** y bloquee la incorporación del cambio al superar el umbral. Una medición que solo
   informa no es una puerta.

Un benchmark que cumpla 1 y 2 pero no 3 deja la deuda abierta con mejor aspecto.

## Project Structure

### Documentation (this feature)

```text
specs/002-midi-input-capture/
├── plan.md              # Este archivo
├── spec.md              # Especificación (34 requisitos, 5 clarificaciones)
├── research.md          # Fase 0
├── data-model.md        # Fase 1
├── quickstart.md        # Fase 1
├── contracts/           # Fase 1
├── checklists/
│   └── requirements.md  # Checklist de calidad (16/16)
└── tasks.md             # Fase 2 (/speckit-tasks)
```

### Source Code (repository root)

```text
core/                              # EXISTE. Suma un módulo; sigue sin dependencias de sistema
└── src/capture/
    ├── mod.rs
    ├── fuente.rs                  # trait FuenteDeEventos (genérico) + FuenteGuionizada
    ├── dispositivo.rs             # identidad: id del sistema (primaria) + (nombre, posición)
    ├── evento.rs                  # EventoCrudo de 16 bytes, PulsacionCapturada, Cierre
    ├── transporte.rs              # cola rtrb acotada, descarte contado, despertar por unpark
    ├── emparejador.rs             # emparejado, cierres y contadores de lo tolerado
    └── sesion.rs                  # estados, doble confirmación de pérdida

midi-io/                           # NUEVO. La única capa sin cobertura automática
├── Cargo.toml
└── src/
    ├── lib.rs                     # MidiIoSource<C: Clock>: implementa FuenteDeEventos
    ├── parser.rs                  # bytes -> nota. ~60 líneas, deny(indexing_slicing)
    ├── macos.rs                   # coremidi: puertos, apertura, bucle de paquetes
    ├── windows.rs                 # WinMM + CM_Register_Notification  (PENDIENTE, ver T042)
    └── vigia.rs                   # sondeo de 1000 ms; notificaciones como acelerador

bench/                             # NUEVO. Fuera de `cargo test`
├── Cargo.toml
└── src/bin/latencia.rs            # n=3000, k=3, puertas de 1 ms y 30 ms, códigos de salida

src-tauri/                         # EXISTE. Crea el reloj de sesión UNA vez y lo comparte
Cargo.toml                         # members += ["midi-io", "bench"]
```

**Structure Decision**: la frontera entre `core/` y `midi-io/` es la que hace cumplible el
Principio III, y se verifica de forma mecánica: `cargo tree -p piano-core` debe dar **exactamente
tres líneas** (`piano-core`, `midi_file` y `rtrb`) en los tres targets, con un grep negativo contra
`coremidi|midir|windows|winapi|core-foundation|objc2|libc|alsa|jack`. Si alguien añade una
dependencia de sistema al núcleo, la integración continua se rompe y hay que justificarlo en el PR.

Todo lo que tiene lógica —emparejar, cerrar, contar, ordenar— vive en `core/` y se prueba sin
hardware. `midi-io/` solo abre el puerto, recorre el paquete, filtra a notas, sella y empuja: sin
ninguna decisión de dominio. Es deliberadamente aburrido, porque es lo que no podemos probar.

### Re-evaluación tras el diseño de la Fase 1

| Principio | Estado tras el diseño | Qué lo confirma |
| --- | --- | --- |
| **I. Precisión Musical** | PASA, reforzado | Al controlar el bucle de paquetes se lee el reloj **una vez por paquete**: un acorde tiene un instante único por construcción. Con `midir` habrían salido tres instantes distintos, con dispersión medida de hasta 53,8 µs. |
| **II. TDD estricto** | PASA | Toda la lógica vive tras `FuenteDeEventos` y se ejerce con `FuenteGuionizada`. Lo no cubierto es `midi-io/`, y el diseño lo mantiene sin decisiones propias precisamente por eso. |
| **III. Núcleo desacoplado** | PASA, verificable mecánicamente | La puerta de `cargo tree` con tres líneas exactas y grep negativo, en tres targets. |
| **IV. Tiempo real** | **PASA — deuda saldada** | Banco entregado, con las tres condiciones: existe, corre sin teclado, y **falla** bloqueando la fusión. Además el diseño evita los dos bloqueos que traía `midir`: el mutex en el callback de tiempo real y las asignaciones. |
| **V. Local primero** | PASA | Sin red. El único dato en disco es qué teclado se eligió. |

**Un gate nuevo aparece con el diseño y hay que declararlo**: el banco de CI mide en torno al
**0,11 %** del recorrido que percibe el alumno. El resto —barrido de teclas del instrumento,
transporte USB, despacho del driver— queda fuera y no es observable desde la aplicación. El banco
imprime esa advertencia en cada ejecución, para que su número no se lea como lo que no es.

## Complexity Tracking

| Violation | Why Needed | Simpler Alternative Rejected Because |
|-----------|------------|-------------------------------------|
| Una porción del código —el backend de Windows y su vigía— no queda cubierta por pruebas automáticas | Ningún runner de integración continua tiene un piano enchufado, y no hay máquina Windows en el entorno de desarrollo. | Simular el sistema operativo entero fue descartado: se acabaría probando el simulador, no el sistema. **La excepción se estrechó al revisar el plan**: `coremidi` permite crear teclados virtuales, así que el adaptador de macOS **sí** se ejerce sin hardware (T036a–T036f) y sale de esta excepción. Lo que queda es la rama de Windows. La mitigación del resto es estructural: esa capa se reduce a abrir el puerto y reenviar bytes, sin ninguna decisión propia. **Archivos acogidos a la excepción, tras la implementación**: únicamente `midi-io/src/windows.rs`. Todo lo demás de `midi-io/` acabó cubierto: `parser.rs`, `macos.rs` y `vigia.rs` se ejercen contra teclados virtuales de CoreMIDI (`macos_virtual_test.rs`, `vigia_test.rs`, `parser_test.rs`). La condición (3) de la excepción —probar donde la plataforma lo permita— se cumple, y la excepción quedó reducida a un archivo que aún no existe. Esta lista es parte de la revisión y no se amplía sin discutirlo. |
| La deuda del Principio IV se salda **parcialmente**: el banco existe, corre sin teclado y falla, pero sin remoto git quien bloquea es un hook `pre-push` local, evitable con `--no-verify` | No hay remoto configurado ni infraestructura de integración continua, y crear uno es una decisión del proyecto, no un detalle técnico que este plan pueda tomar por su cuenta. | Dejar T053 y T059 apuntando a una integración continua inexistente fue descartado: habrían quedado como tareas imposibles de completar, y la feature que existe **para** saldar esta deuda la habría dejado abierta fingiendo lo contrario. En su lugar, `scripts/verificar.sh` concentra las cuatro puertas en un solo comando y `ci.yml` se versiona ya, inerte, para que el día que haya remoto no haya nada que inventar. **Deuda restante, acotada a una acción**: activar el remoto. |
| La ruta de Windows no ha sido ejecutada ni una sola vez: todo lo planificado sobre ella es lectura de código, documentación oficial e issues públicos | No hay ninguna máquina Windows en el entorno de desarrollo, y la compilación cruzada verifica que compila, no que funcione. | Planificar solo macOS fue descartado: la Constitución fija Windows como plataforma objetivo y descubrir los problemas al final es peor. **Deuda explícita y acotada**: un trabajo de un día en una máquina Windows real, como **primer** trabajo del lado Windows, que valide en este orden: enumeración y apertura, notificaciones de conexión, cierre tras retirada del dispositivo sin cuelgue (Microsoft KB4460006 documenta un cuelgue irrecuperable), y la cifra de latencia. Hasta entonces, ninguna afirmación sobre Windows en estos documentos debe leerse como medida. |
| El banco de latencia no vive dentro de `cargo test` | La suite completa debe seguir por debajo de 1 segundo (SC-002 de la feature 001, ya verificado en 60 ms). Un benchmark honesto necesita cientos de muestras y descarte de calentamiento: metido en la suite, la haría inutilizable como bucle de desarrollo. | Meterlo en la suite con menos muestras fue descartado porque un p95 sobre pocas muestras es ruido, y un benchmark intermitente se acaba desactivando, que es la peor deuda posible: la puerta sigue ahí pero ya no protege nada. |
