// El único punto por el que la interfaz habla con el núcleo.
//
// Está aislado a propósito: las pruebas de `App` sustituyen este módulo entero y así
// comprueban el comportamiento de la interfaz sin levantar Tauri ni tocar el disco.

import { Channel, invoke } from "@tauri-apps/api/core";
import type { DispositivoPlano } from "../dispositivos/Selector";
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

/**
 * Un ancla tal como llega del núcleo.
 *
 * `instanteUs` viene del **reloj de sesión de Rust**, cuyo cero no es el de la pantalla.
 * Antes de interpolar hay que pasarlo por `anclarEnRelojLocal`.
 */
export type AnclaDelNucleo = {
  posicionUs: number;
  instanteUs: number;
  num: number;
  den: number;
  topeUs: number | null;
};

/** Pone la canción en marcha desde donde esté. Devuelve ancla si cambió el régimen. */
export async function marcha(): Promise<AnclaDelNucleo | null> {
  return invoke<AnclaDelNucleo | null>("transporte_marcha");
}

/** Detiene el avance sin perder la posición. */
export async function pausa(): Promise<AnclaDelNucleo | null> {
  return invoke<AnclaDelNucleo | null>("transporte_pausa");
}

/** Lleva el cursor a una posición concreta, en microsegundos. */
export async function saltarA(posicionUs: number): Promise<AnclaDelNucleo | null> {
  return invoke<AnclaDelNucleo | null>("transporte_saltar", { posicionUs });
}

/**
 * Cambia la velocidad. Se envía como **racional**, no como decimal.
 *
 * Es lo que hace que reducir a la mitad y volver a normal deje la posición exactamente
 * donde estaba; un decimal por el puente rompería esa garantía en el primer redondeo.
 */
export async function cambiarVelocidad(v: {
  num: number;
  den: number;
}): Promise<AnclaDelNucleo | null> {
  return invoke<AnclaDelNucleo | null>("transporte_velocidad", { num: v.num, den: v.den });
}

/** Un mensaje del núcleo, tal como llega por el canal. */
import type { MensajeDelNucleo } from "./modelo";
export type { MensajeDelNucleo };
export type { DispositivoPlano };

/**
 * Abre el canal por el que el núcleo empuja teclas, anclas y avisos.
 *
 * **Un solo canal para todo**, discriminado por etiqueta: así el orden entre las teclas y
 * las anclas queda garantizado por construcción. Con dos canales no lo estaría.
 */
export async function escucharCanal(
  alRecibir: (m: MensajeDelNucleo) => void,
): Promise<void> {
  const canal = new Channel<MensajeDelNucleo>();
  canal.onmessage = alRecibir;
  return invoke<void>("registrar_canal", { canal });
}

/** En qué situación está el teclado al arrancar. */
export type EstadoDelTeclado =
  | { tipo: "conectado"; nombre: string }
  | { tipo: "hayQueElegir"; dispositivos: DispositivoPlano[] }
  | { tipo: "sinDispositivos" };

/**
 * Intenta conectar el teclado recordado.
 *
 * **Nunca abre "el primero que haya"**: si el recordado no está, devuelve la lista para
 * que el alumno elija (FR-025). Abrir otro sería capturar de un aparato que no eligió, y
 * lo notaría porque nada respondería, sin ninguna pista de por qué.
 */
export async function conectarTeclado(): Promise<EstadoDelTeclado> {
  return invoke<EstadoDelTeclado>("conectar_teclado");
}

/** Recuerda esta elección y empieza a capturar de ella. */
export async function elegirTeclado(d: DispositivoPlano): Promise<EstadoDelTeclado> {
  return invoke<EstadoDelTeclado>("elegir_teclado", {
    posicion: d.posicion,
    nombre: d.nombre,
  });
}

/** Cambia entre reproducir y esperar. Conserva la posición (FR-021). */
export async function cambiarModo(
  m: "porReloj" | "porAcierto",
): Promise<AnclaDelNucleo | null> {
  return invoke<AnclaDelNucleo | null>("transporte_modo", { espera: m === "porAcierto" });
}

/** Elige qué mano se practica. `null` son las dos. */
export async function practicarMano(
  m: "izquierda" | "derecha" | null,
): Promise<AnclaDelNucleo | null> {
  return invoke<AnclaDelNucleo | null>("ajustar_mano", { mano: m });
}

/** Salta la nota pendiente sin acertarla (FR-020). */
export async function saltarPuerta(): Promise<AnclaDelNucleo | null> {
  return invoke<AnclaDelNucleo | null>("transporte_saltar_puerta");
}
