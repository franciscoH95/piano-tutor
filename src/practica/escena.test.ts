// T042 — la escena que se pinta.
//
// Vive aquí y no en `Lienzo.tsx` a propósito. Colocar dónde va cada tecla y cada nota es
// una decisión, y el lienzo es el único archivo acogido a la excepción del Principio II:
// meter esto allí ampliaría la excepción, y la constitución dice que se estrecha.

import { describe, expect, it } from "vitest";
import {
  ALTO_TECLADO,
  construirEscena,
  esNegra,
  nombreDeNota,
  TECLA_MAS_AGUDA,
  TECLA_MAS_GRAVE,
  VENTANA_US,
} from "./escena";
import type { NotaVisiblePlana } from "./puente";

function nota(p: Partial<NotaVisiblePlana> = {}): NotaVisiblePlana {
  return {
    indice: 0,
    key: 60,
    onsetUs: 0,
    endUs: 500_000,
    derecha: true,
    dedo: 1,
    base: 0,
    alteracion: 0,
    estado: "pendiente",
    ...p,
  };
}

describe("el teclado", () => {
  it("tiene ochenta y ocho teclas", () => {
    const e = construirEscena([]);
    expect(e.teclas).toHaveLength(88);
  });

  it("va del la 0 al do 8, que es el piano de verdad", () => {
    expect(TECLA_MAS_GRAVE).toBe(21);
    expect(TECLA_MAS_AGUDA).toBe(108);
  });

  it("distingue las negras de las blancas", () => {
    // Las cinco de la octava, y las siete que no lo son.
    for (const k of [61, 63, 66, 68, 70]) expect(esNegra(k)).toBe(true);
    for (const k of [60, 62, 64, 65, 67, 69, 71]) expect(esNegra(k)).toBe(false);
  });

  it("pinta las negras encima de las blancas", () => {
    // Si se pintaran en el orden natural, una blanca taparía la negra anterior.
    const e = construirEscena([]);
    const primeraNegra = e.teclas.findIndex((t) => t.color === "#222");
    const ultimaBlanca = e.teclas.map((t) => t.color).lastIndexOf("#fff");
    expect(primeraNegra).toBeGreaterThan(ultimaBlanca);
  });

  it("las teclas caben en el ancho de la escena", () => {
    const e = construirEscena([]);
    for (const t of e.teclas) {
      expect(t.x).toBeGreaterThanOrEqual(0);
      expect(t.x + t.ancho).toBeLessThanOrEqual(e.ancho + 0.001);
    }
  });
});

describe("las notas", () => {
  it("una nota por cada nota visible", () => {
    const e = construirEscena([nota({ key: 60 }), nota({ indice: 1, key: 64 })]);
    expect(e.notas).toHaveLength(2);
  });

  it("la nota más aguda se pinta más a la derecha", () => {
    const e = construirEscena([nota({ key: 24 }), nota({ indice: 1, key: 100 })]);
    expect(e.notas[1].x).toBeGreaterThan(e.notas[0].x);
  });

  it("una nota más larga se pinta más alta", () => {
    const corta = construirEscena([nota({ onsetUs: 0, endUs: 100_000 })]);
    const larga = construirEscena([nota({ onsetUs: 0, endUs: 1_000_000 })]);
    expect(larga.notas[0].alto).toBeGreaterThan(corta.notas[0].alto);
  });

  it("las dos manos se distinguen por color", () => {
    const e = construirEscena([
      nota({ derecha: true }),
      nota({ indice: 1, derecha: false }),
    ]);
    expect(e.notas[0].color).not.toBe(e.notas[1].color);
  });

  it("lo que ya está sonando se distingue de lo pendiente", () => {
    const e = construirEscena([
      nota({ estado: "pendiente" }),
      nota({ indice: 1, estado: "sonando" }),
    ]);
    expect(e.notas[0].color).not.toBe(e.notas[1].color);
  });

  it("una nota fuera del piano no se pinta en vez de salirse", () => {
    const e = construirEscena([nota({ key: 5 }), nota({ indice: 1, key: 127 })]);
    expect(e.notas).toHaveLength(0);
  });
});

describe("las etiquetas", () => {
  it("cada nota lleva su nombre y su dedo", () => {
    const e = construirEscena([nota({ base: 0, alteracion: 0, dedo: 3 })]);
    const textos = e.etiquetas.map((l) => l.texto);
    expect(textos.some((t) => t.includes("Do"))).toBe(true);
    expect(textos.some((t) => t.includes("3"))).toBe(true);
  });

  it("formatea el nombre desde el símbolo, no desde una cadena del núcleo", () => {
    // El núcleo manda base y alteración por separado; el texto se compone aquí, que es
    // donde se sabe en qué idioma se escribe.
    expect(nombreDeNota(0, 0)).toBe("Do");
    expect(nombreDeNota(0, 1)).toBe("Do♯");
    expect(nombreDeNota(1, -1)).toBe("Re♭");
    expect(nombreDeNota(6, 0)).toBe("Si");
  });

  it("la ventana visible es un tramo acotado y no la canción entera", () => {
    // Pedir la canción entera haría crecer el coste del fotograma con su longitud.
    expect(VENTANA_US).toBeGreaterThan(0);
    expect(VENTANA_US).toBeLessThanOrEqual(10_000_000);
  });
});

describe("la escena se desplaza con la posición", () => {
  it("una nota se acerca al teclado a medida que avanza la canción", () => {
    // Es lo que hace que la canción "caiga" hacia las teclas. Sin esto la vista sería fija
    // y el alumno no vería llegar nada.
    const n = nota({ onsetUs: 2_000_000, endUs: 2_500_000 });
    const lejos = construirEscena([n], 0);
    const cerca = construirEscena([n], 1_000_000);
    expect(cerca.notas[0].y).toBeGreaterThan(lejos.notas[0].y);
  });

  it("una nota que suena justo ahora toca el teclado", () => {
    const e = construirEscena([nota({ onsetUs: 1_000_000, endUs: 1_500_000 })], 1_000_000);
    // Su borde inferior queda al ras de la línea del teclado.
    const borde = e.notas[0].y + e.notas[0].alto;
    expect(borde).toBeCloseTo(e.alto - ALTO_TECLADO, 5);
  });

  it("la posición no altera el tamaño de la nota, solo su sitio", () => {
    // Si el alto cambiase con la posición, la nota se encogería al acercarse y el alumno
    // leería mal su duración.
    const n = nota({ onsetUs: 2_000_000, endUs: 3_000_000 });
    const a = construirEscena([n], 0).notas[0];
    const b = construirEscena([n], 1_500_000).notas[0];
    expect(b.alto).toBeCloseTo(a.alto, 5);
    expect(b.x).toBeCloseTo(a.x, 5);
    expect(b.ancho).toBeCloseTo(a.ancho, 5);
  });

  it("el teclado no se mueve con la posición", () => {
    const quieto = construirEscena([], 0);
    const avanzado = construirEscena([], 5_000_000);
    expect(avanzado.teclas).toEqual(quieto.teclas);
  });

  it("la etiqueta viaja con su nota", () => {
    const n = nota({ onsetUs: 2_000_000, endUs: 2_500_000 });
    const a = construirEscena([n], 0);
    const b = construirEscena([n], 1_000_000);
    expect(b.etiquetas[0].y - a.etiquetas[0].y).toBeCloseTo(
      b.notas[0].y - a.notas[0].y,
      5,
    );
  });

  it("sin posición se comporta como al principio de la canción", () => {
    // La llamada de una sola argumento se sigue usando mientras no hay reproducción.
    const n = nota({ onsetUs: 1_000_000, endUs: 1_500_000 });
    expect(construirEscena([n])).toEqual(construirEscena([n], 0));
  });
});
