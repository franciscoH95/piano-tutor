# Contrato: API de captura

**Feature**: `002-midi-input-capture` | **Fecha**: 2026-08-18

## Principio de frontera

`piano-core` define **qué** es una fuente de eventos y no sabe **cómo** se obtienen. La capa que
habla con el sistema operativo vive en `piano-midi-io` y no toma ninguna decisión de dominio.

La comprobación no es de confianza, es mecánica:

```sh
cargo tree -p piano-core   # exactamente 3 líneas: piano-core, midi_file y rtrb
```

## Núcleo: `piano_core::capture`

```rust
/// De dónde salen las pulsaciones. Se inyecta por genérico, no como `dyn`.
pub trait FuenteDeEventos {
    /// Recoge las pulsaciones disponibles. No bloquea.
    fn recoger(&mut self, destino: &mut Vec<EventoCrudo>) -> usize;
    /// Espera hasta que haya algo o hasta que se pida parar. Es donde el consumidor duerme.
    fn esperar(&mut self);
    /// `true` si la fuente ya no puede entregar más (dispositivo perdido, o guion agotado).
    fn agotada(&self) -> bool;
}

/// Fuente controlada: la que hace verificable toda la lógica sin hardware.
pub struct FuenteGuionizada { /* ... */ }
impl FuenteGuionizada {
    /// Construye una fuente a partir de una secuencia de (instante, evento).
    pub fn nueva(guion: Vec<(Micros, EventoCrudo)>) -> Self;
}

/// El emparejador: convierte eventos crudos en pulsaciones con principio y final.
pub struct Emparejador { /* ... */ }
impl Emparejador {
    pub fn nuevo() -> Self;
    /// Consume un evento. Devuelve una pulsación cuando el ataque encuentra su suelta.
    pub fn consumir(&mut self, ev: EventoCrudo) -> Option<PulsacionCapturada>;
    /// Cierra las teclas aún hundidas y devuelve sus pulsaciones (FR-015).
    pub fn cerrar(&mut self, at: Micros, motivo: Cierre) -> Vec<PulsacionCapturada>;
    pub fn informe(&self) -> &InformeDeCaptura;
}
```

**Garantías**:

- `consumir` no asigna memoria y su coste es constante: la tabla de voces es plana y está
  reservada.
- Alimentado con el mismo guion, `Emparejador` produce siempre las mismas pulsaciones, en el mismo
  orden (FR-022, SC-004).
- `cerrar` con `Cierre::PorPerdidaDeDispositivo` marca `duracion_censurada = true`.

## Transporte: `piano_core::capture::transporte`

```rust
/// Extremo del productor. Vive dentro del callback del sistema operativo.
pub struct Emisor { /* ... */ }
impl Emisor {
    /// Publica una observacion. **Nunca bloquea y nunca asigna.**
    ///
    /// Toma una `Observacion` (instante, altura, intensidad, tipo, canal) y NO un
    /// `EventoCrudo` ya formado: el `seq` lo asigna el transporte, no el llamante.
    /// Si lo pusiera quien llama, el hueco en la secuencia dejaria de demostrar nada,
    /// porque nadie garantizaria que es monotono.
    ///
    /// Si no hay sitio, descarta la observacion ENTRANTE e incrementa el contador.
    pub fn emitir(&mut self, o: Observacion);
    pub fn descartados(&self) -> u32;
    /// Cuantas veces se sujeto un instante que retrocedia (FR-013).
    pub fn retrocesos(&self) -> u32;
}

/// Extremo del consumidor.
pub struct Receptor { /* ... */ }
impl Receptor {
    pub fn recoger(&mut self, destino: &mut Vec<EventoCrudo>) -> usize;
    /// Duerme hasta que el productor avise. No sondea.
    pub fn esperar(&mut self);
}

/// Crea el par con capacidad fija reservada de una vez.
pub fn canal(capacidad: usize) -> (Emisor, Receptor);
```

**Garantía dura**: `emitir` es la única función de este contrato pensada para ejecutarse dentro del
callback de tiempo real. No asigna, no bloquea, no hace E/S y no puede entrar en pánico.

## Adaptador: `piano_midi_io`

```rust
/// Enumera los teclados disponibles.
pub fn dispositivos() -> Result<Vec<Dispositivo>, ErrorDeEntrada>;

/// Abre un dispositivo y empieza a capturar. El reloj es el de sesión (FR-012a).
pub fn abrir<C: Clock + Clone + Send + 'static>(
    dispositivo: &Dispositivo,
    clock: C,
) -> Result<Captura, ErrorDeEntrada>;

/// Reabre tras una reconexion. Es exactamente `abrir`: el reconocimiento por identidad ya
/// cubre que el sistema haya renumerado los puertos, que es la ventaja de no haber usado
/// nunca el indice de puerto como identidad.
pub fn reabrir<C: Clock + Send + 'static>(
    dispositivo: &Dispositivo,
    clock: C,
) -> Result<Captura, ErrorDeEntrada>;

pub struct Captura { /* ... */ }
impl Captura {
    /// El extremo de lectura, que implementa `FuenteDeEventos`.
    pub fn receptor(&mut self) -> &mut Receptor;
    /// Espera hasta `ventana` a que llegue algo, y dice si llego.
    ///
    /// Existe por un fallo documentado de Windows: tras reconectar, el puerto se abre con
    /// exito y nunca entrega un solo mensaje. Devolver `false` NO significa que este roto
    /// —puede que nadie este tocando—, pero impide dar por hecho que funciona.
    pub fn confirmar_actividad(&mut self, ventana: Duration) -> bool;
    /// Libera el dispositivo para otras aplicaciones (FR-006).
    pub fn cerrar(self);
}
```

## Vigilancia del dispositivo: `piano_midi_io::vigia`

El vigia informa de **hechos**, no de conclusiones. Quien decide si una ausencia es una
perdida es `SesionDeCaptura`, con su regla de doble confirmacion. La separacion no es
decorativa: permite probar la **decision** sin hardware y el **hecho** con hardware virtual,
que es lo que hace verificable toda la historia P3.

```rust
pub enum Presencia { Presente, Ausente }

pub struct Vigia { /* ... */ }
impl Vigia {
    /// Empieza a vigilar con el intervalo por defecto (un segundo).
    pub fn nuevo(objetivo: Dispositivo) -> Self;
    /// Igual, con intervalo a medida. Existe para que las pruebas no tarden segundos.
    pub fn con_intervalo(objetivo: Dispositivo, intervalo: Duration) -> Self;
    /// Lo ultimo observado, si hay algo nuevo. No bloquea.
    pub fn novedad(&mut self) -> Option<Presencia>;
}
```

**Contrato de `abrir`**: nunca entra en pánico, sea cual sea la secuencia de bytes que entregue el
dispositivo. Es el requisito que descalificó a `midir`, y por eso el análisis de mensajes es propio
y vive bajo `deny(clippy::indexing_slicing)`.

## Lo que este contrato NO ofrece

- No compara, evalúa ni puntúa nada (FR-025).
- No produce sonido (FR-027).
- No guarda lo capturado en disco (FR-026). Lo único que se persiste es qué teclado se eligió.
- No captura pedal ni ningún mensaje que no sea nota (FR-014).
