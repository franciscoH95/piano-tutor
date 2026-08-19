---
description: "Lista de tareas para evaluar la interpretación"
---

# Tasks: Evaluar la interpretación

**Feature**: `004-evaluar-interpretacion` | **Fecha**: 2026-08-19

**Input**: [spec.md](./spec.md), [plan.md](./plan.md), [research.md](./research.md),
[data-model.md](./data-model.md), [contracts/](./contracts/), [quickstart.md](./quickstart.md)

## Format: `[ID] [P?] [Story] Description`

- **[P]**: se puede hacer en paralelo con otras marcadas igual (archivos distintos, sin dependencias)
- **[US1]…[US4]**: a qué historia de usuario pertenece

## Las pruebas van primero, y no es opcional

El Principio II de la constitución es **no negociable**: primero la prueba, la prueba falla, después
la implementación mínima. Toda tarea de implementación va después de la prueba que la justifica, y
el orden está pensado para que se pueda seguir literalmente.

Esta feature **no usa la excepción acotada de adaptadores de plataforma**: todo el juicio es lógica
pura y se prueba entero, sin teclado ni ventana.

## Path Conventions

- Núcleo: `core/src/evaluacion/`, pruebas en `core/tests/`
- Puente: `src-tauri/src/comandos.rs`
- Interfaz: `src/evaluacion/`

---

## Phase 1: Setup

- [X] T001 Crear el módulo `core/src/evaluacion/mod.rs` y declararlo en `core/src/lib.rs`, vacío y compilando
- [X] T002 Crear `core/tests/fixtures/interpretaciones/mod.rs` con el constructor de interpretaciones grabadas: una canción, una lista de observaciones con sus instantes y el resultado esperado
- [X] T003 [P] Crear el archivo de pruebas `core/tests/estadistica_test.rs` con el andamiaje mínimo
- [X] T004 [P] Crear el archivo de pruebas `core/tests/emparejar_test.rs` con el andamiaje mínimo
- [X] T005 [P] Crear el archivo de pruebas `core/tests/evaluacion_test.rs` con el andamiaje mínimo

**Checkpoint**: el módulo existe y `cargo test` pasa sin probar nada todavía.

---

## Phase 2: Foundational (bloquea todo lo demás)

**Objetivo**: las piezas que las cuatro historias necesitan. Nada de esto es visible para el alumno.

### Las tolerancias, en un solo sitio

- [X] T006 Prueba en `core/tests/evaluacion_test.rs`: los tres niveles existen y sus ventanas de ataque están **anidadas** — permisivo ⊇ intermedio ⊇ exigente. Es lo que hace que SC-006 se cumpla por aritmética y no por vigilancia
- [X] T007 Implementar `Nivel` y `Tolerancias` en `core/src/evaluacion/tolerancias.rs`, con **todos** los umbrales de la feature: ventana de emparejamiento, ventana de ataque por nivel, cercanía del dedo que se escapa en tiempo y en semitonos, y los dos umbrales del desfase sistemático
- [X] T008 Prueba en `core/tests/evaluacion_test.rs`: **ningún umbral se lee fuera de `Nivel::tolerancias`**. Criterio comprobable: en `core/src/evaluacion/`, fuera de `tolerancias.rs`, **ningún literal entero mayor que 1.000** (el orden de magnitud de un microsegundo relevante) y **ningún literal con separador de millares**. El Principio I lo exige textualmente y es la clase de regla que se erosiona sola; «literales sospechosos» no era un criterio, y una prueba sin criterio pasa siempre

### La estadística, en enteros

- [X] T009 Prueba en `core/tests/estadistica_test.rs`: la mediana de una lista de enteros **con signo**, con número par e impar de elementos. Incluye `[-3, -2]`, donde la división entera de Rust trunca hacia cero y da `-2` en vez de `-3` si no se tiene cuidado
- [X] T010 Prueba en `core/tests/estadistica_test.rs`: los cuartiles y el recorrido intercuartílico, con listas de 1, 2, 3, 4 y 100 elementos
- [X] T011 Implementar mediana y cuartiles en `core/src/evaluacion/estadistica.rs`, **sin coma flotante**, con la regla de redondeo de negativos fijada y documentada
- [X] T012 Prueba en `core/tests/estadistica_test.rs`: el mismo conjunto en distinto orden de entrada da el mismo resultado (SC-008)

### La inversa del ancla

- [X] T013 Prueba en `core/tests/emparejar_test.rs`: `instante_de(ancla, posicion)` es la **inversa exacta** de `posicion_en`. Para 200 pares al azar deterministas, `posicion_en(ancla, instante_de(ancla, p))` devuelve exactamente `p`
- [X] T014 Prueba en `core/tests/emparejar_test.rs`: es el **techo** y no el suelo. Con num/den = 1/3 y una posición que no cae en frontera, el instante devuelto es el primero en que el cursor **alcanza o supera** esa posición, no el último en que está por debajo
- [X] T015 Prueba en `core/tests/emparejar_test.rs`: con velocidad de pausa (`num == 0`) devuelve `None` — el cursor no llega nunca— y con una posición por encima del tope, también
- [X] T016 Prueba en `core/tests/emparejar_test.rs`: **no desborda ni entra en pánico** con `num = u32::MAX` y una canción de 24 horas, y da el **mismo** valor en debug y en release
- [X] T017 Implementar `instante_de` en `core/src/evaluacion/emparejar.rs`: una sola división, `u128` en el intermedio, `try_from` a la salida y **ni un solo `as`**
- [X] T018 Prueba en `core/tests/emparejar_test.rs`: el centinela de «sin sellar» **no puede confundirse con un desfase válido**. Es la prueba que justifica prohibir `as`: `u64::MAX as i64` vale −1, que es un adelanto de un microsegundo dentro de cualquier tolerancia

### Qué es evaluable

- [X] T019 Prueba en `core/tests/evaluacion_test.rs`: `es_evaluable` descarta la percusión (canal 9), lo que cae fuera de las 88 teclas, y lo de la mano no practicada
- [X] T020 Implementar `es_evaluable` en `core/src/evaluacion/mod.rs` y **hacer que `ProgramaDePuertas::nuevo` la consuma también**, en vez de tener su propio filtro. Dos copias del criterio volverían a divergir, que es exactamente lo que ya pasó con la percusión
- [X] T021 Prueba en `core/tests/espera_test.rs`: las puertas y el evaluador coinciden **nota por nota** en qué es evaluable, sobre una pieza con percusión, notas fuera de rango y dos manos

**Checkpoint**: aritmética, estadística y criterio de alcance listos y probados. Todavía no se juzga nada.

---

## Phase 3: User Story 1 — Saber cómo me fue al terminar (P1) 🎯 MVP

**Objetivo**: el alumno toca y, al acabar, sabe cuántas acertó, cuántas se dejó y cuántas tocó de más.

**Prueba independiente**: `cargo test -p piano-core --test evaluacion_test` en verde, con
interpretaciones grabadas y sin nada enchufado.

### Casar ataques con sueltas

- [ ] T022 [US1] Prueba en `core/tests/emparejar_test.rs`: la captura entrega ataques y sueltas por separado; una `Pulsacion` se construye casándolos por altura, en orden
- [ ] T023 [US1] Prueba en `core/tests/emparejar_test.rs`: una tecla **todavía hundida** al cerrar la interpretación produce una pulsación con final **desconocido**, que no es cero. Y una suelta **sin ataque previo** —el alumno ya la tenía pulsada al empezar— se descarta sin romper nada
- [ ] T024 [US1] Implementar el casado de ataques y sueltas en `core/src/evaluacion/emparejar.rs`

### El emparejamiento en línea

- [ ] T025 [US1] Prueba en `core/tests/emparejar_test.rs`: el emparejamiento es **uno a uno** (FR-002). Se comprueba como biyección parcial: ninguna nota recibe dos pulsaciones y ninguna pulsación va a dos notas, sobre una interpretación con repeticiones y acordes
- [ ] T026 [US1] Prueba en `core/tests/emparejar_test.rs`: **una nota ya juzgada no cambia de veredicto** por lo que venga después (FR-004). Se toca una nota, se juzga, y después se toca algo que con visión de futuro habría cambiado el emparejamiento; el veredicto de la primera no se mueve
- [ ] T027 [US1] Prueba en `core/tests/emparejar_test.rs`: la misma tecla dos veces seguidas en la canción y el alumno la toca **una sola vez**. Se declara cuál de las dos recibe la pulsación y por qué, y el resultado es estable
- [ ] T028 [US1] Implementar el sellado del instante esperado al **cruzar** el ataque, con el ancla vigente, en `core/src/evaluacion/emparejar.rs`. Se sella una vez y **no se recalcula nunca**
- [ ] T029 [US1] Prueba en `core/tests/emparejar_test.rs`: el instante esperado **no depende de la cadencia de fotogramas**. El mismo guion con 1, 100 y 10.000 avances da los mismos desfases
- [ ] T029a [US1] Prueba en `core/tests/evaluacion_test.rs`: `Evaluador::observar` **no asigna memoria** por evento. Se cuenta con el contador de asignaciones que la feature 002 ya dejó montado, y con contador por hilo para que no lo ensucien las pruebas en paralelo — el fallo exacto que ya se corrigió una vez ahí
- [ ] T030 [US1] Implementar `Evaluador::observar` y `Evaluador::avanzar` con el emparejamiento en línea completo y todas las reglas de desempate explícitas, en `core/src/evaluacion/emparejar.rs` y `core/src/evaluacion/mod.rs`

### Las medidas

- [ ] T030a [US1] Prueba en `core/tests/evaluacion_test.rs`: para cada nota emparejada se registran **las tres** medidas — desfase de ataque con su signo (FR-005), diferencia de duración con su signo (FR-006) e intensidad (FR-007). El signo del desfase **es** la información: sin él no se distingue ir adelantado de ir atrasado, que es la mitad de FR-016
- [ ] T030b [US1] Prueba en `core/tests/evaluacion_test.rs`: la duración y la intensidad **se miden pero no alteran el veredicto** (FR-006). Una nota con el ataque perfecto y soltada enseguida sigue contando como acertada, y su diferencia de duración se comunica aparte
- [ ] T030c [US1] Prueba en `core/tests/evaluacion_test.rs`: una tecla **todavía hundida** al cerrar deja la diferencia de duración como **desconocida**, que no es cero. Cero significaría que la sostuvo exactamente lo escrito, y eso sería mentir
- [ ] T030d [US1] Implementar `Medida` y su registro en `core/src/evaluacion/emparejar.rs`

### El veredicto

- [ ] T031 [US1] Prueba en `core/tests/evaluacion_test.rs`: las seis clases de veredicto se distinguen — acertada, tocada fuera de tiempo, omitida, no intentada, fuera de alcance, y de más
- [ ] T031a [US1] Prueba en `core/tests/evaluacion_test.rs`: SC-013, el **dedo que se escapa**. Se roza la tecla contigua y acto seguido se toca la correcta: la contigua se clasifica como dedo que se escapa y **no** como nota de más equiparable a tocar un compás equivocado, y el acierto sigue contando como acierto
- [ ] T031b [US1] Prueba en `core/tests/evaluacion_test.rs`: la cercanía del dedo que se escapa **no se traga notas legítimas**. Si la contigua es una nota que la canción sí pide en ese instante, se empareja con ella y se cuenta como acierto, no como dedo escapado
- [ ] T031c [US1] Implementar la clasificación del dedo que se escapa en `core/src/evaluacion/mod.rs`, con su tolerancia en tiempo y en semitonos leída de `tolerancias.rs` (FR-010a, FR-011a)
- [ ] T032 [US1] Prueba en `core/tests/evaluacion_test.rs`: la omisión es un **vencimiento del cursor**, no un temporizador. En modo espera, una nota que el cursor está esperando **no vence** por mucho que pase el reloj real: es la nota que el alumno está a punto de acertar
- [ ] T031d [US1] Prueba en `core/tests/evaluacion_test.rs`: FR-009a, en **modo espera** las notas se evalúan y **los tiempos no**. La misma interpretación en modo espera y por reloj da los mismos recuentos de acertadas y omitidas, pero en modo espera **no** produce desfase ni medidas de ataque: no se puede llegar tarde a algo que te espera, y publicar ese número sería inventarlo
- [ ] T031e [US1] Prueba en `core/tests/evaluacion_test.rs`: si el alumno **cambia de modo a mitad** del intento, cada nota se evalúa según el régimen vigente **cuando se selló**, no según un único indicador del intento entero. Es lo que FR-004 obliga: una nota ya juzgada no se recalcula
- [ ] T032a [US1] Prueba en `core/tests/evaluacion_test.rs`: FR-013, un pasaje saltado con la salida del modo espera queda **no intentado** y **no** cuenta como fallado. Ejercita `Evaluador::saltar`
- [ ] T032b [US1] Prueba en `core/tests/evaluacion_test.rs`: SC-009, el porcentaje de aciertos se calcula **sobre lo que el alumno intentó**. Las no intentadas y las fuera de alcance quedan fuera del denominador; con 20 notas de las que 5 se saltaron, el 100 % de las 15 restantes da 100 %, no 75 %
- [ ] T033 [US1] Implementar el vencimiento, la clasificación y `Evaluador::saltar` en `core/src/evaluacion/mod.rs`
- [ ] T034 [US1] Prueba en `core/tests/evaluacion_test.rs`: SC-001, la interpretación perfecta da 20 acertadas, 0 omitidas y 0 de más **en los tres niveles**
- [ ] T035 [US1] Prueba en `core/tests/evaluacion_test.rs`: SC-002, cero observaciones se comunica como «no se tocó nada» y **no** como 0 % de aciertos
- [ ] T035a [US1] Prueba en `core/tests/evaluacion_test.rs`: FR-014a, **pausar, saltar y llegar al final cierran** la interpretación, y reanudar abre otra. Es la misma frontera que el cursor ya usa para cambiar de régimen, así que se comprueba contra ella y no contra un concepto nuevo
- [ ] T035b [US1] Prueba en `core/tests/evaluacion_test.rs`: FR-014b, una interpretación que **no llega al final** se evalúa igualmente sobre el tramo recorrido. Exigir un recorrido completo dejaría sin retorno al principiante, que casi nunca termina
- [ ] T036 [US1] Implementar `Resultado`, los recuentos y los límites de la interpretación en `core/src/evaluacion/resultado.rs`

### El desfase sistemático

- [ ] T037 [US1] Prueba en `core/tests/evaluacion_test.rs`: SC-003, las 20 notas 40 ms tarde dan 20 acertadas **y** aviso de desfase
- [ ] T038 [US1] Prueba en `core/tests/evaluacion_test.rs`: SC-004, las 20 notas 120 ms tarde dan el aviso de desfase **en vez de** 20 fallos sueltos
- [ ] T039 [US1] Prueba en `core/tests/evaluacion_test.rs`: con menos del mínimo de notas **no** se afirma que haya desfase sistemático. Con dos notas la mediana existe y no significa nada
- [ ] T040 [US1] Implementar la detección del desfase sistemático en `core/src/evaluacion/resultado.rs`
- [ ] T041 [US1] Prueba en `core/tests/evaluacion_test.rs`: SC-012, la misma interpretación a mitad de velocidad con los mismos desfases absolutos da el **mismo** número de aciertos. Si salen más, la tolerancia está escalando con el tempo y FR-008a está roto

### Determinismo (Principio I, no negociable)

- [ ] T041a [US1] Prueba en `core/tests/evaluacion_test.rs`: **SC-005**, la misma interpretación evaluada 100 veces da resultados **idénticos campo a campo**, incluidos los veredictos por nota y el desfase sistemático
- [ ] T041b [US1] Prueba en `core/tests/evaluacion_test.rs`: **SC-008**, ninguna medida ni recuento depende del **orden de llegada** de pulsaciones que ocurrieron en el mismo instante. Se evalúa la misma interpretación con las simultáneas permutadas de las seis formas posibles y los seis resultados coinciden
- [ ] T041c [US1] Prueba en `core/tests/evaluacion_test.rs`: FR-003 y FR-021, el resultado **no depende del perfil de compilación**. Los valores que podrían diferir entre debug y release —los que pasan por `u128` y `try_from`— se comprueban explícitamente, porque la misma entrada con dos salidas según cómo se compile es la violación más silenciosa del Principio I

### El puente y la pantalla

- [ ] T042 [US1] Prueba en `src-tauri/tests/contrato_canal_test.rs`: la forma exacta del JSON del resultado, con los nombres en **camello**, igual que el resto del puente
- [ ] T043 [US1] Implementar `evaluacion_ultimo` y el aplanado en `src-tauri/src/comandos.rs`. **Ninguna tolerancia cruza**: si la interfaz supiera lo que es una ventana de 60 ms, esa constante estaría en dos sitios
- [ ] T044 [US1] Prueba en `src/evaluacion/Resumen.test.tsx`: se muestran los tres recuentos; «no se tocó nada» se muestra como tal y nunca como 0 %
- [ ] T045 [US1] Prueba en `src/evaluacion/Resumen.test.tsx`: un resultado marcado como **parcial lo declara** (FR-015a, SC-011). Un resumen que calla que no se midieron los tiempos se lee como completo
- [ ] T046 [US1] Implementar `src/evaluacion/Resumen.tsx`
- [ ] T047 [US1] Conectar el resumen en `src/App.tsx` al cerrar una interpretación

**Checkpoint**: el alumno toca una pieza entera y ve cuántas acertó. **Es el MVP.**

---

## Phase 4: User Story 2 — Ver en qué parte de la pieza fallo (P2)

**Objetivo**: no un número, sino **dónde**. Un «80 %» no cambia lo que el alumno hace mañana.

**Prueba independiente**: la misma interpretación grabada, comprobando que cada acierto y cada fallo
queda situado en su posición y en su mano.

- [ ] T048 [US2] Prueba en `core/tests/evaluacion_test.rs`: cada veredicto queda asociado a su índice de nota, y una interpretación con todos los fallos en la segunda mitad los sitúa allí y no repartidos
- [ ] T049 [US2] Prueba en `core/tests/evaluacion_test.rs`: con una pieza a dos manos, el resultado separa la izquierda de la derecha (FR-018)
- [ ] T050 [US2] Implementar la localización y el recuento por mano en `core/src/evaluacion/resultado.rs`
- [ ] T051 [US2] Prueba en `core/tests/evaluacion_test.rs`: el veredicto del evaluador se refleja en `EstadoNota` de la vista, de modo que el pentagrama y el resumen **no puedan discrepar**
- [ ] T052 [US2] Cablear el veredicto a `EstadoNota` en `core/src/practica/vista.rs`, y **retirar la mitad juzgadora de `core/src/practica/sonando.rs`** (`tocada`, `informada`, `cursor_omision`, `registrar`, `omitidas`, `clasificar`, `Situacion`). Verificado que no tiene ningún llamador de producción; se conservan `MascaraTeclas` y `vigentes()`, que las puertas sí usan
- [ ] T053 [US2] Prueba en `src/evaluacion/Resumen.test.tsx`: el resumen distingue las dos manos
- [ ] T054 [US2] Implementar la vista por mano y por posición en `src/evaluacion/Resumen.tsx`

**Checkpoint**: el alumno sabe qué repasar mañana.

---

## Phase 5: User Story 3 — Ajustar cuánto se me exige (P2)

**Objetivo**: un principiante y alguien con años no se miden con la misma vara.

**Prueba independiente**: la misma interpretación grabada en dos niveles da resultados distintos y
coherentes.

- [ ] T055 [US3] Prueba en `core/tests/evaluacion_test.rs`: una nota 60 ms tarde cuenta acertada en el nivel permisivo y no en el exigente
- [ ] T056 [US3] Prueba en `core/tests/evaluacion_test.rs`: **SC-006 sobre un barrido**. Para 50 interpretaciones distintas, el permisivo nunca da menos aciertos que el intermedio, ni este menos que el exigente. Con dos ventanas separadas esto es aritmética, pero hay que comprobarlo por si alguien las vuelve a juntar
- [ ] T057 [US3] Prueba en `core/tests/evaluacion_test.rs`: **cambiar de nivel no cambia el emparejamiento**, solo el veredicto. Se compara la lista de emparejamientos entre niveles y debe ser idéntica
- [ ] T058 [US3] Implementar la selección de nivel en `core/src/evaluacion/mod.rs`
- [ ] T059 [US3] Implementar el mando `evaluacion_nivel` en `src-tauri/src/comandos.rs`
- [ ] T060 [US3] Prueba en `src/evaluacion/Resumen.test.tsx`: el selector de nivel emite el nivel elegido y refleja el vigente
- [ ] T061 [US3] Implementar el selector de nivel en `src/evaluacion/Resumen.tsx`

**Checkpoint**: la exigencia se ajusta a quien practica.

---

## Phase 6: User Story 4 — Repetir un pasaje y ver si mejoro (P3)

**Objetivo**: lo que sostiene una sesión de estudio real.

**Prueba independiente**: dos interpretaciones del mismo pasaje, una mejor, se ordenan sin ambigüedad.

- [ ] T062 [US4] Prueba en `core/tests/evaluacion_test.rs`: SC-010, de dos interpretaciones con la mitad de fallos una que otra, siempre se señala la mejor
- [ ] T063 [US4] Prueba en `core/tests/evaluacion_test.rs`: el orden es **total** (FR-020a). Para 100 pares, `comparar` siempre devuelve mayor, menor o igual; nunca «no se sabe». Y es transitivo: si a > b y b > c, entonces a > c
- [ ] T064 [US4] Prueba en `core/tests/evaluacion_test.rs`: el orden es **léxico**, no una puntuación con pesos. Una interpretación con un acierto más y peor ritmo gana; si se pudiera compensar, no sería léxico
- [ ] T065 [US4] Implementar `comparar` en `core/src/evaluacion/resultado.rs`
- [ ] T066 [US4] Prueba en `core/tests/evaluacion_test.rs`: dos intentos del **mismo tramo** se identifican como comparables, y dos de tramos distintos no
- [ ] T067 [US4] Implementar la identificación de tramo y la comparación con el intento anterior en `core/src/evaluacion/mod.rs`
- [ ] T068 [US4] Implementar `evaluacion_comparar_con_anterior` en `src-tauri/src/comandos.rs`
- [ ] T069 [US4] Prueba en `src/evaluacion/Resumen.test.tsx`: se muestra si este intento fue mejor que el anterior, y se dice cuándo no hay anterior con qué comparar
- [ ] T070 [US4] Implementar la comparación en `src/evaluacion/Resumen.tsx`

**Checkpoint**: repetir un pasaje tiene sentido.

---

## Phase 7: Polish

### Los fixtures de referencia (FR-022)

- [ ] T071 Grabar al menos 10 interpretaciones de referencia en `core/tests/fixtures/interpretaciones/`, con su resultado esperado **escrito a mano**. Volcarlo de la implementación copiaría su fallo al fichero y la prueba pasaría a confirmar el error en vez de detectarlo
- [ ] T072 Prueba en `core/tests/evaluacion_test.rs`: **la tabla entera**, todas las interpretaciones contra su resultado esperado de una vez. Existe como tabla y no como pruebas sueltas por la lección de la 003: un ajuste que arregla un caso rompe otro, y solo se ve comprobándolos juntos
- [ ] T073 Documentar en `specs/004-evaluar-interpretacion/quickstart.md` el procedimiento para añadir una interpretación de referencia y qué declarar cuando un ajuste cambia el resultado de alguna

### Medir lo que el plan dejó pendiente

- [ ] T074 Medir **cuánta precisión se pierde por no mirar el futuro** (FR-004): comparar el emparejamiento en línea con el óptimo global sobre las interpretaciones grabadas, y registrar la diferencia en `research.md`. Es la cifra que el plan dejó explícitamente sin suponer
- [ ] T075 Prueba de coste en `core/tests/evaluacion_test.rs`: se **cuentan** notas examinadas por pulsación, no se cronometra, y el número no crece con el tamaño de la canción. En la 003 esta forma de medir destapó un coste 30 veces mayor que ninguna prueba de tiempo vio
- [ ] T076 Prueba de presupuesto en `core/tests/evaluacion_test.rs`: SC-007, una interpretación completa de 10 minutos se evalúa en menos de 1 segundo

### Puertas

- [ ] T077 [P] Documentar con rustdoc la API pública de `core/src/evaluacion/`
- [ ] T078 [P] Verificar `cargo clippy --workspace --all-targets -- -D warnings` limpio y `pnpm build` sin avisos
- [ ] T079 Verificar que `cargo tree -p piano-core` **sigue dando exactamente tres líneas**: esta feature no añade ninguna dependencia
- [ ] T080 Comprobación manual de SC-010 con una persona, anotada en `quickstart.md`: que alguien toque un pasaje dos veces y confirme que el sistema señala como mejor la que él considera mejor. El orden léxico es una decisión de diseño; que coincida con lo que siente un músico hay que comprobarlo
- [ ] T081 Comprobación manual del tono del resumen, anotada en `specs/004-evaluar-interpretacion/quickstart.md`: que un principiante lea su resultado y no se desanime. Un motor correcto que desmoraliza es un motor que nadie usa dos veces, y eso no lo detecta ninguna prueba

---

## Dependencies & Execution Order

```
Phase 1 (Setup) ──► Phase 2 (Foundational) ──► Phase 3 (US1) 🎯 MVP
                                                    │
                                    ┌───────────────┼───────────────┐
                                    ▼               ▼               ▼
                              Phase 4 (US2)   Phase 5 (US3)   Phase 6 (US4)
                                    │               │               │
                                    └───────────────┴───────────────┘
                                                    │
                                                    ▼
                                            Phase 7 (Polish)
```

- **La Fase 2 bloquea todo**: sin la inversa del ancla y sin las tolerancias no se puede juzgar nada.
- **US2, US3 y US4 dependen de US1** pero **no entre sí**: una vez hay veredictos, localizarlos,
  ajustar la exigencia y comparar intentos son tres trabajos independientes.
- **US4 usa el orden que US1 no necesita**, así que `comparar` puede esperar.

### Oportunidades de paralelismo

- **Fase 1**: T003, T004 y T005 son archivos distintos.
- **Fase 2**: el bloque de estadística (T009–T012) y el de la inversa del ancla (T013–T018) no se
  tocan; se pueden hacer a la vez.
- **Fases 4, 5 y 6**: independientes entre sí una vez cerrada la 3.
- **Fase 7**: T077 y T078 en paralelo.

## Implementation Strategy

**MVP = Fases 1 a 3 (64 tareas).** Al terminarlas el alumno toca una pieza y ve cuántas acertó, cuántas se dejó
y cuántas tocó de más, con detección de desfase sistemático. Es entregable por sí solo.

Después, por valor decreciente: **US2** (dónde fallo) es lo que más cambia lo que el alumno hace
mañana; **US3** (exigencia) evita que la evaluación desanime o resulte inútil; **US4** (comparar
intentos) es lo que sostiene una sesión larga.

**No dejar la Fase 7 para el final del todo.** T071 y T072 —los fixtures de referencia y su tabla—
convendría empezarlos en cuanto la Fase 3 dé los primeros veredictos: son la red que impide que un
ajuste de tolerancia rompa un caso ya resuelto, y esa red vale más cuanto antes se tienda.
