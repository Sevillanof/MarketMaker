#!/usr/bin/env bash
# Copias de seguridad de la base de datos (solo datos: las fotos ya viven en R2).
# Las dispara deploy/release.sh cada minuto (`backup.sh tick`), como el usuario
# marketmaker, así se instalan solas con los deploys, sin tocar systemd.
#
#   backup.sh tick     hace lo que toque hoy (copia diaria, prueba semanal)
#   backup.sh run      hace una copia ahora y la sube
#   backup.sh test     baja la última copia y la restaura en una base de prueba
#   backup.sh list     muestra las copias guardadas
#
# Cada noche (después de las 03:00 de Argentina) saca `pg_dump`, lo comprime,
# lo sube a un bucket de R2 APARTE del de las fotos y deja 7 copias diarias,
# 4 semanales (domingos) y 6 mensuales (día 1). Los domingos, además,
# restaura la última copia en una base descartable para comprobar que sirve.
# Si algo falla, manda un email por Resend a ADMIN_EMAILS (una vez por día).
#
# Configuración en /opt/marketmaker/current/.env.local:
#   BACKUP_R2_BUCKET=marketmaker-backups
#   BACKUP_R2_ACCESS_KEY_ID=...  BACKUP_R2_SECRET_ACCESS_KEY=...
#   (la cuenta sale de R2_ACCOUNT_ID, que ya está)
# Para las pruebas locales: BACKUP_REMOTE_DIR=/carpeta usa una carpeta en vez de R2.
set -uo pipefail

BASE=${MM_BASE:-/opt/marketmaker}
ENV_FILE=${MM_ENV:-$BASE/current/.env.local}
STATE=${BACKUP_STATE:-$BASE/backups}
PREFIX=backups/
KEEP_DAILY=7 KEEP_WEEKLY=4 KEEP_MONTHLY=6
HOUR_FROM=${BACKUP_HOUR:-3}        # hora de Argentina desde la que corre
mkdir -p "$STATE"

if [ -f "$ENV_FILE" ]; then set -a; . "$ENV_FILE"; set +a; fi
# Postgres 14+ ignora estas variables si no existen; DATABASE_URL se pasa tal cual.
DB_URL=${DATABASE_URL:-}
TZ_AR=America/Argentina/Buenos_Aires
today() { TZ=$TZ_AR date +%F; }
hour() { TZ=$TZ_AR date +%-H; }
log() { echo "[backup $(TZ=$TZ_AR date '+%F %T')] $*"; }

# ---- almacenamiento: R2 (curl firma sola con SigV4) o una carpeta de prueba ----
EMPTY=e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855
r2() { # r2 MÉTODO RUTA [curl extra…]
  local m=$1 path=$2; shift 2
  curl -sS --fail-with-body --retry 3 --max-time 300 -X "$m" \
    --aws-sigv4 "aws:amz:auto:s3" --user "$BACKUP_R2_ACCESS_KEY_ID:$BACKUP_R2_SECRET_ACCESS_KEY" \
    "https://$R2_ACCOUNT_ID.r2.cloudflarestorage.com/$BACKUP_R2_BUCKET/$path" "$@"
}
need_config() {
  [ -n "${BACKUP_REMOTE_DIR:-}" ] && return 0
  for v in R2_ACCOUNT_ID BACKUP_R2_BUCKET BACKUP_R2_ACCESS_KEY_ID BACKUP_R2_SECRET_ACCESS_KEY; do
    [ -n "${!v:-}" ] || { echo "falta $v en $ENV_FILE"; return 1; }
  done
  [ -n "$DB_URL" ] || { echo "falta DATABASE_URL en $ENV_FILE"; return 1; }
}
remote_put() { # archivo clave
  if [ -n "${BACKUP_REMOTE_DIR:-}" ]; then mkdir -p "$BACKUP_REMOTE_DIR/$(dirname "$2")" && cp "$1" "$BACKUP_REMOTE_DIR/$2"; return; fi
  r2 PUT "$2" --upload-file "$1" -H "x-amz-content-sha256: $(sha256sum "$1" | cut -d' ' -f1)" -o /dev/null
}
remote_get() { # clave archivo
  if [ -n "${BACKUP_REMOTE_DIR:-}" ]; then cp "$BACKUP_REMOTE_DIR/$1" "$2"; return; fi
  r2 GET "$1" -H "x-amz-content-sha256: $EMPTY" -o "$2"
}
remote_del() {
  if [ -n "${BACKUP_REMOTE_DIR:-}" ]; then rm -f "$BACKUP_REMOTE_DIR/$1"; return; fi
  r2 DELETE "$1" -H "x-amz-content-sha256: $EMPTY" -o /dev/null
}
remote_list() { # imprime las claves, una por línea
  if [ -n "${BACKUP_REMOTE_DIR:-}" ]; then (cd "$BACKUP_REMOTE_DIR" 2>/dev/null && ls -1 "$PREFIX" 2>/dev/null | sed "s|^|$PREFIX|"); return; fi
  local out
  out=$(r2 GET "?list-type=2&prefix=$PREFIX&max-keys=1000" -H "x-amz-content-sha256: $EMPTY") || return 1
  grep -o '<Key>[^<]*</Key>' <<<"$out" | sed 's|</\?Key>||g'
}

# ---- aviso por email si algo falla ----
alert() { # asunto detalle
  local stamp="$STATE/alerta-$(today)"
  [ -e "$stamp" ] && return 0
  touch "$stamp"; find "$STATE" -name 'alerta-*' -mtime +7 -delete 2>/dev/null
  local to=${ADMIN_EMAILS:-}; to=${to%%[,; ]*}
  if [ -z "${RESEND_API_KEY:-}" ] || [ -z "$to" ]; then log "no hay a quién avisar (RESEND_API_KEY / ADMIN_EMAILS)"; return 0; fi
  python3 - "$to" "${EMAIL_FROM:-MarketMaker <hola@mkr.com.ar>}" "$1" "$2" <<'PY' |
import json, sys
to, frm, subj, detail = sys.argv[1:5]
print(json.dumps({"from": frm, "to": [to], "subject": subj,
  "text": detail + "\n\nVer el detalle en la VPS: journalctl -u marketmaker-release -n 100\n"}))
PY
    curl -sS -o /dev/null --max-time 30 -X POST https://api.resend.com/emails \
      -H "Authorization: Bearer $RESEND_API_KEY" -H 'Content-Type: application/json' -d @- \
      || log "no se pudo mandar el aviso"
}

# ---- copia ----
do_backup() {
  need_config || return 1
  local day file key tmp
  day=$(today); tmp=$(mktemp -d "$STATE/.tmp-XXXXXX"); file="$tmp/$day.dump"
  # -Fc ya comprime (zlib). Es consistente: pg_dump usa una sola transacción.
  pg_dump "$DB_URL" -Fc --no-owner --no-privileges -f "$file" || { rm -rf "$tmp"; echo "falló pg_dump"; return 1; }
  pg_restore -l "$file" >/dev/null || { rm -rf "$tmp"; echo "la copia salió dañada"; return 1; }
  key="$PREFIX$day.dump"
  remote_put "$file" "$key" || { rm -rf "$tmp"; echo "no se pudo subir $key"; return 1; }
  local size; size=$(stat -c %s "$file"); rm -rf "$tmp"
  echo "$day $size" > "$STATE/ultima-copia"
  log "copia $key subida: $size bytes."
  prune || log "no se pudo limpiar copias viejas (se reintenta mañana)"
}

# Deja: las 7 más nuevas, los 4 últimos domingos, los 6 últimos días 1.
prune() {
  local keys k d keep n
  keys=$(remote_list | grep -E "^${PREFIX}[0-9]{4}-[0-9]{2}-[0-9]{2}\.dump$" | sort -r) || return 1
  [ -n "$keys" ] || return 0
  keep=$(
    {
      head -n $KEEP_DAILY <<<"$keys"
      n=0; while read -r k; do d=${k#"$PREFIX"}; d=${d%.dump}
        [ "$(date -d "$d" +%u)" = 7 ] && { echo "$k"; n=$((n+1)); [ $n -ge $KEEP_WEEKLY ] && break; }
      done <<<"$keys"
      n=0; while read -r k; do d=${k#"$PREFIX"}; d=${d%.dump}
        [ "${d:8:2}" = 01 ] && { echo "$k"; n=$((n+1)); [ $n -ge $KEEP_MONTHLY ] && break; }
      done <<<"$keys"
    } | sort -u)
  while read -r k; do
    grep -qxF "$k" <<<"$keep" || { log "borro $k"; remote_del "$k"; }
  done <<<"$keys"
}

# ---- prueba de restauración ----
do_test() {
  need_config || return 1
  local newest tmp name rc=0
  newest=$(remote_list | grep -E "\.dump$" | sort | tail -n 1)
  [ -n "$newest" ] || { echo "no hay copias para probar"; return 1; }
  tmp=$(mktemp -d "$STATE/.tmp-XXXXXX")
  remote_get "$newest" "$tmp/c.dump" || { rm -rf "$tmp"; echo "no se pudo bajar $newest"; return 1; }
  pg_restore -l "$tmp/c.dump" >/dev/null || { rm -rf "$tmp"; echo "$newest está dañada"; return 1; }
  # La base de prueba se crea en el mismo Postgres, desde la conexión de la app.
  local admin="${DB_URL%/*}/postgres"
  name="restore_test_$(date +%s)"
  if psql "$admin" -qAtc "create database $name" 2>/dev/null; then
    local url="${DB_URL%/*}/$name" a b
    pg_restore --no-owner --no-privileges -d "$url" "$tmp/c.dump" 2>"$tmp/err" || true
    # Tablas esperadas: las mismas que hay hoy en la base real.
    a=$(psql "$DB_URL" -qAtc "select count(*) from information_schema.tables where table_schema='public'")
    b=$(psql "$url" -qAtc "select count(*) from information_schema.tables where table_schema='public'")
    if [ -z "$b" ] || [ "$a" != "$b" ]; then echo "la restauración de $newest dejó $b tablas de $a: $(head -c 500 "$tmp/err")"; rc=1
    else log "restauración de prueba OK: $newest ($b tablas)."; fi
    psql "$admin" -qAtc "drop database $name" >/dev/null 2>&1
  else
    # Sin permiso para crear bases: se comprueba al menos que el archivo se lee entero.
    log "sin permiso CREATEDB: solo se verificó el archivo de $newest."
  fi
  rm -rf "$tmp"; return $rc
}

# Corre una tarea y, si falla, avisa y la reintenta como mucho cada hora.
guarded() { # nombre estampa-de-éxito función
  local name=$1 ok=$2 fn=$3 out
  out=$($fn 2>&1) && { echo "$out"; echo "$(today)" > "$STATE/$ok"; return 0; }
  echo "$out"
  log "FALLÓ $name"
  alert "MarketMaker: falló $name" "$out"
  touch "$STATE/intento-$name"
  return 1
}
due() { # nombre estampa-de-éxito
  [ "$(cat "$STATE/$2" 2>/dev/null)" = "$(today)" ] && return 1
  [ -e "$STATE/intento-$1" ] && [ -z "$(find "$STATE/intento-$1" -mmin +60)" ] && return 1
  return 0
}

case "${1:-tick}" in
  tick)
    [ "$(hour)" -ge "$HOUR_FROM" ] || exit 0
    exec 8>"$STATE/.lock"; flock -n 8 || exit 0
    if due copia ok-copia; then guarded copia ok-copia do_backup; fi
    if [ "$(TZ=$TZ_AR date +%u)" = 7 ] && [ "$(cat "$STATE/ok-copia" 2>/dev/null)" = "$(today)" ] && due prueba ok-prueba; then
      guarded prueba ok-prueba do_test
    fi
    ;;
  run)  exec 8>"$STATE/.lock"; flock -w 600 8; do_backup ;;
  test) exec 8>"$STATE/.lock"; flock -w 600 8; do_test ;;
  list) need_config && remote_list ;;
  *) sed -n '2,10p' "$0"; exit 1 ;;
esac
