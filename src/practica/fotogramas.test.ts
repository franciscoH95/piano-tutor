// T089 y T090 — las cinco cifras de SC-003, y las suspensiones que las falsean.

import { describe, expect, it } from "vitest";
import { analizar, SUSPENSION_MS } from "./fotogramas";

/** Una traza perfecta a 60 Hz. */
function perfecta(n: number, desde = 0): number[] {
  return Array.from({ length: n }, (_, i) => desde + (i * 1000) / 60);
}

describe("una reproducción perfecta", () => {
  it("no pierde ningún fotograma", () => {
    const r = analizar(perfecta(600));
    expect(r.mostrados).toBeCloseTo(100, 1);
    expect(r.intervalosLargos).toBe(0);
    expect(r.peorIntervaloMs).toBeCloseTo(1000 / 60, 5);
    expect(r.suspensiones).toHaveLength(0);
  });
});

describe("las suspensiones del sistema", () => {
  it("se detectan por encima del umbral", () => {
    // Un hueco de dos segundos: la ventana quedó tapada por otra.
    const t = [...perfecta(60), 60_000, ...perfecta(60, 62_000)];
    const r = analizar(t);
    expect(r.suspensiones).toHaveLength(2);
    expect(r.suspensiones[0]).toBeGreaterThan(SUSPENSION_MS);
  });

  it("se excluyen del cálculo en vez de hundirlo", () => {
    // **La razón de ser de SC-003c.** Sin excluirlas, un hueco de dos segundos cuenta como
    // 120 fotogramas perdidos y el informe publica un número que no describe nada.
    const conHueco = [...perfecta(60), 62_000, ...perfecta(60, 62_000 + 1000 / 60)];
    const r = analizar(conHueco);
    // Las suspensiones no cuentan como pérdida: si contaran, un hueco de dos segundos
    // serían 120 fotogramas perdidos y el porcentaje se hundiría sin motivo.
    expect(r.mostrados).toBeGreaterThan(99);
    expect(r.peorIntervaloMs).toBeLessThan(SUSPENSION_MS);
  });

  it("el tiempo suspendido se declara, no se esconde", () => {
    // Un informe que no las declare no es válido: en la primera medición se perdieron 430
    // de 600 segundos por esta causa.
    const t = [...perfecta(60), 61_000, ...perfecta(60, 61_000)];
    const r = analizar(t);
    expect(r.msSuspendidos).toBeGreaterThan(59_000);
    expect(r.msMedidos).toBeLessThan(r.msTotales);
    expect(r.msMedidos + r.msSuspendidos).toBeCloseTo(r.msTotales, 3);
  });

  it("una traza que es casi toda suspensión se declara inválida", () => {
    // Si se midió un minuto de diez, el informe no describe la reproducción: lo dice.
    const t = [0, 100_000, 200_000, 300_000, 300_016, 300_033];
    const r = analizar(t);
    expect(r.valido).toBe(false);
  });

  it("una traza mayoritariamente medida sí es válida", () => {
    // Veinte segundos medidos y uno suspendido: el informe describe la reproducción.
    const r = analizar([...perfecta(600), 11_000, ...perfecta(600, 11_000)]);
    expect(r.valido).toBe(true);
    expect(r.suspensiones).toHaveLength(1);
  });
});

describe("las cifras de SC-003", () => {
  it("cuenta los fotogramas que faltan, no el percentil del intervalo", () => {
    // Un tirón de 100 ms es un fotograma tardío o seis perdidos según cómo se cuente. La
    // primera lectura no distingue una reproducción fluida de una a tirones; la segunda sí.
    const t = [...perfecta(60), 1_100, ...perfecta(60, 1_100 + 1000 / 60)];
    const r = analizar(t);
    expect(r.mostrados).toBeLessThan(100);
    expect(r.mostrados).toBeGreaterThan(90);
  });

  it("cuenta los intervalos por encima de 25 ms (SC-003a)", () => {
    const t = [0, 16.7, 33.4, 63.4, 80.1]; // uno de 30 ms
    expect(analizar(t).intervalosLargos).toBe(1);
  });

  it("registra el peor intervalo (SC-003b)", () => {
    const t = [0, 16.7, 55.0, 71.7];
    expect(analizar(t).peorIntervaloMs).toBeCloseTo(38.3, 1);
  });

  it("el coste de dibujar se mide aparte del ritmo (SC-003d)", () => {
    // Son dos cosas distintas: la pantalla puede ir a tirones con un dibujo baratísimo.
    const r = analizar(perfecta(60), [1.2, 0.9, 15.0, 1.1]);
    expect(r.pintadoP95Ms).toBeCloseTo(15.0, 1);
    expect(r.pintadoP95Ms).not.toBe(r.peorIntervaloMs);
  });

  it("sin medidas de pintado lo dice en vez de inventar un cero", () => {
    expect(analizar(perfecta(60)).pintadoP95Ms).toBeNull();
  });
});

describe("trazas degeneradas", () => {
  it("una traza vacía no es válida y no revienta", () => {
    const r = analizar([]);
    expect(r.valido).toBe(false);
    expect(r.mostrados).toBe(0);
  });

  it("un solo fotograma no permite hablar de intervalos", () => {
    const r = analizar([123]);
    expect(r.valido).toBe(false);
    expect(r.peorIntervaloMs).toBe(0);
  });
});
