# Implementation Plan: Practicar una canción

**Branch**: `003-practicar-una-cancion` | **Date**: 2026-08-18 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `/specs/003-practicar-una-cancion/spec.md`

## Summary

La primera interfaz del producto, construida sobre la misma regla que ya salvó a `midi-io`:
**separar lo que decide de lo que pinta**.

Cuatro decisiones sostienen el diseño, y las dos de rendimiento están **medidas**, no razonadas:

1. **Canvas 2D, un solo lienzo, redibujado completo por fotograma.** Medido durante 10 minutos
   reales con 59 notas visibles y 118 etiquetas de texto: **60,00 fotogramas por segundo, déficit
   del 0,006 %, cero fotogramas por encima de 33 ms**. El techo real está en 439 notas visibles,
   once veces el punto de trabajo. El DOM quedó **refutado por medida**: 37 fps con la misma carga.
2. **El cursor no cruza el puente sesenta veces por segundo.** Cruza un *ancla* —posición, instante,
   ritmo y próximo tope— solo cuando cambia el régimen, y la interfaz interpola entre anclas. Los
   eventos de tecla sí se empujan uno a uno por un `Channel`, desde un **hilo reenviador propio**:
   nunca desde el consumidor de tiempo real, porque enviar cuesta hasta 13 ms en el peor caso y eso
   arruinaría el presupuesto del Principio IV.
3. **El cursor es un dato, no un comportamiento.** El modo de práctica es un campo de un byte que
   decide dónde está el techo del avance, no un trait ni una jerarquía. La velocidad es un racional
   de enteros, coherente con la prohibición de coma flotante en el núcleo.
4. **La digitación se calcula con programación dinámica sobre costes enteros**, así que es
   determinista por construcción, que es justo lo que SC-010 exige.

**Ninguna dependencia nueva.** La puerta del Principio III sigue dando tres líneas.

## Technical Context

**Language/Version**: Rust 1.97.1 (edition 2021) para toda la lógica; TypeScript 5.8 y React 19
sobre Vite 7 para la interfaz.

**Primary Dependencies**: **ninguna nueva.** El dibujo es Canvas 2D, que es del navegador; el
puente es `tauri::ipc::Channel`, que ya viene con Tauri; la digitación es aritmética entera propia.
La puerta de `cargo tree -p piano-core` sigue dando exactamente tres líneas —`piano-core`,
`midi_file`, `rtrb`— sin tocar nada.

Se evaluaron y descartaron: bibliotecas de dibujo (PixiJS, regl, two.js), por meter dependencia de
terceros para un problema medido en 0,3 ms sobre un presupuesto de 16,7; y estructuras de índice
espacial para el recorte de lo visible, innecesarias por dos órdenes de magnitud.

**Storage**: la preferencia de teclado que ya existe, más —posiblemente— el punto de corte de manos
por canción. Nada de la interpretación del alumno (FR-029).

**Testing**: `cargo test` para toda la lógica de dominio, y **Vitest con un renderizador de
componentes** para la interfaz. Los componentes de React toman decisiones —qué archivo abrir, qué
mostrar ante un error, qué emite cada control— así que se prueban como cualquier otra cosa que
decide. Lo único sin prueba automática es la función que pinta.

**Target Platform**: macOS y Windows. La interfaz corre en la WebView del sistema —WKWebView en
macOS, WebView2 en Windows—, no en un Chromium empaquetado, lo que importa para el rendimiento.

**Project Type**: desktop-app. Es la primera feature que produce interfaz.

**Performance Goals**:

- p99 de fotograma < 16,7 ms durante 10 minutos (SC-003).
- Reflejo de una tecla en pantalla < 50 ms p95 desde que el sistema entrega el mensaje (SC-004).
- 1.000 notas en pantalla en < 2 s (SC-002); saltar en < 100 ms (SC-008a).

**Constraints**:

- El núcleo sigue sin dependencias de sistema: la puerta de `cargo tree -p piano-core` debe seguir
  dando exactamente tres líneas.
- Prohibido el punto flotante en el núcleo. El dibujo sí usa coordenadas en coma flotante, así que
  la frontera entre «calcular» y «pintar» es también la frontera entre enteros y píxeles.
- Determinismo: la digitación propuesta debe ser idéntica en 100 ejecuciones (SC-010).
- Sin red, sin telemetría, sin sonido.

**Scale/Scope**: piezas de hasta ~10 minutos y ~10.000 notas, de las cuales unas decenas visibles a
la vez.

## La tensión de esta feature: probar una interfaz

Las dos features anteriores fueron fáciles de defender ante el Principio II: todo era lógica pura y
todo se probaba. Ésta trae pantalla, y la pantalla no se presta al mismo trato.

La estrategia es la misma que funcionó con `midi-io`, y por el mismo motivo: **separar lo que
decide de lo que pinta**. Dónde va cada nota en un instante dado, qué dedo se propone, de qué mano
es, si una tecla pulsada corresponde a algo que suena, cuándo avanza el cursor — todo eso es
aritmética pura y va al núcleo, donde se prueba sin ventana. Lo que queda del otro lado es una
función que recibe una lista de rectángulos y etiquetas y los dibuja, **sin tomar ni una decisión**.

Si la capa de dibujo empieza a decidir algo, esa lógica está en el archivo equivocado. Es
exactamente la regla que ya rige para `midi-io`, y el plan debe declarar qué archivos son esa capa,
igual que exige la excepción del Principio II en la Constitución v1.1.0.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principio | Estado | Cómo lo cumple este plan |
| --- | --- | --- |
| **I. Precisión Musical Primero** | PASA | Todo el cálculo temporal sigue en enteros dentro del núcleo. La conversión a píxeles ocurre en el último paso y no realimenta nada: un error de redondeo al dibujar no puede desplazar la música. |
| **II. TDD estricto** | PASA, con excepción declarada | Toda la lógica nueva —cursor, digitación, reparto de manos, coincidencia— es pura y se prueba headless. La excepción acogida es la función de dibujo, y se mantiene sin decisiones para que lo no cubierto sea trivial (ver sección anterior y Complexity Tracking). |
| **III. Núcleo desacoplado de la UI** | PASA, y es lo que la hace posible | La feature que introduce interfaz es precisamente la que más se apoya en que el núcleo no la conozca. La puerta de `cargo tree -p piano-core` sigue vigente sin cambios. |
| **IV. Tiempo real con presupuesto** | PASA, ampliado | Hasta ahora el presupuesto cubría hasta el consumidor en Rust (p95 medido 18–25 µs). Esta feature extiende la medición **hasta el píxel**, que es lo que el Principio IV siempre quiso decir y no se había podido medir. |
| **V. Local primero** | PASA | El archivo lo abre el usuario de su disco. Sin red, sin telemetría. |

## Project Structure

### Documentation (this feature)

```text
specs/003-practicar-una-cancion/
├── plan.md              # Este archivo
├── spec.md              # 46 requisitos, 14 criterios, 9 clarificaciones
├── research.md          # Fase 0
├── data-model.md        # Fase 1
├── quickstart.md        # Fase 1
├── contracts/           # Fase 1
├── checklists/
│   └── requirements.md  # 16/16
└── tasks.md             # Fase 2
```

### Source Code (repository root)

```text
core/                              # EXISTE. Suma un módulo; sigue sin dependencias de sistema
└── src/practica/                  # NUEVO: toda la lógica de esta feature, probada headless
    ├── mod.rs
    ├── cursor.rs                  # Cursor: modo como CAMPO, velocidad racional, sin flotantes
    ├── puertas.rs                 # los topes del modo espera, precalculados
    ├── sonando.rs                 # qué notas suenan en un instante (decisión D9)
    ├── sesion.rs                  # SesionDePractica<C: Clock, F: FuenteDeEventos>
    ├── manos.rs                   # voces separadas, o corte por altura ajustable
    └── nombres.rs                 # Do/Re/Mi, sostenidos y bemoles

core/src/digitacion/               # NUEVO: módulo propio, no un archivo suelto
├── mod.rs                         # la programación dinámica de segundo orden
├── tablas.rs                      # la tabla de vanos de Parncutt, como datos
└── coste.rs                       # las doce reglas, en aritmética i32

src-tauri/                         # EXISTE. Deja de ser el andamio
└── src/
    ├── lib.rs                     # el reloj de sesión, ya presente, más el estado gestionado
    ├── comandos.rs                # abrir canción, registrar canal, transporte, ajustes
    └── reenviador.rs              # hilo que drena el anillo y empuja por el Channel

src/                               # EXISTE (hoy el andamio de Vite). Se sustituye entero
├── App.tsx
├── practica/
│   ├── Lienzo.tsx                 # LA CAPA SIN DECISIONES: recibe y pinta
│   ├── modelo.ts                  # interpolación entre anclas; sin lógica musical
│   └── controles.tsx              # transporte, velocidad, modo, mano, corte
└── dispositivos/Selector.tsx      # elegir teclado (la pantalla que la 002 aplazó)

bench/                             # EXISTE. Suma el banco de fotogramas
└── src/bin/fotogramas.rs          # arranca ventana real, mide y publica las cinco cifras
```

**Structure Decision**: la frontera entre `core/src/practica/` y `src/practica/Lienzo.tsx` es la
misma idea que separa `core/` de `midi-io/`, y por el mismo motivo. Todo lo que responde *dónde va
esta nota, qué dedo, de qué mano, coincide o no, avanza o no* es aritmética pura y vive en Rust,
donde se prueba sin ventana. `Lienzo.tsx` recibe una lista ya calculada y la pinta.

Un dato que respalda esa frontera, y que se midió intentando refutarla: la versión que **asigna una
lista nueva en cada fotograma** —unos 4,3 millones de objetos en diez minutos— dio 0,1315 ms frente
a 0,1310 ms de la versión que reutiliza memoria. La frontera es gratis. No hay que optimizarla.

### Re-evaluación tras el diseño de la Fase 1

| Principio | Estado tras el diseño | Qué lo confirma |
| --- | --- | --- |
| **I. Precisión Musical** | PASA, reforzado | La velocidad de práctica es un **racional de enteros**, no un factor en coma flotante: reducir a la mitad y volver no acumula error. La posición se deriva del reloj monótono, nunca del número de fotograma. |
| **II. TDD estricto** | PASA, con la excepción declarada y acotada | Cursor, digitación, manos, nombres y coincidencia son aritmética pura y se prueban headless. **Archivo acogido a la excepción: `src/practica/Lienzo.tsx`**, y solo él. |
| **III. Núcleo desacoplado** | PASA, sin cambios | Cero dependencias nuevas. La puerta sigue dando tres líneas. |
| **IV. Tiempo real** | PASA, ampliado hasta el píxel | Se midió el recorrido completo con una aplicación Tauri real y el p95 no supera los 50 ms. Y el diseño protege el presupuesto anterior: enviar por el puente cuesta hasta 13 ms en el peor caso, así que **nunca** ocurre en el hilo de tiempo real, sino en un reenviador aparte. |
| **V. Local primero** | PASA | Sin red. El archivo lo abre el usuario. |

## Complexity Tracking

| Violation | Why Needed | Simpler Alternative Rejected Because |
|-----------|------------|-------------------------------------|
| La función que dibuja no queda cubierta por pruebas automáticas | Comprobar que unos píxeles concretos acabaron en la pantalla exige una ventana real y comparación de imágenes, que es frágil y lento. | Las pruebas de imagen de referencia se descartaron: se rompen con cualquier cambio de fuente, de escala o de versión del sistema, y acaban desactivándose. La mitigación es estructural: la capa de dibujo no toma decisiones, así que lo que queda sin cubrir es «pinta este rectángulo aquí». Todo lo que decide **dónde** va ese rectángulo sí está probado. El plan debe enumerar los archivos acogidos, como exige la excepción del Principio II. |
| **SC-003 no se puede verificar en integración continua**, y no por falta de esfuerzo: está medido que es imposible | Con la WebView creada pero no adjunta a una ventana visible, `requestAnimationFrame` disparó **cero veces en 5.001 ms**. Minimizada corre degradada a 53,8 Hz. Los runners no tienen pantalla. | Ejecutar un servidor de pantalla virtual se descartó: mediría el rendimiento de un simulador, no el de la WebView del sistema, que es justo lo que el criterio quiere proteger. La verificación se desdobla: **puerta obligatoria de PR** sobre la función pura que produce la lista de rectángulos y etiquetas —determinista, headless, con presupuesto duro—, y **trabajo manual o nocturno con ventana real** para las cinco cifras de fotograma, publicado como informe y **no** como bloqueo. Es menos de lo que uno querría, y decirlo es preferible a fingir una puerta que no existe. |
| **SC-003, tal y como lo redactó la especificación, es inmedible y hay que reescribirlo** | Con el vsync enganchado, el intervalo entre fotogramas vale 16,667 ms por construcción, mida lo que mida el dibujo. La prueba decisiva: un pintor que **no dibuja nada** dio p95 34 ms y 16 % de fotogramas sobre 25 ms, **peor** que el pintor real con 59 notas. | Dejar el criterio como está fue descartado en cuanto se midió: es un criterio que un no-pintor falla y un pintor real aprueba, o sea que no mide lo que dice medir. La Decisión 1 de `research.md` propone las cinco cifras que sí significan algo, empezando por el **déficit de fotogramas** —cuántos faltan respecto a 60 por segundo— en lugar del percentil. **Requiere enmendar la especificación antes de implementar.** |
| La ruta de Windows no ha sido medida ni una sola vez | No hay máquina Windows. WebView2 es Chromium y su cadencia sigue al monitor: a 120 o 165 Hz el presupuesto por fotograma no son 16,7 ms sino 8,3 o 6,1. | Extrapolar desde macOS se descartó como afirmación, pero sí se aplicó como **mitigación estructural**: la posición de cada nota se deriva siempre del reloj monótono y **nunca del número de fotograma**, con lo que la cadencia de la pantalla deja de afectar a la corrección, solo a la suavidad. Queda pendiente repetir el barrido en Windows; el arnés es portable. |
