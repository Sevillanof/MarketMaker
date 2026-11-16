# Subir MarketMaker a la VPS

Una VPS Ubuntu 24.04. Caddy atiende https (80/443) y le pasa todo a la app,
que escucha solo en `127.0.0.1:3000`. Postgres escucha solo en la máquina.

Mientras no haya dominio propio, la dirección es la IP con guiones en
sslip.io: la plataforma en `https://1-2-3-4.sslip.io` y cada tienda en
`https://ana.1-2-3-4.sslip.io`. Caddy pide el certificado de cada tienda la
primera vez que alguien entra, y solo si existe (`/tls-ok`).

## La primera vez

    sudo bash setup.sh        # desde una copia del repo en la VPS

Después, completar `/opt/marketmaker/current/.env.local` (lo crea `setup.sh`
con `DATABASE_URL`):

    APP_URL=https://1-2-3-4.sslip.io
    ALLOWED_EMAILS=...
    R2_ACCOUNT_ID=... R2_BUCKET=... R2_ACCESS_KEY_ID=... R2_SECRET_ACCESS_KEY=...
    MP_REDIRECT_URL=https://1-2-3-4.sslip.io/mp/vuelta   # con Mercado Pago

## Cada actualización

    sudo -iu marketmaker /opt/marketmaker/src/deploy/update.sh

Baja `main`, compila con `cargo leptos build --release` y reinicia el servicio.
Logs: `journalctl -u marketmaker -f`.
