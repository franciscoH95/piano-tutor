//! T047-T056 — dónde está la práctica y cómo se mueve.
//!
//! La regla que gobierna todo este módulo: **la posición del núcleo y la que pinta la
//! pantalla son la misma función**. El núcleo no guarda residuos ni es más preciso que el
//! ancla que emite; si lo fuera, el cursor que ve el alumno se separaría del que el núcleo
//! cree, y esa divergencia no la notaría ninguna prueba de un solo lado.

mod fixtures;
use fixtures::SmfBuilder;
use piano_core::clock::{Clock, VirtualClock};
use piano_core::load_smf;
use piano_core::practica::{Cursor, Velocidad};
use piano_core::time::Micros;
use piano_core::Song;

/// Una canción de `dur_ms` milisegundos con una sola nota que la ocupa entera.
fn cancion_de(dur_ms: u64) -> Song {
    let raw = SmfBuilder::new(1000)
        .track(|t| t.tempo(0, 1_000_000).note(0, 60, 90, dur_ms))
        .build();
    load_smf(&raw).expect("valida")
}

fn cancion_vacia() -> Song {
    load_smf(&SmfBuilder::new(1000).track(|t| t).build()).expect("valida")
}

// ---------------------------------------------------------------- Velocidad

#[test]
fn la_velocidad_se_reduce_en_el_constructor() {
    // 2/4 y 1/2 son la misma velocidad. Si no se reducen, comparar régimenes campo a campo
    // trata un ajuste redundante como un cambio real, y cada cambio real rebasa el ancla.
    assert_eq!(Velocidad::nueva(2, 4), Velocidad::nueva(1, 2));
    assert_eq!(Velocidad::nueva(3, 3), Some(Velocidad::NORMAL));
    assert_eq!(Velocidad::nueva(0, 7), Some(Velocidad::PAUSA));
}

#[test]
fn denominador_cero_no_es_una_velocidad() {
    assert_eq!(Velocidad::nueva(1, 0), None);
    assert_eq!(Velocidad::nueva(0, 0), None);
}

#[test]
fn la_pausa_tiene_denominador_uno_y_no_cero() {
    // Con den == 0 la división entera revienta, y el guarda defensivo de `modelo.ts`
    // (`ancla.den === 0 ? 0`) pasaría a ser la ruta normal en vez de una red de seguridad.
    assert_eq!(Velocidad::PAUSA.den(), 1);
    assert_eq!(Velocidad::PAUSA.num(), 0);
}

// ---------------------------------------------------------------- aritmética

#[test]
fn la_posicion_no_depende_de_la_cadencia_de_fotogramas() {
    // T047. El error de una implementación que rebasa el ancla en cada avance es
    // invisible con pasos que el denominador divide; con 1/3 y pasos de 100.000 µs no.
    let song = cancion_de(600_000);
    let cadencias: [u64; 3] = [1_000_000, 1_000, 100];
    let mut posiciones = Vec::new();

    for paso in cadencias {
        let mut reloj = VirtualClock::new();
        let mut c = Cursor::nuevo(&song);
        c.cambiar_velocidad(Velocidad::nueva(1, 3).expect("válida"), reloj.now());
        c.poner_en_marcha(reloj.now());
        let mut t = 0;
        while t < 1_000_000 {
            t += paso;
            reloj.set(Micros(t));
            c.avanzar(reloj.now());
        }
        posiciones.push(c.posicion());
    }
    assert_eq!(posiciones[0], Micros(333_333), "un solo avance");
    assert_eq!(posiciones[1], posiciones[0], "mil avances dan lo mismo");
    assert_eq!(posiciones[2], posiciones[0], "diez mil avances dan lo mismo");
}

#[test]
fn diez_mil_avances_a_un_tercio_dan_la_cifra_exacta() {
    // El paso está elegido para que el denominador NO lo divida: con 1/2 y pasos de
    // 20.000 µs la implementación correcta y la que rebasa cada vez dan lo mismo, y la
    // prueba no probaría nada.
    let song = cancion_de(600_000);
    let mut reloj = VirtualClock::new();
    let mut c = Cursor::nuevo(&song);
    c.cambiar_velocidad(Velocidad::nueva(1, 3).expect("válida"), reloj.now());
    c.poner_en_marcha(reloj.now());
    for i in 1..=10_000u64 {
        reloj.set(Micros(i * 16_667));
        c.avanzar(reloj.now());
    }
    assert_eq!(c.posicion(), Micros(55_556_666));
}

#[test]
fn ir_a_la_mitad_y_volver_deja_la_posicion_donde_toca() {
    // T047/T051. Cifras absolutas, no relativas: separan la implementación correcta tanto
    // de la que no hace nada como de la que rebasa en cada fotograma.
    let song = cancion_de(600_000);
    let mut reloj = VirtualClock::new();
    let mut c = Cursor::nuevo(&song);
    c.poner_en_marcha(reloj.now());

    reloj.set(Micros(5_000_000));
    c.avanzar(reloj.now());
    assert_eq!(c.posicion(), Micros(5_000_000), "a tempo");

    c.cambiar_velocidad(Velocidad::nueva(1, 3).expect("válida"), reloj.now());
    reloj.set(Micros(8_000_000));
    c.avanzar(reloj.now());
    assert_eq!(c.posicion(), Micros(6_000_000), "tres segundos a un tercio son uno");

    c.cambiar_velocidad(Velocidad::NORMAL, reloj.now());
    reloj.set(Micros(13_000_000));
    c.avanzar(reloj.now());
    assert_eq!(c.posicion(), Micros(11_000_000), "y de vuelta a tempo");
}

#[test]
fn ningun_par_aceptado_desborda() {
    // `Velocidad::nueva` acepta hasta u32::MAX y una canción puede durar 24 h. Con `u64`
    // crudo el producto desborda: pánico en debug y valor silencioso en release, es decir
    // **dos salidas distintas según el perfil de compilación** para la misma entrada, que
    // es exactamente lo que el Principio I prohíbe.
    let song = cancion_de(600_000);
    let mut reloj = VirtualClock::new();
    let mut c = Cursor::nuevo(&song);
    c.cambiar_velocidad(Velocidad::nueva(u32::MAX, 1).expect("válida"), reloj.now());
    c.poner_en_marcha(reloj.now());
    reloj.set(Micros(86_400_000_000));
    c.avanzar(reloj.now());
    // Sin pánico, y recortado por el final de la canción.
    assert_eq!(c.posicion(), song.duration_us());
}

// ---------------------------------------------------------------- régimen y ancla

#[test]
fn el_ancla_solo_se_emite_al_cambiar_de_regimen() {
    // T056. Es lo que mantiene el puente vacío: el frontend interpola entre anclas.
    let song = cancion_de(600_000);
    let mut reloj = VirtualClock::new();
    let mut c = Cursor::nuevo(&song);

    assert!(c.poner_en_marcha(reloj.now()).is_some(), "arrancar cambia el régimen");
    let mut anclas = 0;
    for i in 1..=600u64 {
        reloj.set(Micros(i * 16_667));
        if c.avanzar(reloj.now()).ancla.is_some() {
            anclas += 1;
        }
    }
    assert_eq!(anclas, 0, "seiscientos avances sin cambio de régimen: ninguna ancla");
}

#[test]
fn repetir_un_mando_con_el_mismo_valor_no_es_un_cambio_de_regimen() {
    // Un deslizador de React controlado reemite su valor en cada fotograma. Si cada
    // reemisión rebasara, serían 36.000 falsos rebases en diez minutos, y cada uno trunca.
    let song = cancion_de(600_000);
    let mut reloj = VirtualClock::new();
    let mut c = Cursor::nuevo(&song);
    let tercio = Velocidad::nueva(1, 3).expect("válida");
    c.cambiar_velocidad(tercio, reloj.now());
    c.poner_en_marcha(reloj.now());

    let mut anclas = 0;
    for i in 1..=1_000u64 {
        reloj.set(Micros(i * 16_667));
        // El mismo racional, y también sin reducir: 2/6 es 1/3.
        if c.cambiar_velocidad(tercio, reloj.now()).is_some() {
            anclas += 1;
        }
        if c.cambiar_velocidad(Velocidad::nueva(2, 6).expect("válida"), reloj.now()).is_some() {
            anclas += 1;
        }
        c.avanzar(reloj.now());
    }
    assert_eq!(anclas, 0, "ningún cambio real de régimen");
    assert_eq!(c.posicion(), Micros(5_555_666), "y por tanto ninguna deriva");
}

#[test]
fn cambiar_de_velocidad_no_provoca_salto_de_posicion() {
    // FR-010. Se prueba en los dos sentidos: acelerar destapa el salto hacia delante que
    // ralentizar esconde.
    for (num, den) in [(1u32, 2u32), (2, 1)] {
        let song = cancion_de(600_000);
        let mut reloj = VirtualClock::new();
        let mut c = Cursor::nuevo(&song);
        c.poner_en_marcha(reloj.now());
        reloj.set(Micros(10_000_000));
        c.avanzar(reloj.now());
        assert_eq!(c.posicion(), Micros(10_000_000));

        // Sin mover el reloj: la posición no puede cambiar.
        c.cambiar_velocidad(Velocidad::nueva(num, den).expect("válida"), reloj.now());
        assert_eq!(c.posicion(), Micros(10_000_000), "velocidad {num}/{den}");
    }
}

#[test]
fn el_ancla_emitida_lleva_el_instante_del_rebase() {
    // Calcular con el régimen nuevo hace retroceder el cursor; no refrescar el instante lo
    // hace saltar hacia delante. El ancla emitida es la única forma de comprobar ambas.
    let song = cancion_de(600_000);
    let mut reloj = VirtualClock::new();
    let mut c = Cursor::nuevo(&song);
    c.poner_en_marcha(reloj.now());
    reloj.set(Micros(10_000_000));
    c.avanzar(reloj.now());

    let a = c
        .cambiar_velocidad(Velocidad::nueva(1, 2).expect("válida"), reloj.now())
        .expect("cambiar de velocidad es un cambio de régimen");
    assert_eq!(a.posicion_us, Micros(10_000_000), "posición con el régimen VIEJO");
    assert_eq!(a.instante_us, Micros(10_000_000), "instante refrescado");
    assert_eq!((a.num, a.den), (1, 2), "y el régimen nuevo");
}

// ---------------------------------------------------------------- pausa

#[test]
fn la_pausa_no_consume_cancion() {
    // T049. La prueba solo vale si el reloj avanza DURANTE la pausa: con el reloj quieto,
    // la implementación correcta y la que ignora la pausa dan lo mismo.
    let song = cancion_de(600_000);
    let mut reloj = VirtualClock::new();
    let mut c = Cursor::nuevo(&song);
    c.poner_en_marcha(reloj.now());
    reloj.set(Micros(1_000_000));
    c.avanzar(reloj.now());
    assert_eq!(c.posicion(), Micros(1_000_000));

    c.pausar(reloj.now());
    reloj.set(Micros(601_000_000)); // diez minutos de reloj real en pausa
    c.avanzar(reloj.now());
    assert_eq!(c.posicion(), Micros(1_000_000), "la pausa no consume canción");

    c.poner_en_marcha(reloj.now());
    reloj.set(Micros(601_500_000));
    c.avanzar(reloj.now());
    assert_eq!(c.posicion(), Micros(1_500_000), "y reanudar continúa sin salto");
}

#[test]
fn pausar_dos_veces_no_pierde_la_velocidad_de_practica() {
    // Guardar siempre la velocidad vigente al pausar deja `velocidad_previa = PAUSA` a la
    // segunda, y la canción no vuelve a moverse nunca.
    let song = cancion_de(600_000);
    let mut reloj = VirtualClock::new();
    let mut c = Cursor::nuevo(&song);
    c.cambiar_velocidad(Velocidad::nueva(1, 2).expect("válida"), reloj.now());
    c.poner_en_marcha(reloj.now());
    reloj.set(Micros(800_000));
    c.avanzar(reloj.now());
    assert_eq!(c.posicion(), Micros(400_000));

    c.pausar(reloj.now());
    assert!(c.pausar(reloj.now()).is_none(), "pausar en pausa no es cambio de régimen");

    c.poner_en_marcha(reloj.now());
    reloj.set(Micros(1_800_000));
    c.avanzar(reloj.now());
    assert_eq!(c.posicion(), Micros(900_000), "reanuda a la mitad, no a tempo");
}

// ---------------------------------------------------------------- saltar

#[test]
fn saltar_deja_el_cursor_donde_se_pide_y_conserva_el_modo() {
    // FR-007b.
    let song = cancion_de(600_000);
    let mut reloj = VirtualClock::new();
    let mut c = Cursor::nuevo(&song);
    c.cambiar_velocidad(Velocidad::nueva(1, 2).expect("válida"), reloj.now());
    c.poner_en_marcha(reloj.now());
    reloj.set(Micros(10_000_000));
    c.avanzar(reloj.now());

    c.saltar_a(Micros(300_000_000), reloj.now());
    assert_eq!(c.posicion(), Micros(300_000_000));
    assert_eq!(c.velocidad(), Velocidad::nueva(1, 2).expect("válida"), "el modo intacto");

    reloj.set(Micros(12_000_000));
    c.avanzar(reloj.now());
    assert_eq!(c.posicion(), Micros(301_000_000), "y sigue avanzando a su velocidad");
}

#[test]
fn saltar_a_donde_ya_se_esta_no_es_un_cambio_de_regimen() {
    let song = cancion_de(600_000);
    let mut reloj = VirtualClock::new();
    let mut c = Cursor::nuevo(&song);
    c.poner_en_marcha(reloj.now());
    reloj.set(Micros(4_000_000));
    c.avanzar(reloj.now());
    assert!(c.saltar_a(Micros(4_000_000), reloj.now()).is_none());
}

// ---------------------------------------------------------------- el final

#[test]
fn la_cancion_termina_una_sola_vez() {
    // FR-011. Un bool pegajoso da un solo aviso pero no se rearma; un nivel sin flanco
    // manda sesenta avisos por segundo por el puente.
    let song = cancion_de(1_000); // 1.000.000 µs
    let mut reloj = VirtualClock::new();
    let mut c = Cursor::nuevo(&song);
    c.poner_en_marcha(reloj.now());

    let mut avisos = 0;
    for i in 1..=120u64 {
        reloj.set(Micros(i * 16_667));
        if c.avanzar(reloj.now()).terminada {
            avisos += 1;
        }
    }
    assert_eq!(avisos, 1, "exactamente un aviso en ciento veinte avances");
    assert!(c.ha_terminado());
}

#[test]
fn saltar_hacia_atras_rearma_el_final() {
    // Si el aviso fuese un pestillo de una vía, volver al principio y llegar otra vez al
    // final no avisaría, y la aplicación se quedaría creyendo que sigue sonando.
    let song = cancion_de(1_000);
    let mut reloj = VirtualClock::new();
    let mut c = Cursor::nuevo(&song);
    c.poner_en_marcha(reloj.now());
    reloj.set(Micros(2_000_000));
    c.avanzar(reloj.now());
    assert!(c.ha_terminado());

    c.saltar_a(Micros::ZERO, reloj.now());
    assert!(!c.ha_terminado(), "vuelve a no estar terminada sin llamar a avanzar");

    let mut avisos = 0;
    for i in 1..=120u64 {
        reloj.set(Micros(2_000_000 + i * 16_667));
        if c.avanzar(reloj.now()).terminada {
            avisos += 1;
        }
    }
    assert_eq!(avisos, 1, "y vuelve a avisar exactamente una vez");
}

#[test]
fn una_cancion_vacia_termina_una_sola_vez() {
    let song = cancion_vacia();
    let mut reloj = VirtualClock::new();
    let mut c = Cursor::nuevo(&song);
    assert!(c.ha_terminado(), "sin notas, ya está terminada");
    assert_eq!(c.posicion(), Micros::ZERO);

    let mut avisos = 0;
    for i in 0..=100u64 {
        reloj.set(Micros(i * 16_667));
        if c.avanzar(reloj.now()).terminada {
            avisos += 1;
        }
    }
    assert_eq!(avisos, 1, "un solo flanco en ciento un avances");
}

#[test]
fn la_mitad_de_velocidad_duplica_exactamente_la_duracion() {
    // SC-008 / T051a. Sin tolerancia de ningún tipo, ni de un fotograma: es lo que
    // justifica que la velocidad sea un racional y no un decimal.
    let song = cancion_de(600_000); // 600.000.000 µs
    let mut reloj = VirtualClock::new();
    let mut c = Cursor::nuevo(&song);
    c.cambiar_velocidad(Velocidad::nueva(1, 2).expect("válida"), reloj.now());
    c.poner_en_marcha(reloj.now());

    reloj.set(Micros(1_199_999_999));
    c.avanzar(reloj.now());
    assert_eq!(c.posicion(), Micros(599_999_999));
    assert!(!c.ha_terminado(), "a un microsegundo del final todavía no ha terminado");

    reloj.set(Micros(1_200_000_000));
    c.avanzar(reloj.now());
    assert_eq!(c.posicion(), Micros(600_000_000));
    assert!(c.ha_terminado(), "el doble exacto de la duración");
}

#[test]
fn el_reloj_que_no_avanza_no_mueve_el_cursor() {
    let song = cancion_de(600_000);
    let mut reloj = VirtualClock::new();
    let mut c = Cursor::nuevo(&song);
    c.poner_en_marcha(reloj.now());
    reloj.set(Micros(3_000_000));
    c.avanzar(reloj.now());
    let antes = c.posicion();
    for _ in 0..100 {
        c.avanzar(reloj.now());
    }
    assert_eq!(c.posicion(), antes, "sin reloj no hay avance");
}
