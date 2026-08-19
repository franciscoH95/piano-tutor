//! La maquina que juzga, alimentada **en linea**.
//!
//! FR-004 prohibe que una nota ya juzgada cambie de veredicto por lo que venga despues, y
//! eso descarta el emparejamiento optimo global —Hungarian, DTW, programacion dinamica
//! sobre la interpretacion entera—, porque todos revisan decisiones a la luz del futuro.
//! Obliga a decidir sobre la marcha, y a aceptar que a veces se decidira peor de lo que se
//! habria decidido sabiendolo todo. Esa perdida es el precio de FR-004, y se mide aparte.

use crate::capture::Observacion;
use crate::evaluacion::pulsacion::Pulsaciones;
use crate::evaluacion::resultado::{contar, desfase, sistematico};
use crate::evaluacion::tolerancias::{Nivel, Tolerancias};
use crate::evaluacion::{es_evaluable, Medida, Recuento, Resultado, Veredicto};
use crate::practica::Mano;
use crate::time::Micros;
use crate::Song;

/// Una nota que el alumno si tiene que tocar.
struct EnJuego {
    indice: usize,
    key: u8,
    /// Cuando deberia sonar. Se calcula **una vez** y no se recalcula (FR-004).
    esperado: Micros,
    duracion: u64,
    mano: Mano,
    tomada: bool,
    veredicto: Veredicto,
    medida: Option<Medida>,
}

/// Juzga una interpretacion.
pub struct Evaluador {
    tol: Tolerancias,
    notas: Vec<EnJuego>,
    entrada: Pulsaciones,
    /// Primera pulsacion todavia sin emparejar. Solo avanza.
    siguiente: usize,
    /// Pulsaciones que no se emparejaron con ninguna nota.
    ///
    /// Se guardan y se clasifican **al cerrar**, no al llegar. FR-004 prohibe revisar el
    /// veredicto de una NOTA; esto es la etiqueta de una pulsacion suelta, que es otra cosa.
    /// Y hace falta: el dedo que resbala suele llegar **antes** que el acierto al que esta
    /// pegado —se roza el Fa y luego se toca el Mi—, asi que al llegar todavia no hay
    /// ningun acierto al que estar proximo.
    sueltas: Vec<(u8, Micros)>,
    fuera_de_alcance: usize,
    no_intentadas: usize,
    examinadas: usize,
}

impl Evaluador {
    /// Prepara la evaluacion de una cancion, a tempo normal y desde el principio.
    #[must_use]
    pub fn nuevo(cancion: &Song, manos: &[Mano], practicada: Option<Mano>, nivel: Nivel) -> Self {
        let mut notas = Vec::new();
        let mut fuera_de_alcance = 0;
        for (i, n) in cancion.notes().iter().enumerate() {
            let mano = manos.get(i).copied().unwrap_or(Mano::Derecha);
            if es_evaluable(n.channel, n.key, mano, practicada) {
                notas.push(EnJuego {
                    indice: i,
                    key: n.key,
                    esperado: n.onset_us,
                    duracion: n.end_us.0.saturating_sub(n.onset_us.0),
                    mano,
                    tomada: false,
                    veredicto: Veredicto::Omitida,
                    medida: None,
                });
            } else if !n.is_on_88_keys() && practicada.is_none_or(|m| m == mano) {
                // Una nota de la pieza que el alumno **no puede** tocar. Se cuenta aparte y
                // fuera del denominador: no es un fallo suyo (FR-014).
                fuera_de_alcance += 1;
            }
        }
        Self {
            tol: nivel.tolerancias(),
            notas,
            entrada: Pulsaciones::nuevas(),
            siguiente: 0,
            sueltas: Vec::new(),
            fuera_de_alcance,
            no_intentadas: 0,
            examinadas: 0,
        }
    }

    /// Un ataque o una suelta del alumno.
    ///
    /// **Ruta critica**: no asigna por evento salvo el crecimiento amortizado del vector de
    /// pulsaciones, y su coste no depende del tamaño de la cancion.
    pub fn observar(&mut self, obs: Observacion) {
        self.entrada.observar(obs);
    }

    /// La practica llego hasta aqui: empareja lo pendiente y cierra lo que ya vencio.
    pub fn avanzar(&mut self, hasta: Micros) {
        self.emparejar_pendientes();
        self.vencer(hasta);
    }

    /// El alumno salto ese pasaje: no lo intento, asi que no cuenta como fallado (FR-013).
    pub fn saltar(&mut self, desde: Micros, hasta: Micros) {
        for n in &mut self.notas {
            if n.tomada || n.esperado.0 < desde.0 || n.esperado.0 > hasta.0 {
                continue;
            }
            n.veredicto = Veredicto::NoIntentada;
            n.tomada = true; // ya no puede recibir pulsacion ni vencer
            self.no_intentadas += 1;
        }
    }

    /// Cierra la interpretacion y devuelve el resumen.
    #[must_use]
    pub fn cerrar(mut self, hasta: Micros) -> Resultado {
        self.emparejar_pendientes();
        self.vencer(hasta);
        self.resumir()
    }

    /// El veredicto de la nota `indice` de `Song::notes`.
    #[must_use]
    pub fn veredicto_de(&self, indice: usize) -> Veredicto {
        self.notas
            .iter()
            .find(|n| n.indice == indice)
            .map_or(Veredicto::FueraDeAlcance, |n| n.veredicto)
    }

    /// La medida de la nota `indice`, si se emparejo.
    #[must_use]
    pub fn medida_de(&self, indice: usize) -> Option<Medida> {
        self.notas.iter().find(|n| n.indice == indice).and_then(|n| n.medida)
    }

    /// Cuantas notas se han examinado. Se **cuenta**, no se cronometra: cronometrar seria
    /// intermitente y no demostraria nada estructural.
    #[must_use]
    pub const fn examinadas(&self) -> usize {
        self.examinadas
    }

    /// Empareja cada pulsacion nueva con su nota, o la declara suelta.
    fn emparejar_pendientes(&mut self) {
        let vistas = self.entrada.vistas().len();
        for i in self.siguiente..vistas {
            let Some(p) = self.entrada.vistas().get(i).copied() else {
                continue;
            };
            match self.mejor_candidata(p.key, p.ataque_us) {
                Some(k) => {
                    let Some(n) = self.notas.get_mut(k) else {
                        continue;
                    };
                    let d = desfase(p.ataque_us, n.esperado);
                    n.tomada = true;
                    #[allow(clippy::cast_possible_wrap)]
                    let escrita = n.duracion as i64;
                    n.medida = Some(Medida {
                        desfase_us: d,
                        duracion_us: p.final_us.map(|f| {
                            desfase(f, p.ataque_us).wrapping_sub(escrita)
                        }),
                        velocity: p.velocity,
                    });
                    // **Aqui manda el nivel, y solo aqui.** El emparejamiento de arriba no
                    // lo mira: por eso el permisivo no puede dar menos aciertos (SC-006).
                    #[allow(clippy::cast_possible_wrap)]
                    let ventana = self.tol.ventana_ataque_us as i64;
                    n.veredicto = if d.abs() <= ventana {
                        Veredicto::Acertada
                    } else {
                        Veredicto::TocadaFueraDeTiempo
                    };
                }
                None => self.sueltas.push((p.key, p.ataque_us)),
            }
        }
        self.siguiente = vistas;
    }

    /// La nota libre de esa tecla mas cercana en el tiempo, dentro de la ventana de
    /// emparejamiento.
    ///
    /// La ventana es **la misma en los tres niveles**: si dependiera del nivel, cambiar de
    /// nivel cambiaria *que* se empareja con que, y una nota podria quedar acertada en el
    /// exigente y sin pareja en el permisivo.
    ///
    /// Desempate: la mas temprana. Es arbitrario pero **fijo**, que es lo que SC-005 exige;
    /// dejarlo al orden de recorrido lo haria depender de la implementacion.
    fn mejor_candidata(&mut self, key: u8, ataque: Micros) -> Option<usize> {
        let mut mejor: Option<(usize, u64)> = None;
        for (k, n) in self.notas.iter().enumerate() {
            if n.tomada || n.key != key {
                continue;
            }
            self.examinadas = self.examinadas.saturating_add(1);
            let distancia = ataque.0.abs_diff(n.esperado.0);
            if distancia > self.tol.ventana_emparejamiento_us {
                continue;
            }
            match mejor {
                Some((_, d)) if d <= distancia => {}
                _ => mejor = Some((k, distancia)),
            }
        }
        mejor.map(|(k, _)| k)
    }

    /// Si esa pulsacion suelta esta pegada a una nota que si se acerto: un dedo que resbala.
    ///
    /// Es el error mas frecuente de un principiante —roza el Fa y toca el Mi—, y contarlo
    /// igual que tocar un pasaje entero equivocado castiga dos veces el mismo tropiezo y
    /// esconde de que clase de error se trata (FR-010a).
    fn roza_un_acierto(&self, key: u8, ataque: Micros) -> bool {
        self.notas.iter().any(|n| {
            matches!(n.veredicto, Veredicto::Acertada | Veredicto::TocadaFueraDeTiempo)
                && n.key.abs_diff(key) <= self.tol.cercania_dedo_semitonos
                && n.key != key
                && ataque.0.abs_diff(n.esperado.0) <= self.tol.cercania_dedo_us
        })
    }

    /// Declara omitidas las notas cuya ventana ya paso.
    fn vencer(&mut self, hasta: Micros) {
        let ventana = self.tol.ventana_emparejamiento_us;
        for n in &mut self.notas {
            if !n.tomada && n.esperado.0.saturating_add(ventana) < hasta.0 {
                n.tomada = true;
                n.veredicto = Veredicto::Omitida;
            }
        }
    }

    fn resumir(&self) -> Resultado {
        let mut por_mano = [Recuento::default(); 2];
        let mut por_nota = Vec::with_capacity(self.notas.len());
        let (mut acertadas, mut fuera_de_tiempo, mut omitidas) = (0, 0, 0);
        let mut desfases = Vec::new();
        let (mut de_mas, mut dedos_escapados) = (0, 0);
        for (key, ataque) in &self.sueltas {
            if self.roza_un_acierto(*key, *ataque) {
                dedos_escapados += 1;
            } else {
                de_mas += 1;
            }
        }
        for n in &self.notas {
            contar(&mut por_mano, n.mano, n.veredicto);
            por_nota.push((n.indice, n.veredicto));
            match n.veredicto {
                Veredicto::Acertada => acertadas += 1,
                Veredicto::TocadaFueraDeTiempo => fuera_de_tiempo += 1,
                Veredicto::Omitida => omitidas += 1,
                Veredicto::FueraDeAlcance | Veredicto::NoIntentada => {}
            }
            if let Some(m) = n.medida {
                desfases.push(m.desfase_us);
            }
        }
        Resultado {
            acertadas,
            fuera_de_tiempo,
            omitidas,
            de_mas,
            dedos_escapados,
            fuera_de_alcance: self.fuera_de_alcance,
            no_intentadas: self.no_intentadas,
            desfase: sistematico(&desfases, &self.tol),
            sin_tocar: self.entrada.vistas().is_empty(),
            por_mano,
            por_nota,
        }
    }
}
