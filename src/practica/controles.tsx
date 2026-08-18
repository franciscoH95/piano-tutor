// Los controles que viven FUERA del lienzo.
//
// Esto no es un detalle de organización. El lienzo es el único archivo acogido a la
// excepción del Principio II y por tanto el único sin pruebas; todo lo que deba poder
// afirmarse en una prueba se coloca aquí, donde sí las hay.

export type ControlesProps = {
  /** Punto de corte vigente, en altura MIDI. */
  corte: number;
  /** El archivo traía las voces separadas, así que el corte no se aplica. */
  vocesDelArchivo: boolean;
  onCorte: (corte: number) => void;
};

export function Controles({ corte, vocesDelArchivo, onCorte }: ControlesProps) {
  return (
    <section className="controles">
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
