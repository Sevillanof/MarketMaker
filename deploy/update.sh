#!/usr/bin/env bash
# Baja la última versión de main, compila y reinicia MarketMaker.
# Es el camino lento (compila en la VPS): lo normal es deploy/release.sh, que
# baja la versión ya compilada por GitHub. Este queda de respaldo.
# Se corre como el usuario marketmaker: sudo -iu marketmaker /opt/marketmaker/src/deploy/update.sh
set -euo pipefail
SRC=/opt/marketmaker/src
OUT=/opt/marketmaker/current
[ -d "$SRC" ] || git clone https://github.com/Sevillanof/MarketMaker "$SRC"
cd "$SRC" && git fetch -q origin main && git checkout -q main && git reset -q --hard origin/main

# Rust y cargo-leptos (solo la primera vez).
if ! command -v cargo >/dev/null; then
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal
fi
. "$HOME/.cargo/env"
rustup target add wasm32-unknown-unknown
command -v cargo-leptos >/dev/null || cargo install --locked cargo-leptos

cargo leptos build --release --split
deploy/hash-js.sh target/release/hash.txt target/site/pkg
deploy/brotli-pkg.sh target/site/pkg
# Si estaba andando una versión de release.sh, se reemplaza por esta (y
# release.sh la guarda como "local" la próxima vez, para poder volver).
rm -f "$OUT/release"
install -m 755 target/release/server "$OUT/server.new" && mv "$OUT/server.new" "$OUT/server"
# Las versiones de los archivos de /pkg (ver hash-files en Cargo.toml).
if [ -f target/release/hash.txt ]; then install -m 644 target/release/hash.txt "$OUT/hash.txt"; else rm -f "$OUT/hash.txt"; fi
rm -rf "$OUT/site.new" && cp -r target/site "$OUT/site.new" && rm -rf "$OUT/site" && mv "$OUT/site.new" "$OUT/site"
sudo systemctl restart marketmaker
# Los logos SVG con texto necesitan alguna fuente instalada.
fc-list 2>/dev/null | grep -q . || ls /usr/share/fonts/truetype >/dev/null 2>&1 \
  || echo "Aviso: no hay fuentes instaladas; como root: apt-get install -y fonts-dejavu-core"
echo "MarketMaker actualizado ($(git rev-parse --short HEAD))."
