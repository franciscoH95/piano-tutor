// La aplicación. **Sí se prueba**: decide qué archivo abrir, qué mostrar ante un error y
// qué pedirle al núcleo, y eso son decisiones. La excepción del Principio II cubre solo
// `Lienzo.tsx`, que se limita a pintar lo que le dan.

import { useCallback, useEffect, useRef, useState } from "react";
import "./App.css";
import { Selector, type DispositivoPlano } from "./dispositivos/Selector";
import {
  Resumen,
  type Comparacion,
  type NivelElegido,
  type ResultadoPlano,
} from "./evaluacion/Resumen";
import { Controles } from "./practica/controles";
import { Lienzo } from "./practica/Lienzo";
import { construirEscena, VENTANA_US } from "./practica/escena";
import {
  anclarEnRelojLocal,
  aplicar,
  ESTADO_INICIAL,
  posicionEn,
  type Ancla,
} from "./practica/modelo";
import {
  abrirCancion,
  ajustarCorte,
  cambiarModo,
  cambiarNivel,
  compararConAnterior,
  cambiarVelocidad,
  conectarTeclado,
  elegirArchivo,
  elegirTeclado,
  escucharCanal,
  marcha,
  pausa,
  practicarMano,
  saltarA,
  saltarPuerta,
  ultimoResultado,
  vistaActual,
  type AnclaDelNucleo,
  type EstadoDelTeclado,
  type NotaVisiblePlana,
  type ResumenCancion,
} from "./practica/puente";
import type { ManoElegida, Modo, Velocidad } from "./practica/controles";

const CORTE_POR_DEFECTO = 60;
const VELOCIDAD_NORMAL: Velocidad = { num: 1, den: 1 };

export default function App() {
  const [resumen, setResumen] = useState<ResumenCancion | null>(null);
  const [notas, setNotas] = useState<NotaVisiblePlana[]>([]);
  const [corte, setCorte] = useState(CORTE_POR_DEFECTO);
  const [enMarcha, setEnMarcha] = useState(false);
  const [velocidad, setVelocidad] = useState<Velocidad>(VELOCIDAD_NORMAL);
  const [error, setError] = useState<string | null>(null);
  const [ancla, setAncla] = useState<Ancla | null>(null);
  const [posicion, setPosicion] = useState(0);
  const [canal, setCanal] = useState(ESTADO_INICIAL);
  const [teclado, setTeclado] = useState<EstadoDelTeclado | undefined>(undefined);
  const [modo, setModo] = useState<Modo>("porReloj");
  const [resultado, setResultado] = useState<ResultadoPlano | null>(null);
  const [nivel, setNivel] = useState<NivelElegido>("intermedio");
  const [comparacion, setComparacion] = useState<Comparacion | undefined>(undefined);
  const [mano, setMano] = useState<ManoElegida>("ambas");
  const notasRef = useRef(notas);
  notasRef.current = notas;

  /** El reloj de la pantalla, en microsegundos. */
  const ahoraLocal = () => performance.now() * 1000;

  /**
   * Reancla lo que llega del núcleo en el reloj local.
   *
   * El ancla trae el instante del reloj de sesión de Rust, con otro cero. Compararlo con
   * `performance.now()` daría una posición disparatada, y sería un desfase silencioso.
   */
  const recibirAncla = useCallback((a: AnclaDelNucleo | null) => {
    if (a === null) return;
    setAncla(anclarEnRelojLocal(a, ahoraLocal()));
  }, []);

  // El canal y el teclado se piden una sola vez, al montar.
  useEffect(() => {
    void escucharCanal((m) => {
      setCanal((previo) => aplicar(previo, m, performance.now() * 1000));
    });
    void conectarTeclado().then(setTeclado);
  }, []);

  // Un ancla que llega por el canal manda igual que la que devuelve un mando: la emite el
  // núcleo por su cuenta al llegar a una puerta, y sin esto el cursor se congelaría sin que
  // la pantalla se enterase.
  useEffect(() => {
    if (canal.ancla !== null) setAncla(canal.ancla);
  }, [canal.ancla]);

  // El bucle de dibujo. La posición sale **del reloj, nunca del número de fotograma**: la
  // cadencia de la pantalla afecta a la suavidad, no a la corrección. Un contador de
  // fotogramas se desincronizaría en cuanto el navegador saltase uno.
  useEffect(() => {
    if (ancla === null || ancla.num === 0) return undefined;
    let id = 0;
    const fotograma = () => {
      setPosicion(posicionEn(ancla, ahoraLocal()));
      id = requestAnimationFrame(fotograma);
    };
    id = requestAnimationFrame(fotograma);
    return () => cancelAnimationFrame(id);
  }, [ancla]);

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
      setAncla(null);
      setPosicion(0);
      setResultado(null);
      setModo("porReloj");
      setMano("ambas");
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
    recibirAncla(await marcha());
    setEnMarcha(true);
    // Empezar de nuevo retira el resumen anterior: es de otra interpretación.
    setResultado(null);
    setComparacion(undefined);
  }, [recibirAncla]);

  const parar = useCallback(async () => {
    recibirAncla(await pausa());
    setEnMarcha(false);
    // Pausar cierra la interpretación, así que ya hay resumen que enseñar (FR-014a).
    setResultado(await ultimoResultado());
    setComparacion((await compararConAnterior()) ?? undefined);
  }, [recibirAncla]);

  const alPrincipio = useCallback(async () => {
    recibirAncla(await saltarA(0));
    setPosicion(0);
    await refrescar();
  }, [recibirAncla, refrescar]);

  const ponerVelocidad = useCallback(
    async (v: Velocidad) => {
      recibirAncla(await cambiarVelocidad(v));
      setVelocidad(v);
    },
    [recibirAncla],
  );

  const ponerModo = useCallback(async (m: Modo) => {
    recibirAncla(await cambiarModo(m));
    setModo(m);
  }, [recibirAncla]);

  const ponerMano = useCallback(
    async (m: "izquierda" | "derecha" | null) => {
      recibirAncla(await practicarMano(m));
      setMano(m ?? "ambas");
    },
    [recibirAncla],
  );

  const ponerNivel = useCallback(async (n: NivelElegido) => {
    await cambiarNivel(n);
    setNivel(n);
  }, []);

  const saltarLaNota = useCallback(async () => {
    recibirAncla(await saltarPuerta());
  }, [recibirAncla]);

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

      {/* FR-015 y FR-016: se comunican, y no bloquean nada. Ni la canción ni los mandos
          dependen de que haya teclado. */}
      {teclado?.tipo === "sinDispositivos" && (
        <p role="status" className="aviso">
          No se detecta ningún teclado MIDI. Puedes ver y reproducir la canción igual.
        </p>
      )}

      {/* FR-025: el recordado no está, o no había ninguno. Se elige, no se propone uno. */}
      {teclado?.tipo === "hayQueElegir" && (
        <Selector
          dispositivos={teclado.dispositivos}
          recordadoAusente
          onElegir={(d: DispositivoPlano) => {
            void elegirTeclado(d).then(setTeclado);
          }}
        />
      )}
      {canal.dispositivoPerdido && (
        <p role="status" className="aviso">
          Se perdió la conexión con el teclado. La canción sigue.
        </p>
      )}

      {resumen === null ? (
        <p>Abre una canción para empezar a practicar.</p>
      ) : (
        <p>
          {resumen.notas} notas, {Math.round(resumen.duracionUs / 1_000_000)} s
        </p>
      )}

      <Lienzo escena={construirEscena(notas, posicion, canal.pulsadas, canal.esperando)} />

      {/* Siempre visibles, haya canción o no y haya fallado la carga o no: es lo que
          mantiene la aplicación utilizable después de un error (FR-004). */}
      {resultado !== null && (
        <Resumen
          resultado={resultado}
          comparacion={comparacion}
          nivel={nivel}
          onNivel={ponerNivel}
        />
      )}

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
        modo={modo}
        mano={mano}
        onModo={ponerModo}
        onMano={ponerMano}
        onSaltarPuerta={saltarLaNota}
      />
    </main>
  );
}
