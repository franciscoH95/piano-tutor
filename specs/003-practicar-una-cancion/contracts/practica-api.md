# Contrato: práctica y puente con la interfaz

**Feature**: `003-practicar-una-cancion` | **Fecha**: 2026-08-18

Tres fronteras, y la de en medio es la que esta feature estrena.

```
piano-core          decide          probado headless, sin ventana
    │
src-tauri           traduce         el puente, sin lógica musical
    │
src/practica        pinta           sin decisiones (excepción del Principio II)
```

## Núcleo: `piano_core::practica`

```rust
/// El modo de práctica. Un DATO, no un comportamiento.
pub enum Avance { PorReloj, PorAcierto }

/// Proporción respecto al tempo original. Racional: reducir y volver no acumula error.
pub struct Velocidad { /* num, den */ }
impl Velocidad {
    pub const NORMAL: Self;
    pub const PAUSA: Self;
    pub fn nueva(num: u32, den: u32) -> Option<Self>;   // None si den == 0
}

pub struct SesionDePractica<C: Clock, F: FuenteDeEventos> { /* ... */ }

impl<C: Clock, F: FuenteDeEventos> SesionDePractica<C, F> {
    pub fn nueva(cancion: Song, reloj: C, fuente: F) -> Self;

    /// Avanza la práctica hasta el instante actual del reloj y devuelve qué cambió.
    ///
    /// Es la única función que mueve el cursor. En `PorAcierto` se detiene en la puerta
    /// pendiente aunque el reloj siga corriendo: el tiempo no se para, la posición sí.
    pub fn avanzar(&mut self) -> Paso;

    pub fn poner_en_marcha(&mut self);
    pub fn pausar(&mut self);
    pub fn saltar_a(&mut self, posicion: Micros);
    pub fn cambiar_avance(&mut self, avance: Avance);
    pub fn cambiar_velocidad(&mut self, v: Velocidad);
    pub fn practicar_mano(&mut self, mano: Option<Mano>);   // None = las dos
    pub fn mover_corte(&mut self, key: u8);

    pub fn posicion(&self) -> Micros;
    pub fn ha_terminado(&self) -> bool;
}

/// Lo que cambió en un avance. Es lo que cruza el puente.
pub struct Paso {
    pub posicion: Micros,
    /// `Some` solo cuando cambió el régimen: el frontend interpola entre anclas y NO
    /// necesita que esto cruce sesenta veces por segundo.
    pub ancla: Option<Ancla>,
    pub esperando: bool,
    pub terminada: bool,
}

/// Qué dibujar en un instante dado. La capa que pinta recibe esto y nada más.
pub fn vista(cancion: &Song, desde: Micros, hasta: Micros, out: &mut Vec<NotaVisible>);

pub struct NotaVisible {
    pub key: u8,
    pub onset_us: Micros,
    pub end_us: Micros,
    pub mano: Mano,
    pub dedo: Dedo,
    pub nombre: NombreDeNota,   // símbolo, no cadena
    pub estado: EstadoNota,     // pendiente · sonando · acertada · omitida
}
```

**Garantías del núcleo**:

- `avanzar` no asigna memoria y su coste no depende del tamaño de la canción.
- `vista` escribe en un `Vec` que el llamante reutiliza; devolver una lista nueva cada vez también
  vale —está medido que la diferencia es 0,0005 ms— pero el parámetro de salida deja la elección
  al llamante.
- Todo es determinista: misma canción y misma secuencia de pulsaciones, mismo resultado.

## Puente: `src-tauri`

```rust
#[tauri::command] fn abrir_cancion(ruta: String) -> Result<ResumenCancion, String>;
#[tauri::command] fn registrar_canal(canal: Channel<MensajeAlFrontend>);
#[tauri::command] fn transporte(accion: Accion);      // marcha, pausa, saltar, velocidad, modo
#[tauri::command] fn ajustar(ajuste: Ajuste);          // mano, corte de manos
#[tauri::command] fn vista_actual() -> Vec<NotaVisiblePlana>;

#[derive(Serialize)]
#[serde(tag = "tipo")]
enum MensajeAlFrontend {
    Tecla { key: u8, pulsada: bool },
    Ancla { posicion_us: u64, instante_us: u64, num: u32, den: u32, tope_us: Option<u64> },
    Esperando { key: u8 },
    Terminada,
    DispositivoPerdido,
}
```

**Contrato del puente, y la razón de cada punto**:

1. **Un solo `Channel` por sesión**, para eventos de tecla y anclas a la vez, discriminado por
   etiqueta. Un solo canal garantiza el orden entre ambos por construcción; dos canales no.
2. **Quien llama a `send` es un hilo reenviador propio**, que drena el anillo que ya existe usando
   `Receptor::esperar()` —duerme, no sondea—. **Nunca** el callback de CoreMIDI ni el consumidor de
   tiempo real: `send` cuesta p95 entre 0,5 y 1,6 ms y llega a 13 ms en el peor caso, y eso dentro
   de la ruta crítica arruinaría el presupuesto del Principio IV que la feature 002 dejó cerrado.
3. **El cursor no cruza sesenta veces por segundo.** Cruza un ancla solo al cambiar de régimen —al
   arrancar, al pausar, al saltar, al cambiar de velocidad, al llegar a una puerta— y el frontend
   interpola linealmente entre anclas. Es lo que mantiene el puente casi vacío.
4. **`src-tauri` no toma ninguna decisión musical.** Traduce llamadas y reenvía. Si aquí aparece
   lógica, esa lógica pertenece a `piano-core`.

## Capa que pinta: `src/practica/Lienzo.tsx`

```ts
export function pintar(ctx: CanvasRenderingContext2D, escena: Escena): void;
```

**Es el único archivo acogido a la excepción del Principio II** (Constitución v1.1.0), y solo lo
merece si cumple su condición: **no decide nada**. Recibe una escena ya calculada —rectángulos,
etiquetas, teclas iluminadas— y la dibuja.

Reglas que lo mantienen así:

- No consulta la hora, no lee estado global, no calcula posiciones musicales.
- No sabe qué es una nota, un compás ni un tempo. Sabe de rectángulos y textos.
- Si necesita un `if` sobre algo musical, ese `if` va a `piano-core`.

**Prohibido en esta capa**, por medición: sombras, desenfoques y filtros. `shadowBlur = 8` con 298
notas hundió la cadencia a 40,9 fotogramas por segundo **mientras el cronómetro interno seguía
marcando 0,748 ms** — el coste se paga fuera de JavaScript y no se ve desde dentro. Cualquier efecto
visual pasa antes por el banco de fotogramas.

## Lo que este contrato NO ofrece

- No puntúa ni califica (FR-027). Distingue acierto, nota extra y nota omitida; medir *con cuánta
  precisión* se tocó es de la feature siguiente.
- No produce sonido (FR-028).
- No guarda la interpretación (FR-029).
- No dibuja partitura ni muestra la octava de cada nota.
