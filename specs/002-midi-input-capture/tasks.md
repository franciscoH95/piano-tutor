# Tasks: Captura MIDI del teclado

**Feature**: `002-midi-input-capture` | **Fecha**: 2026-08-18
**Entrada**: [spec.md](./spec.md), [plan.md](./plan.md), [research.md](./research.md),
[data-model.md](./data-model.md), [contracts/capture-api.md](./contracts/capture-api.md)

## Dos notas sobre el orden, antes de empezar

**TDD.** La Constitución lo impone como principio no negociable, así que cada tarea de
implementación va precedida por la de su prueba. Una tarea de prueba no está completa si la prueba
pasa nada más escribirla.

**La excepción, declarada a propósito y estrechada.** Parte de `midi-io/` no tiene prueba
automática delante: ningún runner de integración continua tiene un piano enchufado. Pero la
excepción es **más pequeña de lo que parecía**: `coremidi` permite crear teclados virtuales, así
que el adaptador de macOS sí se ejerce sin hardware (T036a–T036f). Lo que queda realmente
descubierto es la rama de Windows. La mitigación del resto es estructural: esa capa no toma ni una
decisión de dominio. **Si alguna tarea de `midi-io/` empieza a necesitar lógica, es señal de que
esa lógica está en el archivo equivocado y debe mudarse a `core/`.**

---

## Phase 1: Setup

- [X] T001 Crear el crate `midi-io` con `cargo new --lib midi-io` (paquete `piano-midi-io`, biblioteca `piano_midi_io`) y añadirlo a `members` en `Cargo.toml` (raíz)
- [X] T002 [P] Crear el crate `bench` con `cargo new bench` (paquete `piano-bench`) y añadirlo a `members` en `Cargo.toml` (raíz)
- [X] T003 Declarar `rtrb = "0.3.4"` en `core/Cargo.toml`, única dependencia nueva del núcleo
- [X] T004 [P] Declarar las dependencias por plataforma en `midi-io/Cargo.toml`: `[target.'cfg(target_os = "macos")'.dependencies] coremidi = "0.9.2"` y `[target.'cfg(windows)'.dependencies] windows = { version = "...", features = [...] }`
- [X] T005 [P] Replicar los lints del núcleo en `midi-io/src/lib.rs`: `#![forbid(unsafe_code)]`, `#![deny(clippy::float_arithmetic)]`, `#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::indexing_slicing)]`. **`indexing_slicing` es el que hace imposible el fallo que descalificó a `midir`**
- [X] T006 Añadir a `.gitignore` lo que genere el banco, si genera algo, y verificar que `cargo check --workspace` sigue en verde
- [X] T006a Crear `scripts/verificar.sh` que ejecute en orden, abortando al primer fallo: `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, la puerta del Principio III y el banco de latencia. Un solo comando, un solo código de salida
- [X] T006b [P] Crear `scripts/instalar-hooks.sh` que enlace `scripts/verificar.sh` como hook `pre-push`, y documentarlo en `README.md`. `.git/hooks/` no se versiona, así que sin este script la puerta no existe en una clonación nueva
- [X] T006c [P] Escribir `.github/workflows/ci.yml` que ejecute `scripts/verificar.sh` en `macos-latest` y `windows-latest`. Queda inerte hasta que haya remoto; se versiona ya para que el día que lo haya no haya que inventarlo

**Checkpoint**: `cargo tree -p piano-core` muestra exactamente tres líneas (`piano-core`, `midi_file` y `rtrb`).

---

## Phase 2: Foundational

**Bloquea todas las historias.** Sin el transporte ni la fuente controlada no se puede escribir
ninguna prueba de las demás.

- [X] T007 Prueba en `core/tests/capture_evento_test.rs`: `size_of::<EventoCrudo>() == 16` y `align == 8`; construir y comparar eventos; `Copy` sin `Drop`
- [X] T008 Implementar `EventoCrudo` (`#[repr(C)]`, campos `at`, `seq`, `key`, `velocity`, `kind`, `channel`) y `TipoEvento` en `core/src/capture/evento.rs`
- [X] T009 Prueba en `core/tests/transporte_test.rs`: un evento emitido se recoge idéntico; el orden se conserva; recoger de una cola vacía devuelve cero
- [X] T009a Prueba en `core/tests/transporte_test.rs`: dos eventos que comparten instante salen en un orden **definido y estable** —el de llegada, desempatado por `seq`— y ese orden es el mismo en 100 ejecuciones. Es el caso real de un acorde que llega en un solo paquete (FR-010, SC-003)
- [X] T010 Implementar `canal(capacidad)`, `Emisor::emitir` y `Receptor::recoger` sobre `rtrb::RingBuffer` en `core/src/capture/transporte.rs`
- [X] T011 Prueba en `core/tests/transporte_test.rs`: al llenar la cola se descarta **lo entrante** (lo ya almacenado sigue intacto y en orden), el contador de descartes cuadra exactamente, y el `seq` deja un hueco que localiza dónde se perdió (FR-011b, FR-011c)
- [X] T012 Implementar la política de desbordamiento y el contador en `core/src/capture/transporte.rs`
- [X] T013 Prueba en `core/tests/transporte_test.rs`: emitir **no asigna memoria**, verificado con un asignador instrumentado que cuenta llamadas, ni siquiera en el primer evento
- [X] T013a Prueba en `core/tests/transporte_test.rs`: con la cola **llena**, `emitir` retorna sin bloquearse —se mide que la llamada termina en un orden de magnitud muy por debajo de cualquier espera— y no toma ningún cerrojo. FR-020 prohíbe esperas bloqueantes, no solo asignaciones
- [X] T014 Ajustar `core/src/capture/transporte.rs` hasta que la prueba de cero asignaciones pase: reservar de una vez al crear el canal, nunca dentro de `emitir`
- [X] T015 Prueba en `core/tests/transporte_test.rs`: el consumidor duerme y lo despierta el productor; si el `unpark` llega antes del `park`, el `park` retorna igualmente y **no se pierde el aviso**
- [X] T016 Implementar el despertar con `park`/`unpark` y el testigo `quiere_despertar` en `core/src/capture/transporte.rs`
- [X] T016a Prueba en `core/tests/transporte_test.rs`: si el reloj devolviese un instante anterior al último sellado, el evento se sella con el último (clamp no decreciente) y se cuenta; los instantes entregados nunca decrecen (FR-013)
- [X] T016b Implementar el clamp de monotonía y su contador en `core/src/capture/transporte.rs`, con `debug_assert!` de que en condiciones normales nunca dispara
- [X] T017 [P] Prueba en `core/tests/fuente_test.rs`: `FuenteGuionizada` entrega los eventos del guion en orden y con los instantes exactos del guion; `agotada()` pasa a `true` al terminar
- [X] T018 Implementar el trait `FuenteDeEventos` (genérico, nunca `dyn`) y `FuenteGuionizada` en `core/src/capture/fuente.rs`

**Checkpoint**: `cargo test -p piano-core` en verde; el transporte funciona sin hardware.

---

## Phase 3: User Story 1 — Que el sistema reciba lo que el alumno toca (P1) 🎯 MVP

**Objetivo**: lo que se toca en el teclado llega al núcleo como pulsaciones con principio y final.

**Prueba independiente**: `cargo test -p piano-core capture` en verde, sin teclado conectado.

### Identidad del dispositivo

- [ ] T019 [US1] Prueba en `core/tests/dispositivo_test.rs`: el reconocimiento prueba primero el identificador del sistema y solo después la pareja (nombre, posición) (FR-004b)
- [ ] T020 [US1] Prueba en `core/tests/dispositivo_test.rs`: si no casa ninguno de los dos criterios, el resultado es "pedir al usuario que elija", **nunca** el dispositivo más parecido (FR-004c)
- [ ] T021 [US1] Prueba en `core/tests/dispositivo_test.rs`: dos dispositivos con el mismo nombre se distinguen por su posición; un nombre vacío recibe una etiqueta generada y sigue siendo elegible
- [ ] T021a [US1] Prueba en `core/tests/dispositivo_test.rs`: con dos dispositivos presentes y uno elegido, **solo llegan eventos del elegido**. Ninguno se autoselecciona y los flujos no se fusionan (FR-004)
- [ ] T022 [US1] Implementar `Dispositivo`, `DeviceId` y la función de reconocimiento en `core/src/capture/dispositivo.rs`
- [ ] T022a [US1] Prueba en `core/tests/dispositivo_test.rs`: los tres modos de fallo —no hay ningún teclado, el elegido no se pudo abrir, el dispositivo está en uso por otra aplicación— producen variantes distintas de `ErrorDeEntrada`, todas comunicables sin interrumpir la aplicación (FR-005)
- [ ] T022b [US1] Implementar `ErrorDeEntrada` con esas variantes en `core/src/capture/dispositivo.rs` y mapear los códigos de la plataforma en `midi-io/src/macos.rs`

### Análisis de mensajes

- [ ] T023 [US1] Prueba en `midi-io/tests/parser_test.rs`: un note-on de tres bytes produce un ataque; un note-off produce una suelta; un note-on con velocity cero produce una **suelta** (FR-009)
- [ ] T024 [US1] Prueba en `midi-io/tests/parser_test.rs`: **el caso que descalificó a `midir`**. Los paquetes truncados `[0x90]`, `[0x90,0x3C]`, `[0x80,0x3C]`, `[0xB0]`, `[0xC0]`, `[0xE0,0x00]` y `[0x90,0x3C,0x64,0x90]` (nota válida seguida de byte de estado suelto) **no entran en pánico**: se ignoran o se reportan
- [ ] T025 [US1] Prueba en `midi-io/tests/parser_test.rs`: pedal (CC64), aftertouch, cambio de instrumento y reloj se descartan sin interrumpir las notas que llegan entremezcladas (FR-014); el estado en carrera (*running status*) se interpreta correctamente
- [ ] T026 [US1] Implementar el analizador en `midi-io/src/parser.rs`, sin indexar ni un solo slice sin comprobar: bajo `deny(clippy::indexing_slicing)` el compilador lo impide

### Emparejamiento y cierres

- [ ] T027 [US1] Prueba en `core/tests/emparejador_test.rs`: un ataque seguido de su suelta produce una `PulsacionCapturada` con `Cierre::PorSuelta` y los instantes correctos
- [ ] T028 [US1] Prueba en `core/tests/emparejador_test.rs`: el caso canónico `on(60)@0, on(60)@10, off(60)@20, off(60)@30` empareja **FIFO**, la misma política que la feature 001
- [ ] T029 [US1] Implementar el emparejador con tabla plana de 2.048 ranuras indexada por (canal, altura) en `core/src/capture/emparejador.rs`, sin asignar y con coste constante
- [ ] T030 [US1] Prueba en `core/tests/emparejador_test.rs`: la misma tecla pulsada dos veces sin soltarse cierra la primera con `Cierre::PorRepulsacion`
- [ ] T031 [US1] Prueba en `core/tests/emparejador_test.rs`: una suelta sin ataque previo se ignora y suma en `sueltas_sin_pulsacion` (FR-016)
- [ ] T032 [US1] Prueba en `core/tests/emparejador_test.rs`: `cerrar(at, PorParada)` cierra las teclas hundidas en el instante de la parada y las etiqueta; ninguna se descarta ni recibe duración inventada (FR-015)
- [ ] T033 [US1] Implementar `Cierre`, `cerrar` y los contadores en `core/src/capture/emparejador.rs`
- [ ] T034 [US1] Prueba en `core/tests/emparejador_test.rs`: la misma altura en canales distintos son notas independientes y no se cierran entre sí
- [ ] T035 [US1] Prueba en `core/tests/informe_test.rs`: notas fuera de las 88 teclas y mensajes descartados se cuentan; una sesión limpia deja el informe entero a cero
- [ ] T036 [US1] Implementar `InformeDeCaptura` y `PulsacionCapturada` en `core/src/capture/informe.rs` y `core/src/capture/evento.rs`

### Adaptador de macOS

**Sí lleva pruebas delante.** `coremidi::Client::virtual_source()` permite crear un teclado
sintético en el propio sistema, así que este adaptador se ejerce sin hardware. La excepción de
cobertura queda reducida a la rama de Windows.

- [ ] T036a [US1] Prueba en `midi-io/tests/macos_virtual_test.rs` (`#![cfg(target_os = "macos")]`): crear una fuente virtual con `coremidi::Client::virtual_source()` y comprobar que aparece en la enumeración con su nombre y su `unique_id`
- [ ] T036b [US1] Prueba en `midi-io/tests/macos_virtual_test.rs`: abrir la fuente virtual, enviarle un note-on y un note-off, y comprobar que el adaptador entrega exactamente esos dos eventos, con altura e intensidad correctas
- [ ] T036c [US1] Prueba en `midi-io/tests/macos_virtual_test.rs`: enviar un acorde de tres notas **en un solo paquete** y comprobar que las tres reciben **el mismo instante**. Es la propiedad que se gana al controlar el bucle de paquetes (research.md, Decisión 3), y la que `midir` impedía
- [ ] T036d [US1] Prueba en `midi-io/tests/macos_virtual_test.rs`: enviar por la fuente virtual los paquetes truncados de T024 y comprobar que **el proceso sobrevive**. Es la regresión que protege contra el fallo exacto que descalificó a `midir`
- [ ] T036e [US1] Prueba en `midi-io/tests/macos_virtual_test.rs`: tras cerrar la captura, enviar 500 mensajes por la fuente virtual y exigir **cero** entregas al consumidor (FR-006)
- [ ] T036f [US1] Prueba en `midi-io/tests/macos_virtual_test.rs`: desde la llamada de apertura hasta que el primer evento es entregable transcurre menos de 1 segundo (SC-006)
- [ ] T037 [US1] Implementar la enumeración de dispositivos en `midi-io/src/macos.rs` con `coremidi`, leyendo `display_name()` y `unique_id()` de cada fuente
- [ ] T038 [US1] Implementar la apertura del puerto y el bucle de paquetes en `midi-io/src/macos.rs`. **Leer el reloj UNA sola vez por paquete** y asignar ese instante a todas las notas del paquete: es lo que da a un acorde un instante único (research.md, Decisión 3)
- [ ] T039 [US1] Implementar el cierre que libera el dispositivo para otras aplicaciones (FR-006) en `midi-io/src/macos.rs`
- [ ] T040 [US1] Implementar `MidiIoSource<C: Clock>`, que implementa `FuenteDeEventos`, en `midi-io/src/lib.rs`
- [ ] T041 [US1] Escribir el ejemplo manual `midi-io/examples/escuchar.rs`: enumera, abre el elegido y muestra por consola lo que se toca. Es la única forma de ejercer esta capa

### Adaptador de Windows

- [ ] T042 [US1] **Spike de validación en máquina Windows real. BLOQUEADO: no hay máquina Windows disponible hoy.** La rama de Windows (T042–T044, T067) queda **suspendida**, no en curso: no se empieza y no cuenta como pendiente de trabajo, sino como pendiente de hardware. El resto de fases avanza sin ella. Es el **primer** trabajo del lado Windows y bloquea a los demás. Validar en este orden: enumeración y apertura; `CM_Register_Notification`; cierre tras retirada del dispositivo **sin cuelgue** (Microsoft KB4460006 documenta uno irrecuperable); y una cifra de latencia. Registrar los hallazgos en `specs/002-midi-input-capture/research.md` bajo las incertidumbres U2 y U4
- [ ] T043 [US1] Implementar enumeración, apertura, bucle de mensajes y cierre en `midi-io/src/windows.rs` con WinMM, aplicando lo aprendido en T042
- [ ] T044 [US1] Verificar que `cargo check --target x86_64-pc-windows-msvc` pasa y que `midi-io/src/lib.rs` selecciona el backend correcto por `cfg`

**Checkpoint**: US1 completa. Una pulsación real llega al núcleo como `PulsacionCapturada`.

---

## Phase 4: User Story 2 — Que llegue lo bastante rápido (P1)

**Objetivo**: saldar la deuda del Principio IV con una puerta que de verdad bloquea.

**Prueba independiente**: `cargo run -p piano-bench --release --bin latencia` termina con código 0.

- [ ] T045 [US2] Implementar el arnés de medición en `bench/src/bin/latencia.rs`: t0 justo antes de publicar en el anillo, t1 tras despertar de un bloqueo real **y** tras decodificar al tipo de dominio (D4). El despertar cuenta
- [ ] T046 [US2] Implementar el muestreo en `bench/src/bin/latencia.rs`: n = 3.000 muestras a 1 ms, 500 de calentamiento descartadas, k = 3 repeticiones tomando el **mínimo de los p95**
- [ ] T047 [US2] Implementar las dos puertas y los códigos de salida en `bench/src/bin/latencia.rs`: 1 ms p95 (capa) y 30 ms p95 (constitucional); salida `0` correcto, `1` supera la de capa, `2` supera la constitucional, `3` error de ejecución
- [ ] T048 [US2] Implementar en `bench/src/bin/latencia.rs` el informe que se imprime **en cada ejecución**: la tabla de tramos no medidos y la frase de alcance. **El banco cubre en torno al 0,11 % del recorrido que percibe el alumno**, y sin esa advertencia el número se lee como lo que no es
- [ ] T049 [US2] Implementar el campo `DELTA_SO_USB` en el informe de `bench/src/bin/latencia.rs`, que imprime `SIN CALIBRAR` mientras no exista una medición con hardware real. Decirlo es más honesto que estimarlo
- [ ] T050 [US2] Implementar el modo `--con-hardware` en `bench/src/bin/latencia.rs`, que mide desde el sello del propio sistema operativo. Su diferencia con el número de CI es exactamente el tramo no cubierto
- [ ] T050a [US2] Implementar el modo `--sostenido` en `bench/src/bin/latencia.rs`: diez minutos de eventos a ritmo realista, comparando el p95 del último minuto con el del primero; falla si la degradación supera el 10 % (SC-008). **No entra en `scripts/verificar.sh`** por duración: se ejecuta a mano o en una tarea nocturna
- [ ] T051 [US2] **Calibrar el umbral en el runner real**: ejecutar el banco 20 veces en el entorno de integración continua y fijar la puerta de capa **con el dato en la mano**, nunca por corazonada. Documentar el valor y su fecha en `bench/README.md`. Los números de referencia se midieron en un M1 Max de 10 núcleos, no en un runner compartido de 2–4 vCPU
- [ ] T052 [US2] Configurar el hilo consumidor con prioridad elevada (QoS *user-interactive* en macOS, por encima de normal en Windows) en `core/src/capture/transporte.rs`. Sin ello la cola de latencia la pone el planificador, no el código: p999 medido de 2,65 ms a prioridad normal
- [ ] T053 [US2] Incorporar el banco a `scripts/verificar.sh` con su código de salida, de modo que un fallo aborte la verificación. **Alcance real**: mientras no exista remoto, quien bloquea es el hook `pre-push` local, que se salta con `--no-verify`. La puerta del Principio IV **no queda cerrada del todo** hasta que `ci.yml` corra en un remoto. Una medición que solo informa no es una puerta

**Checkpoint**: la deuda del Principio IV queda saldada con sus tres condiciones: existe, corre sin teclado y falla.

---

## Phase 5: User Story 3 — Que siga siendo verificable sin teclado (P2)

**Objetivo**: demostrar que toda la funcionalidad se ejerce sin hardware, y que es reproducible.

**Prueba independiente**: la suite entera pasa en una máquina sin ningún dispositivo MIDI.

- [ ] T054 [US3] Prueba en `core/tests/determinismo_captura_test.rs`: alimentar el emparejador 100 veces con el mismo guion produce 100 resultados idénticos, informe incluido (SC-004)
- [ ] T055 [US3] Prueba en `core/tests/determinismo_captura_test.rs`: un guion sucio a propósito (sueltas huérfanas, repulsaciones, teclas hundidas al final, mensajes no-nota) también es reproducible bit a bit
- [ ] T056 [US3] Prueba en `core/tests/carga_test.rs`: una ráfaga de 50 eventos por segundo durante un minuto simulado no pierde ninguna pulsación y deja el contador de descartes en cero (SC-002)
- [ ] T057 [US3] Prueba en `core/tests/carga_test.rs`: con el consumidor detenido a propósito hasta desbordar, la memoria no crece, nada se bloquea y el contador refleja exactamente cuántas se perdieron (SC-002a)
- [ ] T058 [US3] Ajustar lo que haga falta en `core/src/capture/` para que las pruebas anteriores pasen sin relajarlas: nada de `HashMap` en rutas que afecten al orden, nada de reloj del sistema fuera del `Clock` inyectado
- [ ] T059 [US3] Añadir la puerta del Principio III a `scripts/verificar.sh`: `cargo tree -p piano-core` debe dar **exactamente tres líneas** en los tres targets, más un grep negativo contra `coremidi|midir|windows|winapi|core-foundation|objc2|libc|alsa|jack`

**Checkpoint**: la suite completa pasa en una máquina limpia, sin hardware.

---

## Phase 6: User Story 4 — Que enchufar y desenchufar no rompa la sesión (P3)

**Objetivo**: la desaparición del teclado se comunica y no pierde nada de lo capturado.

**Prueba independiente**: `cargo test -p piano-core sesion` en verde, simulando la pérdida.

- [ ] T060 [US4] Prueba en `core/tests/sesion_test.rs`: al declarar la pérdida, las teclas hundidas se cierran con `Cierre::PorPerdidaDeDispositivo`, selladas en el instante del **último evento recibido** —no en el de la detección, que llega más tarde— y marcadas con `duracion_censurada`
- [ ] T061 [US4] Prueba en `core/tests/sesion_test.rs`: todo lo capturado antes de la pérdida se conserva íntegro (SC-007)
- [ ] T062 [US4] Implementar la máquina de estados de `SesionDeCaptura` (Inactiva, Abriendo, Capturando, Perdida, Error) en `core/src/capture/sesion.rs`
- [ ] T063 [US4] Prueba en `core/tests/sesion_test.rs`: la pérdida **nunca** se infiere del silencio; solo la declaran una notificación explícita o una doble ausencia en la enumeración
- [ ] T064 [US4] Implementar la regla de doble confirmación en `core/src/capture/sesion.rs`
- [ ] T065 [US4] Implementar el vigía de macOS en `midi-io/src/vigia.rs` con `coremidi::Client::new_with_notifications`. **Debe crearse antes que cualquier otro cliente MIDI del proceso**, en el hilo cuyo `CFRunLoop` corre; en Tauri, el principal. Es una dependencia de orden invisible que no da error si se rompe
- [ ] T066 [US4] Añadir una aserción de arranque que compruebe que se reciben notificaciones antes de continuar, y el comentario `// NO tocar MIDI antes de esta línea` en `src-tauri/src/lib.rs`
- [ ] T067 [US4] Implementar el vigía de Windows en `midi-io/src/vigia.rs` con `CM_Register_Notification` y `GUID_DEVINTERFACE_MIDI_INPUT`, aplicando lo aprendido en T042
- [ ] T068 [US4] Implementar el sondeo de respaldo cada 1.000 ms en `midi-io/src/vigia.rs`, que cumple el requisito de 2 segundos con doble confirmación aunque la notificación no llegue
- [ ] T069 [US4] Implementar la reapertura tras reconexión en `midi-io/src/lib.rs`, con **ventana de cortesía**: si tras reabrir no llega ningún evento, informar al usuario en lugar de fingir que funciona. `microsoft/MIDI` #906 documenta exactamente ese fallo en Windows
- [ ] T070 [US4] Implementar el reconocimiento del dispositivo reaparecido reutilizando la función de T022 (identificador primero, pareja de reserva después) en `midi-io/src/lib.rs`
- [ ] T071 [US4] Prueba manual con hardware, con el procedimiento anotado en `specs/002-midi-input-capture/quickstart.md`: desconectar el teclado a mitad de captura, comprobar que se comunica en menos de 2 segundos, reconectar y comprobar que la captura se reanuda sin reiniciar

**Checkpoint**: las cuatro historias completas.

---

## Phase 7: Polish

- [ ] T072 [P] Documentar con rustdoc la API pública de `core/src/capture/` y de `midi-io/src/lib.rs`, conforme a `contracts/capture-api.md`
- [ ] T073 [P] Verificar `cargo clippy --workspace --all-targets -- -D warnings` limpio, sin `#[allow]` nuevos sin justificar en comentario
- [ ] T074 [P] Verificar que `midi-io/src/` sigue sin ninguna decisión de dominio: revisar archivo por archivo y mudar a `core/` cualquier lógica que se haya colado
- [ ] T075 Verificar que la suite completa sigue por debajo de 1 segundo y registrar el dato en `specs/002-midi-input-capture/quickstart.md`
- [ ] T076 Persistir la elección de teclado en `src-tauri/src/preferencias.rs`: el identificador del sistema **y** la pareja (nombre, posición). Es el único dato de esta feature que toca el disco
- [ ] T077 Crear el reloj de sesión **una sola vez** en `src-tauri/src/lib.rs` y pasárselo tanto a la captura como a la reproducción (FR-012a). Dos relojes arrancados por separado darían un desfase constante que nadie sabría explicar
- [ ] T078 Actualizar `Complexity Tracking` en `plan.md` con el resultado real del spike de Windows (T042), sustituyendo lo que hoy es lectura de código por lo que se haya medido

---

## Dependencias

```
Phase 1 (Setup) ──► Phase 2 (Foundational) ──► Phase 3 (US1) ──► Phase 4 (US2)
                                                     │                  │
                                                     ├──► Phase 5 (US3) │
                                                     │                  │
                                                     └──► Phase 6 (US4) │
                                                                 │      │
                                                                 └──────┴──► Phase 7 (Polish)
```

- **US2 depende de US1**: el banco mide hasta el evento **decodificado**, así que necesita el
  analizador y el emparejador.
- **US3 depende de US1**: verifica el determinismo de lo que US1 construye.
- **US4 depende de US1**: no se puede perder un dispositivo que no se ha abierto.
- **T006a–T006c (verificación) bloquean T053 y T059**: no se puede incorporar una puerta a un
  script que no existe.
- **T042 (spike de Windows) bloquea T043 y T067, y hoy está bloqueado él mismo**: no hay máquina
  Windows disponible. Esa rama queda **suspendida**, no en curso. El resto del plan avanza sin
  ella, y `cargo check --target x86_64-pc-windows-msvc` (T044) sigue verificando que al menos
  compila.

## Oportunidades de paralelismo

Marcadas con `[P]`. Son pocas a propósito: el TDD estricto serializa casi todo, porque cada
implementación depende de que su prueba exista y falle antes.

- **Phase 1**: T002, T004, T005, T006b y T006c tocan archivos distintos.
- **Phase 3**: la rama de macOS (T036a–T041) y la de Windows (T042–T044) son independientes entre
  sí una vez existe el analizador (T026).
- **Phase 7**: T072, T073 y T074 no dependen entre sí.

## Estrategia de entrega

- **MVP = Phase 1 + Phase 2 + Phase 3**. Incluye ya T036a–T036f, así que el MVP llega con el
  adaptador de macOS **verificado**, no solo escrito. En ese punto una pulsación real llega al núcleo como
  pulsación capturada, con su principio y su final.
- **Phase 4 es la que salda la deuda constitucional**, y por eso va inmediatamente después.
- Phase 5 y 6 endurecen; Phase 7 cierra los cabos con la aplicación.

## Resumen

| Fase | Tareas | De ellas, pruebas |
| --- | --- | --- |
| 1. Setup | T001–T006c (9) | 0 |
| 2. Foundational | T007–T018 (16) | 9 |
| 3. US1 (P1) | T019–T044 (35) | 21 |
| 4. US2 (P1) | T045–T053 (10) | 0 (el banco **es** la verificación) |
| 5. US3 (P2) | T054–T059 (6) | 4 |
| 6. US4 (P3) | T060–T071 (12) | 4 |
| 7. Polish | T072–T078 (7) | 0 |
| **Total** | **95** | **38** |

**Sin cobertura automática, y declarado**: T042–T044 (backend de Windows), T067 (vigía de Windows),
T069–T070 (reconexión) y T071 (prueba manual). El adaptador de macOS **sí queda cubierto** por
T036a–T036f contra fuentes virtuales, así que la excepción se reduce a la rama de Windows, que ya
está declarada como pendiente de validar en máquina real (T042).
