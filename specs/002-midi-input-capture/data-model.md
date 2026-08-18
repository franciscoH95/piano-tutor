# Data Model: Captura MIDI del teclado

**Feature**: `002-midi-input-capture` | **Fecha**: 2026-08-18
**Fuente de las decisiones**: [research.md](./research.md)

Todos los tipos con lógica viven en `core/src/capture/`, sin ninguna dependencia de sistema. Se
reutilizan `Micros` y el trait `Clock` que ya existen de la feature 001.

## Regla transversal heredada

Sigue prohibido el punto flotante en todo el núcleo. Los instantes son `Micros(u64)`, los mismos
que usa la reproducción, porque son literalmente el mismo reloj.

## Identidad del dispositivo

### `Dispositivo`

| Campo | Tipo | Reglas |
| --- | --- | --- |
| `id_sistema` | `Option<DeviceId>` | Identidad **primaria**. En macOS el `unique_id` de CoreMIDI; en Windows la ruta de interfaz. `None` si la plataforma no lo ofrece. |
| `nombre` | `String` | Nombre legible. Si el sistema lo entrega vacío, se sustituye por una etiqueta generada para que el usuario pueda elegirlo igualmente. |
| `posicion` | `u16` | Posición entre los dispositivos que anuncian **ese mismo nombre**. Identidad de **reserva**. |

**Reconocimiento del teclado recordado**, en este orden estricto (FR-004b):

```
1. ¿Coincide `id_sistema`?          -> es este.
2. ¿Coincide (`nombre`, `posicion`)? -> es este.
3. Ninguno casa                      -> PEDIR AL USUARIO QUE ELIJA DE NUEVO.
```

El paso 3 nunca abre "el que más se parezca". Capturar del aparato equivocado sin avisar es peor
que no capturar: el alumno vería fallos que no cometió y no tendría forma de saber por qué.

**No se usa el índice de puerto** bajo ninguna circunstancia: se renumera al conectar o desconectar
cualquier otro aparato.

## Eventos

### `EventoCrudo` — lo que cruza la cola

Exactamente 16 bytes, `Copy`, `#[repr(C)]`. Es el único tipo que viaja por la ruta crítica, y su
tamaño importa porque determina cuántos caben en el espacio acotado.

| Campo | Tipo | Significado |
| --- | --- | --- |
| `at` | `Micros` | Instante sellado por el reloj de sesión. |
| `seq` | `u32` | Contador monótono. Un salto en la secuencia le dice al consumidor **exactamente dónde** hubo un descarte, sin gastar una ranura ni trabajo del productor. |
| `key` | `u8` | Altura MIDI. |
| `velocity` | `u8` | Intensidad. Cero significa suelta. |
| `kind` | `u8` | Ataque o suelta. |
| `channel` | `u8` | Canal MIDI. |

### `PulsacionCapturada` — la nota ya emparejada

| Campo | Tipo | Reglas |
| --- | --- | --- |
| `onset` | `Micros` | Instante del ataque. |
| `end` | `Micros` | Instante del final. Siempre `>= onset`. |
| `key` | `u8` | Altura. |
| `velocity` | `u8` | La del ataque; la de release se descarta. |
| `channel` | `u8` | Canal de origen. |
| `closure` | `Cierre` | Cómo terminó. |
| `duracion_censurada` | `bool` | `true` cuando sabemos cuándo empezó pero no cuándo terminó de verdad. |

### `Cierre`

```
PorSuelta                 la tecla se soltó de verdad
PorParada                 el usuario detuvo la captura con la tecla hundida (D6)
PorPerdidaDeDispositivo   el teclado desapareció (duración censurada)
PorRepulsacion            la misma tecla volvió a pulsarse sin haberse soltado
```

La distinción no es decorativa: una feature posterior necesitará saber que un final no lo produjo
el alumno antes de puntuarlo. Guardarlo en el dato, y no en un registro, es lo que permite que las
pruebas asserten sobre ello.

## Transporte

> **Glosario**: lo que la especificación llama *almacén intermedio* es este `Transporte`. Se
> conservan los dos nombres a propósito: la spec habla para quien decide y este documento para
> quien implementa. Son la misma cosa.

### `Transporte` — la cola acotada

| Propiedad | Valor | Por qué |
| --- | --- | --- |
| Capacidad | 4.096 eventos = **64 KiB** | Dos órdenes de magnitud sobre la ráfaga humana más densa (~50 eventos/s). Cubre más de 80 s sin que el consumidor toque nada. |
| Reserva | Una sola vez, al abrir | La ruta crítica no puede asignar. |
| Al llenarse | Descarta **lo entrante**, incrementa el contador | Descartar lo ya almacenado dejaría ataques sin su suelta: notas huérfanas. |
| Productor | Nunca bloquea | El Principio IV lo prohíbe, y bloquear el hilo del sistema haría que el driver descarte por su cuenta, sin que nos enteremos. |

**Despertar del consumidor**: `park`/`unpark` con testigo `quiere_despertar`. El `unpark` solo se
emite en la transición de vacío a no vacío. El testigo es pegajoso: si el `unpark` llega antes del
`park`, el `park` retorna de inmediato, así que no hay aviso perdido.

## Sesión

### `SesionDeCaptura` — transiciones de estado

```
      Inactiva
         │ elegir dispositivo
         ▼
      Abriendo ──── falla ────► Error (se comunica; la app sigue viva)
         │ éxito
         ▼
    Capturando ◄──────────────┐
         │                    │ reabrir
         ├── detener ─────────┤
         │   (cierra hundidas │
         │    con PorParada)  │
         │                    │
         └── dispositivo      │
             desaparece ──► Perdida
                 (cierra hundidas con PorPerdidaDeDispositivo,
                  selladas en el instante del ÚLTIMO evento recibido,
                  marcadas como duración censurada)
```

El sellado en el último evento recibido, y no en el instante de la detección, es deliberado: la
detección llega hasta un segundo más tarde, y datar la nota ahí inventaría duración que no ocurrió.

### `InformeDeCaptura`

Seis contadores `u32`, todos a cero en una sesión limpia:

`sueltas_sin_pulsacion`, `cerradas_por_parada`, `cerradas_por_perdida`, `repulsaciones`,
`percusion`, `fuera_de_88_teclas`.

Dos que podrían esperarse y **no están aquí**, cada uno por su motivo:

- **Descartes por desbordamiento**: existen, pero viven en el transporte
  (`Emisor::descartados`), no en este informe. Describen un problema de la ruta crítica, no
  del material que se tocó, y mezclarlos obligaría a meter un contador atómico compartido
  dentro de un tipo que hoy es puro dato.
- **Mensajes descartados por no ser notas**: **no se cuentan**. Descartar un pedal o un
  mensaje de reloj es el funcionamiento normal que exige FR-014, no una anomalía tolerada.
  Un teclado que emita reloj MIDI genera veinticuatro mensajes por negra: el contador sería
  un número enorme y sin significado.

Sin registro en disco ni en consola: son exactamente los valores sobre los que assertan las pruebas
de casos límite, igual que el informe de carga de la feature 001.

## Contrato de la fuente

### `FuenteDeEventos`

Trait inyectado **por genérico**, no como `dyn`, igual que el `Clock` que ya existe: no paga
despacho dinámico en la ruta crítica.

| Implementación | Dónde | Qué permite probar |
| --- | --- | --- |
| `FuenteGuionizada` | `core/` | Toda la lógica de dominio con instantes fijos y reproducibles (FR-021, FR-022). |
| `ColaCaptura` | `core/` | El transporte real y el despertar, con un productor sintético. |
| `MidiIoSource` | `midi-io/` | El hardware de verdad. Solo a mano, con teclado delante. |

## Reglas de validación derivadas de los requisitos

| Regla | Requisito | Dónde se aplica |
| --- | --- | --- |
| Identidad primaria = id del sistema; reserva = (nombre, posición) | FR-004b | `dispositivo.rs` |
| Si ninguna casa, preguntar; nunca abrir otro | FR-004c | `dispositivo.rs` |
| Pulsación con intensidad cero = suelta | FR-009 | `midi-io/parser.rs` |
| Solo notas; pedal y demás se descartan | FR-014 | `midi-io/parser.rs`, antes de encolar |
| Instantes **no decrecientes**, no estrictamente crecientes | FR-013 | `transporte.rs` |
| Emparejamiento FIFO por voz | FR-008 | `emparejador.rs` |
| Teclas hundidas al parar se cierran y se etiquetan | FR-015 | `emparejador.rs` |
| Espacio acotado; al llenarse descarta lo entrante y cuenta | FR-011a/b/c | `transporte.rs` |
| Un solo reloj de sesión para captura y reproducción | FR-012a | `src-tauri`, en el arranque |
