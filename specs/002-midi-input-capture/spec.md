# Feature Specification: Captura MIDI del teclado

**Feature Branch**: `002-midi-input-capture`

**Created**: 2026-08-17

**Status**: Draft

**Input**: User description: "Captura MIDI del teclado. Entrada del dispositivo físico a eventos con marca de tiempo, entregados al núcleo. Es la feature que cierra la deuda constitucional: al existir por fin la ruta crítica, trae el benchmark de latencia de 30 ms que el Principio IV exige y que la feature 001 no pudo entregar. Sin evaluación ni puntuación todavía: solo capturar lo que el alumno toca."

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
  el usuario debe poder distinguirlos igualmente.
- **El dispositivo ya está en uso por otra aplicación**: se comunica sin bloquear la aplicación.
- **Tecla soltada sin haber sido pulsada**: puede ocurrir al empezar a capturar con una tecla ya
  hundida. Se ignora y se cuenta, igual que en la carga de canciones.
- **Tecla que se queda hundida al detener la captura**: la nota abierta debe cerrarse de forma
  definida en lugar de quedar colgando indefinidamente.
- **Pulsación con intensidad cero**: en el estándar MIDI equivale a soltar la tecla; se trata como
  tal, igual que en la carga de canciones.
- **Ráfaga muy densa**: un glissando genera decenas de pulsaciones en milisegundos. Ninguna puede
  perderse ni desordenarse.
- **Mensajes que no son notas**: pedales, ruedas de modulación, cambios de instrumento. Se
  descartan sin que ello interrumpa la captura de las notas que llegan entremezcladas.
- **Instantes iguales**: dos teclas de un acorde pueden llegar con el mismo instante. El orden
  entre ellas debe ser estable y definido.
- **El reloj del sistema cambia** (cambio de hora, ajuste horario): no debe alterar los instantes
  ya capturados ni hacer que el tiempo retroceda.

## Requirements *(mandatory)*

### Descubrimiento y conexión

- **FR-001**: El sistema MUST enumerar los teclados MIDI disponibles con un nombre legible por una
  persona.
- **FR-002**: El sistema MUST permitir elegir uno de los teclados disponibles e iniciar la captura
  sobre él.
- **FR-003**: El sistema MUST distinguir entre sí dos dispositivos que anuncien el mismo nombre.
- **FR-004**: El sistema MUST capturar, cuando hay varios teclados disponibles, únicamente el que
  el usuario elija de forma explícita. MUST NOT elegir uno por su cuenta ni fusionar varios
  dispositivos en un mismo flujo: un controlador de pads o un sintetizador que emite reloj
  contaminarían lo que el alumno realmente toca.
- **FR-004a**: El sistema MUST recordar el último teclado elegido y MUST volver a proponerlo la
  próxima vez, para que elegir sea un trámite de una sola vez y no de cada sesión.
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
- **FR-012**: El sistema MUST expresar los instantes de captura en el mismo sistema de tiempo que
  usa la reproducción de canciones, de modo que una feature posterior pueda compararlos sin
  conversiones.
- **FR-013**: Los instantes capturados MUST ser no decrecientes, y MUST NOT verse afectados por
  cambios en la hora del sistema.
- **FR-014**: El sistema MUST capturar únicamente pulsaciones y sueltas de tecla. El pedal de
  resonancia, la rueda de modulación, la presión posterior y los cambios de instrumento MUST
  descartarse, igual que hace la carga de canciones. La simetría es deliberada: si un lado
  aplicase el pedal y el otro no, lo tocado y lo esperado dejarían de ser comparables.
- **FR-015**: El sistema MUST cerrar de forma definida las notas que sigan hundidas al detener la
  captura, en lugar de dejarlas abiertas.
- **FR-016**: El sistema MUST contar los eventos anómalos tolerados (sueltas sin pulsación previa,
  notas cerradas al detener) igual que hace la carga de canciones, sin escribir en disco.

### Latencia

- **FR-017**: El sistema MUST poner cada pulsación a disposición de quien la consuma en menos de
  30 milisegundos en el percentil 95, medidos desde que el sistema operativo la entrega.
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

- **Dispositivo de entrada**: un teclado disponible. Atributos: identificador estable y nombre
  legible.
- **Sesión de captura**: el periodo entre iniciar y detener la captura sobre un dispositivo.
  Conoce qué se ha capturado y en qué estado está el dispositivo.
- **Pulsación capturada**: lo que el alumno tocó. Atributos: altura, intensidad, instante de
  ataque, instante de suelta y cómo se cerró.
- **Fuente de pulsaciones**: el origen de los eventos, sustituible entre el teclado real y una
  secuencia controlada para pruebas.
- **Informe de captura**: contadores de eventos anómalos tolerados durante la sesión.
- **Medición de retraso**: el resultado de la comprobación automática de latencia, con su
  percentil 95 y su veredicto frente al presupuesto.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: El 95 % de las pulsaciones queda disponible en menos de 30 milisegundos desde que el
  sistema operativo las entrega.
- **SC-002**: Ninguna pulsación se pierde en una ráfaga de 50 eventos por segundo sostenida durante
  un minuto: se capturan las 3.000.
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
  operativo entrega el mensaje hasta que el evento queda disponible para su consumidor. El tramo
  anterior —el barrido de teclas del propio instrumento y el transporte por cable, típicamente
  entre 1 y 3 milisegundos— no es observable desde la aplicación y queda fuera de la medición.
  Esta entrega reserva la mayor parte del presupuesto para la parte visual, que llegará después.
- **Presupuesto restante**: al no existir todavía interfaz, esta entrega consume solo su tramo. La
  feature que dibuje en pantalla heredará el presupuesto restante y deberá medir el recorrido
  completo hasta el píxel.
- **Sin sonido**: el alumno oye su propio instrumento. La aplicación no sintetiza ni reenvía
  audio, lo que elimina toda una fuente de latencia y de complejidad.
- **Un solo alumno**: no hay sesiones simultáneas ni varios intérpretes a la vez.
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
