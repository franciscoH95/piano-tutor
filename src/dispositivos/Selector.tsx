// Elegir teclado. La pantalla que la feature 002 aplazó a propósito.

/** Un teclado disponible, tal como lo enumera el sistema. */
export type DispositivoPlano = {
  nombre: string;
  /** Posición entre los homónimos. Es lo que distingue dos teclados del mismo modelo. */
  posicion: number;
  idSistema: number | null;
};

export type SelectorProps = {
  dispositivos: DispositivoPlano[];
  onElegir: (d: DispositivoPlano) => void;
  /** El teclado recordado no está entre los disponibles (FR-025). */
  recordadoAusente?: boolean;
};

export function Selector({ dispositivos, onElegir, recordadoAusente = false }: SelectorProps) {
  if (dispositivos.length === 0) {
    return (
      <section className="selector">
        <p>No se detecta ningún teclado MIDI. Conecta uno y vuelve a intentarlo.</p>
      </section>
    );
  }

  return (
    <section className="selector">
      <h2>Elige tu teclado</h2>

      {/* FR-025: si el recordado no está, se pide elegir de nuevo y NO se abre otro. */}
      {recordadoAusente && (
        <p role="status">
          El teclado que usabas no está conectado. Elige de nuevo cuál quieres usar.
        </p>
      )}

      <ul>
        {dispositivos.map((d) => (
          <li key={`${d.nombre}-${d.posicion}`}>
            {/* Ninguno viene preseleccionado: un preseleccionado invita a aceptar sin
                mirar, que es justo el error que la posición existe para evitar. */}
            <button type="button" aria-pressed="false" onClick={() => onElegir(d)}>
              {d.nombre} <span className="posicion">({d.posicion + 1})</span>
            </button>
          </li>
        ))}
      </ul>
    </section>
  );
}
