//! Que notas hay que dibujar en un instante dado.
//!
//! # Por que el cursor es monotono
//!
//! Una pieza de diez minutos son miles de notas, pero en pantalla caben unas decenas.
//! Buscarlas recorriendo la lista entera en cada fotograma seria cuadratico. Como la
//! linea temporal ya viene ordenada por ataque, basta un cursor que avanza con la
//! reproduccion: cada nota se examina un numero acotado de veces en toda la pieza, no una
//! vez por fotograma.
//!
//! La lista esta ordenada por **ataque**, no por final, asi que una nota larga puede
//! empezar antes de la ventana y seguir sonando dentro. Por eso el cursor se retrasa hasta
//! cubrir la nota mas larga vista hasta el momento, en lugar de saltar al primer ataque
//! visible.
//!
//! # Que NO lleva
//!
//! Ni mano, ni dedo, ni nombre. Esas anotaciones son **constantes de la cancion**, no
//! datos de fotograma: se calculan una vez al cargar y se localizan por `indice`. Copiarlas
//! sesenta veces por segundo seria trabajo inutil.

use crate::time::Micros;
use crate::Song;

/// En que situacion esta una nota respecto a lo que el alumno ha tocado.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum EstadoNota {
    /// Todavia no le toca.
    Pendiente,
    /// Esta sonando ahora mismo en la cancion.
    Sonando,
    /// El alumno la toco mientras sonaba.
    Acertada,
    /// Sono entera y el alumno no la toco.
    Omitida,
}

/// Una nota que entra en la ventana visible.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct NotaVisible {
    /// Posicion en `Song::notes`, para recuperar sus anotaciones.
    pub indice: u32,
    /// Altura MIDI.
    pub key: u8,
    /// Instante del ataque.
    pub onset_us: Micros,
    /// Instante del final.
    pub end_us: Micros,
    /// Situacion respecto a lo tocado.
    pub estado: EstadoNota,
}

/// El estado que hace barato repetir la consulta fotograma tras fotograma.
#[derive(Clone, Debug, Default)]
pub struct Vista {
    /// Primera nota que puede seguir siendo visible. Solo avanza.
    cursor: usize,
    /// Duracion de la nota mas larga vista hasta ahora, para saber cuanto retrasar el
    /// cursor sin perder notas largas que empezaron antes de la ventana.
    max_duracion: u64,
    /// Cuantas notas se han examinado en total. Diagnostico: es lo que permite afirmar
    /// la garantia de coste **contando**, en lugar de cronometrando, que seria
    /// intermitente y no demostraria nada estructural.
    examinadas: usize,
}

impl Vista {
    /// Una vista al principio de la cancion.
    #[must_use]
    pub const fn nueva() -> Self {
        Self { cursor: 0, max_duracion: 0, examinadas: 0 }
    }

    /// Cuantas notas se han examinado desde que se creo.
    #[must_use]
    pub const fn examinadas(&self) -> usize {
        self.examinadas
    }

    /// Recoloca la vista tras un salto **hacia atras**. Coste `O(n)`.
    ///
    /// # Por que no se busca por el final de la nota
    ///
    /// `Song::notes` esta ordenado por **ataque**, no por final. `partition_point` exige un
    /// predicado *monotono* sobre la ordenacion, y `end_us < objetivo` no lo es: un pedal
    /// largo que empieza pronto termina despues de notas cortas posteriores. Con ese
    /// predicado la busqueda devuelve un indice cualquiera y el cursor se coloca por
    /// delante de una nota que todavia suena; al saltar al medio de un pedal, el bajo que
    /// sostiene la armonia simplemente no aparece.
    ///
    /// El predicado que si es monotono es sobre el ataque. Se descarta una nota solo si
    /// **con certeza** ha terminado: `ataque + duracion_maxima < objetivo`. Como ninguna
    /// nota dura mas que la maxima, eso no puede dejar fuera nada que suene.
    ///
    /// # Por que la maxima se recalcula aqui
    ///
    /// `max_duracion` se aprende recorriendo la cancion, asi que en una vista recien creada
    /// vale cero y no acota nada. Un salto no puede fiarse de lo aprendido: necesita la
    /// cota **de la cancion entera**, y por eso es `O(n)` y no `O(log n)`. Es el precio de
    /// que un salto sea correcto, y se paga solo al saltar, no en cada fotograma.
    pub fn reposicionar(&mut self, cancion: &Song, desde: Micros) {
        let notas = cancion.notes();
        self.max_duracion = notas
            .iter()
            .map(|n| n.end_us.0.saturating_sub(n.onset_us.0))
            .max()
            .unwrap_or(0);
        let cota = self.max_duracion;
        self.cursor =
            notas.partition_point(|n| n.onset_us.0.saturating_add(cota) < desde.0);
    }
}

/// Escribe en `out` las notas que se solapan con la ventana `[desde, hasta)`.
///
/// `posicion` es donde esta la practica: sirve para marcar cuales suenan **ahora**.
///
/// El coste crece con el numero de notas devueltas, no con el tamano de la cancion.
pub fn vista(
    cancion: &Song,
    v: &mut Vista,
    posicion: Micros,
    desde: Micros,
    hasta: Micros,
    out: &mut Vec<NotaVisible>,
) {
    let notas = cancion.notes();

    // Adelantar el cursor dejando atras solo lo que ya no puede verse. Se compara contra
    // `desde` y no contra el ataque, porque una nota larga sigue siendo visible mucho
    // despues de haber empezado.
    while let Some(n) = notas.get(v.cursor) {
        if n.end_us >= desde {
            break;
        }
        v.cursor = v.cursor.saturating_add(1);
        v.examinadas = v.examinadas.saturating_add(1);
    }

    for (offset, n) in notas.get(v.cursor..).into_iter().flatten().enumerate() {
        v.examinadas = v.examinadas.saturating_add(1);
        // Ordenadas por ataque: en cuanto una empieza despues de la ventana, las
        // siguientes tambien.
        if n.onset_us >= hasta {
            break;
        }
        v.max_duracion = v.max_duracion.max(n.end_us.0.saturating_sub(n.onset_us.0));
        if n.end_us < desde {
            continue; // empezo y termino antes de la ventana
        }
        let estado = if n.onset_us <= posicion && posicion < n.end_us {
            EstadoNota::Sonando
        } else {
            EstadoNota::Pendiente
        };
        out.push(NotaVisible {
            indice: u32::try_from(v.cursor.saturating_add(offset)).unwrap_or(u32::MAX),
            key: n.key,
            onset_us: n.onset_us,
            end_us: n.end_us,
            estado,
        });
    }
}
