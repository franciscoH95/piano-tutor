# Piano Tutor

Aplicacion de escritorio para aprender canciones al piano con un teclado MIDI.
Local primero, sin conexion, con el repertorio que importa el propio usuario.

## Estructura

| Ruta | Contenido |
| --- | --- |
| `core/` | Nucleo de dominio en Rust: reloj, linea temporal, feedforward y evaluacion. Sin GUI ni hardware. |
| `src-tauri/` | Aplicacion Tauri: ventana, comandos e IPC. Capa fina sobre `core`. |
| `src/` | Interfaz en TypeScript (React + Vite). Solo renderiza estado. |
| `.specify/` | Spec Kit: constitucion, plantillas y specs de cada feature. |

El nucleo vive fuera de `src-tauri` a proposito: la Constitucion (Principio III) exige
que el dominio sea ejecutable y testeable sin ventana y sin teclado conectado.

## Requisitos

- Rust 1.97+
- Node 24+ y pnpm 11+

## Desarrollo

```sh
pnpm install         # dependencias del frontend
pnpm tauri dev       # app de escritorio en modo desarrollo
cargo test           # suite del nucleo (headless, sin hardware)
```

## Gobernanza

Este proyecto se rige por [`.specify/memory/constitution.md`](.specify/memory/constitution.md).
Antes de contribuir, lee los cinco principios: precision musical, TDD estricto,
nucleo desacoplado de la UI, presupuesto de latencia de 30 ms y local primero.

El trabajo sigue el ciclo Spec Kit: `/speckit-specify` -> `/speckit-plan` ->
`/speckit-tasks` -> `/speckit-implement`.
