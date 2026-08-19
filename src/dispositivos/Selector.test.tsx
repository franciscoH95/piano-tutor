// T085a — elegir teclado.
import { cleanup, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import { Selector, type DispositivoPlano } from "./Selector";

afterEach(cleanup);

const DOS_IGUALES: DispositivoPlano[] = [
  { nombre: "Casio CDP-S110", posicion: 0, idSistema: 11 },
  { nombre: "Casio CDP-S110", posicion: 1, idSistema: 22 },
];

describe("la lista de teclados", () => {
  it("muestra la posición además del nombre, para distinguir homónimos", () => {
    // Dos teclados del mismo modelo se llaman igual. Sin la posición, el alumno elegiría
    // a ciegas y la mitad de las veces se equivocaría.
    render(<Selector dispositivos={DOS_IGUALES} onElegir={() => {}} />);
    const opciones = screen.getAllByRole("button", { name: /Casio/ });
    expect(opciones).toHaveLength(2);
    expect(opciones[0]).toHaveTextContent("1");
    expect(opciones[1]).toHaveTextContent("2");
    expect(opciones[0].textContent).not.toBe(opciones[1].textContent);
  });

  it("elegir uno emite exactamente ese, no el homónimo", async () => {
    const onElegir = vi.fn();
    render(<Selector dispositivos={DOS_IGUALES} onElegir={onElegir} />);
    await userEvent.click(screen.getAllByRole("button", { name: /Casio/ })[1]);
    expect(onElegir).toHaveBeenCalledTimes(1);
    expect(onElegir).toHaveBeenCalledWith({
      nombre: "Casio CDP-S110",
      posicion: 1,
      idSistema: 22,
    });
  });

  it("no preselecciona ninguno", () => {
    // FR-025: cuando el recordado no está hay que ELEGIR, no aceptar lo que la aplicación
    // proponga. Un preseleccionado invita a pulsar «aceptar» sin mirar, que es justo el
    // error que la posición existe para evitar.
    render(<Selector dispositivos={DOS_IGUALES} onElegir={() => {}} />);
    for (const b of screen.getAllByRole("button", { name: /Casio/ })) {
      expect(b).toHaveAttribute("aria-pressed", "false");
    }
  });

  it("dice que hay que elegir de nuevo cuando el recordado no está", () => {
    render(<Selector dispositivos={DOS_IGUALES} onElegir={() => {}} recordadoAusente />);
    expect(screen.getByText(/no (está|esta)|vuelve a elegir|elige de nuevo/i)).toBeInTheDocument();
  });

  it("no lo dice la primera vez, cuando no había nada recordado", () => {
    render(<Selector dispositivos={DOS_IGUALES} onElegir={() => {}} />);
    expect(screen.queryByText(/vuelve a elegir|elige de nuevo/i)).not.toBeInTheDocument();
  });

  it("sin teclados lo dice y no muestra una lista vacía", () => {
    render(<Selector dispositivos={[]} onElegir={() => {}} />);
    expect(screen.getByText(/ning(ú|u)n teclado|no se detecta/i)).toBeInTheDocument();
    expect(screen.queryAllByRole("button")).toHaveLength(0);
  });

  it("un teclado sin identificador del sistema se puede elegir igual", () => {
    // La identidad de reserva es nombre + posición; el sistema no siempre da un id.
    const onElegir = vi.fn();
    render(
      <Selector
        dispositivos={[{ nombre: "Teclado raro", posicion: 0, idSistema: null }]}
        onElegir={onElegir}
      />,
    );
    return userEvent.click(screen.getByRole("button", { name: /Teclado raro/ })).then(() => {
      expect(onElegir).toHaveBeenCalledWith({
        nombre: "Teclado raro",
        posicion: 0,
        idSistema: null,
      });
    });
  });
});
