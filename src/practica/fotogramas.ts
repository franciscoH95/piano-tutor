// Las cinco cifras de SC-003, calculadas a partir de una traza de fotogramas.
//
// # Por qué esto vive aquí y no en un banco de Rust
//
// La especificación lo dice: **sin una ventana visible en pantalla el sistema no dibuja ni
// un fotograma**. La medición tiene que ocurrir dentro de la ventana real, con las marcas
// de tiempo que da `requestAnimationFrame`. Lo que sí se puede probar —y es donde está el
// riesgo— es el CÁLCULO: SC-003c avisa de que un informe que ignore las suspensiones
// publica un número inventado, y eso es lógica, no instrumentación.

/** Por encima de esto, el sistema suspendió el dibujo: la ventana quedó tapada. */
export const SUSPENSION_MS = 200;
/** El ritmo de referencia. */
export const OBJETIVO_MS = 1000 / 60;
/** SC-003a. */
export const LARGO_MS = 25;
/** Por debajo de esta fracción medida, el informe no describe la reproducción. */
const FRACCION_MINIMA_VALIDA = 0.5;

export interface Informe {
  /** SC-003: porcentaje de los fotogramas que debían mostrarse y se mostraron. */
  mostrados: number;
  /** SC-003a: cuántos intervalos superaron los 25 ms. */
  intervalosLargos: number;
  /** SC-003b: el peor intervalo, excluidas las suspensiones. */
  peorIntervaloMs: number;
  /** SC-003c: cada suspensión detectada, en milisegundos. */
  suspensiones: number[];
  /** SC-003d: percentil 95 del coste de pintar, o `null` si no se midió. */
  pintadoP95Ms: number | null;
  msTotales: number;
  msMedidos: number;
  msSuspendidos: number;
  /** `false` si se suspendió tanto que el informe no describe nada. */
  valido: boolean;
}

function percentil95(xs: number[]): number | null {
  if (xs.length === 0) return null;
  const orden = [...xs].sort((a, b) => a - b);
  const i = Math.min(orden.length - 1, Math.ceil(orden.length * 0.95) - 1);
  return orden[Math.max(0, i)] ?? null;
}

/**
 * Analiza una traza de instantes de fotograma, en milisegundos.
 *
 * `pintados` son los cronómetros internos del dibujo, si se midieron. Van **aparte** a
 * propósito: la pantalla puede ir a tirones con un dibujo baratísimo, así que mezclarlos
 * escondería cuál de los dos es el problema.
 */
export function analizar(instantes: number[], pintados: number[] = []): Informe {
  const vacio: Informe = {
    mostrados: 0,
    intervalosLargos: 0,
    peorIntervaloMs: 0,
    suspensiones: [],
    pintadoP95Ms: percentil95(pintados),
    msTotales: 0,
    msMedidos: 0,
    msSuspendidos: 0,
    valido: false,
  };
  if (instantes.length < 2) return vacio;

  const suspensiones: number[] = [];
  let msMedidos = 0;
  let intervalosLargos = 0;
  let peor = 0;
  let mostradosReales = 0;

  for (let i = 1; i < instantes.length; i += 1) {
    const dt = (instantes[i] ?? 0) - (instantes[i - 1] ?? 0);
    if (dt > SUSPENSION_MS) {
      // Suspensión: se excluye del cálculo Y se declara. Contarla como fotogramas perdidos
      // hundiría el porcentaje con algo que no es un fallo de la aplicación.
      suspensiones.push(dt);
      continue;
    }
    msMedidos += dt;
    mostradosReales += 1;
    if (dt > LARGO_MS) intervalosLargos += 1;
    if (dt > peor) peor = dt;
  }

  const msTotales = (instantes[instantes.length - 1] ?? 0) - (instantes[0] ?? 0);
  const msSuspendidos = suspensiones.reduce((a, b) => a + b, 0);
  // Cuántos DEBERÍAN haberse mostrado en el tiempo efectivamente medido.
  const esperados = msMedidos / OBJETIVO_MS;
  const mostrados = esperados > 0 ? Math.min(100, (mostradosReales / esperados) * 100) : 0;

  return {
    mostrados,
    intervalosLargos,
    peorIntervaloMs: peor,
    suspensiones,
    pintadoP95Ms: percentil95(pintados),
    msTotales,
    msMedidos,
    msSuspendidos,
    valido: msTotales > 0 && msMedidos / msTotales >= FRACCION_MINIMA_VALIDA,
  };
}
