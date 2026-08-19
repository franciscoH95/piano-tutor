// T015 — la interpolación entre anclas.
//
// No forma parte de la excepción del Principio II: decide dónde está la práctica, y eso
// es una decisión. Solo `Lienzo.tsx` está exento, y solo mientras se limite a pintar.
import { describe, expect, it } from "vitest";
import { posicionEn, type Ancla, anclarEnRelojLocal } from "./modelo";

const ancla = (p: Partial<Ancla> = {}): Ancla => ({
  posicionUs: 0,
  instanteUs: 0,
  num: 1,
  den: 1,
  topeUs: null,
  ...p,
});

describe("posicionEn", () => {
  it("en el instante del ancla devuelve exactamente su posición", () => {
    expect(posicionEn(ancla({ posicionUs: 5_000, instanteUs: 1_000 }), 1_000)).toBe(5_000);
  });

  it("a velocidad normal avanza igual que el tiempo real", () => {
    expect(posicionEn(ancla({ posicionUs: 0, instanteUs: 0 }), 2_000_000)).toBe(2_000_000);
  });

  it("a media velocidad avanza la mitad", () => {
    const a = ancla({ num: 1, den: 2 });
    expect(posicionEn(a, 2_000_000)).toBe(1_000_000);
  });

  it("a doble velocidad avanza el doble", () => {
    const a = ancla({ num: 2, den: 1 });
    expect(posicionEn(a, 1_000_000)).toBe(2_000_000);
  });

  it("en pausa no avanza por mucho que pase el tiempo", () => {
    const a = ancla({ posicionUs: 7_777, num: 0, den: 1 });
    expect(posicionEn(a, 10_000_000)).toBe(7_777);
  });

  it("no retrocede si el instante consultado es anterior al ancla", () => {
    const a = ancla({ posicionUs: 5_000, instanteUs: 3_000 });
    expect(posicionEn(a, 1_000)).toBe(5_000);
  });

  it("se detiene en el tope cuando lo hay, que es el modo espera", () => {
    const a = ancla({ posicionUs: 0, instanteUs: 0, topeUs: 500_000 });
    expect(posicionEn(a, 400_000)).toBe(400_000);
    expect(posicionEn(a, 500_000)).toBe(500_000);
    expect(posicionEn(a, 900_000)).toBe(500_000);
  });

  it("es exacta con racionales que no dan un decimal redondo", () => {
    // 1/3 de velocidad durante 3 s debe dar exactamente 1 s, sin arrastrar decimales.
    const a = ancla({ num: 1, den: 3 });
    expect(posicionEn(a, 3_000_000)).toBe(1_000_000);
  });

  it("devuelve enteros siempre, nunca fracciones de microsegundo", () => {
    const a = ancla({ num: 1, den: 7 });
    for (const t of [1, 10, 999, 123_456, 9_999_999]) {
      const p = posicionEn(a, t);
      expect(Number.isInteger(p)).toBe(true);
    }
  });
});

describe("los dos relojes", () => {
  const ancla: Ancla = {
    posicionUs: 4_000_000,
    instanteUs: 900_000_000, // reloj de sesión de Rust
    num: 1,
    den: 2,
    topeUs: 600_000_000,
  };

  it("el ancla se reancla en el reloj local al llegar", () => {
    // El ancla trae `instanteUs` del reloj de sesión de Rust; la pantalla lee el suyo, con
    // otro cero. Mezclarlos daría una posición absurda: aquí serían 900 segundos de
    // diferencia. Al llegar, se sustituye el instante por la lectura local.
    const local = anclarEnRelojLocal(ancla, 12_345);
    expect(local.instanteUs).toBe(12_345);
    expect(local.posicionUs).toBe(ancla.posicionUs);
    expect(local.num).toBe(ancla.num);
    expect(local.den).toBe(ancla.den);
    expect(local.topeUs).toBe(ancla.topeUs);
  });

  it("en el instante de llegada la posición es exactamente la del ancla", () => {
    const local = anclarEnRelojLocal(ancla, 12_345);
    expect(posicionEn(local, 12_345)).toBe(4_000_000);
  });

  it("después avanza al ritmo del ancla, medido en el reloj local", () => {
    const local = anclarEnRelojLocal(ancla, 12_345);
    // Dos segundos de reloj local a media velocidad son uno de canción.
    expect(posicionEn(local, 12_345 + 2_000_000)).toBe(5_000_000);
  });

  it("usar el instante de Rust con el reloj local da una posición disparatada", () => {
    // Es el fallo que esto evita, y conviene tenerlo escrito: sin reanclar, el cursor se
    // va cientos de segundos y el desfase no rompe ninguna otra prueba.
    // Con el instante de Rust, un instante local pequeño satura la resta a cero: el
    // cursor se queda clavado en la posición del ancla y parece que no avanza.
    expect(posicionEn(ancla, 12_345)).toBe(4_000_000);
    // Y en el reloj de Rust, dos segundos después de su instante son uno de canción.
    expect(posicionEn(ancla, 902_000_000)).toBe(5_000_000);
    // Reanclado, ese mismo número leído como reloj local da algo completamente distinto:
    // 902 segundos locales desde la llegada, no dos.
    const local = anclarEnRelojLocal(ancla, 12_345);
    expect(posicionEn(local, 902_000_000)).toBe(454_993_827);
  });

  it("el tope se sigue respetando tras reanclar", () => {
    const local = anclarEnRelojLocal(ancla, 0);
    expect(posicionEn(local, 10_000_000_000)).toBe(600_000_000);
  });
});
