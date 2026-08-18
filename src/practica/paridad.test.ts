// La posición de la pantalla y la del núcleo son **la misma función**.
//
// Esta prueba y `core/tests/paridad_test.rs` leen el mismo fichero de vectores. Si una
// implementación cambia y la otra no, falla una de las dos. Sin esto, cada lado podría
// estar internamente consistente y aun así pintar el cursor donde el núcleo no cree que
// está: un desfase silencioso, que no rompe ninguna prueba de un solo lado.

import { describe, expect, it } from "vitest";
// Importación cruda de Vite: resuelve la ruta al compilar, así que no depende del
// directorio de trabajo ni del entorno de la prueba.
import CSV from "../../fixtures/paridad-cursor.csv?raw";
import { posicionEn, type Ancla } from "./modelo";

type Vector = { ancla: Ancla; ahoraUs: number; esperado: number; linea: string };

function vectores(): Vector[] {
  const salida: Vector[] = [];
  for (const cruda of CSV.split("\n")) {
    const linea = cruda.trim();
    if (linea === "" || linea.startsWith("#")) continue;
    const campos = linea.split(",");
    expect(campos, `línea mal formada: ${linea}`).toHaveLength(7);
    salida.push({
      ancla: {
        posicionUs: Number(campos[0]),
        instanteUs: Number(campos[1]),
        num: Number(campos[2]),
        den: Number(campos[3]),
        topeUs: campos[4] === "-" ? null : Number(campos[4]),
      },
      ahoraUs: Number(campos[5]),
      esperado: Number(campos[6]),
      linea,
    });
  }
  return salida;
}

describe("paridad con el núcleo", () => {
  const casos = vectores();

  it("el fichero de vectores se ha leído de verdad", () => {
    // Sin esto, una ruta mal puesta dejaría cero casos y la prueba pasaría sola.
    expect(casos.length).toBeGreaterThanOrEqual(15);
  });

  it.each(casos)("$linea", ({ ancla, ahoraUs, esperado }) => {
    expect(posicionEn(ancla, ahoraUs)).toBe(esperado);
  });
});
