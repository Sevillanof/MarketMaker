use axum::http::{header, HeaderValue};
use axum::Router;
use leptos::logging::log;
use leptos::prelude::*;
use leptos_axum::{generate_route_list, LeptosRoutes};
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use tower_http::set_header::SetResponseHeaderLayer;
use web::*;

#[tokio::main]
async fn main() {
    // DATABASE_URL sale de `.env` (o del entorno, que tiene prioridad).
    _ = dotenvy::dotenv();
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

    let app = Router::new()
        .leptos_routes_with_context(
            &leptos_options,
            routes,
            {
                let pool = pool.clone();
                move || provide_context(pool.clone())
            },
            {
                let leptos_options = leptos_options.clone();
                move || shell(leptos_options.clone())
            },
        )
        .fallback(leptos_axum::file_and_error_handler(shell))
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
