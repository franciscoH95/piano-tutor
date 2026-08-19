//! Pruebas de `core/src/evaluacion/emparejar.rs`.

mod fixtures;
use piano_core::evaluacion::instante_de;
use piano_core::practica::{posicion_en, Ancla};
use piano_core::time::Micros;

fn ancla(pos: u64, inst: u64, num: u32, den: u32, tope: Option<u64>) -> Ancla {
    Ancla {
        posicion_us: Micros(pos),
        instante_us: Micros(inst),
        num,
        den,
        tope_us: tope.map(Micros),
    }
}

#[test]
fn devuelve_el_primer_instante_que_alcanza_la_posicion() {
    // T013. La identidad `posicion_en(instante_de(p)) == p` **no es cierta en general**, y
    // conviene decir por qué: con el cursor más rápido que el reloj —num > den— la posición
    // avanza a saltos y hay valores que nunca se alcanzan exactamente. Con num/den = 2/1 la
    // posición pasa de 0 a 2 sin pisar el 1.
    //
    // Lo que sí es cierto, y es la definición de la función, es que devuelve el PRIMER
    // instante que llega o pasa: en él la posición ya alcanzó el objetivo, y un microsegundo
    // antes todavía no. Asertar la identidad habría forzado una implementación equivocada.
    //
    // Doscientos pares deterministas, no aleatorios: SC-005 exige que la misma entrada dé lo
    // mismo siempre, y una prueba con semilla al azar no lo comprobaría.
    for i in 0..200u64 {
        let num = 1 + (i % 7) as u32;
        let den = 1 + ((i * 3) % 5) as u32;
        let a = ancla(i * 1_000, i * 137, num, den, None);
        let objetivo = Micros(i * 1_000 + i * 991);
        let t = instante_de(&a, objetivo).expect("alcanzable");
        assert!(
            posicion_en(&a, t).0 >= objetivo.0,
            "par {i} ({num}/{den}): en {} la posición {} no llega a {}",
            t.0,
            posicion_en(&a, t).0,
            objetivo.0
        );
        if t.0 > a.instante_us.0 {
            assert!(
                posicion_en(&a, Micros(t.0 - 1)).0 < objetivo.0,
                "par {i} ({num}/{den}): un microsegundo antes ya había llegado, no es el primero"
            );
        }
    }
}

#[test]
fn con_el_cursor_mas_lento_que_el_reloj_la_identidad_si_se_cumple() {
    // Cuando num <= den toda posición es alcanzable, y ahí la vuelta sí es exacta. Es el
    // caso normal: velocidad de práctica igual o menor que la del archivo.
    for i in 0..100u64 {
        let den = 1 + (i % 5) as u32;
        let a = ancla(i * 1_000, i * 137, 1, den, None);
        let objetivo = Micros(i * 1_000 + i * 991);
        let t = instante_de(&a, objetivo).expect("alcanzable");
        assert_eq!(posicion_en(&a, t), objetivo, "par {i} (1/{den})");
    }
}

#[test]
fn es_el_techo_y_no_el_suelo() {
    // T014. `posicion_en` aplica `floor`, así que su inversa es el TECHO: el primer instante
    // en que el cursor alcanza esa posición, no el último en que está por debajo. Con el
    // suelo, el instante esperado saldría sistemáticamente pronto y todos los alumnos
    // parecerían ir tarde.
    let a = ancla(0, 0, 1, 3, None);
    // A un tercio de velocidad, la posición 1 se alcanza en el instante 3.
    let t = instante_de(&a, Micros(1)).expect("alcanzable");
    assert_eq!(t, Micros(3));
    assert_eq!(posicion_en(&a, Micros(2)), Micros(0), "en 2 todavía no ha llegado");
    assert_eq!(posicion_en(&a, Micros(3)), Micros(1), "en 3 sí");
}

#[test]
fn en_pausa_el_cursor_no_llega_nunca() {
    // T015. Con num == 0 la posición no avanza: ningún instante la alcanza.
    let a = ancla(0, 0, 0, 1, None);
    assert_eq!(instante_de(&a, Micros(1)), None);
    assert_eq!(instante_de(&a, Micros(0)), Some(Micros(0)), "la actual sí, es donde está");
}

#[test]
fn por_encima_del_tope_no_se_alcanza() {
    // El techo del cursor —el final de la canción, o la puerta pendiente en modo espera—
    // impide llegar más allá. Devolver un instante sería prometer algo que no ocurrirá.
    let a = ancla(0, 0, 1, 1, Some(1_000));
    assert_eq!(instante_de(&a, Micros(1_000)), Some(Micros(1_000)), "el tope sí");
    assert_eq!(instante_de(&a, Micros(1_001)), None, "más allá no");
}

#[test]
fn una_posicion_anterior_al_ancla_no_se_alcanza() {
    // El cursor no retrocede: una posición ya pasada no vuelve a alcanzarse.
    let a = ancla(5_000, 1_000, 1, 1, None);
    assert_eq!(instante_de(&a, Micros(4_999)), None);
    assert_eq!(instante_de(&a, Micros(5_000)), Some(Micros(1_000)));
}

#[test]
fn no_desborda_con_los_valores_que_el_tipo_admite() {
    // T016. `Velocidad::nueva` acepta hasta u32::MAX y una canción puede durar 24 horas.
    // Con `u64` crudo el producto desborda: pánico en debug y valor silencioso en release,
    // es decir dos salidas para la misma entrada según cómo se compile.
    let lento = ancla(0, 0, 1, u32::MAX, None);
    let r = instante_de(&lento, Micros(86_400_000_000));
    assert!(r.is_some() || r.is_none(), "sin pánico, sea cual sea la respuesta");

    let rapido = ancla(0, 0, u32::MAX, 1, None);
    assert!(instante_de(&rapido, Micros(86_400_000_000)).is_some(), "sin pánico");
}

#[test]
fn el_resultado_no_depende_del_perfil_de_compilacion() {
    // T018. Los valores que pasan por `u128` y `try_from` son los que podrían diferir entre
    // debug y release si se usara `as`. Se comprueban explícitamente contra cifras escritas
    // a mano: si alguien mete un `as`, el número cambia aquí y no en producción, que es
    // donde nadie lo vería.
    let a = ancla(0, 0, 1, 3, None);
    assert_eq!(instante_de(&a, Micros(1_000_000)), Some(Micros(3_000_000)));
    let b = ancla(1_000_000, 500_000, 3, 2, None);
    // Faltan 2.000.000 de canción a 3/2: hacen falta ⌈2.000.000·2/3⌉ = 1.333.334 de reloj.
    assert_eq!(instante_de(&b, Micros(3_000_000)), Some(Micros(1_833_334)));
}
