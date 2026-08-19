# Implementation Plan: Evaluar la interpretación

**Branch**: `004-evaluar-interpretacion` | **Date**: 2026-08-19 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `/specs/004-evaluar-interpretacion/spec.md`

## Summary

Medir con cuánta precisión tocó el alumno y decírselo: emparejar cada pulsación con la nota que le
corresponde, medir el desfase de ataque, contar aciertos, omisiones y notas de más, y detectar si
hay un desfase sistemático en vez de presentar veinte fallos sueltos.

Todo el juicio vive en `piano-core`, sin ventana ni teclado, alimentado por interpretaciones
grabadas. La feature 003 ya dejó la línea temporal, el cursor con su tempo de práctica y la
clasificación acierto/extra/omitida; esta añade **la medida y el veredicto**, y no vuelve a
construir la base.

## Technical Context

**Language/Version**: Rust 1.97.1 (edition 2021) en el núcleo; TypeScript 5.8 + React 19 en la
interfaz. Sin cambios respecto a las tres features anteriores.

**Primary Dependencies**: **ninguna nueva en el núcleo.** `cargo tree -p piano-core` debe seguir
dando exactamente tres líneas (`piano-core`, `midi_file`, `rtrb`). Todo lo que esta feature necesita
—mediana, cuartiles, emparejamiento— es aritmética entera sobre datos que ya existen.

**Storage**: ninguno. FR-023 deja fuera el historial: el resultado vive mientras dura la sesión.

**Testing**: `cargo test` con interpretaciones grabadas como fixtures; `vitest` para la interfaz.
Ninguna prueba requiere teclado ni pantalla.

**Target Platform**: macOS y Windows, igual que el resto.

**Project Type**: aplicación de escritorio (Tauri), con núcleo de dominio separado.

**Performance Goals**: evaluar una interpretación completa de 10 minutos en menos de 1 segundo
(SC-007). El emparejamiento ocurre **en línea**, una pulsación cada vez, así que su coste por evento
debe ser constante respecto al tamaño de la canción.

**Constraints**:

- **Sin coma flotante** en el núcleo (`deny(clippy::float_arithmetic)`). Condiciona el cálculo del
  desfase sistemático: mediana y recorrido intercuartílico en vez de media y desviación típica.
- **Sin indexar directamente** (`deny(clippy::indexing_slicing)`).
- **Sin asignaciones no acotadas** en la ruta crítica (Principio IV, <30 ms p95). El emparejamiento
  corre en el mismo camino que el reflejo de teclas.
- **Determinismo absoluto** (Principio I y SC-005): misma entrada, misma salida, en cualquier
  máquina y en cualquier orden de ejecución.

**Scale/Scope**: piezas de hasta 24 horas y decenas de miles de notas, como ya soporta el cargador.
Una interpretación real rara vez pasa de unos miles de pulsaciones.

## Constitution Check

*GATE: debe pasar antes de la investigación de la Fase 0. Se vuelve a comprobar tras el diseño.*

### I. Precisión Musical Primero (NO NEGOCIABLE)

| Exigencia | Cómo la cumple esta feature |
|---|---|
| Juzgar por altura, ataque, duración y velocity | FR-005 a FR-007 miden las cuatro. La duración y la velocity se **miden pero no se juzgan**, y está declarado |
| Derivar el tiempo del reloj de sesión | El instante esperado sale del cursor de la 003, que ya usa ese reloj (FR-008) |
| Tolerancias explícitas y configurables por nivel, nunca dispersas | FR-011, FR-011a y FR-012. **Es la exigencia que más condiciona el diseño**: todas las tolerancias en un solo lugar, incluidas las del dedo que se escapa y las del desfase sistemático |
| Determinismo | FR-003, FR-021, SC-005, SC-008 |
| Fixtures de referencia con su resultado esperado | FR-022. Es la deuda declarada de esta feature: sin ellos, un ajuste de tolerancia rompe otro caso en silencio |

**Veredicto**: cumple. Esta feature **es** el motor de evaluación que el Principio I llama «el
producto», así que el principio se aplica aquí con todo su peso.

### II. Desarrollo Guiado por Pruebas (NO NEGOCIABLE)

Todo el juicio es lógica pura sobre datos en memoria: se prueba entero, sin hardware ni ventana.
**Esta feature no necesita la excepción acotada de adaptadores de plataforma.** No se añade ningún
archivo a la lista de exentos; `src/practica/Lienzo.tsx` sigue siendo el único, y sigue limitándose
a pintar.

**Veredicto**: cumple, sin excepciones que declarar.

### III. Núcleo Determinista Desacoplado de la UI

Emparejar, medir y juzgar son decisiones de dominio y viven en `piano-core`. La interfaz recibe un
resultado ya calculado y lo muestra. Ninguna tolerancia, ningún umbral y ninguna regla de
comparación cruzan a TypeScript.

**Veredicto**: cumple. La puerta 3 de `verificar.sh` lo sigue comprobando sola.

### IV. Tiempo Real con Presupuesto (<30 ms)

El emparejamiento corre en la ruta crítica: llega una pulsación y hay que decidir. Por tanto:

- coste por pulsación **constante**, no dependiente del tamaño de la canción;
- **sin asignar** memoria por evento;
- el resumen —recuentos, mediana, cuartiles— se calcula **al cerrar la interpretación**, no en cada
  pulsación, porque ahí sí hay tiempo.

**Veredicto**: cumple si el diseño respeta lo anterior. Es lo primero que la Fase 0 tiene que fijar.

### V. Local Primero y Propiedad del Usuario

FR-023 no guarda nada y FR-025 no envía nada fuera. No hay red, no hay cuenta, no hay telemetría.

**Veredicto**: cumple trivialmente.

### Puertas de calidad

Las cinco de `verificar.sh` siguen aplicando sin cambios. La quinta —el banco de latencia— cubre la
ruta crítica en la que ahora también vive el emparejamiento.

## Project Structure

### Documentation (this feature)

```text
specs/004-evaluar-interpretacion/
├── plan.md              # Este archivo
├── research.md          # Fase 0
├── data-model.md        # Fase 1
├── quickstart.md        # Fase 1
├── contracts/           # Fase 1
├── checklists/
│   └── requirements.md  # de /speckit-specify, 16/16
└── tasks.md             # de /speckit-tasks, todavía no
```

### Source Code (repository root)

```text
core/src/
├── evaluacion/                 # NUEVO: todo el juicio
│   ├── mod.rs                  # Evaluador: la máquina en línea
│   ├── emparejar.rs            # qué pulsación va con qué nota
│   ├── tolerancias.rs          # LOS NIVELES, EN UN SOLO SITIO (Principio I)
│   ├── estadistica.rs          # mediana y cuartiles, en enteros
│   └── resultado.rs            # recuentos, desfase sistemático, comparación
├── practica/                   # existente; se lee, no se reescribe
└── timeline.rs                 # existente

core/tests/
├── emparejar_test.rs           # NUEVO
├── evaluacion_test.rs          # NUEVO
├── estadistica_test.rs         # NUEVO
└── fixtures/
    └── interpretaciones/       # NUEVO: las grabadas de FR-022

src/evaluacion/                 # NUEVO: mostrar el resultado
├── Resumen.tsx
└── Resumen.test.tsx

src-tauri/src/comandos.rs       # existente; se le añaden los mandos del resultado
```

## Complexity Tracking

*Se rellena cuando el diseño esté cerrado y medido. Por ahora, el único punto donde la complejidad
puede dispararse es el emparejamiento en línea: `FR-004` prohíbe revisar decisiones a la luz de lo
que venga después, lo que descarta el emparejamiento óptimo global y obliga a un algoritmo que
decide sobre la marcha. La Fase 0 tiene que fijar cuánta precisión se pierde por no mirar el futuro
y dejarlo escrito, no descubrirlo durante la implementación.*
