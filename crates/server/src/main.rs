#![recursion_limit = "256"]

use axum::http::{header, HeaderValue};
use axum::Router;
use leptos::logging::log;
use leptos::prelude::*;
use leptos_axum::{generate_route_list, LeptosRoutes};
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use tower_http::services::{ServeDir, ServeFile};
use tower_http::set_header::SetResponseHeaderLayer;
use web::*;

mod html_cache;

#[tokio::main]
async fn main() {
    // DATABASE_URL sale de `.env` (o del entorno, que tiene prioridad).
    // `.env.local` va primero y no está en git: ahí van las claves secretas
    // (las de Cloudflare R2). Lo que ya está definido no se pisa.
    // ALLOWED_EMAILS no se toma de los archivos al arrancar: se relee de
    // ellos en cada pedido (ver `allowed_emails`), así quitar un email de
    // `.env.local` funciona sin reiniciar. Solo queda la de la terminal.
    let shell_allowed = std::env::var_os("ALLOWED_EMAILS");
    _ = dotenvy::from_filename(".env.local");
    _ = dotenvy::dotenv();
    match shell_allowed {
        Some(v) => std::env::set_var("ALLOWED_EMAILS", v),
        None => std::env::remove_var("ALLOWED_EMAILS"),
    }
    let database_url = std::env::var("DATABASE_URL").expect("falta DATABASE_URL (ver .env)");

    let mut connect: PgConnectOptions = database_url
        .parse()
        .expect("DATABASE_URL no es una URL de PostgreSQL válida");
    // Sin usuario en la URL ni PGUSER, SQLx 0.9 entra como "anonymous".
    // Usamos el usuario del sistema, que es el que crea Postgres de Homebrew.
    if !database_url.contains('@') && std::env::var_os("PGUSER").is_none() {
        if let Ok(user) = std::env::var("USER") {
            connect = connect.username(&user);
        }
    }
    // Conexiones a Postgres: 5 por defecto; en producción se sube con
    // DB_POOL_SIZE (en `.env.local`). Postgres admite 100.
    let pool_size = std::env::var("DB_POOL_SIZE")
        .ok()
        .and_then(|v| v.trim().parse::<u32>().ok())
        .filter(|n| (1..=80).contains(n))
        .unwrap_or(5);
    let pool = PgPoolOptions::new()
        .max_connections(pool_size)
        .connect_with(connect)
        .await
        .expect("no se pudo conectar a PostgreSQL");
    // Una versión anterior tiene que poder arrancar contra una base que ya
    // tiene migraciones más nuevas (volver atrás con deploy/release.sh).
    let mut migrator = sqlx::migrate!("../../migrations");
    migrator.set_ignore_missing(true);
    migrator
        .run(&pool)
        .await
        .expect("no se pudieron aplicar las migraciones");
    // La clave para cifrar las llaves de Mercado Pago y Andreani: SECRETS_KEY
    // si está; si no, el archivo `secrets.key` de esta carpeta (se crea solo
    // la primera vez). Sin clave el servidor anda igual y guarda sin cifrar.
    let key = match std::env::var("SECRETS_KEY") {
        Ok(text) if !text.trim().is_empty() => db::secrets::parse_key(&text)
            .ok_or_else(|| "SECRETS_KEY tiene que ser 64 caracteres hexadecimales".to_string()),
        _ => db::secrets::load_or_create_key(std::path::Path::new("secrets.key"))
            .map_err(|e| format!("secrets.key: {e}")),
    };
    match key {
        Ok(key) => db::secrets::init(key),
        Err(e) => leptos::logging::log!(
            "Sin clave para cifrar las llaves de Mercado Pago y Andreani ({e}): se guardan sin cifrar."
        ),
    }
    // La Whitelist de la web se suma a ALLOWED_EMAILS.
    web::auth::refresh_allowlist(&pool)
        .await
        .expect("no se pudo leer la Whitelist");

    let conf = get_configuration(None).unwrap();
    let addr = conf.leptos_options.site_addr;
    let mut leptos_options = conf.leptos_options;
    // Con `hash-files` (Cargo.toml) cargo-leptos deja `hash.txt` junto al
    // servidor y los archivos de `/pkg` llevan la versión en el nombre: el
    // navegador los guarda un año y la segunda visita no los vuelve a bajar.
    let hashed = std::env::current_exe()
        .ok()
        .and_then(|exe| {
            exe.parent()
                .map(|d| d.join(leptos_options.hash_file.as_ref()))
        })
        .is_some_and(|p| p.exists());
    leptos_options.hash_files = hashed;
    if !hashed {
        log!("Sin hash.txt junto al servidor: /pkg se sirve sin guardar en el navegador.");
    }
    // Generate the list of routes in your Leptos App
    let routes = generate_route_list(App);
    let site_root = leptos_options.site_root.to_string();

    // Quién puede entrar (ALLOWED_EMAILS). La cookie de sesión es `Secure`
    // solo en producción, para que funcione en http://localhost.
    let auth = AuthConfig::new(leptos_options.env == leptos::config::Env::PROD);
    match allowed_emails().len() {
        0 => log!(
            "Aviso: ALLOWED_EMAILS está vacío en .env.local, nadie puede entrar al editor (ver README)."
        ),
        n => log!("Emails habilitados para entrar: {n}"),
    }

    // Los archivos van antes que las páginas de la tienda: la ruta `/*any`
    // de la tienda también atraparía `/pkg/…` o `/favicon.ico`.
    // Mis fotos: subir, listar, borrar y servir los archivos (`/media/…`).
    // Dónde se guardan: Cloudflare R2 si está en `.env.local`, si no `MEDIA_DIR`.
    let store = ::media::Store::from_env().unwrap_or_else(|e| panic!("Mis fotos: {e}"));
    log!("Mis fotos se guardan en {}", store.describe());
    tokio::spawn(move_old_photos(pool.clone(), store.clone()));
    tokio::spawn(sweep_loose_files(pool.clone(), store.clone()));
    tokio::spawn(expire_unpaid_orders(pool.clone()));
    tokio::spawn(purge_events(pool.clone()));
    tokio::spawn(web::shipments::track_loop(pool.clone()));
    tokio::spawn(seal_old_secrets(pool.clone()));
    let mut app = Router::new().merge(web::media::media_routes(web::media::MediaState {
        pool: pool.clone(),
        store: store.clone(),
        // Traer fotos de la red local solo en pruebas (debug y a pedido).
        fetcher: ::media::Fetcher::new(
            cfg!(debug_assertions)
                && std::env::var("MEDIA_LINKS_ALLOW_LOCAL").as_deref() == Ok("1"),
        )
        .unwrap_or_else(|e| panic!("Mis fotos: {e}")),
    }));
    // Mercado Pago: vuelve de "Vincular mi cuenta" y avisa los pagos.
    app = app.merge(web::mp::routes(pool.clone()));
    app = app.merge(web::shipments::routes(pool.clone()));
    if web::mp::config().is_none() {
        log!("Mercado Pago sin configurar (MP_CLIENT_ID y MP_CLIENT_SECRET en .env.local): los pedidos se pagan coordinando.");
    }
    // Caddy pregunta acá antes de pedir el certificado https de una dirección
    // nueva (`ana.<plataforma>`): solo la plataforma y las tiendas que existen.
    app = app.route(
        "/tls-ok",
        axum::routing::get({
            let pool = pool.clone();
            move |q: axum::extract::Query<std::collections::HashMap<String, String>>| {
                let pool = pool.clone();
                async move { tls_ok(&pool, q.get("domain").map(String::as_str).unwrap_or("")).await }
            }
        }),
    );
    // Fotos y fuentes propias: no llevan versión en el nombre, así que no
    // pueden ser `immutable`, pero el navegador las reusa un día sin preguntar
    // (en el celular con red lenta cada pregunta es una espera por foto) y
    // una semana más mientras revisa en segundo plano.
    for (dir, cache) in [
        ("fonts", "public, max-age=2592000"),
        (
            "img",
            "public, max-age=86400, stale-while-revalidate=604800",
        ),
    ] {
        app = app.nest_service(
            &format!("/{dir}"),
            Router::new()
                .fallback_service(ServeDir::new(format!("{site_root}/{dir}")))
                .layer(SetResponseHeaderLayer::overriding(
                    header::CACHE_CONTROL,
                    HeaderValue::from_static(cache),
                )),
        );
    }
    // Los .js no se guardan como immutable aunque tengan versión en el
    // nombre: con `--split` cargo-leptos les pone el nombre antes de terminar
    // de escribirlos (ver deploy/hash-js.sh). Son chicos; el navegador
    // pregunta y casi siempre recibe un 304.
    let pkg = Router::new()
        // Si al lado de cada archivo hay un `.br` (lo arma `deploy/brotli-pkg.sh`),
        // se sirve ese: el programa de la página (WASM) baja de ~2 MB (gzip)
        // a ~1,4 MB en el celular.
        .fallback_service(ServeDir::new(format!("{site_root}/pkg")).precompressed_br())
        .layer(axum::middleware::map_response(
            move |uri: axum::http::Uri, mut res: axum::response::Response| async move {
                let cache = if hashed && !uri.path().ends_with(".js") {
                    "public, max-age=31536000, immutable"
                } else {
                    "no-cache"
                };
                res.headers_mut()
                    .insert(header::CACHE_CONTROL, HeaderValue::from_static(cache));
                res
            },
        ));
    app = app.nest_service("/pkg", pkg);
    let app = app
        .route_service(
            "/favicon.ico",
            ServeFile::new(format!("{site_root}/favicon.ico")),
        )
        .leptos_routes_with_context(
            &leptos_options,
            routes,
            {
                let pool = pool.clone();
                let store = store.clone();
                move || {
                    provide_context(pool.clone());
                    provide_context(auth.clone());
                    provide_context(store.clone());
                }
            },
            {
                let leptos_options = leptos_options.clone();
                move || shell(leptos_options.clone())
            },
        )
        .fallback(leptos_axum::file_and_error_handler(shell))
        .layer(axum::middleware::from_fn(same_site_posts))
        // Las páginas (y `/pkg` sin versión en el nombre) van con `no-cache`:
        // el navegador pregunta antes de reusarlas y nunca mezcla una versión
        // vieja con el HTML nuevo (pasaba después de un `git pull`).
        .layer(SetResponseHeaderLayer::if_not_present(
            header::CACHE_CONTROL,
            HeaderValue::from_static("no-cache"),
        ))
        // Protecciones estándar del navegador para todas las respuestas:
        // solo https (HSTS), no adivinar tipos de archivo, no mandar la
        // dirección completa a otros sitios, y que ningún sitio ajeno pueda
        // meter la tienda o el editor dentro de un marco (clickjacking).
        .layer(SetResponseHeaderLayer::if_not_present(
            header::STRICT_TRANSPORT_SECURITY,
            // Un año: la plataforma y todas las tiendas andan solo en HTTPS.
            HeaderValue::from_static("max-age=31536000; includeSubDomains"),
        ))
        .layer(SetResponseHeaderLayer::if_not_present(
            header::X_CONTENT_TYPE_OPTIONS,
            HeaderValue::from_static("nosniff"),
        ))
        .layer(SetResponseHeaderLayer::if_not_present(
            header::REFERRER_POLICY,
            HeaderValue::from_static("strict-origin-when-cross-origin"),
        ))
        .layer(SetResponseHeaderLayer::if_not_present(
            header::X_FRAME_OPTIONS,
            HeaderValue::from_static("DENY"),
        ))
        .layer(SetResponseHeaderLayer::if_not_present(
            header::CONTENT_SECURITY_POLICY,
            HeaderValue::from_static("frame-ancestors 'none'; base-uri 'self'; object-src 'none'"),
        ))
        .layer(SetResponseHeaderLayer::if_not_present(
            header::HeaderName::from_static("permissions-policy"),
            HeaderValue::from_static("camera=(), microphone=(), geolocation=(), payment=()"),
        ))
        .with_state(leptos_options)
        // Lo más de afuera: guarda unos segundos el HTML de los visitantes
        // sin sesión (ver `html_cache`), con todas las cabeceras ya puestas.
        .layer(axum::middleware::from_fn_with_state(
            html_cache::HtmlCache::from_env(),
            html_cache::middleware,
        ));

    log!("listening on http://{}", &addr);
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app.into_make_service())
        .await
        .unwrap();
}

/// ¿Se puede pedir el certificado https de `domain`? Solo si es la
/// plataforma (la de `APP_URL`) o una tienda que existe en ella.
async fn tls_ok(pool: &db::Pool, domain: &str) -> axum::http::StatusCode {
    use axum::http::StatusCode;
    let domain = domain.trim().to_lowercase();
    let app = web::email::app_url();
    let platform = app.split_once("://").map_or(app.as_str(), |(_, h)| h);
    let platform = platform
        .split(':')
        .next()
        .unwrap_or(platform)
        .to_lowercase();
    if domain == platform || domain == format!("www.{platform}") {
        return StatusCode::OK;
    }
    if ::domain::platform_host(&domain) != platform {
        return StatusCode::NOT_FOUND;
    }
    match ::domain::site_from_host(&domain) {
        ::domain::Site::Store(slug) => match db::get_tenant_by_slug(pool, &slug).await {
            Ok(Some(_)) => StatusCode::OK,
            _ => StatusCode::NOT_FOUND,
        },
        ::domain::Site::Platform => StatusCode::NOT_FOUND,
    }
}

/// Rechaza los POST que vienen de otro sitio (un formulario ajeno que intenta
/// guardar con la sesión del dueño). Los navegadores mandan `Origin` en todo
/// POST; si no coincide con la dirección pedida, no se procesa.
async fn same_site_posts(
    req: axum::extract::Request,
    next: axum::middleware::Next,
) -> axum::response::Response {
    use axum::response::IntoResponse;
    if req.method() == axum::http::Method::POST {
        let host = req
            .headers()
            .get(header::HOST)
            .and_then(|v| v.to_str().ok());
        let origin = req
            .headers()
            .get(header::ORIGIN)
            .and_then(|v| v.to_str().ok());
        if let (Some(host), Some(origin)) = (host, origin) {
            let origin_host = origin.split_once("://").map_or(origin, |(_, rest)| rest);
            if origin_host != host {
                return (axum::http::StatusCode::FORBIDDEN, "Pedido de otro sitio.")
                    .into_response();
            }
        }
    }
    next.run(req).await
}

/// Pasa las fotos subidas antes de ordenarlas por usuario (en `<foto>/`) a
/// `usuarios/<dueño>/fotos/<foto>/`. Corre de fondo al arrancar; mientras
/// tanto `/media/…` las busca en las dos carpetas, así nada deja de verse.
async fn move_old_photos(pool: db::Pool, store: ::media::Store) {
    let pending = match db::media::not_in_user_folder(&pool).await {
        Ok(p) => p,
        Err(e) => return log!("No pudimos revisar las fotos para ordenarlas: {e}"),
    };
    if pending.is_empty() {
        return;
    }
    log!("Ordenando {} fotos por usuario…", pending.len());
    let mut moved = 0;
    for (id, owner) in pending {
        let old = id.to_string();
        let new = db::media::media_folder(owner, id);
        let done = match store.move_photo(&old, &new).await {
            Ok(bytes) => Some(Some(bytes)),
            // Quizás ya se había movido y se cortó antes de anotarlo.
            Err(e) => match store.get(&new, "960.webp").await {
                Ok(Some(_)) => Some(None),
                _ => {
                    log!("No pudimos mover la foto {id}: {e}");
                    None
                }
            },
        };
        if let Some(bytes) = done {
            if db::media::mark_in_user_folder(&pool, id, bytes)
                .await
                .is_err()
            {
                continue;
            }
            moved += 1;
        }
    }
    log!("Fotos ordenadas por usuario: {moved}");
}

/// Cada hora cancela los pedidos que esperan el pago online hace más de 48
/// horas sin cambios, y devuelve su stock (ver `db::orders::expire_unpaid`).
/// Borra los recorridos anónimos de más de 90 días (ver `db::analytics`).
async fn purge_events(pool: db::Pool) {
    loop {
        match db::analytics::purge(&pool).await {
            Ok(0) => {}
            Ok(n) => log!("Uso y abandono: se borraron {n} eventos viejos"),
            Err(e) => log!("Uso y abandono: no pudimos borrar eventos viejos: {e}"),
        }
        if let Err(e) = db::buyers::purge_expired(&pool).await {
            log!("Cuentas de compradores: no pudimos limpiar lo vencido: {e}");
        }
        tokio::time::sleep(std::time::Duration::from_secs(6 * 60 * 60)).await;
    }
}

async fn expire_unpaid_orders(pool: db::Pool) {
    const EVERY: std::time::Duration = std::time::Duration::from_secs(60 * 60);
    loop {
        // A las 48 horas se cancela y se le avisa; a las 24, un recordatorio.
        // Primero lo vencido: un pedido que ya se cancela no recibe además
        // el recordatorio.
        match web::email::expire_unpaid(&pool, 48).await {
            Ok(0) => {}
            Ok(n) => log!("Pedidos: se cancelaron {n} que esperaban el pago hace más de 48 horas"),
            Err(e) => log!("Pedidos: no pudimos revisar los pedidos sin pagar: {e}"),
        }
        web::email::remind_unpaid(&pool, 24).await;
        tokio::time::sleep(EVERY).await;
    }
}

/// Cada tanto borra del almacén las carpetas de fotos que no son de ninguna
/// foto de Mis fotos (una subida que se cortó, un borrado que falló) o que
/// quedaron en la carpeta vieja. Solo toca carpetas con forma de foto y de
/// más de una hora, así nunca pisa una subida en curso.
async fn sweep_loose_files(pool: db::Pool, store: ::media::Store) {
    const EVERY: std::time::Duration = std::time::Duration::from_secs(6 * 60 * 60);
    // Primero que termine de mover las fotos viejas.
    tokio::time::sleep(std::time::Duration::from_secs(5 * 60)).await;
    loop {
        match sweep_once(&pool, &store).await {
            Ok(0) => {}
            Ok(n) => log!("Mis fotos: se borraron {n} carpetas sueltas del almacén"),
            Err(e) => log!("Mis fotos: no pudimos revisar el almacén: {e}"),
        }
        tokio::time::sleep(EVERY).await;
    }
}

async fn sweep_once(pool: &db::Pool, store: &::media::Store) -> Result<usize, String> {
    let expected = db::media::all_folders(pool)
        .await
        .map_err(|e| e.to_string())?;
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    let cutoff = ::media::utc_iso(now.saturating_sub(60 * 60));
    let mut removed = 0;
    for f in store.list_folders().await.map_err(|e| e.to_string())? {
        let Some(id) = ::media::parse_folder(&f.folder) else {
            continue;
        };
        // Un archivo recién escrito puede ser de una subida o una mudanza en
        // curso: solo se borra lo que tiene más de una hora.
        let keep = expected.get(id).is_some_and(|want| *want == f.folder);
        if keep || f.newest.is_empty() || f.newest > cutoff {
            continue;
        }
        store
            .delete_all(&f.folder)
            .await
            .map_err(|e| e.to_string())?;
        removed += 1;
    }
    Ok(removed)
}

/// Cifra las llaves de Mercado Pago y Andreani que quedaron guardadas en
/// claro. Espera un rato después de arrancar: si esta versión no anda y
/// `deploy/release.sh` vuelve a una anterior que no sabe leerlas cifradas, la
/// base tiene que estar como la dejó.
async fn seal_old_secrets(pool: sqlx::PgPool) {
    tokio::time::sleep(std::time::Duration::from_secs(15 * 60)).await;
    match db::secrets::seal_stored(&pool).await {
        Ok(0) => {}
        Ok(n) => leptos::logging::log!("Llaves cifradas: {n} cuentas."),
        Err(e) => leptos::logging::log!("No se pudieron cifrar las llaves guardadas: {e}"),
    }
}
