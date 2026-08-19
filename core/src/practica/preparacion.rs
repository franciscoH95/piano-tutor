//! Una canción lista para practicarla: quién toca cada nota y con qué dedo.
//!
//! Reúne lo que hay que rehacer junto. Están aquí y no sueltos porque dependen unos de
//! otros: el reparto de manos decide la digitación, así que tocar el corte sin rehacer los
//! dedos deja digitaciones de la mano contraria (FR-003c).

use crate::digitacion::{digitar, Dedo, Digitacion};
use crate::practica::cursor::{Ancla, Avance, Cursor, Paso, Velocidad};
use crate::evaluacion::{comparar, Evaluador, Nivel, Resultado, Veredicto};
use crate::practica::sonando::MascaraTeclas;
use crate::capture::{Observacion, TipoEvento};
use crate::practica::manos::{repartir, Mano, RepartoDeManos};
use crate::practica::nombres::NombreDeNota;
use crate::practica::vista::{vista, EstadoNota, Vista};
use crate::time::{Micros, Ticks};
use crate::Song;

/// Una nota lista para pintarse, con todo lo que hay que escribir junto a ella.
///
/// `NotaVisible` sólo trae el índice, porque la mano, el dedo y el nombre son constantes de
/// la canción y copiarlas en cada fotograma sería trabajo tirado. El cruce se hace aquí,
/// en el núcleo, y no en el puente: unir un índice con su anotación es una decisión del
/// dominio, y en `src-tauri` no habría pruebas que la cubriesen.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct NotaDetallada {
    /// Posición en `Song::notes`.
    pub indice: usize,
    /// Altura MIDI.
    pub key: u8,
    /// Instante del ataque, en microsegundos.
    pub onset_us: u64,
    /// Instante del final, en microsegundos.
    pub end_us: u64,
    /// Mano que la toca.
    pub mano: Mano,
    /// Dedo propuesto. Es una sugerencia, no una obligación (FR-006c).
    pub dedo: Dedo,
    /// Nombre en la armadura vigente. Símbolo, nunca una cadena.
    pub nombre: NombreDeNota,
    /// Situación respecto a lo tocado.
    pub estado: EstadoNota,
}

/// Canción cargada, repartida entre las dos manos y digitada.
pub struct Preparacion {
    cancion: Song,
    corte: u8,
    reparto: RepartoDeManos,
    digitacion: Digitacion,
    posicion: Micros,
    vista: Vista,
    cursor: Cursor,
    /// Que mano practica el alumno. `None` son las dos.
    practicada: Option<Mano>,
    nivel: Nivel,
    /// La interpretacion en curso, si la hay.
    ///
    /// Existe entre poner en marcha y parar: pausar, saltar y llegar al final la cierran, y
    /// reanudar abre otra (FR-014a). Es la misma frontera que el cursor ya usa para cambiar
    /// de regimen, asi que no hay concepto nuevo que inventar.
    evaluando: Option<Evaluador>,
    /// El resultado de la ultima interpretacion cerrada, con el tramo que abarco.
    ultimo: Option<Intento>,
    /// El anterior, para poder decir si se mejoro.
    anterior: Option<Intento>,
    /// Donde empezo la interpretacion en curso.
    desde: Micros,
}

/// Un intento cerrado, con el tramo de cancion que abarco.
struct Intento {
    resultado: Resultado,
    desde: Micros,
    hasta: Micros,
}

/// Como fue este intento respecto al anterior del mismo tramo.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Comparacion {
    /// Este intento fue mejor que el anterior.
    pub mejor: bool,
    /// O exactamente igual.
    pub igual: bool,
}

impl Preparacion {
    /// Do central. Es el corte por omisión mientras el usuario no diga otra cosa, y el
    /// valor al que vuelve cada canción nueva.
    pub const CORTE_POR_DEFECTO: u8 = 60;

    /// Prepara una canción desde cero.
    #[must_use]
    pub fn nueva(cancion: Song) -> Self {
        let corte = Self::CORTE_POR_DEFECTO;
        let (reparto, digitacion) = Self::repartir_y_digitar(&cancion, corte);
        let manos: Vec<Mano> = (0..reparto.len()).map(|i| reparto.mano(i)).collect();
        let cursor = Cursor::nuevo_con_puertas(&cancion, &manos, None);
        Self {
            corte,
            reparto,
            digitacion,
            cursor,
            practicada: None,
            nivel: Nivel::Intermedio,
            evaluando: None,
            ultimo: None,
            anterior: None,
            desde: Micros(0),
            cancion,
            posicion: Micros(0),
            vista: Vista::nueva(),
        }
    }

    /// Sustituye la canción por otra.
    ///
    /// Se reconstruye el valor **entero** en vez de ir poniendo campos a cero uno a uno.
    /// Así FR-005 se cumple por construcción: no hay forma de olvidar un campo, y cuando
    /// esta estructura crezca con el cursor, las puertas o las teclas hundidas, seguirá
    /// cumpliéndose sin tocar nada aquí.
    pub fn cargar(&mut self, cancion: Song) {
        *self = Self::nueva(cancion);
    }

    /// Mueve el punto de corte entre las dos manos y **rehace la digitación**.
    ///
    /// Si el archivo trae las voces separadas, el reparto manda y el corte se guarda pero
    /// no se aplica; el control sigue visible porque eso lo decide la interfaz.
    pub fn ajustar_corte(&mut self, corte: u8) {
        self.corte = corte;
        let (reparto, digitacion) = Self::repartir_y_digitar(&self.cancion, corte);
        self.reparto = reparto;
        self.digitacion = digitacion;
        // Y las puertas: el corte cambia de qué mano es cada nota, así que con una mano
        // practicada cambia qué puertas existen. Sin rehacerlas, el alumno esperaría en
        // notas que ya no son suyas.
        self.rehacer_puertas(self.posicion);
    }

    /// Coloca la posición de reproducción en un instante concreto.
    ///
    /// Hacia delante no hace falta recolocar nada: el cursor de la vista avanza solo, y es
    /// justo lo que hace que el coste por fotograma no dependa del tamaño de la canción.
    /// **Solo un salto hacia atrás** obliga a rehacer la búsqueda, que es `O(n)`.
    pub fn avanzar_a(&mut self, us: u64) {
        let atras = us < self.posicion.0;
        self.posicion = Micros(us);
        if atras {
            self.vista.reposicionar(&self.cancion, self.posicion);
        }
    }

    /// Vuelca en `out` las notas que caen en la ventana, ya cruzadas con su anotación.
    ///
    /// `out` se reutiliza entre fotogramas: la capa que pinta no asigna memoria por cuadro.
    pub fn detallar(&mut self, desde_us: u64, hasta_us: u64, out: &mut Vec<NotaDetallada>) {
        let mut visibles = Vec::new();
        vista(
            &self.cancion,
            &mut self.vista,
            self.posicion,
            Micros(desde_us),
            Micros(hasta_us),
            &mut visibles,
        );
        out.clear();
        let notas = self.cancion.notes();
        for v in &visibles {
            let indice = v.indice as usize;
            let tick = notas.get(indice).map_or(Ticks(0), |n| n.onset_tick);
            out.push(NotaDetallada {
                indice,
                key: v.key,
                onset_us: v.onset_us.0,
                end_us: v.end_us.0,
                mano: self.reparto.mano(indice),
                dedo: self.digitacion.dedo(indice),
                nombre: self.cancion.armaduras().nombre(tick, v.key),
                // **Un solo oráculo.** El veredicto lo decide el evaluador; la vista solo
                // lo pinta. Con dos sitios que decidan «acertada», el pentagrama y el
                // resumen discreparían en silencio.
                estado: match self.evaluando.as_ref().and_then(|e| e.veredicto_firme(indice)) {
                    Some(Veredicto::Acertada | Veredicto::TocadaFueraDeTiempo) => {
                        EstadoNota::Acertada
                    }
                    Some(Veredicto::Omitida) => EstadoNota::Omitida,
                    // Fuera de alcance y no intentada no son veredictos sobre el alumno, y
                    // mientras no haya veredicto manda lo que dice la canción.
                    _ => v.estado,
                },
            });
        }
    }

    /// Pone la canción en marcha. Devuelve ancla si cambió el régimen.
    ///
    /// **Abre una interpretación** (FR-014a).
    pub fn poner_en_marcha(&mut self, ahora: Micros) -> Option<Ancla> {
        let ancla = self.cursor.poner_en_marcha(ahora);
        if ancla.is_some() {
            self.abrir_interpretacion();
        }
        ancla
    }

    /// Detiene el avance sin perder la posición. **Cierra la interpretación.**
    pub fn pausar(&mut self, ahora: Micros) -> Option<Ancla> {
        let ancla = self.cursor.pausar(ahora);
        if ancla.is_some() {
            self.cerrar_interpretacion();
        }
        ancla
    }

    /// El resultado de la última interpretación cerrada, si la hay.
    #[must_use]
    pub fn resultado(&self) -> Option<&Resultado> {
        self.ultimo.as_ref().map(|i| &i.resultado)
    }

    /// Cómo fue el último intento respecto al anterior **del mismo tramo**.
    ///
    /// `None` si no hay anterior, o si el anterior era de otro tramo: comparar el compás 1
    /// con el compás 9 no dice nada útil, y decirlo igualmente sería peor que callarse.
    #[must_use]
    pub fn comparacion(&self) -> Option<Comparacion> {
        let (ultimo, anterior) = (self.ultimo.as_ref()?, self.anterior.as_ref()?);
        if ultimo.desde != anterior.desde || ultimo.hasta != anterior.hasta {
            return None;
        }
        let orden = comparar(&ultimo.resultado, &anterior.resultado);
        Some(Comparacion {
            mejor: orden == core::cmp::Ordering::Greater,
            igual: orden == core::cmp::Ordering::Equal,
        })
    }

    /// Cuánta exigencia. Afecta a la interpretación siguiente.
    pub fn cambiar_nivel(&mut self, nivel: Nivel) {
        self.nivel = nivel;
    }

    /// Una tecla que el alumno pulsó o soltó.
    pub fn observar_tecla(&mut self, key: u8, pulsada: bool, ahora: Micros) {
        if let Some(e) = self.evaluando.as_mut() {
            e.observar(Observacion {
                at: ahora,
                key,
                velocity: if pulsada { 90 } else { 0 },
                kind: if pulsada { TipoEvento::Ataque } else { TipoEvento::Suelta },
                channel: 0,
            });
        }
    }

    fn abrir_interpretacion(&mut self) {
        self.desde = self.posicion;
        let manos: Vec<Mano> = (0..self.reparto.len()).map(|i| self.reparto.mano(i)).collect();
        let mut e = Evaluador::nuevo(&self.cancion, &manos, self.practicada, self.nivel);
        e.evaluar_tiempos(self.cursor.avance() == Avance::PorReloj);
        // Traduce las posiciones de canción al eje del reloj, que es donde llegan las
        // pulsaciones. Sin esto, al repetir un pasaje el reloj ya no está en cero y el
        // emparejamiento compararía peras con manzanas.
        e.sellar(&self.cursor.ancla());
        self.evaluando = Some(e);
    }

    fn cerrar_interpretacion(&mut self) {
        if let Some(e) = self.evaluando.take() {
            let intento =
                Intento { resultado: e.cerrar(self.posicion), desde: self.desde, hasta: self.posicion };
            self.anterior = self.ultimo.take();
            self.ultimo = Some(intento);
        }
    }

    /// Cambia la velocidad de práctica.
    pub fn cambiar_velocidad(&mut self, v: Velocidad, ahora: Micros) -> Option<Ancla> {
        self.cursor.cambiar_velocidad(v, ahora)
    }

    /// Lleva la práctica a una posición concreta.
    pub fn saltar_a(&mut self, posicion: Micros, ahora: Micros) -> Option<Ancla> {
        let ancla = self.cursor.saltar_a(posicion, ahora);
        if ancla.is_some() {
            self.cerrar_interpretacion();
        }
        self.avanzar_a(self.cursor.posicion().0);
        ancla
    }

    /// Adelanta la práctica hasta el instante del reloj.
    pub fn avanzar(&mut self, ahora: Micros) -> Paso {
        self.avanzar_con(ahora, MascaraTeclas::VACIA)
    }

    /// Adelanta la práctica sabiendo qué teclas tiene pulsadas el alumno.
    pub fn avanzar_con(&mut self, ahora: Micros, pulsadas: MascaraTeclas) -> Paso {
        let paso = self.cursor.avanzar_con(ahora, pulsadas);
        self.avanzar_a(paso.posicion.0);
        if let Some(e) = self.evaluando.as_mut() {
            // Al cambiar el régimen cambia de verdad el instante esperado de lo que aún no
            // ha sonado, así que se vuelve a sellar. Lo ya juzgado no se toca (FR-004).
            if paso.ancla.is_some() {
                e.sellar(&self.cursor.ancla());
            }
            e.avanzar(ahora);
        }
        // Llegar al final tambien cierra la interpretacion (FR-014a).
        if paso.terminada {
            self.cerrar_interpretacion();
        }
        paso
    }

    /// Cambia entre reproducir y esperar, conservando la posición (FR-021).
    pub fn cambiar_avance(&mut self, avance: Avance, ahora: Micros) -> Option<Ancla> {
        let ancla = self.cursor.cambiar_avance(avance, ahora);
        // El regimen manda POR NOTA: las que se emparejen a partir de ahora se juzgan con
        // el nuevo, y las ya juzgadas no se tocan (FR-004).
        if let Some(e) = self.evaluando.as_mut() {
            e.evaluar_tiempos(avance == Avance::PorReloj);
        }
        ancla
    }

    /// Elige qué mano se practica. `None` son las dos.
    pub fn practicar_mano(&mut self, mano: Option<Mano>, ahora: Micros) -> Option<Ancla> {
        self.practicada = mano;
        self.rehacer_puertas(ahora)
    }

    /// Salta la puerta pendiente sin acertarla (FR-020).
    pub fn saltar_puerta(&mut self, ahora: Micros) -> Option<Ancla> {
        let antes = self.cursor.posicion();
        let ancla = self.cursor.saltar_puerta(ahora);
        if ancla.is_some() {
            // Lo saltado no se intento, asi que no cuenta como fallado (FR-013).
            if let Some(e) = self.evaluando.as_mut() {
                e.saltar(antes, self.cursor.posicion());
            }
        }
        ancla
    }

    /// El modo de avance vigente.
    #[must_use]
    pub const fn avance(&self) -> Avance {
        self.cursor.avance()
    }

    /// Las teclas que hay que pulsar para seguir, si el cursor espera.
    #[must_use]
    pub fn pendiente(&self) -> Option<MascaraTeclas> {
        self.cursor.pendiente()
    }

    fn rehacer_puertas(&mut self, ahora: Micros) -> Option<Ancla> {
        let manos: Vec<Mano> = (0..self.reparto.len()).map(|i| self.reparto.mano(i)).collect();
        self.cursor
            .practicar_mano(&self.cancion, &manos, self.practicada, ahora)
    }

    /// El ancla vigente, la que interpola la pantalla.
    #[must_use]
    pub fn ancla(&self) -> Ancla {
        self.cursor.ancla()
    }

    fn repartir_y_digitar(cancion: &Song, corte: u8) -> (RepartoDeManos, Digitacion) {
        let reparto = repartir(cancion, corte);
        let manos: Vec<Mano> = (0..reparto.len()).map(|i| reparto.mano(i)).collect();
        let digitacion = digitar(cancion, &manos);
        (reparto, digitacion)
    }

    /// La canción cargada.
    #[must_use]
    pub const fn cancion(&self) -> &Song {
        &self.cancion
    }

    /// Punto de corte entre manos vigente, aunque el archivo traiga las voces.
    #[must_use]
    pub const fn corte(&self) -> u8 {
        self.corte
    }

    /// Posición de reproducción, en microsegundos desde el principio.
    #[must_use]
    pub const fn posicion(&self) -> u64 {
        self.posicion.0
    }

    /// De qué mano es cada nota.
    #[must_use]
    pub const fn reparto(&self) -> &RepartoDeManos {
        &self.reparto
    }

    /// Qué dedo se propone para cada nota.
    #[must_use]
    pub const fn digitacion(&self) -> &Digitacion {
        &self.digitacion
    }

    /// Estado del recorrido de la línea temporal para pintar.
    #[must_use]
    pub const fn vista(&self) -> &Vista {
        &self.vista
    }
}
