//! De que mano es cada nota.
//!
//! # Dos caminos, y uno se prefiere al otro
//!
//! El material de piano suele traer las manos en voces separadas. Cuando es asi, se usa lo
//! que dice el archivo: es informacion humana y siempre gana. Cuando no —una sola voz con
//! todo mezclado—, se reparte por altura, y ahi la heuristica puede equivocarse: por eso el
//! punto de corte es **ajustable** y el control esta siempre a la vista.
//!
//! # Por que la derecha es la voz mas aguda y no la pista 0
//!
//! Existe la convencion de poner la mano derecha en la primera pista, pero no es universal
//! y fallar en esto invierte la pieza entera. La mediana de altura es un criterio que
//! funciona sin depender de que quien exporto el archivo siguiera la costumbre.

use crate::Song;

/// Cual de las dos manos.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Mano {
    /// Mano izquierda.
    Izquierda,
    /// Mano derecha.
    Derecha,
}

/// De donde salio el reparto.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Reparto {
    /// El archivo traia las manos en voces separadas.
    VocesDelArchivo,
    /// Se dedujo por altura, con el punto de corte.
    CortePorAltura,
}

/// Canal de percusion, contado desde cero. No es una voz de piano.
const CANAL_PERCUSION: u8 = 9;
/// Proporcion minima de notas para que una voz cuente como mano.
const MINIMO_POR_VOZ: usize = 20; // 1/20 = 5 %
/// Separacion minima entre medianas, en semitonos.
const SEPARACION_MINIMA: u8 = 3;

/// El reparto de una cancion concreta.
#[derive(Clone, Debug)]
pub struct RepartoDeManos {
    manos: Vec<Mano>,
    origen: Reparto,
}

impl RepartoDeManos {
    /// De que mano es la nota que ocupa esa posicion en la cancion.
    #[must_use]
    pub fn mano(&self, indice: usize) -> Mano {
        self.manos.get(indice).copied().unwrap_or(Mano::Derecha)
    }

    /// Si se usaron las voces del archivo o el corte por altura.
    #[must_use]
    pub const fn origen(&self) -> Reparto {
        self.origen
    }

    /// Cuantas notas hay repartidas.
    #[must_use]
    pub fn len(&self) -> usize {
        self.manos.len()
    }

    /// `true` si la cancion no tiene notas.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.manos.is_empty()
    }
}

fn mediana(mut alturas: Vec<u8>) -> u8 {
    alturas.sort_unstable();
    alturas.get(alturas.len() / 2).copied().unwrap_or(0)
}

/// Reparte las notas entre las dos manos.
///
/// `corte` es la altura que separa ambas manos cuando hay que deducirlo. Solo se usa si el
/// archivo no trae las manos separadas.
#[must_use]
pub fn repartir(cancion: &Song, corte: u8) -> RepartoDeManos {
    let notas = cancion.notes();

    // Una voz es un par (pista, canal) con notas, descartada la percusion.
    let mut voces: Vec<(u16, u8)> = Vec::new();
    for n in notas {
        if n.channel == CANAL_PERCUSION {
            continue;
        }
        let v = (n.track, n.channel);
        if !voces.contains(&v) {
            voces.push(v);
        }
    }
    // Orden determinista: es lo que hace reproducible el desempate.
    voces.sort_unstable();

    // G1: exactamente dos voces con notas.
    let separadas = if voces.len() == 2 {
        let alturas: Vec<Vec<u8>> = voces
            .iter()
            .map(|v| {
                notas
                    .iter()
                    .filter(|n| (n.track, n.channel) == *v && n.channel != CANAL_PERCUSION)
                    .map(|n| n.key)
                    .collect()
            })
            .collect();
        let total: usize = alturas.iter().map(Vec::len).sum();
        // G3, primera mitad: cada voz con al menos el 5 % de las notas.
        let bastante = alturas.iter().all(|a| a.len().saturating_mul(MINIMO_POR_VOZ) >= total);
        let medianas: Vec<u8> = alturas.into_iter().map(mediana).collect();
        // G3, segunda mitad: medianas separadas al menos tres semitonos.
        let separadas_en_altura = match (medianas.first(), medianas.get(1)) {
            (Some(a), Some(b)) => a.abs_diff(*b) >= SEPARACION_MINIMA,
            _ => false,
        };
        // G2 (mismo instrumento) se satisface hoy siempre: el cargador no extrae los
        // cambios de programa, asi que "ninguna voz declara programa" es cierto por
        // construccion. Queda el hueco para apretarla cuando se extraigan.
        if bastante && separadas_en_altura {
            Some((voces.clone(), medianas))
        } else {
            None
        }
    } else {
        None
    };

    match separadas {
        Some((voces, medianas)) => {
            // La derecha es la voz de mediana mas alta. Empate: la de (pista, canal) menor
            // se queda como izquierda, que es determinista aunque arbitrario.
            let derecha = match (medianas.first(), medianas.get(1)) {
                (Some(a), Some(b)) if b > a => voces.get(1).copied(),
                _ => voces.first().copied(),
            };
            let manos = notas
                .iter()
                .map(|n| {
                    if Some((n.track, n.channel)) == derecha {
                        Mano::Derecha
                    } else {
                        Mano::Izquierda
                    }
                })
                .collect();
            RepartoDeManos { manos, origen: Reparto::VocesDelArchivo }
        }
        None => {
            let manos = notas
                .iter()
                .map(|n| if n.key >= corte { Mano::Derecha } else { Mano::Izquierda })
                .collect();
            RepartoDeManos { manos, origen: Reparto::CortePorAltura }
        }
    }
}
