#!/usr/bin/env bash
# Prepara una VPS Ubuntu 24.04 para MarketMaker (se corre una vez, con sudo).
# Instala Postgres (solo escucha en la propia máquina) y Caddy (https), crea
# el usuario `marketmaker` y su base de datos. No toca otros servicios.
set -euo pipefail
[ "$(id -u)" = 0 ] || { echo "Correr con sudo"; exit 1; }

apt-get update
apt-get install -y build-essential pkg-config libssl-dev git curl postgresql \
  debian-keyring debian-archive-keyring apt-transport-https gnupg

# Caddy desde su repositorio oficial.
if ! command -v caddy >/dev/null; then
  curl -1sLf 'https://dl.cloudsmith.io/public/caddy/stable/gpg.key' | gpg --dearmor -o /usr/share/keyrings/caddy-stable-archive-keyring.gpg
  curl -1sLf 'https://dl.cloudsmith.io/public/caddy/stable/debian.deb.txt' > /etc/apt/sources.list.d/caddy-stable.list
  apt-get update && apt-get install -y caddy
fi

id marketmaker >/dev/null 2>&1 || useradd --system --create-home --home-dir /opt/marketmaker --shell /bin/bash marketmaker
mkdir -p /opt/marketmaker/current && chown -R marketmaker:marketmaker /opt/marketmaker
[ -d /opt/marketmaker/src ] || sudo -u marketmaker git clone -q https://github.com/Sevillanof/MarketMaker /opt/marketmaker/src

# Base de datos con contraseña al azar (queda en .env.local, que solo lee marketmaker).
ENV=/opt/marketmaker/current/.env.local
if ! sudo -u postgres psql -tAc "select 1 from pg_roles where rolname='marketmaker'" | grep -q 1; then
  PASS=$(openssl rand -hex 24)
  sudo -u postgres psql -c "create role marketmaker login password '$PASS'"
  sudo -u postgres psql -c "create database marketmaker owner marketmaker"
  echo "DATABASE_URL=postgres://marketmaker:$PASS@127.0.0.1/marketmaker" >> "$ENV"
fi
touch "$ENV" && chown marketmaker:marketmaker "$ENV" && chmod 600 "$ENV"

# marketmaker solo puede reiniciar su propio servicio (para update.sh).
cp /opt/marketmaker/src/deploy/marketmaker.service /etc/systemd/system/marketmaker.service
echo 'marketmaker ALL=(root) NOPASSWD: /usr/bin/systemctl restart marketmaker' > /etc/sudoers.d/marketmaker
chmod 440 /etc/sudoers.d/marketmaker && visudo -cf /etc/sudoers.d/marketmaker
systemctl daemon-reload && systemctl enable marketmaker

# Caddy: se reemplaza su archivo solo si es el de ejemplo (recién instalado).
if grep -q "^:80" /etc/caddy/Caddyfile 2>/dev/null; then
  cp /opt/marketmaker/src/deploy/Caddyfile /etc/caddy/Caddyfile
  systemctl reload caddy
fi

# Firewall: si ufw está activo, abrir solo web (80/443). Si no, no se toca.
if command -v ufw >/dev/null && ufw status | grep -q "Status: active"; then
  ufw allow 80/tcp && ufw allow 443/tcp
fi

echo "Listo. Ahora: completar $ENV y correr deploy/update.sh como marketmaker."
