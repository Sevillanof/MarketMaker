//! Reportes de errores: cualquier persona marca un área de la página, escribe
//! qué pasó y lo manda desde ahí (el botón y la selección viven en
//! `public/js/reportar.js`). Llegan a `/admin/reportes` y por email a las
//! cuentas de `ADMIN_EMAILS`, con la captura. Solo la cuenta administradora
//! los ve: las capturas pueden mostrar datos personales. Se ven en una
//! ventana (modal) que se abre desde el menú de la cuenta, como la Whitelist.

use leptos::prelude::*;
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
    /// Cuántos hay: nuevos, en curso y resueltos.
    pub nuevos: i64,
    pub en_curso: i64,
    pub resueltos: i64,
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
    let (nuevos, en_curso, resueltos) = db::reports::counts(&pool).await.map_err(internal)?;
    Ok(ReportsData {
        items,
        nuevos,
        en_curso,
        resueltos,
    })
}

/// Cuántos reportes siguen sin atender (el número del menú).
#[server]
pub async fn bug_report_new() -> Result<i64, ServerFnError> {
    use crate::server::{internal, pool};
    require_admin().await?;
    db::reports::count_new(&pool()?).await.map_err(internal)
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
            return reply(
                StatusCode::BAD_REQUEST,
                "Contanos qué pasó para poder ayudarte.",
            );
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

const CONNECTION: &str =
    "No pudimos comunicarnos con el servidor. Revisá tu conexión y probá de nuevo.";

/// Abre la ventana y pide la lista de nuevo.
pub fn open_reports(dialog: NodeRef<leptos::html::Dialog>, version: RwSignal<u32>) {
    version.update(|v| *v += 1);
    if let Some(d) = dialog.get() {
        _ = d.show_modal();
    }
}

/// Copia el reporte (captura + texto) al portapapeles con la ayuda de
/// `window.mkbugCopy`, que vive en `public/js/reportar.js`.
async fn copy_report(num: i64, text: &str, with_image: bool) -> bool {
    use wasm_bindgen::JsCast;
    let Some(win) = web_sys::window() else { return false };
    let Ok(f) = js_sys::Reflect::get(&win, &"mkbugCopy".into()) else { return false };
    let Ok(f) = f.dyn_into::<js_sys::Function>() else { return false };
    let Ok(p) = f.call3(&win, &(num as f64).into(), &text.into(), &with_image.into()) else { return false };
    let Ok(p) = p.dyn_into::<js_sys::Promise>() else { return false };
    wasm_bindgen_futures::JsFuture::from(p).await.map(|v| v.as_bool().unwrap_or(false)).unwrap_or(false)
}

/// La ventana de Reportes: lista a la izquierda, el detalle con la captura a
/// la derecha y, abajo, los botones para cambiar el estado.
#[component]
pub fn ReportsDialog(
    dialog: NodeRef<leptos::html::Dialog>,
    version: RwSignal<u32>,
) -> impl IntoView {
    let filter = RwSignal::new(String::new());
    let selected = RwSignal::new(None::<i64>);
    let problem = RwSignal::new(None::<String>);
    // Solo en el navegador: se pide al abrir la ventana, no al armar la página.
    let data = LocalResource::new(move || {
        let v = version.get();
        let f = filter.get();
        async move {
            if v == 0 {
                Ok(ReportsData::default())
            } else {
                bug_reports(f).await
            }
        }
    });
    // 0 = nada, 1 = copiado, 2 = no se pudo copiar
    let change = Action::new(|(n, e): &(i64, String)| set_bug_status(*n, e.clone()));
    Effect::new(move |_| match change.value().get() {
        Some(Ok(())) => {
            problem.set(None);
            version.update(|v| *v += 1);
        }
        Some(Err(_)) => problem.set(Some(CONNECTION.into())),
        None => {}
    });
    let pill = |active: bool| {
        if active {
            "btn btn-sm btn-primary"
        } else {
            "btn btn-sm btn-outline"
        }
    };
    view! {
        <dialog node_ref=dialog class="modal" aria-labelledby="reportes-titulo"
            on:close=move |_| { problem.set(None); selected.set(None); filter.set(String::new()); }>
            <div class="modal-box flex max-h-[92vh] max-w-4xl flex-col p-0">
                <div class="flex flex-col gap-3 p-6 pb-3">
                    <div class="flex items-start justify-between gap-4">
                        <div>
                            <h2 id="reportes-titulo" class="text-xl font-extrabold">"Reportes"</h2>
                            <p class="text-sm text-base-content/70">"Lo que la gente marcó como problema, con su captura. Solo lo ves vos."</p>
                        </div>
                        <button type="button" class="btn btn-ghost btn-sm btn-circle" aria-label="Cerrar"
                            on:click=move |_| { if let Some(d) = dialog.get() { d.close(); } }>"✕"</button>
                    </div>
                    <nav class="flex flex-wrap gap-2" aria-label="Estado">
                        {move || {
                            let d = data.get().and_then(|r| r.ok()).unwrap_or_default();
                            let n = |c: i64| if c > 0 { format!(" · {c}") } else { String::new() };
                            [
                                ("", "Todos".to_string()),
                                ("nuevo", format!("Nuevos{}", n(d.nuevos))),
                                ("en_curso", format!("En curso{}", n(d.en_curso))),
                                ("resuelto", format!("Resueltos{}", n(d.resueltos))),
                            ].map(|(v, l)| view! {
                                <button type="button" class=move || pill(filter.get() == v)
                                    aria-pressed=move || (filter.get() == v).to_string()
                                    on:click=move |_| { selected.set(None); filter.set(v.to_string()); }>{l}</button>
                            })
                        }}
                    </nav>
                </div>
                {move || match data.get() {
                    None => view! { <ui::ReportsSkeleton/> }.into_any(),
                    Some(Err(_)) => view! { <p class="border-t border-base-300 p-6"><crate::account::FieldNote ok=false text=CONNECTION/></p> }.into_any(),
                    Some(Ok(_)) if version.get() == 0 => ().into_any(),
                    Some(Ok(d)) if d.items.is_empty() => view! {
                        <p class="border-t border-base-300 py-10 text-center text-sm text-base-content/60">"Todavía no hay reportes en esta lista."</p>
                    }.into_any(),
                    Some(Ok(d)) => {
                        let items = d.items;
                        let current = selected.get()
                            .and_then(|n| items.iter().find(|i| i.num == n).cloned())
                            .unwrap_or_else(|| items[0].clone());
                        let cur = current.num;
                        let list = items.into_iter().map(|i| {
                            let (cls, label) = state_badge(&i.estado);
                            let on = i.num == cur;
                            let num = i.num;
                            view! {
                                <li><button type="button" aria-current=on.then_some("true")
                                    class=if on { "flex w-full flex-col gap-1 bg-base-200 px-6 py-2.5 text-left" } else { "flex w-full flex-col gap-1 px-6 py-2.5 text-left hover:bg-base-200/60" }
                                    on:click=move |_| { selected.set(Some(num)); }>
                                    <span class="flex items-center gap-2 text-sm">
                                        <b>{format!("#{}", i.num)}</b>
                                        <span class=cls>{label}</span>
                                        <span class="ml-auto text-xs text-base-content/60">{i.hace}</span>
                                    </span>
                                    <span class="line-clamp-2 text-sm">{i.descripcion}</span>
                                </button></li>
                            }
                        }).collect_view();
                        let (cls, label) = state_badge(&current.estado);
                        let who = if current.quien.is_empty() { "Visitante sin cuenta".to_string() } else { current.quien.clone() };
                        let estado = current.estado.clone();
                        let buttons = [("nuevo", "Nuevo"), ("en_curso", "En curso"), ("resuelto", "Resuelto")].map(|(v, l)| {
                            let active = estado == v;
                            view! {
                                <button type="button" class=if active { "btn btn-sm btn-primary" } else { "btn btn-sm btn-outline" }
                                    aria-pressed=active.to_string()
                                    disabled=move || active || change.pending().get()
                                    on:click=move |_| { change.dispatch((cur, v.to_string())); }>{l}</button>
                            }
                        });
                        let copy_text = format!(
                            "Reporte #{cur} de MarketMaker ({})\n\n{}\n\nPágina: {}\nDe: {}\nPantalla: {}\nNavegador: {}\nRecibido: {}",
                            label, current.descripcion, current.pagina, who, current.pantalla, current.navegador, current.hace
                        );
                        let has_shot = current.captura;
                        let copy = move |_| {
                            let text = copy_text.clone();
                            let n = ui::use_notifier();
                            leptos::task::spawn_local(async move {
                                let ok = copy_report(cur, &text, has_shot).await;
                                if ok {
                                    n.success("Reporte copiado", "Pegalo con Ctrl+V.");
                                } else {
                                    n.error("No pudimos copiar el reporte", "Probá de nuevo.");
                                }
                            });
                        };
                        view! {
                            <div class="grid min-h-0 grow overflow-hidden border-t border-base-300 md:grid-cols-[2fr_3fr]">
                                <ul class="max-h-52 divide-y divide-base-300 overflow-y-auto md:max-h-none">{list}</ul>
                                <section class="flex min-h-0 flex-col gap-3 overflow-y-auto border-base-300 p-5 max-md:border-t md:border-l">
                                    <div class="flex items-center gap-2">
                                        <h3 class="font-semibold">{format!("Reporte #{cur}")}</h3>
                                        <span class=cls>{label}</span>
                                    </div>
                                    {current.captura.then(|| view! {
                                        <a href=format!("/api/reportes/{cur}/captura") target="_blank" rel="noopener" class="block overflow-hidden rounded-lg border border-base-300">
                                            <img src=format!("/api/reportes/{cur}/captura") alt="Captura del problema" loading="lazy" class="max-h-72 w-full bg-base-200 object-contain"/>
                                        </a>
                                    })}
                                    <p class="whitespace-pre-wrap text-sm">{current.descripcion.clone()}</p>
                                    <dl class="grid grid-cols-[auto_1fr] gap-x-3 gap-y-1 text-xs text-base-content/70">
                                        <dt class="font-semibold">"Página"</dt><dd class="break-all">{current.pagina.clone()}</dd>
                                        <dt class="font-semibold">"De"</dt><dd class="break-all">{who}</dd>
                                        <dt class="font-semibold">"Pantalla"</dt><dd>{current.pantalla.clone()}</dd>
                                        <dt class="font-semibold">"Navegador"</dt><dd class="break-all">{current.navegador.clone()}</dd>
                                        <dt class="font-semibold">"Recibido"</dt><dd>{current.hace.clone()}</dd>
                                    </dl>
                                </section>
                            </div>
                            <div class="flex flex-wrap items-center gap-2 border-t border-base-300 p-4" role="group" aria-label="Cambiar el estado del reporte">
                                <span class="mr-1 text-sm text-base-content/70">"Estado"</span>
                                {buttons}
                                {move || problem.get().map(|p| view! { <crate::account::FieldNote ok=false text=p/> })}
                                <div class="ml-auto flex items-center gap-3">
                                    <button type="button" class="btn btn-sm btn-primary" on:click=copy.clone()>
                                        <ui::Icon name="copy" class="h-4 w-4"/>"Copiar reporte"
                                    </button>
                                </div>
                            </div>
                        }.into_any()
                    }
                }}
            </div>
            <form method="dialog" class="modal-backdrop"><button>"Cerrar"</button></form>
        </dialog>
    }
}
