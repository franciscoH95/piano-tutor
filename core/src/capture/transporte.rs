//! El puente entre el hilo del sistema operativo y el consumidor.
//!
//! Es el unico codigo del proyecto que se ejecuta dentro del callback de tiempo real, y
//! por eso obedece tres reglas sin excepcion: **no asigna memoria, no toma cerrojos y no
//! espera**. Cuando no hay sitio descarta lo entrante y lo cuenta, porque bloquear al
//! hilo del sistema haria que el driver descartase por su cuenta, sin que nos enterasemos.

use crate::capture::evento::{EventoCrudo, Observacion};
use crate::time::Micros;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, OnceLock};
use std::thread::Thread;

/// Lo que comparten los dos extremos. Todo atomico: ningun cerrojo en la ruta critica.
#[derive(Debug, Default)]
struct Compartido {
    descartados: AtomicU32,
    retrocesos: AtomicU32,
    quiere_despertar: AtomicBool,
    /// El hilo consumidor, registrado la primera vez que se duerme.
    hilo: OnceLock<Thread>,
}

/// Extremo del productor. Vive dentro del callback del sistema operativo.
#[derive(Debug)]
pub struct Emisor {
    tx: rtrb::Producer<EventoCrudo>,
    compartido: Arc<Compartido>,
    ultimo: Micros,
    seq: u32,
}

impl Emisor {
    /// Publica una observacion. **Nunca bloquea y nunca asigna.**
    ///
    /// Si no hay sitio, descarta la observacion **entrante** e incrementa el contador.
    /// Nunca descarta lo ya almacenado: hacerlo dejaria ataques sin su suelta.
    ///
    /// Aplica ademas la sujecion de monotonia de FR-013: si el instante retrocediese, se
    /// sujeta al ultimo entregado y se cuenta. Con un reloj monotono y un solo hilo de
    /// callback esto no ocurre nunca, pero es barato y convierte una invariante en algo
    /// garantizado en lugar de confiado.
    #[inline]
    pub fn emitir(&mut self, o: Observacion) {
        let at = if o.at < self.ultimo {
            self.compartido.retrocesos.fetch_add(1, Ordering::Relaxed);
            self.ultimo
        } else {
            o.at
        };
        self.ultimo = at;

        let ev = EventoCrudo {
            at,
            seq: self.seq,
            key: o.key,
            velocity: o.velocity,
            kind: o.kind,
            channel: o.channel,
        };
        // El `seq` avanza tambien cuando se descarta: eso es lo que deja el hueco que
        // localiza la perdida.
        self.seq = self.seq.wrapping_add(1);

        if self.tx.push(ev).is_err() {
            self.compartido.descartados.fetch_add(1, Ordering::Relaxed);
        } else if self.compartido.quiere_despertar.swap(false, Ordering::SeqCst) {
            if let Some(h) = self.compartido.hilo.get() {
                h.unpark();
            }
        }
    }

    /// Cuantas observaciones se han descartado por desbordamiento.
    #[must_use]
    pub fn descartados(&self) -> u32 {
        self.compartido.descartados.load(Ordering::Relaxed)
    }

    /// Cuantas veces se sujeto un instante que retrocedia.
    #[must_use]
    pub fn retrocesos(&self) -> u32 {
        self.compartido.retrocesos.load(Ordering::Relaxed)
    }
}

/// Extremo del consumidor.
#[derive(Debug)]
pub struct Receptor {
    rx: rtrb::Consumer<EventoCrudo>,
    compartido: Arc<Compartido>,
}

impl Receptor {
    /// Vacia lo disponible en `destino` y devuelve cuantos eventos habia.
    pub fn recoger(&mut self, destino: &mut Vec<EventoCrudo>) -> usize {
        let mut n = 0;
        while let Ok(ev) = self.rx.pop() {
            destino.push(ev);
            n += 1;
        }
        n
    }

    /// Duerme hasta que el productor avise. **No sondea.**
    ///
    /// El testigo de `unpark` es pegajoso: si el aviso llega antes de que este hilo se
    /// duerma, `park` retorna de inmediato. La doble comprobacion de `is_empty` cierra la
    /// carrera en la que el productor publica justo entre la primera comprobacion y el
    /// registro del testigo.
    pub fn esperar(&mut self) {
        let _ = self.compartido.hilo.set(std::thread::current());
        if !self.rx.is_empty() {
            return;
        }
        self.compartido.quiere_despertar.store(true, Ordering::SeqCst);
        if self.rx.is_empty() {
            std::thread::park();
        }
        self.compartido.quiere_despertar.store(false, Ordering::SeqCst);
    }

    /// Igual que [`esperar`](Self::esperar), pero **se rinde pasado `plazo`**.
    ///
    /// Existe porque `park()` a secas deja al consumidor dormido para siempre en cuanto el
    /// productor calla, y el productor calla justo cuando mas importa despertar: al
    /// desenchufar el teclado. Sin plazo, el bucle de consumo no vuelve a mirar su bandera
    /// de parada, no termina, no suelta la `Captura` y no se ejecuta el `Drop` que cierra el
    /// puerto. En Windows el puerto de entrada es exclusivo: no cerrarlo deja el teclado
    /// inservible hasta que muere el proceso.
    ///
    /// El plazo es un **tope**, no una espera: si llega algo antes, vuelve enseguida.
    pub fn esperar_hasta(&mut self, plazo: std::time::Duration) {
        let _ = self.compartido.hilo.set(std::thread::current());
        if !self.rx.is_empty() {
            return;
        }
        self.compartido.quiere_despertar.store(true, Ordering::SeqCst);
        if self.rx.is_empty() {
            std::thread::park_timeout(plazo);
        }
        self.compartido.quiere_despertar.store(false, Ordering::SeqCst);
    }

    /// `true` si no hay nada pendiente de recoger.
    #[must_use]
    pub fn vacio(&self) -> bool {
        self.rx.is_empty()
    }

    /// Cuantas observaciones se han descartado por desbordamiento.
    #[must_use]
    pub fn descartados(&self) -> u32 {
        self.compartido.descartados.load(Ordering::Relaxed)
    }
}

/// Crea el par productor/consumidor con capacidad fija, reservada de una sola vez.
///
/// Toda la memoria del transporte se pide aqui. Despues de esta llamada, la ruta critica
/// no vuelve a asignar.
#[must_use]
pub fn canal(capacidad: usize) -> (Emisor, Receptor) {
    let (tx, rx) = rtrb::RingBuffer::<EventoCrudo>::new(capacidad);
    let compartido = Arc::new(Compartido::default());
    (
        Emisor { tx, compartido: Arc::clone(&compartido), ultimo: Micros::ZERO, seq: 0 },
        Receptor { rx, compartido },
    )
}
