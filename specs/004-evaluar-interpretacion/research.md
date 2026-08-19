# Investigación: evaluar la interpretación

**Feature**: `004-evaluar-interpretacion` | **Fase**: 0 | **Fecha**: 2026-08-19

Análisis adversarial del diseño antes de escribir código: cuatro lentes independientes sobre el
emparejamiento, la tolerancia, la estadística y el ámbito, cada una refutada por un escéptico. De 20
propuestas sobrevivieron 16. Lo que sigue son las decisiones, con el motivo por el que se toman.

**Las afirmaciones sobre el código existente están verificadas ejecutándolo**, no citadas de
memoria. Las tres que sostienen decisiones aquí se comprobaron una por una.

## Decisión 1: dos ventanas, no una

**Decisión**: separar la ventana de **emparejamiento** (`W`, idéntica en los tres niveles) de la
ventana de **ataque** (`A`, distinta por nivel). El emparejamiento es independiente del nivel; el
nivel solo decide el veredicto de una nota **ya emparejada**.

**Motivo**: convierte `SC-006` de propiedad que hay que comprobar en **consecuencia estructural**.
Si el emparejamiento no depende del nivel y las ventanas de ataque están anidadas
(`A_permisivo ⊇ A_intermedio ⊇ A_exigente`), entonces el permisivo no puede dar menos aciertos que
el exigente: es aritmética, no una prueba que alguien pueda romper ajustando un número.

Con una sola ventana, cambiar de nivel cambiaría **qué se empareja con qué**, y una nota podría
quedar acertada en el nivel exigente y sin pareja en el permisivo. Eso rompe SC-006 de una forma que
solo se ve con el fixture adecuado.

**Consecuencia**: hace falta un veredicto más, `TocadaFueraDeTiempo` — emparejada pero fuera de `A`.
El modelo de datos lo incorpora.

**Alternativa descartada**: una sola ventana por nivel. Más simple de explicar, pero deja SC-006 a
merced de que nadie toque los números sin mirar los otros dos.

## Decisión 2: todo se mide en el eje del reloj de sesión

**Decisión**: la nota se proyecta **hacia el reloj** invirtiendo el ancla, y ese instante esperado se
**sella una sola vez y no se recalcula nunca**. Nunca se lleva la pulsación al eje de canción.

**Motivo**: `FR-004` prohíbe que una nota ya juzgada cambie de veredicto por lo que venga después.
Recalcular el instante esperado al cambiar la velocidad sería exactamente eso. Sellar al cruzar lo
hace imposible por construcción.

Además, `posicion_en` **recorta por el tope** —lo hace el cursor de la 003, verificado en
`cursor.rs`—, así que usarla para llevar una pulsación al eje de canción truncaría la tardanza en
silencio: con la última nota en el segundo 100 y el final en el 200, un ataque real en el 350 se
proyectaría como 200 y el desfase saldría mal sin que nada fallase.

**La inversa exacta es el techo, no el suelo**: `posicion_en` aplica `floor`, y
`⌊Δ·num/den⌋ ≥ D ⟺ Δ ≥ ⌈D·den/num⌉`. Una sola división, en `u128`, con `try_from` a la salida y
**sin un solo `as`**.

**Por qué `as` está prohibido aquí y no basta con `i128`**: si se usa `u64::MAX` como centinela de
«sin sellar», `u64::MAX as i64` vale `−1`, que es un desfase de un microsegundo **adelantado** y
dentro de cualquier tolerancia. El centinela se convertiría en un acierto perfecto. Contra eso no
protege el ancho del entero: protege no usar `as`.

## Decisión 3: la omisión es un vencimiento del cursor, no un temporizador

**Decisión**: una nota vence cuando el cursor **rebasa estrictamente** su ataque y han pasado `W` de
reloj real desde ese cruce.

**Motivo**: en modo espera el cursor se detiene **exactamente** en el ataque de la puerta pendiente
—el techo móvil de la 003 lo clava ahí—. Con el criterio flojo «posición ≥ ataque», la nota que el
cursor está esperando arrancaría su cuenta atrás y se declararía omitida a los pocos segundos,
justo antes de que el alumno la acierte y abra la puerta. El estricto lo elimina por construcción.

**Alternativa descartada**: un temporizador de reloj real. En modo espera correría mientras la
canción espera, que es precisamente cuando no debe correr.

## Decisión 4: un solo oráculo del veredicto

**Verificado ejecutándolo**, no supuesto:

- `EstadoNota::Acertada` y `EstadoNota::Omitida` **no se producen en ningún punto del núcleo**.
  `vista.rs` solo emite `Pendiente` y `Sonando`.
- **`ConjuntoSonando` no tiene ni un solo llamador de producción**: solo su propio archivo de
  pruebas y el reexport de `practica/mod.rs`.

Es decir, la clasificación acierto/extra/omitida que la fase 5 de la 003 construyó y probó **no la
usa nadie**. No hay dos oráculos vivos; hay código sin consumidor que invita a crear el segundo.

**Decisión**: la 004 **cablea** su veredicto a `EstadoNota` y **retira** la mitad juzgadora de
`sonando.rs`. Se conservan `MascaraTeclas` y `vigentes()`, que las puertas sí usan.

**Motivo**: dos sitios que deciden «acertada» pueden discrepar, y discreparían en silencio: el
pentagrama pintaría una cosa y el resumen diría otra. El Principio I exige un solo criterio.

## Decisión 5: qué es evaluable, en una sola función

**Decisión**: una única `es_evaluable(nota, mano, practicada)` que consumen **a la vez** el programa
de puertas y el evaluador.

**Motivo, y defecto real que esto destapó**: `ProgramaDePuertas::nuevo` llevaba escrito que «la
percusión no genera puertas» y **no lo hacía**. Reproducido: con batería en el canal 9, el cursor se
quedaba clavado en la posición 0 esperando una caja. Ya está corregido (commit aparte), pero la
lección es la decisión: si el filtro vive en dos sitios, volverán a divergir.

`is_on_88_keys()` no basta —una caja está en la tecla 38, dentro del piano—: hay que mirar el canal.

## Decisión 6: la estadística, en enteros y con cuidado con el signo

**Decisión**: mediana y recorrido intercuartílico sobre enteros **con signo**, con atención expresa
a dos trampas:

- **La división entera de negativos trunca hacia cero en Rust**, no hacia abajo. Con un número par
  de elementos, la mediana de `[−3, −2]` calculada como `(−3 + −2) / 2` da `−2` y no `−3`. Hay que
  fijar la regla y probarla con valores negativos, no solo positivos.
- **Un mínimo de notas** para que «sistemático» signifique algo. Con dos notas la mediana existe y
  no dice nada.

**Limitación aceptada y declarada**: si el alumno **acelera progresivamente** —empieza a tempo y
acaba corriendo—, la mediana lo esconde. Es una limitación real de esta medida. Se acepta para esta
entrega y se deja escrita, en vez de descubrirla cuando un usuario pregunte por qué no se lo dijeron.

## Decisión 7: el régimen manda por nota, no por intento

**Decisión**: si los tiempos de una nota se evalúan o no depende del régimen vigente **en el momento
de sellarla**, no de un booleano del intento entero.

**Motivo**: el alumno puede cambiar de modo a mitad. Con un booleano del intento habría que decidir
si se descarta todo o se evalúa todo, y las dos son falsas. Por nota es exacto, y `FR-004` obliga de
todos modos a no recalcular.

## Lo que queda sin resolver, a propósito

- **Los valores concretos de las tolerancias**. El análisis propone cifras, pero fijarlas es una
  decisión de producto que conviene tomar con la tabla de fixtures delante. Lo que la especificación
  ya exige —un solo sitio, ventanas anidadas— está garantizado por la Decisión 1.
- ~~**Cuánta precisión se pierde por no mirar el futuro**~~ → **medido el 2026-08-19**, ver abajo.

## Medición: el precio de FR-004

`FR-004` descarta el emparejamiento óptimo global. El plan dejó **sin suponer** cuánto cuesta eso, y
ahora está medido en `core/tests/perdida_en_linea_test.rs`, comparando el emparejamiento en línea
con el óptimo con visión de futuro sobre casos elegidos para que difieran si van a diferir —notas
repetidas de la misma tecla, que es el único sitio donde la elección tiene alternativa—.

| Caso | Óptimo | En línea | Pérdida |
|---|---|---|---|
| una nota, una pulsación | 1 | 1 | 0 |
| dos repetidas, dos pulsaciones | 2 | 2 | 0 |
| dos repetidas, una pulsación en medio | 1 | 1 | 0 |
| tres repetidas, dos pulsaciones tardías | 2 | 2 | 0 |
| **la pulsación se lleva la nota equivocada** | **2** | **1** | **1** |
| diez repetidas tocadas a destiempo | 10 | 10 | 0 |

**Peor caso medido: un emparejamiento.**

El caso que pierde es concreto y merece entenderse. Notas de la misma tecla en 1,000 s y 1,100 s;
pulsaciones en 1,090 s y 1,600 s. En línea, la primera pulsación toma la nota **más cercana** —la de
1,100— y deja varada la de 1,000; después, la pulsación de 1,600 ya no alcanza a la de 1,000, que
queda a 600 ms, fuera de la ventana. El óptimo habría dado la primera pulsación a la nota de 1,000 y
la segunda a la de 1,100, emparejando las dos.

**Se acepta.** La alternativa es revisar decisiones ya tomadas, que es exactamente lo que `FR-004`
prohíbe y por un motivo que vale más que un emparejamiento: un veredicto que cambia solo porque el
alumno siguió tocando es un veredicto en el que no se puede confiar. La prueba deja la cota fijada
en uno; si algún día sube, es que el emparejamiento ha empeorado.
