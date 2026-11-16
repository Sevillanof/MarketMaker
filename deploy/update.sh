#!/usr/bin/env bash
# Baja la última versión de main, compila y reinicia MarketMaker.
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

cargo leptos build --release
install -m 755 target/release/server "$OUT/server.new" && mv "$OUT/server.new" "$OUT/server"
rm -rf "$OUT/site.new" && cp -r target/site "$OUT/site.new" && rm -rf "$OUT/site" && mv "$OUT/site.new" "$OUT/site"
sudo systemctl restart marketmaker
echo "MarketMaker actualizado ($(git rev-parse --short HEAD))."
