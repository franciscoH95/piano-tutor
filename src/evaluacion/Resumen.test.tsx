// T044, T045 — el resumen que ve el alumno.
import { cleanup, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
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
    porMano: {
      izquierda: { acertadas: 0, fueraDeTiempo: 0, omitidas: 0 },
      derecha: { acertadas: 0, fueraDeTiempo: 0, omitidas: 0 },
    },
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

describe("las dos manos", () => {
  it("se muestran por separado cuando la pieza las tiene", () => {
    // T053 (FR-018). «17 de 20» junto no dice si el problema está en una mano concreta,
    // que es justo lo que el alumno necesita saber para repasar mañana.
    render(
      <Resumen
        resultado={resultado({
          acertadas: 12,
          omitidas: 8,
          intentadas: 20,
          porMano: {
            izquierda: { acertadas: 10, fueraDeTiempo: 0, omitidas: 0 },
            derecha: { acertadas: 2, fueraDeTiempo: 0, omitidas: 8 },
          },
        })}
      />,
    );
    expect(screen.getByText(/izquierda/i)).toBeInTheDocument();
    expect(screen.getByText(/derecha/i)).toBeInTheDocument();
  });

  it("no las muestra si la pieza es de una sola mano", () => {
    // Enseñar «derecha: 0 de 0» sería ruido que hace pensar que algo falló.
    render(
      <Resumen
        resultado={resultado({
          acertadas: 10,
          intentadas: 10,
          porMano: {
            izquierda: { acertadas: 10, fueraDeTiempo: 0, omitidas: 0 },
            derecha: { acertadas: 0, fueraDeTiempo: 0, omitidas: 0 },
          },
        })}
      />,
    );
    expect(screen.queryByText(/derecha/i)).not.toBeInTheDocument();
  });
});

describe("el selector de exigencia", () => {
  it("emite el nivel elegido", async () => {
    const onNivel = vi.fn();
    render(<Resumen resultado={resultado()} nivel="intermedio" onNivel={onNivel} />);
    await userEvent.selectOptions(screen.getByRole("combobox", { name: /exigencia/i }), "exigente");
    expect(onNivel).toHaveBeenLastCalledWith("exigente");
  });

  it("refleja el nivel vigente", () => {
    render(<Resumen resultado={resultado()} nivel="permisivo" onNivel={() => {}} />);
    expect(screen.getByRole("combobox", { name: /exigencia/i })).toHaveValue("permisivo");
  });

  it("dice que el cambio afecta a la próxima interpretación, no a esta", () => {
    // Sin decirlo, el alumno cambiaría de nivel esperando que el resumen que está mirando
    // se recalcule, y no lo hace: ese resultado ya está juzgado.
    render(<Resumen resultado={resultado({ acertadas: 5, intentadas: 10 })} nivel="intermedio" onNivel={() => {}} />);
    expect(screen.getByText(/próxima|siguiente/i)).toBeInTheDocument();
  });

  it("no aparece si no se le pasa manejador", () => {
    // El resumen se usa también donde no hay nada que ajustar.
    render(<Resumen resultado={resultado()} />);
    expect(screen.queryByRole("combobox", { name: /exigencia/i })).not.toBeInTheDocument();
  });
});
