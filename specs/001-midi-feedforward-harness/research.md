# Research: Harness feedforward MIDI — decisiones técnicas del crate `core/`

**Feature**: `001-midi-feedforward-harness`
**Fecha**: 2026-08-17
**Alcance**: crate nuevo `core/` de dominio puro (sin Tauri, sin GUI, sin hardware, sin red, sin I/O).
**Toolchain verificada**: rustc 1.97.1 (8bab26f4f 2026-07-14), cargo 1.97.1 (c980f4866 2026-06-30), macOS arm64 (Darwin 25.6.0) + Windows x86_64.
**Estado del repo al investigar**: `Cargo.toml` declara workspace `resolver = "2"` con `members = ["src-tauri"]`; `Cargo.lock` no contiene ninguna crate MIDI (`midly`/`nodi`/`rimd`/`midir` sin resultados). La elección de parser estaba abierta y se cierra aquí.

**Cambio importante respecto a la investigación inicial**: la verificación adversarial **refutó** la recomendación original (midly 0.5.3). Este documento adopta la alternativa que sí resiste el escrutinio (`midi_file 0.2.0`, MIT). Ver Decisión 1.

Todas las cifras citadas provienen de mediciones reales (compilación, benchmarks, fuzzing diferencial, APIs de crates.io/GitHub) documentadas en los probes del scratchpad, no de memoria.

---

## Decisión 1: Crate de parseo MIDI

**Decision**:
Adoptar **`midi_file 0.2.0`** (licencia MIT, `github.com/webern/midi_file`, publicada 2026-07-19), **no** midly.

Declaración en `core/Cargo.toml`:

```toml
[dependencies]
midi_file = "0.2.0"   # MIT, 0 dependencias transitivas, 0 unsafe
```

Reglas de uso obligatorias:

1. **El parser vive detrás de una frontera de un solo módulo**: `core/src/midi/loader.rs` es el ÚNICO fichero de todo el crate que puede nombrar tipos de `midi_file`. Todo lo demás (`timeline.rs`, `tempo.rs`, `feedforward.rs`, `clock.rs`) trabaja sobre tipos propios. Esto convierte un cambio de parser en un trabajo de un día, no en una reescritura.
2. **La API pública del core recibe `&[u8]`**, nunca una ruta ni un `File`: `pub fn load_smf(raw: &[u8]) -> Result<Song, LoadError>`. `midi_file::MidiFile::read` exige `R: Read`, así que internamente se le pasa `std::io::Cursor::new(raw)`. Es `std::io` sobre memoria: cero acceso a disco, cero red, headless, y los fixtures de test son arrays de bytes construidos en código.
3. **Validación propia de la cabecera de 14 bytes ANTES de invocar al parser** (defensa en profundidad, independiente del parser elegido — ver código en *Riesgos y mitigación*).
4. **Parsear UNA sola vez, en carga**, para construir una línea temporal inmutable y propia (`Vec<ScheduledNote>` + `TempoMap` + `Box<[Cue]>`). El parser NUNCA se invoca en la ruta crítica de tiempo real.
5. **Pinnear en `Cargo.lock`** y añadir tests de contrato propios (ver Decisión 10) que fallen si el comportamiento del parser cambia.

**Se descarta explícitamente midly 0.5.3**, que era la recomendación de la investigación inicial. La verificación adversarial la refutó con evidencia reproducible.

**Justificacion**:

*Por qué se cae midly 0.5.3 (5 defectos, todos reproducidos, ninguno citado de segunda mano):*

1. **PANIC en `Smf::parse` con entrada malformada → viola FR-007 directamente.** `midly-0.5.3/src/primitive.rs:495` hace `let fps = -(bit_range!(raw, 8..16) as i8);`, es decir `-(-128i8)`, que desborda. Bastan **14 bytes** (`MThd`, len 6, division `0x8000`). Reproducido **con la declaración exacta que se recomendaba** (`default-features = false, features = ["std","strict"]`): ni `strict` ni `default-features=false` lo evitan. Panica en todo build de dev/test (overflow-checks activados por defecto) y en cualquier release endurecido con `overflow-checks = true`. En release por defecto devuelve `Err` **sólo porque la negación hace wraparound silencioso**: la corrección depende de un desbordamiento, no de un diseño. Es el issue **#34, abierto el 2026-07-25, con cero respuesta del maintainer**.
2. **El tempo truncado se descarta en silencio, incluso con `strict`.** El brazo es `0x51 if data.len() >= 3 => MetaMessage::Tempo(...)`; si la guarda falla, cae a `_ => MetaMessage::Unknown(...)` y el evento se tira. Verificado: tempos de 2, 1 y 0 bytes → `Ok`, sin tempo, sin error, idéntico con y sin `strict`. Consecuencia para este producto: la canción entera se reproduce a 120 bpm por defecto, TODAS las notas caen en el instante equivocado y el alumno se evalúa contra una referencia errónea, sin ningún aviso. Es el fallo más caro posible en una app de aprendizaje.
3. **`strict` no es una mejora pura: rechaza ficheros reales tocables.** El meta `0x59` (KeySignature) es el único sin guarda de longitud (issue **#32**, bug **confirmado por el maintainer** — *"Indeed this is a bug"* — con PR **#33 sin mergear desde 2026-07-10**). Medido con un fichero de 8 notas y un keysig corto: **sin `strict` se pierden 5 de 8 notas en silencio** (exactamente el fallo que `strict` supuestamente previene); **con `strict` se rechaza el fichero entero**. El bug esquiva la mitigación en las dos direcciones. Y `strict` es **compile-time** (`cfg!(feature = "strict")`, 12 usos): es todo-o-nada para el binario, no se puede ser estricto con el corpus propio y permisivo con ficheros del usuario, ni reintentar en modo laxo.
4. **`division == 0` se acepta con `strict`** y devuelve `Metrical(u15(0))`. La conversión ticks→µs que este diseño exige hace entonces **división por cero**, que panica también en release (la división entera por cero siempre panica en Rust, independientemente de `overflow-checks`).
5. **15 usos de `unsafe` sin auditar**, incluyendo `mem::transmute::<Smf<'a>, Smf<'static>>` (smf.rs:155), `unsafe impl Send for Arena`, `UnsafeCell` y casts de puntero crudo a slice; sin `forbid(unsafe_code)`; en un crate **sin commits en master desde 2024-06-15 (2,17 años)** y sin release desde **2023-01-01 (3,62 años)**. `midi_file` tiene **0 usos de `unsafe`**.

*Lo que sí se sostenía de midly, dicho con honestidad*: la API es exactamente la documentada (`Format::{SingleTrack, Parallel, Sequential}` en primitive.rs:459-461, `Smf::parse` en smf.rs:81, `midly::parse` lazy en smf.rs:262 con el comentario literal *"No allocations are made."* en smf.rs:257), soporta tipos 0/1/2 sin defectos, con `default-features=false` da 0 dependencias transitivas (7 paquetes con defaults, no 8 como se reportó), `TrackEvent` es `Copy` de 32 bytes, es determinista en 1000 parseos y es ~6,9x más rápido. **Ninguna de esas virtudes compensa un panic conocido y sin parchear dentro de la llamada que el diseño pone en la ruta de carga de ficheros no confiables.**

*Por qué midi_file 0.2.0 gana ahora:*

- **Licencia MIT**: cumple el criterio de licencia directamente, sin allow-list de `cargo-deny` ni negociación con legal. (midly es Unlicense y además **no publica fichero LICENSE**: su `include` es literalmente `["/src/*", "/Cargo.toml"]`, así que ninguna herramienta de auditoría encuentra texto de licencia.)
- **0 dependencias transitivas** (1 paquete en el árbol) y **0 `unsafe`**.
- **Robustez medida en fuzz diferencial** (semilla fija, `overflow-checks = true`): barrido exhaustivo de los 65.536 valores del campo `division` → **midly 256 panics / midi_file 0**; 200.000 mutaciones aleatorias de 1-3 bytes → **midly 6 panics / midi_file 0**. Total **262 panics en 265.536 entradas frente a 0**. Donde midly revienta, midi_file devuelve un error limpio y descriptivo: `Err(division.rs:108 The MIDI file is invalid: invalid SMPTE frame rate -128)`.
- **Rechaza `division == 0`** (midly lo acepta con `strict`).
- **Estricto por defecto sin trampas**: 83/83 ficheros truncados → `Err`, 0 panics en 830 entradas malformadas.
- **Mantenimiento activo**: publicada hace 29 días (2026-07-19) frente a 3,62 años de midly.
- **Resuelve running status y note-on velocity 0** (se entregan tal cual, sin convertir, que es lo correcto): salida real verificada sobre un SMF tipo 1 construido a mano — `Header { format: Multi, division: QuarterNote(QuarterNoteDivision(480)) }`, `Meta(SetTempo(MicrosecondsPerQuarter(500000)))`, `Midi(NoteOn(NoteMessage { channel: Channel(0), note_number: NoteNumber(60), velocity: Velocity(0) }))`.
- **Su única pega medida, ser ~6,9x más lento, es irrelevante aquí**: 22,769 ms para un fichero patológico de 1,56 MB / 200.000 notas (midly: 3,322 ms). Extrapolando ese factor, una canción realista de 4.000 notas / 32,9 KB cuesta ~0,5 ms. El parseo ocurre **una vez en carga**, dentro del presupuesto de 100 ms de SC-001, y **fuera** del presupuesto de <30 ms p95 de la ruta crítica.

**Alternativas consideradas**:

- **midly 0.5.3** — *Refutada* por los 5 defectos anteriores (panic #34 abierto, tempo truncado silencioso, `strict` que rompe ficheros válidos vía #32, `division==0` aceptado, `unsafe` sin auditar en crate sin mantenimiento). Se conserva **sólo como plan B endurecido** si midi_file falla los tests de contrato (ver *Riesgos y mitigación*).
- **nodi 1.0.3** (MIT) — No es un parser sino una capa de reproducción **construida sobre midly** (`cargo tree`: nodi → midly → rayon → crossbeam). Duplica lo que vamos a construir, hereda el panic de midly, y **se apropia del reloj** (feature por defecto `hybrid-sleep`, o sea dormir/bloquear), lo que choca de frente con el reloj inyectable y con la prohibición de bloquear en la ruta crítica.
- **ghakuf 0.5.6** (MIT/Apache-2.0) — Licencia ideal, pero muerto desde 2020-10-04. API event-driven por callbacks con estado mutable: modelo que complica el determinismo y encaja mal con construir una línea temporal inmutable.
- **rimd 0.0.1** — Abandonado (última actualización 2017-11-30, casi 9 años). Descartable de entrada para una app comercial.
- **augmented-midi 1.8.0** (MIT) — El propio autor lo describe como *"Experimental MIDI file/event parser using nom combinators"*. Subcrate de un monorepo enorme pensado para consumo interno, arrastra `nom`. "Experimental" es incompatible con la base de todo el producto.
- **lumino-midly 0.6.3** — **Trampa a evitar activamente.** Aparece por encima en `cargo search` por número de versión, pero es un fork de vanidad creado el 2026-07-22 con 813 descargas totales y sin historial, que añade features alarmantes para un dominio puro (jemalloc, memmap, sysinfo, memory-report) y hereda el mismo Unlicense y los mismos bugs.
- **Escribir nuestro propio parser SMF** — El formato está congelado desde 1996 y la superficie que usamos es pequeña, pero es trabajo que no aporta valor de producto en la primera entrega y multiplica la superficie de bugs de parseo. Se reevalúa sólo si el plan B también falla.

**Riesgos y mitigacion**:

- **midi_file es pre-1.0 con rotación de API reciente** (0.0.6 → 0.1.0 → 0.2.0 en 2026) y sólo 12.577 descargas totales frente a las 371.853 de midly: mucho menos rodado en producción. *Mitigación*: `midi_file = "0.2.0"` (que en semver pre-1.0 sólo admite 0.2.x, nunca 0.3), versión exacta congelada en `Cargo.lock` comiteado, frontera de un solo módulo (`core/src/midi/loader.rs`) y tests de contrato propios que fijan el comportamiento observado.
- **La API de iteración de pistas/deltas de midi_file no está verificada al 100 %.** Se verificó `MidiFile::read`, el `Header` (`format`, `division` → `QuarterNote(QuarterNoteDivision(480))`), los eventos `Meta(SetTempo(...))` y `Midi(NoteOn(...))` con running status resuelto. **No** se dejó constancia escrita de los accesores exactos para iterar pistas y leer delta-times. *Mitigación*: **la tarea T0 del TDD, antes de escribir cualquier otra cosa**, es un test de contrato que carga un SMF tipo 1 de bytes crudos y asserta pista por pista los ticks absolutos. Si esa API no expone deltas o índice de pista, se activa el plan B en el acto.
- **`R: Read` mete `std::io` en un crate de dominio puro.** *Mitigación*: `std::io::Cursor<&[u8]>`, memoria pura. La firma pública del core sigue siendo `&[u8]`; la lectura de fichero vive en la capa Tauri. El core sigue siendo headless y testeable sin disco.
- **Modelo owned/allocating, sin zero-copy.** *Mitigación*: irrelevante por diseño — se parsea una vez en carga y se construye una línea temporal propia. La ruta crítica sólo lee un slice preordenado.
- **PLAN B documentado (midly endurecido).** Si midi_file falla el contrato: `midly = { version = "0.5.3", default-features = false, features = ["std", "strict"] }` **más, obligatoriamente**: (a) la pre-validación de cabecera de abajo, que neutraliza los dos panics conocidos (`0x80` en el byte alto de division, y `division == 0`); (b) validación explícita de la longitud de los metas `0x51` (tempo, 3 bytes), `0x58` (compás, 4) y `0x59` (armadura, 2) recorriendo los eventos y devolviendo `Err` en vez de aceptar el descarte silencioso; (c) tests de contrato para los 4 casos refutados. Sin esas tres cosas, midly no es adoptable.
- **Pre-validación de cabecera, obligatoria sea cual sea el parser** (14 bytes, coste despreciable, elimina dos clases enteras de fallo):

```rust
const MAX_PPQ: u16 = 32_767;

fn validar_cabecera(raw: &[u8]) -> Result<u16 /*ppq*/, LoadError> {
    if raw.len() < 14 { return Err(LoadError::CabeceraTruncada); }
    if &raw[0..4] != b"MThd" { return Err(LoadError::MagicInvalido); }
    let len = u32::from_be_bytes([raw[4], raw[5], raw[6], raw[7]]);
    if len < 6 { return Err(LoadError::CabeceraInvalida { len }); }
    let format = u16::from_be_bytes([raw[8], raw[9]]);
    match format {
        0 | 1 => {}
        2 => return Err(LoadError::FormatoNoSoportado { format: 2 }),
        f => return Err(LoadError::FormatoInvalido { format: f }),
    }
    let division = u16::from_be_bytes([raw[12], raw[13]]);
    if division & 0x8000 != 0 { return Err(LoadError::TimingSmpteNoSoportado); } // mata el panic #34
    if division == 0 { return Err(LoadError::DivisionCero); }                    // mata la div-by-zero
    let ppq = division & 0x7FFF;                                                 // 1..=32767
    debug_assert!(ppq >= 1 && ppq <= MAX_PPQ);
    Ok(ppq)
}
```

---

## Decisión 2: Representación del tiempo y conversión tick ↔ microsegundo

**Decision**:
Cada nota lleva su tiempo **dos veces**: tiempo musical en **ticks (`u64`)** y tiempo real en **microsegundos (`u64`)**. **Prohibido `f32`/`f64` en todo el crate `core/`.** La conversión se hace con aritmética entera exacta y **una única división truncada al final**, sobre un mapa de tempo precalculado con anclas exactas en unidades de "microsegundos × PPQ".

Newtypes obligatorios (`#[repr(transparent)]`, coste cero):

```rust
#[repr(transparent)] #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
pub struct Ticks(pub u64);
#[repr(transparent)] #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
pub struct Micros(pub u64);
// SIN `impl Sub` (sólo saturating_sub / checked_sub) y SIN `From<Ticks> for Micros`:
// sólo el TempoMap sabe convertir.
```

**Validación de carga que sostiene toda la prueba de no-overflow**: al acumular deltas a tick absoluto se usa `checked_add`, y se rechaza el fichero si `tick_abs > u32::MAX as u64` (`LoadError::TickOverflow`). Ese techo (4.294.967.295 ticks ≈ 621 horas a PPQ 960 y 120 bpm) es holgadísimo y es lo que garantiza que ningún producto desborde.

**Fórmulas canónicas (implementables tal cual):**

```rust
// ---------- CONSTRUCCIÓN (una sola vez, en carga) ----------
// segments ya normalizados: start_tick[0] == 0, estrictamente creciente, us_per_qn >= 1
anchor_scaled[0] = 0u64;
start_us[0]      = 0u64;
for k in 0..n-1 {
    let dt = segments[k+1].start_tick - segments[k].start_tick;      // u64, > 0
    anchor_scaled[k+1] = anchor_scaled[k] + dt * segments[k].us_per_qn as u64;
    start_us[k+1]      = anchor_scaled[k+1] / ppq as u64;            // precalculado, para la inversa
}

// ---------- CONSULTA tick -> microsegundos ----------
let k = segments.partition_point(|s| s.start_tick <= tick) - 1;
let scaled = segments[k].anchor_scaled
           + (tick - segments[k].start_tick) * segments[k].us_per_qn as u64;
let us = scaled / ppq as u64;            // ÚNICA división, truncado (floor)

// ---------- CONSULTA microsegundos -> tick (mayor tick con tiempo exacto <= us) ----------
// NO está en la ruta crítica (sólo seek). u128 para blindar el producto us*ppq.
let k = segments.partition_point(|s| s.start_us <= us) - 1;
let num = (us as u128) * (ppq as u128) - segments[k].anchor_scaled as u128;
let tick = segments[k].start_tick + (num / segments[k].us_per_qn as u128) as u64;
```

**Política de redondeo: truncado (floor). Es parte del contrato público, se documenta y se testea.** Si en algún punto hace falta exactitud sub-microsegundo (comparar dos instantes que difieren en menos de 1 µs), se compara el campo `scaled` **sin dividir**.

**Justificacion**:

*La base normativa.* La cabecera MTHd define `division` (u16): si el bit 15 es 0, los bits 14..0 son PPQ, ticks por negra, rango 1..=32767 — *"If bit 15 of \<division\> is zero, the bits 14 thru 0 represent the number of delta time 'ticks' which make up a quarter-note"*. El Set Tempo es `FF 51 03 tt tt tt`, **microsegundos por negra**, campo de 24 bits, rango 1..=16.777.215. Por defecto, si el archivo no declara tempo: 500.000 µs/negra = 120 bpm — *"If they don't, the time signature is assumed to be 4/4, and the tempo 120 beats per minute"* (FR-005).

La relación básica es `us_por_tick = us_por_negra / PPQ`, **pero ese cociente no debe materializarse jamás como valor intermedio**: es exactamente donde se pierde la exactitud (500000/960 = 520,8333… no es representable ni en binario ni en entero). Por eso todo el numerador se acumula en enteros y la división se aplica una sola vez.

*Por qué NO coma flotante, con datos medidos* (simulación PPQ=960, 300 tramos = un cambio de tempo por compás, 12.000 eventos, ~10 min, tempos no divisibles por PPQ):

| Método | Error máximo | Consecuencia |
|---|---|---|
| Algoritmo entero (el adoptado) | **0,9989 µs** | Sólo el truncado final. Acotado por 1 µs y **NO acumulativo**. 0,003 % del presupuesto de 30 ms. |
| Acumulación incremental f64 | 2,1e-06 µs | Ridículo… pero **1241 de 12000 eventos (10,3 %) caen en un microsegundo entero DISTINTO** al exacto. |
| Acumulación en f32 (segundos) | **1688 µs = 1,69 ms** | **5,6 % del presupuesto de 30 ms consumido gratis.** A los 10 min el ulp de f32 ya vale 32 µs. |

El argumento decisivo no es el error, es el **determinismo**: la misma suma en orden secuencial da `612380625.1208314` y en orden pairwise `612380625.1208334` (bits distintos). Cualquier refactor que cambie el orden de la fusión, paralelice o permita que LLVM vectorice la reducción de otra forma cambia la salida sin que nadie toque la lógica — y compilamos para macOS arm64 y Windows x86_64. Peor aún: **una diferencia de 1 ulp puede invertir el orden de dos eventos** comparados por tiempo. Eso ya no es imprecisión, es comportamiento distinto y test intermitente, prohibido por la Constitución. Y con acumulación, el instante de la nota 5000 depende de las 4999 conversiones anteriores: hacer *seek* al minuto 5 daría un valor distinto que reproducir desde el principio. Con anclas por tramo, **cada nota depende SÓLO de su ancla**: local, testeable y con seek exacto.

*Prueba de no-overflow.* Por el estándar, cada delta-time es un VLQ de como mucho 4 bytes — *"The largest number which is allowed is 0FFFFFFF so that the variable-length representations must fit in 32 bits"*. Validando en carga que el tick absoluto cabe en u32, y siendo el tempo un campo de 24 bits, el peor producto posible es (2³²−1)·(2²⁴−1) = **72.057.589.726.183.425 = 2^55,99999 < 2⁶³**. Salida literal de la verificación: `worst-case scaled product: 72057589726183425 < u64 max: True log2 = 55.99999991367277`. **No hay ninguna combinación legal de archivo que desborde.** En el caso real pedido (PPQ=960, 10 minutos, tempo cambiando cada compás) el acumulador máximo medido es 5,88e11 = 2^39,1: quedan 24 bits de margen.

*Monotonía verificada*: sobre 12.000 eventos la secuencia `us(t)` resultó **estrictamente creciente**; `inverse property violations: 0` sobre 42.000 sondas.

**Alternativas consideradas**:

- **f64 en segundos/ms acumulado incrementalmente** (lo que hacen mido, la mayoría de tutoriales y muchas libs) — Descartada: rompe el determinismo bit a bit. 10,3 % de los eventos en un microsegundo entero distinto; suma secuencial ≠ suma pairwise ya en bits.
- **f32 en segundos** — Descartada: 1,69 ms de deriva medida en una pieza de 10 minutos.
- **Racionales genéricos (`num-rational::BigRational` / `Ratio<i64>`)** — Exactitud innecesaria a cambio de `gcd` en cada operación, denominadores que crecen y asignaciones en el caso Big. Incompatible con la regla de no asignar en la ruta crítica. El esquema de anclas ya da exactitud completa con un `u64` y una división.
- **Nanosegundos en vez de microsegundos** — Técnicamente válido (584 años de rango en u64) y reduciría el truncado a <1 ns, pero el microsegundo es la unidad nativa del estándar (Set Tempo se define en µs/negra) y de las APIs de reloj. Con el ancla exacta el error ya es <1 µs y no acumulativo; los 3 órdenes de magnitud extra no compran nada y añaden conversiones en la frontera. Si algún día hace falta exactitud sub-µs, se compara `scaled` sin dividir.
- **Punto fijo con escala potencia de dos (`us << 16`)** — No aporta exactitud porque PPQ casi nunca es potencia de dos (960 = 2⁶·15, 480, 384). Seguiría habiendo redondeo por tramo; el denominador implícito PPQ hace la representación exacta por construcción.

**Riesgos y mitigacion**:

- **Confundir "negra" con "pulso del compás"**: Set Tempo son microsegundos por **NEGRA**, no por beat del compás. El Time Signature **NO interviene** en la conversión tick→µs; en 6/8 la fórmula es idéntica. Es el error más común en implementaciones caseras. *Mitigación*: test explícito con un fichero en 6/8.
- **Volver a meter float por la puerta de atrás en la frontera Tauri**: convertir a f64 sólo para pintar, y **nunca** reintroducir ese valor en el core. Toda serialización de tiempos hacia el frontend lleva el `u64` de microsegundos. *Mitigación*: lint + revisión de la frontera; el core no depende de serde para tiempos derivados.
- **Casts `as` truncantes entre u32/u64/u128 sin comprobar**: en release un `as` que trunca no avisa. *Mitigación*: `From`/`TryFrom` y `checked_*`; prohibir `as` en revisión de código para conversiones que puedan perder bits.
- **La inversa `us * ppq` puede desbordar u64 en un caso patológico** (duración enorme × PPQ 32767). *Mitigación*: la fórmula de arriba ya usa `u128` en la inversa (no está en la ruta crítica), y además se acota la duración total en carga a 24 h (8,64e10 µs).
- **Tests que esconden el problema**: con PPQ=960 y tempo 500000 casi todas las cuentas salen exactas y un algoritmo incorrecto pasa igual. *Mitigación*: los golden tests **deben** incluir tempos NO divisibles por PPQ (p. ej. 461538, 435897) y subdivisiones no triviales (tresillos, offsets primos).
- **Escalado de tempo de práctica (tocar al 70 %)**: no multiplicar microsegundos por un float en tiempo real. *Mitigación*: aplicarlo como racional exacto sobre `us_per_qn` (`us_per_qn * den / num`, una sola división, u64) y **reconstruir la línea temporal en carga**. Si no, la deriva vuelve por la puerta trasera.
- **Cambiar la política de redondeo más adelante** (floor → round-half-up) movería todos los golden files 1 µs y parecería una regresión. *Mitigación*: documentarla como contrato público y tener un test que la fije.

---

## Decisión 3: Estructura del mapa de tempo y coste de consulta

**Decision**:

```rust
pub struct TempoMap {
    ppq: u32,                       // 1..=32767, validado en carga
    segments: Vec<TempoSegment>,    // Vec con capacidad exacta, construido una vez
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TempoSegment {
    pub start_tick: u64,     // tick absoluto de inicio del tramo (<= u32::MAX por invariante)
    pub anchor_scaled: u64,  // microsegundos x PPQ acumulados hasta start_tick (exacto)
    pub start_us: u64,       // anchor_scaled / ppq, precalculado para la búsqueda inversa
    pub us_per_qn: u32,      // 1..=16_777_215
}
```

**Invariantes** (con `debug_assert!` en el constructor y un test de propiedad dedicado):

1. `segments` no vacío y `segments[0].start_tick == 0`.
2. `start_tick` **estrictamente** creciente (nunca tramos de longitud cero).
3. `us_per_qn >= 1`.
4. De (2) y (3) se deduce que `start_us` es estrictamente creciente — **es lo que legitima la búsqueda binaria en la dirección inversa**.

**Coste de consulta**:

- **Acceso aleatorio (seek, saltar a un compás)**: `partition_point` → **O(log n)**, sin asignaciones ni panics. Con n = 300 tramos (10 min, un cambio por compás) el array ocupa ~9,6 KB, entra en L1 y son **9 comparaciones**; con 10.000 tramos, 14.
- **Conversión masiva en carga**: las notas se recorren en orden de tick creciente, así que se usa un **cursor que avanza junto al índice de tramo** → **O(n_notas + n_tramos)** total, **O(1) amortizado por nota**.
- **Ruta crítica de reproducción: NO se convierte NADA.** En carga se precalculan `onset_us`, `end_us` y `cue_us` de cada nota. Reproducir es avanzar un índice sobre un array ordenado comparando `u64`. **Cero divisiones, cero asignaciones, cero I/O, cero bloqueos.**

**Construcción — casos límite normativos**:

| Caso | Política |
|---|---|
| (a) Varios Set Tempo en el mismo tick | Gana **el último** según el orden de fusión determinista `(tick_abs, track_idx, event_idx)`. Se colapsan **antes** de construir los tramos. Crítico: no dejar nunca tramos de longitud cero. |
| (b) Tempo antes de la primera nota | Caso normal, ningún tratamiento especial; define el tramo inicial. |
| (c) Primer tempo en tick > 0, o ningún tempo | Se antepone un tramo sintético `{ start_tick: 0, us_per_qn: 500_000 }` (120 bpm, default del estándar). Cubre **FR-005**. |
| (d) `us_per_qn == 0` | **Rechazar** con `LoadError::InvalidTempo { tick, us_per_qn: 0 }`. Es división por cero y rompe la monotonía de la que depende toda la prueba de orden de los cues. Un clamp silencioso entregaría una lección falsa (contra FR-007). |
| (e) Tempos absurdos pero legales (1..=16.777.215) | **Aceptar todo el rango legal**: 16.777.215 µs/negra = 3,576 bpm; 1 µs/negra = 60.000.000 bpm. Ambos extremos son aritméticamente seguros. Opcionalmente, aviso **no bloqueante** fuera de 10..400 bpm que **no altera la salida**. |
| (f) `division` con bit 15 = 1 (SMPTE) | Fuera de alcance → `LoadError::TimingSmpteNoSoportado`. Cuando se soporte: `us_por_tick = 1_000_000 / (fps * subframes)`, constante y sin mapa de tempo (ojo: "29" significa 29,97 drop-frame). |
| (g) `division == 0` | Inválido → `LoadError::DivisionCero`. |
| (h) Suma de deltas que desborda el techo de ticks | `checked_add` + techo `u32::MAX` → `LoadError::TickOverflow`. **Nunca envolver.** |
| (i) Set Tempo con longitud declarada ≠ 3 | Meta malformado → **rechazar** (FR-007). No aceptar el descarte silencioso. |
| (j) Set Tempo posterior a la última nota | Se conserva; afecta al final de la pieza y a la cola de cues. Inofensivo. |

**Fuente de los Set Tempo**: se construye el mapa con **TODOS** los Set Tempo de la secuencia fusionada, **no sólo los de la pista 0**.

**Justificacion**:

*Por qué anclas por tramo y no acumulación*: el ancla del tramo k se obtiene con sumas y multiplicaciones enteras exactas a partir del ancla anterior, en carga; en consulta sólo quedan una resta, una multiplicación, una suma y una división. Cada nota depende **sólo** de su ancla, no de los eventos intermedios: eso da seek exacto y localidad para los tests.

*Por qué aceptar tempos de cualquier pista*: el estándar dice *"in format 1 file, the tempo map must be stored as the first track"* y, para los meta-eventos concretos, *"In format 1, these meta-events **should** be contained in the first track"* — **should, no must**. Hay exportadores reales que los ponen en pistas posteriores. Con la lectura estricta esos archivos se reproducirían a **120 bpm constante sin ningún error visible**, que es el peor fallo posible: silencioso y musicalmente erróneo. Aceptar tempos de cualquier pista es un **superconjunto compatible**: si el archivo cumple el estándar, el resultado es idéntico. En formato 0 el tempo está mezclado con las notas por definición — *"for a format 0 file, the tempo will be scattered through the track and the tempo map reader should ignore the intervening events"*.

*Ejemplo numérico completo, verificado a mano y por script* (PPQ = 960; eventos de tempo tras la fusión: (0, 500000), (3840, 400000), (7680, 300000), (7680, 461538) — los dos últimos comparten tick):

```
Paso 1 — colapso del mismo tick: gana el último → se descarta 300000, queda (7680, 461538).
Paso 2 — tramos:
  k=0: start_tick=0     anchor_scaled=0             us_per_qn=500000  start_us=0
  k=1: start_tick=3840  anchor_scaled=1_920_000_000 us_per_qn=400000  start_us=2_000_000
       (comprobación: un compás 4/4 a 120 bpm = 2,000 s ✓)
  k=2: start_tick=7680  anchor_scaled=3_456_000_000 us_per_qn=461538  start_us=3_600_000
       (comprobación: +1,6 s = 4 negras × 0,4 s a 150 bpm ✓)
Paso 3 — nota con onset en tick 9000:
  k = partition_point(start_tick <= 9000) - 1 = 2
  delta  = 9000 - 7680 = 1320
  1320 * 461538 = 609_230_160
  scaled = 3_456_000_000 + 609_230_160 = 4_065_230_160
  us     = 4_065_230_160 / 960 = 4_234_614   (resto 720; exacto 4_234_614,75 µs)
  Verificación cruzada: 4_234_614 * 960 + 720 = 4_065_230_160 ✓
```

**Alternativas consideradas**:

- **Recorrer el mapa linealmente en cada consulta (O(n))** — Con ~300 tramos y ~1000 notas son 300.000 iteraciones evitables. `partition_point` da 9 comparaciones y el barrido con cursor en carga da O(1) amortizado.
- **Leer los Set Tempo únicamente de la pista 0 (lectura estricta)** — Descartada: produce el fallo silencioso descrito arriba con exportadores reales.
- **Estructura duplicada (un array ordenado por tick y otro por µs)** — Innecesaria: la invariante 4 hace que el **mismo** array sirva para las dos direcciones.
- **Recalcular el mapa en cada consulta de reproducción** — Descartada de plano: mete divisiones en la ruta crítica de <30 ms p95.

**Riesgos y mitigacion**:

- **Tramos de longitud cero** cuando hay dos Set Tempo en el mismo tick: rompen la invariante 2 y con ella la corrección de `partition_point` en ambas direcciones. *Mitigación*: colapsar **antes** de construir el array + `debug_assert!` de la invariante + test de propiedad.
- **Tratar una división SMPTE como si fuera PPQ**: el valor se lee como un número enorme y toda la pieza sale con tiempos disparatados. *Mitigación*: comprobar el bit 15 en `validar_cabecera` antes de nada.
- **Tempos ultrarrápidos colapsan varias notas en el mismo microsegundo**. *Mitigación*: el desempate es siempre la clave estable (ver Decisión 7), nunca el orden de iteración de un `HashMap`.
- **Crecimiento patológico del número de tramos** (un fichero adversario con un Set Tempo por tick): *Mitigación*: el coste sigue siendo O(log n) y la construcción O(n); si se quisiera acotar memoria, tope configurable con error tipado. **Queda como decisión abierta menor**: por ahora sin tope, con la validación de tamaño de fichero en la capa de aplicación.

---

## Decisión 4: Patrón de reloj inyectable

**Decision**:
**Sacar el reloj de la ruta crítica** y usar **genérico `C: Clock` en el borde**, no `dyn`, no feature flag.

Tres capas, no una:

```rust
// CAPA 1 — PURA. Sin reloj. Aquí vive el 100 % de la lógica y el 100 % de las pruebas.
impl Playback {
    pub fn advance_to(&mut self, now: Micros) -> Result<&[Cue], Rewind>;
    pub fn seek(&mut self, to: Micros);
}

// CAPA 2 — el ÚNICO tipo genérico del crate. Lee el reloj UNA vez y delega.
pub struct Session<C: Clock> { clock: C, playback: Playback }
impl<C: Clock> Session<C> {
    #[inline] pub fn poll(&mut self) -> Result<&[Cue], Rewind> {
        let now = self.clock.now();     // UNA sola lectura por pasada
        self.playback.advance_to(now)
    }
}

// CAPA 3 — `dyn Clock` sólo si algún día la capa Tauri necesita elegir reloj en runtime.
//          Hoy NO se implementa: existe exactamente un reloj real.
```

`timeline.rs`, `tempo.rs`, `midi/loader.rs` y `feedforward.rs` quedan **libres de parámetros de tipo**.

**Contrato del trait** (la parte más importante del documento):

```rust
pub trait Clock {
    /// Devuelve el instante actual en microsegundos desde el inicio de la sesión.
    /// CONTRATO: la secuencia de valores devueltos es NO DECRECIENTE.
    /// NO es estrictamente creciente: dos llamadas consecutivas PUEDEN devolver el mismo valor.
    fn now(&self) -> Micros;
}
```

**Reloj virtual (tests, benchmarks y futura "reproducir sesión grabada")** — API pública sin `cfg`:

```rust
pub struct VirtualClock { t: Cell<u64> }
impl VirtualClock {
    pub fn at_zero() -> Self;
    pub fn advance(&self, d: Micros);   // única vía; monótono POR CONSTRUCCIÓN
    pub fn jump_to(&self, t: Micros);   // panic si t < now: retroceder es un bug de la prueba
}
impl Clock for VirtualClock { #[inline] fn now(&self) -> Micros { Micros(self.t.get()) } }
```

`Cell<u64>`, **no** `Atomic`: el núcleo es monohilo y determinista; `Atomic` invitaría a compartirlo entre hilos y a reintroducir no determinismo por interleaving.

**Reloj real**:

```rust
pub struct MonotonicClock { origin: Instant, offset: Micros }
impl Clock for MonotonicClock {
    #[inline] fn now(&self) -> Micros {
        Micros(self.offset.0 + self.origin.elapsed().as_micros() as u64)
    }
}
impl MonotonicClock { pub fn rebase_to(&mut self, playhead: Micros); }
```

`Instant`, **JAMÁS `SystemTime`** (no monótono, salta con NTP). Relativo al inicio de la reproducción, no absoluto: el playhead arranca exactamente en 0.

**Cómo se prueba**: la mayoría de los tests **ni siquiera tocan el reloj** — llaman a `Playback::advance_to(Micros(x))` con timestamps literales. `VirtualClock` se usa sólo en los tests que ejercitan `Session` y en la prueba de intercambiabilidad de relojes.

**Justificacion**:

*El dato honesto sobre `dyn`*: en un microbenchmark aislado (2.000.000 de lecturas por corrida, mejor de 40, release + LTO fat + codegen-units=1) contra el reloj **real**: genérico **22,9286 ns/lectura** vs `&dyn` **23,1942 ns**. El sobrecoste de la vtable es **+0,2656 ns**, el 1,16 % de la lectura y el **0,000007 % del presupuesto de 30 ms**. En el benchmark integrado (36.000 avances sobre 10.000 cues) las variantes A-D quedaron dentro del ruido, y `Box<dyn>` llegó a medir *más rápido* que el genérico (2,010 vs 2,833 ns) por puro efecto de alineación de código. **`dyn` no se descarta por lento: afirmarlo sería deshonesto.**

Las razones reales del genérico son tres, y una sí es medible: con el reloj **virtual**, el bucle genérico se optimiza a **0,0000 ns/lectura** (LLVM lo elimina entero por inlining y plegado de constantes) frente a **1,9892 ns** con `&dyn`, que bloquea el inlining. Importa poco en producción y **muchísimo en las pruebas**, donde 10.000 casos aleatorios tienen que caber en el presupuesto de 1 s de SC-002. Segunda: `Box<dyn Clock>` es una asignación en el heap a cambio de nada. Tercera: la object safety cierra la puerta a tipos asociados y obliga a arrastrar `Box<dyn Clock + Send + Sync>`.

**El argumento decisivo es que la pregunta está mal planteada**: si el reloj se lee UNA vez por fotograma (60 Hz) en lugar de una vez por nota, la elección de despacho ocurre 60 veces por segundo, no 10.000. Sacar el reloj del scheduler vuelve el debate irrelevante y hace que el 100 % de la lógica se pruebe sin ninguna abstracción de reloj: se le pasa un `u64`.

*Por qué NO `Instant` en tests*: (a) obliga a `sleep`, y una canción de 10 min no se puede probar (SC-002 exige <1 s); (b) **el 24,06 % de las lecturas consecutivas de `Instant` en macOS arm64 devuelven el MISMO valor** (721.850 de 3.000.000), con delta mínimo no nulo de 41 ns — no se puede controlar "avanza un poco"; (c) la granularidad difiere entre macOS (`CLOCK_UPTIME_RAW`) y Windows (QPC): una prueba pasa en tu Mac y falla en el CI de Windows; (d) el timer por defecto de Windows es de 15,6 ms, o sea que `sleep(10ms)` duerme 15,6 ms; (e) rompe FR-018 (idénticos bit a bit).

**Alternativas consideradas**:

- **`Box<dyn Clock>` como campo de `Session`** — Descartada, **pero no por lenta** (+0,27 ns sobre 22,93 ns). Se descarta porque es una asignación en el heap a cambio de nada (hoy sólo existe un reloj real, no hay elección en runtime que justificar), bloquea el inlining (0,0000 vs 1,9892 ns con reloj virtual, y eso sí pesa en 10.000 iteraciones de property tests) y la object safety obliga a `Box<dyn Clock + Send + Sync>`. **Se mantiene disponible como escape hatch en la capa Tauri.**
- **Feature flag / `cfg(test)` para conmutar reloj** — Descartada por un **bug concreto y verificable**: el plan pone las pruebas en `core/tests/`, y los tests de integración enlazan contra la librería compilada **sin `cfg(test)`**. Un `#[cfg(test)] type TheClock = VirtualClock` sencillamente **no compilaría** para `feedforward_test.rs`. La variante `#[cfg(feature = "test-clock")]` sí compila, pero entonces el binario de release y el de pruebas tienen grafos de tipos distintos (dejas de probar lo que envías), impide tener ambos relojes vivos a la vez y se vuelve frágil con la unificación de features del workspace cuando `src-tauri` dependa de `core`.
- **API `advance_by(delta: Duration)`** — Es monótona por construcción, lo cual suena mejor, pero **pierde exactamente lo que exige FR-020**: no puedes DETECTAR que el SO retrocedió, porque el retroceso se satura a cero en silencio (que es justo lo que hace `Instant::duration_since` y justo lo que la spec prohíbe). Se conserva la idea sólo en el reloj virtual, donde `advance(Micros)` sí es la única vía.
- **Wrapper `MonotonicGuard` con `max(prev, now)` dentro del núcleo** — Descartada: enmascara un bug real (QPC roto en una VM, suspensión del sistema) convirtiéndolo en "el tiempo se congeló", invisible en logs y en pruebas. **Clampar es política de la capa app**: el núcleo informa, la app decide.

**Riesgos y mitigacion**:

- **`now == last` NO es retroceso.** El 24,06 % de las lecturas consecutivas de `Instant` en macOS arm64 devuelven el mismo valor. Si la regla anti-retroceso fuese `now > last`, el reloj real dispararía `Rewind` **una de cada cuatro llamadas**. *Mitigación*: la regla es `now < last → Err`, `now == last → Ok(&[])`, documentada en el trait y con test dedicado.
- **Divergencia macOS/Windows en suspensión del sistema.** La doc de std dice literalmente que si las suspensiones cuentan como tiempo transcurrido *"is also not specified, and the behavior varies across platforms and Rust versions"*. En la práctica Darwin usa `CLOCK_UPTIME_RAW` (el reloj **se para** al dormir) y Windows usa QPC (el reloj **sigue contando** en standby/hibernación) — confirmado empíricamente en rust-lang/rust#79462. Si el usuario cierra la tapa a mitad de canción, en macOS el playhead sigue donde estaba y en Windows salta al final. *Mitigación*: tratarlo explícitamente con `rebase_to()` en la capa app, nunca dejarlo al azar.
- **`Instant::duration_since`/`elapsed`/`sub` SATURAN A CERO** al violarse la monotonía (en versiones antiguas hacían panic): un salto de QPC hacia atrás se vuelve "el tiempo se congeló", invisible. *Mitigación*: usar `checked_duration_since` (devuelve `None`) en el borde si se quiere detectarlo; nunca depender de que `elapsed()` crezca entre dos llamadas.
- **Resolución del temporizador de Windows: 15,6 ms por defecto.** Con un presupuesto de 30 ms, un solo `thread::sleep` se come la mitad. *Mitigación*: **no marcar el ritmo del bucle de feedforward con `sleep`**; engancharlo al callback de render o de audio. `timeBeginPeriod(1)` es asunto de la capa Tauri, no del núcleo.
- **Leer el reloj más de una vez por pasada** (el scheduler por su cuenta y el emisor de eventos por la suya) introduce no determinismo puro: verían timestamps distintos dentro de la misma pasada. Además cuesta 22,9-28,2 ns por lectura, ~10x lo que cuesta el avance completo del scheduler. *Mitigación*: una sola lectura en `Session::poll()`, propagada hacia abajo. Regla de revisión de código.

---

## Decisión 5: Estructura del scheduler de cues e invariante de coste

**Decision**:
**Cursor sobre un array inmutable y preordenado.** No cola de prioridad, no búsqueda binaria por avance.

```rust
pub struct CueSchedule { cues: Box<[Cue]> }              // inmutable, compartible por Arc
pub struct Playback { schedule: Arc<CueSchedule>, next: usize, last: Micros }

#[derive(Clone, Copy, Debug)]     // size_of::<Cue>() == 32 bytes
pub struct Cue {
    pub cue_at: Micros,      // instante de aviso (µs)
    pub onset_at: Micros,    // instante en que hay que pulsar (µs)
    pub cue_tick: u64,       // tiempo musical del aviso (desempate y depuración)
    pub note_index: u32,     // índice en la línea temporal ya ordenada
}

#[inline]
pub fn advance_to(&mut self, now: Micros) -> Result<&[Cue], Rewind> {
    if now < self.last { return Err(Rewind { last: self.last, requested: now }); }
    self.last = now;
    let start = self.next;
    let cues = &self.schedule.cues;
    let mut i = start;
    while i < cues.len() && cues[i].cue_at <= now { i += 1; }
    self.next = i;
    Ok(&cues[start..i])          // SUBSLICE: cero copias, cero asignaciones
}

/// ÚNICO camino legal hacia atrás.
pub fn seek(&mut self, to: Micros) {
    self.next = self.schedule.cues.partition_point(|c| c.cue_at <= to);
    self.last = to;
}
```

**INVARIANTE EXACTA** (se mantiene en todo momento, con `debug_assert!`):

1. `cues` está ordenado de forma **no decreciente** por `cue_at`.
2. `∀ i < next: cues[i].cue_at <= last`
3. `∀ i >= next: cues[i].cue_at > last`
4. `next` es no decreciente entre llamadas a `advance_to` (sólo `seek` lo mueve hacia atrás).
5. `last` es no decreciente entre llamadas a `advance_to`.

**INVARIANTE DE COSTE (SC-006)**: de (1)+(3) se deriva que `advance_to` hace **exactamente `k+1` comparaciones**, donde `k` = número de cues **emitidos** en esa llamada. **Independiente de `cues.len()`.** De (2)+(4) se deriva **FR-012** (cada cue se emite exactamente una vez).

**Acordes (FR-013)**: los cues con el mismo `cue_at` salen **contiguos** en el slice; el consumidor agrupa por `cue_at`. **Nunca** devolver `Vec<Vec<Cue>>`.

**FR-014**: `cue.remaining_at(now) = cue.onset_at.0.saturating_sub(now.0)` — satura a 0 si un salto grande ya cruzó el onset. Comportamiento correcto y documentado.

**Justificacion**:

*Mediciones con el mismo trabajo* (36.000 avances, 10.000 cues, 16.666 µs por avance, mejor de 30):

| Estructura | ns/avance | Factor |
|---|---|---|
| **Cursor sobre slice (adoptada)** | **3,049** | 1x |
| `BinaryHeap<Reverse<Cue>>` | 15,206 | 4,99x peor |
| Rescan ingenuo O(n) | 4.323,855 | **1.418x peor** |

Pero **el argumento fuerte no es la velocidad**: el cursor devuelve `&cues[start..next]`, un **subslice del programa inmutable**. Cero copias y cero asignaciones, verificado con un `GlobalAlloc` contador: **0 asignaciones en 36.000 avances**. El heap está obligado a copiar cada `Cue` a un `Vec` de salida, y muta su estructura en cada `pop` (escrituras dispersas, hostiles a la caché) mientras el cursor sólo incrementa un `usize` y lee memoria contigua.

*Independencia del tamaño de la canción, verificada directamente*: con 1.000 cues emitidos fijos y variando el total de la canción, el coste por avance fue **1,625 / 1,708 / 1,833 / 1,500 ns** para **1.000 / 10.000 / 100.000 / 1.000.000** de cues. **Plano dentro del ruido sobre tres órdenes de magnitud.** Eso es literalmente SC-006.

*El "tiempo no retrocede" se implementa con tipo Y `Result`, en capas distintas*: los newtypes `Micros`/`Ticks` matan la confusión de unidades en tiempo de compilación, pero **ningún tipo puede impedir que QPC salte hacia atrás en una VM**. FR-020 dice "rechazar de forma explícita" = `Err(Rewind { last, requested })`.

*Sobre el orden de los cues bajo cambios de tempo* (la pregunta clave, resuelta y luego deliberadamente no explotada): con un lead **global constante** `L` en ticks, `cue_tick(i) = max(0, onset_tick(i) − L)` es no decreciente en `onset_tick`; el mapa `T: ticks → µs` es no decreciente siempre que `us_per_qn > 0`; la composición de dos funciones no decrecientes es no decreciente. **Por tanto un cambio de tempo NO puede reordenar los cues** (verificado con 4 cambios de tempo, 30 → 240 → 40 → 200 bpm: **0 inversiones en 40 notas**). **PERO** eso sólo vale para un `L` global: con lead variable por nota (mano izquierda 4 negras, derecha 1 negra — exactamente lo que habilita FR-009) se midieron **3 inversiones en 12 notas**. Conclusión: **no apoyarse en la demostración**. Se materializa `cue_at` por cue y se ordena en carga; cuesta un O(n log n) único, muy dentro de los 100 ms de SC-001, y convierte el lead por mano en un cambio de una línea en vez de una reescritura del scheduler.

**Alternativas consideradas**:

- **`BinaryHeap<Reverse<Cue>>`** — 4,99x más lento, obligado a copiar cada cue a un `Vec` de salida en lugar de devolver un subslice, escrituras dispersas hostiles a la caché, y sin `seek` barato. Un heap sólo se justifica si se **insertan** eventos durante la reproducción; aquí la línea temporal es inmutable y se conoce entera en carga.
- **Búsqueda binaria (`partition_point`) en cada avance** — O(log n) por avance en lugar de O(k+1), y sobre todo **viola el requisito literal de SC-006**: el coste crecería con el tamaño de la canción, no con los cues emitidos. Se conserva **únicamente para `seek()`**, que ocurre una vez, no 60 veces por segundo.
- **Rescan ingenuo O(n) por avance** — 1.418x más lento. Descartada sin discusión.
- **Ordenar la línea temporal por onset y confiar en la monotonía demostrada** — La demostración es correcta para lead global constante (0 inversiones verificadas), pero se rompe con lead por nota (3 inversiones en 12 notas medidas). Adoptarla convertiría FR-009 en un bug intermitente que sólo aparece en piezas con cambio de tempo.
- **Devolver `Vec<Cue>` en lugar de `&[Cue]`** — Asignación por avance en la ruta crítica. Prohibido por la Constitución.

**Riesgos y mitigacion**:

- **Alguien "optimiza" quitando `cue_tick` o `note_index` de la clave de orden** por parecer redundantes: se rompe la totalidad de la clave y con ella el determinismo de `sort_unstable`, **sin que falle ningún test existente**. *Mitigación*: documentarlo en el propio `order_key()` y tener un test de propiedad que ordene 10.000 cues generados con colisiones deliberadas de `cue_at`.
- **Un salto de tiempo grande hace que una nota cuyo onset ya pasó reciba igualmente su cue con `remaining_at == 0`.** Es lo que exige FR-015, pero la UI podría interpretarlo como "tócala YA" y mostrar un aviso inútil o engañoso. *Mitigación*: **decidir y documentar la política de UI antes de que exista la UI** (queda registrado como cuestión abierta al final).
- **`seek` con `partition_point` requiere la invariante 1**: si alguien construye `CueSchedule` sin ordenar, `seek` devuelve resultados silenciosamente erróneos. *Mitigación*: el constructor de `CueSchedule` es el único que puede crear el tipo, ordena siempre y verifica el orden con `debug_assert!`.
- **Compartir `Playback` entre hilos**: el cursor es estado mutable. *Mitigación*: el núcleo es monohilo; `Arc<CueSchedule>` es lo único compartible (inmutable), `Playback` no es `Sync` por diseño.

---

## Decisión 6: Semántica de emparejamiento note-on/note-off y solapamiento del mismo pitch

**Decision**:
**Dos fases separadas y testeables por separado**: (A) emparejamiento **FIFO** por clave de voz; (B) post-proceso explícito de **acortado** del solapamiento residual. El orden de las fases es **normativo**: primero FIFO, después acortado.

**R0 — Normalización previa (frontera del parser)**
- `NoteOn { key, vel }` con `vel == 0` se reescribe **SIEMPRE** a `NoteOff { key, vel: 0 }` antes de cualquier otra lógica. Regla dura, sin opción de configuración. **Ningún parser lo hace por nosotros** (verificado tanto en midly como en midi_file: entregan el byte real).
- `NoteOff` con velocity ≠ 0 (release velocity) es un note-off normal; la release velocity se descarta en esta entrega. La velocity de la nota es la del note-on.
- Se ignoran para la línea temporal todos los mensajes que no sean NoteOn/NoteOff (Aftertouch, CC64 sustain, ProgramChange). **El pedal de sustain NO altera duraciones en esta entrega** (fuera de alcance, documentado).

**R1 — Running status**: lo resuelve el parser. **Aun así es obligatorio** un test de regresión con un SMF construido byte a byte que use running status **y** note-on con velocity 0 en el mismo tramo, porque ése es exactamente el patrón que el truco de velocity 0 existe para permitir. Si un día se cambia de crate, ese test lo detecta.

**R2 — Identidad de nota abierta: TRIPLETA `(track_idx, channel, key)`**

```rust
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct VoiceKey { pub track: u16, pub channel: u8, pub key: u8 }
```

**R4 — Emparejamiento FIFO**: una cola por `VoiceKey`; NoteOn → `push_back(seq)`, NoteOff → `pop_front()`. El i-ésimo note-on se empareja con el i-ésimo note-off de la misma voz. Caso canónico `on(60)@0, on(60)@10, off(60)@20, off(60)@30` → notas **[0,20] y [10,30]**. Empates exactos: desempata `seq` (orden del archivo). La cola es determinista siempre.

**R7 — Solapamiento residual del mismo pitch: ACORTAR (activo por defecto, post-proceso separado)**
- Tras FIFO, si dos notas de la misma `VoiceKey` cumplen `n1.onset <= n2.onset < n1.end`, se acorta `n1.end = n2.onset` y se marca `truncated: true`.
- Si `n1.onset == n2.onset` exactamente (note-on duplicado, mismo tick, misma voz): se conserva la primera por orden `seq`, se descarta la segunda y se cuenta en `duplicate_note_ons`. **Así nunca se produce una nota de duración 0.**
- **Invariante resultante, verificable en test**: para cada `VoiceKey`, los intervalos `[onset, end)` son **disjuntos**. Es exactamente lo que la futura fase de evaluación necesita para atribuir una pulsación a una única nota esperada.
- La deduplicación **entre voces distintas** (dos pistas doblando el mismo pitch, físicamente una sola tecla) queda **explícitamente fuera de alcance**: es el problema de asignación de manos que FR-009 aplaza.

**R9 — Percusión y rango de 88 teclas: CONSERVAR Y ETIQUETAR, nunca filtrar**
- `is_percussion = (channel_index == 9)` estrictamente por General MIDI (canal MIDI 10).
- `in_88_range = (21..=108).contains(&pitch)` (A0..C8).
- La carga es una **función total del archivo**: no descarta nada. El filtrado es decisión de la capa de práctica/presentación, reversible sin recargar (FR-009).

**Justificacion**:

*Note-on velocity 0.* La especificación MIDI 1.0 es explícita: *"A Note Off message can take one of two forms: Note Off (8nH), or a Note On (9nH) with a velocity of zero"*, y *"a receiver must be capable of recognizing either method of turning off a note, and should treat them identically"*. El motivo histórico es precisamente el running status: usar note-on vel 0 permite que el running status continúe (un note-off real introduce otro status byte y lo cancela), ahorrando ~1/3 de los bytes en pasajes densos. Ningún parser convierte por nosotros: midly documenta *"by convention a `NoteOn` message with a velocity of 0 is equivalent to a `NoteOff`"* pero su parser hace `0x9 => MidiMessage::NoteOn { key, vel }` sin convertir, y la salida verificada de midi_file muestra `NoteOn(NoteMessage { ..., velocity: Velocity(0) })`. **Si el código de la línea temporal no aplica R0, las notas nunca cierran.**

*FIFO vs LIFO, exhaustivo.* Traza `on(60)@0, on(60)@10, off(60)@20, off(60)@30`:
- **FIFO**: [0,20] y [10,30] — intervalos entrelazados.
- **LIFO**: [10,20] y [0,30] — intervalos anidados.

Ambos conservan el número de notas (2) y la suma de duraciones (40 ticks), así que ése no es el criterio. **El criterio decisivo es cuál recupera la intención del autor.** El origen abrumadoramente mayoritario de este patrón es un piano roll donde dos notas del mismo pitch, de la misma longitud pretendida, se solapan un poco por legato o cuantización: el autor escribió [0,20] y [10,30]. **FIFO devuelve exactamente eso.** LIFO inventa una nota un 50 % más larga ([0,30]) y otra más corta ([10,20]): corrompe el material didáctico. El caso que LIFO recupera bien (una nota corta anidada dentro de otra larga del mismo pitch) es **físicamente imposible en un piano**: no puedes pulsar una tecla ya pulsada sin soltarla. Ventajas adicionales de FIFO: (a) es robusto al orden ambiguo dentro de un mismo tick — el note-off cierra siempre la más antigua sin importar el orden en el archivo; (b) la duración de una nota no depende de eventos arbitrariamente lejanos en el futuro, lo que importa para la ventana de feedforward; (c) es una cola, O(1) y sin lookahead.

*Qué hacen los proyectos reales — están genuinamente divididos, así que la decisión es nuestra y debe ser explícita*:

| Proyecto | Política | Resultado con la traza canónica |
|---|---|---|
| pretty_midi | clave `(channel, note)`, "cerrar todas" | [0,20] y [10,20], segundo note-off huérfano |
| music21 | itera hacia atrás → LIFO efectivo | [10,20] y [0,30] |
| Neothesia (clon de Synthesia en Rust) | truncado: el note-on cierra la anterior | [0,10] y [10,20] — **pierde el tramo 20..30** |
| FluidSynth (sintetizador) | *"overlapping notes are not allowed"*, libera la voz previa | acortado |
| Ardour | lo considera inválido, **6 políticas configurables** | incl. *"shorten the overlapped existing note"* |
| FL Studio | LIFO | — |
| Logic Pro | FIFO | — |

Dos DAWs mayoritarios con comportamiento **opuesto** confirman que no hay estándar de facto. Adoptamos **FIFO para el emparejamiento** (recupera la intención, es total y determinista) **más un post-proceso separado de acortado** (R7, alineado con Ardour y con lo que hará cualquier sintetizador GM, o sea lo que el alumno realmente oirá). Separarlos es lo que permite testear cada uno aislado y deja el acortado como política conmutable sin tocar el parser.

*Por qué la tripleta y no un par*:
- **`(canal, pitch)` falla en SMF tipo 1**: es habitual que mano izquierda y derecha estén en dos MTrk distintos usando **ambos el canal 0**; sin la pista se fusionarían y se corromperían las duraciones. Además destruye la identidad de pista que exige FR-006.
- **`(pista, pitch)` falla en SMF tipo 0**: una sola pista con hasta 16 canales; el bajo del canal 2 y el piano del canal 0 tocando el mismo pitch se pisan. **Es exactamente el bug latente de Neothesia**, que usa `HashMap<u8, NoteInfo>` con el pitch como única clave dentro de cada pista.
- La tripleta además satisface FR-006 sin trabajo extra: **la voz es la propia clave**.

*Percusión y rango*: General MIDI reserva el canal 10 para percusión, y su Percussion Key Map cubre las notas 35..81, **todas dentro de 21..108** — o sea que un filtro por rango **no sustituye** al filtro por canal. Conservar y etiquetar mantiene la carga como función total (más fácil de testear y razonar) y convierte un problema silencioso en información accionable: *"esta canción tiene 3 notas fuera del rango de tu piano, prueba a transponerla una octava"* en vez de que desaparezcan y el alumno crea que su archivo está roto.

**Alternativas consideradas**:

- **LIFO (pila)** — Corrompe la intención del autor en el caso mayoritario: convierte [0,20]+[10,30] en [0,30]+[10,20], inventando una nota un 50 % más larga. Sólo acierta en el caso anidado, físicamente imposible en un piano.
- **"Cerrar todas" con un solo note-off (pretty_midi)** — Produce [0,20] y [10,20] y deja el segundo note-off huérfano: pierde la duración real de la segunda nota y descuadra el conteo. Es una heurística para análisis estadístico de corpus, no para material didáctico donde la duración se muestra al alumno.
- **Truncado puro en el parser (Neothesia)** — Mezcla dos decisiones en un paso y pierde información: descarta el tramo 20..30 y el segundo note-off. Su resultado deseable (intervalos disjuntos) se obtiene igual con FIFO + R7, pero separado, conmutable y testeable por partes.
- **Clave `(canal, pitch)`** — Falla en tipo 1 (manos en pistas distintas con canal 0).
- **Clave sólo `(pitch)` por pista** — Falla en tipo 0 (16 canales en una pista). Bug demostrable en Neothesia.
- **Filtrar percusión y notas fuera de 21..108 en la carga** — Viola FR-009, es destructivo e irreversible sin recargar, y oculta información útil al alumno.
- **Tratar también el canal índice 15 como percusión (heurística de Neothesia)** — No está en General MIDI; clasificaría mal música legítima. Ceñirse al índice 9; overrides GS/XG más adelante si hacen falta.

**Riesgos y mitigacion**:

- **Aplicar R7 antes que FIFO** invierte los resultados y produce las duraciones erróneas de Neothesia. *Mitigación*: el orden de fases es normativo y está cubierto por un test que asserta ambos resultados intermedios.
- **Notas de duración 0** si dos note-on de la misma voz caen en el mismo tick: rompen la invariante `onset < end` que la UI y el futuro scoring asumen. *Mitigación*: deduplicación de R7 + test explícito (`duplicate_note_ons == 1`, ninguna nota de duración 0).
- **Divergencia consciente frente a un sintetizador real**: dos pistas que comparten canal y pitch producen **dos** notas en nuestra línea temporal y **una** sola voz audible. *Mitigación*: documentarlo; si no, la futura fase de evaluación podría exigir dos pulsaciones para una sola tecla. La unificación entre voces queda fuera de alcance.
- **Pedal de sustain (CC64) ignorado**: las duraciones reales sonarán más largas que las de la línea temporal en piezas muy pedaleadas. *Mitigación*: documentar como limitación conocida **de esta entrega**, antes de que aparezca como bug en la futura evaluación.
- **El acortado de R7 altera duraciones respecto al archivo original.** *Mitigación*: exponer el flag `truncated` por nota y el contador `truncated_overlaps` en el `LoadReport`, para poder explicar la diferencia si algún día se exporta o se compara con el MIDI fuente.
- **Colas por `VoiceKey` implementadas con `HashMap`** → orden de iteración no determinista al cerrar notas colgadas. *Mitigación*: ver Decisión 7 y Decisión 11 (R10): índices sobre un `Vec`, o `BTreeMap<VoiceKey, VecDeque<u32>>`.

---

## Decisión 7: Orden total y estable para notas y cues simultáneos

**Decision**:
**Dos claves de orden, ambas TOTALES (sin empates posibles), y una deriva de la otra.**

**(A) Línea temporal de notas** — se ordena por **tiempo musical**, nunca por microsegundos derivados:

```
clave_nota = (onset_tick, pitch, track_idx, channel, seq)      // todo ascendente
```

`seq` es un contador `u32` monótono asignado en la fusión, único por construcción. Tras ordenar, la posición en el `Vec<ScheduledNote>` se llama **`note_index`** y es única y canónica.

**(B) Programa de cues** — se construye **a partir** de la línea temporal ya ordenada:

```
clave_cue = (cue_at_us, cue_tick, note_index)                  // todo ascendente
```

Es total porque `note_index` es único, y **hereda automáticamente** el desempate musical de (A): dentro de un mismo `cue_at`, un acorde sale de grave a agudo sin necesidad de repetir `pitch`/`track`/`channel` dentro de `Cue` (que se queda en 32 bytes exactos). `cue_tick` va antes que `note_index` para que dos cues con distinto tiempo musical que colapsan en el mismo microsegundo (tempos ultrarrápidos, truncado a floor) queden en orden musical correcto.

**Consecuencia práctica**: se puede usar `sort_unstable_by_key` y el resultado es **idéntico bit a bit** entre ejecuciones, entre plataformas y entre versiones de std. **No se depende jamás de la estabilidad del sort ni del orden de entrada.**

**Orden canónico de la fusión de pistas (define el "orden de llegada" y por tanto `seq`)**:

1. Por cada pista, acumular deltas a tick **absoluto** con `checked_add` y el techo `u32::MAX` (Decisión 2).
2. Recolectar tuplas `(tick_abs, track_idx, event_idx_en_pista, evento)` de **todas** las pistas.
3. Ordenar de forma **estable** por la clave total `(tick_abs, track_idx, event_idx)`. **Esa clave total, y no sólo el tick, es lo que hace la fusión reproducible bit a bit (FR-008).** Nunca ordenación inestable sobre una clave con empates, nunca paralelismo, nunca `HashMap`.
4. Asignar `seq` monótono en ese orden.
5. **No reordenar note-off antes de note-on dentro del mismo tick**: se respeta el orden del archivo. FIFO (Decisión 6) hace que esa ambigüedad deje de importar.
6. El tempo **no se asocia a ninguna pista** (Decisión 3); la identidad de voz se conserva en `track_idx`/`channel` por nota (FR-006).
7. El emparejamiento Note On/Off se hace **después** de la fusión pero con colas separadas por `VoiceKey`, **nunca globales**.
8. Formato 2 (pistas secuencialmente independientes, cada una con su propio eje temporal): **no se fusiona**, está fuera de alcance → error tipado.

**Justificacion**:

- **Total > estable**: si la clave tiene empates, el orden entre iguales depende del algoritmo de ordenación y puede variar entre versiones de std. Con `seq`/`note_index` único, el orden queda determinado **por la clave**, no por el algoritmo. Es la única forma de garantizar "bit a bit" sin depender de una propiedad del sort.
- **`pitch` antes que `track_idx`**: un acorde se lee de grave a agudo (convención musical con la que un profesor nombra las notas), la lectura es la misma aunque el acorde esté repartido entre pistas, y los golden files de test son legibles.
- **Ordenar por ticks y no por microsegundos** en la línea temporal evita que el truncado del mapa de tempo introduzca colisiones o reordenamientos.
- **Reconciliación explícita de las dos líneas de investigación**: la investigación del scheduler proponía `(cue_at, onset_tick, track, pitch, source_index)` y la de semántica de notas proponía `(onset_ticks, pitch, track, channel, seq)`. Son objetos distintos (cues vs notas). Se unifican derivando la clave de cue de la de nota vía `note_index`: mismo efecto, una sola definición canónica de "orden musical", `Cue` de 32 bytes y **cero riesgo de que las dos claves se desincronicen** al evolucionar.
- **`sort_unstable` es más rápido y no asigna**; es seguro **sólo** porque la clave es total.

**Alternativas consideradas**:

- **Confiar en `sort_by_key` estable con clave parcial `(onset_tick, pitch)`** — Descartada: la estabilidad de `sort` de std es una garantía de la API, pero el orden de **entrada** que estabiliza depende de nuestra fusión; cualquier cambio en la fusión reordenaría silenciosamente. La clave total elimina la dependencia.
- **Ordenar el acorde por pista y luego por pitch** — Un acorde repartido entre dos pistas se leería entrelazado en vez de de grave a agudo, contra la convención musical, y hace los golden files menos legibles.
- **Ordenar la línea temporal por microsegundos** — El truncado a floor puede colapsar dos onsets distintos en el mismo µs; el orden pasaría a depender del desempate en vez del tiempo musical, y cualquier cambio de política de redondeo movería el orden.
- **Repetir `pitch`/`track`/`channel` dentro de `Cue` para desempatar** — Innecesario (lo aporta `note_index`) y engorda `Cue` por encima de los 32 bytes medidos, empeorando la localidad de caché del array que se recorre 60 veces por segundo.
- **Ordenación paralela (rayon) para acelerar la carga** — Prohibida: introduce planificación de hilos no determinista, y la carga ya cabe holgadamente en SC-001.

**Riesgos y mitigacion**:

- **Alguien quita `seq` o `note_index` de la clave "porque parece redundante"**: se rompe FR-008 **sin que falle ningún test existente**. *Mitigación*: comentario normativo en `order_key()` + test de propiedad con colisiones deliberadas en todos los demás campos.
- **`HashMap`/`HashSet` en cualquier punto que produzca salida** (típicamente al cerrar notas colgadas): `RandomState` se siembra al azar por proceso y el orden de iteración cambia entre ejecuciones. **Es la fuente número uno de no determinismo en Rust.** *Mitigación*: prohibición dura vía `clippy.toml` (`disallowed-types`) + `#![deny(clippy::disallowed_types)]`; usar `BTreeMap`/`Vec`; el cierre de notas colgadas recorre el `Vec` **en orden de índice**, nunca el mapa.
- **Iteración de pistas en orden no determinista** al fusionar. *Mitigación*: `track_idx` explícito y ordenación por la clave total; nunca depender del orden de un iterador del parser.
- **Overflow al acumular `seq`** en un fichero adversario con más de 2³² eventos. *Mitigación*: imposible por el techo de tamaño de fichero de la capa app, pero `checked_add` y error tipado igualmente.

---

## Decisión 8: Política ante ficheros corruptos y notas colgadas

**Decision**:
**Dos categorías estrictamente separadas.**

**(1) Fallos ESTRUCTURALES → `Err` tipado, la carga falla (FR-007).** Nunca panic, nunca `unwrap`, nunca corrupción silenciosa:

```rust
#[non_exhaustive]
pub enum LoadError {
    CabeceraTruncada,
    MagicInvalido,
    CabeceraInvalida { len: u32 },
    FormatoInvalido { format: u16 },
    FormatoNoSoportado { format: u16 },      // formato 2
    TimingSmpteNoSoportado,                  // division bit 15 = 1
    DivisionCero,
    ChunkTruncado { track: u16 },
    MetaMalformado { track: u16, tick: u64, tipo: u8, len_declarada: usize },
    InvalidTempo { tick: u64, us_per_qn: u32 },   // us_per_qn == 0
    TickOverflow { track: u16 },                  // tick_abs > u32::MAX
    DuracionExcesiva { us: u64 },                 // > 24 h (8.64e10 µs)
}
```

**(2) Datos SUCIOS pero tocables → la carga tiene éxito, se etiqueta y se cuenta.** Nunca abortar:

```rust
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct LoadReport {
    pub hanging_notes: u32,        // note-on sin note-off, cerradas en R5
    pub orphan_note_offs: u32,     // note-off sin note-on abierto
    pub duplicate_note_ons: u32,   // dos note-on misma voz mismo tick
    pub truncated_overlaps: u32,   // acortadas por R7
    pub percussion_notes: u32,     // canal índice 9
    pub notes_out_of_88_range: u32,// fuera de 21..=108
    pub tempo_events_outside_track0: u32, // aviso, no altera la salida
}
```

Sin I/O y sin logging: son los contadores que assertan los tests de casos sucios.

**R5 — Nota colgada: CERRAR AL FINAL DE SU PISTA Y ETIQUETAR.**

```rust
end_tick = max(onset_tick + 1, end_of_track_tick(track))
// end_of_track_tick = tick del meta End of Track de ESA pista,
// o el tick del último evento de la pista si el meta falta.
closure  = Closure::HangingClosedAtTrackEnd
hanging_notes += 1
```

**No se descarta. No se usa duración por defecto mágica. Se cierra la PISTA, no la pieza.**

**R6 — Note-off huérfano: ignorar y contar.** Cola vacía en el `pop_front` → no genera nota, no aborta la carga, `orphan_note_offs += 1`. **Nunca panic, nunca error de carga.**

**Defensa en profundidad**: la pre-validación de cabecera de la Decisión 1 se ejecuta **siempre**, antes del parser, con independencia de qué crate esté detrás. El módulo de carga entero se compila bajo `#![forbid(unsafe_code)]` y con prohibición de `unwrap`/`expect`/`panic!` en la ruta de carga (lint + revisión).

**Justificacion**:

*Por qué la separación en dos categorías*: los defectos de la categoría (2) **son rutinarios en archivos MIDI reales** tras ediciones o recortes. Rechazarlos convertiría la mayoría del repertorio disponible en no cargable. FR-007 debe reservarse a fallos estructurales, donde no hay ninguna interpretación defendible de los bytes.

*Por qué cerrar las notas colgadas al final de la pista, argumento pedagógico*:
- **Descartarla es lo peor**: la nota existe en la partitura y en el audio; si desaparece de la lección, el alumno recibe un cue que no coincide con lo que oye y **concluye que se está equivocando él**.
- **Duración por defecto** (una negra, p. ej.) inventa información que no está en el archivo, introduce un parámetro mágico que habría que justificar en cada género, y produce lecciones distintas según el valor elegido.
- **Cerrar al final de la pista** es la única opción que conserva la nota, no inventa ningún valor arbitrario y es totalmente reproducible.
- **El daño está acotado por diseño**: en esta entrega el cue se dispara por el **ONSET**, no por la duración, así que una nota colgada mal cerrada degrada como mucho el dibujo en pantalla, **nunca hace perder el aviso**. Y R7 la acota de forma natural: si esa misma voz vuelve a tocar ese pitch más adelante, la nota colgada se trunca en ese onset.

*Por qué ignorar los note-off huérfanos*: no aportan ninguna información pedagógica (no hay nota que cerrar) y aparecen de forma rutinaria. Es lo que hacen pretty_midi (*"Check that a note-on exists (ignore spurious note-offs)"*) y Neothesia. **Contarlos** permite que un test lo assertee y que se detecten archivos sistemáticamente rotos, sin degradar la carga.

*Por qué la lista de errores estructurales es exactamente ésa*: cada entrada corresponde a un fallo **reproducido** durante la investigación — `TimingSmpteNoSoportado` y `DivisionCero` neutralizan los dos panics medidos; `MetaMalformado` cubre el descarte silencioso de tempo/keysig truncados (2, 1 y 0 bytes → `Ok` sin tempo en midly, medido); `TickOverflow` sostiene la prueba de no-overflow; `DuracionExcesiva` protege el producto `us * ppq` de la conversión inversa.

**Alternativas consideradas**:

- **Descartar las notas colgadas** — Hace desaparecer de la lección notas que el alumno ve en la partitura y oye en la referencia; el alumno atribuye el desajuste a su propia ejecución. Pérdida de información irrecuperable en la carga.
- **Cerrarlas con una duración por defecto (p. ej. una negra)** — Inventa información ausente e introduce una constante mágica que cambia la lección según el valor y el género. No hay criterio objetivo para fijarla.
- **Cerrarlas al final de la PIEZA (max global)** — Una pista corta contaminaría la lección con notas de varios minutos. El final de la propia pista es el límite superior físicamente defendible.
- **Rechazar el archivo si contiene notas colgadas o note-offs huérfanos** — Convertiría la mayoría del repertorio real en no cargable.
- **Aceptar el descarte silencioso de metas truncados** (lo que hace midly incluso con `strict`) — Es el fallo más caro del producto: la pieza suena a 120 bpm y el alumno se evalúa contra una referencia errónea, sin aviso.
- **Clampar `us_per_qn == 0` a 500.000** — Entregaría una lección falsa (contra FR-007) y rompería la monotonía del mapa de tempo, base de toda la demostración de orden de los cues. No hay lectura musical defendible del valor 0.
- **Recuperación con reintento en modo laxo** (parsear estricto y, si falla, reintentar permisivo) — Imposible con midly (`strict` es compile-time, todo-o-nada) y **indeseable en general**: produce dos comportamientos para la misma entrada según el estado del proceso, contra el determinismo.

**Riesgos y mitigacion**:

- **Una nota colgada cerrada al final de su pista puede dar una duración enorme** (minutos) y dibujarse como una barra gigante. *Mitigación*: flag `Closure::HangingClosedAtTrackEnd` para que la UI la represente como **duración incierta**, más el acortado de R7 cuando la voz vuelva a tocar ese pitch.
- **Un `LoadReport` que nadie mira** deja pasar archivos sistemáticamente rotos. *Mitigación*: la capa app muestra un resumen no bloqueante al cargar; los tests de casos sucios assertan cada contador.
- **Acumular ticks con suma sin comprobar** puede desbordar o entrar en pánico con archivos corruptos o maliciosos. *Mitigación*: `u64` + `checked_add` + techo `u32::MAX` → `LoadError::TickOverflow`, nunca panic.
- **El parser subyacente panica en una entrada que nuestra pre-validación no cubre.** *Mitigación*: (a) `midi_file` dio **0 panics en 265.536 entradas** de fuzz diferencial; (b) el módulo de carga es la única superficie expuesta y se le añade un test de fuzz propio con semilla fija en CI; (c) **queda como cuestión abierta** si envolver `load_smf` en `catch_unwind` en la capa Tauri — decisión provisional: **no** en el core (ocultaría bugs), **sí** evaluarlo en la capa app antes de la primera release pública.
- **`end_of_track_tick` ausente** en una pista sin meta EOT: *Mitigación*: fallback documentado al tick del último evento de esa pista, con test.

---

## Decisión 9: Antelación feedforward en tiempo musical

**Decision**:
**La antelación (`lead`) se expresa y se almacena en PULSOS (ticks), nunca en milisegundos.** El tiempo real del aviso se **deriva** del mapa de tempo, así que la antelación se estira y encoge con el tempo automáticamente (FR-011).

Cálculo, **todo en carga**, nunca en la ruta crítica:

```rust
// lead_ticks: u64, parámetro de la sesión (p. ej. 480 ticks = una corchea a PPQ 960)
let cue_tick   = onset_tick.saturating_sub(lead_ticks);   // satura a 0 si la nota va antes
let cue_at_us  = tempo_map.tick_to_us(cue_tick);          // Decisión 2
let onset_at_us= tempo_map.tick_to_us(onset_tick);
// margen real del aviso, sólo informativo:
let margen_us  = onset_at_us - cue_at_us;
```

En reproducción: `cue.remaining_at(now) = cue.onset_at.0.saturating_sub(now.0)` (FR-014).

**El lead es GLOBAL y constante en esta entrega.** El lead **por voz/mano** (FR-009) se habilita sin cambiar el scheduler porque `cue_at` está materializado y ordenado (Decisión 5/7): es un cambio de una línea en la construcción, no una reescritura.

**Justificacion**:

*Ejemplo numérico verificado* (PPQ = 960, mapa de tempo del ejemplo de la Decisión 3, lead = 480 ticks = una corchea):

```
Nota en tick 9000 (tramo k=2, us_per_qn = 461538 → 130 bpm)
  cue_tick = 9000 - 480 = 8520
  delta    = 8520 - 7680 = 840;  840 * 461538 = 387_691_920
  scaled   = 3_456_000_000 + 387_691_920 = 3_843_691_920
  cue_at   = 3_843_691_920 / 960 = 4_003_845 µs
  onset_at = 4_234_614 µs   (calculado en la Decisión 3)
  margen real = 4_234_614 - 4_003_845 = 230_769 µs = 230,77 ms
  Verificación independiente: 480 ticks = media negra = 461538 / 2 = 230_769 ✓

La MISMA antelación de 480 ticks en el primer tramo (500000 µs/negra, 120 bpm):
  us(480) - us(0) = 250_000 - 0 = 250_000 µs = 250 ms
```

**Misma distancia musical, margen real distinto (250 ms vs 230,77 ms): eso es literalmente FR-011 y el escenario 6 de la Historia 2.** El mismo comportamiento se verificó de forma independiente sobre datos reales de parser: con PPQ 480, lead 480 ticks y mapa `[(0, 500000), (960, 250000)]`, la antelación vale 500.000 µs a 120 bpm y 250.000 µs a 240 bpm.

`saturating_sub` en `cue_tick` cubre el caso de una nota cuyo onset es menor que la antelación (arranque de la pieza): su cue cae en tick 0 y se emite en el primer avance — hay un test dedicado a esto.

**Alternativas consideradas**:

- **Guardar la antelación en milisegundos y convertirla a ticks en reproducción** — Contradice FR-011 (la antelación debe ser musical y estirarse con el tempo) **y** mete una división en la ruta crítica de <30 ms p95.
- **Guardar la antelación en ms y recalcular `cue_at` en cada cambio de tempo** — Requiere lógica de re-planificación en tiempo real, rompe la inmutabilidad del programa de cues y con ella el cursor O(k+1) y las 0 asignaciones.
- **Antelación expresada en negras/compases en vez de ticks** — Equivalente pero peor: el compás introduce el Time Signature en un cálculo que no lo necesita (ver riesgo de la Decisión 2). Los ticks son la unidad nativa del archivo; la UI puede presentarlo como "media negra" haciendo `lead_ticks = ppq / 2`.
- **Emitir el cue con antelación por nota desde el día uno** — Fuera del alcance de esta entrega (FR-009 lo aplaza), pero el diseño ya lo soporta sin cambios estructurales.

**Riesgos y mitigacion**:

- **Alguien "simplifica" ordenando por onset y sumando el lead en reproducción**: funciona con lead global (0 inversiones verificadas con 4 cambios de tempo entre 30 y 240 bpm) y **se rompe en silencio** el día que se añada lead por mano (3 inversiones en 12 notas medidas), sólo en piezas con cambio de tempo. *Mitigación*: `cue_at` materializado y ordenado desde el día uno + comentario normativo.
- **`remaining_at` saturado a 0 tras un salto grande** puede leerse en la UI como "tócala YA". *Mitigación*: política de UI a decidir y documentar (cuestión abierta).
- **`lead_ticks` mayor que la pieza entera**: todos los cues caen en tick 0 y se emiten juntos en el primer avance. Es correcto y determinista, pero conviene un test que lo fije como comportamiento esperado.

---

## Decisión 10: Estrategia de tests y garantía de determinismo

**Decision**:
**Fixtures dorados como columna vertebral + property tests con semilla FIJADA**, todo bajo un presupuesto de suite <1 s (SC-002).

**1. Fixtures dorados**: SMF construidos **byte a byte en código Rust**, no ficheros binarios en el repo. Cubren tipos 0 y 1, running status, note-on vel 0, acordes repartidos entre pistas, dos cambios de tempo, tempos no divisibles por PPQ (461538, 435897), 6/8, notas colgadas, note-offs huérfanos, canal 10 y notas fuera de 21..108. Coste medido: ~0 ms.

**2. Property tests con `proptest 1.11.0`** (dev-dependency pura, nunca en el binario ni en la ruta crítica). Las cinco propiedades, por orden de valor:

1. **Partir la secuencia de avances arbitrariamente da la misma traza que un único avance grande** (FR-015). *La de mayor valor, prácticamente imposible de cubrir con fixtures.*
2. `run(song, steps) == run(song, steps)` bit a bit (FR-018).
3. Sin duplicados y `emitidos == cues.filter(cue_at <= total).count()` (FR-012).
4. El orden de emisión == orden de la clave total (FR-013).
5. `advance_to(t)` seguido de `advance_to(t' < t)` siempre devuelve `Err` (FR-020).

**3. LA TRAMPA, verificada empíricamente: `ProptestConfig::rng_seed` es ALEATORIA en cada ejecución.** Medidos los 5 primeros valores generados en 3 corridas: **distintos las 3 veces**. Eso es *flakiness estructural*. **Arreglo obligatorio**:

```rust
let cfg = ProptestConfig { cases: 256, ..ProptestConfig::default() };
let mut runner = TestRunner::new_with_rng(
    cfg,
    TestRng::deterministic_rng(RngAlgorithm::ChaCha),   // verificado: idéntico en 3 corridas
);
```
(o fijar `PROPTEST_RNG_SEED` en CI). Y **comitear `proptest-regressions/`**: cada contraejemplo encontrado se convierte en fixture permanente.

**4. Añadir al `Cargo.toml` del workspace, ANTES de escribir las property tests:**

```toml
[profile.test]
opt-level = 2
debug-assertions = true   # que sigan disparando los debug_assert! de las invariantes
```

**5. Tests de contrato del parser** (`core/tests/parser_contract.rs`): fijan el comportamiento observado de `midi_file` — tipos 0/1, PPQ, tempo, running status, note-on vel 0, y los cuatro casos que refutaron midly (SMPTE `0x80`, `division == 0`, tempo truncado, keysig corto). Son los que hacen barato cambiar de parser.

**Justificacion**:

- **El presupuesto de pruebas es real y aprieta**: 10.000 casos aleatorios con un SplitMix64 propio tardan **0,63 s en perfil dev** — **el 63 % del presupuesto de 1 s en un solo test**. Con `[profile.test] opt-level = 2` caen a **0,05 s**. proptest con 256 casos × 2 propiedades: 0,13 s en dev, 0,01 s optimizado. **Una línea de Cargo.toml = 10x de presupuesto.** Y esto invierte la conclusión intuitiva: **proptest no es el problema, el perfil sin optimizar sí.**
- **Por qué proptest y no sólo un PRNG propio**: un SplitMix64 de 10 líneas cubre el 95 % del valor, pero **no hace shrinking** — y el shrinking es todo el valor cuando falla una canción aleatoria de 200 notas con 300 avances: sin él tienes un contraejemplo ilegible. La restricción constitucional sobre dependencias aplica a la ruta crítica, no a las dev-dependencies.
- **El esqueleto de referencia ya existe y funciona**: compila con `#![forbid(unsafe_code)]` y pasa **12 pruebas en 0,04 s** (now==last no es retroceso; retroceso es `Err` explícito; salto grande no se salta ninguna nota; cada cue exactamente una vez sin duplicados; particionar el avance no cambia la traza; cue en tick 0 se emite en el primer avance; canción vacía termina limpiamente; seek como único camino atrás; reloj virtual y real intercambiables; determinismo bit a bit).
- **El test de doble carga compara el `Vec<ScheduledNote>` COMPLETO**, no sólo su longitud: es el único que detecta un `HashMap` colado en la ruta de cierre de notas colgadas.

**Alternativas consideradas**:

- **Sólo fixtures deterministas, sin proptest** (cero dependencias, la Constitución prefiere std) — Descartada por la falta de shrinking; sin él, un contraejemplo aleatorio es inutilizable. Se adoptan **ambos**, con fixtures como columna vertebral.
- **proptest con la configuración por defecto** — Descartada: `rng_seed` aleatoria por corrida hace que un test pase en local y falle en CI **sin ningún cambio de código**, contra "determinismo total, cero flakiness".
- **Tests con `Instant` y `sleep`** — Descartada por los cinco motivos de la Decisión 4 (timer de Windows de 15,6 ms, 24,06 % de deltas nulos, divergencia de granularidad entre plataformas, imposibilidad de probar una canción de 10 min en <1 s, violación de FR-018).
- **Ficheros `.mid` binarios como fixtures en el repo** — Descartada: opacos en el diff, imposibles de razonar en una revisión, y no permiten construir los casos patológicos exactos (truncados en un offset concreto, byte-flips deterministas).
- **Dejar el perfil de test por defecto y reducir el número de casos** — Reduce cobertura para arreglar un problema que se resuelve con una línea de configuración.

**Riesgos y mitigacion**:

- **Añadir `[profile.test] opt-level = 2` DESPUÉS de escribir las property tests**: la suite ya habrá reventado el presupuesto y la reacción natural será recortar casos. *Mitigación*: es la primera tarea del setup, antes del primer test.
- **Quitar `debug-assertions` "para ir más rápido"**: se apagan todos los `debug_assert!` de invariantes, que son la mitad de la red de seguridad. *Mitigación*: fijado explícitamente en el perfil y comentado.
- **`proptest-regressions/` no comiteado**: los contraejemplos encontrados se pierden en cada CI. *Mitigación*: comitear el directorio y añadirlo a la revisión de PR.
- **Los golden tests con PPQ=960 y tempo 500000 pasan aunque el algoritmo sea incorrecto** (casi todas las cuentas salen exactas). *Mitigación*: incluir obligatoriamente tempos no divisibles por PPQ y subdivisiones no triviales.
- **La suite crece y rompe el segundo sin que nadie lo note**. *Mitigación*: un test/paso de CI que mida el tiempo total y falle por encima de 1 s.

---

## Decisión 11: Prohibiciones estructurales en `core/`

**Decision**:
Restricciones **verificadas por herramienta**, no por buena voluntad.

En `core/src/lib.rs`:

```rust
#![forbid(unsafe_code)]
#![deny(clippy::disallowed_types)]
#![deny(clippy::float_arithmetic)]
```

En `core/clippy.toml`:

```toml
disallowed-types = [
  { path = "std::collections::HashMap", reason = "orden de iteracion no determinista (RandomState). Usa BTreeMap o Vec." },
  { path = "std::collections::HashSet", reason = "idem. Usa BTreeSet o Vec." },
  { path = "std::time::SystemTime",     reason = "no monotono (salta con NTP). Usa Instant via MonotonicClock." },
]
```

Reglas adicionales, verificadas en revisión de código:

| Prohibición | Motivo |
|---|---|
| `f32` / `f64` en cualquier punto del core | Determinismo bit a bit (Decisión 2). Sólo se convierte a float **en la frontera de la UI**, y ese valor **nunca** vuelve al core. |
| Acceso a red, a disco, a variables de entorno | Constitución. El core recibe `&[u8]`. |
| `thread::sleep` o cualquier bloqueo | Ruta crítica <30 ms p95; timer de Windows de 15,6 ms. |
| Hilos, `rayon`, cualquier paralelismo | Planificación no determinista. |
| Asignaciones en la ruta crítica | Verificado a 0 con un `GlobalAlloc` contador en 36.000 avances. |
| `unwrap` / `expect` / `panic!` en la ruta de carga | FR-007: errores tipados. |
| Casts `as` que puedan truncar | `From`/`TryFrom` + `checked_*`. |
| Dependencias en el core más allá de `midi_file` | Superficie mínima; `proptest` sólo como dev-dependency. |

**Justificacion**:

- **`HashMap` es la fuente número uno de no determinismo en Rust**: `RandomState` se siembra al azar por proceso, así que el orden de iteración cambia entre ejecuciones. Rompería FR-008/FR-018 **en silencio y de forma intermitente**, que es el peor modo de fallo posible y dificilísimo de depurar. La prohibición por lint es lo único que sobrevive a un cambio de equipo.
- **El float ya está justificado en la Decisión 2** con datos: 10,3 % de los eventos en un microsegundo distinto con f64 acumulado, 1,69 ms de deriva con f32, y suma secuencial ≠ pairwise ya en bits, con dos targets distintos (macOS arm64 y Windows x86_64).
- **`forbid(unsafe_code)`** contrasta directamente con lo que motivó descartar midly (15 usos de `unsafe` sin auditar, incluido un `transmute` de lifetimes, en un crate sin mantenimiento). Es coherente exigirnos lo que exigimos a las dependencias.
- **Se prefiere el lint al comentario**: la investigación mostró que todos los fallos graves detectados eran silenciosos y sólo aparecían midiendo. Un lint falla en CI; un comentario no.

**Alternativas consideradas**:

- **Documentar las reglas en CLAUDE.md / en el README sin lints** — Descartada: las reglas silenciosas se rompen sin que falle nada, y todos los riesgos identificados en esta investigación son de fallo silencioso.
- **Permitir `HashMap` con un hasher determinista fijo (`FxHashMap`, `BuildHasherDefault`)** — Resolvería el determinismo del orden pero añade una dependencia o un hasher propio a cambio de nada: con n pequeño, `BTreeMap`/`Vec` son igual de rápidos y sin ningún riesgo residual. Se reevalúa sólo si aparece un perfil que lo justifique.
- **Permitir `f64` "sólo para logging/depuración"** — Descartada: es exactamente la puerta trasera por la que el float vuelve al cálculo. El logging formatea el `u64` a la salida y punto.
- **`deny(unsafe_code)` en vez de `forbid`** — `forbid` no se puede desactivar con un `#[allow]` local; es la variante que resiste el "sólo aquí, es seguro".

**Riesgos y mitigacion**:

- **`deny(clippy::float_arithmetic)` puede ser demasiado ruidoso** si alguna utilidad legítima (p. ej. un formateo de bpm para depuración) necesita float. *Mitigación*: si aparece, se aísla en un módulo `display` con `#[allow]` explícito y comentado, **que no puede devolver valores al core**. **Queda como cuestión abierta menor**: decisión provisional, activarlo y relajarlo sólo con justificación escrita.
- **La unificación de features del workspace** puede colar features indeseadas en `core/` cuando `src-tauri` dependa de él. *Mitigación*: `resolver = "2"` (ya está) y un test de CI que compruebe el `cargo tree` de `core/` (1 dependencia, 0 transitivas).
- **`clippy.toml` sólo actúa si CI ejecuta clippy.** *Mitigación*: `cargo clippy -- -D warnings` como paso obligatorio del CI, no opcional.

---

## Decisión 12: Alcance declarado de la primera entrega

**Decision**:
Fuera de alcance **explícito y con error tipado o etiqueta**, no silencioso:

| Tema | Tratamiento en esta entrega |
|---|---|
| SMF formato 2 (pistas secuencialmente independientes) | `LoadError::FormatoNoSoportado { format: 2 }`. No se fusiona: cada pista tiene su propio eje temporal. |
| Timing SMPTE (`division` bit 15 = 1) | `LoadError::TimingSmpteNoSoportado`. La premisa de "tiempo musical invariante al tempo" **no aplica** a división SMPTE. Cuando se soporte: `us_por_tick = 1_000_000 / (fps * subframes)`, constante y sin mapa de tempo (ojo: "29" = 29,97 drop-frame). |
| Pedal de sustain (CC64) | Ignorado. Las duraciones reales sonarán más largas en piezas muy pedaleadas. **Limitación conocida documentada.** |
| Release velocity (`NoteOff` con vel ≠ 0) | Descartada. La velocity de la nota es la del note-on. |
| Evaluación y puntuación | Fuera de alcance. Esta entrega no compara nada con la entrada del alumno. |
| Asignación de manos / filtrado de voces (FR-009) | Fuera de alcance, pero **habilitado por diseño**: las etiquetas (`track`, `channel`, `is_percussion`, `in_88_range`) se conservan y `cue_at` está materializado, así que el lead por mano es un cambio de una línea. |
| Deduplicación entre voces distintas (dos pistas doblando el mismo pitch) | Fuera de alcance. Es el problema de asignación de manos. |
| Escalado de tempo de práctica (tocar al 70 %) | Fuera de alcance, con el camino ya fijado: racional exacto sobre `us_per_qn` (`us_per_qn * den / num`, u64, una división) y **reconstrucción de la línea temporal en carga**. Nunca multiplicar µs por un float en tiempo real. |
| Red | Prohibida en runtime. |

**Justificacion**:
Declarar el alcance con **error tipado** en vez de con comportamiento indefinido es lo que convierte una limitación en información accionable. El caso paradigmático es SMPTE: tratar una división con bit 15 = 1 como si fuera PPQ hace que el valor se lea como un número enorme y **toda la pieza salga con tiempos disparatados** — un fallo silencioso y desconcertante. Un `Err` claro le dice al usuario exactamente qué pasa. Lo mismo con el formato 2: fusionarlo produciría una superposición musicalmente absurda de piezas independientes.

Las limitaciones que **no** pueden ser errores (sustain, release velocity) se documentan **antes** de que aparezcan como bug en la futura fase de evaluación, que es cuando la diferencia entre duración anotada y duración sonada se vuelve visible.

**Alternativas consideradas**:

- **Soportar SMPTE ya** — Trabajo real (fps, subframes, drop-frame 29,97) para un caso de uso que no existe en repertorio de piano didáctico. Se aplaza con el camino ya trazado.
- **Soportar formato 2 concatenando las pistas** — Inventaría una estructura musical que el archivo no declara.
- **Aplicar el sustain estirando duraciones ya** — Requiere modelar el pedal como estado y decidir qué significa pedagógicamente ("mantén la tecla" vs "el sonido sigue"). Decisión de producto, no técnica; se aplaza deliberadamente.
- **Silenciar las limitaciones** — Descartada por principio: todos los fallos caros identificados en esta investigación eran silenciosos.

**Riesgos y mitigacion**:

- **Un usuario con ficheros SMPTE o formato 2 se queda bloqueado sin entender por qué.** *Mitigación*: mensaje de error de la capa app específico por variante de `LoadError`, no un genérico "archivo inválido".
- **La limitación del sustain se descubre en la fase de evaluación y se interpreta como bug del scoring.** *Mitigación*: documentada aquí y en el README del crate desde el día uno.

---

## Cuestiones abiertas y decisiones provisionales

Nada de esto está silenciado: son los puntos que la investigación **no** cerró.

1. **API de iteración de pistas y delta-times de `midi_file 0.2.0` — NO verificada por escrito.** Se verificó `MidiFile::read`, el `Header` (`format: Multi`, `division: QuarterNote(QuarterNoteDivision(480))`), `Meta(SetTempo(MicrosecondsPerQuarter(500000)))` y `Midi(NoteOn(NoteMessage { channel, note_number, velocity }))` con running status resuelto, pero **no los accesores exactos para recorrer pistas y leer ticks**.
   **Decisión provisional**: la **tarea T0 del TDD**, antes de cualquier otra cosa, es un test de contrato que carga un SMF tipo 1 de bytes crudos y asserta ticks absolutos pista por pista. Si esa API no expone lo necesario, se activa el plan B (midly endurecido con la pre-validación de cabecera + validación de longitud de metas 0x51/0x58/0x59) documentado en la Decisión 1.

2. **`midi_file` está mucho menos rodado que midly** (12.577 vs 371.853 descargas) y es pre-1.0 con rotación de API reciente.
   **Decisión provisional**: adoptarlo igualmente — la robustez medida (0 panics en 265.536 entradas de fuzz, 0 `unsafe`, MIT, mantenido este mes) pesa más que la popularidad de una crate con un panic abierto sin parchear. Mitigado con frontera de un solo módulo, tests de contrato y `Cargo.lock` comiteado.

3. **Política de UI cuando `remaining_at` satura a 0** tras un salto de tiempo grande (FR-015). El core hace lo correcto (emite el cue con margen 0), pero la UI podría mostrarlo como "tócala YA", que es engañoso.
   **Decisión provisional**: el core expone `remaining_at` **y** el flag de que el onset ya pasó; la decisión de presentación se toma cuando exista la UI, y se documenta como contrato pendiente.

4. **`catch_unwind` alrededor de `load_smf` en la capa Tauri**: protegería contra un panic residual del parser en un fichero del usuario, pero ocultaría bugs.
   **Decisión provisional**: **no** en el core; evaluarlo en la capa app **antes de la primera release pública**, nunca antes de tener el fuzz propio en CI.

5. **Tope al número de tramos de tempo** (fichero adversario con un Set Tempo por tick).
   **Decisión provisional**: sin tope en el core (el coste sigue siendo O(log n) en consulta y O(n) en construcción); la validación de tamaño de fichero vive en la capa de aplicación.

6. **`deny(clippy::float_arithmetic)` puede resultar demasiado ruidoso** si alguna utilidad de presentación legítima necesita float.
   **Decisión provisional**: activarlo; relajarlo sólo en un módulo `display` aislado con `#[allow]` explícito y comentado que **no pueda devolver valores al core**.

7. **Divergencia macOS/Windows en suspensión del sistema** (`CLOCK_UPTIME_RAW` se para al dormir, QPC sigue contando): la doc de std declara el comportamiento **no especificado**.
   **Decisión provisional**: `MonotonicClock::rebase_to(playhead)` existe desde el día uno, pero **la política de cuándo invocarlo** (detección de suspensión, umbral de salto aceptable) es de la capa app y queda sin decidir. Test de plataforma cruzada pendiente en el CI de Windows.

8. **Auditoría de licencias con `cargo-about`/`cargo-deny`** no ejecutada todavía sobre el árbol completo del workspace (sólo se auditó la crate MIDI). Con `midi_file` (MIT) desaparece la necesidad de añadir Unlicense a la allow-list, pero la auditoría global sigue pendiente.
   **Decisión provisional**: tarea de setup, antes del primer merge a la rama principal.
