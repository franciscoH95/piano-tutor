<!--
Sync Impact Report
- Cambio de versión: plantilla sin rellenar → 1.0.0 (ratificación inicial)
- Principios definidos (5):
  - I. Precisión Musical Primero (NO NEGOCIABLE)
  - II. Desarrollo Guiado por Pruebas (NO NEGOCIABLE)
  - III. Núcleo Determinista Desacoplado de la UI
  - IV. Tiempo Real con Presupuesto (<30 ms)
  - V. Local Primero y Propiedad del Usuario
- Secciones añadidas: Restricciones Técnicas y de Contenido; Flujo de Desarrollo y
  Puertas de Calidad; Governance
- Secciones eliminadas: ninguna (documento previo era la plantilla sin valores)
- Placeholders pendientes: ninguno
- Seguimiento manual: crear `CLAUDE.md` en la raíz como guía operativa en tiempo de
  ejecución, referenciada por Governance
-->

# Teacher Learn Piano Songs App Constitution

## Core Principles

### I. Precisión Musical Primero (NO NEGOCIABLE)

El motor de evaluación es el producto; todo lo demás es presentación. La app MUST juzgar
cada nota por altura, momento de ataque (onset), duración y velocity, y MUST derivar el
tiempo del reloj de la sesión MIDI, nunca del reloj de la interfaz ni de temporizadores de
la UI. Las tolerancias de acierto (ventana de onset, umbral de duración, criterio de nota
extra u omitida) MUST estar definidas explícitamente en código y ser configurables por
nivel de dificultad, nunca constantes dispersas o implícitas.

Toda evaluación MUST ser determinista: la misma secuencia de eventos MIDI produce siempre
la misma puntuación. Cualquier cambio en las reglas de puntuación MUST ir acompañado de
fixtures de referencia ("golden files") con interpretaciones grabadas y su resultado
esperado, y MUST declarar en el PR qué fixtures cambian de resultado y por qué.

Rationale: un feedback incorrecto enseña mal a tocar. Un alumno autodidacta no tiene un
profesor que corrija a la app, así que un error de evaluación se convierte en un error
aprendido.

### II. Desarrollo Guiado por Pruebas (NO NEGOCIABLE)

TDD es obligatorio en todo el código de producción: primero la prueba, la prueba falla,
después la implementación mínima que la hace pasar, después el refactor. El ciclo
Red-Green-Refactor MUST ser visible en el historial de commits: un commit que introduce
comportamiento sin una prueba previa o acompañante que lo cubra MUST ser rechazado en
revisión.

Las pruebas MUST cubrir el comportamiento observable, no la estructura interna. Corregir un
defecto MUST empezar por una prueba que lo reproduzca y falle. Está prohibido debilitar o
silenciar una prueba para que pase el build; si una prueba es incorrecta, se corrige la
prueba de forma explícita y justificada en el PR.

Rationale: el núcleo musical es lógica temporal difícil de depurar a mano y prácticamente
imposible de verificar tocando manualmente cada caso. Las pruebas son la única forma
sostenible de saber que la app puntúa bien.

### III. Núcleo Determinista Desacoplado de la UI

La lógica de dominio —ingesta MIDI, modelo de tiempo, comparación contra la partitura,
puntuación y progreso— MUST vivir en el núcleo Rust y MUST ser ejecutable sin interfaz
gráfica y sin un teclado físico conectado. La capa TypeScript MUST limitarse a renderizar
estado y capturar intención del usuario; MUST NOT contener reglas de evaluación, tolerancias
ni cálculo de puntuación.

Todo comportamiento del núcleo MUST ser testeable alimentando flujos MIDI sintéticos con
marcas de tiempo explícitas. El límite entre núcleo e interfaz MUST expresarse como un
contrato de comandos y eventos tipado y versionado; los cambios incompatibles en ese
contrato MUST documentarse en el PR.

Rationale: separar el núcleo permite probar miles de interpretaciones en segundos en CI y
mantiene la ruta crítica de tiempo real libre del ciclo de render.

### IV. Tiempo Real con Presupuesto (<30 ms)

El retardo entre la pulsación de una tecla MIDI y su feedback visible MUST mantenerse por
debajo de 30 ms en el percentil 95 sobre el hardware de referencia del proyecto. La ruta
crítica (recepción MIDI → evaluación → emisión de evento a la UI) MUST NOT realizar E/S de
disco, consultas a base de datos, peticiones de red ni asignaciones de memoria no acotadas.

El presupuesto de latencia MUST medirse mediante un benchmark automatizado en CI. Una
regresión que supere el umbral MUST bloquear la fusión; relajar el umbral requiere una
enmienda a esta constitución, no un ajuste en el archivo de configuración del benchmark.

Rationale: por encima de ~30 ms el intérprete percibe el desfase y deja de confiar en el
feedback, lo que destruye el valor de practicar a tempo.

### V. Local Primero y Propiedad del Usuario

La app MUST ser completamente funcional sin conexión: importar canciones, practicar,
evaluar y consultar el progreso MUST NOT requerir red ni cuenta de usuario. Los datos del
alumno (sesiones, progreso, biblioteca) MUST persistirse localmente como fuente de verdad.

La sincronización en la nube MUST ser opt-in explícita, desactivada por defecto y
revocable, y el usuario MUST poder exportar y borrar todos sus datos desde la propia app.
La telemetría MUST ser opt-in y anónima; MUST NOT incluir contenido musical del usuario ni
identificadores personales. Los archivos MIDI o MusicXML importados por el usuario MUST NOT
salir del dispositivo salvo que el usuario active la sincronización.

Rationale: el usuario practica con su propio repertorio en su propia máquina; convertir eso
en dependencia de un servidor añade coste, riesgo de privacidad y modos de fallo sin
aportar valor al acto de practicar.

## Restricciones Técnicas y de Contenido

**Stack**: aplicación de escritorio construida con Tauri; núcleo de dominio en Rust,
interfaz en TypeScript. Plataformas objetivo: Windows y macOS, ambas MUST compilar y pasar
la suite completa en CI antes de cada release.

**Entrada**: el teclado MIDI es la única fuente de interpretación soportada en la v1. La
detección por micrófono está fuera de alcance; la arquitectura MUST NOT asumir MIDI como
único origen posible de eventos de nota, de modo que una fuente futura pueda añadirse sin
reescribir la evaluación.

**Contenido**: el repertorio proviene exclusivamente de archivos que el usuario importa
(.mid y MusicXML). El proyecto MUST NOT empaquetar, alojar ni redistribuir obras musicales
de terceros, y MUST NOT incluir un catálogo remoto de canciones sin una enmienda previa a
esta constitución que resuelva las obligaciones de licencia.

**Persistencia**: almacenamiento local embebido en el dispositivo, con migraciones de
esquema versionadas y con prueba automatizada de migración hacia adelante. Una actualización
de la app MUST NOT corromper ni descartar el progreso existente.

**Dependencias**: cualquier dependencia nueva en la ruta crítica de tiempo real MUST
justificarse en el PR indicando qué alternativa se descartó y por qué. Ante igualdad de
condiciones, se prefiere la biblioteca estándar sobre una dependencia externa.

## Flujo de Desarrollo y Puertas de Calidad

**Flujo**: todo trabajo sustancial sigue el ciclo de Spec Kit: `/speckit-specify` →
`/speckit-plan` → `/speckit-tasks` → `/speckit-implement`. Cada plan MUST incluir una
comprobación de cumplimiento contra esta constitución antes de generar tareas.

**Puertas obligatorias antes de fusionar**:

1. La suite de pruebas completa pasa en Windows y macOS.
2. Evidencia de TDD: la prueba del comportamiento nuevo existe y falló antes de la
   implementación.
3. Los fixtures de referencia de puntuación pasan sin cambios, o los cambios están
   justificados explícitamente en el PR.
4. El benchmark de latencia se mantiene bajo el umbral de 30 ms (p95).
5. Sin advertencias del linter ni del compilador en el código nuevo.
6. Ninguna funcionalidad nueva rompe el modo sin conexión.

**Revisión**: todo PR MUST verificar el cumplimiento de los cinco principios. La complejidad
añadida MUST justificarse; si un revisor no entiende por qué existe una abstracción, esa
abstracción se simplifica o se documenta. Un PR que viole un principio MUST ser rechazado o
acompañado de la enmienda correspondiente a esta constitución.

## Governance

Esta constitución prevalece sobre cualquier otra práctica, convención o preferencia del
proyecto. En caso de conflicto entre este documento y cualquier otra guía, prevalece este
documento.

**Enmiendas**: toda modificación MUST realizarse mediante `/speckit-constitution`, MUST
documentar qué cambia y por qué, y MUST incluir un plan de migración cuando invalide código
o procesos existentes. Las enmiendas entran en vigor al fusionarse.

**Versionado semántico** de este documento:

- MAJOR: se elimina o redefine un principio de forma incompatible con la práctica anterior.
- MINOR: se añade un principio o sección, o se amplía materialmente una guía existente.
- PATCH: aclaraciones, redacción o correcciones sin cambio de significado.

**Cumplimiento**: la conformidad se verifica en cada revisión de PR y en cada fase de
planificación. La guía operativa en tiempo de ejecución para agentes de desarrollo se
mantiene en `CLAUDE.md` en la raíz del repositorio; ese archivo MUST NOT contradecir esta
constitución y, ante discrepancia, se corrige `CLAUDE.md`.

**Version**: 1.0.0 | **Ratified**: 2026-08-17 | **Last Amended**: 2026-08-17
