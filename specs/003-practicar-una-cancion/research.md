# Research: Practicar una canción — decisiones técnicas

**Feature**: `003-practicar-una-cancion` | **Fecha**: 2026-08-18

> **Todo lo que sigue está medido en macOS**, en un portátil Apple Silicon con pantalla de dpr 2.
> La ruta de Windows (WebView2, que es Chromium y sigue el vsync del monitor) **no se ha ejecutado
> ni una sola vez**: es el riesgo número uno de esta feature y está anotado como tal.

## Lo que la medición cambió respecto a lo planeado

Dos hallazgos que no salieron de razonar sino de medir, y que obligan a tocar la especificación:

1. **SC-003, tal como está escrito, es inmedible.** «El 99 % de los fotogramas se dibuja en menos
   de 16,7 ms» suena riguroso y no lo es: con el vsync enganchado, el intervalo entre fotogramas
   vale 16,667 ms *por construcción*, independientemente de lo que cueste dibujar. La prueba
   decisiva: un pintor que **no dibuja nada** dio p95 34 ms y 16 % de fotogramas por encima de
   25 ms — **peor** que el pintor real con 59 notas. El criterio hay que reescribirlo en forma
   operativa, y la Decisión 1 propone las cinco cifras que sí significan algo.
2. **SC-003 no se puede verificar sin ventana, y está medido**: con la WebView creada pero no
   adjunta a una ventana visible, `requestAnimationFrame` disparó **cero veces en 5.001 ms**. Los
   runners de integración continua no tienen pantalla, así que la puerta de PR tiene que recaer
   sobre otra cosa. La Decisión 1 desdobla la verificación en dos niveles.

Ambas recomendaciones de rendimiento pasaron por un verificador adversarial que intentó romperlas
midiendo, y **ninguna se rompió**.

---

## Decisión 1: Técnica de dibujo de las notas cayendo

**Decisión**: **Canvas 2D**, un único lienzo, redibujado inmediato y completo por fotograma, texto con `fillText` directo (sin atlas de glifos), culling por cursor monótono sobre la línea temporal ya ordenada por ataque. La capa de dibujo es una **función sumidero**: recibe una lista de rectángulos y etiquetas ya calculada por el núcleo y no decide nada. WebGL2 queda documentado como salida de emergencia estructuralmente abierta pero **no implementada**.

**SC-003 se reescribe en forma operativa** (la redacción literal es inmedible: un pintor que no dibuja nada la falla). Puerta de aceptación, cinco cifras, todas producidas por el banco:

- (a) **Déficit de fotogramas** = 1 − observados / (60 × segundos_activos) < **0,5 %**.
- (b) Proporción de intervalos de rAF > **25 ms** < **0,1 %**.
- (c) **Cero** intervalos > **33,4 ms** (dos vsyncs).
- (d) Suspensiones de la WebView (todo intervalo > **200 ms**) detectadas, **excluidas** de los percentiles y **declaradas** en el informe. Un banco que no las declara produce un p99 inventado.
- (e) Coste de CPU del dibujo medido **aparte**, en bucle apretado amortizado sobre ≥2.000 repeticiones (porque `performance.now()` está recortado a exactamente 1,000 ms), presupuesto < 16,7 ms.

Dos niveles de verificación: **puerta obligatoria y determinista en CI** sobre el *productor* (la función pura Rust que convierte posición del cursor en lista de rectángulos y etiquetas, probada headless con presupuesto duro), y **trabajo nocturno o manual con ventana real, visible y al frente** para las cifras (a)-(e), publicado como artefacto y **no** como bloqueo de PR.

**Justificación**: Verificación adversarial independiente, arnés propio, 10 minutos reales, 59 notas visibles de media y 118 cadenas por fotograma, teclado de 88 teclas, lienzo 1440×900 CSS = 2880×1800 px (dpr 2): **36.002 fotogramas en 600 s = 60,00 fps, déficit de fotogramas 0,006 %**, p50 17 / p95 18 / p99 20 / p99.9 23 / máx 31 ms, 9 fotogramas sobre 25 ms (0,025 %), **cero** sobre 33,4 ms. Segundo soak a 4× la densidad (169 visibles, 338 cadenas): 60,00 fps, déficit 0,009 %, 3 fotogramas sobre 25 ms, cero sobre 33,4 ms, 0,342 ms de CPU (margen ×49). Techo real: 439 notas visibles y 878 cadenas siguen a 60,00 fps con **cero** fotogramas sobre 25 ms y 0,898 ms de CPU — 11× el punto de trabajo (40-80 notas).

El texto de D2 no es caro: **0,865-0,869 µs por cadena corta**, confirmado de forma independiente. Con 118 cadenas son ~0,10 ms = 0,60 % del presupuesto. El atlas pre-rasterizado está **refutado por medida**: con 2.386 etiquetas, `fillText` 1.962 µs frente a 2.447 µs (atlas canvas) y 2.515 µs (ImageBitmap). WebKit ya cachea glifos y agrupa tiradas de texto; cada `drawImage` es un quad independiente.

El culling no necesita estructura: cursor monótono con cota superior de duración de nota, **<1 µs por fotograma con 10.000 notas, 2 µs con 200.000**, promedio de 4,2 µs en el soak (0,025 % del presupuesto). SC-008a (salto): búsqueda binaria más redibujado completo, media **0,167 ms**, p95 1 ms, máx 3 ms, contra 100 ms de límite. SC-002: 1.000 notas son 78 KB de JSON y <1 ms de parseo, contra 2 s de límite.

**Alternativas consideradas**:

- **DOM** (pool de divs, `translate3d`, `contain:strict`): descartado por dato. Con 58 notas visibles p50 27 ms, 37,20 fps, 69,62 % de fotogramas sobre 25 ms; con 169, 15,05 fps. Falla **por debajo** del requisito real. Dato instructivo: DOM gastaba 0,081 ms en JS, **menos** que Canvas (0,131 ms), y aun así iba a 37 fps — el coste se paga en el commit, fuera de JavaScript.
- **SVG**: funciona. Corrección explícita a la investigación original, que decía «cuesta el triple»: la verificación mide **1,3×** (0,1715 vs 0,1315 ms) y SVG aguanta 60,00 fps con cero fotogramas perdidos en el punto de trabajo. **Su rechazo es arquitectónico, no de rendimiento**, y hay que presentarlo así: mete la geometría de cada nota en el árbol del documento, justo donde el plan de 003 dice que no debe vivir ninguna decisión. Canvas hace ese error imposible por construcción.
- **WebGL2 instanciado**: no descartado, **aplazado**. Mantuvo 60 fps con 19.100 notas visibles; 75 µs por fotograma con 1.193 notas y 2.386 etiquetas en 2 draw calls. Ese margen resuelve un problema que no existe (Canvas ya sobra ×11). Cuesta 4-18 ms de compilación de shaders por escena, ~250 líneas de instancing/VAOs/buffers, manejo de pérdida de contexto y un modo de fallo opaco. **Regla de activación explícita**: si se miden >2.000 notas visibles sostenidas, o si Windows con WebView2 a 165 Hz no cumple con Canvas, se sustituye el sumidero por el de WebGL sin tocar nada más.
- **Biblioteca WebGL (PixiJS, regl, two.js)**: dependencia de terceros y capa que auditar para un problema medido en 0,3-0,6 ms sobre 16,7. Incompatible con un proyecto que vigila mecánicamente su árbol de dependencias.
- **Árbol de intervalos / R-tree para el culling**: innecesario por dos órdenes de magnitud (ver cifras arriba). Además `Playback::advance_to` y `Playback::seek` ya dan exactamente esa estructura.
- **Medir SC-003 headless como puerta de PR**: **imposible, medido**. Con la WKWebView creada pero no adjunta a ventana visible, `requestAnimationFrame` disparó **0 veces en 5.001 ms** y `visibilityState` valía `hidden`. Minimizada corre degradada a 53,8 Hz. Por eso el desdoblamiento en dos niveles.

**Riesgos y mitigación**:

1. **Windows/WebView2 sin medir — riesgo número uno.** WebView2 es Chromium y su rAF sigue el vsync del monitor: a 120/144/165 Hz el presupuesto es 8,3/6,9/6,1 ms, no 16,7. Canvas consume 0,13-0,34 ms en el punto de trabajo, así que *debería* caber, pero es extrapolación. Mitigación estructural obligatoria: **la posición de cada nota se deriva SIEMPRE del reloj monótono, nunca del número de fotograma**, con lo que la cadencia deja de importar para la corrección. Mitigación de verificación: repetir el barrido en Windows antes de dar SC-003 por cumplido; el arnés es portable (HTML + servidor Python + anfitrión).
2. **El intervalo de rAF no mide coste de dibujo.** Control decisivo: un pintor que **no dibuja nada** dio p95 34 ms, p99 35 ms y 16,07 % sobre 25 ms — **peor** que Canvas dibujando 59 notas. Con vsync enganchado el intervalo vale 16,667 ms por construcción y el reloj recortado a 1 ms lo lee como 16 o 17. Por eso la cifra que sostiene la conclusión es el **recuento de fotogramas** (criterio a), no el percentil ni el cronómetro dentro del callback.
3. **La WebView se suspende en silencio al ser tapada.** Primer soak: 430 de 600 s perdidos en cinco huecos, uno de 283 s; con `caffeinate` seguían perdiéndose 232 s; solo forzando la ventana al frente cada 2 s bajó a 6,2 s. La verificación cazó 2 suspensiones de ~1,87 s y el `occlusionState` nativo las registró en el mismo instante. El banco **debe** aplicar el criterio (d).
4. **Efectos visuales con acantilados invisibles desde JS.** `shadowBlur = 8` con 298 notas cayó a 40,9 fps y 3,55 % de fotogramas sobre 25 ms mientras el cronómetro interno marcaba 0,748 ms. Matiz de la verificación: a 58 notas visibles `shadowBlur` mantuvo 60 fps y cero fotogramas sobre 25 ms — el acantilado es real pero solo a densidades muy superiores. Regla: cualquier sombra, desenfoque o filtro pasa por el banco antes de entrar, y el criterio es el recuento de fotogramas, nunca el cronómetro.
5. **La frontera núcleo/pintor es gratis** (ataque fallido, se reporta como riesgo cerrado): la versión que asigna una lista nueva por fotograma (~4,3 M objetos en 10 min) da 0,1315 ms frente a 0,1310 ms de la versión con arrays reutilizados, mismo p99, y el p99 por minuto **mejora** con el tiempo (20,20,19,20,20,20,20,20,19,18). No hay que optimizar la frontera prematuramente.
6. **dpr > 2 sin medir**. Todo es dpr 2. Un monitor 5K externo o dpr 3 multiplica la tasa de relleno, a la que Canvas 2D es sensible. Medir antes de prometer SC-003 en cualquier máquina.
7. **Coste de CPU es cota inferior**: no incluye composición. Por eso nunca es el criterio principal.

---

## Decisión 2: Puente Rust ↔ interfaz

**Decisión**: **Empujar** cada evento de tecla por un `tauri::ipc::Channel<EventoTecla>` abierto una vez por sesión, con **payload JSON**, **un envío por evento** (sin agrupar), desde un **hilo reenviador propio**. **Cero sondeo por fotograma.** El cursor no cruza 60 veces por segundo: se cruza un **ancla** (posición, instante, ritmo, próximo tope) solo al cambiar de régimen, y el frontend interpola linealmente entre anclas.

Contrato de implementación, sin ambigüedad:

1. **Un `Channel` por sesión**, creado por el frontend y registrado con un `invoke` al arrancar la práctica; Rust lo guarda en el estado gestionado. **Un solo canal** para eventos de tecla y anclas de cursor, discriminado por una etiqueta en el payload, para que el orden entre ambos quede garantizado por construcción.
2. Quien llama a `send` es un **hilo reenviador**, consumidor de la cola rtrb de 4096 que ya existe en `core/src/capture/transporte.rs`, usando el `Receptor::esperar()` ya escrito (duerme, no sondea). **Nunca** el callback de CoreMIDI ni el consumidor de tiempo real: `send` cuesta p95 0,5-1,6 ms y máx 5-13 ms cuando la interfaz pinta, y asigna en cada llamada (`format!` del script + serde). Eso en la ruta crítica viola el Principio IV.
3. **JSON, no binario.** Por debajo de 1024 bytes Tauri re-codifica los bytes crudos como array JSON de decimales (`tauri-2.11.5/src/ipc/channel.rs:163`, `MAX_RAW_DIRECT_EXECUTE_THRESHOLD = 1024` en la línea 39): 16 bytes crudos se convierten en **39 bytes de texto**. Por encima de 1024 cambia a un viaje `fetch` que **añade** una ida y vuelta completa.
4. **Un envío por evento.** Agrupar por fotograma da el mismo p50/p95 pero empeora el p99 de 34,0-36,4 ms a **48,2-50,6 ms**, a cambio de ahorrar 4,55 µs por segundo. La agrupación se reserva **solo como camino de desbordamiento**: si el reenviador encuentra muchos eventos en una pasada (glissando, acorde denso), envía un lote — **topado por debajo de 8192 bytes**, ver riesgo 4.
5. **Cada evento lleva su sello `at` en `Micros`** del reloj de sesión y un `seq` monótono.
6. **CORRECCIÓN — el descarte por antigüedad se elimina.** La investigación proponía «el frontend descarta lo que llegue con más de dos fotogramas de retraso». La verificación adversarial lo **refutó como fallo de corrección**: los eventos de tecla son **flancos, no niveles**; descartar un note-off deja la tecla encendida para siempre y descartar un note-on la deja apagada mientras se pulsa, y el descarte se dispararía justo en los atascos. Regla que sí se sostiene: **los eventos de tecla se aplican tarde, nunca se tiran**; el hueco de `seq` solo sirve para pedir la **resincronización**. Las **anclas de cursor sí son niveles** y sí pueden descartarse por obsolescencia.
7. **`invoke` se usa, pero nunca por fotograma**: registrar el canal, resincronizar el estado completo de las 88 teclas al saltar (D10, SC-008a) y recuperarse de un hueco de `seq`. Ciclo completo p50 232-252 µs, p95 296-357 µs, y el tamaño del payload no se nota (224-250 µs p50 con 1 o con 50 eventos, JSON o binario).
8. `Channel::new` es público y acepta un cierre cualquiera: el puente entero se ejercita **sin ventana, sin webview y sin event loop**, afirmando sobre los bytes exactos que cruzarían. Eso satisface SC-007 y el Principio III y permite TDD estricto (Principio II) antes de que exista una línea de interfaz.

**Justificación**: La verificación construyó una app Tauri 2.11.5 real que implementa este diseño y **dibuja de verdad** la escena que la investigación original admitía no dibujar (canvas 2560×1536 reales, teclado de 88 teclas, 40-90 notas con nombre latino y dedo). 17.000 eventos con recorrido completo, 0 perdidos, 0 descartados, 0 errores JS. Recorrido total hasta pintar, 1000 eventos por condición a ~10/s (ritmo real de quien toca), todo cronometrado con el reloj de Rust:

| Condición | p50 | p95 | p99 |
|---|---|---|---|
| 88 divs planos | 27,9 ms | **35,8 ms** | 37,8 ms |
| Canvas real (notas + nombres + dedos) | 28,4 ms | **35,9 ms** | 38,1 ms |
| Modo espera, cursor parado (D5) | 28,2 ms | **35,6 ms** | 37,6 ms |
| Denso, 90 notas + acordes | 28,5 ms | **36,3 ms** | 38,1 ms |

**Doce reproducciones independientes en cuatro corridas: p95 siempre entre 33,4 y 37,1 ms.** Margen contra los 50 ms de SC-004: **~14 ms**.

Dónde se va el tiempo (canvas real, p50): cola rtrb **14 µs (0,05 %)**, `Channel::send` **20 µs (0,07 %)**, puente send→manejador JS **727 µs (2,6 %)**, espera al fotograma **10,3 ms (36,3 %)**, pintado y composición **16,8 ms (59,2 %)**. **El 95,5 % del presupuesto se lo come la tubería de 60 Hz; el puente entero es el 2,6 % y Rust el 0,12 %.** Todo el debate de mecanismos se juega dentro de 1-3 ms sobre un suelo de ~28 ms p50 / ~35 ms p95 que no depende del transporte.

Serializar un evento a JSON cuesta **91 ns y 62 bytes**; a 50 eventos/s son 4,55 µs por segundo, el 0,00046 % de un núcleo. El puente no es el problema.

Sobre por qué `Channel` y no `emit`: **no es por latencia**. La prueba A/B entrelazada (600 eventos alternando los cuatro mecanismos evento a evento) da `emit` 1251/4034, `emit_to` 1392/3465, `canal` 1155/3979, `canal_raw` 1315/3364 µs (p50/p95): **son el mismo número**, y corrige un falso hallazgo previo que era un artefacto de orden. Las tres razones reales: (i) `Channel` se prueba headless y `emit` exige un webview real; (ii) `emit` difunde a todos los webviews con búsqueda de oyentes por nombre en JS, trabajo regalado con un solo consumidor; (iii) `listen`/`emit` exigen la capability ACL `core:event:default` y sin ella el frontend muere con `Command plugin:event|listen not allowed by ACL`, **entero y en silencio**, sin una línea de log. La verificación corroboró esto **por omisión**: sus capabilities declaran solo `core:default`, `core:webview:default` y `core:window:default`, y el `Channel` más los comandos propios funcionaron en las cuatro corridas **sin una sola entrada de ACL**.

**Alternativas consideradas**:

- **Sondear el estado con un `invoke` por fotograma**: la alternativa que había que tomarse en serio y la que peor sale. Misma convención y misma sesión: p50 41,4 ms, **p95 49,0 ms**, contra 27,4/35,4 ms del empuje. Agota el presupuesto de 50 ms ella sola repintando solo 88 divs planos. La causa está aislada: la **latencia de detección de un muestreo discreto** es p50 7,9-9,0 ms y p95 15,3-16,3 ms — exactamente medio fotograma y un fotograma, porque muestrear pierde en promedio medio periodo, siempre. Adelgazar el payload no salva nada: json 49,0 / binario 49,2 / bitmap de 16 bytes 48,6 ms p95.
- **Sondeo adelantado** (lanzar el `invoke` justo tras dibujar): la forma más lista del sondeo, medida para no montar un espantapájaros. **Peor**: p50 58,4 ms, p95 65,8 ms. El dato que dibujas en el fotograma N se muestreó justo después del N−1 y llega rancio de un fotograma entero por construcción. No hay dónde colocar el sondeo.
- **`emit` / `emit_to`**: mismos números de latencia; descartados por ACL, difusión innecesaria y, sobre todo, por no ser ejercitables headless (SC-007, Principio III).
- **Binario crudo (`InvokeResponseBody::Raw`)**: falsa optimización verificada en fuente. Ganas 30 ns por evento (1,5 µs/s a 50 eventos/s) a cambio de que el frontend descifre un `DataView`, y por encima de 1024 bytes es peor.
- **Agrupar uno por fotograma**: p99 14 ms peor. Solo camino de desbordamiento.
- **Empujar la posición del cursor 60 veces por segundo**: trabajo cierto por un dato que nace rancio, y duplica el tráfico. `PorReloj` es una función del tiempo: un ancla y una interpolación lineal dan el valor exacto en el instante del fotograma. La regla sigue en el núcleo, determinista y probada headless.
- **`send` desde el callback de CoreMIDI o el consumidor de tiempo real**: prohibido por el Principio IV y confirmado midiendo (p95 hasta 1,6 ms, máx 13 ms, con asignación en cada llamada).

**Riesgos y mitigación**:

1. **CORRECCIÓN de la justificación original: la banda «18-35 ms p95» es falsa.** La investigación afirmaba que el doble rAF sobrestima en hasta 16,7 ms. La verificación lo refuta con su propia medida: la etapa raf1→raf2 da p50 16,2-17,2 ms (un periodo exacto de 60 Hz) y **raf1 se sella ANTES de dibujar**, así que «valor medido − 16,7 ms» aterriza *antes* del pintado del fotograma que contiene el cambio; los fotones no pueden preceder al pintado. **raf2 es el fotón más temprano posible, no el más tardío.** El margen real es **~14 ms, no 15-32 ms**. (No se pudieron medir fotones: `screencapture` denegado en el entorno.)
2. **CORRECCIÓN: el margen no se gasta dibujando, se pierde entero al perder un fotograma.** Dibujar las notas, nombres y dedos cuesta 0-2 ms p95 de JS y **no consume el margen**. Los ~14 ms son 0,8 fotogramas. Barrido de carga por fotograma: 0/4/8 ms → 60 fps y p95 34,6/33,6/33,4 ms; **20 ms → 48,8 fps y p95 40,8 ms; 33 ms → 24,5 fps, p50 ya en 54,2 ms y p95 2159 ms**. **SC-004 no es un presupuesto independiente: es SC-003 dicho de otro modo.** Si el fotograma se pasa, ambos caen a la vez, y el puente no será la causa. Ese mismo margen es también el de portabilidad a Windows.
3. **La cola del puente es ilimitada y no pierde nada, y eso es el riesgo.** Con el hilo principal secuestrado 5 s, Rust emitió 100.000 eventos en 125,3 ms (media 1 µs, máx 108 µs) sin bloquearse, llegaron los 100.000 con 0 huecos y drenaron en ~2,2 s. Un destello 2 s tarde es peor que ningún destello. Mitigación, ya corregida en el punto 6 de la decisión: **no se descartan flancos**; se aplican tarde, y la defensa real es el camino de **resincronización** por `invoke` (232-252 µs p50) ante hueco de `seq` o salto. Las anclas de cursor sí se descartan por obsolescencia. La cola rtrb de 4096 acota el lado de Rust; **no acota el lado del webview**, que es el que crece.
4. **Hallazgo nuevo de la verificación: el orden del `Channel` se garantiza BLOQUEANDO.** `@tauri-apps/api` 2.11.1 `core.js` líneas 75-113 buferea por índice (`#pendingMessages`, `#nextMessageIndex`), así que **un mensaje lento retrasa a todos los posteriores**. Un lote de desbordamiento en un glissando (176 eventos × ~62 B ≈ 11 KB) cruza `MAX_JSON_DIRECT_EXECUTE_THRESHOLD = 8192`, se va por el viaje `fetch` y **atasca detrás todos los eventos de tecla**. Mitigación obligatoria: **topar el tamaño del lote por debajo de 8192 bytes** y partirlo si hace falta.
5. **La feature `tracing` de Tauri cambia el puente en silencio.** Sin ella, `eval_script` es `send_user_message` → `proxy.send_event`, no bloqueante. Con ella pasa a `getter!` y **bloquea al llamante**, convirtiendo al hilo reenviador en rehén del hilo de interfaz y metiendo un bloqueo en una ruta sujeta al Principio IV. **Debe quedar apagada, con puerta mecánica de CI**, igual que la de `cargo tree`.
6. **El p99 no es robusto.** En 2 de 12 condiciones sanas apareció un congelamiento único del frontend de 2,3 s y 3,2 s, alojado en la espera al fotograma: el proceso WebContent no entregó fotograma. No es del puente y ningún diseño de puente lo arregla; **el peor fue en modo DOM**, lo que refuerza la Decisión 1. SC-004 es p95 y p95 aguanta, pero «50 ms p95» **no debe leerse como «50 ms siempre»**.
7. **`performance.now()` en WKWebView tiene granularidad de 1000,00 µs.** El frontend no puede cronometrar ni sellar nada por debajo del milisegundo. **Toda la autoridad temporal se queda en Rust**; cualquier verificación de SC-004 apoyada en el reloj del navegador estará midiendo su propio redondeo.
8. **Windows sin medir.** Todo es macOS/M1 Max a 60 Hz. La *decisión* (empujar, no sondear) descansa en un argumento de muestreo independiente de la plataforma y debería sobrevivir; los *números* hay que volver a tomarlos antes de dar SC-004 por cumplido en ambas plataformas.

---

## Decisión 3: Abstracción del cursor

**Decisión**: Módulo nuevo **`core/src/practica/`** en `piano-core` (`cursor.rs`, `puertas.rs`, `sonando.rs`, `sesion.rs`), reexportado desde `lib.rs`. Tres piezas:

**(1) `Cursor`: struct plano con el modo como CAMPO de datos** — ni trait genérico ni enum con comportamiento.

```rust
pub enum Avance { PorReloj, PorAcierto }   // 1 byte, Copy. SIN comportamiento: solo elige el techo.
pub struct Velocidad { num: u32, den: u32 }  // racional; pausa = 0/1. Sin flotantes.
pub struct MascaraTeclas([u64; 2]);          // 128 teclas, 16 bytes, Copy

pub struct Cursor {
    avance: Avance,
    velocidad: Velocidad,
    ancla_real: Micros,     // instante del reloj de sesion en el ultimo rebase
    ancla_cancion: Micros,  // posicion de cancion en ese mismo rebase
    pos: Micros,
    ultimo_t: Micros,
    puertas: Arc<ProgramaDePuertas>,
    puerta: usize,          // indice de la primera puerta pendiente
    hundidas: MascaraTeclas,
    consumidas: MascaraTeclas,
    fin: Micros,
}

pub struct SesionDePractica<C: Clock, F: FuenteDeEventos> {
    reloj: C, fuente: F, cursor: Cursor,
    sonando: ConjuntoSonando, playback: Playback, /* ... */
}
```

El genérico se conserva donde tiene sentido (reloj y fuente, **un nivel más arriba**, en la sesión); el enum, donde tiene sentido (elegir un techo, un `match` de dos brazos que compila a cmov).

**(2) `ProgramaDePuertas`**: `Box<[Puerta]>` precalculado en la carga, con `Puerta { onset_us: Micros, requeridas: MascaraTeclas, primera: u32, ultima: u32 }` (32 bytes). Una puerta por cada tramo maximal de notas que comparten `onset_tick` (la línea temporal ya viene ordenada, `core/src/timeline.rs:193`), filtrando por la mano practicada (D7, FR-026a, SC-012).

**(3) `ConjuntoSonando`**: barrido con `orden_por_fin: Box<[u32]>` (permutación por `(end_us, indice)`, materializada una vez) y `cuentas: [u16; 128]`. `suena(key)` es `cuentas[key] > 0`: **O(1), una lectura de array**.

**Invariante exacta del modo espera** (t = reloj de sesión monótono; v = num/den):

```
proyeccion(t) = ancla_cancion + floor((t - ancla_real) * num / den)   // u128 intermedio, UNA division
techo(t)      = Micros(u64::MAX)                     si avance == PorReloj
              = puertas[puerta].onset_us (o MAX)     si avance == PorAcierto

I1:  pos(t) = max( pos(t⁻), min( proyeccion(t), techo(t) ) )
```

**Regla de rebase I2** (la otra mitad, donde está toda la sustancia): en el instante `t_s` en que se satisface la puerta P, **si y solo si el techo estaba sujetando** (`pos(t_s⁻) == P.onset_us`):

```
ancla_cancion := P.onset_us ;   ancla_real := max(t_s, ultimo_t)
```

Si el alumno se adelantó, **no se rebasa nada** y el cursor sigue fluyendo sin dar ningún salto.

**Acorde (D6), sin ninguna ventana**:
```
puerta satisfecha  ⟺  requeridas ⊆ (hundidas \ consumidas)
```
Un AND, un AND-NOT y una comparación sobre dos `u64`. **No se compara ni un solo instante**: la simultaneidad es *estructural* (coexistencia en el conjunto de hundidas), no *métrica*. Satisfacer una puerta marca sus teclas en `consumidas`; una tecla vuelve a contar solo tras un `Ataque` fresco, que limpia su bit.

**`Playback` no necesita ni una línea de cambio**: se le alimenta con la **posición del cursor** en lugar del reloj crudo.

**`seek(a, t)` (D10)** — regla limpia: *reinicia todo lo derivado de la canción, conserva todo lo que es hecho del mundo físico* (solo `hundidas`):
1. `pos = ancla_cancion = a`; `ancla_real = max(t, ultimo_t)`.
2. `puerta = puertas.partition_point(|p| p.onset_us < a)` — con `<` **estricto**, para que aterrizar sobre un acorde lo deje pendiente y no regalado.
3. `ConjuntoSonando::reposicionar(a)` (dos `partition_point` + reconstrucción de `cuentas`).
4. `Playback::seek(a)` (ya existe, O(log n), `feedforward.rs:202`).
5. **`consumidas |= hundidas`** — todo lo hundido queda rancio. Es lo que impide que saltar sobre un acorde que el alumno casualmente tiene hundido lo abra gratis.
6. Se descarta la contabilidad pendiente de «omitidas».

**`cambiar_modo(m, t)` (FR-021)**: `avanzar_a(t)` para asentar → `ancla_cancion = pos; ancla_real = max(t, ultimo_t)` → `consumidas |= hundidas` → `avance = m`. **Sin ese rebase, pasar de espera a tempo salta hacia delante la espera acumulada entera.** Idéntico para `cambiar_velocidad` (FR-010). `saltar_puerta(t)` (FR-020) marca la puerta como satisfecha sin entrada por el **mismo camino de código**.

**Justificación**: `Clock` y `FuenteDeEventos` son genéricos porque se sustituyen en la **frontera** y no cambian durante la sesión. El modo **sí cambia a mitad de canción conservando la posición** (FR-021, D4), así que un genérico lo convertiría en un cambio de **tipo**: obligaría a reconstruir el cursor perdiendo justo el estado que hay que conservar, o a meter `dyn` en la ruta crítica. Y contradice literalmente D4: «cambiar de modo es un parámetro de sesión, no una rama de código distinta».

Propiedades derivadas, todas verificadas numéricamente en el prototipo ejecutado **contra el crate real** (18 baterías en verde):

- **I3 (FR-018a, el ritmo transcurre de verdad)**: mientras `proyeccion(t) < techo`, se cumple exactamente `pos(t2) − pos(t1) = (t2 − t1)·v`.
- **I4 (FR-018, la parada)**: `pos(t) ≤ onset_us(primera puerta pendiente)` en todo instante. Eso **es**, formalmente, «detenerse al llegar a una nota no tocada».
- **I5 (continuidad)**: sin discontinuidad al liberar; **la espera nunca se recupera**. Verificado: esperar 4,5 s ante una puerta y liberarla continúa en 500 ms, no en 5 s.
- **I6 (D4, un solo camino)**: `PorReloj ≡ PorAcierto con techo ≡ MAX`, **verificado bit a bit sobre 188 fotogramas**.
- **I7 (monotonía)**: `techo` es no decreciente (puertas ordenadas, índice solo crece) y `proyeccion` es no decreciente en t; el mínimo de dos no decrecientes es no decreciente. **`pos` es monótona por construcción, no por comprobación.**

`Playback` **tolera el estancamiento hoy**: `advance_to` solo falla con `now < self.last` (`core/src/feedforward.rs:174`); `now == self.last` ya es legal y devuelve un corte vacío, y la suite ya lo afirma (`core/tests/feedforward_test.rs:144`). Verificado: 1.000 `advance_to` con el mismo instante, cero errores, 1 comparación cada uno. **La monotonía de ese código ya es NO ESTRICTA, y por eso el modo espera encaja sin tocarlo**; un fotograma estancado cuesta *menos*, no más. Sobreviven intactas: cues ordenados por `cue_at`, cada cue emitido exactamente una vez, coste k+1 comparaciones independiente del tamaño, `seek` como único camino atrás.

**Se reformula (documentación y aserciones, no código)**: `Playback::position()` pasa a significar «posición dentro de la canción», no «instante de reloj alcanzado» (solo coinciden a velocidad 1/1 sin estancamientos); `Rewind` deja de ser alcanzable por el usuario — por I7 un `Err` solo indica invariante del cursor rota, y se conserva el `Result` como defensa en profundidad afirmando en pruebas que nunca dispara; nace `debug_assert!(playback.position() == cursor.pos())` tras cada tick — **el cursor es la única fuente de verdad**; e `is_finished()` habla de avisos, no de la pieza: el final (FR-011) es `pos >= song.duration_us()` **y** sin puertas pendientes, o en modo espera la canción «terminaría» esperando su último acorde.

**Números medidos** (prototipo `probe-cursor`, `cargo run --release`, todo verde): tubería entera (cursor + barrido + `Playback`) sobre 20.000 notas y 74.999 fotogramas: **607 ms totales = 8 µs por fotograma**, 0,05 % del presupuesto de 16,7 ms de SC-003. Salto completo (cursor + reposicionar sonando + `Playback::seek`) sobre 20.000 notas: **peor caso 10 µs** contra 100 ms de SC-008a. Construir `ProgramaDePuertas` para 20.000 notas: **125 µs**; orden por final: **23 µs**, contra 2 s de SC-002. Barrido de sonando: **amortizado 2 pasos por nota en toda la pieza** (7 pasos totales para 4 notas en 200 fotogramas), nunca n por fotograma.

**Dónde vive**: en `piano-core`. Es lógica entera determinista sin E/S, sin ventana y sin dispositivo — la definición misma del núcleo del Principio III; la Constitución **prohíbe explícitamente** que TypeScript contenga reglas de evaluación, tolerancias o puntuación. **No añade ninguna dependencia**: `MascaraTeclas`, `[u16;128]` y `partition_point` son std, y `cargo tree -p piano-core` sigue dando exactamente 3 líneas (verificado: piano-core, midi_file 0.2.0, rtrb 0.3.4). Se ejercita entero headless con `VirtualClock` + `FuenteGuionizada`, que ya existen: **SC-007 se cumple por construcción y el Principio II aplica sin excepción** — aquí no hay ningún adaptador de plataforma que pueda acogerse a ella. Sin flotantes (racional u32/u32, una multiplicación u128 y una división por consulta), así que `deny(clippy::float_arithmetic)` se mantiene. Fuera del núcleo: leer el archivo, instanciar `MonotonicClock` (ya hecho, y es justo la base de tiempo de `ancla_real`), bombear eventos, serializar la instantánea y dibujarla. La app llama a `sesion.tick()` una vez por fotograma y recibe una instantánea; **no toma ninguna decisión**. La digitación (FR-030..033, D3, SC-010, SC-011) también es núcleo pero es un **módulo aparte** (`core/src/digitacion/`) que consume el reparto de manos: no se mezcla con el cursor.

**Alternativas consideradas**:

- **`Cursor<A: Avance>` con trait genérico**: cambiar de modo sería un cambio de tipo. Contradice D4 y FR-021.
- **`Box<dyn Avance>`**: despacho dinámico en ruta crítica, prohibido por convención del proyecto; y no aporta nada, hay dos modos y ninguno lleva comportamiento.
- **Enum con comportamiento por variante**: reintroduce por la puerta de atrás la rama que D4 prohíbe y duplica la lógica de `seek` y `cambiar_modo` en dos sitios — justo donde nacen los bugs de notas colgando (FR-007b).
- **Dos tipos separados (`ReproduccionPorReloj` / `ReproduccionPorAcierto`)**: verificado bit a bit que son la misma función con distinto techo; duplicaría anclas, velocidad, seek y estado de teclado y haría imposible FR-021 sin traspaso manual campo a campo.
- **Meter el modo dentro de `Playback`**: rompería su prueba de coste (`cost_invariant_test.rs`) y pondría estado de entrada junto al corte de avisos. Alimentarlo con la posición del cursor deja `feedforward.rs` con **cero líneas modificadas**.
- **Árbol de intervalos / segment tree para «qué suena ahora»**: O(log n + k) con asignación para una pregunta que el barrido monótono responde en O(1) con una lectura de array, precisamente porque el cursor es monótono por I7.
- **Máscara de bits para las notas sonando**: dos notas de la misma altura en pistas distintas pueden solaparse (el cargador solo acorta el solapamiento de la **misma voz**, `timeline.rs` fase 2); un bit lo apagaría la primera en terminar. `[u16;128]` = 256 bytes exactos.
- **Ventana de tolerancia para el acorde**: prohibida por D9/FR-014b/FR-022a y además innecesaria — la contención de conjuntos no compara instantes, no hay constante que ajustar. Verificado: tres notas con **30 s** de separación pero coexistiendo hundidas → **abre**; las mismas pulsadas y soltadas una tras otra con **1 µs** → **no abre**.
- **Derivar `pos` del reloj sin anclas**: cambiar velocidad o pausar produciría un salto instantáneo, violando FR-010.
- **Rebasar las anclas en cada fotograma**: acumula el error de truncado de la división — la deriva contra la que advierte la cabecera de `core/src/tempo.rs`.
- **Leer las hundidas de la tabla de `Emparejador`**: 2048 ranuras por consulta contra dos palabras de 64 bits, y hoy no expone esa consulta. Se acepta la duplicación mínima (dos consumidores del mismo flujo de `EventoCrudo`) con un `debug_assert!` cruzado en pruebas.
- **Cursor en la capa de app (src-tauri o React)**: prohibido por la Constitución; perdería SC-007, SC-010 y el determinismo entero.

**Riesgos y mitigación**:

1. **SIN RESOLVER — caso límite real detectado: `onset_us == end_us`.** Con `ppq = 30_000` y tempo 10.000 µs/negra (ambos legales: `core/src/tempo.rs:68` admite ppq hasta 32.767, y el truncado está documentado en la cabecera del mismo archivo), notas de 1 tick producen `onset_us == end_us` pese a `end_tick > onset_tick`; reproducido con las 2 notas de una pieza y `suena(60)` en t=0 devolviendo `false`. Con la definición literal de D9 esas notas **no pueden casar jamás** y serían siempre «omitidas». **Decisión provisional: responder D9 en TICKS**, donde el cargador ya garantiza `end_tick > onset_tick`, con una prueba que lo fije. Alternativa si se prefiere trabajar en µs: definir el intervalo sonoro como `[onset_us, max(end_us, onset_us + 1))`. Elegir en el plan, no dejarlo abierto en el código.
2. **SIN RESOLVER — agrupación de acordes cuantizados a mano.** Las puertas se agrupan por `onset_tick` **idéntico**; un acorde repartido en 1-2 ticks se convertiría en varias puertas, obligando a un gesto nota a nota, lo contrario de D6. **Decisión provisional: agrupar por un epsilon musical en ticks EN LA CARGA**, que es determinista y **no** viola FR-014b (no es tolerancia sobre lo que toca el alumno, es lectura del archivo). El valor concreto del epsilon queda por fijar en el plan con MIDIs reales.
3. **`hundidas` indexa solo por altura**, mientras `Emparejador` indexa por (canal, altura): una tecla hundida en el canal 1 y soltada en el 2 apagaría el bit antes de tiempo. Despreciable con un solo teclado físico. Mitigación: documentarlo como supuesto o usar `[u8;128]` en lugar de bits.
4. **Eventos desordenados**: `EventoCrudo.at` y el `now` del fotograma vienen del mismo reloj pero un evento puede llegar sellado antes del último fotograma dibujado. Contemplado con `ancla_real = max(at, ultimo_t)`; **sin esa cláusula un evento tardío rebasa hacia el pasado y el cursor pega un tirón hacia delante**. Necesita prueba explícita con `FuenteGuionizada`.
5. **FR-014a (notas omitidas)** necesita un vector de notas activas con bit `tocada`. Debe dimensionarse con la **polifonía máxima calculada en la carga por el mismo barrido** (sin coste extra); si no, es asignación no acotada en ruta crítica y viola el Principio IV.
6. **FR-020 depende de que `saltar_puerta` sea alcanzable desde la UI.** Si la interfaz lo omite, el modo espera se bloquea para siempre ante una nota que el teclado del alumno no tiene (teclado de 61 teclas contra pieza de 88). Riesgo de producto, no de núcleo, pero el núcleo debe exponerlo desde el primer día.
7. **Cambiar la mano (D7) o mover el corte por altura (D8, FR-003c) reconstruye `ProgramaDePuertas`** con otro filtro: hay que recolocar `puerta` con `partition_point` **y** aplicar `consumidas |= hundidas`, exactamente igual que en `seek`. Si se olvida, el índice apunta a otra puerta y el cursor espera la nota equivocada.
8. **Pedal**: hoy se descarta en la frontera (`capture/evento.rs`). Cuando se añada, decidir **explícitamente que el pedal NO modifica `hundidas`**: una nota sostenida por pedal suena, pero la tecla no está hundida, y confundirlo abriría puertas solo.
9. **No reintroducir el estancamiento en el bucle de la interfaz.** Si la app decide «no dibujar cuando el cursor no avanza», el alumno pierde el reflejo de sus propias teclas en modo espera y **SC-004 cae**. El fotograma se dibuja siempre; lo que se estanca es la posición de la canción, no el render. (La verificación del puente lo midió: el modo espera con cursor parado, repintando solo 1001 de 6096 fotogramas por firma de escena sin cambios, mantiene 60,0 fps y p95 35,6 ms — la optimización por firma **sí** es legítima; apagar el bucle no.)

---

## Decisión 4: Algoritmo de digitación

**Decisión**: Módulo `core/src/digitacion/` con el modelo ergonómico de **Parncutt et al. (1997)** —12 reglas sobre una tabla de vanos en semitonos— resuelto por **programación dinámica exacta de segundo orden**, en aritmética exclusivamente `i32`. Sin dependencias nuevas: la puerta de `cargo tree -p piano-core` de 3 líneas queda intacta.

**Convenios obligatorios (si se codifican mal, todo lo demás falla):**
1. **Vano canónico**: la distancia se mide SIEMPRE del dedo de número menor al mayor. Para el par (3,1) con intervalo ascendente de +3 semitonos, el vano canónico es **−3**, no +3. Sin esto el paso del pulgar no se detecta nunca.
2. **Mano izquierda = derecha reflejada**: altura relativa a la mano `h(p) = p` (derecha), `h(p) = −p` (izquierda). Las mismas tablas y reglas sirven para ambas. El color de tecla (blanca/negra) se consulta siempre sobre la altura MIDI real, que **no** se refleja.

**Tabla de vanos (Parncutt Tabla 1), semitonos, mano derecha** — DATOS, con `#[rustfmt::skip]`:

| Par f-g | MinPrac | MinComf | MinRel | MaxRel | MaxComf | MaxPrac |
|---|---|---|---|---|---|---|
| 1-2 | −5 | −3 | 1 | 5 | 8 | 10 |
| 1-3 | −4 | −2 | 3 | 7 | 10 | 12 |
| 1-4 | −3 | −1 | 5 | 9 | 12 | 14 |
| 1-5 | −1 | 1 | 7 | 10 | 13 | 15 |
| 2-3 | 1 | 1 | 1 | 2 | 3 | 5 |
| 2-4 | 1 | 1 | 3 | 4 | 5 | 7 |
| 2-5 | 2 | 2 | 5 | 6 | 8 | 10 |
| 3-4 | 1 | 1 | 1 | 2 | 2 | 4 |
| 3-5 | 1 | 1 | 3 | 4 | 5 | 7 |
| 4-5 | 1 | 1 | 1 | 2 | 3 | 5 |

Los pares dedo-consigo-mismo tienen los seis umbrales a **0**: así «no repetir dedo entre notas consecutivas distintas» sale gratis del mismo mecanismo, sin regla aparte.

**Las 14 reglas.** Sobre pares consecutivos (s = vano canónico, umbrales del par):

1. **Estiramiento**: `2` puntos por semitono en que s excede MaxComf o queda bajo MinComf.
2. **Vano pequeño**: si s < MinRel, `1` punto/semitono si interviene el pulgar, `2` si no.
3. **Vano grande**: si s > MaxRel, `1` punto/semitono con pulgar, `2` sin él.

Sobre ternas (1.ª y 3.ª nota):

4. **Número de cambios de posición**: si el vano canónico 1.ª–3.ª sale de MinComf..MaxComf hay cambio. `2` puntos si es COMPLETO (la nota de en medio la toca el pulgar **y** su altura queda entre las otras dos **y** el vano 1-3 sale de MinPrac..MaxPrac), `1` si es medio. Si 1.ª y 3.ª son la misma altura con dedos distintos → medio cambio de tamaño cero (`1`).
5. **Tamaño del cambio**: `1` punto por semitono de diferencia entre el vano 1-3 y el umbral cómodo violado.

Fuerza y agilidad:

6. **Dedo débil**: `1` punto por cada uso del 4 o del 5.
7. **Tres-cuatro-cinco**: `1` punto cada vez que 3, 4 y 5 aparecen consecutivos en cualquier orden (los grupos se solapan).
8. **Tres-a-cuatro**: `1` punto cada vez que al 3 le sigue inmediatamente el 4.
9. **Cuatro en negra**: `1` punto si 3 y 4 van consecutivos en cualquier orden con el 3 en blanca y el 4 en negra.

Blancas y negras:

10. **Pulgar en negra**: `1` siempre que el pulgar toque negra; `+2` si la nota anterior es blanca; `+2` si la siguiente es blanca (máx. 5).
11. **Cinco en negra**: `0` si anterior y siguiente son negras; `+2` si la anterior es blanca; `+2` si la siguiente es blanca.
12. **Paso del pulgar**: hay paso cuando interviene el pulgar y el vano canónico es negativo. `1` punto si es al mismo nivel (blanca→blanca o negra→negra); `3` si la nota grave es blanca con dedo distinto del pulgar y la aguda es negra con el pulgar. El caso fácil —del dedo en negra al pulgar en blanca, que es lo que hacen los libros de escalas— cuesta **0**.

Dos reglas propias, necesarias y verificadas en prototipo:

13. **Extremos del acorde** (solo lajas de ≥2 notas): `2 · ((a₀−1) + (5−a_k))`, es decir 2 puntos por cada dedo que queda colgando fuera del acorde. **Sin ella, Do-Mi-Sol sale 1-2-4; con ella sale 1-3-5.**
14. **Desplazamiento de la mano** entre lajas: `1` punto por semitono de distancia entre las posiciones implícitas del pulgar, con posición implícita = `h(nota más grave de la mano) − MinRel(1, dedo de esa nota)`, donde `MinRel(1,f) = [0, 1, 3, 5, 7]`.

**Alcance practicable**: en vez de prohibir (Parncutt prohíbe y su red se queda sin caminos), se cobra **`100` puntos por semitono** fuera de MinPrac..MaxPrac. Barrera blanda: nunca se elige si hay alternativa, pero **garantiza que siempre hay solución** (FR-032, SC-009).

**Función de coste**, con los pesos enteros **congelados como constantes en el código**:

```
coste = Σ coste_nota(fᵢ) + Σ coste_par(fᵢ₋₁,fᵢ) + Σ coste_terna(fᵢ₋₂,fᵢ₋₁,fᵢ)

estiramiento 1 · vano 1 · cambio_num 1 · cambio_tam 1 · debil4 1 · debil5 1
tres45 1 · tres_a_cuatro 1 · cuatro_negra 1
pulgar_negra 4 · cinco_negra 4 · paso_pulgar 4     <- calibrados
extremos 2 · desplazamiento 1                      <- reglas propias
```

**Algoritmo**: Viterbi determinista con **estado = (dedo de la nota anterior, dedo de la actual)** — hacen falta los dos porque las reglas 4, 5 y 7 miran tres notas. 25 estados por nota × 5 predecesores = **125 evaluaciones de transición por nota**. Retropropagación con tabla de padres de 25 bytes/nota (125 KB a 5.000 notas). Dos arreglos fijos `[i32;25]`, sin asignaciones en el bucle interno, sin `dyn`, sin E/S. **Determinismo estructural**: iteración en orden fijo de dedo (1→5) y comparación con `<` estricto ⇒ ante empate gana siempre el lexicográficamente menor. Sin HashMap, sin flotantes, sin dependencia del orden de sumas ni de la plataforma. SC-010 es una propiedad del diseño, no una esperanza. Usar `saturating_add` en la acumulación.

**Polifonía — el mismo algoritmo, otra unidad.** La unidad pasa de NOTA a **LAJA**: todas las notas de una mano que atacan en el mismo `onset_tick` (tenemos ticks exactos, basta la igualdad; el umbral publicado de 30 ms no hace falta).
- **Candidatas por laja**: subconjuntos crecientes de {1..5} de tamaño k → C(5,k) ≤ **10**. «Creciente en altura relativa a la mano» codifica la única restricción que Nakamura conserva: **no se cruzan dedos dentro de un acorde**. Para la izquierda, «creciente» significa dedo mayor en la nota más grave, y sale solo del reflejo `h(p) = −p`.
- **Coste vertical**: reglas de vano y barrera de alcance sobre **todos** los pares del acorde (≤10), con puntuación **doblada** (Balliauw regla 14), más la regla 13. Cubrir todos los pares y no solo los adyacentes es lo que impide proponer un acorde que la mano no abarca.
- **Coste horizontal**: **NO se aplican las reglas melódicas entre lajas**, se aplica la regla 14 (desplazamiento de la mano). Esto es lo importante y va enunciado explícitamente en el plan.
- **Estado de la PD**: (asignación anterior, asignación actual) ≤ 10×10 = 100 estados, ≤1.000 transiciones por laja.
- **Propiedad verificada**: con lajas de una sola nota, el camino polifónico produce **exactamente** el mismo resultado que el melódico. Es un algoritmo con dos casos, no dos algoritmos.
- **Más de 5 notas simultáneas en una mano** — sin resolver en la literatura. **Decisión provisional**: recortar a las 5 notas EXTREMAS de la laja (la más grave y las 4 restantes por orden de altura desde los extremos hacia dentro) y marcar el resto como «sin dedo sugerido», nunca como error; la nota se sigue dibujando y sigue exigiéndose para el acierto. Se elige por encima de reasignar a la otra mano porque reasignar rompería D7/D8 (el reparto lo manda el alumno). Revisable si aparece repertorio real que lo justifique.

**Coste computacional medido** (prototipo, rustc 1.97.1, `-O`, una mano): 1.000 notas **1,9 ms** · 5.000 notas **9,7 ms** · 20.000 notas **40 ms**. Con acordes (~2,5 notas/laja) los tiempos son iguales dentro del ruido. Lineal, ~2 µs/nota. Dos manos y 5.000 notas: **~20 ms, el 1 % del presupuesto de 2 s de SC-002**. Consecuencia para D8/FR-003c: 1.000 notas cuestan 1,9 ms, así que **mover el corte de manos recalcula reparto y digitación por debajo de un fotograma** — sin incrementalidad, sin caché, sin hilo aparte.

**Digitación canónica que SC-011 debe afirmar** — escala de Do mayor, una octava (MD Do4→Do5, MI Do3→Do4):

| | Ascendente | Descendente |
|---|---|---|
| **Mano derecha** | **1 2 3 1 2 3 4 5** | **5 4 3 2 1 3 2 1** |
| **Mano izquierda** | **5 4 3 2 1 3 2 1** | **1 2 3 1 2 3 4 5** |

Alturas MIDI construidas en la prueba, sin archivo `.mid`: `[60,62,64,65,67,69,71,72]` y `[48,50,52,53,55,57,59,60]`. Coinciden literalmente dos referencias independientes (piano.org y Piano with Norbert, que la atribuye al estándar ABRSM), confirmadas en descendente por swindonpianolessons. El prototipo produce las cuatro secuencias **exactas ya con los pesos originales de Parncutt, sin calibrar**.

**Batería de pruebas (headless, sin ventana ni teclado — SC-007, Principio III)**:
1. **SC-011 estricta**: cuatro igualdades exactas sobre las secuencias de arriba.
2. **SC-011 ampliada, NO bloqueante**: 12 escalas mayores × 2 manos × 2 direcciones con umbral ≥90 % de notas, nunca igualdad. Red de seguridad ante regresiones al tocar pesos.
3. **SC-010**: N pasajes con semilla fija, 100 repeticiones, igualdad. Medido: 200 pasajes × 20 repeticiones × 2 manos = 8.000 ejecuciones, **0 discrepancias**.
4. **SC-009**: propiedad — toda nota recibe dedo en 1..=5, probada con un pasaje deliberadamente inejecutable (saltos de 3 octavas nota a nota) para ejercitar la barrera blanda.
5. **Acordes**: tríada 1-3-5, octava 1-5, séptima 1-2-3-5, progresión I-V-vi-IV (1-3-5 las cuatro); y coherencia: lajas de una nota == camino melódico.
6. **Presupuesto**: 5.000 notas × 2 manos por debajo de 200 ms (×10 de margen).
7. **Adversarial de desbordamiento** con pasaje patológico.

**TDD (Principio II)**: la digitación es núcleo puro —entra un corte de notas, sale un vector de dedos—, **no hay adaptador de plataforma y no aplica ninguna excepción**. Todo test-first, en este orden: (1) tablas y vano canónico, con los tres ejemplos numéricos del artículo como casos literales; (2) cada una de las 12 reglas por separado; (3) PD sobre pasajes de 3 notas; (4) SC-011; (5) lajas y acordes.

**Justificación**: en el banco PIG (Nakamura et al., Tabla 2, 30 piezas) el acuerdo **entre pianistas humanos es del 71,4 %**. No existe «la digitación correcta». El mejor método publicado (HMM de 2.º orden, 64,3 %) está a 7 puntos del techo humano; las reglas, a ~11. Pagar datos entrenados, coma flotante y licencia de corpus por ~4 puntos, en un producto que ya presenta la digitación como SUGERENCIA (D3, FR-006c), no compensa. Y el enfoque de reglas es el único **interpretable**: cuando el alumno pregunte «¿por qué este dedo?», hay respuesta. Todos los pesos publicados de Parncutt son enteros; el único valor fraccionario de la literatura (regla 8 de Balliauw, +0,5) desaparece al escalar por 2. `grep` sobre el prototipo confirma cero apariciones de f32/f64/sqrt/pow. Resultados medidos: 12 escalas mayores × 2 manos × 2 direcciones (384 notas) **356/384 = 92,7 %** con pesos calibrados (76,0 % sin calibrar); notas interiores **280/288 = 97,2 %**. Acordes verificados: Do M 60-64-67 → 1-3-5 · 1.ª inversión 64-67-72 → 1-2-5 · séptima 60-64-67-71 → 1-2-3-5 · octava 60-72 → 1-5 · Fa# M todo negras → 1-3-5 · vals de MI (bajo suelto + acorde Mi-Sol) → bajo 5, acorde 3-1. Los tres ejemplos numéricos del artículo original se reproducen exactamente (Mi4-Sol4 con 3-1 → 6 de vano pequeño; Do4-Mi4-Sol4 con 2-1-2 → 7, con 2-1-3 → 4, con 3-1-2 → 8; Do4-Si4 con 1-3 → 4 de vano grande + 2 de estiramiento), lo que confirma que el vano canónico es el convenio correcto.

**Alternativas consideradas**:
- **HMM de 2.º orden entrenado con PIG** (Mgen 64,3 %, el mejor medido): descartado. Gana ~4 puntos sobre un techo humano del 71,4 %, a cambio de empaquetar parámetros en coma flotante —prohibida por `deny(clippy::float_arithmetic)`, habría que cuantizar log-probabilidades a enteros y demostrar que la cuantización no cambia el argmax—, asumir licencia y atribución del corpus, y perder la interpretabilidad. **Queda como mejora posterior sin coste de reescritura: el esqueleto de la PD es EL MISMO Viterbi, solo cambia de dónde salen los números.**
- **Redes neuronales (seq2seq con haz, LSTM, feed-forward)**: descartadas. Miden **peor** que un HMM de 1.er orden en el mismo banco (61,3 % y 61,5 % frente a 61,7 %) y bastante peor en coherencia secuencial (Mrec 69,5 % frente a 74,0 %), pidiendo runtime de inferencia, pesos de decenas de MB y coma flotante.
- **Aprendizaje por refuerzo** (PianoFingering.jl): descartado. No determinista por construcción — choca de frente con SC-010 y con el principio de determinismo.
- **Metaheurísticas (VNS de Balliauw, tabú)**: descartadas por dos motivos independientes. Estocásticas (sin semilla congelada violan SC-010; con ella siguen sin garantizar el óptimo) y son las **peores** del banco: Mgen 56,7 % frente a 63,1 % del HMM de 2.º orden sobre el mismo subconjunto, y el método **no consiguió producir digitación para 16 de las 30 piezas**. Sí se les toma prestada la idea de aplicar los vanos dentro del acorde con puntuación doblada.
- **Reglas voraces sin búsqueda**: descartadas. El paso del pulgar es intrínsecamente una decisión con anticipación —en Do mayor ascendente el pulgar va en Fa por lo que viene DESPUÉS—; un voraz solo acertaría SC-011 por casualidad. Y la PD exacta cuesta 2 µs/nota: no se ahorra nada.
- **Tabla codificada a mano de escalas y arpegios**: descartada. Pasaría SC-011 trivialmente y fallaría FR-032 (digitación para CUALQUIER canción cargable); sería hacer trampa con el criterio. Como mejora posterior legítima —el propio Parncutt observa que los pianistas aplican digitaciones estándar al reconocer un patrón— un pre-paso que detecte tramos diatónicos con la PD de respaldo sería la vía para arreglar el anclaje del 5 en escalas de teclas negras.
- **Geometría de 14 unidades por octava** (teclas negras imaginarias entre Mi-Fa y Si-Do, Balliauw/Jacobs): descartada por ahora. Es físicamente más fiel, pero las tablas publicadas y validadas están en semitonos y adoptar la rejilla de 14 obliga a rederivar los 6 umbrales de los 10 pares sin ninguna referencia contra la que contrastar. Anotado como refinamiento futuro.
- **Búsqueda exhaustiva sobre 5^n o búsqueda por haz**: la exhaustiva es imposible (el propio Parncutt la limita a 8 notas); el haz es innecesario porque la PD de 25 estados ya es exacta, óptima global y cuesta 2 µs/nota.

**Riesgos y mitigación**:
- **Sobreajuste de los pesos.** Los tres pesos subidos a 4 se eligieron por descenso por coordenadas sobre un banco que son SOLO escalas. Una primera pasada sin restricciones llegó al 94 % **apagando por completo tres reglas** (estiramiento, número de cambios de posición, dedo 4), lo que habría destrozado la digitación de música real. *Mitigado*: ningún peso baja de 1 (ninguna regla se desactiva) y se conservaron los tres pesos con justificación pedagógica independiente (pulgar fuera de las negras, paso del pulgar caro, meñique fuera de las negras) en vez de los que prefería el optimizador. *Pendiente*: congelar los pesos como constantes y tratar el banco de escalas como prueba de REGRESIÓN, jamás como entrenamiento en tiempo de ejecución. Robustez medida: con 2.000 vectores de pesos aleatorios en 1..=4, **1.969 (98,4 %)** siguen dando la escala canónica en las dos manos — SC-011 no está en el filo de la calibración.
- **El modelo de Parncutt es de legato de dedo.** Aplicarlo entre acordes en bloque es un error de categoría, no un ajuste fino: repetir dedo entre acordes consecutivos dispara la barrera (los pares f-f tienen umbrales a 0). En el prototipo, I-V-vi-IV salía `[2,4,5] [1,2,4] [2,3,5] [1,2,3]`; con la regla 14 sale `[1,3,5]` en las cuatro. *Mitigado y probado*, pero el plan debe **enunciarlo explícitamente** o alguien lo reintroducirá al refactorizar. *Corolario abierto*: la frontera exacta entre «legato» y «desplazamiento» para notas sucesivas de una voz separadas por un silencio largo no está resuelta. **Decisión provisional**: hoy se decide solo por si hay acorde de por medio; no se introduce umbral de silencio hasta tener repertorio que lo motive.
- **Techo humano del 71,4 %.** No se puede prometer «la digitación correcta» en ningún texto de interfaz ni criterio de aceptación. D3/FR-006c ya lo plantean bien; mantener ese lenguaje también en tests y plan.
- **Anclaje en los extremos.** El modelo no sabe que una escala de una octava es fragmento de algo más largo y no ancla el 5 arriba en escalas de teclas negras: ahí están 24 de los 28 errores del banco. Do mayor no se ve afectada. *Mitigación*: SC-011 se queda en «una escala sencilla», como está escrito hoy; **si alguien lo amplía a las 12 tonalidades, fallará**.
- **Tamaño de mano fijo.** La tabla asume mano adulta media; un niño no alcanza MaxPrac(1-5)=15 semitonos. Aceptable no parametrizarlo en esta entrega, pero **la tabla debe vivir en un solo sitio y ser un parámetro**, no constantes esparcidas.
- **Desbordamiento entero.** Peor caso: barrera 100 × 88 semitonos × 5.000 notas ≈ 4,4·10⁷, dentro de `i32` (2,1·10⁹) con dos órdenes de margen. *Mitigación*: `saturating_add` y prueba adversarial, coherente con la política de «sin pánicos» del núcleo.
- **Deuda de TDD.** Tablas y 12 reglas son mucha superficie que tienta a escribir de un tirón. El artículo trae ejemplos numéricos verificados que sirven de casos literales regla por regla; si no se usan desde el principio, un error de signo en el vano canónico pasa desapercibido y se manifiesta como «la digitación es rara», imposible de depurar después.

---

## Decisión 5: Reparto de manos

**Decisión**: **VOZ = par (track, channel)** con al menos una nota, descartado el canal 9 (percusión). `ScheduledNote` ya conserva ambos campos (`core/src/timeline.rs:40-61`).

El archivo **«trae las manos separadas»** si y solo si se cumplen las **tres guardas**:
- **G1**: hay exactamente **2 voces con notas**.
- **G2**: mismo instrumento — mismo canal, o mismo programa resuelto por canal, o ninguna declara programa.
- **G3**: cada voz tiene **≥5 %** de las notas **Y** las medianas de altura difieren **≥3 semitonos**.

Si se cumplen: **MANO DERECHA = la voz de mediana de altura más alta**, jamás por índice de pista. Desempate determinista por `(track, channel)` ascendente.

Si no se cumplen: **corte por altura nota a nota, umbral por defecto 60 (Do central)**, ajustable por el alumno. Nota con `key ≥ corte` → derecha; `key < corte` → izquierda.

**El control del corte debe estar SIEMPRE disponible**, con «usar las voces del archivo» como valor por defecto cuando se detectan. No se oculta nunca.

**Justificación**: no existe convención en la especificación MIDI; lo que hay son costumbres de cada programa. Medidos **250 archivos de 5 corpus** con un lector SMF escrito para esto (`.../scratchpad/smfstat.py`):

- **piano-midi.de** (secuenciado a mano, 60): formato 1, 60/60 con pista 0 de dirección sin notas, 56/60 con exactamente 2 voces, 60/60 con nombre de pista que dice la mano, y —clave— **59/60 usan UN SOLO CANAL para las dos manos**.
- **ASAP midi_score** (de MusicXML vía MuseScore, 40): 39/40 con 2 voces, **40/40 con TODO en el canal 0**, 0/40 con nombre de pista, 39/40 sin pista de dirección (la pista 0 ya lleva notas).
- **Mutopia/LilyPond** (10): canales distintos por voz, nombres heterogéneos, a menudo más de 2 voces.
- **ASAP performance** (Disklavier, 40): **40/40 con UNA sola voz**. Cero separación.
- **ADL pop/rock de Lakh** (100): 44 con 1 voz, 43 con 2; de los 42 de 2 voces, solo ~20 son manos de verdad.

Conclusión: **el canal es inútil para separar manos** (fallaría en ~98 % del material clásico); la separación vive en la PISTA — pero LilyPond sí usa canales distintos. Por eso la unidad es el par, no uno de los dos.

**Por qué la mediana y no el orden de pista**: en los 75 archivos cuyo nombre declara la mano, la derecha va primero en **74/75** — es tendencia fuerte, no regla. El contraejemplo es real y es salida de LilyPond (`mut/wtk1-prelude1.mid`: pista 1 = «lower:2» = izquierda, pista 2 = «upper:» = derecha). En cambio **«la voz de mediana más alta es la derecha» acierta 75/75**, con separación mediana de 16,7 semitonos (mín 3,2 / máx 29,4). Cuesta lo mismo y no falla.

**Por qué 60 y no el óptimo medido**: datos contra la mano real del archivo — error en 60: media **18,3 %** de las notas en piano-midi.de (mediana 17,6 %, mín 3,7 %, máx 36,2 %) y **15,4 %** en ASAP. Mejor corte fijo global: 62-63, con **16,6 %** — gana 1-2 puntos. Mejor corte por pieza (oráculo): **12,4 %**, o sea ajustar el corte recupera **~6 puntos como techo absoluto**. Los rangos de las dos manos se solapan en **56/56 piezas**, con solape mediano de **21,5 semitonos**: ningún corte fijo puede resolver eso. Elegir 60 y no 62 es deliberado: 62 gana 1-2 puntos y pierde la única explicación que el alumno entiende sin ayuda — «la línea está en el Do central».

**Validación de las guardas**: **0 falsos negativos** sobre los 75 pares confirmados por nombre (con G3 a 3 semitonos; a 5 se pierde 1 de 75), y rechazan **20 de 28** archivos pop de 2 voces que no son manos. Cobertura: piano-midi.de 56/60, ASAP score 39/40, Mutopia 5/10, ASAP performance 0/40 (correcto: todos caen al corte), ADL pop 22 detectados / 21 rechazados. **Hay que contar VOCES CON NOTAS, no pistas**: según el generador la pista 0 es de dirección o ya lleva música, y contar pistas a secas da resultados distintos.

Corrobora la elección la literatura: Hadjakos et al. 2019 lo dicen textualmente — *«most systems allocate notes by splitting at the middle C… This approach is highly inaccurate as soon as one hand crosses this split point»*. PianoBooster *«mixes all tracks together and guesses the note hand based on its pitch»*. Synthesia es el caso opuesto: no propone corte automático, obliga a *«hacer clic cerca de la mitad de las dos manos»* y ajustar a mano.

**Lo que falta en el código**: el loader actual solo trata `SetTempo` y `EndOfTrack` (`core/src/midi/loader.rs:134` y `:141`) y descarta el formato SMF tras validarlo (líneas 76-81). Hay que conservar formato SMF, nombres de pista y armaduras. **Sin dependencias nuevas**: `midi_file` 0.2.0 ya expone `MetaEvent::TrackName(Text)` (`meta_event.rs:58`) y `MetaEvent::KeySignature` (`:138`).

**Alternativas consideradas**:
- **Asignar la mano por índice de pista**: descartada. No existe tal convención en la especificación; 74/75 la cumplen pero hay contraejemplo real de LilyPond. Basta uno para que el alumno vea las manos cambiadas sin poder explicárselo, y la mediana acierta 75/75 al mismo coste.
- **Separar por canal MIDI**: descartada. 59/60 de piano-midi.de y 40/40 de ASAP ponen ambas manos en el mismo canal — fallaría en ~98 % del material clásico.
- **Corte adaptativo automático (Otsu entero sobre el histograma, o mediana de la pieza)**: descartado. Otsu da 15,4 % de error medio frente a 18,3 % — gana 2-3 puntos — pero **empeora el peor caso (47,1 % frente a 36,2 %)** y coloca la línea donde el alumno no puede predecir ni explicar. La mediana simple no mejora nada (17,0 %).
- **Corte por defecto en 62-63, el óptimo global medido**: descartado. Gana 1-2 puntos y pierde la explicación pedagógica del Do central.
- **Separación con aprendizaje automático (RNN o Kalman, Hadjakos et al.)**: descartada pese a ser claramente mejor — 93,25 % en tiempo real y 94,47 % en diferido frente al ~82-85 % del corte en Do central, unos 10 puntos. Exige empaquetar un modelo entrenado y el punto flotante que necesita está prohibido en el núcleo (`deny(clippy::float_arithmetic)`) y pondría en riesgo el determinismo bit a bit de SC-010.
- **Ocultar el control del corte cuando se detectan voces separadas** (como sugería la clarificación del spec): descartada. La detección se equivoca de verdad: de 42 archivos pop con 2 voces, ~20 eran manos y ~22 no. Si el archivo se detecta mal y el control está oculto, el alumno se queda sin salida.
- **Pedir al alumno que dibuje la línea, como Synthesia**: descartada como requisito previo — es trabajo antes de tocar la primera nota. La idea de fondo (el corte lo manda el alumno) sí se conserva: es exactamente D8.

**Riesgos y mitigación**:
- **Falsos positivos de detección**: dos voces pueden no ser dos manos (melodía + acompañamiento en instrumentos distintos, pistas de copyright con notas dentro). G2/G3 rechazan 20 de 28 casos medidos, no todos. *Mitigación obligatoria*: el control del corte siempre disponible.
- **Archivos con más de 2 voces** (4/60 en piano-midi.de, 5/10 en Mutopia, ~7-8 % del corpus clásico) **pierden información de manos que el archivo SÍ tiene**: p. ej. `bach_846` trae «Piano right» y «Piano left» junto a cuatro pistas «Fuga 1..4». Caer al corte por altura ahí es peor de lo necesario. *Mejora posible, no probada*: asignar cada VOZ entera a una mano comparando su mediana con el corte, en vez de nota a nota — conserva la integridad de la voz y sigue obedeciendo al control del alumno.
- **El error del corte en 60 es alto en repertorio real (15-18 %)**. Si la interfaz colorea las manos, el alumno verá notas del color equivocado en cualquier pieza sin voces separadas. *Mitigación*: **etiquetar el reparto como deducido en la interfaz**, igual que la digitación se etiqueta como sugerencia por D3. No dejar que lo descubra tocando.
- **Sesgo del corpus**: es repertorio avanzado (Bach, Beethoven, Chopin). Las cifras son un techo pesimista para una app de aprendizaje: en el material de método del mismo corpus (Burgmüller, Clementi op. 36) el error en 60 baja a **7,6-13,7 %**. No citar el 18 % sin esa aclaración.
- **El loader no guarda lo que hace falta** (`loader.rs:134`, `:141`, `:76-81`). Riesgo controlado: `midi_file` 0.2.0 ya expone todo lo necesario, la puerta de `cargo tree` sigue intacta.

---

## Decisión 6: Nombres de nota

**Decisión**: **Nombre = base en nomenclatura latina con mayúscula inicial** (Do, Re, Mi, Fa, Sol, La, Si) **más alteración con los signos musicales Unicode U+266F ♯ y U+266D ♭**, nunca «#» (almohadilla) ni «b» (letra). Máximo **4 caracteres** («Sol♯»), que es la abreviatura: no hace falta ninguna otra, la etiqueta ya es corta y el resto del sitio lo ocupa el dedo sugerido que D2 pide en la misma etiqueta.

El núcleo emite un **valor simbólico `{ base, alteración }`, no una cadena**; el formateo pertenece a la capa que pinta (Principio III).

**Sostenido o bemol, desde un número de tecla**: **mapa de armaduras por tick**, fusionando todas las pistas, con la misma forma que el `TempoMap` que ya existe. Para una nota en el tick `t` se toma la última armadura con tick ≤ `t`, del meta-evento `FF 59 02 sf mi` (sf de −7 a 7):

- **sf < 0** → tabla de bemoles: `Do, Re♭, Re, Mi♭, Mi, Fa, Sol♭, Sol, La♭, La, Si♭, Si`
- **sf ≥ 0** → tabla de sostenidos: `Do, Do♯, Re, Re♯, Mi, Fa, Fa♯, Sol, Sol♯, La, La♯, Si`
- **sin armadura declarada** → sostenidos

Índice = `key % 12`. **Simplificación declarada**: una tecla blanca nunca lleva alteración — no hay Mi♯ ni Do♭.

**Octava: NO se muestra** sobre la nota que cae.

**Justificación**: en español la alteración se dice «do sostenido» / «re bemol» y se escribe Do♯ / Re♭ (Musicca en español: *«escribimos Do♯ para indicar Do sostenido»*). La Wikipedia en español advierte que *«el símbolo de sostenido (♯) puede confundirse con el signo conocido como almohadilla o numeral (#)»* — son caracteres distintos, y en una app de música usar «#» es un error tipográfico.

La decisión sostenido-o-bemol **la da el propio archivo**. Cobertura medida de `FF 59`: piano-midi.de **60/60**, ASAP score **40/40**, Mutopia **10/10**, ADL pop 63/100, grabaciones humanas 3/40. Es decir: **en el material que un alumno usa para aprender, la armadura está SIEMPRE**. Y tiene que ser **por posición, no por archivo**: cambian de armadura a mitad de pieza **24/60 en piano-midi.de (40 %)** y **12/40 en ASAP (30 %)**.

La dirección importa: en el corpus clásico **dominan los bemoles** — 29 de 60 en piano-midi.de y 23 de 40 en ASAP, frente a 21 y 10 de sostenidos. «Siempre sostenidos» daría el nombre equivocado en más de la mitad del repertorio.

La simplificación de no alterar nunca una tecla blanca es la correcta aquí: el alumno necesita saber **qué tecla pulsar**, no la ortografía de la partitura. Y no afecta al acierto, porque D9 compara números de tecla.

**Por qué no se muestra la octava.** Dos razones. Primera: es **redundante** — la nota cae sobre una tecla concreta de las 88 dibujadas, la posición horizontal YA es la octava, y ese espacio lo necesita el dedo. Segunda, y decisiva: **no hay convención hispana única**. España usa el índice franco-belga (do central = do3, la 440 = la3); el resto del mundo hispanohablante usa el científico/ISO 16 (do central = do4, igual que el C4 anglosajón). La Wikipedia en español confirma ambos usos y la conversión trivial (franco-belga = científico − 1), pero **mostrar un número obliga a elegir bando y a equivocarse con la mitad de los hispanohablantes**. Donde haya que nombrar una altura concreta —el control del punto de corte, que ES una altura— se muestra **el nombre y la posición dibujada en el teclado, nunca el número de octava**.

**Alternativas consideradas**:
- **Deducir la ortografía del contexto melódico o estimar la tonalidad (Krumhansl-Schmuckler)**: descartada. El archivo ya declara la armadura el 100 % de las veces en el material que importa; estimar añadiría coma flotante y no determinismo para resolver algo ya resuelto.
- **Elegir siempre sostenidos**: descartada. En el corpus clásico dominan los bemoles (29/60 y 23/40 frente a 21 y 10): daría el nombre equivocado en más de la mitad del repertorio.
- **Leer la armadura una sola vez al principio del archivo**: descartada. Cambia a mitad de pieza en el 40 % de piano-midi.de y el 30 % de ASAP. Hace falta el mapa por tick.
- **Mostrar la octava (Do3, Do4…)**: descartada por redundante con la posición en el teclado y por la ausencia de convención hispana única.
- **Usar «#» y «b» ASCII como forma principal**: descartada. «#» es la almohadilla y «b» se lee como letra. Se conserva **únicamente como respaldo global** si la capa de dibujo no puede garantizar la fuente, y **nunca mezclado** con los signos reales.

**Riesgos y mitigación**:
- **Riesgo de fuente — el más concreto de esta decisión.** Medido en esta máquina (macOS 25.6, fontTools 4.60.2 sobre `/System/Library/Fonts`): **Helvetica no tiene ninguno de los dos signos**; **Arial tiene ♯ pero NO ♭** (asimetría venenosa: el sostenido sale en la fuente pedida y el bemol cae a otra, con otro tamaño y otra línea base); **Helvetica Neue los tiene solo en Regular/Bold/Medium**, no en Light/Thin/Italic/Condensed; **System Font (SF Pro, lo que da `system-ui`) los tiene en redonda pero NO en cursiva**. En Windows pasa lo mismo: fileformat.info lista Segoe UI para U+266F pero **solo Segoe UI Symbol para U+266D**. *Mitigación*: la pila de fuentes debe estar **verificada para AMBOS signos**, sin cursiva y sin pesos Light. Alternativas seguras verificadas: Apple Symbols, Arial Unicode MS y Menlo tienen ♭ ♯ ♮. **Esto es riesgo para SC-003, no solo estético**: esas etiquetas se miden por fotograma y un salto de fuente cambia las métricas de texto.
- **`sf = 0` escrito por defecto sin estar en Do mayor** (36 de los 63 archivos ADL con armadura). La elección sostenido/bemol será a veces la contraria a la partitura. *Alcance del daño*: solo el nombre mostrado; nunca la tecla ni el acierto, porque D9 compara números de tecla.
- **Alteraciones accidentales de compás no cubiertas.** Un Sol♭ escrito como tal dentro de Do mayor se mostrará «Sol♯»: enarmónicamente correcto, ortográficamente no. Es consecuencia aceptada de la simplificación declarada; **dejarla escrita en el plan para que no se descubra como bug**.
- **El loader no lee armaduras hoy** (`core/src/midi/loader.rs:134` y `:141`). Hay que añadirlo junto con el nombre de pista de la Decisión 5. Sin dependencia nueva: `midi_file` 0.2.0 expone `MetaEvent::KeySignature` (`meta_event.rs:138`) y `KeyAccidentals` recortado a −7..7 con defecto 0 (`:517-556`).
