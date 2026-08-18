// Los controles que viven FUERA del lienzo.
//
// Esto no es un detalle de organización. El lienzo es el único archivo acogido a la
// excepción del Principio II y por tanto el único sin pruebas; todo lo que deba poder
// afirmarse en una prueba se coloca aquí, donde sí las hay.

/** Proporción respecto al tempo original. **Racional, nunca decimal.** */
export type Velocidad = { num: number; den: number };

/** Las velocidades que se ofrecen. Todas con denominador distinto de cero: el núcleo
 *  rechaza `den == 0`, y un control que pudiera emitirlo mentiría al usuario. */
const VELOCIDADES: { etiqueta: string; v: Velocidad }[] = [
  { etiqueta: "Mitad", v: { num: 1, den: 2 } },
  { etiqueta: "Tres cuartos", v: { num: 3, den: 4 } },
  { etiqueta: "Normal", v: { num: 1, den: 1 } },
];

/** Dos racionales son la misma velocidad aunque no estén reducidos: 2/4 es 1/2.
 *  Comparar los campos por separado dejaría el control apagado sin motivo. */
function mismaVelocidad(a: Velocidad, b: Velocidad): boolean {
  return a.num * b.den === b.num * a.den;
}

export type ControlesProps = {
  /** Punto de corte vigente, en altura MIDI. */
  corte: number;
  /** El archivo traía las voces separadas, así que el corte no se aplica. */
  vocesDelArchivo: boolean;
  onCorte: (corte: number) => void;
  /** La canción está sonando. */
  enMarcha: boolean;
  velocidad: Velocidad;
  onMarcha: () => void;
  onPausa: () => void;
  onVolverAlPrincipio: () => void;
  onVelocidad: (v: Velocidad) => void;
};

export function Controles({
  corte,
  vocesDelArchivo,
  onCorte,
  enMarcha,
  velocidad,
  onMarcha,
  onPausa,
  onVolverAlPrincipio,
  onVelocidad,
}: ControlesProps) {
  return (
    <section className="controles">
      <div className="transporte">
        {/* Un solo botón que cambia de papel. Dos a la vez darían dos caminos al mismo
            sitio y uno estaría siempre desactivado. */}
        <button type="button" onClick={enMarcha ? onPausa : onMarcha}>
          {enMarcha ? "Pausar" : "Reproducir"}
        </button>
        <button type="button" onClick={onVolverAlPrincipio}>
          Volver al principio
        </button>
      </div>

      <div className="velocidad" role="group" aria-label="Velocidad">
        {VELOCIDADES.map(({ etiqueta, v }) => (
          <button
            key={etiqueta}
            type="button"
            data-den={v.den}
            aria-pressed={mismaVelocidad(v, velocidad)}
            onClick={() => onVelocidad(v)}
          >
            {etiqueta}
          </button>
        ))}
      </div>

      <label className="control-corte">
        <span>Corte entre manos</span>
        <input
          type="range"
          min={21}
          max={108}
          value={corte}
          aria-label="Corte entre manos"
          onChange={(e) => onCorte(Number(e.target.value))}
        />
        <output>{corte}</output>
      </label>

      {/* Siempre visible, tanto si se aplica como si no: ocultarlo escondería justo la
          información de qué criterio se está usando. */}
      {vocesDelArchivo && (
        <p className="nota-reparto">
          Usar las voces del archivo: el corte no se aplica.
        </p>
      )}

      {/* FR-006c. La digitación se propone, no se impone. */}
      <p className="nota-digitacion">
        Los dedos son una sugerencia, no una obligación: si te va mejor otro, úsalo.
      </p>
    </section>
  );
}
