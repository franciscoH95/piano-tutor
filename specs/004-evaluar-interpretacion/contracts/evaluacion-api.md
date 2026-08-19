# Contrato: evaluación y su puente

**Feature**: `004-evaluar-interpretacion` | **Fecha**: 2026-08-19

Las mismas tres fronteras de la 003, y esta feature vive entera en la primera.

```
piano-core          juzga            probado headless, con interpretaciones grabadas
    │
src-tauri           traduce          el puente, sin lógica musical
    │
src/evaluacion      muestra          sin decisiones
```

## Núcleo: `piano_core::evaluacion`

```rust
/// Cuánta exigencia. Las tolerancias viven AQUÍ y en ningún otro sitio.
pub enum Nivel { Permisivo, Intermedio, Exigente }
impl Nivel {
    pub const fn tolerancias(self) -> Tolerancias;
}

/// La máquina que juzga, alimentada en línea.
pub struct Evaluador { /* ... */ }

impl Evaluador {
    /// Empieza una interpretación en la posición actual.
    pub fn nueva(cancion: &Song, manos: &RepartoDeManos, nivel: Nivel, desde: Micros) -> Self;

    /// Un ataque o una suelta del alumno.
    ///
    /// **Ruta crítica**: coste constante respecto al tamaño de la canción y sin asignar
    /// memoria. Es lo que permite que el Principio IV siga cumpliéndose.
    pub fn observar(&mut self, obs: Observacion, posicion: Micros);

    /// La práctica avanzó hasta aquí sin novedades. Cierra las notas cuya ventana ya pasó.
    pub fn avanzar(&mut self, posicion: Micros);

    /// El alumno saltó la puerta pendiente: ese pasaje no se intentó (FR-013).
    pub fn saltar(&mut self, desde: Micros, hasta: Micros);

    /// Cierra la interpretación y calcula el resumen.
    ///
    /// **Fuera de la ruta crítica**: aquí sí se puede ordenar, calcular medianas y recorrer
    /// la lista entera, porque el alumno ya ha parado.
    pub fn cerrar(self, hasta: Micros) -> Resultado;
}

/// Orden total y léxico: primero aciertos, luego desviación (FR-020, FR-020a).
pub fn comparar(a: &Resultado, b: &Resultado) -> core::cmp::Ordering;
```

**Garantías del núcleo**:

- `observar` no asigna memoria y su coste no depende del tamaño de la canción.
- El emparejamiento es **uno a uno** y no se revisa: una nota ya juzgada no cambia de veredicto por
  lo que venga después (FR-002, FR-004).
- Todo es determinista, incluido el desempate entre pulsaciones simultáneas (SC-005, SC-008).
- Ninguna tolerancia se lee fuera de `Nivel::tolerancias`.

## Puente: `src-tauri`

```rust
#[tauri::command] fn evaluacion_nivel(nivel: String);
#[tauri::command] fn evaluacion_ultimo() -> Option<ResultadoPlano>;
#[tauri::command] fn evaluacion_comparar_con_anterior() -> Option<Comparacion>;
```

El resultado cruza **aplanado**: recuentos, la mediana con su signo, el indicador de parcial y el
veredicto de cada nota. **Ninguna tolerancia cruza el puente**, ni siquiera para mostrarla: si la
interfaz supiera lo que es una ventana de 60 ms, esa constante estaría en dos sitios.

Los nombres de los campos siguen la convención que la 003 dejó fijada por prueba: **camello en los
dos caminos**, tanto en los mandos como en el canal (`contrato_canal_test.rs`).

## Capa que muestra: `src/evaluacion/Resumen.tsx`

**No está acogida a la excepción del Principio II**: se prueba como cualquier componente. Decide qué
se enseña primero y cómo se redacta, y eso son decisiones.

Reglas:

- No recalcula nada. Recibe recuentos y los muestra.
- Si el resultado viene marcado como parcial, **lo dice** (FR-015a). Un resumen que calla que no se
  midieron los tiempos se lee como completo.
- «No se tocó nada» se muestra como tal, nunca como 0 % (FR-019, SC-002).

## Lo que este contrato NO ofrece

- No guarda historial entre sesiones (FR-023).
- No produce una nota ni una puntuación única (FR-020).
- No juzga duración, intensidad, pedal ni fraseo (FR-006, FR-026).
- No produce sonido (FR-024) ni envía nada fuera (FR-025).
