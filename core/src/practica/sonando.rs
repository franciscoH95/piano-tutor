//! Que esta sonando en la cancion, y como se compara con lo que el alumno toca.
//!
//! # Sin ventana de tolerancia
//!
//! FR-014b lo prohibe expresamente: la pregunta aqui es **unicamente** si la nota estaba
//! sonando. Medir con cuanta precision se toco respecto al momento ideal es cosa de la
//! feature de evaluacion, y meter aqui una tolerancia "por si acaso" la adelantaria sin
//! haberla especificado.
//!
//! Los extremos son **cerrado en el ataque y abierto en el final**, el mismo convenio que
//! usa `vista.rs`. Tiene que ser el mismo: con criterios distintos, la misma nota estaria
//! sonando para una parte del nucleo y no para otra. Ademas es el unico convenio que hace
//! que dos notas seguidas de la misma tecla no dejen un microsegundo de silencio entre
//! ellas ni se solapen.

use crate::time::Micros;
use crate::Song;

/// Que teclas estan pulsadas, o sonando. Las 128 del protocolo MIDI en 16 bytes.
///
/// Cabe entera en dos palabras, asi que copiarla y compararla es gratis y **no asigna
/// memoria**: puede vivir en la ruta critica sin tocar el presupuesto del Principio IV.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct MascaraTeclas([u64; 2]);

impl MascaraTeclas {
    /// Ninguna tecla.
    pub const VACIA: Self = Self([0, 0]);

    const fn sitio(key: u8) -> (usize, u64) {
        let k = (key & 0x7F) as usize;
        (k / 64, 1_u64 << (k % 64))
    }

    /// Marca una tecla. Ponerla dos veces es lo mismo que ponerla una.
    pub const fn poner(&mut self, key: u8) {
        let (i, bit) = Self::sitio(key);
        if i == 0 {
            self.0[0] |= bit;
        } else {
            self.0[1] |= bit;
        }
    }

    /// Desmarca una tecla. Soltar una que no estaba pulsada no hace nada.
    pub const fn quitar(&mut self, key: u8) {
        let (i, bit) = Self::sitio(key);
        if i == 0 {
            self.0[0] &= !bit;
        } else {
            self.0[1] &= !bit;
        }
    }

    /// Si la tecla esta marcada.
    #[must_use]
    pub const fn contiene(&self, key: u8) -> bool {
        let (i, bit) = Self::sitio(key);
        if i == 0 {
            self.0[0] & bit != 0
        } else {
            self.0[1] & bit != 0
        }
    }

    /// Cuantas teclas estan marcadas.
    #[must_use]
    pub const fn cuenta(&self) -> u32 {
        self.0[0].count_ones() + self.0[1].count_ones()
    }

    /// Si no hay ninguna.
    #[must_use]
    pub const fn esta_vacia(&self) -> bool {
        self.0[0] == 0 && self.0[1] == 0
    }
}

/// Que relacion tiene una tecla pulsada con lo que la cancion pide en ese instante.
///
/// La tercera situacion de FR-014a, la nota **omitida**, no esta aqui a proposito: no es una
/// propiedad de una pulsacion sino de una nota que dejo de sonar sin que nadie la tocara, y
/// solo puede afirmarse cuando su duracion ha pasado entera.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Situacion {
    /// La cancion tenia esa tecla sonando.
    Acierto,
    /// La cancion no la pedia.
    Extra,
}

/// Que teclas de la cancion suenan en la posicion actual.
pub struct ConjuntoSonando {
    /// Primera nota que puede seguir sonando. Solo avanza, salvo al recolocar.
    cursor: usize,
    /// Duracion de la nota mas larga de **la cancion entera**.
    ///
    /// De la cancion entera y no de lo visto hasta ahora: `Vista` lo aprendia sobre la
    /// marcha, y en un salto recien cargado valia cero y no acotaba nada.
    max_duracion: u64,
    /// Las teclas que suenan ahora. Se recalcula una vez por avance, no por consulta.
    vigentes: MascaraTeclas,
    /// Diagnostico: permite afirmar la garantia de coste **contando**, no cronometrando.
    examinadas: usize,
    /// Por nota: si el alumno la tenia pulsada en algun momento de su duracion.
    ///
    /// Se reserva una vez, al construir, y no vuelve a crecer: puede vivir en la ruta
    /// critica sin tocar el presupuesto del Principio IV.
    tocada: Vec<bool>,
    /// Por nota: si su omision ya se comunico. Sin esto, el puente llevaria sesenta avisos
    /// por segundo de una nota que ya paso.
    informada: Vec<bool>,
    /// Primera nota que podria seguir pendiente de informar. Solo avanza.
    cursor_omision: usize,
}

/// La cota de duracion de la cancion.
fn cota(cancion: &Song) -> u64 {
    cancion
        .notes()
        .iter()
        .map(|n| n.end_us.0.saturating_sub(n.onset_us.0))
        .max()
        .unwrap_or(0)
}

impl ConjuntoSonando {
    /// Un conjunto al principio de la cancion.
    #[must_use]
    pub fn nuevo(cancion: &Song) -> Self {
        Self {
            cursor: 0,
            max_duracion: cota(cancion),
            vigentes: MascaraTeclas::VACIA,
            examinadas: 0,
            tocada: vec![false; cancion.notes().len()],
            informada: vec![false; cancion.notes().len()],
            cursor_omision: 0,
        }
    }

    /// Recoloca tras un salto **hacia atras**. Coste `O(log n)`.
    ///
    /// El predicado es sobre el **ataque**, que es por donde estan ordenadas las notas.
    /// Buscar por el final no es monotono sobre esa ordenacion —un pedal largo termina
    /// despues de notas cortas posteriores— y devolveria un indice cualquiera.
    pub fn recolocar(&mut self, cancion: &Song, posicion: Micros) {
        let cota = self.max_duracion;
        self.cursor = cancion
            .notes()
            .partition_point(|n| n.onset_us.0.saturating_add(cota) < posicion.0);
        self.recalcular(cancion, posicion);
    }

    /// Adelanta hasta `posicion` y recalcula que suena.
    ///
    /// Hacia atras recoloca; hacia delante el cursor avanza solo, que es lo que hace que el
    /// coste no dependa del tamano de la cancion.
    pub fn avanzar(&mut self, cancion: &Song, posicion: Micros) {
        let notas = cancion.notes();
        // Aqui **se tiene la nota en la mano**, asi que rige su final propio: se deja atras
        // en cuanto ha terminado de verdad. El bucle se para en la primera que aun suena,
        // asi que no puede saltarse ninguna.
        //
        // La cota de duracion NO se usa aqui, y no es un detalle. Es un recurso de la
        // busqueda binaria de `recolocar`, donde no hay nota que mirar y hace falta un
        // predicado monotono. Usarla tambien aqui deja el cursor retrasado la cota ENTERA
        // de forma permanente: un pedal de treinta segundos al principio seguia costando
        // treinta segundos de retraso diez minutos despues de haber terminado. Medido en
        // una pieza de 10 min con 2.400 notas: 118 notas examinadas por fotograma en vez
        // de 5, y creciendo con la densidad de la pieza.
        while let Some(n) = notas.get(self.cursor) {
            if n.end_us.0 > posicion.0 {
                break;
            }
            self.cursor = self.cursor.saturating_add(1);
            self.examinadas = self.examinadas.saturating_add(1);
        }
        self.recalcular(cancion, posicion);
    }

    fn recalcular(&mut self, cancion: &Song, posicion: Micros) {
        let mut m = MascaraTeclas::VACIA;
        for n in cancion.notes().get(self.cursor..).into_iter().flatten() {
            self.examinadas = self.examinadas.saturating_add(1);
            // Ordenadas por ataque: en cuanto una empieza despues, las siguientes tambien.
            if n.onset_us.0 > posicion.0 {
                break;
            }
            if posicion.0 < n.end_us.0 {
                m.poner(n.key);
            }
        }
        self.vigentes = m;
    }

    /// Si la cancion tiene esa tecla sonando ahora. `O(1)`.
    #[must_use]
    pub const fn suena(&self, key: u8) -> bool {
        self.vigentes.contiene(key)
    }

    /// Las teclas que suenan ahora.
    #[must_use]
    pub const fn vigentes(&self) -> MascaraTeclas {
        self.vigentes
    }

    /// Que relacion tiene una tecla pulsada con lo que la cancion pide. `O(1)`.
    #[must_use]
    pub const fn clasificar(&self, key: u8) -> Situacion {
        if self.suena(key) {
            Situacion::Acierto
        } else {
            Situacion::Extra
        }
    }

    /// Anota que teclas tenia el alumno pulsadas en este instante.
    ///
    /// Se llama **en cada avance**, no solo cuando llega un ataque. Es la diferencia entre
    /// un modelo correcto y uno que declara omitida una nota que el alumno esta tocando:
    /// en un teclado real, una tecla mantenida **no genera eventos nuevos**, asi que mirar
    /// solo los ataques la perderia por completo.
    pub fn registrar(&mut self, cancion: &Song, pulsadas: MascaraTeclas, posicion: Micros) {
        if pulsadas.esta_vacia() {
            return;
        }
        for (i, n) in cancion.notes().iter().enumerate().skip(self.cursor_omision) {
            if n.onset_us.0 > posicion.0 {
                break;
            }
            if posicion.0 < n.end_us.0 && pulsadas.contiene(n.key) {
                if let Some(t) = self.tocada.get_mut(i) {
                    *t = true;
                }
            }
        }
    }

    /// Vuelca en `out` las notas que terminaron sin que nadie las tocara.
    ///
    /// Cada una se comunica **una sola vez**, como el final de la cancion.
    pub fn omitidas(&mut self, cancion: &Song, posicion: Micros, out: &mut Vec<usize>) {
        out.clear();
        let notas = cancion.notes();
        for (i, n) in notas.iter().enumerate().skip(self.cursor_omision) {
            // Ordenadas por ataque: mas alla de la posicion no puede haber nada terminado.
            if n.onset_us.0 > posicion.0 {
                break;
            }
            if n.end_us.0 > posicion.0 {
                continue; // todavia suena
            }
            let ya = self.informada.get(i).copied().unwrap_or(true);
            if !ya && !self.tocada.get(i).copied().unwrap_or(true) {
                out.push(i);
            }
            if let Some(f) = self.informada.get_mut(i) {
                *f = true;
            }
        }
        // Mismo criterio que en `avanzar`: la nota esta en la mano, asi que se pasa de
        // largo cuando ha terminado **y** ya se informo, no antes.
        while let Some(n) = notas.get(self.cursor_omision) {
            if n.end_us.0 > posicion.0 || !self.informada.get(self.cursor_omision).copied().unwrap_or(false) {
                break;
            }
            self.cursor_omision = self.cursor_omision.saturating_add(1);
        }
    }

    /// Cuantas notas se han examinado desde que se creo.
    #[must_use]
    pub const fn examinadas(&self) -> usize {
        self.examinadas
    }
}
