# Modelo de datos: evaluar la interpretación

**Feature**: `004-evaluar-interpretacion` | **Fase**: 1 | **Fecha**: 2026-08-19

Todo vive en `piano-core`. Nada de esto cruza a TypeScript sin aplanar: la interfaz recibe un
resultado ya juzgado y no conoce ninguna tolerancia.

## Lo que llega, y lo que hay que construir con ello

La captura de la feature 002 entrega **ataques y sueltas por separado** —`Observacion { at, key,
velocity, kind, channel }`—, no duraciones. Verificado en `core/src/capture/evento.rs`. El evaluador
tiene que casarlos él, y eso trae dos casos que hay que decidir aquí y no descubrir después:

- Una tecla que sigue **hundida cuando la interpretación termina**: no tiene suelta. Su duración es
  desconocida, no cero.
- Una suelta **sin ataque previo** dentro de la interpretación: pasa cuando el alumno ya tenía la
  tecla pulsada al empezar.

Como la duración se mide pero no se juzga (FR-006), ninguno de los dos casos puede alterar un
veredicto. Lo que no pueden es corromper el recuento ni entrar en pánico.

## Entidades

### `Pulsacion`

Lo que el alumno tocó, ya con sus dos extremos casados.

| Campo | Tipo | Notas |
|---|---|---|
| `key` | `u8` | Altura MIDI |
| `ataque_us` | `Micros` | Sellado con el reloj de sesión |
| `final_us` | `Option<Micros>` | `None` si seguía hundida al cerrar la interpretación |
| `velocity` | `u8` | Del ataque; la de la suelta se descarta |

### `Emparejamiento`

La correspondencia entre una pulsación y una nota, o la constancia de que no la hay (FR-001).

| Campo | Tipo | Notas |
|---|---|---|
| `pulsacion` | `usize` | Índice en la lista de pulsaciones del intento |
| `nota` | `Option<usize>` | Índice en `Song::notes`. `None` = no corresponde a ninguna |

**Invariante uno a uno (FR-002)**: ningún `nota` se repite entre dos emparejamientos, y ninguna
pulsación aparece dos veces. Es una biyección parcial, y hay que poder afirmarlo en una prueba.

### `Medida`

Solo existe para una pulsación emparejada.

| Campo | Tipo | Notas |
|---|---|---|
| `desfase_us` | `i64` | **Con signo**: negativo se adelantó, positivo se atrasó (FR-005) |
| `duracion_us` | `Option<i64>` | Diferencia con lo escrito, con signo. `None` si la tecla seguía hundida |
| `velocity` | `u8` | Se registra, no se juzga |

`i64` y no `u64` porque el signo **es** la información: sin él no se puede distinguir ir adelantado
de ir atrasado, que es la mitad de FR-016.

### `Veredicto`

Uno por nota de la canción dentro del tramo, o por pulsación suelta.

De una **nota**:

- `Acertada` — recibió su pulsación dentro de tolerancia
- `Omitida` — nadie la tocó
- `FueraDeAlcance` — cae fuera de las 88 teclas (FR-014)
- `NoIntentada` — el alumno saltó ese pasaje con la salida del modo espera (FR-013), o el tramo no
  llegó hasta ella (FR-014b)

De una **pulsación** sin nota:

- `DeMas` — no corresponde a nada de la pieza
- `DedoQueSeEscapa` — muy próxima en tiempo y altura a una que sí se acertó (FR-010a)

`NoIntentada` y `FueraDeAlcance` existen para que el denominador del porcentaje sea honesto: SC-009
exige calcularlo sobre lo que el alumno **sí intentó**, y una nota que no puede tocar o a la que no
llegó no es un fallo suyo.

### `Nivel` y `Tolerancias`

**El único sitio donde vive un umbral.** El Principio I lo exige textualmente: nunca constantes
dispersas.

| Campo | Tipo | Notas |
|---|---|---|
| `ventana_ataque_us` | `u64` | Absoluta, no escala con la velocidad (FR-008a) |
| `cercania_dedo_us` | `u64` | Y su equivalente en semitonos, para el dedo que se escapa |
| `cercania_dedo_semitonos` | `u8` | |
| `mediana_sistematico_us` | `u64` | Umbral de la mediana para llamarlo sistemático (FR-016) |
| `dispersion_sistematico_us` | `u64` | Recorrido intercuartílico máximo para llamarlo sistemático |
| `minimo_notas_sistematico` | `usize` | Por debajo, «sistemático» no significa nada |

Tres niveles por omisión. **SC-006 exige que el permisivo nunca dé menos aciertos que el exigente**,
y eso no se cumple solo: hay que imponer que las ventanas estén ordenadas por inclusión y poder
afirmarlo en una prueba.

### `Interpretacion`

El tramo que va de poner en marcha a parar (FR-014a).

| Campo | Tipo | Notas |
|---|---|---|
| `desde_us` / `hasta_us` | `Micros` | Posiciones **de canción**, no de reloj |
| `modo` | `Avance` | `PorReloj` o `PorAcierto`; decide si los tiempos se evalúan (FR-009a) |
| `pulsaciones` | `Vec<Pulsacion>` | |
| `saltadas` | rango de notas | Las que se pasaron con la salida del atasco |

### `Resultado`

Lo que se le enseña al alumno.

| Campo | Tipo | Notas |
|---|---|---|
| `acertadas` / `omitidas` / `de_mas` / `dedos_escapados` | `usize` | FR-015 |
| `no_intentadas` / `fuera_de_alcance` | `usize` | Fuera del denominador |
| `desfase` | `Option<Sistematico>` | `None` si no lo hay o si no se midieron tiempos |
| `parcial` | `bool` | `true` si se practicó en modo espera: **hay que declararlo** (FR-015a) |
| `por_mano` | `[Recuento; 2]` | FR-018 |
| `por_nota` | `Vec<Veredicto>` | Para situar cada acierto y cada fallo (FR-017) |

### `Sistematico`

| Campo | Tipo | Notas |
|---|---|---|
| `mediana_us` | `i64` | Con signo: adelantado o atrasado |
| `dispersion_us` | `u64` | Recorrido intercuartílico |

## Orden y comparación

`FR-020` exige un orden **total** y **léxico**: primero `acertadas`, y solo al empatar decide la
desviación (menor mediana absoluta es mejor). Prohibido combinarlos en una puntuación con pesos.

Que sea total significa que `comparar(a, b)` siempre devuelve mayor, menor o igual. `FR-020a` lo
dice: «no se puede saber» no es una respuesta admisible.

## Lo que este modelo NO tiene

- **Ningún historial.** FR-023: el resultado vive mientras dura la sesión.
- **Ninguna puntuación agregada** ni nota numérica. FR-020 lo prohíbe expresamente.
- **Ningún juicio sobre duración, intensidad, pedal ni fraseo** (FR-006, FR-026).
