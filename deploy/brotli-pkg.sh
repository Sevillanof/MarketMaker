#!/usr/bin/env bash
# Deja un `.br` (brotli, máxima compresión) al lado de cada .wasm/.js/.css de
# /pkg. El servidor lo entrega a los navegadores que lo aceptan (casi todos);
# Caddy sigue comprimiendo con zstd/gzip lo que no tenga `.br`.
#
# Va después de deploy/hash-js.sh (los nombres ya tienen su versión):
#   deploy/brotli-pkg.sh target/site/pkg
set -euo pipefail
PKG=${1:?uso: brotli-pkg.sh <carpeta pkg>}
if ! command -v brotli >/dev/null; then
  echo "Aviso: no está brotli instalado; /pkg se sirve solo con gzip/zstd (apt-get install -y brotli)." >&2
  exit 0
fi
find "$PKG" -type f \( -name '*.wasm' -o -name '*.js' -o -name '*.css' \) -print0 \
  | while IFS= read -r -d '' f; do brotli -q 11 -f -o "$f.br" "$f"; done
ls -l "$PKG" | grep '\.br$' || true
