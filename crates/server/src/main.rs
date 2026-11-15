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
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect_with(connect)
        .await
        .expect("no se pudo conectar a PostgreSQL");
    sqlx::migrate!("../../migrations")
        .run(&pool)
        .await
        .expect("no se pudieron aplicar las migraciones");

    let conf = get_configuration(None).unwrap();
    let addr = conf.leptos_options.site_addr;
    let leptos_options = conf.leptos_options;
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
    let mut app = Router::new().merge(web::media::media_routes(web::media::MediaState {
        pool: pool.clone(),
        store: store.clone(),
    }));
    for dir in ["pkg", "fonts", "img"] {
        app = app.nest_service(
            &format!("/{dir}"),
            ServeDir::new(format!("{site_root}/{dir}")),
        );
    }
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
        // El JS, WASM y CSS de `/pkg` tienen siempre el mismo nombre: con
        // `no-cache` el navegador pregunta antes de reusarlos y nunca mezcla
        // una versión vieja con el HTML nuevo (pasaba después de un `git pull`).
        .layer(SetResponseHeaderLayer::if_not_present(
            header::CACHE_CONTROL,
            HeaderValue::from_static("no-cache"),
        ))
        .with_state(leptos_options);

    log!("listening on http://{}", &addr);
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app.into_make_service())
        .await
        .unwrap();
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
