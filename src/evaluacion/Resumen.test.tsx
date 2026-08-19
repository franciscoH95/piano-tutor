// T044, T045 — el resumen que ve el alumno.
import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import { Resumen, type ResultadoPlano } from "./Resumen";

afterEach(cleanup);

function resultado(p: Partial<ResultadoPlano> = {}): ResultadoPlano {
  return {
    acertadas: 0,
    fueraDeTiempo: 0,
    omitidas: 0,
    deMas: 0,
    dedosEscapados: 0,
    fueraDeAlcance: 0,
    noIntentadas: 0,
    intentadas: 0,
    desfaseMedianaUs: null,
    desfaseDispersionUs: null,
    sinTocar: false,
    parcial: false,
    ...p,
  };
}

describe("los recuentos", () => {
  it("muestra acertadas, omitidas y de más", () => {
    render(<Resumen resultado={resultado({ acertadas: 17, omitidas: 3, deMas: 2, intentadas: 20 })} />);
    expect(screen.getByText(/17/)).toBeInTheDocument();
    expect(screen.getByText(/\b3\b/)).toBeInTheDocument();
    expect(screen.getByText(/\b2\b/)).toBeInTheDocument();
  });

  it("«no se tocó nada» no se muestra como 0 %", () => {
    // SC-002. Son cosas distintas y el alumno las lee distinto: un 0 % dice «lo hiciste
    // fatal», y no tocar nada no dice eso.
    render(<Resumen resultado={resultado({ sinTocar: true })} />);
    expect(screen.getByText(/no (se )?toc/i)).toBeInTheDocument();
    expect(screen.queryByText(/0\s*%/)).not.toBeInTheDocument();
  });

  it("tocar mal sí muestra el recuento", () => {
    render(<Resumen resultado={resultado({ acertadas: 0, omitidas: 5, intentadas: 5 })} />);
    expect(screen.queryByText(/no (se )?toc/i)).not.toBeInTheDocument();
  });
});

describe("el resultado parcial", () => {
  it("cuando los tiempos no se evaluaron, LO DICE", () => {
    // T045 (FR-015a, SC-011). Un resumen que calla que no se midieron los tiempos se lee
    // como completo, y el alumno creería que su ritmo está bien cuando nadie lo ha mirado.
    render(<Resumen resultado={resultado({ acertadas: 10, intentadas: 10, parcial: true })} />);
    const aviso = screen.getByRole("note");
    expect(aviso).toHaveTextContent(/no se han evaluado los tiempos/i);
    expect(aviso).toHaveTextContent(/modo espera/i);
  });

  it("cuando sí se evaluaron, no lo dice", () => {
    render(<Resumen resultado={resultado({ acertadas: 10, intentadas: 10 })} />);
    expect(screen.queryByRole("note")).not.toBeInTheDocument();
  });
});

describe("el desfase sistemático", () => {
  it("dice si va adelantado o atrasado, no solo el número", () => {
    // El signo es la información. «40 ms» no le dice nada al alumno; «vas 40 ms tarde» sí.
    render(<Resumen resultado={resultado({ acertadas: 20, intentadas: 20, desfaseMedianaUs: 40_000, desfaseDispersionUs: 5_000 })} />);
    expect(screen.getByText(/tarde|atras/i)).toBeInTheDocument();
  });

  it("y distingue adelantarse de atrasarse", () => {
    render(<Resumen resultado={resultado({ acertadas: 20, intentadas: 20, desfaseMedianaUs: -40_000, desfaseDispersionUs: 5_000 })} />);
    expect(screen.getByText(/pronto|adelant/i)).toBeInTheDocument();
  });

  it("sin desfase no inventa un mensaje", () => {
    render(<Resumen resultado={resultado({ acertadas: 20, intentadas: 20 })} />);
    expect(screen.queryByText(/tarde|pronto|adelant|atras/i)).not.toBeInTheDocument();
  });
});

describe("lo que no es culpa del alumno", () => {
  it("las notas fuera de su alcance se dicen aparte y no cuentan como fallo", () => {
    render(<Resumen resultado={resultado({ acertadas: 10, intentadas: 10, fueraDeAlcance: 3 })} />);
    expect(screen.getByText(/fuera de.*alcance|tu teclado/i)).toBeInTheDocument();
  });

  it("lo saltado se dice aparte", () => {
    render(<Resumen resultado={resultado({ acertadas: 5, intentadas: 5, noIntentadas: 4 })} />);
    expect(screen.getByText(/salt/i)).toBeInTheDocument();
  });
});
