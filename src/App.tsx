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


  /**
   * Ejecuta una llamada al puente y **muestra el motivo si falla**.
   *
   * Existe porque ninguna lo hacía: cada mando era un `await` suelto, así que un error del
   * núcleo quedaba en un rechazo sin capturar y la aplicación parecía rota sin decir de qué.
   * Lo descubrí ejecutándola: pulsar «abrir» no hacía absolutamente nada.
   */
  const intentar = useCallback(async (accion: () => Promise<void>) => {
    try {
      await accion();
    } catch (motivo) {
      setError(String(motivo));
    }
  }, []);

  /**
   * Hasta dónde llegan las notas ya pedidas.
   *
   * Se pide **por delante** de la posición y se vuelve a pedir cuando la reproducción se
   * acerca al borde, no en cada fotograma: el ancla existe precisamente para que el cursor
   * no cruce el puente sesenta veces por segundo, y pedir las notas ahí tiraría eso por
   * tierra.
   */
  const pedidoHasta = useRef(0);
  /**
   * Testigo de la última petición lanzada.
   *
   * Dos peticiones de notas pueden convivir —al saltar al principio, el bucle de dibujo
   * sigue vivo con la posición vieja hasta que el efecto se rehace—, y **nada garantiza el
   * orden en que resuelven**. Sin este testigo, la vieja llegaba la última, pisaba las
   * notas buenas y dejaba `pedidoHasta` en un punto lejano: ya nunca se volvía a pedir
   * cerca de cero, así que tras «volver al principio» no se veía ninguna nota. Silencioso
   * y permanente, que es la peor combinación.
   */
  const ultimaPeticion = useRef(0);

  /** Cuánta canción se pide de una vez. Dos ventanas: una para ver y otra de reserva. */
  const TRAMO_US = VENTANA_US * 2;

  const refrescar = useCallback(
    async (desde = 0) => {
      // Un poco antes de la posición, para no perder una nota larga que empezó justo antes.
      const inicio = Math.max(0, desde - VENTANA_US);
      const fin = desde + TRAMO_US;
      ultimaPeticion.current += 1;
      const mia = ultimaPeticion.current;
      const notas = await vistaActual(inicio, fin);
      // Si mientras tanto se pidió otra cosa, esta respuesta ya no vale. Aplicarla sería
      // pintar el pasado.
      if (mia !== ultimaPeticion.current) return;
      setNotas(notas);
      pedidoHasta.current = fin;
    },
    [TRAMO_US],
  );

  // El bucle de dibujo. La posición sale **del reloj, nunca del número de fotograma**: la
  // cadencia de la pantalla afecta a la suavidad, no a la corrección. Un contador de
  // fotogramas se desincronizaría en cuanto el navegador saltase uno.
  useEffect(() => {
    if (ancla === null || ancla.num === 0) return undefined;
    let id = 0;
    const fotograma = () => {
      const p = posicionEn(ancla, ahoraLocal());
      setPosicion(p);
      // Cuando lo que queda por delante baja de una ventana, se pide el tramo siguiente.
      // Sin esto, las notas dejaban de verse en cuanto la reproducción pasaba de los
      // primeros segundos: se pedían una vez, al abrir, y nunca más.
      if (p + VENTANA_US > pedidoHasta.current) {
        // `pedidoHasta` se adelanta ya, no al resolver: sin eso el bucle dispararía una
        // petición por fotograma hasta que llegara la primera respuesta.
        pedidoHasta.current = p + TRAMO_US;
        void refrescar(p);
      }
      id = requestAnimationFrame(fotograma);
    };
    id = requestAnimationFrame(fotograma);
    return () => cancelAnimationFrame(id);
  }, [ancla, TRAMO_US, refrescar]);

  const abrir = useCallback(async () => {
    // El diálogo va DENTRO del try. Estaba fuera, y por eso un fallo suyo —el permiso
    // `dialog:default` que faltaba en las capacidades de Tauri— dejaba un rechazo sin
    // capturar: al pulsar «abrir» no pasaba absolutamente nada. Ni archivo, ni error, ni
    // pista de dónde mirar. Un fallo silencioso es peor que uno ruidoso.
    try {
      const ruta = await elegirArchivo();
      // Cancelar es lo normal, no un error: ni se abre nada ni se avisa de nada.
      if (ruta === null) return;
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
      pedidoHasta.current = 0;
      await refrescar(0);
    } catch (motivo) {
      // FR-004. Se muestra el motivo **tal cual**, sin sustituirlo por un mensaje genérico:
      // quien falló sabe mejor qué pasó. Y no se toca nada más, así que la aplicación sigue
      // en pie con lo que ya tuviera.
      setError(String(motivo));
    }
  }, [refrescar]);

  const moverCorte = useCallback(
    async (nuevo: number) => {
      setCorte(nuevo);
      await intentar(async () => {
        await ajustarCorte(nuevo);
        await refrescar(posicion);
      });
    },
    [intentar, posicion, refrescar],
  );

  const poner = useCallback(async () => {
    await intentar(async () => {
      recibirAncla(await marcha());
      setEnMarcha(true);
      // Empezar de nuevo retira el resumen anterior: es de otra interpretación.
      setResultado(null);
      setComparacion(undefined);
    });
  }, [intentar, recibirAncla]);

  const parar = useCallback(async () => {
    await intentar(async () => {
      recibirAncla(await pausa());
      setEnMarcha(false);
      // Pausar cierra la interpretación, así que ya hay resumen que enseñar (FR-014a).
      setResultado(await ultimoResultado());
      setComparacion((await compararConAnterior()) ?? undefined);
    });
  }, [intentar, recibirAncla]);

  const alPrincipio = useCallback(async () => {
    await intentar(async () => {
      recibirAncla(await saltarA(0));
      setPosicion(0);
      await refrescar(0);
    });
  }, [intentar, recibirAncla, refrescar]);

  const ponerVelocidad = useCallback(
    async (v: Velocidad) => {
      await intentar(async () => {
        recibirAncla(await cambiarVelocidad(v));
        setVelocidad(v);
      });
    },
    [intentar, recibirAncla],
  );

  const ponerModo = useCallback(
    async (m: Modo) => {
      await intentar(async () => {
        recibirAncla(await cambiarModo(m));
        setModo(m);
      });
    },
    [intentar, recibirAncla],
  );

  const ponerMano = useCallback(
    async (m: "izquierda" | "derecha" | null) => {
      await intentar(async () => {
        recibirAncla(await practicarMano(m));
        setMano(m ?? "ambas");
      });
    },
    [intentar, recibirAncla],
  );

  const ponerNivel = useCallback(
    async (n: NivelElegido) => {
      await intentar(async () => {
        await cambiarNivel(n);
        setNivel(n);
      });
    },
    [intentar],
  );

  const saltarLaNota = useCallback(async () => {
    await intentar(async () => recibirAncla(await saltarPuerta()));
  }, [intentar, recibirAncla]);

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
      {/* Distinto de lo anterior: éste nunca llegó a estar conectado. El motivo lo da el
          sistema y trae su código cuando lo hay, que es lo único con lo que se puede buscar
          qué pasa de verdad. */}
      {canal.falloAlAbrir !== null && (
        <p role="status" className="aviso">
          No se pudo abrir el teclado: {canal.falloAlAbrir}
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
