// El resumen de una interpretación.
//
// **No está acogido a la excepción del Principio II**: decide qué se enseña primero y cómo
// se redacta, y eso son decisiones. Se prueba como cualquier componente.
//
// No recalcula nada: recibe recuentos ya juzgados. Ninguna tolerancia llega hasta aquí.

/** El resumen tal como lo aplana el puente. */
export type ResultadoPlano = {
  acertadas: number;
  fueraDeTiempo: number;
  omitidas: number;
  deMas: number;
  dedosEscapados: number;
  fueraDeAlcance: number;
  noIntentadas: number;
  /** El denominador honesto: lo que se le pidió de verdad al alumno. */
  intentadas: number;
  /** Con signo: negativo se adelanta, positivo se atrasa. */
  desfaseMedianaUs: number | null;
  desfaseDispersionUs: number | null;
  sinTocar: boolean;
  /** Los tiempos no se evaluaron. Hay que decirlo. */
  parcial: boolean;
  /** Recuento de cada mano (FR-018). */
  porMano: { izquierda: Recuento; derecha: Recuento };
};

/** Lo que le pasó a una mano. */
export type Recuento = {
  acertadas: number;
  fueraDeTiempo: number;
  omitidas: number;
};

/** Si a esta mano se le pidió algo. */
function tuvoNotas(r: Recuento): boolean {
  return r.acertadas + r.fueraDeTiempo + r.omitidas > 0;
}

function ms(us: number): number {
  return Math.round(Math.abs(us) / 1000);
}

/** Cuánta exigencia. */
export type NivelElegido = "permisivo" | "intermedio" | "exigente";

export type ResumenProps = {
  resultado: ResultadoPlano;
  nivel?: NivelElegido;
  /** Sin manejador no se ofrece el selector: hay sitios donde no hay nada que ajustar. */
  onNivel?: (n: NivelElegido) => void;
};

function SelectorDeNivel({
  nivel,
  onNivel,
}: {
  nivel: NivelElegido;
  onNivel: (n: NivelElegido) => void;
}) {
  return (
    <div className="exigencia">
      <label>
        Exigencia
        <select value={nivel} onChange={(e) => onNivel(e.target.value as NivelElegido)}>
          <option value="permisivo">Permisiva</option>
          <option value="intermedio">Intermedia</option>
          <option value="exigente">Exigente</option>
        </select>
      </label>
      {/* Sin decirlo, el alumno cambiaría de nivel esperando que este resumen se
          recalcule. No lo hace: este resultado ya está juzgado. */}
      <p className="aviso-nivel">Se aplica a la próxima vez que toques, no a este resultado.</p>
    </div>
  );
}

export function Resumen({ resultado: r, nivel, onNivel }: ResumenProps) {
  // SC-002: no tocar nada no es tocarlo todo mal. Un 0 % dice «lo hiciste fatal»; esto no.
  if (r.sinTocar) {
    return (
      <section className="resumen">
        <h2>No se tocó ninguna nota</h2>
        <p>Cuando toques, aquí verás cómo te fue.</p>
      </section>
    );
  }

  return (
    <section className="resumen">
      <h2>Cómo te fue</h2>

      <ul className="recuentos">
        <li>
          <strong>{r.acertadas}</strong> acertadas de {r.intentadas}
        </li>
        {r.fueraDeTiempo > 0 && (
          <li>
            <strong>{r.fueraDeTiempo}</strong> tocadas fuera de tiempo
          </li>
        )}
        <li>
          <strong>{r.omitidas}</strong> se te pasaron
        </li>
        <li>
          <strong>{r.deMas}</strong> de más
        </li>
        {r.dedosEscapados > 0 && (
          <li>
            <strong>{r.dedosEscapados}</strong> dedos que se escaparon a la tecla de al lado
          </li>
        )}
      </ul>

      {/* Solo si la pieza tiene las dos: enseñar «derecha: 0 de 0» es ruido que hace
          pensar que algo falló. */}
      {tuvoNotas(r.porMano.izquierda) && tuvoNotas(r.porMano.derecha) && (
        <ul className="por-mano">
          <li>
            Izquierda: {r.porMano.izquierda.acertadas} de{" "}
            {r.porMano.izquierda.acertadas +
              r.porMano.izquierda.fueraDeTiempo +
              r.porMano.izquierda.omitidas}
          </li>
          <li>
            Derecha: {r.porMano.derecha.acertadas} de{" "}
            {r.porMano.derecha.acertadas +
              r.porMano.derecha.fueraDeTiempo +
              r.porMano.derecha.omitidas}
          </li>
        </ul>
      )}

      {/* El signo ES la información: «40 ms» no le dice nada al alumno, «vas 40 ms tarde» sí. */}
      {r.desfaseMedianaUs !== null && (
        <p className="desfase">
          {r.desfaseMedianaUs > 0
            ? `Vas unos ${ms(r.desfaseMedianaUs)} ms tarde de forma constante.`
            : `Entras unos ${ms(r.desfaseMedianaUs)} ms pronto de forma constante.`}
        </p>
      )}

      {/* FR-015a: un resultado incompleto que no se declara incompleto se lee como completo. */}
      {r.parcial && (
        <p className="parcial" role="note">
          Practicaste en modo espera, así que <strong>no se han evaluado los tiempos</strong>:
          la canción te esperaba, y ahí no se puede llegar tarde.
        </p>
      )}

      {/* Fuera del denominador: no son fallos suyos, y decirlo evita que lo parezcan. */}
      {r.fueraDeAlcance > 0 && (
        <p className="aparte">
          {r.fueraDeAlcance} notas quedan fuera del alcance de tu teclado y no cuentan.
        </p>
      )}
      {r.noIntentadas > 0 && (
        <p className="aparte">
          {r.noIntentadas} notas que saltaste tampoco cuentan: no llegaste a intentarlas.
        </p>
      )}

      {onNivel !== undefined && (
        <SelectorDeNivel nivel={nivel ?? "intermedio"} onNivel={onNivel} />
      )}
    </section>
  );
}
