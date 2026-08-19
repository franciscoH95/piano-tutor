//! **El unico sitio donde vive un umbral de esta feature.**
//!
//! El Principio I lo exige textualmente: las tolerancias MUST estar definidas
//! explicitamente y ser configurables por nivel, «nunca constantes dispersas o implicitas».
//! `core/tests/evaluacion_test.rs` lo comprueba **por ausencia**: ningun literal grande
//! puede aparecer en el resto del modulo.
//!
//! # Por que dos ventanas y no una
//!
//! La **ventana de emparejamiento** decide con que nota se casa una pulsacion, y es
//! **igual en los tres niveles**. La **ventana de ataque** decide si esa nota ya emparejada
//! cuenta como acertada, y esa si cambia.
//!
//! Con una sola ventana, cambiar de nivel cambiaria *que* se empareja con que, y una nota
//! podria quedar acertada en el nivel exigente y **sin pareja** en el permisivo. Separadas,
//! SC-006 —el permisivo nunca da menos aciertos que el exigente— deja de ser una propiedad
//! que hay que vigilar y pasa a ser aritmetica: mismo emparejamiento, ventanas anidadas.

/// Cuanta exigencia quiere el alumno.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Nivel {
    /// Para empezar.
    Permisivo,
    /// El punto medio.
    #[default]
    Intermedio,
    /// Para quien ya toca.
    Exigente,
}

/// Todos los umbrales, juntos.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Tolerancias {
    /// Cuanto puede alejarse una pulsacion de una nota para casarse con ella.
    ///
    /// **Igual en los tres niveles**, a proposito.
    pub ventana_emparejamiento_us: u64,
    /// Cuanto puede desviarse el ataque de una nota ya emparejada para contar acertada.
    ///
    /// **Absoluta, en microsegundos, y NO escala con la velocidad** (FR-008a). El oido mide
    /// en milisegundos, no en fracciones de negra: un desfase de 60 ms suena igual de mal a
    /// cualquier tempo. Que escalase haria que bajar la velocidad no exigiese mas precision,
    /// y bajar la velocidad es precisamente como se gana precision.
    pub ventana_ataque_us: u64,
    /// Cuanto puede alejarse en el tiempo una pulsacion de un acierto para ser un dedo que
    /// se escapa y no una nota de mas.
    pub cercania_dedo_us: u64,
    /// Y cuanto en altura. Un semitono es la tecla de al lado.
    pub cercania_dedo_semitonos: u8,
    /// A partir de que mediana se llama sistematico a un desfase.
    pub mediana_sistematico_us: u64,
    /// Y por debajo de que dispersion. Los dos a la vez (FR-016).
    pub dispersion_sistematico_us: u64,
    /// Cuantas notas hacen falta para que «sistematico» signifique algo.
    ///
    /// Con dos notas la mediana existe y no dice nada.
    pub minimo_notas_sistematico: usize,
}

/// Ventana de emparejamiento, comun a los tres niveles.
///
/// Medio segundo: lo bastante ancho para casar una pulsacion con su nota aunque el alumno
/// llegue muy tarde —de otro modo una nota tardia se contaria como omitida **y** como nota
/// de mas, dos fallos por un solo tropiezo—, y lo bastante estrecho para no invadir la nota
/// siguiente en un pasaje normal.
const EMPAREJAMIENTO_US: u64 = 500_000;

/// Umbral de la mediana para llamar sistematico a un desfase: 30 ms.
///
/// Por debajo, el desfase esta en el orden de lo que un interprete no controla.
const MEDIANA_SISTEMATICO_US: u64 = 30_000;

/// Dispersion maxima para llamarlo sistematico: 40 ms de recorrido intercuartilico.
///
/// Es lo que separa «va consistentemente tarde» de «va irregular»: si la mitad central de
/// los desfases cabe en 40 ms, el alumno tiene un problema de tempo, no de precision.
const DISPERSION_SISTEMATICO_US: u64 = 40_000;

/// Ocho notas. Con menos, la mediana existe y no describe nada.
const MINIMO_NOTAS_SISTEMATICO: usize = 8;

/// Cercania de un dedo que se escapa: 150 ms y un semitono.
///
/// Un semitono porque el error tipico es rozar la tecla **contigua**; dos ya no es un dedo
/// que resbala, es leer mal la nota.
const CERCANIA_DEDO_US: u64 = 150_000;
const CERCANIA_DEDO_SEMITONOS: u8 = 1;

impl Nivel {
    /// Los umbrales de este nivel.
    ///
    /// Las tres ventanas de ataque estan **anidadas** —120 ⊃ 60 ⊃ 30 ms— y esa relacion es
    /// lo que hace cierto SC-006. Cambiarlas sin mantener el orden lo rompe, y la prueba
    /// `las_ventanas_de_ataque_estan_anidadas` esta ahi para impedirlo.
    ///
    /// Las cifras salen de que el oido humano empieza a percibir asincronia en torno a los
    /// 20-30 ms: el nivel exigente pide lo que un oyente notaria, y el permisivo da margen
    /// de sobra a quien esta aprendiendo a coordinar las dos manos.
    #[must_use]
    pub const fn tolerancias(self) -> Tolerancias {
        let ventana_ataque_us = match self {
            Self::Permisivo => 120_000,
            Self::Intermedio => 60_000,
            Self::Exigente => 30_000,
        };
        Tolerancias {
            ventana_emparejamiento_us: EMPAREJAMIENTO_US,
            ventana_ataque_us,
            cercania_dedo_us: CERCANIA_DEDO_US,
            cercania_dedo_semitonos: CERCANIA_DEDO_SEMITONOS,
            mediana_sistematico_us: MEDIANA_SISTEMATICO_US,
            dispersion_sistematico_us: DISPERSION_SISTEMATICO_US,
            minimo_notas_sistematico: MINIMO_NOTAS_SISTEMATICO,
        }
    }
}
