//! El veredicto de cada nota y el resumen que ve el alumno.

use crate::evaluacion::estadistica::{cuartiles, mediana};
use crate::evaluacion::tolerancias::Tolerancias;
use crate::practica::Mano;
use crate::time::Micros;

/// Que paso con una nota de la cancion, o con una pulsacion suelta.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Veredicto {
    /// Se toco dentro de la ventana de ataque del nivel.
    Acertada,
    /// Se emparejo, pero fuera de la ventana de ataque.
    ///
    /// Existe **porque hay dos ventanas**: el emparejamiento no depende del nivel y el
    /// veredicto si. Sin esta clase, cambiar de nivel cambiaria que se empareja con que.
    TocadaFueraDeTiempo,
    /// Nadie la toco.
    Omitida,
    /// Cae fuera de las 88 teclas: el alumno no puede tocarla (FR-014).
    FueraDeAlcance,
    /// El alumno salto ese pasaje con la salida del modo espera (FR-013).
    NoIntentada,
}

/// Lo medido de una nota emparejada.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Medida {
    /// **Con signo**: negativo se adelanto, positivo se atraso.
    ///
    /// El signo *es* la informacion: sin el no se distingue ir adelantado de ir atrasado,
    /// que es la mitad de FR-016.
    pub desfase_us: i64,
    /// Diferencia de duracion respecto a lo escrito, con signo.
    ///
    /// `None` si la tecla seguia hundida al cerrar: **desconocida, que no es cero**.
    pub duracion_us: Option<i64>,
    /// Se registra, no se juzga.
    pub velocity: u8,
}

/// Un desfase que no es de una nota, sino de toda la interpretacion.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Sistematico {
    /// Con signo: adelantado o atrasado.
    pub mediana_us: i64,
    /// Recorrido intercuartilico: como de junta esta la mitad central.
    pub dispersion_us: u64,
}

/// Recuento de una mano.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct Recuento {
    /// Acertadas.
    pub acertadas: usize,
    /// Emparejadas pero fuera de tiempo.
    pub fuera_de_tiempo: usize,
    /// Omitidas.
    pub omitidas: usize,
}

/// Lo que se le enseña al alumno.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Resultado {
    /// Notas acertadas.
    pub acertadas: usize,
    /// Emparejadas pero fuera de la ventana de ataque.
    pub fuera_de_tiempo: usize,
    /// Notas que nadie toco.
    pub omitidas: usize,
    /// Pulsaciones que no corresponden a ninguna nota.
    pub de_mas: usize,
    /// Pulsaciones muy proximas a un acierto: dedos que resbalan (FR-010a).
    pub dedos_escapados: usize,
    /// Notas fuera de las 88 teclas. **Fuera del denominador**: no son culpa del alumno.
    pub fuera_de_alcance: usize,
    /// Pasajes saltados con la salida del modo espera. Tambien fuera del denominador.
    pub no_intentadas: usize,
    /// El desfase de toda la interpretacion, si lo hay.
    pub desfase: Option<Sistematico>,
    /// **No se toco ni una tecla.** Distinto de tocarlo todo mal (FR-019).
    pub sin_tocar: bool,
    /// Los tiempos **no** se evaluaron, porque se practico en modo espera (FR-015a).
    ///
    /// Hay que decirlo: un resultado incompleto que no se declara incompleto se lee como
    /// completo.
    pub parcial: bool,
    /// Recuento de la izquierda y de la derecha (FR-018).
    pub por_mano: [Recuento; 2],
    /// El veredicto de cada nota, por su indice en `Song::notes` (FR-017).
    pub por_nota: Vec<(usize, Veredicto)>,
}

impl Resultado {
    /// Cuantas notas se le pidieron de verdad al alumno.
    ///
    /// **El denominador honesto** (SC-009): lo que no puede tocar y lo que no llego a
    /// intentar quedan fuera. Meterlos dentro convertiria en fallo del alumno algo que no
    /// lo es.
    #[must_use]
    pub const fn intentadas(&self) -> usize {
        self.acertadas + self.fuera_de_tiempo + self.omitidas
    }

    /// La fraccion de aciertos, **sin escala**: aciertos y total, tal cual.
    ///
    /// No devuelve un porcentaje a proposito. Elegir la escala —porcentaje, por mil— es una
    /// decision de presentacion, y ademas meteria aqui un numero que no es una tolerancia
    /// pero que la prueba de T008 no puede distinguir de una. Sin numero, sin problema.
    ///
    /// `None` si no se le pidio nada al alumno.
    #[must_use]
    pub const fn fraccion_de_aciertos(&self) -> Option<(usize, usize)> {
        match self.intentadas() {
            0 => None,
            total => Some((self.acertadas, total)),
        }
    }
}

/// Calcula el desfase sistematico, si lo hay.
///
/// Hace falta que se cumplan **las dos condiciones a la vez**: mediana por encima del
/// umbral y dispersion por debajo. Solo la mediana confundiria «va irregular» con «va
/// tarde»; solo la dispersion no diria nada de la direccion.
#[must_use]
pub fn sistematico(desfases: &[i64], tol: &Tolerancias) -> Option<Sistematico> {
    // Con muy pocas notas la mediana existe y no describe nada.
    if desfases.len() < tol.minimo_notas_sistematico {
        return None;
    }
    let m = mediana(desfases)?;
    let (q1, q3) = cuartiles(desfases)?;
    let dispersion = q3.saturating_sub(q1).unsigned_abs();
    let magnitud = m.unsigned_abs();
    (magnitud >= tol.mediana_sistematico_us && dispersion <= tol.dispersion_sistematico_us)
        .then_some(Sistematico { mediana_us: m, dispersion_us: dispersion })
}

/// Suma un veredicto al recuento de su mano.
pub(crate) fn contar(por_mano: &mut [Recuento; 2], mano: Mano, v: Veredicto) {
    let i = usize::from(mano == Mano::Derecha);
    let Some(r) = por_mano.get_mut(i) else {
        return;
    };
    match v {
        Veredicto::Acertada => r.acertadas += 1,
        Veredicto::TocadaFueraDeTiempo => r.fuera_de_tiempo += 1,
        Veredicto::Omitida => r.omitidas += 1,
        Veredicto::FueraDeAlcance | Veredicto::NoIntentada => {}
    }
}

/// El instante esperado de una nota, para medir contra el.
pub(crate) const fn desfase(pulsacion: Micros, esperado: Micros) -> i64 {
    #[allow(clippy::cast_possible_wrap)]
    let a = pulsacion.0 as i64;
    #[allow(clippy::cast_possible_wrap)]
    let b = esperado.0 as i64;
    a.wrapping_sub(b)
}
