/// Pruebas de los componentes de interfaz.
///
/// Existen porque `App.tsx`, `controles.tsx` y `Selector.tsx` **toman decisiones** —qué
/// archivo abrir, qué mostrar ante un error, qué emite cada control— y por tanto no pueden
/// acogerse a la excepción del Principio II, cuya primera condición es no decidir nada.
/// El único archivo exento es `src/practica/Lienzo.tsx`, que solo pinta.
import { defineConfig } from "vitest/config";
import react from "@vitejs/plugin-react";

export default defineConfig({
  plugins: [react()],
  test: {
    environment: "happy-dom",
    globals: true,
    setupFiles: ["./src/pruebas/preparar.ts"],
    include: ["src/**/*.test.{ts,tsx}"],
  },
});
