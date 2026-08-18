# Data Model: Practicar una canción

**Feature**: `003-practicar-una-cancion` | **Fecha**: 2026-08-18
**Fuente de las decisiones**: [research.md](./research.md)

Todo lo que aparece aquí vive en `piano-core`, sin dependencias de sistema y sin coma flotante. La
interfaz recibe estos datos ya calculados; no los deriva.

## El cursor

### `Avance`

```
PorReloj     el tiempo mueve el cursor de principio a fin
PorAcierto   el tiempo lo mueve hasta el próximo tope, y ahí espera
```

Un byte, `Copy`, **sin comportamiento**. No es un trait ni un enum con métodos: es un dato que
decide *dónde está el techo* del avance. Esa fue la decisión de diseño: el modo de práctica no es
una jerarquía de tipos, es un campo.

### `Velocidad`

Racional de enteros (`num`/`den`), no un factor en coma flotante. Pausa es `0/1`.

Importa más de lo que parece: reducir a la mitad, practicar diez minutos y volver a velocidad
normal **no acumula error** — cosa que con un `f64` multiplicado fotograma a fotograma no se puede
garantizar, y el Principio I prohíbe la deriva.

### `MascaraTeclas`

`[u64; 2]` = 128 bits, 16 bytes, `Copy`. Qué teclas están hundidas ahora, y cuáles ya se han
consumido para satisfacer la puerta actual. Comprobar un acorde completo es un `and` de máscaras,
no un recorrido.

### `Cursor`

| Campo | Para qué |
| --- | --- |
| `avance` | Cuál de los dos regímenes rige |
| `velocidad` | Proporción respecto al tempo original |
| `ancla_real`, `ancla_cancion` | Instante del reloj y posición de canción en el último rebase |
| `pos` | Posición actual dentro de la canción |
| `puertas`, `puerta` | El programa de topes y cuál es el primero pendiente |
| `hundidas`, `consumidas` | Qué se está tocando y qué ya cuenta para la puerta |
| `fin` | Dónde termina la pieza |

**Invariante del modo espera** (decisión D5): el cursor avanza gobernado por el reloj **hasta un
tope móvil**. No se detiene el tiempo: se detiene la posición al llegar a la puerta pendiente. Por
eso entre notas se percibe la figura rítmica.

## Las puertas

### `ProgramaDePuertas`

Precalculado al cargar, inmutable, compartible. Una puerta por instante de ataque: qué teclas hay
que tener pulsadas **a la vez** para que el cursor pueda pasar (decisión D6).

Ordenado por posición, recorrido con un cursor monótono igual que `CueSchedule`: el coste de
comprobar si se puede avanzar no depende del tamaño de la canción.

## Qué suena ahora

### `ConjuntoSonando`

Responde la pregunta de la decisión D9: *¿está esta nota sonando en este instante?*, entendiendo por
tal que la posición actual caiga entre su ataque y su final.

La línea temporal está ordenada por **ataque**, no por final, así que la consulta no es inmediata.
Se resuelve con un cursor de entrada más una cota superior de duración de nota: se avanza mientras
el ataque haya pasado, y se descartan las que ya terminaron. Medido: menos de 1 µs por fotograma
con 10.000 notas.

De aquí salen las tres situaciones de FR-014a: **acierto** (tecla pulsada que suena), **nota
extra** (pulsada que no suena) y **nota omitida** (sonó entera sin que se pulsara).

## Digitación

### `Dedo`

1 a 5, con el convenio del piano: 1 es el pulgar, 5 el meñique. En ambas manos.

### El modelo

Reglas ergonómicas de **Parncutt et al. (1997)**: doce reglas sobre una tabla de vanos en
semitonos, resueltas con **programación dinámica exacta de segundo orden** en aritmética `i32`.
Determinista por construcción, que es lo que exige SC-010.

**Dos convenios de los que depende todo lo demás**, y que si se codifican mal rompen el resultado
en silencio:

1. **El vano se mide siempre del dedo de número menor al mayor.** Para el par (3,1) con un
   intervalo ascendente de +3 semitonos, el vano canónico es **−3**, no +3. Sin esto el paso del
   pulgar no se detecta nunca, y el paso del pulgar es la mitad de la técnica de las escalas.
2. **La mano izquierda es la derecha reflejada**: la altura relativa a la mano es `h(p) = p` para la
   derecha y `h(p) = −p` para la izquierda. Las mismas tablas sirven para las dos. El color de la
   tecla, en cambio, se consulta siempre sobre la altura MIDI **real**, que no se refleja.

La tabla de vanos completa está en [research.md](./research.md), Decisión 4, y va al código como
datos, no como condicionales: vive en `core/src/digitacion/tablas.rs`, separada de las reglas
(`coste.rs`) y del algoritmo (`mod.rs`).

## Reparto de manos

### `Voz`

El par `(track, channel)` con al menos una nota, descartado el canal 9 (percusión). `ScheduledNote`
ya conserva ambos campos desde la feature 001.

### Cuándo se considera que el archivo trae las manos separadas

Las **tres** guardas a la vez:

| | Guarda |
| --- | --- |
| **G1** | Hay exactamente dos voces con notas |
| **G2** | Mismo instrumento: mismo canal, o mismo programa, o ninguna lo declara |
| **G3** | Cada voz tiene al menos el 5 % de las notas **y** sus medianas de altura difieren en 3 semitonos o más |

Si se cumplen: **la mano derecha es la voz cuya mediana de altura es más alta**, nunca la de índice
de pista menor. Desempate determinista por `(track, channel)` ascendente.

Si no se cumplen: corte por altura nota a nota, **umbral por defecto 60 (Do central)**, ajustable.
`key >= corte` va a la derecha; `key < corte`, a la izquierda.

**El control del corte está siempre disponible**, con «usar las voces del archivo» como valor por
defecto cuando se detectan. No se oculta nunca, porque la heurística puede equivocarse y el alumno
necesita poder discrepar.

## Nombre de la nota

### `NombreDeNota`

Valor **simbólico** `{ base, alteración }`, **no una cadena**. El formateo pertenece a la capa que
pinta: el núcleo no sabe de textos ni de idiomas. Es el Principio III aplicado a algo tan pequeño
como una etiqueta.

- **Base**: Do, Re, Mi, Fa, Sol, La, Si.
- **Alteración**: ninguna, sostenido o bemol. Al pintar se usan los signos musicales ♯ y ♭, nunca la
  almohadilla ni la letra be.

### Cómo se decide sostenido o bemol

Un número de tecla MIDI no lo dice: el 61 es Do♯ o Re♭ según el contexto. Se resuelve con un **mapa
de armaduras por tick**, fusionando todas las pistas, con la misma forma que el `TempoMap` que ya
existe. Para una nota en el tick `t` se toma la última armadura con tick ≤ `t`:

| Armadura | Tabla |
| --- | --- |
| Bemoles (`sf < 0`) | Do, Re♭, Re, Mi♭, Mi, Fa, Sol♭, Sol, La♭, La, Si♭, Si |
| Sostenidos (`sf >= 0`) | Do, Do♯, Re, Re♯, Mi, Fa, Fa♯, Sol, Sol♯, La, La♯, Si |
| Sin declarar | Sostenidos |

**Simplificación declarada**: una tecla blanca nunca lleva alteración. No hay Mi♯ ni Do♭, aunque
existan en la teoría. Para etiquetar una nota que cae, la diferencia no compensa la complejidad.

**La octava no se muestra.** El sitio de la etiqueta lo ocupan el nombre y el dedo.

## La sesión

### `SesionDePractica<C: Clock, F: FuenteDeEventos>`

Reúne el reloj, la fuente de pulsaciones, el cursor, el conjunto de lo que suena y la reproducción.
Los genéricos se conservan donde ya existían —reloj y fuente— para no pagar despacho dinámico y
para poder ejercerla entera sin hardware.

**Transiciones que hay que dejar coherentes** (FR-007b, FR-021):

```
saltar(a)          -> se recolocan cursor y puertas; se vacían hundidas y consumidas
cambiar de modo    -> se conserva la posición; se recalcula el tope
cambiar velocidad  -> se rebasa el ancla; la posición no salta
mover el corte     -> se recalculan manos Y digitación (FR-003c)
```
