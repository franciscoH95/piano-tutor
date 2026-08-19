//! Como se llama cada nota.
//!
//! # Por que hace falta la armadura
//!
//! Un numero de tecla MIDI no dice el nombre: el 61 es Do sostenido o Re bemol segun el
//! contexto, y el contexto es la armadura de la pieza. Sin ella habria que elegir uno
//! arbitrariamente y la mitad de las piezas se leerian mal.
//!
//! El mapa tiene la misma forma que el [`TempoMap`](crate::tempo::TempoMap) que ya existe:
//! tramos ordenados por tick, y para una nota se toma el ultimo con tick menor o igual.
//!
//! # Simbolo, no cadena
//!
//! Se emite `{ base, alteracion }`, nunca un texto. El formateo —si se escribe «Do♯» o
//! «C#», con que signo, en que idioma— pertenece a la capa que pinta. El nucleo no sabe de
//! textos (Principio III).

use crate::time::Ticks;

/// La letra de la nota, en nomenclatura latina.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Base {
    /// Do.
    Do,
    /// Re.
    Re,
    /// Mi.
    Mi,
    /// Fa.
    Fa,
    /// Sol.
    Sol,
    /// La.
    La,
    /// Si.
    Si,
}

/// Si la nota lleva alteracion, y cual.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Alteracion {
    /// Tecla blanca.
    Ninguna,
    /// Sostenido (♯).
    Sostenido,
    /// Bemol (♭).
    Bemol,
}

/// El nombre de una nota, en forma simbolica.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub struct NombreDeNota {
    /// La letra.
    pub base: Base,
    /// La alteracion, si la hay.
    pub alteracion: Alteracion,
}

/// Tabla de sostenidos, indexada por `key % 12`.
const SOSTENIDOS: [(Base, Alteracion); 12] = [
    (Base::Do, Alteracion::Ninguna),
    (Base::Do, Alteracion::Sostenido),
    (Base::Re, Alteracion::Ninguna),
    (Base::Re, Alteracion::Sostenido),
    (Base::Mi, Alteracion::Ninguna),
    (Base::Fa, Alteracion::Ninguna),
    (Base::Fa, Alteracion::Sostenido),
    (Base::Sol, Alteracion::Ninguna),
    (Base::Sol, Alteracion::Sostenido),
    (Base::La, Alteracion::Ninguna),
    (Base::La, Alteracion::Sostenido),
    (Base::Si, Alteracion::Ninguna),
];

/// Tabla de bemoles, indexada por `key % 12`.
const BEMOLES: [(Base, Alteracion); 12] = [
    (Base::Do, Alteracion::Ninguna),
    (Base::Re, Alteracion::Bemol),
    (Base::Re, Alteracion::Ninguna),
    (Base::Mi, Alteracion::Bemol),
    (Base::Mi, Alteracion::Ninguna),
    (Base::Fa, Alteracion::Ninguna),
    (Base::Sol, Alteracion::Bemol),
    (Base::Sol, Alteracion::Ninguna),
    (Base::La, Alteracion::Bemol),
    (Base::La, Alteracion::Ninguna),
    (Base::Si, Alteracion::Bemol),
    (Base::Si, Alteracion::Ninguna),
];

/// Las armaduras de una cancion, ordenadas por tick.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MapaDeArmaduras {
    /// `(tick, sf)`, con `sf` de -7 a 7. Negativo son bemoles.
    tramos: Vec<(u64, i8)>,
}

impl MapaDeArmaduras {
    /// Construye el mapa a partir de los meta-eventos de armadura.
    ///
    /// No hace falta que vengan ordenados. Si hay varios en el mismo tick gana el ultimo,
    /// que es la misma regla que aplica el mapa de tempo.
    #[must_use]
    pub fn desde(armaduras: &[(u64, i8)]) -> Self {
        let mut pares: Vec<(u64, i8)> = armaduras.to_vec();
        pares.sort_by_key(|(tick, _)| *tick);
        let mut tramos: Vec<(u64, i8)> = Vec::with_capacity(pares.len());
        for (tick, sf) in pares {
            match tramos.last_mut() {
                Some(ultimo) if ultimo.0 == tick => ultimo.1 = sf,
                _ => tramos.push((tick, sf)),
            }
        }
        Self { tramos }
    }

    /// La armadura vigente en un tick. Cero —sostenidos— si no hay ninguna declarada antes.
    #[must_use]
    pub fn vigente(&self, tick: Ticks) -> i8 {
        let k = self.tramos.partition_point(|(t, _)| *t <= tick.0);
        match k.checked_sub(1).and_then(|i| self.tramos.get(i)) {
            Some((_, sf)) => *sf,
            None => 0,
        }
    }

    /// El nombre de una altura MIDI en un tick dado.
    #[must_use]
    pub fn nombre(&self, tick: Ticks, key: u8) -> NombreDeNota {
        let tabla = if self.vigente(tick) < 0 { &BEMOLES } else { &SOSTENIDOS };
        let (base, alteracion) = tabla
            .get(usize::from(key % 12))
            .copied()
            .unwrap_or((Base::Do, Alteracion::Ninguna));
        NombreDeNota { base, alteracion }
    }
}
