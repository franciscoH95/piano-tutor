//! Comprueba que el arnés de pruebas de interfaz funciona antes de apoyarse en él.
import { describe, expect, it } from "vitest";

describe("arnés de pruebas", () => {
  it("ejecuta pruebas de TypeScript", () => {
    expect(1 + 1).toBe(2);
  });
  it("tiene un DOM disponible", () => {
    const div = document.createElement("div");
    div.textContent = "hola";
    expect(div.textContent).toBe("hola");
  });
});
