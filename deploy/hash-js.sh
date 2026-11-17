#!/usr/bin/env bash
# Le pone al .js principal de /pkg un nombre según su contenido real.
#
# Con `--split`, cargo-leptos calcula ese nombre ANTES de escribirle adentro
# el nombre de __wasm_split.<x>.js, así que dos versiones distintas salen con
# el mismo nombre. Si el navegador lo había guardado, se quedaba con el viejo,
# que pide un __wasm_split que ya no existe, y la página no respondía.
# (El servidor además manda los .js con no-cache: ver crates/server.)
#
# Uso (después de `cargo leptos build --release --split`):
#   deploy/hash-js.sh target/release/hash.txt target/site/pkg [nombre]
set -euo pipefail
HASH_TXT=$1
PKG=$2
NAME=${3:-marketplace}
[ -f "$HASH_TXT" ] || { echo "hash-js: no hay $HASH_TXT, nada que hacer"; exit 0; }
OLD=$(sed -n 's/^js: //p' "$HASH_TXT")
[ -n "$OLD" ] && [ -f "$PKG/$NAME.$OLD.js" ] || { echo "hash-js: no encuentro $PKG/$NAME.$OLD.js" >&2; exit 1; }
NEW=$(sha256sum "$PKG/$NAME.$OLD.js" | cut -c1-22)
[ "$NEW" = "$OLD" ] && exit 0
mv "$PKG/$NAME.$OLD.js" "$PKG/$NAME.$NEW.js"
# __wasm_split.<x>.js importa el .js principal por su nombre.
for f in "$PKG"/__wasm_split.*.js; do
  if [ -f "$f" ]; then sed -i "s/$NAME\.$OLD\.js/$NAME.$NEW.js/g" "$f"; fi
done
sed -i "s/^js: .*/js: $NEW/" "$HASH_TXT"
if grep -rq "$NAME\.$OLD\.js" "$PKG"; then
  echo "hash-js: todavía hay referencias a $NAME.$OLD.js" >&2; exit 1
fi
echo "hash-js: $NAME.$OLD.js -> $NAME.$NEW.js"
