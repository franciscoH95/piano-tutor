// Dónde está la práctica en cada fotograma.
//
// El cursor **no cruza el puente sesenta veces por segundo**. Rust envía un *ancla* solo
// cuando cambia el régimen —al arrancar, al pausar, al saltar, al cambiar de velocidad, al
// llegar a una nota pendiente— y aquí se interpola entre anclas. Eso es lo que mantiene el
// puente casi vacío y el reflejo por debajo de su presupuesto.
//
// Este archivo **sí se prueba**: decide dónde está la práctica, y eso es una decisión. La
// excepción del Principio II cubre únicamente `Lienzo.tsx`, que solo pinta.

/** Un punto de referencia desde el que interpolar. */
export interface Ancla {
  /** Posición dentro de la canción en ese instante. */
  posicionUs: number;
  /** Instante del reloj de sesión en que se tomó el ancla. */
  instanteUs: number;
  /** Velocidad como racional: `num`/`den`. Pausa es 0/1. */
  num: number;
  den: number;
  /** Hasta dónde puede avanzar. En modo espera es la nota pendiente; sin tope, `null`. */
  topeUs: number | null;
}

/**
 * Posición dentro de la canción en el instante `ahoraUs` del reloj de sesión.
 *
 * Nunca retrocede y nunca pasa del tope. El resultado es un entero de microsegundos: la
 * velocidad es un racional a propósito, para que reducir a la mitad y volver no acumule
 * error, y devolver fracciones aquí desharía esa garantía.
 */
export function posicionEn(ancla: Ancla, ahoraUs: number): number {
  const transcurrido = Math.max(0, ahoraUs - ancla.instanteUs);
  const avance = ancla.den === 0 ? 0 : Math.floor((transcurrido * ancla.num) / ancla.den);
  const posicion = ancla.posicionUs + avance;
  return ancla.topeUs === null ? posicion : Math.min(posicion, ancla.topeUs);
}

/**
 * Vuelve a anclar en el reloj de la pantalla.
 *
 * El ancla llega con `instanteUs` del **reloj de sesión de Rust**, cuyo cero es el arranque
 * del proceso; la pantalla lee `performance.now()`, cuyo cero es otro. Mezclarlos daría una
 * posición disparatada, y el desfase sería silencioso: no rompe ninguna prueba de un solo
 * lado, solo pinta el cursor donde el núcleo no cree que está.
 *
 * Al llegar un ancla se sustituye su instante por la lectura local de ese momento. Lo que
 * se pierde es el tiempo de tránsito por el puente, que queda absorbido como un desfase
 * constante —del orden del milisegundo— y **se vuelve a poner a cero con cada ancla nueva**,
 * así que no se acumula. La alternativa, preguntar la posición en cada fotograma, tiraría
 * por tierra la razón de ser del ancla.
 */
export function anclarEnRelojLocal(ancla: Ancla, llegadaUs: number): Ancla {
  return { ...ancla, instanteUs: llegadaUs };
}
