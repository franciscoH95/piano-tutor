# Research: Captura MIDI del teclado — decisiones técnicas

**Feature**: `002-midi-input-capture` | **Fecha**: 2026-08-17
**Toolchain verificada**: rustc 1.97.1, cargo 1.97.1, macOS arm64 (Darwin 25.6.0) + Windows x86_64.

> **Aviso de alcance de la verificación**: todo lo medido y ejecutado en este documento se hizo en
> **macOS**. La ruta de Windows está sostenida por lectura de código, documentación oficial e
> issues públicos, **no por medición**. Ver el riesgo U8 al final: hay un trabajo de un día en una
> máquina Windows real que debe ser lo primero que se haga del lado Windows.

---

## Decisión 1: Capa de acceso al hardware MIDI

**Decision**:
**No usar `midir`.** Hablar directamente con la API de cada plataforma:

- **macOS**: `coremidi = "0.9.2"` (MIT, publicada 2026-08-02, dependencias: `coremidi-sys`,
  `core-foundation`, `objc2`/`block2`).
- **Windows**: `windows` (WinMM: `midiInOpen`, `midiInStart`, `midiInStop`, `midiInClose`).
- **Análisis de mensajes**: propio, en `midi-io/src/parser.rs`, unas 60 líneas, bajo los mismos
  lints que el núcleo, incluido `deny(clippy::indexing_slicing)`.

**Justificacion**:

`midir 0.11.0` quedó **descalificado por evidencia reproducida**, no por preferencia:

1. **Aborta el proceso ante un paquete truncado, en la ruta exacta de note-on/note-off.**
   `src/backend/coremidi/mod.rs:173` hace `&pdata[cur_byte..(cur_byte + size)]` con un `size`
   deducido **solo del byte de estado**, sin comprobar la longitud real del paquete. Verificado en
   este repositorio leyendo el fuente, y reproducido ejecutando: 14 pánicos de 16 casos.
   `[0x90]` produce literalmente `range end index 3 out of range for slice of length 1`, el mismo
   mensaje del **issue #160** (abierto 2024-10-31, sin corregir, reportado con un teclado real
   Midiplus X-6 III). También panica `[0x90,0x3C,0x64,0x90]`: **una nota válida seguida de un byte
   de estado suelto**.
2. **No es contenible.** El pánico ocurre en el hilo del callback de CoreMIDI, cruzando una
   frontera FFI: no se convierte en `unwind` capturable. Con `catch_unwind` envolviendo todo desde
   `main` el resultado fue `libc++abi: terminating due to uncaught foreign exception`, **EXIT=134
   (SIGABRT)**. Un solo teclado que emita basura al encenderse mata la aplicación del alumno.
   `Ignore::All` **no protege**: la rama de notas no consulta `ignore` en ningún momento.
3. **`close()` aborta el proceso si llega un mensaje durante el cierre** — que es exactamente el
   escenario de la decisión D6 (detener la captura con teclas hundidas). Libera el mutex antes de
   destruir el puerto, y el callback hace `unwrap()` sobre `None`.
4. **Toma un `Mutex` con `unwrap()` dentro del callback de tiempo real**
   (`coremidi/mod.rs:207-208`). Es un primitivo bloqueante en la ruta crítica: lo prohíbe el
   Principio IV.
5. **Usa símbolos privados de CoreAudio** (`_AudioGetCurrentHostTime`,
   `_AudioConvertHostTimeToNanos`). Los issues 132, 169 y 170 documentan **rechazos de la Mac App
   Store** por ello.

Y lo que inclina la balanza: **`midir` tampoco notifica las desconexiones en ninguna plataforma**
(issues #78, #86, #84, #191, años abiertos). Íbamos a escribir código específico de macOS y de
Windows de todas formas para cumplir la historia P3. Lo que aportaba `midir` se reducía a
*enumerar, abrir y recibir bytes*: no compensa mantener un fork parcheado indefinidamente para eso.

Ventaja adicional de hablar directamente con CoreMIDI, que `midir` impide: **podemos leer el reloj
una sola vez por paquete** y asignar el mismo instante a todas las notas de ese paquete.
`midir` fija el timestamp una vez por paquete pero invoca el callback una vez por mensaje, lo que
obligaba a leer el reloj tres veces para un acorde y producía dispersión intra-acorde (medido:
p50 1,08 µs, p95 5,71 µs, máx 53,8 µs, y los pares consecutivos salían 193 iguales / 207 distintos:
una moneda al aire). Controlando el bucle de paquetes, un acorde tiene **un solo instante, exacto y
por construcción**.

**Alternativas consideradas**:

- *`midir 0.11.0` tal cual*: descalificado por el pánico que aborta el proceso (arriba).
- *Fork de `midir` con tres parches, fijado por SHA*: era la recomendación inicial de la
  investigación. Descartado: obliga a mantener el fork indefinidamente (upstream sin respuesta
  desde 2024), **no arregla** el mutex en el callback de tiempo real ni los símbolos privados, y no
  ahorra el código por plataforma que la historia P3 exige de todos modos.
- *`rtmidi` (envoltorio de RtMidi en C++)*: añade una dependencia de C++ y un compilador de C++ al
  build de las dos plataformas. 3.254 descargas.
- *WinRT MIDI en lugar de WinMM*: `midir` avisa de que su backend `winrt` está peor probado;
  además `microsoft/MIDI` documenta problemas de enumeración. Se descarta para la v1.

**Riesgos y mitigación**:

- **Dos backends que escribir y mantener** en lugar de uno. Mitigación: la superficie es pequeña y
  está acotada por diseño —enumerar, abrir, recibir bytes, cerrar, notificar— sin ninguna decisión
  de dominio dentro. El plan declara qué archivos son y exige mantenerlos así.
- **La ruta de Windows no está medida** (ver U8). Mitigación: spike de un día en máquina real como
  primer trabajo del lado Windows.
- **`coremidi` es una envoltura de una API de C**: cualquier error nuestro en la FFI se paga caro.
  Mitigación: el bucle de paquetes y el análisis viven bajo `deny(clippy::indexing_slicing)`, que
  hace **imposible por construcción** la clase de fallo que tumbó a `midir`.

---

## Decisión 2: Dónde vive cada cosa (Principio III)

**Decision**: cuatro miembros en el workspace.

```text
core/        piano-core      CERO dependencias de sistema. Define el contrato y toda la lógica.
midi-io/     piano-midi-io   Depende de piano-core + coremidi (macOS) / windows (Windows).
bench/       piano-bench     Banco de latencia. Fuera de `cargo test`.
src-tauri/   piano-tutor     Crea el reloj de sesión UNA vez y se lo pasa a captura y reproducción.
```

Reparto exacto:

- **`core/src/capture/`** — todo lo que tiene lógica y por tanto todo lo que se prueba:
  el trait `FuenteDeEventos` (inyectado **por genérico, no `dyn`**, igual que `Clock`), los tipos
  `Dispositivo`, `PulsacionCapturada`, `InformeDeCaptura`, el emparejador de pulsación con suelta,
  el cierre por parada y por pérdida de dispositivo, el transporte acotado y los contadores.
  También `FuenteGuionizada`, la fuente controlada que exigen FR-021 y FR-022.
- **`midi-io/src/`** — la única capa sin cobertura automática, deliberadamente mínima: abrir el
  puerto, recorrer el paquete, filtrar a notas, sellar con el reloj y empujar al anillo. **Sin
  ninguna decisión de dominio.**

**Puerta mecánica del Principio III**: `cargo tree -p piano-core` debe mostrar **exactamente tres
líneas** (`piano-core`, `midi_file` y `rtrb`), en los tres targets, más un grep negativo contra
`coremidi|midir|windows|winapi|core-foundation|objc2|libc|alsa|jack`. Añadir cualquier crate al
núcleo rompe la integración continua y obliga a discutirlo en el PR.

**Justificacion**: el Principio III exige que el núcleo se pruebe sin ventana y sin teclado. Un
trait genérico logra eso sin coste en la ruta crítica, y la puerta de `cargo tree` convierte el
principio en algo **verificable**, no confiado.

**Alternativas consideradas**:

- *Todo dentro de `piano-core` tras una feature de cargo desactivada por defecto*: descartado
  porque `cargo test --all-features` en integración continua volvería a arrastrar el sistema al
  núcleo, y la puerta dejaría de significar nada.
- *Todo dentro de `src-tauri`*: descartado porque el emparejamiento y los cierres son lógica de
  dominio con casos límite; enterrarlos en la capa de aplicación los deja sin pruebas.

**Riesgos y mitigación**:

- **`rtrb` en el núcleo debilita la puerta** de "una línea" a "tres líneas con lista blanca". Se
  acepta con la puerta reforzada descrita arriba. Si se prefiere conservar la puerta de una sola
  línea, la alternativa es un quinto crate `piano-transport` sin dependencias de sistema.

---

## Decisión 3: Sellado del instante de cada evento

**Decision**: **sellar con nuestro propio reloj de sesión, no con el del sistema operativo.**

1. La **primera instrucción** del callback lee el reloj de sesión: `let t = clock.now();`. Como
   controlamos el bucle de paquetes, se lee **una sola vez por paquete** y ese instante se asigna a
   todas las notas del paquete: un acorde tiene un instante único por construcción.
2. El timestamp del sistema se conserva **solo como diagnóstico** (`os_timestamp_us: Option<u64>`),
   nunca participa en ninguna decisión musical y nunca se mezcla con el reloj de sesión.
3. **Monotonía (FR-013)**: `t = max(t, ultimo); ultimo = t;`. Con reloj propio y un único hilo de
   callback nunca dispara, pero se conserva como red de seguridad con `debug_assert!` y contador.

**Desfase con el reloj de reproducción: cero, por construcción.** Es el mismo reloj (decisión D7).
No hay fórmula de alineación porque no hay nada que alinear, que es justo lo que la hace robusta.

**Justificacion**: el timestamp del sistema **no es portable**. Verificado leyendo el código:

| Plataforma | Origen (epoch) | Unidad efectiva |
| --- | --- | --- |
| macOS (CoreMIDI) | arranque de la máquina | 1 µs |
| Windows (WinMM) | la llamada a `midiInStart` | 1 ms |

Épocas distintas y resoluciones que difieren en tres órdenes de magnitud. Tratarlos igual sería
exactamente el error silencioso que se temía: compila, pasa las pruebas, y la evaluación futura
concluye que el alumno siempre llega tarde. Coste de leer nuestro reloj: **~40 ns**, el 0,00013 %
del presupuesto de 30 ms.

**Alternativas consideradas**:

- *Usar el sello del sistema y calcular el desfase una vez*: descartado. Requiere una fórmula de
  alineación por plataforma, sufre deriva entre relojes en sesiones largas, y en Windows la
  resolución de 1 ms se come el 3,3 % del presupuesto sin dar nada a cambio.
- *Usar el sello del sistema solo en macOS*: descartado por incoherencia: el mismo código se
  comportaría distinto según la plataforma, y las pruebas de una no dirían nada de la otra.

**Riesgos y mitigación**:

- **El acuerdo entre CoreMIDI y `std::time::Instant` depende de que `std` siga usando
  `CLOCK_UPTIME_RAW`.** Irrelevante bajo esta decisión (no usamos el sello del sistema), pero se
  deja anotado por si alguien lo reintroduce.
- **`FR-013` debe decir "no decreciente", nunca "estrictamente creciente"**: los empates existen y
  son legítimos (un acorde en un paquete). El desempate es el orden de llegada.
- **Tolerancia técnica de simultaneidad**: `TOLERANCIA_SIMULTANEIDAD_US = 1_000`, usada
  **solo en pruebas** para afirmar que un paquete produjo un acorde. La tolerancia **musical**
  (decenas de ms) pertenece a la feature de puntuación y queda **fuera de alcance** de la 002.

---

## Decisión 4: Transporte entre el callback y el consumidor

**Decision**: `rtrb = "0.3.4"` (MIT OR Apache-2.0), una cola SPSC por dispositivo abierto.

- Capacidad **4.096 eventos × 16 bytes = 64 KiB**, reservada de una vez al abrir.
- Evento de 16 bytes exactos, verificado:
  `#[repr(C)] struct EventoCrudo { at: Micros /*u64*/, seq: u32, key: u8, velocity: u8, kind: u8, channel: u8 }`
- El productor hace **exactamente esto**, sin asignar y sin bloquearse:

```rust
if self.tx.push(ev).is_err() {                    // D5: se descarta lo ENTRANTE
    self.descartados.fetch_add(1, Ordering::Relaxed);
} else if self.quiere_despertar.swap(false, Ordering::SeqCst) {
    self.consumidor.unpark();                     // solo en la transición vacío -> no vacío
}
```

- El filtrado de D3 (descartar pedal, aftertouch, control, reloj) ocurre **antes** del `push`: dos
  comparaciones, y evita que una barrida de pedal consuma ranuras.
- El `seq` monótono permite al consumidor localizar **exactamente dónde** hubo descarte, sin gastar
  una ranura extra ni trabajo en el productor.

**Coste medido en el productor**: p50 41 ns, p95 84–625 ns, p999 < 5 µs. Cero asignaciones, cero
cerrojos.

**Justificacion**: cumple D5 al pie de la letra —acotada, descarta lo entrante, nunca bloquea, nunca
crece— y el productor es *wait-free*. 4.096 ranuras son **dos órdenes de magnitud** por encima de
la ráfaga humana más densa (un glissando muy rápido no pasa de ~50 eventos/s; 4.096 cubren más de
80 segundos de esa ráfaga sin que el consumidor toque nada).

**Alternativas consideradas**:

- *`std::sync::mpsc::sync_channel` + `try_send`*: cumple el contrato, pero es MPMC internamente y
  paga sincronización que no necesitamos.
- *`crossbeam-queue::ArrayQueue`*: correcta y sin `unsafe` en nuestro lado, pero MPMC: más cara que
  una SPSC dedicada en la ruta crítica.
- *Escribirla nosotros*: descartado, exigiría `unsafe` y el núcleo tiene `forbid(unsafe_code)`.

**Riesgos y mitigación**:

- **`rtrb` usa `unsafe` internamente** (71 apariciones en 2.382 líneas). No lo auditamos línea a
  línea: la confianza viene del uso (10,1 M de descargas, estándar de facto en audio en Rust).
- **`rtrb 0.3.4` arrastra un fallo de `ReadChunk::commit()`** corregido en 0.4.0. **No nos afecta**:
  no usamos la API de *chunks* y `EventoCrudo` es `Copy` sin `Drop`.
- **La cola de la latencia no viene de la cola de eventos sino del planificador**: p99 270 µs,
  p999 2,65 ms con el consumidor a prioridad normal. **Mitigación obligatoria**: QoS
  *user-interactive* en macOS y prioridad por encima de normal en Windows para el hilo consumidor.

---

## Decisión 5: Cómo despierta el consumidor

**Decision**: `std::thread::park` / `unpark` con testigo `quiere_despertar`, protocolo tipo Dekker.
**No sondeo.**

```rust
loop {
    while let Ok(ev) = rx.pop() { procesar(ev); }
    if parar.load(Relaxed) { break; }
    quiere_despertar.store(true, SeqCst);
    if rx.is_empty() { std::thread::park(); }
    quiere_despertar.store(false, SeqCst);
}
```

**Coste medido del despertar: p95 37,5 µs**, el **0,125 %** del presupuesto de 30 ms. CPU en reposo:
0,57 ms por cada 3 s.

**Justificacion**: la decisión D4 exige que el despertar del consumidor **cuente** dentro de la
medida. Un sondeo cada 16 ms consumiría por sí solo **más de la mitad** del presupuesto en el peor
caso, y gastaría CPU sin tocar nada. El testigo de `unpark` es *pegajoso*: si el `unpark` llega
antes del `park`, el `park` retorna de inmediato. No hay aviso perdido.

**Alternativas consideradas**:

- *Sondeo a intervalo fijo*: descartado por lo anterior. A 1 ms el coste en latencia sería
  aceptable pero la CPU en reposo, no.
- *Condvar + Mutex*: correcto, pero introduce un mutex que el productor tendría que tomar. El
  Principio IV prohíbe bloqueos en la ruta crítica.

---

## Decisión 6: Diseño del banco de latencia

**Decision**: arnés propio en `bench/`, **fuera de `cargo test`**, ejecutado como binario en CI.

**Qué mide**: `entrega_us = t1 - t0`.

- **t0**: el instante que lee el hilo productor **inmediatamente antes** de publicar en el anillo.
  Es exactamente la misma línea donde, en producción, publica el callback real.
- **t1**: el instante que lee el consumidor **después** de haber sido despertado de un bloqueo real
  **y después** de decodificar el evento al tipo de dominio. No cuando entra en la cola, no cuando
  se emite la notificación, no cuando el planificador lo marca ejecutable. Cuando el valor
  decodificado está en su marco de pila. Eso cumple D4 al pie de la letra.

**Qué NO mide, y hay que decirlo en cada informe**: el barrido de teclas del instrumento, el
transporte USB y el despacho del driver del sistema operativo. Según la literatura, entre 1 y 9 ms.
**El banco de CI cubre en torno al 0,11 % del recorrido que percibe el alumno.** El propio banco
imprime esa tabla y esa frase en cada ejecución, para que el número no se lea como lo que no es.

**Parámetros**: n = 3.000 muestras a 1 ms, k = 3 repeticiones tomando el **mínimo de los p95**,
500 muestras de calentamiento descartadas.

**Dos umbrales**:

| Puerta | Umbral | Qué protege |
| --- | --- | --- |
| Capa | 1 ms p95 | Regresiones de **nuestro** código. Es la que salta de verdad. |
| Constitucional | 30 ms p95 | El presupuesto del Principio IV. Margen enorme a propósito. |

**Mecanismo de fallo**: código de salida (0 correcto, 1 supera la puerta de capa, 2 supera la
constitucional, 3 error de ejecución), registrado como *required status check* de la rama.

**Justificacion**: la suite de pruebas debe seguir por debajo de 1 segundo (hoy: 60 ms). Un p95
honesto necesita miles de muestras: dentro de `cargo test` destruiría el bucle de desarrollo, y un
benchmark intermitente se acaba desactivando —que es la peor deuda posible, porque la puerta sigue
ahí pero ya no protege nada.

**Alternativas consideradas**:

- *`criterion`*: excelente para comparar versiones de una función, pero orientado a rendimiento de
  CPU, no a latencia de extremo a extremo con hilos y despertares.
- *`divan`*: más ligero, mismo desajuste conceptual.
- *Meterlo en `cargo test` con menos muestras*: descartado, un p95 sobre pocas muestras es ruido.

**Riesgos y mitigación**:

- **Los números que sostienen los umbrales se midieron en un Apple M1 Max de 10 núcleos**, no en un
  runner compartido de 2–4 vCPU. **Acción previa obligatoria**: ejecutar el banco 20 veces en el
  runner real antes de fijar el umbral definitivo, y ajustarlo **con el dato en la mano**.
- **Modo con hardware real** (`--con-hardware`), a ejecutar a mano de vez en cuando: mide desde el
  sello del sistema hasta t1, y su diferencia con el número de CI es exactamente el tramo que el
  banco sintético no cubre.

---

## Decisión 7: Detección de conexión y desconexión

**Decision**: **notificación push por plataforma, más sondeo de 1.000 ms como respaldo**, con doble
confirmación. **Nunca inferir la pérdida por silencio.**

- **macOS**: `coremidi::Client::new_with_notifications` (verificado en `client.rs:84`), escuchando
  `Notification::ObjectRemoved` / `ObjectAdded`. **Restricción de orden**: debe crearse **antes**
  que cualquier otro cliente MIDI del proceso, en un hilo cuyo `CFRunLoop` esté corriendo — en
  Tauri, el hilo principal.
- **Windows**: `CM_Register_Notification` (cfgmgr32) con
  `ClassGuid = GUID_DEVINTERFACE_MIDI_INPUT`. Sin ventana, sin COM, sin WinRT.
- **Respaldo**: re-enumerar cada 1.000 ms. La pérdida se declara si la identidad elegida falta en
  **dos enumeraciones consecutivas**, lo que cumple SC-007 (< 2 s) sin falsos positivos.

**Cómo se distingue "dejó de tocar" de "se desconectó"**: no se infiere nunca del silencio. Solo la
notificación del sistema o la doble ausencia en la enumeración cuentan como pérdida.

**Notas hundidas al perderse el aparato**: se cierran con etiqueta **propia**
(`PorPerdidaDeDispositivo`, distinta de la `PorParada` de D6), selladas con el instante del
**último evento recibido** —no el de la detección, que llega más tarde— y marcadas como de
**duración censurada**: sabemos cuándo empezaron, no cuándo terminaron.

**Riesgos y mitigación**:

- **Windows sin parchear**: `microsoft/MIDI` #597 y #783 documentan que con MidiSrv las apps WinMM
  no ven dispositivos conectados después del arranque. Mitigación: el respaldo por sondeo.
- **`microsoft/MIDI` #906 (abierto)**: tras reconectar, el puerto se abre con éxito y **nunca
  entrega un solo mensaje**. Implica que "reanudar tras reconectar" (historia P3) **no puede darse
  por hecho en Windows**. Mitigación: tras reabrir, exigir al menos un evento en una ventana de
  cortesía; si no llega, informar al usuario en lugar de fingir que funciona.

---

## Decisión 8: Emparejamiento en tiempo real y cierres

**Decision**: tabla plana de 2.048 ranuras indexada por `(canal, altura)`, no un mapa dinámico.
Política **FIFO por voz**, la misma que ya usa la feature 001, para que lo tocado y lo esperado sean
comparables por construcción.

Etiquetas de cierre:

```text
PorSuelta                 la tecla se soltó de verdad
PorParada                 el usuario detuvo la captura (D6)
PorPerdidaDeDispositivo   el teclado desapareció (duración censurada)
PorRepulsacion            la misma tecla volvió a pulsarse sin soltarse
```

**Justificacion**: una tabla plana no asigna memoria y tiene coste constante, lo que la ruta crítica
exige. 2.048 ranuras cubren los 16 canales × 128 alturas del estándar.

---

## Decisión 9: Contrato sustituible y cómo se prueba sin hardware

**Decision**: `trait FuenteDeEventos` inyectado **por genérico**, no como `dyn`, exactamente igual
que el `Clock` que ya existe. Tres implementaciones, en tres niveles de fidelidad:

| Implementación | Dónde vive | Qué prueba |
| --- | --- | --- |
| `FuenteGuionizada` | `core/` | Toda la lógica de dominio, con instantes fijos y reproducibles. Es la que cumple FR-021 y FR-022. |
| `ColaCaptura` | `core/` | El transporte real y el despertar, con un productor sintético. |
| Adaptador real | `midi-io/` | Solo a mano, con teclado delante. |

**Justificacion**: el genérico no paga despacho dinámico en la ruta crítica, y es coherente con lo
que ya hace el núcleo. Toda la suite pasa sin ventana y sin teclado (SC-005).

---

## Decisión 10: Identidad del dispositivo (refinamiento de D1)

**Decision**: **identificador del sistema como clave primaria, y la pareja (nombre, posición entre
homónimos) como reserva.** La regla normativa de D1 queda intacta como respaldo, no derogada.

- **macOS**: `unique_id()` de CoreMIDI (verificado en `coremidi-0.9.2/src/object.rs:26`).
- **Windows**: ruta de interfaz del dispositivo.
- Si el identificador no casa, se prueba la pareja (nombre, posición). Si tampoco, **se pide elegir
  de nuevo**: nunca se abre otro dispositivo en su lugar.

**Justificacion**: cuando esta decisión se cerró con el usuario, la opción del identificador del
sistema se presentó como no portable. Era una descripción **incompleta**: `coremidi` lo expone
directamente y Windows ofrece la ruta de interfaz. Ambos sobreviven a renumeraciones y a mover el
teclado de conector USB, que es justo donde la pareja (nombre, posición) falla. Confirmado con el
usuario el 2026-08-17.

---

## Incertidumbres que quedan abiertas

| # | Qué no sabemos | Qué hacemos mientras |
| --- | --- | --- |
| **U1** | Los umbrales del banco se midieron en un M1 Max, no en un runner de CI. | Ejecutar el banco 20 veces en el runner real **antes** de fijar el umbral. Nunca por corazonada. |
| **U2** | **La ruta de Windows no ha sido ejecutada ni una sola vez**: solo compilación cruzada y lectura de código y documentación. | **Spike de un día en una máquina Windows real como primer trabajo del lado Windows**, validando en este orden: enumeración y apertura; `CM_Register_Notification`; cierre tras retirada PnP sin cuelgue (KB4460006); y la cifra de latencia. Hasta entonces, todo lo escrito aquí sobre Windows es lectura, no medición. |
| **U3** | `DELTA_SO_USB` (el tramo que el banco no cubre) es desconocido: exige teclado e interfaz físicos. | El banco publica `SIN CALIBRAR` en su informe hasta que exista el dato. Esperado por la literatura: 1–9 ms. Si al calibrar supera los 8 ms, es hardware inadecuado y se documenta como no soportado, igual que Bluetooth. |
| **U4** | Cuelgue irrecuperable en Windows al cerrar tras retirada PnP (Microsoft KB4460006). | Se valida en el spike U2. Si se reproduce, hay que cerrar en un hilo aparte con tiempo límite. |
| **U5** | `microsoft/MIDI` #906: tras reconectar, el puerto abre pero no entrega nada. | Ventana de cortesía tras reabrir; si no llega ningún evento, informar en vez de fingir. |
