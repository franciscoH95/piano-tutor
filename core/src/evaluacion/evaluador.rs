//! La maquina que juzga, alimentada **en linea**.
//!
//! FR-004 prohibe que una nota ya juzgada cambie de veredicto por lo que venga despues, y
//! eso descarta el emparejamiento optimo global —Hungarian, DTW, programacion dinamica
//! sobre la interpretacion entera—, porque todos revisan decisiones a la luz del futuro.
//! Obliga a decidir sobre la marcha, y a aceptar que a veces se decidira peor de lo que se
//! habria decidido sabiendolo todo. Esa perdida es el precio de FR-004, y se mide aparte.

use crate::capture::Observacion;
use crate::evaluacion::emparejar::instante_de;
use crate::evaluacion::pulsacion::Pulsaciones;
use crate::evaluacion::resultado::{contar, desfase, sistematico};
use crate::evaluacion::tolerancias::{Nivel, Tolerancias};
use crate::evaluacion::{es_evaluable, Medida, Recuento, Resultado, Veredicto};
use crate::practica::{Ancla, Mano};
use crate::time::Micros;
use crate::Song;

/// Una nota que el alumno si tiene que tocar.
struct EnJuego {
    indice: usize,
    key: u8,
    /// Donde cae en la CANCION.
    onset: Micros,
    /// Cuando deberia sonar **en el reloj de sesion**, que es el eje en el que llegan las
    /// pulsaciones del alumno.
    ///
    /// `None` mientras no se haya podido sellar. Se sella con el ancla vigente y **no se
    /// recalcula una vez la nota esta juzgada** (FR-004); las que aun no lo estan si se
    /// vuelven a sellar al cambiar el ancla, porque su instante esperado cambio de verdad.
    esperado: Option<Micros>,
    duracion: u64,
    mano: Mano,
    tomada: bool,
    /// Si el tiempo de ESTA nota se juzga.
    ///
    /// Por nota y no por intento: el alumno puede cambiar de modo a mitad, y con un
    /// indicador del intento entero habria que descartarlo todo o evaluarlo todo, y las dos
    /// son falsas. Se fija **al emparejar** y no se recalcula (FR-004).
    tiempo_evaluado: bool,
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
    /// Si el regimen actual permite juzgar los tiempos.
    ///
    /// En modo espera **no**: la cancion aguarda al alumno, asi que no se puede llegar
    /// tarde y publicar un desfase seria inventarlo (FR-009a).
    tiempos: bool,
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
                    onset: n.onset_us,
                    esperado: None,
                    duracion: n.end_us.0.saturating_sub(n.onset_us.0),
                    mano,
                    tomada: false,
                    tiempo_evaluado: true,
                    veredicto: Veredicto::Omitida,
                    medida: None,
                });
            } else if !n.is_on_88_keys() && practicada.is_none_or(|m| m == mano) {
                // Una nota de la pieza que el alumno **no puede** tocar. Se cuenta aparte y
                // fuera del denominador: no es un fallo suyo (FR-014).
                fuera_de_alcance += 1;
            }
        }
        let mut evaluador = Self {
            tol: nivel.tolerancias(),
            notas,
            entrada: Pulsaciones::nuevas(),
            siguiente: 0,
            sueltas: Vec::new(),
            fuera_de_alcance,
            no_intentadas: 0,
            examinadas: 0,
            tiempos: true,
        };
        // Se sella con un ancla **identidad**: reloj y cancion coincidiendo, a tempo normal
        // desde cero. Es lo correcto para una interpretacion que empieza al principio con el
        // reloj a cero, y quien tenga un ancla de verdad —`Preparacion`— vuelve a sellar
        // acto seguido. Sin sellar de entrada, un evaluador recien creado no emparejaria
        // nada y ese silencio seria dificil de diagnosticar.
        evaluador.sellar(&Ancla {
            posicion_us: Micros(0),
            instante_us: Micros(0),
            num: 1,
            den: 1,
            tope_us: None,
        });
        evaluador
    }

    /// Sella el instante de reloj en que deberia sonar cada nota todavia sin juzgar.
    ///
    /// **Es lo que traduce entre los dos ejes.** Las notas viven en posiciones de cancion y
    /// las pulsaciones llegan en instantes de reloj; sin esta traduccion, en cuanto la
    /// cancion no arranca con el reloj a cero —al reanudar, al repetir un pasaje— el
    /// emparejamiento compara peras con manzanas y no empareja nada.
    ///
    /// Se proyecta la NOTA hacia el reloj y no la pulsacion hacia la cancion: `posicion_en`
    /// recorta por el tope, asi que una nota tocada mas alla del final se proyectaria al
    /// final y su tardanza se truncaria en silencio.
    pub fn sellar(&mut self, ancla: &Ancla) {
        for n in &mut self.notas {
            if !n.tomada {
                n.esperado = instante_de(ancla, n.onset);
            }
        }
    }

    /// Dice si a partir de ahora se juzgan los tiempos.
    ///
    /// `false` en modo espera. Afecta a las notas que se emparejen **desde ahora**; las ya
    /// juzgadas no se tocan (FR-004).
    pub const fn evaluar_tiempos(&mut self, si: bool) {
        self.tiempos = si;
    }

    /// Un ataque o una suelta del alumno.
    ///
    /// **Ruta critica**: no asigna por evento salvo el crecimiento amortizado del vector de
    /// pulsaciones, y su coste no depende del tamaño de la cancion.
    pub fn observar(&mut self, obs: Observacion) {
        self.entrada.observar(obs);
    }

    /// La practica llego hasta aqui: empareja lo pendiente y cierra lo que ya vencio.
    pub fn avanzar(&mut self, ahora: Micros) {
        self.emparejar_pendientes();
        self.vencer(ahora);
    }

    /// El alumno salto ese pasaje: no lo intento, asi que no cuenta como fallado (FR-013).
    pub fn saltar(&mut self, desde: Micros, hasta: Micros) {
        // El tramo saltado se da en posiciones de CANCION, que es como lo ve el alumno.
        for n in &mut self.notas {
            if n.tomada || n.onset.0 < desde.0 || n.onset.0 > hasta.0 {
                continue;
            }
            n.veredicto = Veredicto::NoIntentada;
            n.tomada = true; // ya no puede recibir pulsacion ni vencer
            self.no_intentadas += 1;
        }
    }

    /// Cierra la interpretacion y devuelve el resumen.
    #[must_use]
    pub fn cerrar(mut self, ahora: Micros) -> Resultado {
        self.emparejar_pendientes();
        self.vencer(ahora);
        self.resumir()
    }

    /// El veredicto de la nota `indice`, **solo si ya es firme**.
    ///
    /// `None` mientras la nota sigue abierta: todavia puede recibir una pulsacion. Pintarla
    /// como omitida antes de tiempo seria acusar al alumno de algo que aun tiene ocasion de
    /// hacer.
    #[must_use]
    pub fn veredicto_firme(&self, indice: usize) -> Option<Veredicto> {
        self.notas.iter().find(|n| n.indice == indice).filter(|n| n.tomada).map(|n| n.veredicto)
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
        let tiempos = self.tiempos;
        let ventana_ataque = self.tol.ventana_ataque_us;
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
                    let Some(esperado) = n.esperado else {
                        continue;
                    };
                    let d = desfase(p.ataque_us, esperado);
                    n.tomada = true;
                    n.tiempo_evaluado = tiempos;
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
                    let ventana = ventana_ataque as i64;
                    // Sin tiempos que juzgar, emparejada es acertada: en modo espera la
                    // cancion aguardo a que el alumno tocase esa nota, y la toco.
                    n.veredicto = if !tiempos || d.abs() <= ventana {
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
            let Some(esperado) = n.esperado else {
                continue; // sin sellar: todavia no se sabe cuando deberia sonar
            };
            if n.tomada || n.key != key {
                continue;
            }
            self.examinadas = self.examinadas.saturating_add(1);
            let distancia = ataque.0.abs_diff(esperado.0);
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
                && n
                    .esperado
                    .is_some_and(|e| ataque.0.abs_diff(e.0) <= self.tol.cercania_dedo_us)
        })
    }

    /// Declara omitidas las notas cuya ventana ya paso, **en el eje del reloj**.
    ///
    /// Una nota sin sellar no puede vencer: no se sabe cuando deberia sonar, asi que
    /// declararla omitida seria acusar al alumno sin base.
    fn vencer(&mut self, ahora: Micros) {
        let ventana = self.tol.ventana_emparejamiento_us;
        for n in &mut self.notas {
            let Some(esperado) = n.esperado else {
                continue;
            };
            if !n.tomada && esperado.0.saturating_add(ventana) < ahora.0 {
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
            // Solo entran en la estadistica las notas cuyo tiempo se juzgo: mezclar las
            // de modo espera daria una mediana calculada sobre desfases que no significan
            // nada.
            if let (Some(m), true) = (n.medida, n.tiempo_evaluado) {
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
            // Se DECLARA parcial. Un resultado incompleto que no se declara incompleto se
            // lee como completo, y el alumno creeria que su ritmo esta bien cuando nadie lo
            // ha mirado (FR-015a).
            parcial: self.notas.iter().any(|n| n.tomada && !n.tiempo_evaluado),
            por_mano,
            por_nota,
        }
    }
}
