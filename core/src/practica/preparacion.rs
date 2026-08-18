//! Una canción lista para practicarla: quién toca cada nota y con qué dedo.
//!
//! Reúne lo que hay que rehacer junto. Están aquí y no sueltos porque dependen unos de
//! otros: el reparto de manos decide la digitación, así que tocar el corte sin rehacer los
//! dedos deja digitaciones de la mano contraria (FR-003c).

use crate::digitacion::{digitar, Dedo, Digitacion};
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
        Self {
            cancion,
            corte,
            reparto,
            digitacion,
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
    }

    /// Coloca la posición de reproducción en un instante concreto.
    pub fn avanzar_a(&mut self, us: u64) {
        self.posicion = Micros(us);
        self.vista.reposicionar(&self.cancion, self.posicion);
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
                estado: v.estado,
            });
        }
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
