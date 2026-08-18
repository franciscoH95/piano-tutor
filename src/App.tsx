// La aplicación. **Sí se prueba**: decide qué archivo abrir, qué mostrar ante un error y
// qué pedirle al núcleo, y eso son decisiones. La excepción del Principio II cubre solo
// `Lienzo.tsx`, que se limita a pintar lo que le dan.

import { useCallback, useState } from "react";
import "./App.css";
import { Controles } from "./practica/controles";
import { Lienzo } from "./practica/Lienzo";
import { construirEscena, VENTANA_US } from "./practica/escena";
import {
  abrirCancion,
  ajustarCorte,
  elegirArchivo,
  vistaActual,
  type NotaVisiblePlana,
  type ResumenCancion,
} from "./practica/puente";

const CORTE_POR_DEFECTO = 60;

export default function App() {
  const [resumen, setResumen] = useState<ResumenCancion | null>(null);
  const [notas, setNotas] = useState<NotaVisiblePlana[]>([]);
  const [corte, setCorte] = useState(CORTE_POR_DEFECTO);
  const [error, setError] = useState<string | null>(null);

  const refrescar = useCallback(async () => {
    setNotas(await vistaActual(0, VENTANA_US));
  }, []);

  const abrir = useCallback(async () => {
    const ruta = await elegirArchivo();
    // Cancelar es lo normal, no un error: ni se abre nada ni se avisa de nada.
    if (ruta === null) return;
    try {
      const nuevo = await abrirCancion(ruta);
      setResumen(nuevo);
      setCorte(nuevo.corte);
      // El aviso viejo se retira: no se acumulan errores de intentos anteriores.
      setError(null);
      await refrescar();
    } catch (motivo) {
      // FR-004. Se muestra el motivo **tal cual lo da el núcleo**, sin sustituirlo por un
      // mensaje genérico: el núcleo sabe mejor qué ha pasado. Y no se toca nada más, así
      // que la aplicación sigue en pie con lo que ya tuviera.
      setError(String(motivo));
    }
  }, [refrescar]);

  const moverCorte = useCallback(
    async (nuevo: number) => {
      setCorte(nuevo);
      await ajustarCorte(nuevo);
      await refrescar();
    },
    [refrescar],
  );

  return (
    <main className="practica">
      <h1>Piano Tutor</h1>

      <button type="button" onClick={abrir}>
        Abrir una canción
      </button>

      {error !== null && (
        <p role="alert" className="error">
          {error}
        </p>
      )}

      {resumen === null ? (
        <p>Abre una canción para empezar a practicar.</p>
      ) : (
        <p>
          {resumen.notas} notas, {Math.round(resumen.duracionUs / 1_000_000)} s
        </p>
      )}

      <Lienzo escena={construirEscena(notas)} />

      {/* Siempre visibles, haya canción o no y haya fallado la carga o no: es lo que
          mantiene la aplicación utilizable después de un error (FR-004). */}
      <Controles
        corte={corte}
        vocesDelArchivo={resumen?.vocesDelArchivo ?? false}
        onCorte={moverCorte}
      />
    </main>
  );
}
