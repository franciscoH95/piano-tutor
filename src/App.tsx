// La aplicación. **Sí se prueba**: decide qué archivo abrir y qué mostrar ante un error,
// y eso son decisiones. La excepción del Principio II cubre solo `Lienzo.tsx`.

import { useState } from "react";
import "./App.css";

export default function App() {
  const [cancion, setCancion] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  return (
    <main className="practica">
      <h1>Piano Tutor</h1>
      {error !== null && (
        <p role="alert" className="error">
          {error}
        </p>
      )}
      {cancion === null ? (
        <p>Abre una canción para empezar a practicar.</p>
      ) : (
        <p>{cancion}</p>
      )}
      {/* El selector de archivos y el lienzo llegan en la fase 3 (T041, T042). */}
      <button type="button" onClick={() => { setCancion(null); setError(null); }}>
        Reiniciar
      </button>
    </main>
  );
}
