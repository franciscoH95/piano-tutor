#!/usr/bin/env bash
# Las cuatro puertas del proyecto, en un solo comando y con un solo codigo de salida.
#
# Se ejecuta como hook `pre-push` (ver scripts/instalar-hooks.sh) y desde .github/workflows/ci.yml.
# Aborta en la primera que falle.
set -euo pipefail
cd "$(dirname "$0")/.."

paso() { printf '\n\033[1m=== %s ===\033[0m\n' "$1"; }

paso "1/4  Suite de pruebas"
cargo test --workspace

paso "2/4  Lints"
cargo clippy --workspace --all-targets -- -D warnings

paso "3/4  Principio III: el nucleo no toca el sistema"
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

paso "4/4  Banco de latencia"
cargo run -p piano-bench --release --bin latencia

printf '\n\033[1;32mTodas las puertas en verde.\033[0m\n'
