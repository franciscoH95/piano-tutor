// LA CAPA QUE PINTA. Único archivo acogido a la excepción del Principio II
// (Constitución v1.1.0), y solo mientras cumpla su condición: **no decidir nada**.
//
// Recibe una escena ya calculada —rectángulos, etiquetas, teclas iluminadas— y la dibuja.
// No consulta la hora, no lee estado global, no calcula posiciones musicales, no sabe qué
// es un compás ni un tempo. Si aquí aparece un `if` sobre algo musical, ese `if` pertenece
// a `piano-core`, donde sí está cubierto por pruebas.
//
// PROHIBIDO por medición: sombras, desenfoques y filtros. `shadowBlur = 8` con 298 notas
// hundió la cadencia a 40,9 fotogramas por segundo **mientras el cronómetro interno seguía
// marcando 0,7 ms** — el coste se paga fuera de JavaScript y no se ve desde dentro.
// Cualquier efecto visual pasa antes por el banco de fotogramas.

import { useEffect, useRef } from "react";

/** Un rectángulo ya situado en píxeles. La capa no sabe de qué nota viene. */
export interface Rect {
  x: number;
  y: number;
  ancho: number;
  alto: number;
  color: string;
}

/** Una etiqueta ya situada. El texto viene formateado desde fuera. */
export interface Etiqueta {
  x: number;
  y: number;
  texto: string;
  color: string;
}

/** Todo lo que hay que pintar en un fotograma. */
export interface Escena {
  ancho: number;
  alto: number;
  fondo: string;
  teclas: Rect[];
  notas: Rect[];
  etiquetas: Etiqueta[];
}

/** Pinta la escena. Sin decisiones: dibuja lo que le dan, en el orden que le dan. */
export function pintar(ctx: CanvasRenderingContext2D, escena: Escena): void {
  ctx.fillStyle = escena.fondo;
  ctx.fillRect(0, 0, escena.ancho, escena.alto);

  for (const r of escena.teclas) {
    ctx.fillStyle = r.color;
    ctx.fillRect(r.x, r.y, r.ancho, r.alto);
  }
  for (const r of escena.notas) {
    ctx.fillStyle = r.color;
    ctx.fillRect(r.x, r.y, r.ancho, r.alto);
  }
  ctx.textBaseline = "top";
  for (const e of escena.etiquetas) {
    ctx.fillStyle = e.color;
    ctx.fillText(e.texto, e.x, e.y);
  }
}

/** El lienzo, con su bucle de dibujo. La escena la produce quien lo usa. */
export function Lienzo({ escena }: { escena: Escena }) {
  const ref = useRef<HTMLCanvasElement>(null);
  useEffect(() => {
    const ctx = ref.current?.getContext("2d");
    if (ctx) pintar(ctx, escena);
  }, [escena]);
  return <canvas ref={ref} width={escena.ancho} height={escena.alto} />;
}
