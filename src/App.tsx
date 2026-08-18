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
  cambiarVelocidad,
  elegirArchivo,
  marcha,
  pausa,
  saltarA,
  vistaActual,
  type NotaVisiblePlana,
  type ResumenCancion,
} from "./practica/puente";
import type { Velocidad } from "./practica/controles";

const CORTE_POR_DEFECTO = 60;
const VELOCIDAD_NORMAL: Velocidad = { num: 1, den: 1 };

export default function App() {
  const [resumen, setResumen] = useState<ResumenCancion | null>(null);
  const [notas, setNotas] = useState<NotaVisiblePlana[]>([]);
  const [corte, setCorte] = useState(CORTE_POR_DEFECTO);
  const [enMarcha, setEnMarcha] = useState(false);
  const [velocidad, setVelocidad] = useState<Velocidad>(VELOCIDAD_NORMAL);
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
      // Una canción nueva empieza parada y a tempo: no hereda el transporte de la
      // anterior (FR-005).
      setEnMarcha(false);
      setVelocidad(VELOCIDAD_NORMAL);
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

  const poner = useCallback(async () => {
    await marcha();
    setEnMarcha(true);
  }, []);

  const parar = useCallback(async () => {
    await pausa();
    setEnMarcha(false);
  }, []);

  const alPrincipio = useCallback(async () => {
    await saltarA(0);
    await refrescar();
  }, [refrescar]);

  const ponerVelocidad = useCallback(async (v: Velocidad) => {
    await cambiarVelocidad(v);
    setVelocidad(v);
  }, []);

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
        enMarcha={enMarcha}
        velocidad={velocidad}
        onMarcha={poner}
        onPausa={parar}
        onVolverAlPrincipio={alPrincipio}
        onVelocidad={ponerVelocidad}
      />
    </main>
  );
}
