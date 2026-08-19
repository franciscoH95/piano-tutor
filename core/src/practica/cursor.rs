//! Donde esta la practica y como se mueve.
//!
//! # La regla que gobierna este modulo
//!
//! **La posicion del nucleo y la que pinta la pantalla son la misma funcion.** El nucleo
//! emite un *ancla* y la interfaz interpola desde ella con `posicionEn`, en
//! `src/practica/modelo.ts`. Por eso aqui:
//!
//! - la posicion es una **funcion pura del ancla**, no un campo que se va acumulando;
//! - se multiplica **antes** de dividir, y hay **una sola division**;
//! - **no se guarda residuo**.
//!
//! Lo ultimo va contra el instinto: un residuo haria la aritmetica interna exacta. Pero
//! haria al nucleo *mas preciso que el ancla que emite*, y como la pantalla solo tiene el
//! ancla, el cursor que ve el alumno se separaria del que el nucleo cree. Ser exacto por
//! dentro y mentir por fuera es peor que ser consistente: el alumno no ve el campo, ve el
//! pixel. La ausencia de deriva se consigue de otra forma —rebasando el ancla **solo
//! cuando el regimen cambia de verdad**— y se comprueba contra la implementacion de la
//! pantalla en `fixtures/paridad-cursor.json`.

use crate::practica::manos::Mano;
use crate::practica::puertas::ProgramaDePuertas;
use crate::practica::sonando::MascaraTeclas;
use crate::time::Micros;
use crate::Song;

/// Como avanza la practica. Un **dato**, no un comportamiento.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Avance {
    /// El reloj manda: la cancion suena aunque el alumno no toque.
    #[default]
    PorReloj,
    /// El alumno manda: la cancion espera en cada nota pendiente.
    PorAcierto,
}

/// Proporcion respecto al tempo original.
///
/// Racional y **reducido en el constructor**: 2/4 y 1/2 tienen que ser el mismo valor, o
/// comparar regimenes campo a campo tratara un ajuste redundante como un cambio real, y
/// cada cambio real rebasa el ancla.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Velocidad {
    num: u32,
    den: u32,
}

const fn mcd(a: u32, b: u32) -> u32 {
    if b == 0 {
        a
    } else {
        mcd(b, a % b)
    }
}

impl Velocidad {
    /// El tempo del archivo.
    pub const NORMAL: Self = Self { num: 1, den: 1 };
    /// Detenida.
    ///
    /// Es `0/1`, **nunca `1/0`**: con denominador cero la division entera revienta, y el
    /// guarda defensivo de `modelo.ts` (`ancla.den === 0 ? 0`) pasaria a ser la ruta normal
    /// en vez de una red de seguridad.
    pub const PAUSA: Self = Self { num: 0, den: 1 };

    /// Una velocidad, o `None` si el denominador es cero.
    #[must_use]
    pub const fn nueva(num: u32, den: u32) -> Option<Self> {
        if den == 0 {
            return None;
        }
        // `mcd(0, d) == d`, asi que `0/7` se reduce a `0/1` y sale igual a `PAUSA` gratis.
        let g = mcd(num, den);
        if g == 0 {
            return Some(Self::PAUSA);
        }
        Some(Self { num: num / g, den: den / g })
    }

    /// Numerador, ya reducido.
    #[must_use]
    pub const fn num(self) -> u32 {
        self.num
    }

    /// Denominador, ya reducido. Nunca cero.
    #[must_use]
    pub const fn den(self) -> u32 {
        self.den
    }

    /// Esta detenida.
    #[must_use]
    pub const fn es_pausa(self) -> bool {
        self.num == 0
    }
}

/// El punto de referencia desde el que la pantalla interpola.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Ancla {
    /// Posicion dentro de la cancion en ese instante.
    pub posicion_us: Micros,
    /// Instante del reloj de sesion en que se tomo.
    pub instante_us: Micros,
    /// Velocidad como racional.
    pub num: u32,
    /// Denominador. Nunca cero.
    pub den: u32,
    /// Hasta donde puede avanzar.
    pub tope_us: Option<Micros>,
}

/// Posicion dentro de la cancion en el instante `ahora`, interpolando desde `ancla`.
///
/// **Es el espejo exacto de `posicionEn` en `src/practica/modelo.ts`**, y esta expuesta
/// como funcion libre precisamente para poder afirmarlo: las dos se comprueban contra los
/// mismos vectores en `fixtures/paridad-cursor.csv`. Si una cambia y la otra no, falla una
/// de las dos pruebas.
///
/// Multiplica **antes** de dividir y hace **una sola division**. Dividir primero "para no
/// desbordar" separaria las dos implementaciones hasta en `num - 1` microsegundos.
#[must_use]
pub fn posicion_en(ancla: &Ancla, ahora: Micros) -> Micros {
    if ancla.den == 0 {
        return ancla.posicion_us;
    }
    let transcurrido = ahora.saturating_sub(ancla.instante_us);
    // `u128` en el intermedio y `try_from` a la salida, no `as`: `Velocidad::nueva` acepta
    // hasta `u32::MAX` y una cancion puede durar 24 h, asi que el producto deja de caber en
    // `u64`. Con `u64` crudo seria panico en debug y valor silencioso en release: la misma
    // entrada con dos salidas segun el perfil de compilacion.
    let producto = u128::from(transcurrido.0) * u128::from(ancla.num);
    let avance = u64::try_from(producto / u128::from(ancla.den)).unwrap_or(u64::MAX);
    let proyectada = Micros(ancla.posicion_us.0.saturating_add(avance));
    match ancla.tope_us {
        Some(tope) if proyectada.0 > tope.0 => tope,
        _ => proyectada,
    }
}

/// Lo que cambio en un avance.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Paso {
    /// Donde esta la practica.
    pub posicion: Micros,
    /// `Some` **solo al cambiar de regimen**. La interfaz interpola entre anclas y no
    /// necesita que esto cruce sesenta veces por segundo.
    pub ancla: Option<Ancla>,
    /// La cancion acaba de llegar al final. Es un **flanco**, no un nivel.
    pub terminada: bool,
    /// El cursor esta detenido en una puerta, esperando a que el alumno acierte.
    pub esperando: bool,
}

/// El regimen: la quintupla que determina por completo donde estara el cursor.
///
/// El ancla se emite **por diferencia** de este valor, no por llamada. Asi las diez causas
/// de cambio de regimen salen correctas por construccion sin enumerarlas, y repetir un
/// mando con el mismo valor no rebasa: un deslizador de React controlado reemite su valor
/// en cada fotograma, y serian 36.000 falsos rebases en diez minutos.
type Regimen = (Micros, Micros, u32, u32, Micros);

/// Donde esta la practica.
pub struct Cursor {
    velocidad: Velocidad,
    /// A que velocidad volver al reanudar. **Nunca puede valer `PAUSA`**, o pausar dos
    /// veces dejaria la cancion parada para siempre.
    velocidad_previa: Velocidad,
    ancla_real: Micros,
    ancla_cancion: Micros,
    /// Monotoniza el reloj. Dominio de reloj, no de posicion.
    ultimo_t: Micros,
    fin: Micros,
    /// El **nivel** del instante anterior, no un pestillo de una via: si fuese un pestillo,
    /// volver al principio y llegar otra vez al final no avisaria.
    estaba_al_final: bool,
    regimen_emitido: Regimen,
    avance: Avance,
    puertas: ProgramaDePuertas,
    /// La puerta pendiente. Solo avanza, salvo al saltar hacia atras.
    puerta: usize,
}

impl Cursor {
    /// Un cursor al principio de la cancion, en pausa.
    #[must_use]
    pub fn nuevo(cancion: &Song) -> Self {
        let fin = cancion.duration_us();
        let mut c = Self {
            velocidad: Velocidad::PAUSA,
            velocidad_previa: Velocidad::NORMAL,
            ancla_real: Micros::ZERO,
            ancla_cancion: Micros::ZERO,
            ultimo_t: Micros::ZERO,
            fin,
            // Falso incluso en una cancion vacia, donde `ha_terminado()` ya es cierto en la
            // posicion cero: asi el flanco se emite en el primer `avanzar()`, una vez.
            estaba_al_final: false,
            regimen_emitido: (Micros::ZERO, Micros::ZERO, 0, 1, fin),
            avance: Avance::PorReloj,
            puertas: ProgramaDePuertas::default(),
            puerta: 0,
        };
        c.regimen_emitido = c.regimen();
        c
    }

    /// Un cursor con el programa de puertas del modo espera.
    #[must_use]
    pub fn nuevo_con_puertas(cancion: &Song, manos: &[Mano], practicada: Option<Mano>) -> Self {
        let mut c = Self::nuevo(cancion);
        c.puertas = ProgramaDePuertas::nuevo(cancion, manos, practicada);
        c.regimen_emitido = c.regimen();
        c
    }

    /// El techo del avance. Incluye **siempre** el final de la cancion, y en modo espera
    /// tambien la puerta pendiente.
    ///
    /// La ausencia de puerta es el infinito, **jamas el cero**: con cero el cursor se
    /// congelaria para siempre en cuanto se acabasen las puertas.
    fn techo(&self) -> Micros {
        let puerta = match self.avance {
            Avance::PorAcierto => self
                .puertas
                .get(self.puerta)
                .map_or(Micros(u64::MAX), |p| p.onset_us),
            Avance::PorReloj => Micros(u64::MAX),
        };
        if puerta.0 < self.fin.0 {
            puerta
        } else {
            self.fin
        }
    }

    fn regimen(&self) -> Regimen {
        (
            self.ancla_cancion,
            self.ancla_real,
            self.velocidad.num,
            self.velocidad.den,
            self.techo(),
        )
    }

    /// El ancla vigente, la misma que interpola la pantalla.
    #[must_use]
    pub fn ancla(&self) -> Ancla {
        Ancla {
            posicion_us: self.ancla_cancion,
            instante_us: self.ancla_real,
            num: self.velocidad.num,
            den: self.velocidad.den,
            tope_us: Some(self.techo()),
        }
    }

    /// Posicion proyectada en el instante `t`.
    ///
    /// Delega en `posicion_en`, **la misma funcion que usa la pantalla**. No hay una
    /// aritmetica del nucleo y otra de la interfaz: hay una sola, y esta ahi.
    fn proyectar(&self, t: Micros) -> Micros {
        posicion_en(&self.ancla(), t)
    }

    /// Instante efectivo: el reloj monotonizado. Un reloj que retrocede no puede hacer
    /// retroceder el cursor ni provocar una resta negativa.
    fn instante(&mut self, ahora: Micros) -> Micros {
        if ahora > self.ultimo_t {
            self.ultimo_t = ahora;
        }
        self.ultimo_t
    }

    /// Rebasa el ancla: calcula con el regimen **viejo** y refresca el instante.
    ///
    /// El orden no es negociable. Calcular con el regimen nuevo hace retroceder el cursor
    /// al bajar de velocidad; no refrescar el instante lo hace saltar hacia delante.
    fn rebasar(&mut self, t: Micros) {
        self.ancla_cancion = self.proyectar(t);
        self.ancla_real = t;
    }

    /// Emite ancla solo si el regimen cambio de verdad.
    fn emitir(&mut self) -> Option<Ancla> {
        let ahora = self.regimen();
        if ahora == self.regimen_emitido {
            return None;
        }
        self.regimen_emitido = ahora;
        Some(self.ancla())
    }

    /// Adelanta la practica hasta el instante del reloj.
    pub fn avanzar(&mut self, ahora: Micros) -> Paso {
        self.avanzar_con(ahora, MascaraTeclas::VACIA)
    }

    /// Adelanta la practica sabiendo que teclas tiene pulsadas el alumno.
    ///
    /// En `PorAcierto`, si el cursor esta en la puerta pendiente y **todas** sus teclas
    /// estan pulsadas a la vez, la puerta se abre y el ancla se rebasa **en este instante**.
    /// Rebasarla en el de llegada a la puerta convertiria la duda del alumno en avance de
    /// cancion: treinta segundos pensandolo serian treinta segundos de musica de golpe.
    pub fn avanzar_con(&mut self, ahora: Micros, pulsadas: MascaraTeclas) -> Paso {
        let t = self.instante(ahora);
        let mut posicion = self.proyectar(t);

        if self.avance == Avance::PorAcierto {
            // Se abren todas las puertas satisfechas de golpe: dos notas simultaneas de la
            // misma mano son una sola puerta, pero un acorde muy rapido puede dejar dos
            // seguidas en el mismo instante.
            while let Some(p) = self.puertas.get(self.puerta) {
                if posicion.0 < p.onset_us.0 || !pulsadas.contiene_todas(p.teclas) {
                    break;
                }
                self.puerta = self.puerta.saturating_add(1);
                self.ancla_cancion = posicion;
                self.ancla_real = t;
                posicion = self.proyectar(t);
            }
        }

        let al_final = posicion.0 >= self.fin.0;
        let flanco = al_final && !self.estaba_al_final;
        self.estaba_al_final = al_final;
        let esperando = self.avance == Avance::PorAcierto
            && self
                .puertas
                .get(self.puerta)
                .is_some_and(|p| posicion.0 >= p.onset_us.0);
        Paso { posicion, ancla: self.emitir(), terminada: flanco, esperando }
    }

    /// Cambia entre reproducir y esperar. **Conserva la posicion** (FR-021).
    pub fn cambiar_avance(&mut self, avance: Avance, ahora: Micros) -> Option<Ancla> {
        if avance == self.avance {
            return None;
        }
        let t = self.instante(ahora);
        self.rebasar(t);
        self.avance = avance;
        // La puerta pendiente es la siguiente **por delante**, no una ya pasada: al activar
        // el modo espera a mitad de cancion, volver a una puerta anterior seria retroceder.
        self.puerta = self.puertas.desde(self.ancla_cancion);
        self.emitir()
    }

    /// Salta la puerta pendiente sin acertarla (FR-020).
    ///
    /// Es la salida para cuando el modo espera no puede satisfacerse: una nota que el
    /// teclado del alumno no tiene dejaria la practica atascada para siempre. Salta **una**
    /// puerta, no todas: saltarlas todas equivaldria a apagar el modo sin decirlo.
    pub fn saltar_puerta(&mut self, ahora: Micros) -> Option<Ancla> {
        if self.avance != Avance::PorAcierto || self.puertas.get(self.puerta).is_none() {
            return None;
        }
        let t = self.instante(ahora);
        self.rebasar(t);
        self.puerta = self.puerta.saturating_add(1);
        self.emitir()
    }

    /// Rehace el programa de puertas, por ejemplo al elegir otra mano.
    pub fn practicar_mano(
        &mut self,
        cancion: &Song,
        manos: &[Mano],
        practicada: Option<Mano>,
        ahora: Micros,
    ) -> Option<Ancla> {
        let t = self.instante(ahora);
        self.rebasar(t);
        self.puertas = ProgramaDePuertas::nuevo(cancion, manos, practicada);
        self.puerta = self.puertas.desde(self.ancla_cancion);
        self.emitir()
    }

    /// El modo de avance vigente.
    #[must_use]
    pub const fn avance(&self) -> Avance {
        self.avance
    }

    /// Las teclas que hay que pulsar para pasar, si el cursor esta esperando.
    #[must_use]
    pub fn pendiente(&self) -> Option<MascaraTeclas> {
        if self.avance != Avance::PorAcierto {
            return None;
        }
        let p = self.puertas.get(self.puerta)?;
        (self.posicion().0 >= p.onset_us.0).then_some(p.teclas)
    }

    /// Pone la cancion en marcha a la velocidad de practica.
    pub fn poner_en_marcha(&mut self, ahora: Micros) -> Option<Ancla> {
        let v = self.velocidad_previa;
        self.cambiar_velocidad(v, ahora)
    }

    /// Detiene el avance sin perder la posicion.
    pub fn pausar(&mut self, ahora: Micros) -> Option<Ancla> {
        self.cambiar_velocidad(Velocidad::PAUSA, ahora)
    }

    /// Cambia la velocidad sin mover la posicion.
    pub fn cambiar_velocidad(&mut self, v: Velocidad, ahora: Micros) -> Option<Ancla> {
        if v == self.velocidad {
            return None;
        }
        let t = self.instante(ahora);
        self.rebasar(t);
        // Solo se recuerda una velocidad de practica: guardar la vigente sin mirar dejaria
        // `velocidad_previa = PAUSA` al pausar dos veces.
        if !v.es_pausa() {
            self.velocidad_previa = v;
        }
        self.velocidad = v;
        self.emitir()
    }

    /// Lleva el cursor a una posicion concreta, conservando el modo.
    pub fn saltar_a(&mut self, posicion: Micros, ahora: Micros) -> Option<Ancla> {
        let destino = if posicion.0 > self.fin.0 { self.fin } else { posicion };
        let t = self.instante(ahora);
        // Si ya se esta ahi, no hay nada que hacer. Basta comparar la POSICION, no el
        // ancla: dos anclas distintas pueden describir la misma trayectoria —(0, 0, 1/1) y
        // (4s, 4s, 1/1) dan lo mismo para todo instante posterior—, y rebasar por un salto
        // que no mueve nada truncaria la division sin motivo.
        if destino == self.proyectar(t) {
            return None;
        }
        self.ancla_cancion = destino;
        self.ancla_real = t;
        // Saltar hacia atras **rearma** el aviso de final.
        self.estaba_al_final = destino.0 >= self.fin.0;
        self.emitir()
    }

    /// Donde esta la practica ahora mismo, sin mover el reloj.
    #[must_use]
    pub fn posicion(&self) -> Micros {
        self.proyectar(self.ultimo_t)
    }

    /// La velocidad vigente.
    #[must_use]
    pub const fn velocidad(&self) -> Velocidad {
        self.velocidad
    }

    /// La cancion llego al final.
    #[must_use]
    pub fn ha_terminado(&self) -> bool {
        self.posicion().0 >= self.fin.0
    }
}
