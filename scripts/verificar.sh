#!/usr/bin/env bash
# Las cuatro puertas del proyecto, en un solo comando y con un solo codigo de salida.
#
# Se ejecuta como hook `pre-push` (ver scripts/instalar-hooks.sh) y desde .github/workflows/ci.yml.
# Aborta en la primera que falle.
set -euo pipefail
cd "$(dirname "$0")/.."

paso() { printf '\n\033[1m=== %s ===\033[0m\n' "$1"; }

paso "1/5  Suite de pruebas de Rust"
cargo test --workspace

paso "2/5  Lints"
cargo clippy --workspace --all-targets -- -D warnings

paso "3/5  Principio III: el nucleo no toca el sistema"
# El nucleo puede depender EXACTAMENTE de esto y de nada mas. Anadir un crate aqui
# rompe la verificacion a proposito: obliga a justificarlo en el PR.
ESPERADO="midi_file piano-core rtrb"
PROHIBIDO='coremidi|midir|windows|winapi|core-foundation|objc2|libc|alsa|jack'
for t in aarch64-apple-darwin x86_64-apple-darwin x86_64-pc-windows-msvc; do
  actual=$(cargo tree -p piano-core --target "$t" --prefix none 2>/dev/null \
           | awk 'NF {print $1}' | sort -u | tr '\n' ' ' | sed 's/ *$//')
  if [ "$actual" != "$ESPERADO" ]; then
    echo "FALLO en $t"
    echo "  esperado: $ESPERADO"
    echo "  real:     $actual"
    exit 1
  fi
  if cargo tree -p piano-core --target "$t" --prefix none 2>/dev/null | grep -qE "$PROHIBIDO"; then
    echo "FALLO en $t: el nucleo arrastra una dependencia de sistema"
    exit 1
  fi
  echo "  ok  $t"
done

paso "4/5  Pruebas de la interfaz"
# Los componentes de React toman decisiones, asi que se prueban. El unico archivo
# exento es src/practica/Lienzo.tsx, que solo pinta.
#
# La instalacion va AQUI y no en el flujo de la CI. Este script es el que ejecutan las dos,
# y si la CI hiciera un paso de mas, verde en local dejaria de significar verde en CI. En
# una maquina que ya las tiene, `--frozen-lockfile` no hace nada y tarda un instante.
pnpm install --frozen-lockfile
pnpm test

paso "5/5  Banco de latencia"
cargo run -p piano-bench --release --bin latencia

printf '\n\033[1;32mTodas las puertas en verde.\033[0m\n'
