# Feature Specification: Captura MIDI del teclado

**Feature Branch**: `002-midi-input-capture`

**Created**: 2026-08-17

**Status**: Draft

**Input**: User description: "Captura MIDI del teclado. Entrada del dispositivo físico a eventos con marca de tiempo, entregados al núcleo. Es la feature que cierra la deuda constitucional: al existir por fin la ruta crítica, trae el benchmark de latencia de 30 ms que el Principio IV exige y que la feature 001 no pudo entregar. Sin evaluación ni puntuación todavía: solo capturar lo que el alumno toca."

## Clarifications

### Session 2026-08-18

- Q: La investigación encontró que sí existe un identificador de dispositivo estable y accesible en
  ambas plataformas. ¿Se refina FR-004a? → A: Sí. El identificador del sistema pasa a ser la clave
  primaria y la pareja (nombre, posición entre homónimos) queda como reserva. La regla anterior no
  se deroga: sigue siendo el respaldo cuando el identificador no case. El motivo del cambio es que
  la opción se había descartado por "no portable", y esa descripción era incompleta.

### Session 2026-08-17

- Q: ¿Cómo se alinean los instantes de la captura con los de la reproducción? → A: Un único reloj
  de sesión, creado una sola vez y compartido por ambas. El cero es el mismo por construcción, de
  modo que el desfase de origen no puede existir en lugar de tener que corregirse.

- Q: ¿Qué se hace con las teclas aún hundidas al detener la captura? → A: Se cierran en el
  instante de la parada y se etiqueta que el final no lo puso el alumno, en simetría con la
  feature 001. No se descartan (perdería una nota real) ni se les da una duración inventada.

- Q: ¿Qué pasa si llegan pulsaciones más rápido de lo que el consumidor las recoge? → A: Un
  almacén intermedio acotado y holgado. Al llenarse se descarta y se cuenta en el informe de
  captura, nunca se bloquea al hilo del sistema operativo ni se crece sin techo. La pérdida debe
  ser visible, no silenciosa.

- Q: ¿Hasta qué punto se mide el retraso de una pulsación para la puerta de 30 ms? → A: De punta
  a punta dentro de esta capa: desde que el sistema operativo entrega el mensaje hasta que el
  consumidor lo recibe efectivamente, incluido su despertar. Medir solo hasta la cola interna
  daría verde aunque el consumidor se ralentizase.

- Q: ¿Qué dato identifica a un teclado entre sesiones, para recordar el elegido? → A: El nombre
  del puerto más su posición entre los homónimos. Es lo único portable entre macOS y Windows; si
  al arrancar no se encuentra esa combinación, se pide elegir de nuevo en vez de abrir otro
  dispositivo a ciegas.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Que el sistema reciba lo que el alumno toca (Priority: P1)

El alumno conecta su teclado al ordenador, lo elige en la aplicación y empieza a tocar. Cada
tecla que pulsa y suelta queda registrada por el sistema con la altura, la intensidad y el
instante exacto en que ocurrió.

**Why this priority**: es el requisito del que depende todo lo demás. Sin saber qué toca el
alumno no hay corrección posible, y la aplicación se queda en un reproductor de canciones.

**Independent Test**: se prueba entera alimentando el sistema con una secuencia de pulsaciones
de origen controlado y comprobando que produce exactamente los eventos esperados, sin necesidad
de un teclado físico.

**Acceptance Scenarios**:

1. **Given** un teclado disponible, **When** el usuario consulta los dispositivos, **Then** el
   sistema los lista con un nombre legible que permite distinguirlos.
2. **Given** un teclado elegido, **When** el alumno pulsa una tecla, **Then** el sistema registra
   un ataque con su altura, su intensidad y su instante.
3. **Given** una tecla pulsada, **When** el alumno la suelta, **Then** el sistema registra el
   final de esa nota, emparejado con su ataque.
4. **Given** un acorde de tres teclas pulsadas a la vez, **When** se capturan, **Then** los tres
   ataques quedan registrados, ninguno se pierde y su orden relativo es estable.
5. **Given** una captura en curso, **When** el usuario la detiene, **Then** el sistema deja de
   registrar y libera el dispositivo para otras aplicaciones.

---

### User Story 2 - Que llegue lo bastante rápido para tocar a tempo (Priority: P1)

Cuando el alumno pulsa una tecla, el sistema dispone de esa pulsación con margen suficiente para
reaccionar antes de que el retraso se note. El proyecto puede demostrar con una medición
automática, y no con una impresión subjetiva, que ese margen se respeta.

**Why this priority**: es la razón de ser de esta entrega. El Principio IV de la Constitución fija
un presupuesto de 30 milisegundos y exige medirlo en integración continua; la feature 001 no pudo
entregar esa medición porque la ruta crítica todavía no existía. Aquí existe, y la deuda se salda.

**Independent Test**: se prueba ejecutando la medición automática sobre una ráfaga de pulsaciones
y comprobando que el percentil 95 queda bajo el presupuesto.

**Acceptance Scenarios**:

1. **Given** una ráfaga de pulsaciones, **When** se mide el recorrido de cada una, **Then** el
   percentil 95 del retraso queda por debajo de 30 milisegundos.
2. **Given** la medición automática, **When** una modificación empeora el retraso por encima del
   presupuesto, **Then** la comprobación falla y bloquea la incorporación del cambio.
3. **Given** una sesión de práctica sostenida, **When** se capturan pulsaciones durante varios
   minutos, **Then** el retraso no se degrada con el tiempo.

---

### User Story 3 - Que el sistema siga siendo verificable sin teclado (Priority: P2)

Quien desarrolla o verifica el sistema puede ejercer toda la funcionalidad de captura sin ningún
teclado enchufado, sustituyendo el dispositivo real por una fuente de pulsaciones controlada.

**Why this priority**: el Principio III de la Constitución exige que el núcleo se pruebe sin
hardware. Sin esta historia, la mitad de la aplicación dejaría de ser verificable en integración
continua y solo se podría probar a mano.

**Independent Test**: se prueba ejecutando la suite completa en una máquina sin ningún dispositivo
MIDI conectado y comprobando que ninguna prueba se salta ni falla por ello.

**Acceptance Scenarios**:

1. **Given** una máquina sin teclado conectado, **When** se ejecuta la verificación completa,
   **Then** todas las pruebas de captura se ejecutan y pasan.
2. **Given** una fuente de pulsaciones controlada, **When** se le da una secuencia con instantes
   fijos, **Then** el sistema produce siempre los mismos eventos.

---

### User Story 4 - Que enchufar y desenchufar no rompa la sesión (Priority: P3)

Si el alumno desconecta el teclado a mitad de una sesión, o lo conecta cuando la aplicación ya
está abierta, el sistema se entera y se lo dice, en lugar de quedarse mudo o cerrarse.

**Why this priority**: ocurre constantemente en el uso real —el cable se suelta, el teclado se
apaga solo— y un fallo silencioso aquí se percibe como que la aplicación está rota.

**Independent Test**: se prueba simulando la desaparición y la reaparición del dispositivo y
comprobando que el sistema lo comunica y conserva lo capturado hasta ese momento.

**Acceptance Scenarios**:

1. **Given** una captura en curso, **When** el teclado se desconecta, **Then** el sistema lo
   comunica, conserva todo lo capturado hasta ese instante y no termina abruptamente.
2. **Given** ningún teclado conectado, **When** el usuario intenta empezar, **Then** el sistema lo
   dice con claridad en lugar de fallar de forma opaca.
3. **Given** un teclado que se reconecta, **When** el usuario lo vuelve a elegir, **Then** la
   captura se reanuda sin reiniciar la aplicación.

---

### Edge Cases

- **Ningún dispositivo disponible**: la lista sale vacía y el sistema lo comunica; no es un error
  que interrumpa la aplicación.
- **Varios teclados a la vez**: solo se captura el elegido; el resto se listan pero se ignoran.
- **Nombres de dispositivo repetidos o vacíos**: dos teclados del mismo modelo se anuncian igual;
  se distinguen por su posición entre los homónimos. Un nombre vacío se sustituye por una etiqueta
  legible generada por el sistema para que el usuario pueda elegirlo igualmente.
- **El teclado recordado ya no está**: el conjunto de aparatos cambió desde la última sesión. Se
  pide elegir de nuevo, nunca se abre otro en su lugar.
- **El teclado cambió de conector USB entre sesiones**: el identificador del sistema lo reconoce
  igualmente; es precisamente el caso donde la pareja (nombre, posición) fallaría.
- **El dispositivo ya está en uso por otra aplicación**: se comunica sin bloquear la aplicación.
- **Tecla soltada sin haber sido pulsada**: puede ocurrir al empezar a capturar con una tecla ya
  hundida. Se ignora y se cuenta, igual que en la carga de canciones.
- **Tecla que se queda hundida al detener la captura**: la nota se cierra en el instante de la
  parada y queda etiquetada como cerrada por la parada, no por el alumno.
- **Pulsación con intensidad cero**: en el estándar MIDI equivale a soltar la tecla; se trata como
  tal, igual que en la carga de canciones.
- **Ráfaga muy densa**: un glissando genera decenas de pulsaciones en milisegundos. Ninguna puede
  perderse ni desordenarse.
- **Consumidor atascado**: si quien recoge las pulsaciones se detiene, el almacén se llena. Se
  descarta lo entrante y se cuenta; nunca se bloquea al productor ni se crece sin techo.
- **Mensajes que no son notas**: pedales, ruedas de modulación, cambios de instrumento. Se
  descartan sin que ello interrumpa la captura de las notas que llegan entremezcladas.
- **Instantes iguales**: dos teclas de un acorde pueden llegar con el mismo instante. El orden
  entre ellas debe ser estable y definido.
- **El reloj del sistema cambia** (cambio de hora, ajuste horario): no debe alterar los instantes
  ya capturados ni hacer que el tiempo retroceda.
- **La captura empieza antes que la reproducción, o al revés**: al compartir un mismo reloj de
  sesión, el orden en que arrancan no introduce ningún desfase entre lo tocado y lo esperado.

## Requirements *(mandatory)*

### Descubrimiento y conexión

- **FR-001**: El sistema MUST enumerar los teclados MIDI disponibles con un nombre legible por una
  persona.
- **FR-002**: El sistema MUST permitir elegir uno de los teclados disponibles e iniciar la captura
  sobre él.
- **FR-003**: El sistema MUST distinguir entre sí dos dispositivos que anuncien el mismo nombre,
  usando su posición relativa entre los homónimos presentes.
- **FR-004**: El sistema MUST capturar, cuando hay varios teclados disponibles, únicamente el que
  el usuario elija de forma explícita. MUST NOT elegir uno por su cuenta ni fusionar varios
  dispositivos en un mismo flujo: un controlador de pads o un sintetizador que emite reloj
  contaminarían lo que el alumno realmente toca.
- **FR-004a**: El sistema MUST recordar el último teclado elegido y MUST volver a proponerlo la
  próxima vez, para que elegir sea un trámite de una sola vez y no de cada sesión. Lo recordado
  MUST incluir el identificador que el sistema operativo asigna al dispositivo **y** la pareja
  (nombre del puerto, posición entre los homónimos).
- **FR-004b**: Al reconocer el teclado recordado, el sistema MUST intentar primero el identificador
  del sistema operativo, que sobrevive a renumeraciones y a cambiar el teclado de conector, y MUST
  recurrir a la pareja (nombre, posición) solo si aquél no casa.
- **FR-004c**: Si ninguno de los dos criterios encuentra el dispositivo, el sistema MUST pedir al
  usuario que elija de nuevo. MUST NOT abrir un dispositivo distinto en su lugar: capturar del
  aparato equivocado sin avisar es peor que no capturar.
- **FR-005**: El sistema MUST comunicar de forma explícita, y sin interrumpir la aplicación, que no
  hay ningún teclado disponible o que el elegido no se pudo abrir.
- **FR-006**: El sistema MUST liberar el dispositivo al detener la captura, de modo que otra
  aplicación pueda usarlo.

### Captura

- **FR-007**: El sistema MUST registrar cada pulsación con su altura, su intensidad y el instante
  en que ocurrió.
- **FR-008**: El sistema MUST registrar cada suelta de tecla y emparejarla con su pulsación,
  siguiendo la misma política de emparejamiento que ya usa la carga de canciones.
- **FR-009**: El sistema MUST tratar una pulsación de intensidad cero como una suelta de tecla.
- **FR-010**: El sistema MUST conservar el orden en que ocurrieron las pulsaciones, y MUST aplicar
  un desempate estable y definido cuando dos comparten instante.
- **FR-011**: El sistema MUST NOT perder ninguna pulsación en ráfagas de hasta 50 eventos por
  segundo sostenidas durante un minuto.
- **FR-011a**: El sistema MUST almacenar las pulsaciones pendientes de recoger en un espacio
  **acotado**, dimensionado con al menos un orden de magnitud de margen sobre la ráfaga humana más
  densa. MUST NOT crecer sin límite: agotar la memoria degradaría la aplicación entera en lugar de
  degradar solo la captura.
- **FR-011b**: Si ese espacio se llena, el sistema MUST descartar la pulsación entrante y MUST
  contarla en el informe de captura. MUST NOT bloquear al productor esperando sitio, ni descartar
  eventos ya almacenados: descartar un ataque y conservar su suelta dejaría notas huérfanas.
- **FR-011c**: Un descarte por desbordamiento MUST ser observable por quien consume la captura, de
  modo que la pérdida se detecte en una prueba en lugar de manifestarse como notas fantasma.
- **FR-012**: Los instantes de captura y los de la reproducción MUST ser directamente comparables,
  sin ninguna conversión ni corrección de por medio. *Éste es el resultado exigido; FR-012a fija el
  mecanismo que lo garantiza.*
- **FR-012a**: La captura y la reproducción MUST recibir **el mismo** reloj de sesión, creado una
  sola vez. MUST NOT arrancar cada una el suyo: dos relojes que empiezan en cero en instantes
  distintos producirían un desfase constante, y la evaluación futura concluiría que el alumno
  siempre llega tarde por un error de origen y no por cómo toca.
- **FR-012b**: El reloj de sesión MUST ser sustituible por uno controlado, igual que en la
  reproducción, para que la captura se pueda ejercer con instantes fijos y reproducibles.
- **FR-013**: Los instantes capturados MUST ser no decrecientes, y MUST NOT verse afectados por
  cambios en la hora del sistema. **No se exige que sean estrictamente crecientes**: varias notas
  de un mismo acorde comparten instante de forma legítima, y el desempate es su orden de llegada.
- **FR-014**: El sistema MUST capturar únicamente pulsaciones y sueltas de tecla. El pedal de
  resonancia, la rueda de modulación, la presión posterior y los cambios de instrumento MUST
  descartarse, igual que hace la carga de canciones. La simetría es deliberada: si un lado
  aplicase el pedal y el otro no, lo tocado y lo esperado dejarían de ser comparables.
- **FR-015**: El sistema MUST cerrar las notas que sigan hundidas al detener la captura en el
  instante mismo de la parada, y MUST etiquetarlas indicando que su final no lo produjo el alumno.
  MUST NOT descartarlas, porque el alumno sí pulsó esa tecla, ni asignarles una duración inventada,
  porque un dato fabricado es indistinguible de uno real y contaminaría cualquier evaluación
  posterior. Es la misma política que aplica la carga de canciones a las notas colgadas.
- **FR-016**: El sistema MUST contar los eventos anómalos tolerados (sueltas sin pulsación previa,
  notas cerradas al detener) igual que hace la carga de canciones, sin escribir en disco.

### Latencia

- **FR-017**: El sistema MUST entregar cada pulsación a su consumidor en menos de 30 milisegundos
  en el percentil 95, medidos desde que el sistema operativo entrega el mensaje hasta que el
  consumidor lo tiene efectivamente en la mano. El tramo de espera y despertar del consumidor
  MUST contar dentro de esa medida: es donde se pierde el tiempo que importa.
- **FR-018**: El proyecto MUST incluir una medición automatizada de ese retraso, ejecutable sin
  intervención manual y sin teclado conectado.
- **FR-019**: La medición MUST hacer fallar la comprobación cuando el retraso supere el
  presupuesto, de modo que bloquee la incorporación del cambio.
- **FR-020**: El camino que recorre una pulsación MUST NOT realizar acceso a disco, peticiones de
  red, ni esperas que puedan bloquearlo indefinidamente.

### Verificabilidad y desconexión

- **FR-021**: El sistema MUST permitir sustituir el teclado real por una fuente de pulsaciones
  controlada, de modo que toda la funcionalidad se pueda ejercer sin hardware.
- **FR-022**: Alimentado con la misma secuencia controlada, el sistema MUST producir siempre los
  mismos eventos.
- **FR-023**: El sistema MUST detectar la desaparición del dispositivo durante una captura,
  comunicarla y conservar lo capturado hasta ese instante.
- **FR-024**: El sistema MUST permitir reanudar la captura tras reconectar el teclado, sin
  reiniciar la aplicación.

### Límites del alcance

- **FR-025**: El sistema MUST NOT comparar, evaluar ni puntuar lo que toca el alumno en esta
  entrega. Solo registra lo que ocurrió.
- **FR-026**: El sistema MUST NOT enviar nada fuera del dispositivo ni escribir lo capturado en
  disco.
- **FR-027**: El sistema MUST NOT producir sonido. Esta entrega captura, no sintetiza.

### Key Entities

- **Dispositivo de entrada**: un teclado disponible. Atributos: identificador asignado por el
  sistema operativo (identidad primaria), nombre legible del puerto y posición entre los
  dispositivos que anuncian ese mismo nombre (identidad de reserva). No se usa el índice de puerto,
  que se renumera al conectar o desconectar cualquier otro aparato.
- **Sesión de captura**: el periodo entre iniciar y detener la captura sobre un dispositivo.
  Conoce qué se ha capturado y en qué estado está el dispositivo.
- **Pulsación capturada**: lo que el alumno tocó. Atributos: altura, intensidad, instante de
  ataque, instante de suelta y cómo se cerró (por la suelta real de la tecla, o por la parada de
  la captura con la tecla aún hundida).
- **Fuente de pulsaciones**: el origen de los eventos, sustituible entre el teclado real y una
  secuencia controlada para pruebas.
- **Reloj de sesión**: el origen de tiempo único, creado al arrancar la sesión y compartido por la
  captura y la reproducción. Sustituible por uno controlado en las pruebas.
- **Informe de captura**: contadores de eventos anómalos tolerados durante la sesión: pulsaciones
  descartadas por desbordamiento, sueltas sin pulsación previa, notas cerradas al detener la
  captura, notas cerradas al perderse el dispositivo, teclas repulsadas sin haberse soltado, notas
  fuera de las 88 teclas de un piano y mensajes descartados por no ser notas.
- **Medición de retraso**: el resultado de la comprobación automática de latencia, con su
  percentil 95 y su veredicto frente al presupuesto.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: El 95 % de las pulsaciones llega a su consumidor en menos de 30 milisegundos desde
  que el sistema operativo las entrega, incluyendo el tiempo que el consumidor tarda en
  despertar y recogerlas.
- **SC-002**: Ninguna pulsación se pierde en una ráfaga de 50 eventos por segundo sostenida durante
  un minuto: se capturan las 3.000, y el contador de descartes queda en cero.
- **SC-002a**: Con el consumidor detenido a propósito hasta desbordar el almacén, la aplicación
  sigue respondiendo, la memoria no crece sin límite y el contador de descartes refleja
  exactamente cuántas pulsaciones se perdieron.
- **SC-003**: El orden de las pulsaciones capturadas coincide con el orden en que ocurrieron el
  100 % de las veces.
- **SC-004**: Alimentar el sistema 100 veces con la misma secuencia controlada produce 100
  resultados idénticos.
- **SC-005**: La verificación completa de esta funcionalidad se ejecuta y pasa en una máquina sin
  ningún teclado conectado.
- **SC-006**: Desde que el usuario elige un teclado hasta que la captura está activa transcurre
  menos de 1 segundo.
- **SC-007**: Desconectar el teclado durante una captura se comunica en menos de 2 segundos y
  conserva el 100 % de lo capturado hasta ese instante.
- **SC-008**: Una sesión de captura de 10 minutos no degrada el retraso: el percentil 95 del último
  minuto no supera al del primero en más de un 10 %.

## Assumptions

- **Qué se mide como retraso**: el presupuesto de 30 milisegundos se mide desde que el sistema
  operativo entrega el mensaje hasta que el consumidor lo recibe de verdad, despertar incluido.
  Medir solo hasta que el evento entra en la cola interna daría un número bonito y engañoso: un
  cambio que ralentizase al consumidor no haría fallar la puerta. El tramo anterior —el barrido de teclas del propio instrumento y el transporte por cable, típicamente
  entre 1 y 3 milisegundos— no es observable desde la aplicación y queda fuera de la medición.
  Esta entrega reserva la mayor parte del presupuesto para la parte visual, que llegará después.
- **Presupuesto restante**: al no existir todavía interfaz, esta entrega consume solo su tramo. La
  feature que dibuje en pantalla heredará el presupuesto restante y deberá medir el recorrido
  completo hasta el píxel.
- **Sin sonido**: el alumno oye su propio instrumento. La aplicación no sintetiza ni reenvía
  audio, lo que elimina toda una fuente de latencia y de complejidad.
- **Un solo alumno**: no hay sesiones simultáneas ni varios intérpretes a la vez.
- **Alcance del reloj compartido**: el cero del reloj de sesión es el arranque de la sesión, no el
  de cada canción. Medir el desvío respecto al inicio de una pieza es una resta que hará la
  feature de evaluación; hacerlo aquí obligaría a reajustar la captura en cada canción y dejaría
  sin definir el caso de tocar libremente sin ninguna cargada.
- **Conexión por cable o USB**: se asume conexión local. Los teclados por Bluetooth añaden un
  retraso propio que puede consumir el presupuesto entero; quedan fuera de alcance y se
  documentarán como no soportados si aparecen.
- **Sin persistencia**: lo capturado vive en memoria durante la sesión, conforme al alcance de esta
  entrega. Guardar interpretaciones para revisarlas después es una feature posterior.
- **Consecuencia de ignorar el pedal**: un alumno que pedalea suelta las teclas antes de que el
  sonido cese. Como la canción de referencia también mide sus duraciones por la suelta de tecla y
  no por el pedal, ambos lados siguen siendo comparables. Aplicar el pedal exigiría reabrir la
  feature 001, y por eso queda fuera de esta entrega.
- **Rango del teclado**: se aceptan todas las alturas MIDI. Las que caen fuera de las 88 teclas de
  un piano se capturan y se cuentan, igual que en la carga de canciones.
- **Coherencia con la carga de canciones**: el emparejamiento de pulsación con suelta, el
  tratamiento de la intensidad cero y el criterio de orden estable siguen las mismas reglas ya
  fijadas para las canciones. Dos políticas distintas para el mismo problema harían imposible
  comparar lo tocado con lo esperado en la feature siguiente.
