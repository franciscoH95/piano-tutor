//! T006: los newtypes de tiempo.

use piano_core::time::{Micros, Ticks};

#[test]
fn los_newtypes_no_cuestan_nada() {
    assert_eq!(std::mem::size_of::<Ticks>(), 8, "Ticks debe ser transparente sobre u64");
    assert_eq!(std::mem::size_of::<Micros>(), 8, "Micros debe ser transparente sobre u64");
}

#[test]
fn ordenan_como_su_entero() {
    assert!(Ticks(0) < Ticks(1));
    assert!(Micros(1_000) > Micros(999));
    let mut v = vec![Ticks(5), Ticks(1), Ticks(3)];
    v.sort();
    assert_eq!(v, vec![Ticks(1), Ticks(3), Ticks(5)]);
}

#[test]
fn saturating_sub_satura_en_cero_en_vez_de_desbordar() {
    assert_eq!(Ticks(10).saturating_sub(Ticks(3)), Ticks(7));
    assert_eq!(Ticks(3).saturating_sub(Ticks(10)), Ticks(0), "nunca por debajo de cero");
    assert_eq!(Micros(0).saturating_sub(Micros(u64::MAX)), Micros(0));
}

#[test]
fn checked_sub_avisa_del_desbordamiento_en_vez_de_ocultarlo() {
    assert_eq!(Ticks(10).checked_sub(Ticks(3)), Some(Ticks(7)));
    assert_eq!(Ticks(3).checked_sub(Ticks(10)), None);
}

#[test]
fn checked_add_avisa_del_desbordamiento() {
    assert_eq!(Ticks(1).checked_add(Ticks(2)), Some(Ticks(3)));
    assert_eq!(Ticks(u64::MAX).checked_add(Ticks(1)), None);
}
