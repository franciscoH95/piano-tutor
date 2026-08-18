// El único punto por el que la interfaz habla con el núcleo.
//
// Está aislado a propósito: las pruebas de `App` sustituyen este módulo entero y así
// comprueban el comportamiento de la interfaz sin levantar Tauri ni tocar el disco.

import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";

/** Lo que el núcleo cuenta de una canción recién abierta. */
export type ResumenCancion = {
  notas: number;
  duracionUs: number;
  /** El archivo traía las dos manos separadas. */
  vocesDelArchivo: boolean;
  corte: number;
};

/** Una nota lista para dibujar. El nombre viaja simbólico, sin formatear. */
export type NotaVisiblePlana = {
  indice: number;
  key: number;
  onsetUs: number;
  endUs: number;
  derecha: boolean;
  /** Dedo propuesto, de 1 a 5. */
  dedo: number;
  /** 0 = Do, 1 = Re, ... 6 = Si. */
  base: number;
  /** -1 bemol, 0 natural, 1 sostenido. */
  alteracion: number;
  estado: "pendiente" | "sonando" | "acertada" | "omitida";
};

/** Abre el diálogo del sistema. Devuelve `null` si el usuario cancela. */
export async function elegirArchivo(): Promise<string | null> {
  const elegido = await open({
    multiple: false,
    directory: false,
    filters: [{ name: "MIDI", extensions: ["mid", "midi"] }],
  });
  return typeof elegido === "string" ? elegido : null;
}

/** Carga la canción. Rechaza con el motivo que devuelve el núcleo. */
export async function abrirCancion(ruta: string): Promise<ResumenCancion> {
  return invoke<ResumenCancion>("abrir_cancion", { ruta });
}

/** Mueve el punto de corte entre manos. Rehace manos y digitación. */
export async function ajustarCorte(corte: number): Promise<void> {
  return invoke<void>("ajustar_corte", { corte });
}

/** Las notas que caen en la ventana pedida. */
export async function vistaActual(
  desdeUs: number,
  hastaUs: number,
): Promise<NotaVisiblePlana[]> {
  return invoke<NotaVisiblePlana[]>("vista_actual", { desdeUs, hastaUs });
}
