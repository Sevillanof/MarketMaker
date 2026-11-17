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

    APP_URL=https://mkr.com.ar
    ALLOWED_EMAILS=...
    R2_ACCOUNT_ID=... R2_BUCKET=... R2_ACCESS_KEY_ID=... R2_SECRET_ACCESS_KEY=...
    MP_REDIRECT_URL=https://mkr.com.ar/mp/vincular   # con Mercado Pago
    RESEND_API_KEY=...
    EMAIL_FROM="MarketMaker <hola@mkr.com.ar>"   # con comillas: si no, se deja de leer el archivo ahí

## Cada actualización

GitHub compila la app cada vez que algo entra a `main` (ver
`.github/workflows/vps-build.yml`, tarda unos minutos) y deja la versión
lista en Releases como `vps-<commit>`. La VPS solo la baja y reinicia:

    sudo -iu marketmaker bash -c 'cd src && git pull -q --ff-only && deploy/release.sh'

- `deploy/release.sh` instala la última de `main` (menos de un minuto).
- `deploy/release.sh deploy abc1234` instala esa versión.
- `deploy/release.sh rollback` vuelve a la anterior.
- `deploy/release.sh list` muestra las guardadas y cuál anda.

Cada versión queda en `/opt/marketmaker/releases/<commit>/`; `current/server`
y `current/site` apuntan a la que anda (`current/release`). `.env.local` sigue
en `current/`. Si la versión nueva no responde en 40 segundos, vuelve sola a
la anterior. Se guardan las últimas 5.

Para que se instale sola cada vez que algo entra a `main`, activar el timer
(ver `marketmaker-release.timer`).

### Respaldo: compilar en la VPS

    sudo -iu marketmaker /opt/marketmaker/src/deploy/update.sh

Baja `main`, compila con `cargo leptos build --release` y reinicia el servicio.
Es lento y carga la VPS; sirve si GitHub no compiló. `release.sh` guarda esa
compilación como versión `local` la próxima vez, para poder volver a ella.

Logs: `journalctl -u marketmaker -f`.
