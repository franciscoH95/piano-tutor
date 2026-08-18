# Quickstart: validar la captura MIDI

**Feature**: `002-midi-input-capture` | **Fecha**: 2026-08-18

## Requisitos

- Rust 1.97.1 o superior.
- **Ningún teclado MIDI.** Toda la verificación automática corre sin hardware; ésa es justamente
  una de las cosas que se verifica (SC-005).

## Validación completa

```sh
cd /Users/frankohiggins/Projects/teacher_learn_piano_songs_app
cargo test -p piano-core
```

**Resultado esperado**: todo en verde, y la suite completa (las 79 pruebas de la feature 001 más
las nuevas) por debajo de **1 segundo**.

## Las tres puertas que hay que mirar

### 1. El núcleo no ha tocado el hardware

```sh
cargo tree -p piano-core
```

**Resultado esperado**: exactamente dos líneas, `piano-core` y `rtrb`. Si aparece `coremidi`,
`windows`, `midir` o cualquier cosa de sistema, el Principio III está roto y hay que detener la
fusión, aunque las pruebas pasen.

### 2. El banco de latencia

```sh
cargo run -p piano-bench --release --bin latencia
```

**Resultado esperado**: código de salida 0, y un informe con el percentil 95 por debajo de **1 ms**
(la puerta de nuestra capa) y muy por debajo de **30 ms** (la puerta constitucional).

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

Nada de esto corre en integración continua. Se hace de vez en cuando, con el instrumento delante:

```sh
cargo run -p piano-midi-io --example escuchar     # enumera, abre y muestra lo que tocas
cargo run -p piano-bench --release --bin latencia -- --con-hardware
```

El segundo mide desde el sello del propio sistema operativo. **La diferencia entre ese número y el
de integración continua es exactamente el tramo que el banco sintético no cubre.** Hasta que se
ejecute al menos una vez, el informe imprime `DELTA_SO_USB: SIN CALIBRAR`, y eso es honesto: no lo
sabemos.

## Lo que este quickstart todavía NO valida

- **Windows.** Nada de lo planificado para Windows se ha ejecutado ni una sola vez: está sostenido
  por lectura de código, documentación oficial e issues públicos. El primer trabajo del lado
  Windows es un día de validación en máquina real. Ver `plan.md`, Complexity Tracking.
- **La aplicación de escritorio.** Esta entrega no dibuja nada: la pantalla de selección de teclado
  llega con la feature de visualización.
