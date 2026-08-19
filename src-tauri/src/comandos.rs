//! El puente entre el nucleo y la interfaz.
//!
//! Aqui **no se toma ninguna decision musical**. Se traducen llamadas y se reenvian datos.
//! Si en este archivo aparece un `if` sobre notas, tempos o manos, esa logica pertenece a
//! `piano-core`, donde si esta cubierta por pruebas.

use piano_core::clock::Clock;
use piano_core::evaluacion::Nivel;
use piano_core::practica::{
    Alteracion, Ancla, Avance, Base, Mano, NotaDetallada, Preparacion, Reparto, Velocidad,
};
use piano_core::time::Micros;
use serde::Serialize;
use std::sync::Mutex;
use tauri::ipc::Channel;

/// Lo que el nucleo empuja hacia la interfaz.
///
/// **Un solo canal** para todo, discriminado por etiqueta: asi el orden entre las teclas y
/// las anclas queda garantizado por construccion. Con dos canales no lo estaria.
#[derive(Clone, Debug, Serialize)]
// `rename_all` sobre un enum renombra **las variantes**, no los campos de dentro; hace
// falta `rename_all_fields` para eso. Sin el, este canal enviaba `posicion_us` mientras
// `AnclaPlana` —que devuelven los mandos— enviaba `posicionUs`: dos convenciones para el
// mismo dato en el mismo puente, y ninguna de las dos falla ruidosamente. El campo llegaria
// como `undefined` y el cursor se quedaria clavado sin ningun error.
#[serde(tag = "tipo", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum MensajeAlFrontend {
    /// Una tecla se pulso o se solto.
    Tecla { key: u8, pulsada: bool },
    /// Cambio el regimen del cursor. **No se envia sesenta veces por segundo**: la
    /// interfaz interpola entre anclas, y eso es lo que mantiene el puente casi vacio.
    Ancla {
        posicion_us: u64,
        instante_us: u64,
        num: u32,
        den: u32,
        tope_us: Option<u64>,
    },
    /// El cursor espera a que se toquen **estas** notas, todas a la vez.
    ///
    /// El contrato original traia una sola tecla. No basta: una puerta puede ser un acorde
    /// y FR-022 exige que esten todas pulsadas simultaneamente, asi que con una sola el
    /// alumno no podria ver que le falta.
    Esperando { teclas: Vec<u8> },
    /// La cancion llego a su fin.
    Terminada,
    /// El teclado desaparecio a mitad de practica.
    DispositivoPerdido,
}

/// Lo que la interfaz necesita saber de una cancion recien abierta.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResumenCancion {
    /// Numero de notas.
    pub notas: usize,
    /// Duracion total en microsegundos.
    pub duracion_us: u64,
    /// Si el archivo traia las dos manos separadas. La interfaz lo usa para rotular el
    /// control del corte, que sigue visible en ambos casos.
    pub voces_del_archivo: bool,
    /// Punto de corte vigente.
    pub corte: u8,
}

/// Una nota lista para dibujar, aplanada para cruzar el puente.
///
/// El nombre viaja ya separado en base y alteracion, que es como lo produce el nucleo. No
/// se formatea aqui: el texto es cosa de quien pinta.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NotaVisiblePlana {
    pub indice: usize,
    pub key: u8,
    pub onset_us: u64,
    pub end_us: u64,
    /// `true` si es de la mano derecha.
    pub derecha: bool,
    /// Dedo propuesto, de 1 a 5.
    pub dedo: u8,
    /// Letra de la nota: 0 = Do, 1 = Re, ... 6 = Si.
    pub base: u8,
    /// -1 bemol, 0 natural, 1 sostenido.
    pub alteracion: i8,
    /// Situacion respecto a lo tocado.
    pub estado: &'static str,
}

impl From<&NotaDetallada> for NotaVisiblePlana {
    fn from(d: &NotaDetallada) -> Self {
        Self {
            indice: d.indice,
            key: d.key,
            onset_us: d.onset_us,
            end_us: d.end_us,
            derecha: d.mano == piano_core::practica::Mano::Derecha,
            dedo: d.dedo.numero(),
            // Mapeo explicito y no `as`: un `as` sobre el enum ata este contrato al
            // ORDEN de las variantes, y reordenarlas cambiaria en silencio lo que ve la
            // interfaz. Con `Bemol` en tercera posicion, `as i8` daba 2 en vez de -1.
            base: match d.nombre.base {
                Base::Do => 0,
                Base::Re => 1,
                Base::Mi => 2,
                Base::Fa => 3,
                Base::Sol => 4,
                Base::La => 5,
                Base::Si => 6,
            },
            alteracion: match d.nombre.alteracion {
                Alteracion::Bemol => -1,
                Alteracion::Ninguna => 0,
                Alteracion::Sostenido => 1,
            },
            estado: match d.estado {
                piano_core::practica::EstadoNota::Pendiente => "pendiente",
                piano_core::practica::EstadoNota::Sonando => "sonando",
                piano_core::practica::EstadoNota::Acertada => "acertada",
                piano_core::practica::EstadoNota::Omitida => "omitida",
            },
        }
    }
}

/// Estado compartido de la aplicacion.
#[derive(Default)]
pub struct Estado {
    canal: Mutex<Option<Channel<MensajeAlFrontend>>>,
    preparacion: Mutex<Option<Preparacion>>,
    /// Se levanta al cerrar para que el hilo de captura salga de su bucle.
    pub parar: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

impl Estado {
    /// Empuja un mensaje si hay canal registrado.
    ///
    /// **Nunca se llama desde el hilo de tiempo real.** `send` cuesta hasta 13 ms en el
    /// peor caso, y eso dentro de la ruta critica arruinaria el presupuesto del Principio
    /// IV que la feature 002 dejo cerrado. Quien llama aqui es el hilo reenviador.
    pub fn enviar(&self, mensaje: MensajeAlFrontend) {
        let guarda = match self.canal.lock() {
            Ok(g) => g,
            Err(envenenado) => envenenado.into_inner(),
        };
        if let Some(canal) = guarda.as_ref() {
            let _ = canal.send(mensaje);
        }
    }
}

/// La interfaz registra su canal al arrancar la practica.
#[tauri::command]
pub fn registrar_canal(estado: tauri::State<'_, Estado>, canal: Channel<MensajeAlFrontend>) {
    let mut guarda = match estado.canal.lock() {
        Ok(g) => g,
        Err(envenenado) => envenenado.into_inner(),
    };
    *guarda = Some(canal);
}

/// Abre una cancion del disco.
///
/// **Leer del disco es cosa de esta capa**, no del nucleo, que sigue recibiendo `&[u8]` y
/// por eso se puede probar sin tocar el sistema de archivos (Principio III).
///
/// El motivo del fallo se devuelve tal cual lo da el nucleo: la interfaz lo muestra sin
/// interpretarlo (FR-004).
#[tauri::command]
pub fn abrir_cancion(
    estado: tauri::State<'_, std::sync::Arc<Estado>>,
    ruta: String,
) -> Result<ResumenCancion, String> {
    let bytes = std::fs::read(&ruta).map_err(|e| format!("No se pudo leer el archivo: {e}"))?;
    let cancion = piano_core::load_smf(&bytes).map_err(|e| e.to_string())?;
    let preparacion = Preparacion::nueva(cancion);
    let resumen = ResumenCancion {
        notas: preparacion.cancion().notes().len(),
        duracion_us: preparacion.cancion().duration_us().0,
        voces_del_archivo: preparacion.reparto().origen() == Reparto::VocesDelArchivo,
        corte: preparacion.corte(),
    };
    let mut guarda = match estado.preparacion.lock() {
        Ok(g) => g,
        Err(envenenado) => envenenado.into_inner(),
    };
    // Se sustituye entera: cargar otra cancion no arrastra nada de la anterior (FR-005).
    *guarda = Some(preparacion);
    Ok(resumen)
}

/// Mueve el punto de corte entre manos. Rehace manos **y** digitacion (FR-003c).
#[tauri::command]
pub fn ajustar_corte(estado: tauri::State<'_, std::sync::Arc<Estado>>, corte: u8) {
    let mut guarda = match estado.preparacion.lock() {
        Ok(g) => g,
        Err(envenenado) => envenenado.into_inner(),
    };
    if let Some(p) = guarda.as_mut() {
        p.ajustar_corte(corte);
    }
}

/// Las notas que caen en la ventana pedida, listas para dibujar.
#[tauri::command]
pub fn vista_actual(
    estado: tauri::State<'_, std::sync::Arc<Estado>>,
    desde_us: u64,
    hasta_us: u64,
) -> Vec<NotaVisiblePlana> {
    let mut guarda = match estado.preparacion.lock() {
        Ok(g) => g,
        Err(envenenado) => envenenado.into_inner(),
    };
    let Some(p) = guarda.as_mut() else {
        return Vec::new();
    };
    let mut detalle = Vec::new();
    p.detallar(desde_us, hasta_us, &mut detalle);
    detalle.iter().map(NotaVisiblePlana::from).collect()
}

/// Un ancla aplanada para cruzar el puente como respuesta de un mando.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AnclaPlana {
    pub posicion_us: u64,
    /// Instante del **reloj de sesion de Rust**. La interfaz NO puede compararlo con su
    /// propio reloj: lo sustituye por su lectura local al recibirlo (`anclarEnRelojLocal`).
    pub instante_us: u64,
    pub num: u32,
    pub den: u32,
    pub tope_us: Option<u64>,
}

impl From<Ancla> for AnclaPlana {
    fn from(a: Ancla) -> Self {
        Self {
            posicion_us: a.posicion_us.0,
            instante_us: a.instante_us.0,
            num: a.num,
            den: a.den,
            tope_us: a.tope_us.map(|t| t.0),
        }
    }
}

/// Traduce un ancla del nucleo al mensaje que cruza el puente.
fn mensaje_de(a: Ancla) -> MensajeAlFrontend {
    MensajeAlFrontend::Ancla {
        posicion_us: a.posicion_us.0,
        instante_us: a.instante_us.0,
        num: a.num,
        den: a.den,
        tope_us: a.tope_us.map(|t| t.0),
    }
}

/// Aplica una operacion de transporte y empuja el ancla si cambio el regimen.
///
/// **El ancla solo cruza cuando cambia el regimen**, nunca en cada fotograma: es lo que
/// mantiene el puente casi vacio. La interfaz interpola entre anclas.
fn transportar<F>(
    estado: &std::sync::Arc<Estado>,
    reloj: &crate::RelojDeSesion,
    operacion: F,
) -> Option<AnclaPlana>
where
    F: FnOnce(&mut Preparacion, Micros) -> Option<Ancla>,
{
    let ahora = reloj.0.now();
    let mut guarda = match estado.preparacion.lock() {
        Ok(g) => g,
        Err(envenenado) => envenenado.into_inner(),
    };
    let ancla = operacion(guarda.as_mut()?, ahora)?;
    // Se suelta el candado antes de enviar: `send` cuesta hasta 13 ms en el peor caso y no
    // puede quedarse con el estado bloqueado mientras tanto.
    drop(guarda);
    estado.enviar(mensaje_de(ancla));
    Some(ancla.into())
}

/// Pone la cancion en marcha desde donde este.
#[tauri::command]
pub fn transporte_marcha(
    estado: tauri::State<'_, std::sync::Arc<Estado>>,
    reloj: tauri::State<'_, crate::RelojDeSesion>,
) -> Option<AnclaPlana> {
    transportar(&estado, &reloj, Preparacion::poner_en_marcha)
}

/// Detiene el avance sin perder la posicion.
#[tauri::command]
pub fn transporte_pausa(
    estado: tauri::State<'_, std::sync::Arc<Estado>>,
    reloj: tauri::State<'_, crate::RelojDeSesion>,
) -> Option<AnclaPlana> {
    transportar(&estado, &reloj, Preparacion::pausar)
}

/// Lleva el cursor a una posicion concreta.
#[tauri::command]
pub fn transporte_saltar(
    estado: tauri::State<'_, std::sync::Arc<Estado>>,
    reloj: tauri::State<'_, crate::RelojDeSesion>,
    posicion_us: u64,
) -> Option<AnclaPlana> {
    transportar(&estado, &reloj, |p, ahora| p.saltar_a(Micros(posicion_us), ahora))
}

/// Cambia la velocidad. Llega como **racional**, no como decimal.
///
/// Un denominador cero se rechaza sin tocar nada: `Velocidad::nueva` devuelve `None` y
/// aqui no hay decision que tomar, solo la traduccion.
#[tauri::command]
pub fn transporte_velocidad(
    estado: tauri::State<'_, std::sync::Arc<Estado>>,
    reloj: tauri::State<'_, crate::RelojDeSesion>,
    num: u32,
    den: u32,
) -> Option<AnclaPlana> {
    // Un denominador cero se rechaza sin tocar nada. Aqui no hay decision, solo traduccion.
    let v = Velocidad::nueva(num, den)?;
    transportar(&estado, &reloj, |p, ahora| p.cambiar_velocidad(v, ahora))
}

/// Adelanta la practica hasta el instante actual y devuelve donde esta.
///
/// La interfaz **no** necesita llamar a esto en cada fotograma: interpola desde el ancla.
/// Existe para el arranque y para despues de un salto.
#[tauri::command]
pub fn transporte_posicion(
    estado: tauri::State<'_, std::sync::Arc<Estado>>,
    reloj: tauri::State<'_, crate::RelojDeSesion>,
) -> u64 {
    let ahora = reloj.0.now();
    let mut guarda = match estado.preparacion.lock() {
        Ok(g) => g,
        Err(envenenado) => envenenado.into_inner(),
    };
    guarda.as_mut().map_or(0, |p| p.avanzar(ahora).posicion.0)
}

/// Un dispositivo tal como lo ve la interfaz.
///
/// Lleva la **posicion** ademas del nombre porque dos teclados del mismo modelo se llaman
/// igual, y sin ella el alumno no podria distinguirlos en la lista.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DispositivoPlano {
    pub nombre: String,
    pub posicion: u16,
    pub id_sistema: Option<u64>,
}

impl From<&piano_core::capture::Dispositivo> for DispositivoPlano {
    fn from(d: &piano_core::capture::Dispositivo) -> Self {
        Self {
            nombre: d.nombre.clone(),
            posicion: d.posicion,
            id_sistema: d.id_sistema.map(|i| i.0),
        }
    }
}

/// En que situacion esta el teclado al arrancar.
#[derive(Clone, Debug, Serialize)]
#[serde(tag = "tipo", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum EstadoDelTeclado {
    /// Se reconocio el recordado y ya esta capturando.
    Conectado { nombre: String },
    /// Hay teclados, pero **ninguno se abre solo**: o no habia nada recordado, o el
    /// recordado no esta. FR-025 lo prohibe expresamente.
    HayQueElegir { dispositivos: Vec<DispositivoPlano> },
    /// No hay ningun teclado enchufado.
    SinDispositivos,
}

/// Donde se recuerda la eleccion.
fn ruta_preferencias() -> std::path::PathBuf {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(std::path::PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| std::path::PathBuf::from(h).join(".config")))
        .unwrap_or_else(std::env::temp_dir);
    base.join("piano-tutor").join("teclado.json")
}

/// Enumera los teclados MIDI disponibles.
#[tauri::command]
pub fn listar_dispositivos() -> Vec<DispositivoPlano> {
    piano_midi_io::dispositivos()
        .unwrap_or_default()
        .iter()
        .map(DispositivoPlano::from)
        .collect()
}

/// Arranca la captura con el teclado recordado, si se reconoce.
///
/// # Por que no abre "el primero que haya"
///
/// FR-025 lo prohibe: si el teclado recordado no esta, hay que **pedir que se elija de
/// nuevo** y no abrir otro en su lugar. Abrir el primero seria capturar de un aparato que
/// el alumno no eligio —otro teclado de la casa, un modulo de sonido— y lo notaria porque
/// nada respondería, sin ninguna pista de por que.
///
/// El reconocimiento por identidad ya existe y esta probado en la feature 002: aqui solo
/// se usa.
#[tauri::command]
pub fn conectar_teclado(
    estado: tauri::State<'_, std::sync::Arc<Estado>>,
    reloj: tauri::State<'_, crate::RelojDeSesion>,
) -> EstadoDelTeclado {
    let disponibles = piano_midi_io::dispositivos().unwrap_or_default();
    if disponibles.is_empty() {
        return EstadoDelTeclado::SinDispositivos;
    }
    let pedir = || EstadoDelTeclado::HayQueElegir {
        dispositivos: disponibles.iter().map(DispositivoPlano::from).collect(),
    };
    let Some(recordado) = crate::preferencias::cargar(&ruta_preferencias()) else {
        return pedir();
    };
    let buscado = piano_core::capture::Dispositivo::from(&recordado);
    match piano_core::capture::reconocer(&buscado, &disponibles) {
        piano_core::capture::Reconocimiento::Encontrado(i) => match disponibles.get(i) {
            Some(d) => arrancar_captura(&estado, &reloj, d.clone()),
            None => pedir(),
        },
        piano_core::capture::Reconocimiento::PedirAlUsuario => pedir(),
    }
}

/// El alumno elige un teclado de la lista. Se recuerda y se abre.
#[tauri::command]
pub fn elegir_teclado(
    estado: tauri::State<'_, std::sync::Arc<Estado>>,
    reloj: tauri::State<'_, crate::RelojDeSesion>,
    posicion: u16,
    nombre: String,
) -> EstadoDelTeclado {
    let disponibles = piano_midi_io::dispositivos().unwrap_or_default();
    let Some(d) = disponibles
        .iter()
        .find(|d| d.posicion == posicion && d.nombre == nombre)
    else {
        return EstadoDelTeclado::HayQueElegir {
            dispositivos: disponibles.iter().map(DispositivoPlano::from).collect(),
        };
    };
    // Guardar puede fallar; no es motivo para no practicar. Como mucho, la proxima vez
    // habra que volver a elegir.
    let _ = crate::preferencias::guardar(&ruta_preferencias(), &(d.into()));
    arrancar_captura(&estado, &reloj, d.clone())
}

/// Lanza el hilo de captura sobre un dispositivo concreto.
///
/// `Captura` retiene el puerto y el cliente de CoreMIDI, que no son `Send`, asi que se
/// abre **dentro** del hilo. Se le pasa el reloj de sesion, el mismo que gobierna la
/// reproduccion (FR-012a): `MonotonicClock` es `Copy` y guarda su origen, asi que copiarlo
/// no crea un reloj nuevo.
fn arrancar_captura(
    estado: &std::sync::Arc<Estado>,
    reloj: &crate::RelojDeSesion,
    dispositivo: piano_core::capture::Dispositivo,
) -> EstadoDelTeclado {
    let nombre = dispositivo.nombre.clone();
    let clock = reloj.0;
    let compartido = std::sync::Arc::clone(estado);
    let parar = std::sync::Arc::clone(&estado.parar);
    std::thread::spawn(move || match piano_midi_io::abrir(&dispositivo, clock) {
        Ok(mut captura) => crate::reenviador::bucle(captura.receptor(), &compartido, &parar),
        // Estaba en la lista y no se pudo abrir: para el alumno es lo mismo que perderlo.
        Err(_) => compartido.enviar(MensajeAlFrontend::DispositivoPerdido),
    });
    EstadoDelTeclado::Conectado { nombre }
}

/// Cambia entre reproducir y esperar. Conserva la posicion (FR-021).
#[tauri::command]
pub fn transporte_modo(
    estado: tauri::State<'_, std::sync::Arc<Estado>>,
    reloj: tauri::State<'_, crate::RelojDeSesion>,
    espera: bool,
) -> Option<AnclaPlana> {
    let avance = if espera { Avance::PorAcierto } else { Avance::PorReloj };
    transportar(&estado, &reloj, |p, ahora| p.cambiar_avance(avance, ahora))
}

/// Elige que mano se practica. `None` son las dos.
#[tauri::command]
pub fn ajustar_mano(
    estado: tauri::State<'_, std::sync::Arc<Estado>>,
    reloj: tauri::State<'_, crate::RelojDeSesion>,
    mano: Option<String>,
) -> Option<AnclaPlana> {
    let elegida = match mano.as_deref() {
        Some("izquierda") => Some(Mano::Izquierda),
        Some("derecha") => Some(Mano::Derecha),
        _ => None,
    };
    transportar(&estado, &reloj, |p, ahora| p.practicar_mano(elegida, ahora))
}

/// Salta la nota pendiente sin acertarla (FR-020).
#[tauri::command]
pub fn transporte_saltar_puerta(
    estado: tauri::State<'_, std::sync::Arc<Estado>>,
    reloj: tauri::State<'_, crate::RelojDeSesion>,
) -> Option<AnclaPlana> {
    transportar(&estado, &reloj, Preparacion::saltar_puerta)
}

/// El resumen de una interpretacion, aplanado para cruzar el puente.
///
/// **Ninguna tolerancia cruza**, ni siquiera para mostrarla: si la interfaz supiera lo que
/// es una ventana de 60 ms, esa constante estaria en dos sitios y el Principio I lo prohibe.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResultadoPlano {
    pub acertadas: usize,
    pub fuera_de_tiempo: usize,
    pub omitidas: usize,
    pub de_mas: usize,
    pub dedos_escapados: usize,
    pub fuera_de_alcance: usize,
    pub no_intentadas: usize,
    /// El denominador honesto: lo que se le pidio de verdad al alumno (SC-009).
    pub intentadas: usize,
    /// Con signo: negativo se adelanta, positivo se atrasa. `null` si no hay desfase.
    pub desfase_mediana_us: Option<i64>,
    pub desfase_dispersion_us: Option<u64>,
    /// No se toco ni una tecla. Distinto de tocarlo todo mal (FR-019).
    pub sin_tocar: bool,
    /// Los tiempos NO se evaluaron. **Hay que decirlo** (FR-015a).
    pub parcial: bool,
    /// Recuento de cada mano (FR-018).
    pub por_mano: PorMano,
}

/// El recuento de las dos manos.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PorMano {
    pub izquierda: RecuentoPlano,
    pub derecha: RecuentoPlano,
}

/// Lo que le paso a una mano.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecuentoPlano {
    pub acertadas: usize,
    pub fuera_de_tiempo: usize,
    pub omitidas: usize,
}

impl From<piano_core::evaluacion::Recuento> for RecuentoPlano {
    fn from(r: piano_core::evaluacion::Recuento) -> Self {
        Self { acertadas: r.acertadas, fuera_de_tiempo: r.fuera_de_tiempo, omitidas: r.omitidas }
    }
}

impl From<&piano_core::evaluacion::Resultado> for ResultadoPlano {
    fn from(r: &piano_core::evaluacion::Resultado) -> Self {
        Self {
            acertadas: r.acertadas,
            fuera_de_tiempo: r.fuera_de_tiempo,
            omitidas: r.omitidas,
            de_mas: r.de_mas,
            dedos_escapados: r.dedos_escapados,
            fuera_de_alcance: r.fuera_de_alcance,
            no_intentadas: r.no_intentadas,
            intentadas: r.intentadas(),
            desfase_mediana_us: r.desfase.map(|d| d.mediana_us),
            desfase_dispersion_us: r.desfase.map(|d| d.dispersion_us),
            sin_tocar: r.sin_tocar,
            parcial: r.parcial,
            por_mano: PorMano {
                izquierda: r.por_mano[0].into(),
                derecha: r.por_mano[1].into(),
            },
        }
    }
}

/// El resumen de la ultima interpretacion cerrada, si la hay.
#[tauri::command]
pub fn evaluacion_ultimo(
    estado: tauri::State<'_, std::sync::Arc<Estado>>,
) -> Option<ResultadoPlano> {
    let guarda = match estado.preparacion.lock() {
        Ok(g) => g,
        Err(envenenado) => envenenado.into_inner(),
    };
    guarda.as_ref()?.resultado().map(ResultadoPlano::from)
}

/// Cuanta exigencia. Afecta a la interpretacion siguiente.
#[tauri::command]
pub fn evaluacion_nivel(estado: tauri::State<'_, std::sync::Arc<Estado>>, nivel: String) {
    let elegido = match nivel.as_str() {
        "permisivo" => Nivel::Permisivo,
        "exigente" => Nivel::Exigente,
        _ => Nivel::Intermedio,
    };
    let mut guarda = match estado.preparacion.lock() {
        Ok(g) => g,
        Err(envenenado) => envenenado.into_inner(),
    };
    if let Some(p) = guarda.as_mut() {
        p.cambiar_nivel(elegido);
    }
}
