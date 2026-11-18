#!/usr/bin/env bash
# Instala en la VPS una versión ya compilada por GitHub (ver
# .github/workflows/vps-build.yml), sin compilar nada acá.
# Se corre como el usuario marketmaker:
#
#   release.sh                 instala la última versión de main
#   release.sh deploy abc1234  instala esa versión (commit corto)
#   release.sh rollback        vuelve a la versión anterior
#   release.sh list            muestra las versiones guardadas
#
# Cada versión queda en /opt/marketmaker/releases/<commit>/ y
# /opt/marketmaker/current/release apunta a la que está andando
# (current/server y current/site apuntan ahí). Cambiar de versión es cambiar
# ese enlace y reiniciar. Si la nueva no responde, vuelve sola a la anterior.
# .env.local sigue en /opt/marketmaker/current, como siempre.
set -euo pipefail
REPO=Sevillanof/MarketMaker
BASE=${MM_BASE:-/opt/marketmaker}
REL=$BASE/releases
CUR=$BASE/current
KEEP=5
URL=${MM_URL:-https://github.com/$REPO/releases}

mkdir -p "$REL"
# Una instalación por vez (el timer y una a mano no se pisan).
exec 9>"$REL/.lock"
if [ "${AUTO:-}" = 1 ]; then
  flock -n 9 || exit 0
else
  flock -w 600 9 || { echo "Hay otra instalación en curso hace más de 10 minutos."; exit 1; }
fi
rm -rf "$REL"/.bajando-*
# Copias de seguridad de la base (deploy/backup.sh): se pasa cada minuto con el
# timer y solo hace algo una vez por día. Nunca debe frenar un deploy.
if [ "${AUTO:-}" = 1 ] && [ -x "$(dirname "$0")/backup.sh" ]; then
  timeout 900 "$(dirname "$0")/backup.sh" tick 9>&- || true
fi
current() { [ -L "$CUR/release" ] && basename "$(readlink "$CUR/release")" || true; }

download() {
  local ver=$1 tmp
  [ -x "$REL/$ver/server" ] && return 0
  echo "Bajando $ver…"
  tmp=$(mktemp -d "$REL/.bajando-XXXXXX")
  if curl -fsSL --retry 3 -o "$tmp/marketmaker.tar.gz" "$URL/download/vps-$ver/marketmaker.tar.gz" \
    && curl -fsSL --retry 3 -o "$tmp/marketmaker.tar.gz.sha256" "$URL/download/vps-$ver/marketmaker.tar.gz.sha256" \
    && (cd "$tmp" && sha256sum -c --quiet marketmaker.tar.gz.sha256) \
    && tar -C "$tmp" -xzf "$tmp/marketmaker.tar.gz" \
    && [ -x "$tmp/marketmaker/server" ]; then
    rm -rf "${REL:?}/$ver" && mv "$tmp/marketmaker" "$REL/$ver"
    rm -rf "$tmp"
  else
    rm -rf "$tmp"
    echo "No se pudo bajar la versión $ver (¿terminó de compilar en GitHub?)."
    exit 1
  fi
}

# Reemplaza $CUR/$1 por un enlace a $2, aunque hoy sea un archivo o carpeta
# (lo que deja deploy/update.sh).
link() {
  ln -sfn "$2" "$CUR/$1.nuevo"
  if [ -e "$CUR/$1" ] && [ ! -L "$CUR/$1" ]; then rm -rf "$CUR/$1.viejo"; mv "$CUR/$1" "$CUR/$1.viejo"; fi
  mv -T "$CUR/$1.nuevo" "$CUR/$1"
  rm -rf "$CUR/$1.viejo"
}

healthy() {
  local code
  for _ in $(seq 1 40); do
    code=$(curl -s -o /dev/null -w '%{http_code}' --max-time 3 http://127.0.0.1:3000/ || true)
    if [ "$code" -ge 200 ] 2>/dev/null && [ "$code" -lt 500 ]; then return 0; fi
    sleep 1
  done
  return 1
}

# Pone en marcha la versión $1. Si no responde, vuelve a la que estaba.
activate() {
  local ver=$1 prev
  prev=$(current)
  # Si hoy anda lo que compiló deploy/update.sh, se guarda como versión
  # "local" para poder volver a ella.
  if [ -z "$prev" ] && [ -f "$CUR/server" ] && [ ! -L "$CUR/server" ]; then
    rm -rf "$REL/local" && mkdir -p "$REL/local"
    cp -a "$CUR/server" "$REL/local/server"
    [ -d "$CUR/site" ] && cp -a "$CUR/site" "$REL/local/site"
    [ -f "$CUR/hash.txt" ] && cp -a "$CUR/hash.txt" "$REL/local/hash.txt"
    prev=local
  fi
  link release "$REL/$ver"
  link server release/server
  link site release/site
  # hash.txt se lee junto al servidor real (releases/<commit>/hash.txt).
  rm -f "$CUR/hash.txt"
  [ -n "$prev" ] && [ "$prev" != "$ver" ] && link release.prev "$REL/$prev"
  sudo systemctl restart marketmaker 9>&-
  if healthy; then
    rm -f "$REL/$ver.fallo"
    touch "$REL/$ver"
    echo "MarketMaker andando con la versión $ver."
    return 0
  fi
  touch "$REL/$ver.fallo"
  if [ -n "$prev" ] && [ "$prev" != "$ver" ] && [ -x "$REL/$prev/server" ]; then
    echo "La versión $ver no respondió. Vuelvo a $prev."
    link release "$REL/$prev"
    sudo systemctl restart marketmaker 9>&-
    healthy && echo "MarketMaker andando otra vez con $prev." || echo "Tampoco responde $prev: mirar journalctl -u marketmaker."
  else
    echo "La versión $ver no respondió: mirar journalctl -u marketmaker."
  fi
  exit 1
}

# Borra las versiones viejas; nunca la actual ni la anterior.
prune() {
  local keep_cur keep_prev
  keep_cur=$(current)
  keep_prev=$([ -L "$CUR/release.prev" ] && basename "$(readlink "$CUR/release.prev")" || true)
  ls -1t "$REL" | grep -v '\.fallo$' | grep -v '^\.' | tail -n +$((KEEP + 1)) | while read -r v; do
    [ "$v" = "$keep_cur" ] || [ "$v" = "$keep_prev" ] || { rm -rf "${REL:?}/$v" "$REL/$v.fallo"; }
  done
}

case "${1:-deploy}" in
  deploy)
    ver=${2:-$(curl -fsSL --retry 3 "$URL/latest/download/version.txt")}
    [[ $ver =~ ^([0-9a-f]{7,40}|local)$ ]] || { echo "Versión rara: '$ver'"; exit 1; }
    if [ "$ver" = "$(current)" ] && [ "$(readlink "$CUR/server" || true)" = release/server ]; then
      [ "${AUTO:-}" = 1 ] || echo "Ya está andando $ver."
      exit 0
    fi
    # Con --auto (el timer) no se reintenta una versión que ya falló.
    if [ "${AUTO:-}" = 1 ] && [ -e "$REL/$ver.fallo" ]; then
      echo "La versión $ver ya falló antes; no se reintenta sola."; exit 0
    fi
    download "$ver"
    activate "$ver"
    prune
    ;;
  rollback)
    [ -L "$CUR/release.prev" ] || { echo "No hay versión anterior guardada."; exit 1; }
    prev=$(basename "$(readlink "$CUR/release.prev")")
    # La que se deja queda marcada: el timer no la vuelve a instalar sola.
    cur=$(current)
    [ -n "$cur" ] && touch "$REL/$cur.fallo"
    activate "$prev"
    ;;
  list)
    cur=$(current)
    ls -1t "$REL" | grep -v '^\.' | while read -r v; do
      case $v in *.fallo) continue ;; esac
      mark=""; [ "$v" = "$cur" ] && mark="  ← andando"
      [ -e "$REL/$v.fallo" ] && mark="$mark  (falló)"
      echo "$v$mark"
    done
    ;;
  *) sed -n '2,12p' "$0"; exit 1 ;;
esac
