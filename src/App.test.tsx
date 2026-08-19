// T040a y T045a — abrir una canción desde la interfaz.
import { act, cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
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
  vi.mocked(puente.conectarTeclado).mockResolvedValue({ tipo: "conectado", nombre: "Piano de pruebas" });
  vi.mocked(puente.escucharCanal).mockResolvedValue();
  vi.mocked(puente.elegirTeclado).mockResolvedValue({ tipo: "sinDispositivos" });
  vi.mocked(puente.cambiarModo).mockResolvedValue(null);
  vi.mocked(puente.practicarMano).mockResolvedValue(null);
  vi.mocked(puente.saltarPuerta).mockResolvedValue(null);
  vi.mocked(puente.ultimoResultado).mockResolvedValue(null);
  vi.mocked(puente.cambiarNivel).mockResolvedValue();
  vi.mocked(puente.compararConAnterior).mockResolvedValue(null);
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

describe("el teclado", () => {
  async function abrirUna() {
    vi.mocked(puente.elegirArchivo).mockResolvedValue("/musica/a.mid");
    vi.mocked(puente.abrirCancion).mockResolvedValue(RESUMEN);
    render(<App />);
    await userEvent.click(screen.getByRole("button", { name: /abrir/i }));
    await waitFor(() => expect(puente.abrirCancion).toHaveBeenCalled());
  }

  it("sin teclado avisa, pero la canción se ve y se reproduce igual", async () => {
    // FR-015. Lo importante no es el aviso: es que NADA quede desactivado por su culpa.
    vi.mocked(puente.conectarTeclado).mockResolvedValue({ tipo: "sinDispositivos" });
    await abrirUna();

    expect(await screen.findByText(/sin teclado|no se detect/i)).toBeInTheDocument();
    // La canción está cargada y los mandos siguen vivos.
    expect(screen.getByText(/3 notas/)).toBeInTheDocument();
    const reproducir = screen.getByRole("button", { name: /reproducir/i });
    expect(reproducir).toBeEnabled();
    await userEvent.click(reproducir);
    expect(puente.marcha).toHaveBeenCalled();
    expect(screen.getByRole("slider", { name: /corte/i })).toBeEnabled();
  });

  it("con teclado no muestra el aviso", async () => {
    vi.mocked(puente.conectarTeclado).mockResolvedValue({ tipo: "conectado", nombre: "Piano de pruebas" });
    await abrirUna();
    await waitFor(() => expect(puente.conectarTeclado).toHaveBeenCalled());
    expect(screen.queryByText(/sin teclado|no se detect/i)).not.toBeInTheDocument();
  });

  it("perder el teclado a mitad avisa sin detener la reproducción", async () => {
    // FR-016. La prueba que importa: que la canción NO se pare.
    vi.mocked(puente.conectarTeclado).mockResolvedValue({ tipo: "conectado", nombre: "Piano de pruebas" });
    let emitir: ((m: puente.MensajeDelNucleo) => void) | null = null;
    vi.mocked(puente.escucharCanal).mockImplementation((cb) => {
      emitir = cb;
      return Promise.resolve();
    });

    await abrirUna();
    await userEvent.click(screen.getByRole("button", { name: /reproducir/i }));
    await waitFor(() => expect(emitir).not.toBeNull());

    await act(async () => {
      emitir?.({ tipo: "dispositivoPerdido" });
    });

    expect(await screen.findByText(/se perdió|desconect/i)).toBeInTheDocument();
    // Sigue en marcha: el botón ofrece pausar, no reproducir.
    expect(screen.getByRole("button", { name: /pausar/i })).toBeInTheDocument();
    expect(puente.pausa).not.toHaveBeenCalled();
  });

  it("las teclas que llegan por el canal se pintan", async () => {
    vi.mocked(puente.conectarTeclado).mockResolvedValue({ tipo: "conectado", nombre: "Piano de pruebas" });
    let emitir: ((m: puente.MensajeDelNucleo) => void) | null = null;
    vi.mocked(puente.escucharCanal).mockImplementation((cb) => {
      emitir = cb;
      return Promise.resolve();
    });

    await abrirUna();
    await waitFor(() => expect(emitir).not.toBeNull());
    await act(async () => {
      emitir?.({ tipo: "tecla", key: 60, pulsada: true });
    });

    const pulsadas = vi.mocked(escena.construirEscena).mock.lastCall?.[2];
    expect(pulsadas?.has(60)).toBe(true);
  });
});

describe("elegir teclado al arrancar", () => {
  it("cuando el recordado no está, ofrece elegir y NO abre otro", async () => {
    // FR-025. Lo que se comprueba no es solo que aparezca la lista: es que **no** se haya
    // conectado nada por su cuenta. Abrir el primero disponible sería capturar de un
    // aparato que el alumno no eligió, y lo notaría porque nada respondería.
    vi.mocked(puente.conectarTeclado).mockResolvedValue({
      tipo: "hayQueElegir",
      dispositivos: [
        { nombre: "Casio CDP-S110", posicion: 0, idSistema: 11 },
        { nombre: "Casio CDP-S110", posicion: 1, idSistema: 22 },
      ],
    });
    render(<App />);

    expect(await screen.findByText(/elige tu teclado/i)).toBeInTheDocument();
    expect(puente.elegirTeclado).not.toHaveBeenCalled();
  });

  it("elegir uno lo conecta y la lista desaparece", async () => {
    vi.mocked(puente.conectarTeclado).mockResolvedValue({
      tipo: "hayQueElegir",
      dispositivos: [{ nombre: "Casio CDP-S110", posicion: 1, idSistema: 22 }],
    });
    vi.mocked(puente.elegirTeclado).mockResolvedValue({
      tipo: "conectado",
      nombre: "Casio CDP-S110",
    });
    render(<App />);

    await userEvent.click(await screen.findByRole("button", { name: /Casio/ }));
    await waitFor(() =>
      expect(puente.elegirTeclado).toHaveBeenCalledWith({
        nombre: "Casio CDP-S110",
        posicion: 1,
        idSistema: 22,
      }),
    );
    await waitFor(() =>
      expect(screen.queryByText(/elige tu teclado/i)).not.toBeInTheDocument(),
    );
  });

  it("mientras se elige teclado, la canción se sigue pudiendo abrir", async () => {
    // FR-015 otra vez: nada de esto puede bloquear ver y reproducir.
    vi.mocked(puente.conectarTeclado).mockResolvedValue({
      tipo: "hayQueElegir",
      dispositivos: [{ nombre: "Casio", posicion: 0, idSistema: 1 }],
    });
    vi.mocked(puente.elegirArchivo).mockResolvedValue("/musica/a.mid");
    vi.mocked(puente.abrirCancion).mockResolvedValue(RESUMEN);
    render(<App />);

    await screen.findByText(/elige tu teclado/i);
    await userEvent.click(screen.getByRole("button", { name: /abrir una canción/i }));
    await waitFor(() => expect(puente.abrirCancion).toHaveBeenCalled());
    expect(screen.getByRole("button", { name: /reproducir/i })).toBeEnabled();
  });
});

describe("cuando el propio diálogo falla", () => {
  it("lo dice, en vez de no hacer absolutamente nada", async () => {
    // Encontrado ejecutando la app: faltaba el permiso `dialog:default` en las capacidades
    // de Tauri, así que el diálogo se denegaba. Pero el fallo de verdad es que
    // `elegirArchivo()` estaba FUERA del try: el rechazo quedaba sin capturar y al alumno
    // no le pasaba nada al pulsar «abrir». Ni archivo, ni error, ni pista.
    //
    // Un fallo silencioso es peor que uno ruidoso: no hay nada que buscar.
    vi.mocked(puente.elegirArchivo).mockRejectedValue(
      new Error("dialog.open not allowed"),
    );

    render(<App />);
    await userEvent.click(screen.getByRole("button", { name: /abrir/i }));

    const aviso = await screen.findByRole("alert");
    expect(aviso).toHaveTextContent(/dialog\.open not allowed/);
  });

  it("y la aplicación sigue en pie", async () => {
    vi.mocked(puente.elegirArchivo).mockRejectedValue(new Error("lo que sea"));
    render(<App />);
    await userEvent.click(screen.getByRole("button", { name: /abrir/i }));
    await screen.findByRole("alert");
    expect(screen.getByRole("button", { name: /abrir/i })).toBeEnabled();
    expect(screen.getByRole("slider", { name: /corte/i })).toBeInTheDocument();
  });
});

describe("ningún mando falla en silencio", () => {
  // El fallo de «abrir» no era un caso aislado sino una CLASE: ninguna llamada al puente
  // tenía try. Si el núcleo devuelve un error, el alumno tiene que enterarse; si no, la app
  // parece rota sin decir de qué.
  async function abrirUna() {
    vi.mocked(puente.elegirArchivo).mockResolvedValue("/a.mid");
    vi.mocked(puente.abrirCancion).mockResolvedValue(RESUMEN);
    render(<App />);
    await userEvent.click(screen.getByRole("button", { name: /abrir/i }));
    await waitFor(() => expect(puente.abrirCancion).toHaveBeenCalled());
  }

  it("reproducir", async () => {
    await abrirUna();
    vi.mocked(puente.marcha).mockRejectedValue(new Error("no se pudo arrancar"));
    await userEvent.click(screen.getByRole("button", { name: /reproducir/i }));
    expect(await screen.findByRole("alert")).toHaveTextContent(/no se pudo arrancar/);
  });

  it("mover el corte", async () => {
    await abrirUna();
    vi.mocked(puente.ajustarCorte).mockRejectedValue(new Error("corte inválido"));
    fireEvent.change(screen.getByRole("slider", { name: /corte/i }), {
      target: { value: "80" },
    });
    expect(await screen.findByRole("alert")).toHaveTextContent(/corte inválido/);
  });

  it("volver al principio", async () => {
    await abrirUna();
    vi.mocked(puente.saltarA).mockRejectedValue(new Error("no se pudo saltar"));
    await userEvent.click(screen.getByRole("button", { name: /al principio/i }));
    expect(await screen.findByRole("alert")).toHaveTextContent(/no se pudo saltar/);
  });
});

describe("la ventana de notas sigue a la reproducción", () => {
  it("pide más notas a medida que la canción avanza", async () => {
    // Encontrado usando la app: las notas dejaban de verse pasado cierto punto. `refrescar`
    // pedía SIEMPRE los primeros cuatro segundos y solo al abrir, así que en cuanto el
    // cursor los pasaba no quedaba nada que dibujar.
    //
    // Y no basta con pedir la ventana correcta una vez: hay que volver a pedirla al avanzar,
    // sin cruzar el puente sesenta veces por segundo, que es justo lo que el ancla evita.
    vi.mocked(puente.elegirArchivo).mockResolvedValue("/a.mid");
    vi.mocked(puente.abrirCancion).mockResolvedValue(RESUMEN);
    vi.mocked(puente.marcha).mockResolvedValue({
      posicionUs: 0,
      instanteUs: 0,
      num: 1,
      den: 1,
      topeUs: null,
    });

    let ahoraMs = 0;
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

    const peticionesIniciales = vi.mocked(puente.vistaActual).mock.calls.length;

    // Treinta segundos de canción: muy por delante de cualquier ventana inicial.
    ahoraMs = 30_000;
    await act(async () => {
      pendientes.shift()?.(0);
    });
    await waitFor(() =>
      expect(vi.mocked(puente.vistaActual).mock.calls.length).toBeGreaterThan(
        peticionesIniciales,
      ),
    );

    // Y la última petición cubre la posición actual, no el principio de la canción.
    const ultima = vi.mocked(puente.vistaActual).mock.lastCall;
    expect(ultima?.[1]).toBeGreaterThan(30_000_000);
  });

  it("no cruza el puente en cada fotograma", async () => {
    // La razón de ser del ancla es que el cursor NO cruce sesenta veces por segundo. Pedir
    // las notas en cada cuadro tiraría eso por tierra.
    vi.mocked(puente.elegirArchivo).mockResolvedValue("/a.mid");
    vi.mocked(puente.abrirCancion).mockResolvedValue(RESUMEN);
    vi.mocked(puente.marcha).mockResolvedValue({
      posicionUs: 0,
      instanteUs: 0,
      num: 1,
      den: 1,
      topeUs: null,
    });

    let ahoraMs = 0;
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

    const antes = vi.mocked(puente.vistaActual).mock.calls.length;
    // Sesenta cuadros dentro del mismo segundo.
    for (let i = 0; i < 60; i += 1) {
      ahoraMs = i * 16;
      await act(async () => {
        pendientes.shift()?.(0);
      });
    }
    const despues = vi.mocked(puente.vistaActual).mock.calls.length;
    expect(despues - antes).toBeLessThan(5);
  });
});
