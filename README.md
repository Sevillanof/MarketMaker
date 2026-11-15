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
- Entrar: <http://localhost:3000/entrar>

### Entrar al editor

El editor es solo para el dueño de la tienda. Por ahora solo pueden entrar los emails de una lista: en `.env`, completá `ALLOWED_EMAILS` con tu email (varios, separados por comas) y reiniciá el servidor.

1. Abrí `/editor` (o `/entrar`) y escribí tu email.
2. Todavía no se mandan emails: el código de 6 números aparece en la terminal donde corre `cargo leptos watch` ("Código para entrar de …").
3. Escribilo y entrás al editor. La sesión dura 30 días en ese navegador.

La primera persona habilitada que entra queda como dueña de la tienda `demo`; las demás pueden entrar pero no editarla. Con la sesión abierta, la tienda muestra arriba la barra **Editar mi tienda** (solo la ve el dueño) con el botón **Salir**.
- Catálogo de widgets (solo en desarrollo): <http://localhost:3000/dev/widgets>

En el editor (pensado para alguien sin experiencia):

- Agregá bloques desde la paleta de la izquierda (miniaturas agrupadas, con buscador). El bloque nuevo se agrega al final, queda elegido y la vista lo lleva hasta él.
- Elegí un bloque con un clic (o con Tab y Enter) y editalo en el panel de la derecha. La pestaña **Contenido** tiene sus textos, imágenes, listas y el grupo **Botón** (un interruptor para mostrarlo, su texto y adónde lleva, con un aviso si falta algo). La pestaña **Diseño** tiene el tamaño (ancho con nombres, Más bajo/Más alto y "Ajustar a mano" para los números exactos) y las letras del bloque. Las imágenes se eligen de una galería; también se puede pegar una dirección.
- Doble clic sobre un título, subtítulo o descripción en el lienzo para escribir ahí mismo: Enter (o Ctrl/⌘+Enter en textos largos) o un clic afuera lo guarda, Esc lo deja como estaba. Los botones y enlaces se cambian en el panel.
- Movelo arrastrando, con los botones Subir y Bajar o con las flechas del teclado. Cambiá el tamaño tirando de los bordes, con los botones de ancho o con Mayús + flechas.
- Los errores aparecen junto al campo y en un aviso al guardar, en palabras simples.
- Atajos: Ctrl/⌘+Z deshacer, Ctrl/⌘+Mayús+Z rehacer, Ctrl/⌘+S guardar, Suprimir borrar, Esc soltar el bloque. Lo que escribís en un campo se deshace de una sola vez.
- Los cambios sin guardar quedan también en el navegador: si cerrás la pestaña, al volver el editor ofrece recuperarlos.
- **Letras** (en la barra de arriba) cambia las letras de toda la tienda: una combinación de fuentes para títulos y texto, o cada una por separado, y el tamaño general (Normal, Grande, Muy grande). Cada bloque tiene además **Letras de este bloque** en la pestaña Diseño del panel derecho: fuente, tamaño, negrita, cursiva y alineación; lo que no se toca sigue a la tienda. Las fuentes se sirven desde `public/fonts` (licencia OFL, ver `public/fonts/LICENSE.txt`).
- **Vista previa** muestra la página sin bordes ni controles. **Guardar** publica los cambios; recargá `/` para verlos.

## Widgets

Menú superior, Banner, Carrusel, Categorías, Cuadrícula de productos, Producto destacado, Beneficios, Oferta con cuenta regresiva, Tarjeta, Texto, Pestañas, Preguntas frecuentes, Galería de fotos, Testimonios, Logos de marcas, Imagen y Pie de página.

La grilla del panel tiene 48 columnas y filas de 8 px, con imán: los bloques se ubican y miden con precisión sin salirse de la grilla. Cada widget define su ancho (columnas, de 48) y alto (filas de 8 px) mínimos, máximos y por defecto en `Widget::limits` (`crates/domain/src/widgets.rs`). El editor y el guardado no permiten salir de esos límites. `/dev/widgets` muestra cada widget en su tamaño mínimo, por defecto y máximo.

Mientras carga una página o una imagen se ve un skeleton de daisyUI.

## Imágenes

Las imágenes de ejemplo se sirven desde `public/img`, sin depender de servicios externos: las fotos en AVIF con respaldo WebP (`<picture>`), y los logos en SVG. Para regenerarlas: `python3 scripts/gen_images.py` (necesita Pillow 11.3 o más nuevo).

Para una imagen propia en `public/img`, alcanza con poner `/img/nombre.webp` en el editor: si existe `/img/nombre.avif`, el navegador la usa automáticamente.

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
public/img/       imágenes de ejemplo (AVIF + WebP, logos SVG)
scripts/          gen_images.py (genera public/img)
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

Cada página guarda su panel como un documento JSON entero en `page_versions`. El campo `schema` indica el formato: en el 2 cada nodo guarda `"size": { "w": 6, "h": 4 }`. Los documentos del schema 1 (tamaños fijos como `"size": "full"`) se convierten con una migración y, por las dudas, también al leerlos (`domain::parse_doc`). Cada Guardar inserta una versión nueva y se conservan las últimas 20. Si otra pestaña guardó antes, el editor avisa del conflicto y ofrece recargar.

## Para crear una migración nueva

```bash
sqlx migrate add <nombre>
```

Se aplica sola la próxima vez que arranca el servidor.
