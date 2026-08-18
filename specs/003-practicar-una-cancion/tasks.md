# Tasks: Practicar una canción

**Feature**: `003-practicar-una-cancion` | **Fecha**: 2026-08-18
**Entrada**: [spec.md](./spec.md), [plan.md](./plan.md), [research.md](./research.md),
[data-model.md](./data-model.md), [contracts/practica-api.md](./contracts/practica-api.md)

## Dos notas sobre el orden

**TDD.** Cada tarea de implementación va precedida por la de su prueba. Una tarea de prueba no está
completa si la prueba pasa nada más escribirla.

**La excepción, y es una sola.** El único archivo acogido a la excepción del Principio II
(Constitución v1.1.0) es **`src/practica/Lienzo.tsx`**, y solo la merece mientras cumpla su
condición: **no decidir nada**. Recibe una escena ya calculada y la pinta. Si empieza a necesitar
un `if` sobre algo musical, ese `if` va a `piano-core`. Todo lo demás de esta feature —incluido el
código TypeScript que interpola entre anclas— sí se prueba.

---

## Phase 1: Setup

- [ ] T001 Crear el módulo `core/src/practica/mod.rs` con sus submódulos vacíos y reexportarlo desde `core/src/lib.rs`
- [ ] T002 [P] Crear `core/src/digitacion/mod.rs` y reexportarlo desde `core/src/lib.rs`
- [ ] T003 [P] Añadir el binario `bench/src/bin/fotogramas.rs` al manifiesto de `bench/Cargo.toml`
- [ ] T004 Verificar que `cargo tree -p piano-core` sigue dando exactamente tres líneas. **Esta feature no añade ninguna dependencia**; si aparece una cuarta, algo se ha colado

**Checkpoint**: `./scripts/verificar.sh` en verde con los módulos vacíos.

---

## Phase 2: Foundational

**Bloquea todas las historias.** Sin la escena y sin el puente no se puede ver ni probar nada.

### La escena: lo que el núcleo produce y la pantalla consume

- [ ] T005 Prueba en `core/tests/vista_test.rs`: `vista()` devuelve solo las notas cuyo intervalo se solapa con la ventana pedida, y ninguna más
- [ ] T006 Implementar `NotaVisible`, `EstadoNota` y `vista(cancion, desde, hasta, out)` en `core/src/practica/vista.rs`, escribiendo en un `Vec` del llamante
- [ ] T007 Prueba en `core/tests/vista_test.rs`: el recorte usa un cursor monótono con cota superior de duración, y su coste **no** crece con el tamaño de la canción — se cuentan notas examinadas, no milisegundos
- [ ] T008 Implementar el recorte con cursor monótono en `core/src/practica/vista.rs`
- [ ] T009 Prueba de presupuesto en `core/tests/vista_presupuesto_test.rs`: producir la escena de un fotograma con una pieza densa (40 notas visibles, 10.000 en total) cabe holgadamente en 16,7 ms. **Es la puerta de rendimiento que sí puede bloquear un cambio**, porque es determinista y no necesita pantalla

### El puente

- [ ] T010 Implementar el estado gestionado y `registrar_canal` en `src-tauri/src/comandos.rs`, con **un solo `Channel`** por sesión para eventos de tecla y anclas, discriminados por etiqueta
- [ ] T011 Implementar el hilo reenviador en `src-tauri/src/reenviador.rs`, que drena el anillo con `Receptor::esperar()` y empuja por el canal. **Nunca desde el hilo de tiempo real**: `send` cuesta hasta 13 ms en el peor caso y eso rompería el presupuesto que la feature 002 dejó cerrado
- [ ] T012 Implementar `MensajeAlFrontend` en `src-tauri/src/comandos.rs` conforme al contrato

### La capa que pinta

- [ ] T013 Sustituir el andamio de Vite: `src/App.tsx`, `src/practica/` y el estilo base
- [ ] T014 Implementar `pintar(ctx, escena)` en `src/practica/Lienzo.tsx`. **Único archivo acogido a la excepción del Principio II.** Sin sombras, sin desenfoques y sin filtros: está medido que `shadowBlur` hunde la cadencia a 40,9 fotogramas por segundo mientras el cronómetro interno sigue marcando 0,7 ms
- [ ] T015 Prueba en `src/practica/modelo.test.ts`: la interpolación entre anclas devuelve la posición correcta en instantes intermedios, y es exacta en los extremos
- [ ] T016 Implementar la interpolación entre anclas en `src/practica/modelo.ts`. **Sí se prueba**: no forma parte de la excepción

**Checkpoint**: se puede pintar una escena fija y el cálculo de la escena tiene puerta de rendimiento.

---

## Phase 3: User Story 1 — Abrir una canción y verla (P1) 🎯 MVP

**Objetivo**: el alumno elige un `.mid` y ve la pieza en pantalla, con nombres y dedos.

**Prueba independiente**: abrir un archivo y comprobar que aparece con cada nota en su sitio, sin teclado.

### Nombres de nota

- [ ] T017 [US1] Prueba en `core/tests/nombres_test.rs`: con armadura de sostenidos la tecla 61 es Do♯; con armadura de bemoles es Re♭; sin armadura declarada, sostenidos
- [ ] T018 [US1] Prueba en `core/tests/nombres_test.rs`: el mapa de armaduras por tick toma la última con tick ≤ t, igual que hace el mapa de tempo; varias en el mismo tick, gana la última
- [ ] T019 [US1] Implementar el mapa de armaduras y `NombreDeNota { base, alteracion }` en `core/src/practica/nombres.rs`. **Valor simbólico, nunca una cadena**: el formateo pertenece a quien pinta
- [ ] T020 [US1] Prueba en `core/tests/nombres_test.rs`: una tecla blanca nunca lleva alteración (no hay Mi♯ ni Do♭), que es la simplificación declarada

### Reparto de manos

- [ ] T021 [US1] Prueba en `core/tests/manos_test.rs`: las tres guardas. Un archivo con dos voces del mismo instrumento, cada una con ≥5 % de las notas y medianas separadas ≥3 semitonos, se considera con manos separadas; si falla **cualquiera** de las tres, no
- [ ] T022 [US1] Prueba en `core/tests/manos_test.rs`: con manos separadas, la derecha es la voz de **mediana de altura más alta**, nunca la de índice de pista menor. Incluye un archivo donde la pista 0 es la mano izquierda
- [ ] T023 [US1] Implementar `Voz`, las tres guardas y la asignación por mediana en `core/src/practica/manos.rs`
- [ ] T024 [US1] Prueba en `core/tests/manos_test.rs`: sin manos separadas se reparte por altura con umbral 60, y mover el umbral reasigna las notas afectadas y **solo** ésas
- [ ] T025 [US1] Implementar el corte por altura ajustable en `core/src/practica/manos.rs`
- [ ] T026 [US1] Prueba en `core/tests/manos_test.rs`: el reparto es determinista en 100 ejecuciones, incluido el desempate por `(track, channel)`

### Digitación

- [ ] T027 [US1] Prueba en `core/tests/digitacion_test.rs`: **el vano canónico se mide del dedo menor al mayor**. Para el par (3,1) con intervalo ascendente de +3 semitonos el vano es −3. Sin esto el paso del pulgar no se detecta jamás, así que es la primera prueba que hay que escribir
- [ ] T028 [US1] Implementar la tabla de vanos de Parncutt como datos en `core/src/digitacion/tablas.rs`, con `#[rustfmt::skip]`
- [ ] T029 [US1] Prueba en `core/tests/digitacion_test.rs`: la mano izquierda es la derecha reflejada (`h(p) = −p`), pero el color de la tecla se consulta sobre la altura MIDI **real**, que no se refleja
- [ ] T030 [US1] Implementar la reflexión de mano y la consulta de color de tecla en `core/src/digitacion/mod.rs`
- [ ] T031 [US1] Prueba en `core/tests/digitacion_test.rs`: las doce reglas de coste, una por una, con los pares que cada una penaliza y con los que no
- [ ] T032 [US1] Implementar la función de coste con las doce reglas en `core/src/digitacion/coste.rs`, en aritmética `i32` exclusivamente
- [ ] T033 [US1] Prueba en `core/tests/digitacion_test.rs`: **la escala de Do mayor de una octava**, ascendente y descendente, para las dos manos, debe dar la digitación canónica que enseña cualquier método (SC-011)
- [ ] T034 [US1] Implementar la programación dinámica exacta de segundo orden en `core/src/digitacion/mod.rs`
- [ ] T035 [US1] Prueba en `core/tests/digitacion_test.rs`: los acordes reparten dedos de una misma mano sin repetir ninguno
- [ ] T036 [US1] Implementar la digitación de acordes en `core/src/digitacion/mod.rs`
- [ ] T037 [US1] Prueba en `core/tests/digitacion_test.rs`: **toda** nota de cualquier canción cargable recibe dedo (SC-009); cuando no hay buena solución se propone la menos mala, nunca ninguna
- [ ] T038 [US1] Prueba en `core/tests/digitacion_test.rs`: la misma canción produce la misma digitación en 100 ejecuciones (SC-010)
- [ ] T039 [US1] Prueba de rendimiento en `core/tests/digitacion_test.rs`: 5.000 notas se digitan con holgura dentro del presupuesto de 2 s de SC-002

### Abrir y ver

- [ ] T040 [US1] Implementar `abrir_cancion(ruta)` en `src-tauri/src/comandos.rs`: lee el archivo del disco y llama a `load_smf`. **Es la capa de aplicación quien lee del disco**, no el núcleo, que sigue recibiendo `&[u8]`
- [ ] T041 [US1] Implementar el selector de archivos y el estado de carga en `src/App.tsx`
- [ ] T042 [US1] Implementar el dibujo del teclado de 88 teclas y de las notas con sus etiquetas en `src/practica/Lienzo.tsx`
- [ ] T043 [US1] Implementar el control del punto de corte de manos en `src/practica/controles.tsx`, **siempre visible**, con «usar las voces del archivo» por defecto cuando se detecten
- [ ] T044 [US1] Prueba en `core/tests/manos_test.rs`: mover el corte recalcula manos **y digitación** (FR-003c), no solo el color
- [ ] T045 [US1] Implementar el recálculo encadenado en `core/src/practica/manos.rs`
- [ ] T046 [US1] Implementar el aviso de archivo ilegible en `src/App.tsx`, mostrando el motivo que devuelve `LoadError` sin dejar la aplicación en un estado a medias

**Checkpoint**: US1 completa. Se abre un `.mid` y se ve, con nombres y dedos. Es el MVP.

---

## Phase 4: User Story 2 — Reproducir y seguirla con la vista (P1)

**Objetivo**: la canción avanza a tempo y el alumno la sigue; puede pausar, saltar y cambiar velocidad.

**Prueba independiente**: `cargo test -p piano-core cursor` en verde, sin pantalla.

- [ ] T047 [US2] Prueba en `core/tests/cursor_test.rs`: `Velocidad` es racional; reducir a la mitad y volver a normal deja la posición **exactamente** donde correspondía, sin deriva acumulada tras 10.000 pasos
- [ ] T048 [US2] Implementar `Velocidad` y `Avance` en `core/src/practica/cursor.rs`, sin coma flotante
- [ ] T049 [US2] Prueba en `core/tests/cursor_test.rs`: en `PorReloj` la posición avanza según reloj y velocidad; pausar la detiene y reanudar continúa sin salto
- [ ] T050 [US2] Implementar `Cursor` con anclas (`ancla_real`, `ancla_cancion`) y el rebase al cambiar de régimen en `core/src/practica/cursor.rs`
- [ ] T051 [US2] Prueba en `core/tests/cursor_test.rs`: cambiar de velocidad a mitad de reproducción **no** provoca salto de posición (FR-010)
- [ ] T052 [US2] Prueba en `core/tests/cursor_test.rs`: `saltar_a` deja el cursor en la posición pedida, sin notas colgando y con el modo intacto (FR-007b)
- [ ] T053 [US2] Implementar `saltar_a` y el reinicio de estado en `core/src/practica/cursor.rs`
- [ ] T054 [US2] Prueba en `core/tests/cursor_test.rs`: la canción termina y se comunica una sola vez (FR-011)
- [ ] T055 [US2] Implementar `SesionDePractica<C: Clock, F: FuenteDeEventos>` y `avanzar() -> Paso` en `core/src/practica/sesion.rs`
- [ ] T056 [US2] Prueba en `core/tests/cursor_test.rs`: el ancla solo se emite al **cambiar de régimen**, no en cada avance. Es lo que mantiene el puente vacío
- [ ] T057 [US2] Implementar el transporte (`marcha`, `pausa`, `saltar`, `velocidad`) en `src-tauri/src/comandos.rs`
- [ ] T058 [US2] Implementar los controles de transporte y velocidad en `src/practica/controles.tsx`
- [ ] T059 [US2] Implementar el bucle de dibujo con `requestAnimationFrame` en `src/practica/Lienzo.tsx`, derivando la posición **del reloj y nunca del número de fotograma**: es lo que hace que la cadencia de la pantalla afecte a la suavidad pero no a la corrección

**Checkpoint**: la canción se reproduce, se pausa, se salta y se ralentiza.

---

## Phase 5: User Story 3 — Tocar y que la aplicación responda (P1)

**Objetivo**: las teclas pulsadas se ven, y se distingue lo que la canción pedía de lo que no.

**Prueba independiente**: `cargo test -p piano-core sonando` en verde, con fuente guionizada.

- [ ] T060 [US3] Prueba en `core/tests/sonando_test.rs`: una nota está sonando si la posición cae entre su ataque y su final, **sin ninguna ventana de tolerancia** (FR-014b)
- [ ] T061 [US3] Implementar `ConjuntoSonando` en `core/src/practica/sonando.rs`, con cursor de entrada y cota superior de duración, porque la línea temporal está ordenada por ataque y no por final
- [ ] T062 [US3] Prueba en `core/tests/sonando_test.rs`: las tres situaciones de FR-014a se distinguen — acierto, nota extra y nota omitida
- [ ] T063 [US3] Implementar la clasificación de las tres situaciones en `core/src/practica/sonando.rs`
- [ ] T064 [US3] Prueba en `core/tests/sonando_test.rs`: el coste de la consulta no crece con el tamaño de la canción; se cuentan notas examinadas
- [ ] T065 [US3] Conectar la captura a la sesión en `src-tauri/src/lib.rs`, pasando **el mismo reloj de sesión** que ya se crea una sola vez (FR-012a)
- [ ] T066 [US3] Implementar el reflejo de teclas pulsadas en `src/practica/Lienzo.tsx`
- [ ] T067 [US3] Prueba en `src/practica/modelo.test.ts`: el estado de teclas pulsadas que llega por el canal se aplica en orden y una tecla soltada deja de estar marcada
- [ ] T068 [US3] Implementar el manejo de mensajes del canal en `src/practica/modelo.ts`
- [ ] T069 [US3] Implementar el aviso de «sin teclado» en `src/App.tsx`, que **no debe bloquear** ver ni reproducir la canción (FR-015)
- [ ] T070 [US3] Implementar el aviso de dispositivo perdido a mitad de práctica en `src/App.tsx` (FR-016)

**Checkpoint**: el instrumento y la aplicación están conectados.

---

## Phase 6: User Story 4 — Practicar en modo espera (P2)

**Objetivo**: la canción espera a que el alumno acierte, sin que el ritmo desaparezca.

**Prueba independiente**: `cargo test -p piano-core espera` en verde, con fuente guionizada.

- [ ] T071 [US4] Prueba en `core/tests/espera_test.rs`: con `PorAcierto`, el cursor avanza **a tempo** entre notas y se detiene al llegar a una pendiente. El tiempo entre notas transcurre de verdad (FR-018a)
- [ ] T072 [US4] Implementar `ProgramaDePuertas` precalculado en `core/src/practica/puertas.rs`, recorrido con cursor monótono
- [ ] T073 [US4] Implementar el techo móvil del avance en `core/src/practica/cursor.rs`: en `PorAcierto` el reloj gobierna hasta la puerta pendiente
- [ ] T074 [US4] Prueba en `core/tests/espera_test.rs`: una nota equivocada **no** hace avanzar el cursor, y se comunica sin interrumpir (FR-019)
- [ ] T075 [US4] Prueba en `core/tests/espera_test.rs`: un acorde avanza solo con **todas** sus notas pulsadas a la vez; acertarlas una tras otra soltando entre medias **no** basta (FR-022)
- [ ] T076 [US4] Implementar `MascaraTeclas` y la comprobación de acorde completo en `core/src/practica/cursor.rs`, con `and` de máscaras
- [ ] T077 [US4] Prueba en `core/tests/espera_test.rs`: con una mano elegida, tocar notas de la otra **no** hace avanzar el cursor (SC-012)
- [ ] T078 [US4] Implementar el filtrado por mano practicada en `core/src/practica/puertas.rs`
- [ ] T079 [US4] Prueba en `core/tests/espera_test.rs`: cambiar de modo a mitad de canción conserva la posición y no deja notas colgando (FR-021)
- [ ] T080 [US4] Implementar el cambio de modo en caliente en `core/src/practica/sesion.rs`
- [ ] T081 [US4] Prueba en `core/tests/espera_test.rs`: si la canción pide una nota que el teclado no tiene, existe una salida y el modo espera no se queda atascado para siempre (FR-020)
- [ ] T082 [US4] Implementar la salida del atasco en `core/src/practica/cursor.rs` y su control en `src/practica/controles.tsx`
- [ ] T083 [US4] Implementar el selector de modo y de mano en `src/practica/controles.tsx`
- [ ] T084 [US4] Implementar el indicador visual de nota pendiente en `src/practica/Lienzo.tsx`

**Checkpoint**: un principiante puede recorrer entera una pieza que no domina.

---

## Phase 7: User Story 5 — Elegir y recordar el teclado (P3)

**Objetivo**: la pantalla que la feature 002 aplazó a propósito.

**Prueba independiente**: la lista muestra los dispositivos y la elección sobrevive al reinicio.

- [ ] T085 [US5] Implementar el comando de enumerar dispositivos en `src-tauri/src/comandos.rs`, sobre `piano_midi_io::dispositivos()`, que ya existe y está probado
- [ ] T086 [US5] Implementar el selector en `src/dispositivos/Selector.tsx`, mostrando nombre y posición para poder distinguir homónimos
- [ ] T087 [US5] Conectar la persistencia con `src-tauri/src/preferencias.rs`, ya construido y probado en la feature 002
- [ ] T088 [US5] Implementar en `src/dispositivos/Selector.tsx` la propuesta automática del teclado recordado al arrancar, y la petición de elegir de nuevo cuando no case (FR-025). El reconocimiento por identidad ya existe en `core/src/capture/dispositivo.rs`: aquí solo se usa

**Checkpoint**: elegir teclado es un trámite de una sola vez.

---

## Phase 8: Polish

- [ ] T089 Implementar el banco de fotogramas en `bench/src/bin/fotogramas.rs`: abre ventana real, mide y publica las **cinco cifras** de SC-003 a SC-003d
- [ ] T090 Implementar en `bench/src/bin/fotogramas.rs` la detección de suspensiones del sistema (intervalos > 200 ms), su exclusión del cálculo y su **declaración en el informe**. Sin esto el informe publica un número inventado: en la primera medición se perdieron 430 de 600 segundos por esta causa
- [ ] T091 Ejecutar el banco de fotogramas con una pieza densa y registrar las cinco cifras en `specs/003-practicar-una-cancion/quickstart.md`, con fecha y máquina
- [ ] T092 [P] Documentar con rustdoc la API pública de `core/src/practica/` y `core/src/digitacion/`
- [ ] T093 [P] Verificar `cargo clippy --workspace --all-targets -- -D warnings` limpio y `pnpm build` sin avisos
- [ ] T094 [P] Revisar `src/practica/Lienzo.tsx` archivo a archivo y mudar a `piano-core` cualquier decisión que se haya colado. Es la condición que sostiene su excepción constitucional
- [ ] T095 Verificar que `cargo tree -p piano-core` sigue dando exactamente tres líneas
- [ ] T096 Actualizar `Complexity Tracking` en `plan.md` con el resultado real del banco de fotogramas

---

## Dependencias

```
Phase 1 ──► Phase 2 ──► Phase 3 (US1) ──► Phase 4 (US2) ──► Phase 5 (US3) ──► Phase 6 (US4)
                                │                                                    │
                                └──────────────► Phase 7 (US5) ◄─────────────────────┘
                                                       │
                                                       └──► Phase 8 (Polish)
```

- **US2 depende de US1**: no se reproduce lo que no se ha cargado ni se ve.
- **US3 depende de US2**: el reflejo de teclas necesita una posición contra la que compararse.
- **US4 depende de US3**: el modo espera necesita saber si lo tocado coincide.
- **US5 es independiente** de US2, US3 y US4; solo necesita que la aplicación exista.
- **T009 (puerta de presupuesto) bloquea la Fase 8**: no se mide fluidez sin haber acotado antes el
  coste del cálculo.

## Oportunidades de paralelismo

Marcadas con `[P]`. Pocas, porque el TDD estricto serializa casi todo.

- **Phase 1**: T002 y T003 tocan archivos distintos.
- **Phase 3**: los tres bloques —nombres, manos, digitación— son independientes entre sí hasta T040.
- **Phase 8**: T092, T093 y T094 no dependen entre sí.

## Estrategia de entrega

- **MVP = Fases 1 a 3.** En ese punto se abre un `.mid` y se ve en pantalla con nombres y dedos: es
  la primera vez que el proyecto tiene algo que enseñar.
- **Fases 4 y 5** cierran el bucle: la canción suena y el instrumento responde.
- **Fase 6** es la que hace que un principiante llegue al final de una pieza.
- **Fases 7 y 8** pulen.

## Resumen

| Fase | Tareas | De ellas, pruebas |
| --- | --- | --- |
| 1. Setup | T001–T004 (4) | 0 |
| 2. Foundational | T005–T016 (12) | 5 |
| 3. US1 (P1) | T017–T046 (30) | 15 |
| 4. US2 (P1) | T047–T059 (13) | 6 |
| 5. US3 (P1) | T060–T070 (11) | 5 |
| 6. US4 (P2) | T071–T084 (14) | 6 |
| 7. US5 (P3) | T085–T088 (4) | 0 |
| 8. Polish | T089–T096 (8) | 0 |
| **Total** | **96** | **37** |

**Sin cobertura automática, y declarado**: `src/practica/Lienzo.tsx` (T014, T042, T059, T066, T084)
y el banco de fotogramas (T089–T091), que necesita pantalla. Nada más.
