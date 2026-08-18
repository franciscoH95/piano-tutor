# Feature Specification: Practicar una canción

**Feature Branch**: `003-practicar-una-cancion`

**Created**: 2026-08-18

**Status**: Draft

**Input**: User description: "La primera feature con interfaz. Cierra el bucle mínimo del producto: el alumno abre un archivo de canción, ve en pantalla qué tiene que tocar, lo toca en su teclado y la aplicación responde. Incluye los dos modos de práctica decididos el 2026-08-18: a tempo fijo y modo espera, ambos moviendo un mismo cursor de posición. No incluye evaluación ni puntuación, que llegan después."

## Clarifications

### Session 2026-08-18

- Q: ¿Cómo se mide que las notas cayendo se ven fluidas? → A: El **99 % de los fotogramas** debe
  dibujarse en menos de **16,7 ms** (60 por segundo) a lo largo de una pieza de diez minutos.
  Igualar la frecuencia real de la pantalla sería lo ideal, pero convertiría el criterio en algo
  dependiente de la máquina y por tanto imposible de afirmar de forma estable en integración
  continua.

- Q: En modo espera, ¿qué gobierna el control de velocidad? → A: Entre notas la canción **avanza a
  la velocidad elegida**, y se detiene al llegar a una nota que el alumno aún no ha tocado. Así el
  alumno sigue percibiendo la figura rítmica, que es lo que salva la tensión entre el modo espera y
  el Principio I: el ritmo no desaparece, solo se vuelve indulgente.

- Q: ¿Qué control tiene el alumno sobre dónde está dentro de la canción? → A: Puede **saltar a
  cualquier punto**. El bucle de repetición A-B queda para más adelante: añade estado propio,
  interfaz y decidir cómo se comporta el modo espera dentro del bucle.

- Q: Si la aplicación asigna una nota a la mano equivocada, ¿puede el alumno corregirlo? → A: Sí,
  mediante un **punto de corte por altura ajustable**, que solo entra en juego cuando el archivo no
  trae las manos separadas. Un editor de reasignación nota a nota resolvería también las manos
  cruzadas, pero es una feature en sí misma y no cabe en la primera interfaz del producto.

- Q: A tempo fijo, ¿qué cuenta como que lo tocado «coincide con lo esperado»? → A: Que la canción
  tenga esa nota **sonando en ese instante**, es decir, que el momento actual caiga entre su ataque
  y su final. Es una comprobación exacta sobre datos que el núcleo ya tiene y no introduce ninguna
  ventana de tolerancia, que es lo que FR-027 aparta para la feature de evaluación.

- Q: ¿Cómo se representa la canción en pantalla? → A: Notas cayendo sobre el teclado, **con el
  nombre de la nota y el dedo sugerido escritos sobre cada una**. No partitura: el núcleo no
  guarda claves, compases ni alteraciones, y añadirlas obligaría a reabrir la feature 001.
- Q: Los archivos MIDI no contienen digitación. ¿De dónde sale? → A: Se **calcula** con las reglas
  estándar del piano y se presenta explícitamente como **sugerencia**, no como verdad. Una
  digitación mal impuesta es peor que ninguna: el principiante la interioriza y luego hay que
  desaprenderla.
- Q: En modo espera, ¿cuándo avanza un acorde? → A: Cuando todas sus notas estén pulsadas **a la
  vez**. Un acorde es un gesto simultáneo; aceptarlo nota a nota enseñaría un hábito a corregir.
- Q: ¿Se puede practicar una mano sola? → A: Sí. El alumno elige mano y la otra sigue **visible**
  pero no se le exige. Es el método clásico de estudio, y el núcleo ya conserva la voz de origen de
  cada nota desde la feature 001 precisamente para esto.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Abrir una canción y verla (Priority: P1)

El alumno abre la aplicación, elige un archivo de canción de su ordenador y la ve aparecer en
pantalla: el teclado abajo y las notas de la pieza dispuestas sobre él, listas para empezar.

**Why this priority**: sin esto no hay producto. Todo el trabajo hecho hasta ahora vive en motores
que nadie puede ver ni usar; ésta es la historia que los convierte en algo que se abre y se mira.

**Independent Test**: se prueba abriendo un archivo de canción y comprobando que la pieza aparece
representada con sus notas en el lugar y el momento correctos, sin necesidad de teclado conectado.

**Acceptance Scenarios**:

1. **Given** la aplicación abierta sin ninguna canción, **When** el alumno elige un archivo válido,
   **Then** la pieza aparece en pantalla y queda lista para empezar, detenida al principio.
2. **Given** un archivo que no es una canción válida, **When** se intenta abrir, **Then** la
   aplicación lo dice con un motivo comprensible y sigue funcionando.
3. **Given** una canción cargada, **When** el alumno abre otra, **Then** la anterior se sustituye
   por completo, sin restos de la primera.
4. **Given** una canción cargada, **When** el alumno mira la pantalla, **Then** distingue qué notas
   corresponden a cada mano.

---

### User Story 2 - Reproducir la canción y seguirla con la vista (Priority: P1)

El alumno pone en marcha la canción y ve avanzar la música: las notas se acercan al teclado y le
indican cuándo tocar cada una. Puede pausar, volver al principio y cambiar la velocidad para
practicar más despacio.

**Why this priority**: es el valor que la aplicación ya puede dar sin que el alumno toque nada, y
es la base sobre la que se apoyan las dos historias siguientes.

**Independent Test**: se prueba poniendo en marcha una canción y comprobando que la posición avanza
al ritmo correcto, que la pausa detiene el avance y que reducir la velocidad lo hace más lento en
la proporción exacta.

**Acceptance Scenarios**:

1. **Given** una canción cargada, **When** el alumno la pone en marcha, **Then** la posición avanza
   al tempo de la pieza y las notas llegan al teclado en su momento.
2. **Given** una canción en marcha, **When** el alumno pausa, **Then** todo se detiene y nada se
   pierde; al reanudar continúa donde estaba.
3. **Given** una canción en marcha, **When** el alumno reduce la velocidad a la mitad, **Then**
   todo transcurre al doble de tiempo, y las alturas de las notas no cambian.
4. **Given** una canción, **When** el alumno vuelve al principio, **Then** la posición se reinicia y
   la pieza queda como recién cargada.
5. **Given** una canción que llega a su fin, **When** suena la última nota, **Then** la aplicación
   lo indica y se detiene sin errores.

---

### User Story 3 - Tocar y que la aplicación responda (Priority: P1)

El alumno toca en su teclado y ve inmediatamente el reflejo en pantalla: las teclas que pulsa se
iluminan, y puede comparar de un vistazo lo que está tocando con lo que la canción le pedía.

**Why this priority**: es la primera vez que el instrumento y la aplicación se conectan. Sin esta
historia el producto es un reproductor; con ella empieza a ser un compañero de estudio.

**Independent Test**: se prueba alimentando el sistema con una secuencia de pulsaciones controlada
y comprobando que la representación en pantalla refleja exactamente esas pulsaciones, sin teclado
físico.

**Acceptance Scenarios**:

1. **Given** un teclado conectado, **When** el alumno pulsa una tecla, **Then** esa tecla se marca
   en pantalla de inmediato y deja de estarlo al soltarla.
2. **Given** una canción en marcha, **When** el alumno toca la nota que la canción pedía, **Then**
   la aplicación lo distingue de haber tocado otra distinta.
3. **Given** ningún teclado conectado, **When** el alumno abre una canción, **Then** puede verla y
   reproducirla igualmente, y la aplicación indica que no hay teclado sin bloquear nada.
4. **Given** un acorde, **When** el alumno pulsa varias teclas a la vez, **Then** todas se marcan.

---

### User Story 4 - Practicar en modo espera (Priority: P2)

El alumno activa el modo espera: la canción deja de avanzar sola y aguarda a que toque cada nota
correctamente. Así puede recorrer entera una pieza que todavía no domina, sin quedarse atascado ni
frustrarse.

**Why this priority**: es la razón por la que un principiante autodidacta llega al final de una
pieza en lugar de abandonarla. Depende de la Historia 3, pero cambia por completo la experiencia.

**Independent Test**: se prueba alimentando pulsaciones controladas y comprobando que la posición
solo avanza cuando la pulsación coincide con lo esperado, y que no avanza cuando no coincide.

**Acceptance Scenarios**:

1. **Given** el modo espera activo y la canción detenida ante una nota, **When** el alumno toca la
   nota correcta, **Then** la canción avanza hasta la siguiente y espera de nuevo.
2. **Given** el mismo estado, **When** el alumno toca una nota equivocada, **Then** la canción
   **no** avanza, y la aplicación lo indica sin castigar ni interrumpir.
3. **Given** el modo espera activo, **When** el alumno deja de tocar, **Then** la canción espera
   indefinidamente sin avanzar ni terminar.
4. **Given** una canción en modo espera, **When** el alumno cambia a tempo fijo, **Then** la
   reproducción continúa desde la misma posición, sin saltos.

---

### User Story 5 - Elegir y recordar el teclado (Priority: P3)

El alumno elige su teclado de una lista la primera vez, y las siguientes la aplicación lo reconoce
sola.

**Why this priority**: la capacidad ya está construida y probada en la feature anterior; lo que
falta es la pantalla. Con un solo teclado conectado la aplicación puede arreglárselas sin ella, así
que es comodidad, no requisito para practicar.

**Independent Test**: se prueba comprobando que la lista muestra los dispositivos disponibles, que
elegir uno inicia la captura sobre ése, y que al volver a abrir la aplicación se propone el mismo.

**Acceptance Scenarios**:

1. **Given** varios teclados conectados, **When** el alumno abre la selección, **Then** los ve
   todos con un nombre que le permite distinguirlos.
2. **Given** un teclado elegido antes, **When** el alumno abre la aplicación, **Then** se le propone
   ése sin tener que elegir de nuevo.
3. **Given** el teclado recordado ya no está, **When** se abre la aplicación, **Then** se le pide
   elegir de nuevo, y nunca se abre otro en su lugar.

---

### Edge Cases

- **Canción sin notas**: se abre, se muestra vacía y se puede reproducir hasta el final sin errores.
- **Canción muy larga o muy densa**: miles de notas no deben hacer que la pantalla se atasque ni que
  la reproducción pierda el paso.
- **Notas fuera de las 88 teclas**: existen en algunos archivos. Deben poder mostrarse o indicarse
  de algún modo, en lugar de desaparecer sin avisar.
- **El teclado se desconecta a mitad de la práctica**: la canción sigue siendo visible y
  reproducible; la aplicación avisa de que se quedó sin entrada.
- **Modo espera con una nota imposible**: si la canción pide una nota que el teclado del alumno no
  tiene, el modo espera se quedaría atascado para siempre. Debe existir una salida.
- **Cambiar de velocidad a mitad de la reproducción**: no debe provocar saltos ni desincronizar lo
  que ya está en pantalla.
- **Pausar en mitad de un acorde**: las notas que estaban sonando deben quedar en un estado
  coherente al reanudar.
- **Elegir un archivo enorme**: debe haber una respuesta visible mientras se carga, no una ventana
  congelada.

## Requirements *(mandatory)*

### Abrir una canción

- **FR-001**: El alumno MUST poder elegir un archivo de canción de su ordenador desde la aplicación.
- **FR-002**: El sistema MUST mostrar la canción cargada representada visualmente, con cada nota en
  su altura y su momento.
- **FR-003**: El sistema MUST distinguir visualmente a qué mano corresponde cada nota.
- **FR-003a**: Cuando el archivo traiga las manos en voces separadas, el sistema MUST usar esa
  información y MUST NOT deducirla por altura.
- **FR-003b**: Cuando el archivo traiga todo en una sola voz, el sistema MUST repartir las notas
  entre las dos manos por un **punto de corte por altura**, y el alumno MUST poder moverlo.
- **FR-003c**: Mover el punto de corte MUST recalcular tanto el reparto de manos como la digitación
  propuesta, porque FR-033 la calcula por manos separadas.
- **FR-004**: El sistema MUST rechazar un archivo ilegible indicando el motivo, sin dejar de
  funcionar ni quedar en un estado a medias.
- **FR-005**: Abrir una canción nueva MUST sustituir por completo a la anterior.
- **FR-006**: El sistema MUST representar la canción como notas que descienden hacia un teclado
  dibujado, de modo que la posición horizontal indique la tecla y la vertical, cuándo tocarla.
- **FR-006a**: El sistema MUST mostrar el **nombre de cada nota** sobre ella.
- **FR-006b**: El sistema MUST mostrar el **dedo sugerido** para cada nota.
- **FR-006c**: El sistema MUST NOT presentar el dedo sugerido como una obligación. Debe quedar
  claro para el alumno que es una propuesta con la que puede discrepar.

### Reproducir

- **FR-007**: El alumno MUST poder poner en marcha, pausar y volver al principio.
- **FR-007a**: El alumno MUST poder saltar a cualquier punto de la canción, sin tener que
  reproducir lo anterior. Practicar es repetir un pasaje concreto, y sin esto repetir el compás 40
  obliga a tocar los 39 anteriores cada vez.
- **FR-007b**: Saltar MUST dejar la práctica en un estado coherente: sin notas colgando de antes
  del salto y con el modo de práctica vigente intacto.
- **FR-008**: El sistema MUST hacer avanzar la canción al tempo de la pieza, respetando sus cambios
  de tempo.
- **FR-009**: El alumno MUST poder cambiar la velocidad de práctica sin que cambien las alturas.
- **FR-010**: Un cambio de velocidad a mitad de la reproducción MUST NOT provocar saltos de
  posición.
- **FR-011**: El sistema MUST indicar cuándo la canción ha terminado.
- **FR-012**: La posición dentro de la canción MUST provenir de un **cursor**, y el paso del tiempo
  y el acierto de las notas MUST ser dos formas distintas de moverlo. Cambiar entre una y otra MUST
  ser un ajuste de la sesión, no un modo de funcionamiento aparte.

### Tocar

- **FR-013**: El sistema MUST reflejar en pantalla las teclas que el alumno pulsa y suelta, mientras
  las pulsa.
- **FR-014**: El sistema MUST poder determinar si una tecla pulsada corresponde a una nota que la
  canción tiene **sonando en ese instante**, entendiendo por tal que el momento actual caiga entre
  el ataque y el final de esa nota.
- **FR-014a**: El sistema MUST distinguir tres situaciones: una tecla pulsada que la canción pedía
  (acierto), una que no pedía (nota extra) y una que la canción pedía y el alumno no tocó en ningún
  momento de su duración (nota omitida).
- **FR-014b**: El sistema MUST NOT aplicar ninguna ventana de tolerancia temporal para decidirlo.
  Medir con cuánta precisión se tocó respecto al momento ideal es cosa de la feature de evaluación;
  aquí la pregunta es únicamente si la nota estaba sonando o no.
- **FR-015**: El sistema MUST funcionar sin teclado conectado: la canción se ve y se reproduce
  igual, y la falta de teclado se comunica sin bloquear nada.
- **FR-016**: El sistema MUST seguir mostrando y reproduciendo la canción si el teclado se
  desconecta a mitad de la práctica, comunicándolo.

### Modo espera

- **FR-017**: El alumno MUST poder activar y desactivar el modo espera.
- **FR-018**: Con el modo espera activo, el cursor MUST avanzar a la velocidad de práctica elegida
  mientras no haya nada pendiente, y MUST detenerse al alcanzar una nota que el alumno todavía no
  haya tocado, esperando ahí indefinidamente.
- **FR-018a**: El tiempo entre notas MUST transcurrir de verdad, no saltarse. Es lo que permite al
  alumno percibir la figura rítmica —si una nota es una redonda o una semicorchea— incluso mientras
  la canción le espera.
- **FR-019**: Una nota equivocada MUST NOT hacer avanzar el cursor, y MUST comunicarse sin
  interrumpir la práctica.
- **FR-020**: El sistema MUST ofrecer una salida cuando el modo espera no pueda satisfacerse, por
  ejemplo si la canción pide una nota que el teclado del alumno no tiene.
- **FR-021**: Cambiar de modo a mitad de una canción MUST conservar la posición actual.
- **FR-022**: Con el modo espera activo y varias notas simultáneas, el cursor MUST avanzar solo
  cuando **todas** las notas del acorde estén pulsadas **al mismo tiempo**. No basta con haberlas
  acertado una tras otra: un acorde es un gesto simultáneo, y aceptarlo por partes enseñaría un
  hábito que después hay que corregir.
- **FR-022a**: El sistema MUST NOT exigir precisión de milisegundos para considerar simultáneas las
  notas de un acorde. Basta con que coincidan pulsadas en algún instante.

### Elegir teclado

- **FR-023**: El alumno MUST poder ver los teclados disponibles con un nombre que los distinga.
- **FR-024**: El alumno MUST poder elegir uno, y el sistema MUST recordarlo para la próxima sesión.
- **FR-025**: Si el teclado recordado no está, el sistema MUST pedir que se elija de nuevo, y MUST
  NOT abrir otro en su lugar.

### Digitación sugerida

- **FR-030**: El sistema MUST proponer un dedo para cada nota aplicando las reglas habituales del
  piano: evitar el pulgar en las teclas negras siempre que se pueda, minimizar los desplazamientos
  de la mano, y usar el paso del pulgar en los pasajes por grados conjuntos.
- **FR-031**: La digitación propuesta MUST ser determinista: la misma canción produce siempre la
  misma propuesta.
- **FR-032**: El sistema MUST proponer una digitación para **cualquier** canción que se pueda
  cargar, sin dejar notas sin dedo. Cuando no haya una buena solución, MUST proponer la menos mala
  en lugar de no proponer nada.
- **FR-033**: El sistema MUST calcular la digitación por manos separadas: la propuesta para la mano
  izquierda MUST NOT depender de lo que haga la derecha.

### Práctica por manos

- **FR-026**: El alumno MUST poder elegir practicar la pieza completa, solo la mano izquierda o
  solo la derecha.
- **FR-026a**: Al practicar una mano, la otra MUST seguir **visible** en pantalla, para que el
  alumno sepa dónde encaja lo que toca, pero MUST NOT exigírsele: en modo espera el cursor avanza
  atendiendo únicamente a la mano elegida.

### Límites del alcance

- **FR-027**: El sistema MUST NOT puntuar ni calificar la interpretación. Determinar si una nota
  coincide (FR-014) es lo mínimo que el modo espera necesita; medir con cuánta precisión se tocó,
  contar aciertos y fallos y dar una nota son cosa de una entrega posterior.
- **FR-028**: El sistema MUST NOT producir sonido. El alumno oye su propio instrumento.
- **FR-029**: El sistema MUST NOT guardar la interpretación del alumno ni enviar nada fuera del
  dispositivo.

### Key Entities

- **Canción cargada**: la pieza que se está practicando, con sus notas y su información de tempo.
- **Cursor de posición**: dónde está la práctica dentro de la canción. Lo mueve el paso del tiempo
  o el acierto de las notas, según el modo.
- **Modo de práctica**: si el cursor lo gobierna el tiempo o el acierto.
- **Velocidad de práctica**: proporción respecto al tempo original, sin afectar a las alturas.
  Gobierna el avance del cursor en los dos modos: a tempo fijo de principio a fin, y en modo espera
  durante los tramos en que no hay ninguna nota pendiente.
- **Teclas pulsadas**: qué está tocando el alumno en este instante.
- **Digitación propuesta**: qué dedo se sugiere para cada nota, y de qué mano.
- **Mano en práctica**: si se está practicando la pieza completa, la izquierda o la derecha.
- **Punto de corte de manos**: la altura que separa mano izquierda de derecha cuando el archivo no
  las trae separadas. Ajustable por el alumno.
- **Sesión de práctica**: la canción, el cursor, el modo, la velocidad y el teclado elegido.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Desde que el alumno abre la aplicación hasta que ve una canción suya en pantalla pasan
  menos de 30 segundos y no más de tres acciones.
- **SC-002**: Una canción de 1.000 notas se abre y aparece en pantalla en menos de 2 segundos.
- **SC-003**: Durante la reproducción completa de una pieza de 10 minutos, el 99 % de los
  fotogramas se dibuja en menos de 16,7 milisegundos (60 por segundo).
- **SC-004**: Al pulsar una tecla, el reflejo aparece en pantalla en menos de 50 milisegundos en el
  percentil 95, medidos desde que el sistema operativo entrega el mensaje.
- **SC-005**: En modo espera, el 100 % de las notas correctas hace avanzar el cursor y el 100 % de
  las incorrectas no lo hace.
- **SC-005a**: A tempo fijo, una tecla pulsada mientras su nota suena en la canción se distingue
  del 100 % de las teclas pulsadas que la canción no pedía.
- **SC-006**: Un alumno que no sabe tocar la pieza consigue llegar a su final en modo espera.
- **SC-007**: Toda la funcionalidad, salvo lo que requiere un teclado físico, se verifica de forma
  automática sin ningún dispositivo conectado.
- **SC-008**: Reducir la velocidad a la mitad duplica exactamente la duración de la reproducción.
- **SC-008a**: Saltar a cualquier punto de una canción de 10 minutos deja la práctica lista en ese
  punto sin retraso perceptible por el alumno.
- **SC-009**: El 100 % de las notas de cualquier canción cargable recibe un dedo propuesto.
- **SC-010**: La misma canción produce la misma digitación propuesta en 100 ejecuciones seguidas.
- **SC-011**: En una escala sencilla de una octava, la digitación propuesta coincide con la que
  enseña cualquier método de piano.
- **SC-012**: En modo espera con la mano izquierda elegida, tocar notas de la derecha no hace
  avanzar el cursor.

## Assumptions

- **Origen de las canciones**: las aporta el alumno desde su disco, conforme a la Constitución. No
  hay catálogo ni descarga.
- **Sin sonido**: la aplicación no sintetiza ni reproduce audio. El alumno oye su instrumento, que
  además elimina toda una fuente de latencia.
- **Sin puntuación**: esta entrega responde «¿es ésta la nota?», no «¿qué tal lo has hecho?».
- **Un solo alumno y una sola canción a la vez**: no hay sesiones simultáneas ni comparación entre
  intérpretes.
- **Sin persistencia de la práctica**: lo único que sobrevive entre sesiones es qué teclado se
  eligió. El progreso y las estadísticas son una entrega posterior.
- **Nombre de las notas**: se asume la nomenclatura latina (Do, Re, Mi…), coherente con el idioma
  de la aplicación. Ofrecer también la anglosajona (C, D, E…) es una mejora posterior.
- **De qué mano es cada nota**: se asume que se deduce de la voz de origen que el núcleo ya
  conserva desde la feature 001 cuando el archivo trae las manos separadas, que es lo habitual en
  el material de piano. Cuando venga todo en una sola voz, se asume un reparto por altura respecto
  al Do central. Es una heurística y puede equivocarse; por eso la mano practicada es elegible y no
  impuesta.
- **Calidad de la digitación propuesta**: se asume que acertará en pasajes sencillos y fallará en
  los difíciles. Es un problema sin solución perfecta —depende del tamaño de la mano, del pasaje
  siguiente y hasta del fraseo— y por eso se presenta como sugerencia. Importar digitación escrita
  por un editor humano, mediante formatos que sí la llevan, queda como mejora posterior.
- **Metrónomo**: fuera de alcance, por la misma razón que el sonido.
- **Reutilización**: la carga de canciones y la captura del teclado ya existen y están probadas;
  esta entrega las conecta y les pone cara, no las rehace.
