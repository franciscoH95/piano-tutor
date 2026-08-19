//! Donde el modo espera detiene el cursor.
//!
//! Una **puerta** es un instante de la cancion con un conjunto de teclas que hay que tener
//! pulsadas **a la vez** para pasar. Se precalculan al preparar la cancion y se recorren
//! con un cursor monotono: durante la practica no se busca nada, solo se mira la pendiente.

use crate::practica::manos::Mano;
use crate::practica::sonando::MascaraTeclas;
use crate::time::Micros;
use crate::Song;

/// Un instante que hay que acertar para seguir.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Puerta {
    /// Instante de la cancion en que el cursor se detiene.
    pub onset_us: Micros,
    /// Las teclas que hay que tener pulsadas **todas a la vez**.
    pub teclas: MascaraTeclas,
}

/// Todas las puertas de una cancion, en orden.
#[derive(Clone, Debug, Default)]
pub struct ProgramaDePuertas {
    puertas: Vec<Puerta>,
}

impl ProgramaDePuertas {
    /// Calcula las puertas de una cancion.
    ///
    /// `manos` dice de que mano es cada nota, en el orden de `Song::notes`. `practicada`
    /// filtra: con una mano elegida, **las puertas de la otra no existen**. No es que se
    /// abran solas; es que no estan. Si estuvieran, el cursor se detendria en ellas a
    /// esperar algo que el alumno no tiene que tocar (SC-012).
    ///
    /// La percusion no genera puertas: no se toca con las manos en el teclado.
    #[must_use]
    pub fn nuevo(cancion: &Song, manos: &[Mano], practicada: Option<Mano>) -> Self {
        let mut puertas: Vec<Puerta> = Vec::new();
        for (i, n) in cancion.notes().iter().enumerate() {
            if let (Some(quiero), Some(suya)) = (practicada, manos.get(i).copied()) {
                if quiero != suya {
                    continue;
                }
            }
            // Las notas vienen ordenadas por ataque, asi que las simultaneas caen seguidas
            // y basta mirar la ultima puerta.
            match puertas.last_mut() {
                Some(p) if p.onset_us == n.onset_us => p.teclas.poner(n.key),
                _ => {
                    let mut teclas = MascaraTeclas::VACIA;
                    teclas.poner(n.key);
                    puertas.push(Puerta { onset_us: n.onset_us, teclas });
                }
            }
        }
        Self { puertas }
    }

    /// La puerta numero `i`.
    #[must_use]
    pub fn get(&self, i: usize) -> Option<&Puerta> {
        self.puertas.get(i)
    }

    /// Cuantas puertas hay.
    #[must_use]
    pub fn len(&self) -> usize {
        self.puertas.len()
    }

    /// Si no hay ninguna.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.puertas.is_empty()
    }

    /// La primera puerta cuyo instante es **posterior o igual** a `posicion`.
    ///
    /// Hace falta al activar el modo espera a mitad de cancion: la puerta pendiente es la
    /// siguiente que queda por delante, no una que ya se paso.
    #[must_use]
    pub fn desde(&self, posicion: Micros) -> usize {
        self.puertas.partition_point(|p| p.onset_us.0 < posicion.0)
    }
}
