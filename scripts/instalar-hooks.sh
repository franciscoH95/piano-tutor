#!/usr/bin/env bash
# Instala scripts/verificar.sh como hook `pre-push`.
#
# `.git/hooks/` no se versiona, asi que sin ejecutar esto una vez la puerta no existe
# en una clonacion nueva. Es la limitacion honesta de no tener remoto todavia: quien
# bloquea es un hook local, y un hook local se salta con `git push --no-verify`.
set -euo pipefail
cd "$(dirname "$0")/.."
mkdir -p .git/hooks
cat > .git/hooks/pre-push <<'HOOK'
#!/usr/bin/env bash
exec "$(git rev-parse --show-toplevel)/scripts/verificar.sh"
HOOK
chmod +x .git/hooks/pre-push
echo "Hook pre-push instalado. Se puede saltar con 'git push --no-verify'."
