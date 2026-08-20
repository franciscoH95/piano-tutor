//! La regla de perdida del teclado, ejercida sin hardware.
//!
//! Existe porque hasta el 2026-08-19 el vigia **no se construia en ninguna parte**:
//! `grep -rn Vigia src-tauri/src` no daba ni un resultado. Desenchufar el teclado a media
//! practica no producia nada, la interfaz seguia diciendo «Conectado», y el unico camino
//! que encendia `DispositivoPerdido` era que fallase la apertura inicial. Que no hubiera
//! hardware en integracion continua no era excusa: la regla es logica, no hardware.

use piano_core::capture::{FuenteGuionizada, Observacion, TipoEvento};
use piano_core::time::Micros;
use piano_midi_io::vigia::Presencia;
use piano_tutor_lib::comandos::Estado;
use piano_tutor_lib::reenviador::{bucle, Desenlace, ObservadorDePresencia};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// Un vigia de mentira que entrega un guion de presencias y luego para el bucle.
///
/// Para el bucle al agotarse a proposito: sin eso, la prueba de «una ausencia NO basta»
/// no podria terminar nunca, que es justo lo que tiene que demostrar.
struct Guion {
    pasos: Vec<Presencia>,
    siguiente: usize,
    parar: Arc<AtomicBool>,
}

impl ObservadorDePresencia for Guion {
    fn novedad(&mut self) -> Option<Presencia> {
        match self.pasos.get(self.siguiente) {
            Some(p) => {
                self.siguiente += 1;
                Some(*p)
            }
            None => {
                self.parar.store(true, Ordering::Relaxed);
                None
            }
        }
    }
}

fn correr(pasos: Vec<Presencia>) -> Desenlace {
    let estado = Arc::new(Estado::default());
    let parar = Arc::new(AtomicBool::new(false));
    let mut guion = Guion { pasos, siguiente: 0, parar: Arc::clone(&parar) };
    let mut fuente = FuenteGuionizada::nueva(vec![Observacion {
        at: Micros(1_000),
        key: 60,
        velocity: 90,
        kind: TipoEvento::Ataque,
        channel: 0,
    }]);
    bucle(&mut fuente, &estado, &parar, &mut guion)
}

#[test]
fn una_sola_ausencia_no_apaga_el_teclado() {
    // Puede ser un parpadeo del sistema al reenumerar. Declarar la perdida por ella
    // apagaria el teclado del alumno a mitad de compas, sin que hubiera pasado nada.
    assert_eq!(correr(vec![Presencia::Ausente]), Desenlace::Detenido);
}

#[test]
fn dos_ausencias_seguidas_declaran_la_perdida() {
    assert_eq!(
        correr(vec![Presencia::Ausente, Presencia::Ausente]),
        Desenlace::Perdido
    );
}

#[test]
fn una_presencia_por_medio_reinicia_la_cuenta() {
    // Dos ausencias que NO son consecutivas no son una perdida. Si la cuenta no se
    // reiniciase, un teclado que parpadea cada tanto acabaria dandose por perdido sin
    // haberse movido de su sitio.
    assert_eq!(
        correr(vec![Presencia::Ausente, Presencia::Presente, Presencia::Ausente]),
        Desenlace::Detenido
    );
}

#[test]
fn la_bandera_de_parada_termina_el_bucle_aunque_no_llegue_nada() {
    // La propiedad que hacia falta para poder soltar el puerto: antes el hilo dormia en
    // `park()` sin plazo y no volvia a mirar la bandera nunca.
    let estado = Arc::new(Estado::default());
    let parar = Arc::new(AtomicBool::new(true));
    let mut fuente = FuenteGuionizada::nueva(Vec::new());
    let mut ninguno = piano_tutor_lib::reenviador::SinVigilancia;
    assert_eq!(bucle(&mut fuente, &estado, &parar, &mut ninguno), Desenlace::Detenido);
}
