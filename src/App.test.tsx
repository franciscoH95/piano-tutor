// T040a y T045a — abrir una canción desde la interfaz.
import { act, cleanup, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import App from "./App";
import * as escena from "./practica/escena";
import * as puente from "./practica/puente";

vi.mock("./practica/puente");
// Se espía la construcción de escena para leer con qué posición se pinta cada fotograma.
vi.mock("./practica/escena", async (real) => {
  const m = await real<typeof escena>();
  return { ...m, construirEscena: vi.fn(m.construirEscena) };
});

const RESUMEN = {
  notas: 3,
  duracionUs: 2_000_000,
  vocesDelArchivo: false,
  corte: 60,
};

beforeEach(() => {
  vi.mocked(puente.vistaActual).mockResolvedValue([]);
  vi.mocked(puente.ajustarCorte).mockResolvedValue();
  vi.mocked(puente.marcha).mockResolvedValue(null);
  vi.mocked(puente.pausa).mockResolvedValue(null);
  vi.mocked(puente.saltarA).mockResolvedValue(null);
  vi.mocked(puente.cambiarVelocidad).mockResolvedValue(null);
});

afterEach(() => {
  cleanup();
  vi.resetAllMocks();
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
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

describe("el bucle de dibujo", () => {
  it("deriva la posición del reloj y no del número de fotograma", async () => {
    // T059. Es la diferencia entre una animación correcta y una que se desincroniza en
    // cuanto el navegador salta un fotograma: si la posición viniera de contar cuadros,
    // dos cadencias distintas darían dos posiciones distintas para el mismo instante.
    vi.mocked(puente.elegirArchivo).mockResolvedValue("/musica/a.mid");
    vi.mocked(puente.abrirCancion).mockResolvedValue(RESUMEN);
    vi.mocked(puente.marcha).mockResolvedValue({
      posicionUs: 0,
      instanteUs: 900_000_000, // reloj de Rust: la aplicación debe reanclarlo
      num: 1,
      den: 1,
      topeUs: 2_000_000,
    });

    // Reloj local controlado y un `requestAnimationFrame` que solo dispara cuando se le
    // pide: así el número de fotogramas y el tiempo transcurrido son independientes.
    let ahoraMs = 1_000;
    const pendientes: FrameRequestCallback[] = [];
    vi.spyOn(performance, "now").mockImplementation(() => ahoraMs);
    vi.stubGlobal("requestAnimationFrame", (cb: FrameRequestCallback) => {
      pendientes.push(cb);
      return pendientes.length;
    });
    vi.stubGlobal("cancelAnimationFrame", () => {});

    render(<App />);
    await userEvent.click(screen.getByRole("button", { name: /abrir/i }));
    await waitFor(() => expect(puente.abrirCancion).toHaveBeenCalled());
    await userEvent.click(screen.getByRole("button", { name: /reproducir/i }));
    await waitFor(() => expect(pendientes.length).toBeGreaterThan(0));

    // Un solo fotograma, medio segundo de reloj después.
    ahoraMs = 1_500;
    await act(async () => {
      pendientes.shift()?.(0);
    });
    const trasUnFotograma = vi.mocked(escena.construirEscena).mock.lastCall?.[1];

    // Y ahora diez fotogramas SIN que el reloj avance: la posición no puede moverse.
    for (let i = 0; i < 10; i += 1) {
      await act(async () => {
        pendientes.shift()?.(0);
      });
    }
    const trasOnceFotogramas = vi.mocked(escena.construirEscena).mock.lastCall?.[1];

    expect(trasUnFotograma).toBe(500_000);
    expect(trasOnceFotogramas).toBe(500_000);
  });
});
