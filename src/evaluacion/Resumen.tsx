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
};

function ms(us: number): number {
  return Math.round(Math.abs(us) / 1000);
}

export function Resumen({ resultado: r }: { resultado: ResultadoPlano }) {
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
    </section>
  );
}
