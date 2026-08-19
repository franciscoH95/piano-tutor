// De datos del núcleo a rectángulos.
//
// Todo lo que decide **dónde** va algo vive aquí, con pruebas, y no en `Lienzo.tsx`, que
// es el único archivo acogido a la excepción del Principio II y por eso el único sin
// cobertura. La excepción no se amplía: se estrecha.

import type { Escena, Etiqueta, Rect } from "./Lienzo";
import type { NotaVisiblePlana } from "./puente";

/** La 0: la tecla más grave de un piano de 88. */
export const TECLA_MAS_GRAVE = 21;
/** Do 8: la más aguda. */
export const TECLA_MAS_AGUDA = 108;
/** Cuánta canción se ve por delante. Acotado: el coste del fotograma no debe crecer con
 *  la longitud de la pieza. */
export const VENTANA_US = 4_000_000;

const ANCHO = 1200;
const ALTO = 600;
/** Alto de la franja del teclado, en unidades de escena. */
export const ALTO_TECLADO = 90;

const COLOR = {
  fondo: "#111",
  blanca: "#fff",
  negra: "#222",
  blancaPulsada: "#4a9eff",
  negraPulsada: "#1f6fbf",
  derecha: "#4a9eff",
  izquierda: "#ff9a4a",
  sonandoDerecha: "#b3d9ff",
  sonandoIzquierda: "#ffd6b3",
  etiqueta: "#fff",
} as const;

const NOMBRES = ["Do", "Re", "Mi", "Fa", "Sol", "La", "Si"] as const;

/** Los cinco grados alterados de la octava. */
export function esNegra(key: number): boolean {
  return [1, 3, 6, 8, 10].includes(((key % 12) + 12) % 12);
}

/** Compone el texto a partir del símbolo. El núcleo manda base y alteración por separado
 *  precisamente para que el idioma se decida aquí. */
export function nombreDeNota(base: number, alteracion: number): string {
  const letra = NOMBRES[base] ?? "?";
  if (alteracion > 0) return `${letra}♯`;
  if (alteracion < 0) return `${letra}♭`;
  return letra;
}

/** Cuántas blancas hay por debajo de una tecla: es lo que fija su sitio horizontal. */
function blancasHasta(key: number): number {
  let n = 0;
  for (let k = TECLA_MAS_GRAVE; k < key; k += 1) if (!esNegra(k)) n += 1;
  return n;
}

const TOTAL_BLANCAS = blancasHasta(TECLA_MAS_AGUDA + 1);
const ANCHO_BLANCA = ANCHO / TOTAL_BLANCAS;

/** Dónde cae una tecla, y cuánto ocupa. */
function sitio(key: number): { x: number; ancho: number } {
  const ancho = esNegra(key) ? ANCHO_BLANCA * 0.6 : ANCHO_BLANCA;
  const x = esNegra(key)
    ? blancasHasta(key) * ANCHO_BLANCA - ancho / 2
    : blancasHasta(key) * ANCHO_BLANCA;
  return { x, ancho };
}

function teclado(pulsadas: ReadonlySet<number>): Rect[] {
  const blancas: Rect[] = [];
  const negras: Rect[] = [];
  for (let key = TECLA_MAS_GRAVE; key <= TECLA_MAS_AGUDA; key += 1) {
    const { x, ancho } = sitio(key);
    const negra = esNegra(key);
    // Una negra pulsada y una blanca pulsada llevan colores distintos: con el mismo, sobre
    // fondo oscuro, dejarían de distinguirse justo cuando importa verlas.
    const pulsada = pulsadas.has(key);
    const color = pulsada
      ? negra
        ? COLOR.negraPulsada
        : COLOR.blancaPulsada
      : negra
        ? COLOR.negra
        : COLOR.blanca;
    (negra ? negras : blancas).push({
      x,
      y: ALTO - ALTO_TECLADO,
      ancho,
      alto: negra ? ALTO_TECLADO * 0.6 : ALTO_TECLADO,
      color,
    });
  }
  // Las negras van después: pintadas en el orden natural, la blanca siguiente taparía a
  // la negra anterior.
  return [...blancas, ...negras];
}

function colorDeNota(n: NotaVisiblePlana): string {
  if (n.estado === "sonando") {
    return n.derecha ? COLOR.sonandoDerecha : COLOR.sonandoIzquierda;
  }
  return n.derecha ? COLOR.derecha : COLOR.izquierda;
}

/**
 * Convierte las notas visibles en rectángulos y etiquetas.
 *
 * `posicionUs` es dónde está la práctica: las notas se sitúan **relativas a ella**, de modo
 * que la canción cae hacia el teclado y la que suena ahora queda al ras de las teclas. El
 * tamaño de cada nota no depende de la posición, solo su sitio: si se encogiera al
 * acercarse, el alumno leería mal su duración.
 */
export function construirEscena(
  notas: NotaVisiblePlana[],
  posicionUs = 0,
  pulsadas: ReadonlySet<number> = new Set(),
): Escena {
  const rects: Rect[] = [];
  const etiquetas: Etiqueta[] = [];
  const alturaUtil = ALTO - ALTO_TECLADO;

  for (const n of notas) {
    // Una nota fuera del piano no se pinta: es preferible no enseñarla a enseñarla en un
    // sitio que no le corresponde.
    if (n.key < TECLA_MAS_GRAVE || n.key > TECLA_MAS_AGUDA) continue;

    const { x, ancho } = sitio(n.key);
    const duracion = Math.max(0, n.endUs - n.onsetUs);
    const alto = (duracion / VENTANA_US) * alturaUtil;
    // Cuanto más lejos queda el ataque, más arriba se dibuja: la nota cae hacia el teclado.
    const restante = n.onsetUs - posicionUs;
    const y = alturaUtil - (restante / VENTANA_US) * alturaUtil - alto;

    rects.push({ x, y, ancho, alto, color: colorDeNota(n) });
    etiquetas.push({
      x,
      y: y + alto,
      texto: `${nombreDeNota(n.base, n.alteracion)} ${n.dedo}`,
      color: COLOR.etiqueta,
    });
  }

  return {
    ancho: ANCHO,
    alto: ALTO,
    fondo: COLOR.fondo,
    teclas: teclado(pulsadas),
    notas: rects,
    etiquetas,
  };
}
