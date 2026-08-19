# Feature Specification: Evaluar la interpretación

**Feature Branch**: `004-evaluar-interpretacion`

**Created**: 2026-08-19

**Status**: Draft

**Input**: Feature elegida sobre lo que la 003 aplazó explícitamente (FR-027): medir con cuánta
precisión se tocó, contar aciertos y fallos, y decirle al alumno cómo le fue.

## Clarifications

### Session 2026-08-19

- Q: ¿Qué convierte un conjunto de desfases en un «desfase sistemático»? → A: Mediana y dispersión:
  la mediana de los desfases supera un umbral **y** el recorrido intercuartílico es pequeño. Se
  elige frente a la desviación típica porque mediana y recorrido intercuartílico se calculan con
  aritmética entera, y el núcleo prohíbe la coma flotante.
- Q: Al comparar dos interpretaciones, una con menos fallos y otra con mejor ritmo, ¿cuál es mejor?
  → A: Notas primero, ritmo como desempate. Gana quien acierta más notas; solo si empatan decide la
  desviación. Da un orden total sin inventar pesos, y coincide con el orden en que se aprende: nadie
  pule el ritmo de un pasaje cuyas notas todavía se equivoca.
- Q: Si el alumno toca la nota correcta en el momento correcto pero la suelta enseguida, ¿cuenta como
  acertada? → A: Sí. La duración se mide pero no se juzga, igual que la intensidad. Juzgarla exigiría
  otra tolerancia por nivel y decidir cómo interactúa con el pedal, que queda fuera (FR-026); y sin
  sonido el alumno no puede oír si sostuvo bien, así que castigarlo por algo que no percibe sería
  injusto en esta entrega.
- Q: ¿Qué delimita una «interpretación» si el alumno pausa, salta o repite sin llegar al final? →
  A: Un intento va de poner en marcha a parar; pausar, saltar o llegar al final lo cierran, y
  reanudar abre otro. Coincide con una frontera que el núcleo ya calcula —el cursor cambia de
  régimen exactamente ahí— y hace que repetir un pasaje produzca intentos comparables sin pedirle
  nada al alumno.
- Q: A mitad de velocidad, ¿la tolerancia de ataque se duplica o sigue siendo la misma en
  milisegundos? → A: Absoluta, en milisegundos fijos. Un desfase de 60 ms suena igual de mal a
  cualquier tempo porque el oído mide en milisegundos, no en fracciones de negra. Con la tolerancia
  fija, practicar despacio es de verdad más fácil de clavar y subir la velocidad se siente como
  progreso; con la tolerancia relativa, el mismo alumno obtiene el mismo resultado a cualquier
  velocidad y no aprende nada de subirla. **Sustituye la redacción anterior de FR-008.**

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Saber cómo me fue al terminar (Priority: P1)

El alumno toca una pieza de principio a fin y, al acabar, quiere saber cómo lo hizo: cuántas notas
acertó, cuántas se dejó, cuántas tocó de más, y si en general va adelantado o atrasado respecto al
tempo.

**Why this priority**: Es lo mínimo que convierte «practicar» en «practicar y aprender». Sin esto la
aplicación acompaña pero no enseña, y el Principio I dice que el motor de evaluación es el producto.

**Independent Test**: Se alimenta al núcleo una interpretación grabada —una lista de pulsaciones con
sus instantes— contra una canción conocida, y se comprueba el resumen resultante. No hace falta ni
teclado ni pantalla.

**Acceptance Scenarios**:

1. **Given** una pieza de 20 notas y una interpretación que las toca todas dentro de la tolerancia,
   **When** termina la reproducción, **Then** el resumen dice 20 acertadas, 0 omitidas y 0 de más.
2. **Given** la misma pieza y una interpretación que se salta 3 notas y toca 2 que no existían,
   **When** termina, **Then** el resumen dice 17 acertadas, 3 omitidas y 2 de más.
3. **Given** una interpretación en la que todas las notas llegan 40 ms tarde, **When** termina,
   **Then** el resumen comunica un retraso sistemático, no 20 fallos independientes.

---

### User Story 2 - Ver en qué parte de la pieza fallo (Priority: P2)

El alumno no quiere solo un número: quiere saber **dónde**. Un resumen que dice «80 %» no le sirve
para decidir qué repasar mañana; uno que dice «los compases 9 a 12 con la izquierda» sí.

**Why this priority**: Un número sin localización no cambia lo que el alumno hace después, y cambiar
lo que hace después es todo el objetivo. Va después de P1 porque necesita que P1 exista.

**Independent Test**: Con la misma interpretación grabada, se comprueba que el resultado sitúa cada
acierto y cada fallo en su posición de la canción y en su mano.

**Acceptance Scenarios**:

1. **Given** una interpretación con todos los fallos concentrados en la segunda mitad, **When**
   termina, **Then** el resultado los sitúa allí y no repartidos.
2. **Given** una pieza a dos manos donde la izquierda va bien y la derecha mal, **When** termina,
   **Then** el resultado distingue una mano de la otra.

---

### User Story 3 - Ajustar cuánto se me exige (Priority: P2)

Un principiante y alguien que lleva años no pueden medirse con la misma vara. El alumno elige cuánta
exigencia quiere, y la evaluación se ajusta.

**Why this priority**: Sin esto, la evaluación desanima a quien empieza o resulta inútil a quien
avanza. El Principio I lo exige expresamente: las tolerancias MUST ser configurables por nivel de
dificultad, nunca constantes dispersas.

**Independent Test**: La misma interpretación grabada, evaluada en dos niveles distintos, da
resultados distintos y coherentes: lo que en el nivel exigente es un fallo, en el permisivo no.

**Acceptance Scenarios**:

1. **Given** una nota tocada 60 ms tarde, **When** se evalúa en el nivel más permisivo, **Then**
   cuenta como acertada; **When** se evalúa en el más exigente, **Then** no.
2. **Given** un nivel elegido, **When** se vuelve a evaluar la misma interpretación, **Then** el
   resultado es idéntico.

---

### User Story 4 - Repetir un pasaje y ver si mejoro (Priority: P3)

El alumno repite el mismo fragmento varias veces seguidas y quiere ver si la última vez le salió
mejor que la primera.

**Why this priority**: Es lo que sostiene una sesión de estudio real, pero solo tiene sentido cuando
las tres anteriores existen.

**Independent Test**: Dos interpretaciones del mismo pasaje, una claramente mejor, dan resultados
comparables entre sí y ordenables.

**Acceptance Scenarios**:

1. **Given** dos intentos del mismo fragmento, **When** el segundo tiene menos fallos y menos
   desviación, **Then** se puede afirmar que fue mejor sin ambigüedad.

---

### Edge Cases

- El alumno no toca **nada** en toda la pieza: el resultado no puede ser un error ni un 0 % sin
  explicación; es «no se tocó nada», que es distinto de tocar mal.
- El alumno toca **mucho más** de lo que la pieza pide (por ejemplo, apoya el antebrazo): las notas
  de más no pueden inutilizar la evaluación de las que sí acertó.
- Una nota que la pieza pide **dos veces seguidas** en la misma tecla, y el alumno la toca una sola
  vez sosteniéndola: hay que decidir si eso es una acertada y una omitida, o dos acertadas.
- Un acorde en el que el alumno toca las notas **desplegadas** en vez de a la vez.
- Una pieza cuyas notas caen fuera de las 88 teclas: el alumno no puede tocarlas y no debe cargar
  con esas omisiones.
- Cambiar de velocidad a mitad de la interpretación: la desviación se mide contra el tempo de
  práctica, no contra el del archivo.
- Un pasaje que el alumno **salta** con la salida del modo espera (FR-020 de la 003): no puede
  contar como fallado, porque no llegó a intentarlo.

## Requirements *(mandatory)*

### Functional Requirements

**Emparejar lo tocado con lo escrito**

- **FR-001**: El sistema MUST decidir, para cada pulsación del alumno, a qué nota de la canción
  corresponde, o que no corresponde a ninguna.
- **FR-002**: Una pulsación MUST NOT emparejarse con más de una nota de la canción, ni una nota de
  la canción recibir más de una pulsación.
- **FR-003**: El emparejamiento MUST ser determinista: la misma interpretación sobre la misma
  canción produce siempre el mismo resultado.
- **FR-004**: El emparejamiento MUST NOT depender de lo que el alumno toque **después**: una nota ya
  juzgada no cambia de veredicto por lo que venga luego.

**Medir**

- **FR-005**: Para cada nota emparejada, el sistema MUST medir el **desfase de ataque**: cuánto se
  adelantó o se atrasó respecto al momento en que debía sonar, con su signo.
- **FR-006**: Para cada nota emparejada, el sistema MUST medir la **diferencia de duración**
  respecto a lo escrito, con su signo. La duración se **mide pero no se juzga**: MUST NOT alterar el
  veredicto de la nota, igual que la intensidad. Queda registrada y se comunica como dato.
- **FR-007**: Para cada nota emparejada, el sistema MUST registrar la **intensidad** con que se
  tocó.
- **FR-008**: El **instante esperado** de cada nota MUST calcularse contra el tempo de práctica
  vigente, no contra el del archivo: a mitad de velocidad, una negra dura el doble y el momento en
  que debía sonar se mueve con ella.
- **FR-008a**: La **tolerancia**, en cambio, MUST ser absoluta en milisegundos y MUST NOT escalar
  con la velocidad. El oído mide en milisegundos, no en fracciones de negra: un desfase de 60 ms
  suena igual de mal a cualquier tempo. Que la tolerancia escalase haría que bajar la velocidad no
  exigiese más precisión, y bajar la velocidad es precisamente como se gana precisión: el alumno
  obtendría el mismo resultado a cualquier tempo y no aprendería nada de subirlo.

**Juzgar**

- **FR-009**: El sistema MUST clasificar cada nota de la canción en exactamente una de: acertada,
  omitida, o fuera del alcance del alumno.
- **FR-010**: El sistema MUST clasificar cada pulsación que no corresponde a ninguna nota como nota
  de más.
- **FR-010a**: El sistema MUST distinguir, dentro de las notas de más, las que son un **dedo que se
  escapa**: una pulsación muy próxima en tiempo y en altura a una nota que sí se acertó. Es el error
  más frecuente de un principiante —roza el Fa y toca el Mi—, y contarlo igual que tocar un pasaje
  entero equivocado castiga dos veces el mismo tropiezo y esconde qué clase de error fue.
- **FR-011**: Las tolerancias MUST estar definidas explícitamente y ser **configurables por nivel de
  dificultad**, nunca constantes dispersas por el código.
- **FR-011a**: La cercanía que convierte una nota de más en un dedo que se escapa MUST ser una
  tolerancia explícita más, sujeta a la misma regla que las demás: definida en un solo sitio y
  configurable por nivel.
- **FR-012**: El sistema MUST ofrecer al menos tres niveles de exigencia, y el alumno MUST poder
  cambiarlos.
- **FR-009a**: En **modo espera** el sistema MUST evaluar las notas —acertada, de más, omitida— y
  MUST NOT evaluar los tiempos. Cuando la canción aguarda al alumno, el desfase de ataque no mide
  nada: no se puede llegar tarde a algo que te espera, y publicar ese número sería inventarlo.
- **FR-013**: Un pasaje saltado con la salida del modo espera MUST quedar marcado como no intentado,
  y MUST NOT contar como fallado.
- **FR-014**: Las notas que caen fuera de las 88 teclas MUST quedar fuera del alcance del alumno y
  MUST NOT contar como omitidas.

**Contar y comunicar**

- **FR-014a**: El sistema MUST tratar como una **interpretación** cada tramo que va de poner en
  marcha a parar. Pausar, saltar y llegar al final lo cierran; reanudar abre otro. Es la misma
  frontera que el cursor ya usa para cambiar de régimen, así que no introduce un concepto nuevo.
- **FR-014b**: Una interpretación que no llega al final MUST evaluarse igualmente, sobre el tramo
  que el alumno sí recorrió. Exigir un recorrido completo dejaría sin ningún retorno al principiante,
  que casi nunca termina.
- **FR-014c**: Dos intentos del **mismo tramo** MUST poder compararse entre sí (FR-020). Repetir un
  pasaje es la forma normal de estudiar, y es lo que sostiene la Historia 4.
- **FR-015**: Al terminar una interpretación, el sistema MUST comunicar cuántas notas se acertaron,
  cuántas se omitieron y cuántas se tocaron de más, **sobre el tramo recorrido**.
- **FR-015a**: Cuando el resultado sea parcial —porque se practicó en modo espera y los tiempos no
  se han medido—, el sistema MUST decirlo. Un resultado incompleto que no se declara incompleto se
  lee como completo, y el alumno creería que su ritmo está bien cuando nadie lo ha mirado.
- **FR-016**: El sistema MUST comunicar si hay un **desfase sistemático** —ir adelantado o atrasado
  de forma consistente— y distinguirlo de fallos sueltos. Hay desfase sistemático cuando la
  **mediana** de los desfases supera un umbral **y** su **recorrido intercuartílico** es pequeño:
  casi todas las notas se desvían en la misma dirección y en cantidad parecida. Los dos umbrales
  MUST estar sujetos a la misma regla que las demás tolerancias (FR-011).
- **FR-016a**: La medida del desfase sistemático MUST calcularse en **aritmética entera**. Mediana y
  recorrido intercuartílico se eligieron por eso: una desviación típica exigiría una raíz cuadrada,
  y con ella una excepción al Principio III o una aproximación que haría el resultado dependiente de
  la implementación y rompería SC-005.
- **FR-017**: El sistema MUST situar cada acierto y cada fallo en su posición dentro de la canción.
- **FR-018**: El sistema MUST separar el resultado **por mano** cuando la canción tenga las dos.
- **FR-019**: El sistema MUST distinguir «no se tocó nada» de «se tocó mal».
- **FR-020**: Dos interpretaciones de la misma pieza MUST poder compararse entre sí para decir cuál
  fue mejor. El criterio es **léxico**: manda el número de notas acertadas, y solo cuando empatan
  decide la desviación de ataque. MUST NOT combinarse en una puntuación única con pesos: unos pesos
  arbitrarios reordenarían en silencio interpretaciones ya juzgadas cada vez que se ajustasen, y
  serían justo la clase de constante dispersa que el Principio I prohíbe.
- **FR-020a**: La comparación MUST ser un orden total: para cualquier par de interpretaciones de la
  misma pieza el sistema MUST poder decir cuál fue mejor, o que fueron equivalentes. «No se puede
  saber» no es una respuesta admisible, porque es justo cuando el alumno más quiere saberlo.

**Reproducibilidad**

- **FR-021**: Toda evaluación MUST ser determinista: la misma secuencia de pulsaciones produce
  siempre el mismo resultado, en cualquier máquina.
- **FR-022**: Cualquier cambio en las reglas de evaluación MUST ir acompañado de interpretaciones de
  referencia grabadas con su resultado esperado, y MUST declarar cuáles cambian de resultado y por
  qué.

### Límites del alcance

- **FR-023**: El sistema MUST NOT guardar el historial entre sesiones. Ver la evolución a lo largo
  de las semanas es de la entrega siguiente; aquí el resultado vive mientras dura la sesión.
- **FR-024**: El sistema MUST NOT producir sonido, igual que la 003.
- **FR-025**: El sistema MUST NOT enviar nada fuera del dispositivo.
- **FR-026**: El sistema MUST NOT juzgar el pedal, la dinámica como intención musical, ni el fraseo.
  Mide lo que un archivo MIDI contiene, no lo que un profesor oiría.

### Key Entities

- **Pulsación**: una tecla que el alumno tocó, con su altura, su instante de ataque, su instante de
  suelta y su intensidad.
- **Emparejamiento**: la correspondencia entre una pulsación y una nota de la canción, o la
  constancia de que no la hay.
- **Medida**: para una nota emparejada, su desfase de ataque, su diferencia de duración y su
  intensidad.
- **Veredicto**: la clasificación de una nota de la canción —acertada, omitida, fuera de alcance, no
  intentada— o de una pulsación suelta —nota de más, o dedo que se escapa.
- **Nivel de exigencia**: el conjunto de tolerancias que decide qué cuenta como acertado.
- **Resultado**: el recuento, el desfase sistemático y la localización de todo lo anterior.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Una interpretación nota por nota perfecta obtiene el 100 % de aciertos, 0 omitidas y 0
  de más, en los tres niveles de exigencia.
- **SC-002**: Una interpretación en la que no se toca ninguna tecla se comunica como «no se tocó
  nada», y **no** como una interpretación con el 100 % de fallos.
- **SC-003**: Con todas las notas desplazadas la misma cantidad dentro de la tolerancia, el 100 % se
  cuentan acertadas y se comunica el desfase sistemático.
- **SC-004**: Con todas las notas desplazadas la misma cantidad **fuera** de la tolerancia, se
  comunica el desfase sistemático en vez de presentar cada nota como un fallo aislado.
- **SC-005**: Evaluar dos veces la misma interpretación da resultados idénticos byte a byte, en
  cualquier máquina y en cualquier orden de ejecución.
- **SC-006**: La misma interpretación evaluada en el nivel más permisivo nunca obtiene menos
  aciertos que en el más exigente.
- **SC-007**: Evaluar una interpretación completa de una pieza de 10 minutos tarda menos de 1
  segundo.
- **SC-008**: Ninguna medida ni recuento depende de en qué orden llegaron pulsaciones que ocurrieron
  en el mismo instante.
- **SC-009**: Un pasaje saltado con la salida del modo espera aparece como no intentado, y el
  porcentaje de aciertos se calcula sobre lo que el alumno sí intentó.
- **SC-010**: De dos interpretaciones de la misma pieza, una con la mitad de fallos que la otra, el
  sistema siempre señala la mejor como mejor.
- **SC-011**: Una interpretación hecha entera en modo espera comunica el recuento de notas **y**
  declara que los tiempos no se han evaluado. Un resultado parcial que no se declare parcial se
  considera un fallo del sistema, no una limitación aceptable.
- **SC-012**: La misma interpretación, tocada a mitad de velocidad con los mismos desfases absolutos,
  obtiene el **mismo** número de aciertos que a velocidad normal. Bajar la velocidad no regala
  tolerancia; lo que regala es tiempo para acertar.
- **SC-013**: Rozar la tecla contigua y tocar acto seguido la correcta se comunica como un dedo que
  se escapa, no como una nota de más equiparable a tocar un compás entero equivocado. El acierto
  sigue contando como acierto.

## Assumptions

- **La evaluación no se guarda**: el resultado vive mientras dura la sesión. Persistirlo e ir viendo
  la evolución es la entrega siguiente, y separarlas mantiene esta acotada.
- **Se evalúa sobre lo que ya existe**: la línea temporal, el reparto de manos y la clasificación
  acierto/extra/omitida de la 003. Esta feature añade la medida y el juicio, no vuelve a construir
  la base.
- **Tres niveles de exigencia** por omisión: uno permisivo para empezar, uno intermedio y uno
  exigente, y los tres se distinguen por el ancho de una ventana en milisegundos. Los valores
  concretos son una decisión de diseño y se fijan en el plan; lo que la especificación exige es que
  vivan en un solo sitio y que el más permisivo nunca dé menos aciertos que el más exigente
  (SC-006).
- **La duración y la intensidad se registran pero no se juzgan**: el veredicto lo decide el ataque.
  Juzgar la duración exigiría otra tolerancia por nivel y decidir cómo interactúa con el pedal, que
  queda fuera (FR-026). Y sin sonido (FR-024) el alumno no puede oír si sostuvo bien, así que
  castigarlo por algo que no percibe sería injusto; cuando haya sonido, la puerta queda abierta.
- **La intensidad se registra pero no se juzga**: medir si el alumno tocó fuerte o flojo es un dato
  útil; decidir si «debía» tocar así es interpretación musical, y queda fuera (FR-026).
- **El modo espera da un resultado parcial, y lo dice** (FR-009a, FR-015a): se evalúan las notas y
  no los tiempos. La alternativa —no evaluar nada en modo espera— dejaría sin ningún retorno
  precisamente al principiante, que es quien vive ahí. Lo que no es aceptable es callarlo: un
  resultado incompleto que no se declara incompleto se lee como completo.
- **Emparejamiento por cercanía**: cada pulsación se empareja con la nota más cercana en el tiempo
  que tenga su misma altura y siga libre.
- **Una nota equivocada y luego la correcta son dos sucesos, no uno** (FR-010a): la equivocada es
  una pulsación de más y la correcta un acierto. Pero se distingue el **dedo que se escapa** del
  error de verdad, para no castigar dos veces el mismo tropiezo ni perder de vista qué clase de
  error fue. Es información que un profesor usaría: rozar teclas se corrige de otra manera que
  equivocarse de pasaje.
