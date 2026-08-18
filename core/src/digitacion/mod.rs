//! Que dedo se propone para cada nota.
//!
//! Los archivos MIDI **no contienen digitacion**: es una anotacion editorial que ponen los
//! editores en las partituras. Aqui se calcula con el modelo ergonomico de Parncutt et al.
//! (1997) y se entrega como **sugerencia**, nunca como verdad. Una digitacion mal impuesta
//! es peor que ninguna: el principiante la interioriza y luego hay que desaprenderla.
//!
//! # El convenio del que depende todo
//!
//! El vano entre dos dedos se mide **siempre del dedo de numero menor al mayor**. Para el
//! par (3,1) con un intervalo ascendente de tres semitonos, el vano canonico es **−3**, no
//! +3. Codificarlo al reves hace que el paso del pulgar no se detecte jamas, y las escalas
//! salen absurdas sin que nada falle de forma visible.
//!
//! # La mano izquierda es la derecha reflejada
//!
//! La altura relativa a la mano es `h(p) = p` en la derecha y `h(p) = −p` en la izquierda.
//! Con eso, las mismas tablas y las mismas reglas sirven para ambas. El **color** de la
//! tecla, en cambio, se consulta siempre sobre la altura MIDI real: reflejar sirve para la
//! geometria de la mano, no para el teclado.

mod coste;
mod tablas;

use crate::practica::Mano;
use crate::Song;
use coste::{
    coste_forma_acorde, coste_nota, coste_nota_simultanea, coste_terceto, coste_transicion,
    coste_vano,
};

/// Un dedo, con el convenio del piano: 1 es el pulgar, 5 el meniique, en ambas manos.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
pub enum Dedo {
    /// Pulgar.
    D1,
    /// Indice.
    D2,
    /// Corazon.
    D3,
    /// Anular.
    D4,
    /// Meniique.
    D5,
}

impl Dedo {
    /// Su numero, de 1 a 5.
    #[must_use]
    pub const fn numero(self) -> u8 {
        match self {
            Self::D1 => 1,
            Self::D2 => 2,
            Self::D3 => 3,
            Self::D4 => 4,
            Self::D5 => 5,
        }
    }

    const fn desde(n: u8) -> Self {
        match n {
            1 => Self::D1,
            2 => Self::D2,
            3 => Self::D3,
            4 => Self::D4,
            _ => Self::D5,
        }
    }
}

/// Altura relativa a la mano. La izquierda ve el teclado al reves.
#[must_use]
const fn h(key: u8, mano: Mano) -> i32 {
    match mano {
        Mano::Derecha => key as i32,
        Mano::Izquierda => -(key as i32),
    }
}

/// El vano entre dos dedos, medido del de numero menor al mayor.
///
/// Es el convenio del que depende el resto del modelo: ver la documentacion del modulo.
#[must_use]
pub fn vano_canonico(dedo_a: Dedo, key_a: u8, dedo_b: Dedo, key_b: u8, mano: Mano) -> i32 {
    let (a, b) = (dedo_a.numero(), dedo_b.numero());
    if a == b {
        return 0;
    }
    if a < b {
        h(key_b, mano) - h(key_a, mano)
    } else {
        h(key_a, mano) - h(key_b, mano)
    }
}

/// La digitacion propuesta para una cancion.
#[derive(Clone, Debug, Default)]
pub struct Digitacion {
    dedos: Vec<Dedo>,
}

impl Digitacion {
    /// El dedo propuesto para la nota que ocupa esa posicion.
    #[must_use]
    pub fn dedo(&self, indice: usize) -> Dedo {
        self.dedos.get(indice).copied().unwrap_or(Dedo::D3)
    }

    /// Cuantas notas tienen dedo.
    #[must_use]
    pub fn len(&self) -> usize {
        self.dedos.len()
    }

    /// `true` si no hay ninguna nota.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.dedos.is_empty()
    }
}

/// Un grupo de notas que suenan a la vez en la misma mano.
struct Grupo {
    /// Indices en `Song::notes`, ordenados por altura relativa a la mano.
    indices: Vec<usize>,
    /// Alturas MIDI reales, en el mismo orden.
    keys: Vec<u8>,
}

/// Reparticiones posibles de dedos para un grupo: combinaciones **estrictamente
/// crecientes**, porque un dedo no puede tocar dos teclas y la mano no se cruza consigo
/// misma dentro de un acorde.
fn candidatos(n: usize) -> Vec<Vec<u8>> {
    let mut salida = Vec::new();
    let mut actual = Vec::with_capacity(n);
    fn generar(n: usize, desde: u8, actual: &mut Vec<u8>, salida: &mut Vec<Vec<u8>>) {
        if actual.len() == n {
            salida.push(actual.clone());
            return;
        }
        let mut d = desde;
        while d <= 5 {
            actual.push(d);
            generar(n, d + 1, actual, salida);
            actual.pop();
            d += 1;
        }
    }
    generar(n.min(5), 1, &mut actual, &mut salida);
    salida
}

/// Digita una mano completa con programacion dinamica de segundo orden.
///
/// El estado es **el par de asignaciones de los dos ultimos grupos**, no solo el ultimo:
/// hace falta el penultimo para poder penalizar tercetos como 3-4-5, que un par no ve.
///
/// Al final se reconstruye el camino hacia atras. Elegir lo mejor de cada grupo por
/// separado **no es programacion dinamica**, es una eleccion voraz sin memoria, y produce
/// digitaciones absurdas: una escala descendente sale entera con el pulgar.
fn digitar_mano(grupos: &[Grupo], mano: Mano, dedos: &mut [Dedo]) {
    let n = grupos.len();
    if n == 0 {
        return;
    }
    let opciones: Vec<Vec<Vec<u8>>> =
        grupos.iter().map(|g| candidatos(g.indices.len())).collect();

    // Coste propio de cada asignacion, sin mirar a los grupos vecinos.
    let base: Vec<Vec<i32>> = grupos
        .iter()
        .zip(&opciones)
        .map(|(g, ops)| {
            ops.iter()
                .map(|op| {
                    // Un grupo con mas de una nota es un acorde: suenan y se sostienen a
                    // la vez, y eso cambia que reglas aplican.
                    let simultaneo = g.keys.len() > 1;
                    let mut c: i32 = 0;
                    for (d, k) in op.iter().zip(g.keys.iter()) {
                        c += if simultaneo {
                            coste_nota_simultanea(*d, *k)
                        } else {
                            coste_nota(*d, *k)
                        };
                    }
                    // **Todos** los pares, no solo los contiguos: un acorde se sostiene
                    // entero, asi que si el par exterior no cabe en la mano el acorde no se
                    // puede tocar, por comodos que sean los pares de al lado.
                    for (i, (da, ka)) in op.iter().zip(g.keys.iter()).enumerate() {
                        for (db, kb) in op.iter().zip(g.keys.iter()).skip(i + 1) {
                            let v =
                                vano_canonico(Dedo::desde(*da), *ka, Dedo::desde(*db), *kb, mano);
                            // Solo la apertura: el color de las teclas ya se ha cobrado
                            // arriba, una vez por nota.
                            c = c.saturating_add(if simultaneo {
                                coste_vano(*da, *db, v)
                            } else {
                                coste_transicion(*da, *ka, *db, *kb, v)
                            });
                        }
                    }
                    if simultaneo {
                        c = c.saturating_add(coste_forma_acorde(op));
                    }
                    c
                })
                .collect()
        })
        .collect();

    let ultimo_dedo = |i: usize, a: usize| -> u8 {
        opciones
            .get(i)
            .and_then(|o| o.get(a))
            .and_then(|v| v.last())
            .copied()
            .unwrap_or(3)
    };
    let primer_dedo = |i: usize, b: usize| -> u8 {
        opciones
            .get(i)
            .and_then(|o| o.get(b))
            .and_then(|v| v.first())
            .copied()
            .unwrap_or(3)
    };
    // Coste de encadenar la asignacion `a` del grupo `i-1` con la `b` del grupo `i`.
    let enlace = |i: usize, a: usize, b: usize| -> i32 {
        let previo = i.saturating_sub(1);
        let (da, db) = (ultimo_dedo(previo, a), primer_dedo(i, b));
        let ka = grupos
            .get(previo)
            .and_then(|g| g.keys.last())
            .copied()
            .unwrap_or(60);
        let kb = grupos
            .get(i)
            .and_then(|g| g.keys.first())
            .copied()
            .unwrap_or(60);
        let v = vano_canonico(Dedo::desde(da), ka, Dedo::desde(db), kb, mano);
        coste_transicion(da, ka, db, kb, v)
    };

    // dp[i][b][a] = coste minimo hasta el grupo i con `b` elegido en i y `a` en i-1.
    // de[i][b][a] = la eleccion en i-2 que consiguio ese minimo (el terceto la necesita).
    let mut dp: Vec<Vec<Vec<i32>>> = Vec::with_capacity(n);
    let mut de: Vec<Vec<Vec<usize>>> = Vec::with_capacity(n);

    // El grupo 0 no tiene anterior: su segunda dimension es un unico indice ficticio.
    dp.push(
        base.first()
            .map_or_else(Vec::new, |f| f.iter().map(|c| vec![*c]).collect()),
    );
    de.push(vec![vec![usize::MAX]; opciones.first().map_or(0, Vec::len)]);

    for i in 1..n {
        let na = opciones.get(i - 1).map_or(0, Vec::len);
        let nb = opciones.get(i).map_or(0, Vec::len);
        let mut capa = vec![vec![i32::MAX; na]; nb];
        let mut proc = vec![vec![usize::MAX; na]; nb];
        for b in 0..nb {
            for a in 0..na {
                // `a` es la eleccion en i-1, asi que hay que entrar en dp[i-1] por su
                // PRIMERA dimension; la segunda recorre las elecciones en i-2.
                let Some(anteriores) = dp.get(i - 1).and_then(|c| c.get(a)) else {
                    continue;
                };
                let mut mejor = i32::MAX;
                let mut mejor_p = 0_usize;
                for (p, previo) in anteriores.iter().enumerate() {
                    if *previo == i32::MAX {
                        continue;
                    }
                    let terceto = if i >= 2 {
                        coste_terceto(
                            ultimo_dedo(i - 2, p),
                            ultimo_dedo(i - 1, a),
                            primer_dedo(i, b),
                        )
                    } else {
                        0
                    };
                    let total = previo.saturating_add(terceto);
                    if total < mejor {
                        mejor = total;
                        mejor_p = p;
                    }
                }
                if mejor == i32::MAX {
                    continue;
                }
                let propio = base.get(i).and_then(|f| f.get(b)).copied().unwrap_or(0);
                if let Some(celda) = capa.get_mut(b).and_then(|f| f.get_mut(a)) {
                    *celda = mejor
                        .saturating_add(propio)
                        .saturating_add(enlace(i, a, b));
                }
                if let Some(celda) = proc.get_mut(b).and_then(|f| f.get_mut(a)) {
                    *celda = mejor_p;
                }
            }
        }
        dp.push(capa);
        de.push(proc);
    }

    // Reconstruccion hacia atras desde el mejor estado final.
    let ultimo = n - 1;
    let (mut mejor_c, mut mejor_b, mut mejor_a) = (i32::MAX, 0_usize, 0_usize);
    if let Some(capa) = dp.get(ultimo) {
        for (b, fila) in capa.iter().enumerate() {
            for (a, c) in fila.iter().enumerate() {
                if *c < mejor_c {
                    mejor_c = *c;
                    mejor_b = b;
                    mejor_a = a;
                }
            }
        }
    }
    let mut eleccion = vec![0_usize; n];
    if let Some(slot) = eleccion.get_mut(ultimo) {
        *slot = mejor_b;
    }
    if n >= 2 {
        if let Some(slot) = eleccion.get_mut(ultimo - 1) {
            *slot = mejor_a;
        }
        let (mut b, mut a) = (mejor_b, mejor_a);
        let mut i = ultimo;
        while i >= 2 {
            let p = de
                .get(i)
                .and_then(|c| c.get(b))
                .and_then(|f| f.get(a))
                .copied()
                .filter(|x| *x != usize::MAX)
                .unwrap_or(0);
            if let Some(slot) = eleccion.get_mut(i - 2) {
                *slot = p;
            }
            i -= 1;
            b = a;
            a = p;
        }
    }

    for ((g, ops), e) in grupos.iter().zip(&opciones).zip(&eleccion) {
        let Some(op) = ops.get(*e) else { continue };
        for (idx, d) in g.indices.iter().zip(op.iter()) {
            if let Some(slot) = dedos.get_mut(*idx) {
                *slot = Dedo::desde(*d);
            }
        }
    }
}

/// Coste de una digitacion concreta sobre un pasaje de una nota a la vez.
///
/// Es **la funcion objetivo que minimiza `digitar`**, expuesta a proposito. El modelo de
/// coste es parte del contrato de esta feature —la especificacion nombra las reglas de
/// Parncutt una por una—, no un detalle interno. Y sin poder puntuar una digitacion dada
/// no hay forma de comprobar regla por regla que el modelo las aplica todas: dos reglas
/// (recolocacion de mano y paso del pulgar) faltaban enteras y ninguna prueba lo noto,
/// porque solo se comprobaba el resultado global de la escala.
///
/// Solo para pasajes monofonicos: en un acorde el coste tambien depende del reparto
/// interno, que `digitar` calcula aparte.
#[must_use]
pub fn coste_de(keys: &[u8], dedos: &[u8], mano: Mano) -> i32 {
    let mut t = 0_i32;
    for (d, k) in dedos.iter().zip(keys.iter()) {
        t = t.saturating_add(coste_nota(*d, *k));
    }
    for (pd, pk) in dedos.windows(2).zip(keys.windows(2)) {
        if let ([da, db], [ka, kb]) = (pd, pk) {
            let v = vano_canonico(Dedo::desde(*da), *ka, Dedo::desde(*db), *kb, mano);
            t = t.saturating_add(coste_transicion(*da, *ka, *db, *kb, v));
        }
    }
    for td in dedos.windows(3) {
        if let [a, b, c] = td {
            t = t.saturating_add(coste_terceto(*a, *b, *c));
        }
    }
    t
}

/// Propone un dedo para cada nota de la cancion.
///
/// `manos` dice de que mano es cada nota, en el mismo orden que `Song::notes`. Cada mano se
/// digita **por separado**: lo que haga la izquierda no altera la propuesta de la derecha
/// (FR-033).
#[must_use]
pub fn digitar(cancion: &Song, manos: &[Mano]) -> Digitacion {
    let notas = cancion.notes();
    let mut dedos = vec![Dedo::D3; notas.len()];

    for mano in [Mano::Izquierda, Mano::Derecha] {
        // Agrupar por instante de ataque: las simultaneas van juntas.
        let mut grupos: Vec<Grupo> = Vec::new();
        for (i, n) in notas.iter().enumerate() {
            if manos.get(i).copied() != Some(mano) {
                continue;
            }
            match grupos.last_mut() {
                Some(g) if notas.get(*g.indices.first().unwrap_or(&0)).map(|p| p.onset_tick)
                    == Some(n.onset_tick) =>
                {
                    g.indices.push(i);
                    g.keys.push(n.key);
                }
                _ => grupos.push(Grupo { indices: vec![i], keys: vec![n.key] }),
            }
        }
        // Dentro de cada grupo, orden por altura relativa a la mano: asi los dedos
        // crecientes caen donde deben en las dos manos.
        for g in &mut grupos {
            let mut pares: Vec<(usize, u8)> =
                g.indices.iter().copied().zip(g.keys.iter().copied()).collect();
            pares.sort_by_key(|(_, k)| h(*k, mano));
            g.indices = pares.iter().map(|(i, _)| *i).collect();
            g.keys = pares.iter().map(|(_, k)| *k).collect();
        }
        digitar_mano(&grupos, mano, &mut dedos);
    }

    Digitacion { dedos }
}
