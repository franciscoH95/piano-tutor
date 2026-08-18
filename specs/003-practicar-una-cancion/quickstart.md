# Quickstart: validar la práctica de una canción

**Feature**: `003-practicar-una-cancion` | **Fecha**: 2026-08-18

## Requisitos

- Rust 1.97.1+, Node 24+, pnpm 11+.
- **Teclado MIDI opcional.** Toda la lógica se verifica sin él; solo el ensayo manual lo necesita.

## Nivel 1 — lo que la integración continua comprueba en cada cambio

```sh
./scripts/verificar.sh
```

Es el mismo comando de siempre, y ahora cubre además toda la lógica nueva: cursor, puertas,
conjunto de lo que suena, digitación, reparto de manos y nombres de nota. Todo headless.

**Lo que sí es puerta obligatoria del rendimiento** es la función pura que produce la escena:

```sh
cargo test -p piano-core --release vista_presupuesto
```

Comprueba que calcular la lista de rectángulos y etiquetas de un fotograma cabe holgadamente en el
presupuesto, con una canción densa. Es determinista y no necesita pantalla, así que sí puede
bloquear un cambio.

### Comprobaciones concretas que conviene conocer

```sh
cargo test -p piano-core digitacion    # incluye la escala canónica de Do mayor (SC-011)
cargo test -p piano-core manos         # las tres guardas y el corte por altura
cargo test -p piano-core cursor        # modo espera, acordes simultáneos, saltar, cambiar de modo
cargo test -p piano-core sonando       # acierto, nota extra, nota omitida (FR-014a)
```

## Nivel 2 — el banco de fotogramas, que necesita pantalla

```sh
cargo run -p piano-bench --release --bin fotogramas
```

**No es una puerta de integración continua, y no puede serlo.** Está medido: con la vista creada
pero sin ventana visible, `requestAnimationFrame` dispara **cero veces en cinco segundos**, y
minimizada corre degradada. Los runners no tienen pantalla. Se ejecuta a mano o de noche, y publica
un informe.

**Cinco cifras**, y la primera es la que manda:

| | Criterio | Umbral |
| --- | --- | --- |
| a | Déficit de fotogramas respecto a 60/s | < 0,5 % |
| b | Intervalos por encima de 25 ms | < 0,1 % |
| c | Intervalos por encima de 33,4 ms | **cero** |
| d | Suspensiones de la vista (> 200 ms) | detectadas, excluidas y **declaradas** |
| e | Coste de CPU del dibujo, medido aparte | < 16,7 ms |

**Por qué no es el percentil del intervalo entre fotogramas**, que es lo que la especificación pedía
al principio: con el vsync enganchado ese intervalo vale 16,667 ms por construcción, mida lo que
mida el dibujo. La prueba que lo zanjó: un pintor que **no dibuja nada** dio p95 34 ms y 16 % de
intervalos por encima de 25 ms — peor que el pintor real con 59 notas. El recuento de fotogramas sí
distingue; el percentil no.

**Y la ventana tiene que estar visible y al frente.** En la primera medición se perdieron 430 de
600 segundos porque la vista se suspende en silencio al quedar tapada. El banco detecta esas
suspensiones y las declara; un banco que no lo hiciera publicaría un número inventado.

Números de referencia (macOS, Apple Silicon, dpr 2, 10 minutos, 59 notas visibles, 118 etiquetas):
**60,00 fotogramas por segundo, déficit 0,006 %, cero intervalos por encima de 33,4 ms.**

## Nivel 3 — con teclado, a mano

```sh
pnpm tauri dev
```

Abre una canción, elige tu teclado y toca. Comprueba:

1. Las teclas que pulsas se iluminan sin retraso apreciable.
2. En modo espera, la canción avanza a tempo entre notas y te espera en la que fallas.
3. Un acorde solo pasa cuando tienes las notas pulsadas a la vez.
4. Eliges una mano y la otra sigue viéndose, pero no te la exige.
5. Saltas a mitad de la pieza y la práctica queda lista ahí.

## Lo que este quickstart todavía NO valida

- **Windows.** WebView2 es Chromium y su cadencia sigue al monitor: a 120 o 165 Hz el presupuesto
  por fotograma no son 16,7 ms sino 8,3 o 6,1. Nada de esto se ha medido allí. El diseño lo mitiga
  derivando la posición del reloj y nunca del número de fotograma, así que la cadencia afecta a la
  suavidad pero no a la corrección — pero medirlo sigue pendiente.
- **Pantallas de dpr mayor que 2.** Todo se midió en dpr 2. Un monitor 5K multiplica el relleno, al
  que Canvas 2D es sensible.
