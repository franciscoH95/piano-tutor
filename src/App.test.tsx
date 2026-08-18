// T040a y T045a — abrir una canción desde la interfaz.
import { cleanup, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import App from "./App";
import * as puente from "./practica/puente";

vi.mock("./practica/puente");

const RESUMEN = {
  notas: 3,
  duracionUs: 2_000_000,
  vocesDelArchivo: false,
  corte: 60,
};

beforeEach(() => {
  vi.mocked(puente.vistaActual).mockResolvedValue([]);
  vi.mocked(puente.ajustarCorte).mockResolvedValue();
  vi.mocked(puente.marcha).mockResolvedValue();
  vi.mocked(puente.pausa).mockResolvedValue();
  vi.mocked(puente.saltarA).mockResolvedValue();
  vi.mocked(puente.cambiarVelocidad).mockResolvedValue();
});

afterEach(() => {
  cleanup();
  vi.resetAllMocks();
});

describe("abrir una canción", () => {
  it("invoca el comando de abrir con la ruta elegida", async () => {
    vi.mocked(puente.elegirArchivo).mockResolvedValue("/musica/preludio.mid");
    vi.mocked(puente.abrirCancion).mockResolvedValue(RESUMEN);

    render(<App />);
    await userEvent.click(screen.getByRole("button", { name: /abrir/i }));

    await waitFor(() =>
      expect(puente.abrirCancion).toHaveBeenCalledWith("/musica/preludio.mid"),
    );
  });

  it("no invoca nada si se cancela el diálogo", async () => {
    // Cancelar es lo normal, no un error: no debe abrir nada ni avisar de nada.
    vi.mocked(puente.elegirArchivo).mockResolvedValue(null);

    render(<App />);
    await userEvent.click(screen.getByRole("button", { name: /abrir/i }));

    await waitFor(() => expect(puente.elegirArchivo).toHaveBeenCalled());
    expect(puente.abrirCancion).not.toHaveBeenCalled();
    expect(screen.queryByRole("alert")).not.toBeInTheDocument();
  });
});

describe("un archivo que no se puede leer", () => {
  it("muestra el motivo que devuelve el núcleo", async () => {
    // FR-004. El motivo se muestra tal cual: la interfaz no lo interpreta ni lo sustituye
    // por un mensaje genérico, porque el núcleo sabe mejor qué ha pasado.
    vi.mocked(puente.elegirArchivo).mockResolvedValue("/musica/roto.mid");
    vi.mocked(puente.abrirCancion).mockRejectedValue(
      "cabecera MThd ausente",
    );

    render(<App />);
    await userEvent.click(screen.getByRole("button", { name: /abrir/i }));

    const aviso = await screen.findByRole("alert");
    expect(aviso).toHaveTextContent(/cabecera MThd ausente/);
  });

  it("deja la aplicación utilizable y permite reintentar con otro archivo", async () => {
    // La otra mitad de FR-004, y la que se olvida: no basta con avisar, hay que seguir
    // en pie. Ni pantalla en blanco ni estado a medias.
    vi.mocked(puente.elegirArchivo).mockResolvedValue("/musica/roto.mid");
    vi.mocked(puente.abrirCancion).mockRejectedValue("archivo vacío");

    render(<App />);
    await userEvent.click(screen.getByRole("button", { name: /abrir/i }));
    await screen.findByRole("alert");

    // El botón sigue ahí y sigue funcionando.
    const boton = screen.getByRole("button", { name: /abrir/i });
    expect(boton).toBeEnabled();

    vi.mocked(puente.elegirArchivo).mockResolvedValue("/musica/bueno.mid");
    vi.mocked(puente.abrirCancion).mockResolvedValue(RESUMEN);
    await userEvent.click(boton);

    await waitFor(() =>
      expect(puente.abrirCancion).toHaveBeenCalledWith("/musica/bueno.mid"),
    );
    // Y el aviso anterior desaparece: no se acumulan errores viejos.
    await waitFor(() => expect(screen.queryByRole("alert")).not.toBeInTheDocument());
  });

  it("un fallo al abrir no deja la aplicación sin el control de manos", async () => {
    vi.mocked(puente.elegirArchivo).mockResolvedValue("/musica/roto.mid");
    vi.mocked(puente.abrirCancion).mockRejectedValue("no se pudo leer");

    render(<App />);
    await userEvent.click(screen.getByRole("button", { name: /abrir/i }));
    await screen.findByRole("alert");

    expect(screen.getByRole("slider", { name: /corte/i })).toBeInTheDocument();
  });
});
