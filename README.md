# MarketMaker

Creador de marketplaces en Rust. Esta primera etapa arma un marketplace de prueba en local: en `/editor` se arma un panel con widgets sobre una grilla, se guarda, y `/` muestra la última versión guardada renderizada en el servidor.

Stack: Leptos 0.8 (SSR con hidratación), Axum 0.8, SQLx 0.9, PostgreSQL, Tailwind CSS 4 y daisyUI 5.

## Puesta en marcha desde cero (Mac)

### 1. Herramientas

```bash
# Herramientas de línea de comandos de Apple (si no las tenés)
xcode-select --install

# Rust (stable) y el target de WebAssembly
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup target add wasm32-unknown-unknown

# cargo-leptos (compila servidor + WASM y descarga Tailwind solo)
cargo install --locked cargo-leptos

# sqlx-cli (para crear migraciones nuevas; no hace falta para levantar el proyecto)
cargo install sqlx-cli --no-default-features --features native-tls,postgres
```

### 2. PostgreSQL

Con [Homebrew](https://brew.sh):

```bash
brew install postgresql@16
brew services start postgresql@16
createdb marketplace_dev
```

Comprobación: `cargo leptos --version`, `sqlx --version` y `psql marketplace_dev -c "select 1"` tienen que responder.

La conexión sale de `.env` (`DATABASE_URL=postgres://localhost/marketplace_dev`). Si tu Postgres usa otro usuario o contraseña, cambiá esa línea o exportá `DATABASE_URL` en la terminal, que tiene prioridad.

### 3. Levantar

```bash
cargo leptos watch
```

La primera compilación tarda unos minutos. Al arrancar, el servidor aplica las migraciones de `migrations/` y carga los datos de ejemplo (tenant `demo`, página `home` y ocho productos). No hay pasos manuales además de crear la base.

- Marketplace: <http://localhost:3000>
- Editor: <http://localhost:3000/editor>
- Catálogo de widgets (solo en desarrollo): <http://localhost:3000/dev/widgets>

En el editor: agregá widgets desde la paleta, arrastralos por la grilla, elegí el tamaño y los campos en el inspector, y tocá **Guardar**. Recargá `/` para ver el resultado.

## Comprobaciones

```bash
cargo fmt --check
cargo clippy --workspace -- -D warnings
cargo test --workspace
```

Los tests de guardado (`crates/db/tests/save.rs`) necesitan saber con qué usuario entrar a Postgres. Si la URL no lo trae, exportá `PGUSER` (una sola vez):

```bash
echo 'export PGUSER=$USER' >> ~/.zshrc && source ~/.zshrc
```

Usan la base local: `sqlx::test` crea una base temporal por test a partir de `DATABASE_URL`, le aplica las migraciones y la borra al terminar. El usuario de Postgres necesita permiso para crear bases (con Homebrew ya lo tiene).

## Estructura

```text
Cargo.toml        workspace + configuración de cargo-leptos
.env              DATABASE_URL
migrations/       SQL versionado (esquema y datos de ejemplo)
style/            tailwind.css, daisyui.mjs, daisyui-theme.mjs
crates/
  domain/         tipos y validación del panel
  grid/           matemática de la grilla (mover, empujar, compactar)
  ui/             primitivas Leptos sobre clases daisyUI
  widgets/        render de cada widget
  editor/         lienzo, paleta e inspector
  storefront/     marketplace final
  db/             consultas SQLx (solo servidor)
  web/            rutas y server functions (load_page, save_page)
  wasm/           entrada del navegador (hidratación)
  server/         binario Axum: pool, migraciones, rutas Leptos
```

Las dependencias van en una sola dirección: `server` y `wasm` dependen de `web`; `web`, de `editor` y `storefront`; y estos, de `widgets`, `ui`, `grid` y `domain`.

## Datos

Cada página guarda su panel como un documento JSON entero en `page_versions`. Cada Guardar inserta una versión nueva y se conservan las últimas 20. Si otra pestaña guardó antes, el editor avisa del conflicto y ofrece recargar.

## Para crear una migración nueva

```bash
sqlx migrate add <nombre>
```

Se aplica sola la próxima vez que arranca el servidor.
