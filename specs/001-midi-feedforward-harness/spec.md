# Feature Specification: Harness feedforward del núcleo

**Feature Branch**: `001-midi-feedforward-harness`

**Created**: 2026-08-17

**Status**: Draft

**Input**: User description: "Harness feedforward mínimo del núcleo. Un crate Rust headless (`core/`) que: (1) carga un archivo MIDI estándar y lo convierte en una línea temporal determinista de notas programadas con altura, onset, duración y velocity en tiempo musical y en milisegundos; (2) emite hacia adelante eventos de «cue» que indican qué nota toca a continuación y cuándo, con antelación configurable; (3) todo ello gobernado por un reloj inyectable, de modo que las pruebas usen un reloj virtual determinista y la app real un reloj monótono. Sin interfaz gráfica, sin teclado MIDI físico y sin evaluación ni puntuación todavía: solo el camino hacia adelante."

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Convertir una canción en una lección con estructura temporal (Priority: P1)

El alumno tiene un archivo MIDI de la canción que quiere aprender. El sistema lo convierte en
una secuencia ordenada de notas, cada una con su altura, el instante exacto en que debe sonar,
cuánto dura y con qué intensidad. A partir de ese momento la canción deja de ser un archivo
opaco y pasa a ser material didáctico con estructura temporal.

**Why this priority**: sin línea temporal no hay nada que enseñar. Es el cimiento del que
dependen todas las demás historias y todas las funcionalidades futuras (anticipación,
evaluación, puntuación, progreso).

**Independent Test**: se prueba por completo cargando archivos MIDI de referencia y comparando
la secuencia de notas resultante con la lista esperada, sin ventana ni teclado conectado.

**Acceptance Scenarios**:

1. **Given** una canción de una sola voz con cinco notas consecutivas, **When** se carga,
   **Then** el sistema produce cinco notas ordenadas por instante de inicio, cada una con
   altura, inicio, duración e intensidad.
2. **Given** una canción que contiene un acorde de tres notas simultáneas, **When** se carga,
   **Then** el sistema produce tres notas con idéntico instante de inicio.
3. **Given** una canción con cambios de tempo a mitad de la pieza, **When** se carga, **Then**
   los instantes expresados en tiempo real reflejan el tempo vigente en cada tramo.
4. **Given** una canción con la misma altura repetida sin silencio intermedio, **When** se
   carga, **Then** el sistema produce dos notas distintas y no una sola nota alargada.
5. **Given** un archivo que no es una canción válida, **When** se intenta cargar, **Then** el
   sistema lo rechaza indicando el motivo y no entrega una lección a medias.

---

### User Story 2 - Saber qué nota viene antes de tener que tocarla (Priority: P2)

Mientras avanza la canción, el alumno recibe con antelación el aviso de qué nota debe tocar y
en qué momento, de forma que le dé tiempo a colocar la mano. La antelación es configurable
según su nivel: un principiante necesita más margen que alguien avanzado.

**Why this priority**: es la esencia del feedforward y lo que diferencia practicar de escuchar.
Depende de la Historia 1, pero aporta valor por sí sola aunque todavía no exista evaluación.

**Independent Test**: se prueba avanzando una canción cargada y comprobando que la secuencia
de avisos emitidos coincide, en contenido y en momento, con la esperada para una antelación
dada.

**Acceptance Scenarios**:

1. **Given** una canción cargada y una antelación configurada, **When** la reproducción alcanza
   el instante en que una nota entra dentro de esa antelación, **Then** el sistema anuncia esa
   nota una sola vez, indicando cuánto falta para que deba tocarse.
2. **Given** un acorde de varias notas simultáneas, **When** entra en la ventana de antelación,
   **Then** el sistema anuncia todas sus notas juntas, sin desordenarlas ni omitir ninguna.
3. **Given** una antelación mayor que la duración total de la canción, **When** comienza la
   reproducción, **Then** el sistema anuncia todas las notas al inicio y no falla.
4. **Given** una reproducción en curso, **When** se avanza el tiempo en saltos grandes que
   cruzan varias notas de golpe, **Then** el sistema anuncia todas las notas cruzadas y no se
   salta ninguna.
5. **Given** una canción sin notas, **When** se reproduce, **Then** el sistema no anuncia nada
   y termina limpiamente.
6. **Given** una canción con un cambio de tempo, **When** una nota posterior al cambio entra en
   la ventana de antelación, **Then** el aviso se emite a la misma distancia musical de la nota
   que antes del cambio, aunque el margen real en segundos sea distinto.

---

### User Story 3 - Comportamiento reproducible bajo un reloj controlado (Priority: P3)

Quien desarrolla o verifica el sistema puede ejecutar una canción entera bajo un reloj que él
controla, avanzando el tiempo a voluntad, y obtiene siempre exactamente la misma secuencia de
avisos. En la aplicación real ese mismo mecanismo funciona con el paso real del tiempo.

**Why this priority**: es lo que convierte el sistema en verificable. Sin ella, cada prueba
dependería del reloj de la máquina y sería lenta e intermitente. Habilita el TDD estricto que
exige la Constitución.

**Independent Test**: se prueba ejecutando la misma canción dos veces con la misma secuencia de
avances de tiempo y comprobando que ambas ejecuciones producen resultados idénticos.

**Acceptance Scenarios**:

1. **Given** una canción y una secuencia fija de avances de tiempo, **When** se ejecuta dos
   veces, **Then** ambas ejecuciones producen la misma secuencia de avisos, en el mismo orden y
   con los mismos valores.
2. **Given** una canción de varios minutos, **When** se ejecuta bajo el reloj controlado,
   **Then** la ejecución completa termina sin esperar tiempo real.

---

### Edge Cases

- **Nota sin cierre**: una nota que empieza y nunca termina antes del final de la pieza. El
  sistema debe cerrarla al final de la canción en lugar de descartarla o colgarse.
- **Cierre sin apertura**: un final de nota que no corresponde a ninguna nota abierta. Debe
  ignorarse sin invalidar el resto de la canción.
- **Intensidad cero como final**: en el formato MIDI, un inicio de nota con intensidad cero
  equivale a un final de nota. Debe tratarse como tal.
- **Notas superpuestas de la misma altura**: la misma tecla vuelve a sonar antes de que la
  anterior termine. El sistema debe emparejar aperturas y cierres de forma predecible y
  documentada.
- **Canción sin ningún cambio de tempo declarado**: debe asumirse el tempo por defecto del
  estándar MIDI (120 negras por minuto) en lugar de fallar.
- **Cambio de tempo antes de la primera nota, o varios cambios seguidos en el mismo instante**:
  debe prevalecer el último declarado en ese instante.
- **Archivo vacío, truncado o con cabecera corrupta**: rechazo con motivo, sin pánico ni
  lectura fuera de límites.
- **Antelación de cero**: el aviso coincide con el instante de la nota; sigue siendo válido.
- **Retroceso del tiempo**: si se solicita avanzar a un instante anterior al ya alcanzado, el
  sistema debe rechazarlo o reposicionarse de forma explícita, nunca emitir avisos duplicados
  de forma silenciosa.
- **Canción muy densa**: miles de notas no deben degradar la emisión de avisos ni obligar a
  recorrer la canción entera en cada avance de tiempo.

## Requirements *(mandatory)*

### Functional Requirements

**Carga y línea temporal**

- **FR-001**: El sistema MUST aceptar canciones en formato MIDI estándar y convertirlas en una
  secuencia de notas ordenada por instante de inicio.
- **FR-002**: Cada nota MUST incluir altura, instante de inicio, duración e intensidad.
- **FR-003**: El sistema MUST expresar el instante de inicio y la duración de cada nota tanto
  en tiempo musical (independiente del tempo) como en tiempo real transcurrido, de modo que un
  cambio de tempo altere el segundo sin alterar el primero.
- **FR-004**: El sistema MUST respetar los cambios de tempo declarados en la canción al calcular
  el tiempo real, incluidos los que ocurren a mitad de la pieza.
- **FR-005**: El sistema MUST asumir 120 negras por minuto cuando la canción no declare tempo.
- **FR-006**: El sistema MUST combinar todas las voces o pistas de la canción en una única
  secuencia ordenada, conservando la identidad de la voz de origen de cada nota.
- **FR-007**: El sistema MUST rechazar una canción ilegible o corrupta indicando el motivo, sin
  interrumpir abruptamente el programa y sin entregar una línea temporal parcial.
- **FR-008**: El sistema MUST producir la misma línea temporal cada vez que se carga la misma
  canción, incluyendo el orden de las notas que comparten instante de inicio.
- **FR-009**: El sistema MUST considerar material a tocar todas las notas de la línea temporal,
  sin filtrar ni descartar voces. La selección por mano o por pista queda fuera de alcance en
  esta entrega y MUST poder construirse después sobre la voz de origen registrada en FR-006,
  sin rehacer la carga.

**Anticipación (feedforward)**

- **FR-010**: El sistema MUST anunciar cada nota antes de su instante de inicio, con una
  antelación configurable.
- **FR-011**: El sistema MUST expresar la antelación en tiempo musical (pulsos), no en tiempo
  real. En consecuencia, el margen real del aviso MUST estirarse al practicar a tempo lento y
  encogerse a tempo rápido, y un cambio de tempo MUST alterar el instante real del aviso sin
  alterar su distancia musical a la nota anunciada.
- **FR-012**: El sistema MUST anunciar cada nota exactamente una vez por reproducción.
- **FR-013**: El sistema MUST anunciar juntas las notas que comparten instante de inicio,
  preservando su orden dentro del grupo.
- **FR-014**: Cada aviso MUST indicar qué nota es y cuánto falta para que deba tocarse.
- **FR-015**: El sistema MUST anunciar todas las notas cuyo momento de aviso quede cubierto por
  un avance de tiempo, aunque ese avance cruce varias notas de una vez.
- **FR-016**: El sistema MUST indicar cuándo la canción ha terminado, es decir, cuándo no queda
  ninguna nota por anunciar.

**Reloj y determinismo**

- **FR-017**: El sistema MUST obtener el paso del tiempo de una fuente sustituible, de modo que
  pueda gobernarse tanto por el paso real del tiempo como por una secuencia controlada.
- **FR-018**: El sistema MUST producir resultados idénticos ante secuencias de tiempo idénticas.
- **FR-019**: El sistema MUST permitir ejecutar una canción completa sin esperar su duración
  real.
- **FR-020**: El sistema MUST rechazar de forma explícita cualquier intento de retroceder el
  tiempo dentro de una misma reproducción.

**Límites del alcance**

- **FR-021**: El sistema MUST funcionar sin interfaz gráfica y sin ningún teclado conectado.
- **FR-022**: El sistema MUST NOT evaluar, comparar ni puntuar lo que el alumno toca en esta
  entrega; solo describe qué debe tocarse y cuándo.
- **FR-023**: El sistema MUST NOT acceder a la red ni enviar datos fuera del dispositivo.

### Key Entities

- **Canción**: la pieza cargada. Agrupa la información de tempo y el conjunto de notas.
- **Nota programada**: una nota concreta que debe tocarse. Atributos: altura, instante de
  inicio, duración, intensidad y voz de origen.
- **Mapa de tempo**: la relación entre tiempo musical y tiempo real a lo largo de la pieza.
- **Aviso (cue)**: el anuncio anticipado de una nota. Atributos: la nota anunciada y el margen
  restante hasta su instante de inicio.
- **Reproducción**: el recorrido de una canción a lo largo del tiempo. Conoce hasta dónde se ha
  avanzado y qué queda por anunciar.
- **Fuente de tiempo**: el origen del avance temporal, sustituible entre el paso real del tiempo
  y una secuencia controlada.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Una canción de 5 minutos con 1.000 notas se convierte en lección en menos de 100
  milisegundos.
- **SC-002**: La verificación automática completa de esta funcionalidad termina en menos de 1
  segundo.
- **SC-003**: Ejecutar la misma canción 100 veces con la misma secuencia de tiempo produce 100
  resultados idénticos: cero variabilidad.
- **SC-004**: El 100 % de las notas de una canción se anuncian, exactamente una vez cada una y
  siempre antes de su instante de inicio.
- **SC-005**: Ninguna canción malformada, de entre el conjunto de casos límite recogidos,
  provoca una interrupción abrupta del programa: todas se rechazan con un motivo o se corrigen
  según la regla documentada.
- **SC-006**: Consultar qué debe anunciarse tras un avance de tiempo no requiere recorrer la
  canción entera: el coste crece con el número de notas anunciadas, no con el tamaño de la
  canción.
- **SC-007**: La funcionalidad completa se ejerce sin ventana y sin teclado conectado.

## Assumptions

- **Formatos de canción**: se asumen los formatos MIDI de pista única y multipista sincronizada
  (tipos 0 y 1), que cubren prácticamente todo el material disponible. Las pistas
  independientes no sincronizadas (tipo 2) quedan fuera de alcance.
- **Resolución temporal**: se asume que las canciones expresan el tiempo en pulsos por negra,
  la convención habitual. La codificación alternativa por fotogramas (SMPTE) queda fuera de
  alcance en esta entrega.
- **Origen de las canciones**: las importa el propio usuario desde su disco, conforme a la
  Constitución; no hay catálogo remoto ni descarga.
- **Notas superpuestas de la misma altura**: se asume emparejamiento en orden de llegada (la
  apertura más antigua se cierra con el primer cierre que aparece).
- **Cambios de compás y armadura**: se leen si están presentes pero no afectan al
  comportamiento de esta entrega; se conservan para funcionalidades posteriores.
- **Pedales y controladores**: fuera de alcance en esta entrega.
- **Antelación por defecto**: a falta de configuración se asume una negra de antelación, un
  margen razonable para un principiante a tempo de estudio.
- **Alcance deliberadamente parcial**: esta entrega cubre solo el camino de ida (qué tocar y
  cuándo). La captura de lo que toca el alumno, la comparación y la puntuación son
  funcionalidades posteriores, pero la estructura de datos definida aquí debe poder alimentarlas
  sin rediseñarse.
