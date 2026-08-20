# Quickstart: validar la captura MIDI

**Feature**: `002-midi-input-capture` | **Fecha**: 2026-08-18

## Requisitos

- Rust 1.97.1 o superior.
- **Ningún teclado MIDI.** Toda la verificación automática corre sin hardware; ésa es justamente
  una de las cosas que se verifica (SC-005).

## Validación completa

```sh
cd <raíz del repositorio>
cargo test -p piano-core
```

**Resultado esperado** (medido el 2026-08-18): **162 pruebas en verde**, suite completa por debajo
de un segundo. De ellas, 79 son de la feature 001 y 83 nuevas.

El adaptador de macOS **sí está cubierto**: `coremidi` permite crear teclados virtuales, así que
`midi-io/tests/macos_virtual_test.rs` y `vigia_test.rs` lo ejercen de verdad contra CoreMIDI, sin
hardware. Lo único sin cobertura automática es la rama de Windows, que aún no existe.

## Las tres puertas que hay que mirar

### 1. El núcleo no ha tocado el hardware

```sh
cargo tree -p piano-core
```

**Resultado esperado**: exactamente tres líneas: `piano-core`, `midi_file` (de la feature 001) y `rtrb`. Si aparece `coremidi`,
`windows`, `midir` o cualquier cosa de sistema, el Principio III está roto y hay que detener la
fusión, aunque las pruebas pasen.

### 2. El banco de latencia

```sh
cargo run -p piano-bench --release --bin latencia
```

**Resultado esperado**: código de salida 0, y un informe con el percentil 95 por debajo de **1 ms**
(la puerta de nuestra capa) y muy por debajo de **30 ms** (la puerta constitucional). Medido el
2026-08-18 en un portátil Apple Silicon: **p95 = 21–25 µs**.

Para comprobar que la puerta **muerde** de verdad, y no solo que está escrita:

```sh
PIANO_BENCH_PUERTA_US=0 cargo run -p piano-bench --release --bin latencia; echo $?
```

Debe imprimir `1`. Esa misma variable es la que permite calibrar el umbral en un runner real
antes de fijarlo, en vez de heredar un número medido en otra máquina.

Códigos de salida: `0` correcto · `1` supera la puerta de capa · `2` supera la constitucional ·
`3` error de ejecución.

**Lee la advertencia que imprime.** El banco mide en torno al **0,11 %** del recorrido que percibe
el alumno: cubre desde que nuestro código recibe el mensaje hasta que el consumidor lo tiene
decodificado. Quedan fuera el barrido de teclas del instrumento, el transporte USB y el despacho
del driver, que no son observables desde la aplicación. Es un detector de regresiones **de nuestro
código**, no una afirmación sobre lo que siente quien toca.

### 3. Nada asigna memoria en la ruta crítica

```sh
cargo test -p piano-core sin_asignaciones
```

Verifica con un asignador instrumentado que publicar un evento no asigna ni una sola vez.

## Comprobaciones puntuales

```sh
cargo test -p piano-core emparejador     # FIFO por voz, cierres, repulsación
cargo test -p piano-core desbordamiento  # se descarta lo entrante y se cuenta
cargo test -p piano-core determinismo    # el mismo guion, el mismo resultado, 100 veces
cargo test -p piano-core identidad       # id del sistema primero, (nombre, posición) de reserva
```

## Con un teclado de verdad (a mano)

Nada de esto corre en integración continua. Se hace de vez en cuando, con el instrumento delante.
**Los dos programas funcionan igual en macOS y en Windows**: hasta el 2026-08-19 tenían el cuerpo
entero bajo `#[cfg(target_os = "macos")]` y en Windows contestaban «solo implementado para macOS»
saliendo con código 0, de modo que la única plataforma que hacía falta validar era la única que no
se podía ejercer.

```sh
cargo run -p piano-midi-io --example escuchar     # enumera, abre y muestra lo que tocas
cargo run -p piano-midi-io --example escuchar -- 2   # si hay varios, el de la posición 2
cargo run -p piano-bench --release --bin latencia -- --con-hardware
```

### El procedimiento de T042 y T071, paso a paso

`escuchar` recorre la lista de T042 en orden y da un veredicto por paso, así que el procedimiento
manual **es ejecutarlo y leer**:

| Paso | Qué se comprueba | Qué hacer |
|------|------------------|-----------|
| 1 | Enumerar | nada; si falla, el texto lleva el código que devolvió el sistema |
| 2 | Abrir | nada |
| 3 | Recibir notas | tocar unas cuantas, incluido algún acorde |
| 4 | Detectar la retirada | **desenchufar el teclado** a mitad de captura (FR-014: < 2000 ms) |
| 5 | Cerrar sin colgarse | nada; el paso se anuncia *antes* de intentarlo a propósito, para que un cuelgue deje escrito en pantalla dónde fue (Microsoft KB4460006) |
| 6 | Reabrir | **volver a enchufarlo**; la captura debe reanudarse sin reiniciar nada |

Un fallo de enumeración y «no hay ningún teclado conectado» se distinguen en el texto: son causas
opuestas y antes daban el mismo cartel.

El segundo mide desde el sello del propio sistema operativo. **La diferencia entre ese número y el
de integración continua es exactamente el tramo que el banco sintético no cubre.** Hasta que se
ejecute al menos una vez, el informe imprime `DELTA_SO_USB: SIN CALIBRAR`, y eso es honesto: no lo
sabemos.

### 4. Que una sesión larga no se degrada (SC-008)

```sh
cargo run -p piano-bench --release --bin latencia -- --sostenido
```

Diez minutos reales. Compara el percentil 95 del último minuto con el del primero y falla si la
degradación supera el 10 %. **Medido el 2026-08-18**: p95 por minuto entre 39 y 62 µs, primero
52 µs, último 44 µs, límite 57 µs. Código de salida 0.

Para comprobar que el modo funciona sin gastar diez minutos:

```sh
PIANO_BENCH_MINUTOS=2 cargo run -p piano-bench --release --bin latencia -- --sostenido
```

## Lo que este quickstart todavía NO valida

- **Windows.** Nada de lo planificado para Windows se ha ejecutado ni una sola vez: está sostenido
  por lectura de código, documentación oficial e issues públicos. El primer trabajo del lado
  Windows es un día de validación en máquina real. Ver `plan.md`, Complexity Tracking.
  Las herramientas para hacerlo ya existen y compilan para `aarch64-pc-windows-msvc` y
  `x86_64-pc-windows-msvc`; lo que falta es ejecutarlas. **Compilar no es funcionar.**
- **La tabla de traducción de `HRESULT`** (`midi-io/src/windows.rs`). Se dedujo leyendo
  documentación, no midiendo: nadie ha visto todavía qué devuelve WinRT con el puerto ocupado. Por
  eso un código no contemplado ya no se convierte en «no se pudo abrir», que se tragaba el número,
  sino que llega con él en hexadecimal. La primera ejecución en Windows corrige la tabla con un
  dato en vez de con una suposición.
- **La aplicación de escritorio.** Esta entrega no dibuja nada: la pantalla de selección de teclado
  llega con la feature de visualización.
