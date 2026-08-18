// T042a y T042b — los controles que viven fuera del lienzo.
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { Controles } from "./controles";

afterEach(cleanup);

describe("el control del punto de corte", () => {
  it("está visible aunque el archivo traiga las voces separadas", () => {
    // Siempre visible: el usuario debe poder ver qué criterio se está usando, y decidir
    // otro. Ocultarlo cuando el archivo trae voces esconde precisamente esa información.
    render(<Controles corte={60} vocesDelArchivo={true} onCorte={() => {}} />);
    expect(screen.getByRole("slider", { name: /corte/i })).toBeInTheDocument();
  });

  it("está visible cuando el reparto es por altura", () => {
    render(<Controles corte={60} vocesDelArchivo={false} onCorte={() => {}} />);
    expect(screen.getByRole("slider", { name: /corte/i })).toBeInTheDocument();
  });

  it("indica que se están usando las voces del archivo cuando se detectan", () => {
    render(<Controles corte={60} vocesDelArchivo={true} onCorte={() => {}} />);
    expect(screen.getByText(/usar las voces del archivo/i)).toBeInTheDocument();
  });

  it("no lo indica cuando el reparto es por altura", () => {
    render(<Controles corte={60} vocesDelArchivo={false} onCorte={() => {}} />);
    expect(screen.queryByText(/usar las voces del archivo/i)).not.toBeInTheDocument();
  });

  it("emite el ajuste con el valor movido", async () => {
    const onCorte = vi.fn();
    render(<Controles corte={60} vocesDelArchivo={false} onCorte={onCorte} />);
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
    render(<Controles corte={60} vocesDelArchivo={false} onCorte={() => {}} />);
    expect(screen.getByText(/sugerencia/i)).toBeInTheDocument();
  });

  it("sigue diciéndolo con las voces del archivo", () => {
    render(<Controles corte={60} vocesDelArchivo={true} onCorte={() => {}} />);
    expect(screen.getByText(/sugerencia/i)).toBeInTheDocument();
  });
});
