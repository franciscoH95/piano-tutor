// T042a, T042b y T057a — los controles que viven fuera del lienzo.
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import { Controles, type ControlesProps } from "./controles";

afterEach(cleanup);

/** Props por omisión: cada prueba sobrescribe solo lo que le importa. */
function props(p: Partial<ControlesProps> = {}): ControlesProps {
  return {
    corte: 60,
    vocesDelArchivo: false,
    onCorte: () => {},
    enMarcha: false,
    velocidad: { num: 1, den: 1 },
    onMarcha: () => {},
    onPausa: () => {},
    onVolverAlPrincipio: () => {},
    onVelocidad: () => {},
    modo: "porReloj",
    mano: "ambas",
    onModo: () => {},
    onMano: () => {},
    onSaltarPuerta: () => {},
    ...p,
  };
}

describe("el control del punto de corte", () => {
  it("está visible aunque el archivo traiga las voces separadas", () => {
    // Siempre visible: el usuario debe poder ver qué criterio se está usando, y decidir
    // otro. Ocultarlo cuando el archivo trae voces esconde precisamente esa información.
    render(<Controles {...props({ vocesDelArchivo: true })} />);
    expect(screen.getByRole("slider", { name: /corte/i })).toBeInTheDocument();
  });

  it("está visible cuando el reparto es por altura", () => {
    render(<Controles {...props()} />);
    expect(screen.getByRole("slider", { name: /corte/i })).toBeInTheDocument();
  });

  it("indica que se están usando las voces del archivo cuando se detectan", () => {
    render(<Controles {...props({ vocesDelArchivo: true })} />);
    expect(screen.getByText(/usar las voces del archivo/i)).toBeInTheDocument();
  });

  it("no lo indica cuando el reparto es por altura", () => {
    render(<Controles {...props()} />);
    expect(screen.queryByText(/usar las voces del archivo/i)).not.toBeInTheDocument();
  });

  it("emite el ajuste con el valor movido", async () => {
    const onCorte = vi.fn();
    render(<Controles {...props({ onCorte })} />);
    const control = screen.getByRole("slider", { name: /corte/i });

    // Un deslizador no se escribe, se mueve: `type` no produce ningún `change` sobre un
    // `input[type=range]`, así que la prueba anterior no comprobaba nada.
    fireEvent.change(control, { target: { value: "72" } });

    expect(onCorte).toHaveBeenCalledTimes(1);
    expect(onCorte).toHaveBeenLastCalledWith(72);
    // Y llega como número, no como la cadena que trae el DOM.
    expect(typeof onCorte.mock.calls[0][0]).toBe("number");
  });
});

describe("la digitación es una sugerencia", () => {
  it("lo dice de forma visible", () => {
    // FR-006c. Vive FUERA del lienzo a propósito: dentro sería parte de la capa acogida a
    // la excepción del Principio II, que no tiene pruebas. Aquí sí se puede afirmar que
    // está, y por eso se coloca aquí.
    render(<Controles {...props()} />);
    expect(screen.getByText(/sugerencia/i)).toBeInTheDocument();
  });

  it("sigue diciéndolo con las voces del archivo", () => {
    render(<Controles {...props({ vocesDelArchivo: true })} />);
    expect(screen.getByText(/sugerencia/i)).toBeInTheDocument();
  });
});

describe("el transporte", () => {
  it("pone en marcha cuando está parada", async () => {
    const onMarcha = vi.fn();
    render(<Controles {...props({ enMarcha: false, onMarcha })} />);
    await userEvent.click(screen.getByRole("button", { name: /reproducir/i }));
    expect(onMarcha).toHaveBeenCalledTimes(1);
  });

  it("pausa cuando está en marcha", async () => {
    const onPausa = vi.fn();
    render(<Controles {...props({ enMarcha: true, onPausa })} />);
    await userEvent.click(screen.getByRole("button", { name: /pausar/i }));
    expect(onPausa).toHaveBeenCalledTimes(1);
  });

  it("el mismo botón cambia de papel según el estado, y solo hay uno", () => {
    // Dos botones a la vez darían dos formas de llegar al mismo sitio y una de ellas
    // estaría siempre desactivada. Uno solo, que dice lo que va a hacer.
    const { rerender } = render(<Controles {...props({ enMarcha: false })} />);
    expect(screen.getByRole("button", { name: /reproducir/i })).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: /pausar/i })).not.toBeInTheDocument();

    rerender(<Controles {...props({ enMarcha: true })} />);
    expect(screen.getByRole("button", { name: /pausar/i })).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: /reproducir/i })).not.toBeInTheDocument();
  });

  it("vuelve al principio", async () => {
    const onVolverAlPrincipio = vi.fn();
    render(<Controles {...props({ onVolverAlPrincipio })} />);
    await userEvent.click(screen.getByRole("button", { name: /al principio/i }));
    expect(onVolverAlPrincipio).toHaveBeenCalledTimes(1);
  });

  it("volver al principio funciona también en marcha", async () => {
    const onVolverAlPrincipio = vi.fn();
    render(<Controles {...props({ enMarcha: true, onVolverAlPrincipio })} />);
    await userEvent.click(screen.getByRole("button", { name: /al principio/i }));
    expect(onVolverAlPrincipio).toHaveBeenCalledTimes(1);
  });
});

describe("el control de velocidad", () => {
  it("emite un racional, nunca un decimal", async () => {
    // Es lo que justifica todo el diseño: reducir a la mitad y volver a normal debe dejar
    // la posición EXACTAMENTE donde estaba. Un 0,75 por el puente rompe esa garantía en el
    // primer redondeo, y el error se acumula sin que nada avise.
    const onVelocidad = vi.fn();
    render(<Controles {...props({ onVelocidad })} />);

    await userEvent.click(screen.getByRole("button", { name: /tres cuartos|75/i }));

    expect(onVelocidad).toHaveBeenCalledTimes(1);
    const emitido = onVelocidad.mock.calls[0][0];
    expect(emitido).toEqual({ num: 3, den: 4 });
    expect(Number.isInteger(emitido.num)).toBe(true);
    expect(Number.isInteger(emitido.den)).toBe(true);
  });

  it("ofrece la mitad y la velocidad normal, también como racionales", async () => {
    const onVelocidad = vi.fn();
    render(<Controles {...props({ onVelocidad })} />);

    await userEvent.click(screen.getByRole("button", { name: /mitad|50/i }));
    expect(onVelocidad).toHaveBeenLastCalledWith({ num: 1, den: 2 });

    await userEvent.click(screen.getByRole("button", { name: /normal|100/i }));
    expect(onVelocidad).toHaveBeenLastCalledWith({ num: 1, den: 1 });
  });

  it("ninguna velocidad ofrecida tiene denominador cero", () => {
    // `Velocidad::nueva` devuelve None con den == 0; si la interfaz pudiera emitirlo, el
    // núcleo rechazaría el ajuste en silencio y el control mentiría.
    render(<Controles {...props()} />);
    for (const b of screen.getAllByRole("button")) {
      const den = b.getAttribute("data-den");
      if (den !== null) expect(Number(den)).toBeGreaterThan(0);
    }
  });

  it("señala cuál es la velocidad vigente", () => {
    render(<Controles {...props({ velocidad: { num: 1, den: 2 } })} />);
    expect(screen.getByRole("button", { name: /mitad|50/i })).toHaveAttribute(
      "aria-pressed",
      "true",
    );
    expect(screen.getByRole("button", { name: /normal|100/i })).toHaveAttribute(
      "aria-pressed",
      "false",
    );
  });

  it("reconoce la velocidad vigente aunque llegue sin reducir", () => {
    // 2/4 y 1/2 son la misma velocidad. Comparar los campos por separado marcaría ninguna
    // como vigente y el control se vería apagado sin motivo.
    render(<Controles {...props({ velocidad: { num: 2, den: 4 } })} />);
    expect(screen.getByRole("button", { name: /mitad|50/i })).toHaveAttribute(
      "aria-pressed",
      "true",
    );
  });
});

describe("el modo de práctica", () => {
  it("cambia entre reproducir y esperar", async () => {
    const onModo = vi.fn();
    render(<Controles {...props({ modo: "porReloj", onModo })} />);
    await userEvent.click(screen.getByRole("checkbox", { name: /espera/i }));
    expect(onModo).toHaveBeenLastCalledWith("porAcierto");
  });

  it("y vuelve a desactivarse", async () => {
    const onModo = vi.fn();
    render(<Controles {...props({ modo: "porAcierto", onModo })} />);
    await userEvent.click(screen.getByRole("checkbox", { name: /espera/i }));
    expect(onModo).toHaveBeenLastCalledWith("porReloj");
  });

  it("refleja el modo vigente", () => {
    render(<Controles {...props({ modo: "porAcierto" })} />);
    expect(screen.getByRole("checkbox", { name: /espera/i })).toBeChecked();
  });
});

describe("la mano que se practica", () => {
  it("emite la mano elegida, y las dos como ausencia de elección", async () => {
    const onMano = vi.fn();
    render(<Controles {...props({ onMano })} />);
    const selector = screen.getByRole("combobox", { name: /mano/i });

    await userEvent.selectOptions(selector, "izquierda");
    expect(onMano).toHaveBeenLastCalledWith("izquierda");
    await userEvent.selectOptions(selector, "derecha");
    expect(onMano).toHaveBeenLastCalledWith("derecha");
    await userEvent.selectOptions(selector, "ambas");
    expect(onMano).toHaveBeenLastCalledWith(null);
  });

  it("refleja la mano vigente", () => {
    render(<Controles {...props({ mano: "izquierda" })} />);
    expect(screen.getByRole("combobox", { name: /mano/i })).toHaveValue("izquierda");
  });
});

describe("la salida del atasco", () => {
  it("está disponible solo en modo espera", () => {
    // T082a. En modo reloj no hay nada que saltar: ofrecerla sería un botón que no hace
    // nada, y el alumno no sabría por qué.
    const { rerender } = render(<Controles {...props({ modo: "porReloj" })} />);
    expect(screen.queryByRole("button", { name: /saltar/i })).not.toBeInTheDocument();

    rerender(<Controles {...props({ modo: "porAcierto" })} />);
    expect(screen.getByRole("button", { name: /saltar/i })).toBeInTheDocument();
  });

  it("emite la orden de saltar la nota pendiente", async () => {
    const onSaltarPuerta = vi.fn();
    render(<Controles {...props({ modo: "porAcierto", onSaltarPuerta })} />);
    await userEvent.click(screen.getByRole("button", { name: /saltar/i }));
    expect(onSaltarPuerta).toHaveBeenCalledTimes(1);
  });

  it("explica para qué sirve, porque no es evidente", () => {
    // FR-020: existe para cuando la canción pide una nota que el teclado no tiene. Sin
    // explicación, parece un botón de hacer trampa.
    render(<Controles {...props({ modo: "porAcierto" })} />);
    expect(screen.getByText(/no tienes|no puedes tocar|teclado/i)).toBeInTheDocument();
  });
});
