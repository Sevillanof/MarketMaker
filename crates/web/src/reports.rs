//! Reportes de errores: cualquier persona marca un área de la página, escribe
//! qué pasó y lo manda desde ahí (el botón y la selección viven en
//! `public/js/reportar.js`). Llegan a `/admin/reportes` y por email a las
//! cuentas de `ADMIN_EMAILS`, con la captura. Solo la cuenta administradora
//! los ve: las capturas pueden mostrar datos personales.

use leptos::prelude::*;
use leptos_router::hooks::use_query_map;
use serde::{Deserialize, Serialize};

/// Lo que se muestra de un reporte en la bandeja.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct ReportItem {
    pub num: i64,
    pub estado: String,
    pub descripcion: String,
    pub pagina: String,
    pub quien: String,
    pub pantalla: String,
    pub navegador: String,
    pub captura: bool,
    pub hace: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
pub struct ReportsData {
    pub items: Vec<ReportItem>,
    pub nuevos: i64,
}

#[cfg(feature = "ssr")]
fn hace(secs: i64) -> String {
    match secs.max(0) {
        0..=59 => "hace un momento".into(),
        60..=3599 => format!("hace {} min", secs / 60),
        3600..=86_399 => format!("hace {} h", secs / 3600),
        s => match s / 86_400 {
            1 => "hace 1 día".into(),
            d => format!("hace {d} días"),
        },
    }
}

#[cfg(feature = "ssr")]
async fn require_admin() -> Result<(), ServerFnError> {
    match crate::auth::current_user().await? {
        Some(u) if crate::auth::is_admin(&u.email) => Ok(()),
        _ => Err(ServerFnError::new(
            "Solo el administrador puede ver esta página.",
        )),
    }
}

/// Los reportes (más nuevos primero). `estado` vacío = todos.
#[server]
pub async fn bug_reports(estado: String) -> Result<ReportsData, ServerFnError> {
    use crate::server::{internal, pool};
    require_admin().await?;
    let pool = pool()?;
    let estado = match estado.as_str() {
        "nuevo" | "en_curso" | "resuelto" => estado,
        _ => String::new(),
    };
    let items = db::reports::list(&pool, &estado, 100)
        .await
        .map_err(internal)?
        .into_iter()
        .map(|r| ReportItem {
            num: r.num,
            estado: r.estado,
            descripcion: r.descripcion,
            pagina: r.pagina,
            quien: r.quien,
            pantalla: r.pantalla,
            navegador: r.navegador,
            captura: r.tiene_captura,
            hace: hace(r.hace),
        })
        .collect();
    let nuevos = db::reports::count_new(&pool).await.map_err(internal)?;
    Ok(ReportsData { items, nuevos })
}

/// Cambia el estado de un reporte.
#[server]
pub async fn set_bug_status(num: i64, estado: String) -> Result<(), ServerFnError> {
    use crate::server::{internal, pool};
    require_admin().await?;
    if !matches!(estado.as_str(), "nuevo" | "en_curso" | "resuelto") {
        return Err(ServerFnError::new("Estado no válido."));
    }
    db::reports::set_status(&pool()?, num, &estado)
        .await
        .map_err(internal)?;
    Ok(())
}

// ---------------------------------------------------------------- rutas HTTP

#[cfg(feature = "ssr")]
pub use http_routes::routes;

#[cfg(feature = "ssr")]
mod http_routes {
    use axum::extract::{DefaultBodyLimit, Multipart, Path, State};
    use axum::http::{header, HeaderMap, StatusCode};
    use axum::response::{IntoResponse, Response};
    use axum::routing::{get, post};
    use axum::Router;
    use std::time::Duration;

    /// La captura pesa como mucho esto (el navegador la achica antes).
    const MAX_IMAGE: usize = 1_200_000;
    const MAX_TEXT: usize = 2000;

    /// `POST /api/reportes` (multipart) y `GET /api/reportes/{num}/captura`
    /// (solo administrador).
    pub fn routes<S: Clone + Send + Sync + 'static>(pool: db::Pool) -> Router<S> {
        Router::new()
            .route("/api/reportes", post(submit))
            .route("/api/reportes/{num}/captura", get(screenshot))
            .layer(DefaultBodyLimit::max(MAX_IMAGE + 128 * 1024))
            .with_state(pool)
    }

    fn reply(status: StatusCode, msg: &str) -> Response {
        (
            status,
            [(header::CONTENT_TYPE, "text/plain; charset=utf-8")],
            msg.to_string(),
        )
            .into_response()
    }

    /// La cuenta de la sesión, si hay.
    async fn session(pool: &db::Pool, headers: &HeaderMap) -> Option<db::auth::SessionUser> {
        let token = headers
            .get_all(header::COOKIE)
            .iter()
            .filter_map(|v| v.to_str().ok())
            .flat_map(|v| v.split(';'))
            .filter_map(|c| c.trim().split_once('='))
            .find(|(name, _)| *name == crate::auth::COOKIE)
            .map(|(_, v)| v.to_string())
            .filter(|v| !v.is_empty() && v.len() <= 128)?;
        db::auth::session_user(pool, &token).await.ok().flatten()
    }

    /// Sin parámetros (`?t=…` puede ser un pase de entrada) ni `#`.
    fn clean_page(raw: &str) -> String {
        raw.split(['?', '#'])
            .next()
            .unwrap_or("")
            .chars()
            .take(300)
            .collect()
    }

    fn one_line(s: &str, max: usize) -> String {
        s.chars()
            .filter(|c| !c.is_control())
            .take(max)
            .collect::<String>()
            .trim()
            .to_string()
    }

    async fn submit(
        State(pool): State<db::Pool>,
        headers: HeaderMap,
        mut form: Multipart,
    ) -> Response {
        let ip = crate::limits::ip_from(&headers);
        // Contra el que manda reportes en serie: pocos por hora, algunos por
        // día, y un tope general para que nadie llene la base.
        if !crate::limits::allow(&format!("bug-h:{ip}"), 5, Duration::from_secs(3600))
            || !crate::limits::allow(&format!("bug-d:{ip}"), 15, Duration::from_secs(86_400))
            || !crate::limits::allow("bug-all", 200, Duration::from_secs(3600))
        {
            return reply(
                StatusCode::TOO_MANY_REQUESTS,
                "Recibimos varios reportes seguidos. Probá de nuevo en un rato.",
            );
        }
        let (mut desc, mut page, mut contact, mut screen) =
            (String::new(), String::new(), String::new(), String::new());
        let mut image: Option<Vec<u8>> = None;
        while let Ok(Some(field)) = form.next_field().await {
            let name = field.name().unwrap_or("").to_string();
            let Ok(bytes) = field.bytes().await else {
                return reply(StatusCode::BAD_REQUEST, "No pudimos leer el reporte.");
            };
            match name.as_str() {
                "captura" if !bytes.is_empty() => {
                    if bytes.len() > MAX_IMAGE {
                        return reply(
                            StatusCode::PAYLOAD_TOO_LARGE,
                            "La captura pesa demasiado. Probá con un área más chica.",
                        );
                    }
                    // Solo JPEG.
                    if !bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
                        return reply(StatusCode::BAD_REQUEST, "La captura no es válida.");
                    }
                    image = Some(bytes.to_vec());
                }
                "descripcion" => desc = String::from_utf8_lossy(&bytes).into_owned(),
                "pagina" => page = String::from_utf8_lossy(&bytes).into_owned(),
                "contacto" => contact = String::from_utf8_lossy(&bytes).into_owned(),
                "pantalla" => screen = String::from_utf8_lossy(&bytes).into_owned(),
                _ => {}
            }
        }
        let desc = desc.trim().chars().take(MAX_TEXT).collect::<String>();
        if desc.chars().count() < 5 {
            return reply(StatusCode::BAD_REQUEST, "Contanos qué pasó para poder ayudarte.");
        }
        let user = session(&pool, &headers).await;
        let quien = match &user {
            Some(u) => u.email.clone(),
            None => one_line(&contact, 120),
        };
        let agent = headers
            .get(header::USER_AGENT)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        let page = clean_page(&page);
        let screen = one_line(&screen, 40);
        let agent = one_line(agent, 200);
        let num = match db::reports::insert(
            &pool,
            db::reports::NewReport {
                descripcion: &desc,
                pagina: &page,
                quien: &quien,
                pantalla: &screen,
                navegador: &agent,
                captura: image.as_deref(),
            },
        )
        .await
        {
            Ok(n) => n,
            Err(e) => {
                leptos::logging::log!("Reportes: no se pudo guardar: {e}");
                return reply(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "No pudimos enviar el reporte. Probá de nuevo en unos minutos.",
                );
            }
        };
        // Aviso por email a la cuenta administradora (de fondo).
        let to: Vec<String> = crate::auth::admin_emails().into_iter().collect();
        if let Ok(Some(r)) = db::reports::get(&pool, num).await {
            let has = image.is_some();
            let mail = crate::email::bug_report(to, &r, has);
            let files = image
                .map(|bytes| {
                    vec![crate::email::Attachment {
                        filename: format!("reporte-{num}.jpg"),
                        content_id: Some("captura".into()),
                        bytes,
                    }]
                })
                .unwrap_or_default();
            crate::email::send_attached(mail, files);
        }
        (StatusCode::CREATED, format!("{num}")).into_response()
    }

    async fn screenshot(
        State(pool): State<db::Pool>,
        headers: HeaderMap,
        Path(num): Path<i64>,
    ) -> Response {
        match session(&pool, &headers).await {
            Some(u) if crate::auth::is_admin(&u.email) => {}
            _ => return reply(StatusCode::NOT_FOUND, "No encontrado."),
        }
        match db::reports::screenshot(&pool, num).await {
            Ok(Some(bytes)) => (
                [
                    (header::CONTENT_TYPE, "image/jpeg"),
                    (header::CACHE_CONTROL, "private, max-age=3600"),
                ],
                bytes,
            )
                .into_response(),
            _ => reply(StatusCode::NOT_FOUND, "No encontrado."),
        }
    }
}

// ---------------------------------------------------------------- la bandeja

fn state_badge(estado: &str) -> (&'static str, &'static str) {
    match estado {
        "resuelto" => ("badge badge-success badge-soft badge-sm", "Resuelto"),
        "en_curso" => ("badge badge-warning badge-soft badge-sm", "En curso"),
        _ => ("badge badge-error badge-soft badge-sm", "Nuevo"),
    }
}

/// `/admin/reportes`: los reportes que mandó la gente. Solo el administrador.
#[component]
pub fn ReportsPage() -> impl IntoView {
    if !matches!(crate::site::current_site(), domain::Site::Platform) {
        return view! { <crate::platform::NotAStore/> }.into_any();
    }
    let q = use_query_map();
    let get = move |k: &str| q.with(|m| m.get(k).unwrap_or_default());
    let filter = move || match get("estado").as_str() {
        s @ ("nuevo" | "en_curso" | "resuelto") => s.to_string(),
        _ => String::new(),
    };
    let selected = move || get("r").parse::<i64>().ok();
    let change = ServerAction::<SetBugStatus>::new();
    let data = Resource::new(
        move || (filter(), change.version().get()),
        |(f, _)| bug_reports(f),
    );
    let href = move |estado: &str, r: Option<i64>| {
        let mut u = format!("/admin/reportes?estado={estado}");
        if let Some(r) = r {
            u.push_str(&format!("&r={r}"));
        }
        u
    };
    let pill = |active: bool| {
        if active {
            "btn btn-sm btn-primary"
        } else {
            "btn btn-sm btn-outline"
        }
    };
    view! {
        <leptos_meta::Title text="Reportes · MarketMaker"/>
        <div data-theme="light" class="min-h-screen bg-base-200 text-base-content">
            <header class="flex items-center gap-3 border-b border-base-300 bg-base-100 px-6 py-3">
                <a href="/" class="text-lg"><crate::account::Logo/></a>
                <span class="text-sm text-base-content/60">"Administración"</span>
            </header>
            <main class="mx-auto flex max-w-6xl flex-col gap-5 px-4 py-8">
                <div>
                    <h1 class="text-2xl font-bold">"Reportes"</h1>
                    <p class="text-sm text-base-content/70">"Lo que la gente marcó como problema, con su captura. Solo lo ves vos."</p>
                </div>
                <nav class="flex flex-wrap gap-2" aria-label="Estado">
                    {[("", "Todos"), ("nuevo", "Nuevos"), ("en_curso", "En curso"), ("resuelto", "Resueltos")].map(|(v, l)| view! {
                        <a class=move || pill(filter() == v) href=move || href(v, None)
                            aria-current=move || (filter() == v).then_some("true")>{l}</a>
                    })}
                </nav>
                <Suspense fallback=|| view! { <ui::ReportsSkeleton/> }>
                    {move || Suspend::new(async move {
                        match data.await {
                            Err(_) => view! {
                                <p class="text-sm text-base-content/70">"Esta página es solo para la cuenta administradora."</p>
                            }.into_any(),
                            Ok(d) if d.items.is_empty() => view! {
                                <p class="py-8 text-center text-sm text-base-content/60">"Todavía no hay reportes en esta lista."</p>
                            }.into_any(),
                            Ok(d) => {
                                let items = d.items;
                                let current = selected()
                                    .and_then(|n| items.iter().find(|i| i.num == n).cloned())
                                    .unwrap_or_else(|| items[0].clone());
                                let list = items.into_iter().map(|i| {
                                    let (cls, label) = state_badge(&i.estado);
                                    let on = i.num == current.num;
                                    let target = href(&filter(), Some(i.num));
                                    view! {
                                        <li><a href=target aria-current=on.then_some("true")
                                            class=move || if on { "flex flex-col gap-1 bg-base-200 px-3 py-2.5" } else { "flex flex-col gap-1 px-3 py-2.5 hover:bg-base-200/60" }>
                                            <span class="flex items-center gap-2 text-sm">
                                                <b>{format!("#{}", i.num)}</b>
                                                <span class=cls>{label}</span>
                                                <span class="ml-auto text-xs text-base-content/60">{i.hace}</span>
                                            </span>
                                            <span class="line-clamp-2 text-sm">{i.descripcion}</span>
                                        </a></li>
                                    }
                                }).collect_view();
                                let num = current.num;
                                let (cls, label) = state_badge(&current.estado);
                                let who = if current.quien.is_empty() { "Visitante sin cuenta".to_string() } else { current.quien.clone() };
                                let buttons = [("nuevo", "Nuevo"), ("en_curso", "En curso"), ("resuelto", "Resuelto")].map(|(v, l)| {
                                    let active = current.estado == v;
                                    view! {
                                        <button type="button" class=if active { "btn btn-sm btn-primary" } else { "btn btn-sm btn-outline" }
                                            disabled=active
                                            on:click=move |_| { change.dispatch(SetBugStatus { num, estado: v.to_string() }); }>{l}</button>
                                    }
                                });
                                view! {
                                    <div class="grid items-start gap-4 lg:grid-cols-[2fr_3fr]">
                                        <ul class="divide-y divide-base-300 overflow-hidden rounded-box border border-base-300 bg-base-100">{list}</ul>
                                        <section class="card border border-base-300 bg-base-100"><div class="card-body gap-3 p-4">
                                            <div class="flex items-center gap-2">
                                                <h2 class="text-lg font-semibold">{format!("Reporte #{num}")}</h2>
                                                <span class=cls>{label}</span>
                                                <span class="ml-auto text-xs text-base-content/60">{current.hace.clone()}</span>
                                            </div>
                                            {current.captura.then(|| view! {
                                                <a href=format!("/api/reportes/{num}/captura") target="_blank" rel="noopener" class="block overflow-hidden rounded-lg border border-base-300">
                                                    <img src=format!("/api/reportes/{num}/captura") alt="Captura del problema" loading="lazy" class="max-h-96 w-full bg-base-200 object-contain"/>
                                                </a>
                                            })}
                                            <p class="whitespace-pre-wrap text-sm">{current.descripcion.clone()}</p>
                                            <dl class="grid grid-cols-[auto_1fr] gap-x-3 gap-y-1 text-xs text-base-content/70">
                                                <dt class="font-semibold">"Página"</dt><dd class="break-all">{current.pagina.clone()}</dd>
                                                <dt class="font-semibold">"De"</dt><dd class="break-all">{who}</dd>
                                                <dt class="font-semibold">"Pantalla"</dt><dd>{current.pantalla.clone()}</dd>
                                                <dt class="font-semibold">"Navegador"</dt><dd class="break-all">{current.navegador.clone()}</dd>
                                            </dl>
                                            <div class="flex flex-wrap gap-2" role="group" aria-label="Cambiar estado">{buttons}</div>
                                        </div></section>
                                    </div>
                                }.into_any()
                            }
                        }
                    })}
                </Suspense>
            </main>
        </div>
    }
    .into_any()
}
