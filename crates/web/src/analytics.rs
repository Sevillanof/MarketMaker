//! Uso y abandono: dónde se traba la gente. Los eventos son anónimos (un
//! código al azar por visita y por día; sin nombre, email, IP ni nada de lo que
//! se escribió) y el panel es solo para la cuenta administradora.

use leptos::prelude::*;
use leptos_router::hooks::{use_location, use_query_map};
use serde::{Deserialize, Serialize};

/// Pasos que cuentan como del dueño (se guardan así aunque lleguen desde la
/// dirección de una tienda, como el editor).
#[cfg(feature = "ssr")]
const OWNER_STEPS: &[&str] = &[
    "inicio", "entrar", "panel", "productos", "pedidos", "cobros", "ventas", "datos", "editor",
    "cuenta", "fotos", "tienda_creada", "publico",
];
/// Pasos del comprador.
#[cfg(feature = "ssr")]
const BUYER_STEPS: &[&str] = &[
    "tienda", "producto", "carrito", "comprar", "entrega", "pago", "pedido", "error_compra",
];

/// Lo corto, para la columna "Llegó hasta".
#[cfg(feature = "ssr")]
fn short(step: &str) -> &'static str {
    match step {
        "inicio" => "Inicio",
        "entrar" => "Entrar",
        "panel" => "Panel",
        "productos" => "Productos",
        "pedidos" => "Pedidos",
        "cobros" => "Cobros",
        "ventas" => "Ventas",
        "datos" => "Datos de la tienda",
        "editor" => "Editor",
        "cuenta" => "Mi cuenta",
        "fotos" => "Mis fotos",
        "tienda_creada" => "Creó tienda",
        "publico" => "Publicó",
        "tienda" => "Tienda",
        "producto" => "Producto",
        "carrito" => "Carrito",
        "comprar" => "Datos",
        "entrega" => "Entrega",
        "pago" => "Pago",
        "pedido" => "Pedido hecho",
        "error_compra" => "Error al comprar",
        _ => "Otro",
    }
}

/// Lo largo, para el paso a paso de una visita.
#[cfg(feature = "ssr")]
fn long(step: &str, detail: Option<&str>) -> String {
    let d = detail.unwrap_or("");
    match step {
        "inicio" => "Vio la página de inicio".into(),
        "entrar" => "Fue a Entrar".into(),
        "panel" => "Abrió el panel".into(),
        "productos" => "Fue a Productos".into(),
        "pedidos" => "Fue a Pedidos".into(),
        "cobros" => "Fue a Cobros".into(),
        "ventas" => "Fue a Ventas".into(),
        "datos" => "Fue a Datos de la tienda".into(),
        "editor" => "Abrió el editor".into(),
        "cuenta" => "Fue a Mi cuenta".into(),
        "fotos" => "Fue a Mis fotos".into(),
        "tienda_creada" => "Creó su tienda".into(),
        "publico" => "Publicó la tienda".into(),
        "tienda" => "Entró a la tienda".into(),
        "producto" => "Abrió un producto".into(),
        "carrito" => "Agregó al carrito".into(),
        "comprar" => "Empezó la compra".into(),
        "entrega" => "Pasó al paso Entrega".into(),
        "pago" => "Pasó al paso Pago".into(),
        "pedido" => "Hizo el pedido".into(),
        "error_compra" if d.is_empty() => "Vio un error en el formulario".into(),
        "error_compra" => format!("Vio un error en el campo «{d}»"),
        _ => "Otro paso".into(),
    }
}

/// Un paso es un problema (se marca en rojo en el recorrido).
#[cfg(feature = "ssr")]
fn is_problem(step: &str) -> bool {
    step == "error_compra"
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct VisitItem {
    pub code: String,
    pub last: String,
    pub time: String,
    pub device: String,
    pub done: bool,
    pub ago: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
pub struct UsageSummary {
    pub owner: Vec<i64>,
    pub buyer: Vec<i64>,
    pub visits: Vec<VisitItem>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct StepItem {
    pub at: String,
    pub text: String,
    pub problem: bool,
}

#[cfg(feature = "ssr")]
fn clean_detail(d: &str) -> Option<String> {
    let d: String = d
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '_')
        .take(30)
        .collect();
    (!d.is_empty()).then_some(d)
}

#[cfg(feature = "ssr")]
fn is_bot(ua: &str) -> bool {
    let ua = ua.to_lowercase();
    ua.is_empty()
        || ["bot", "crawl", "spider", "headless", "preview", "monitor", "curl"]
            .iter()
            .any(|w| ua.contains(w))
}

/// Guarda un paso de la visita actual. No guarda nada si la persona pidió que
/// no la sigan (DNT / GPC), si es un robot o si es la cuenta administradora.
#[cfg(feature = "ssr")]
pub(crate) async fn record_event(step: &str, detail: Option<&str>) {
    let Some(parts) = use_context::<http::request::Parts>() else {
        return;
    };
    let header = |k: &str| {
        parts
            .headers
            .get(k)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_string()
    };
    if header("dnt") == "1" || header("sec-gpc") == "1" {
        return;
    }
    let ua = header("user-agent");
    if is_bot(&ua) {
        return;
    }
    let who = if OWNER_STEPS.contains(&step) {
        "owner"
    } else if BUYER_STEPS.contains(&step) {
        "buyer"
    } else {
        return;
    };
    let store = match crate::site::current_site() {
        domain::Site::Store(slug) => Some(slug),
        domain::Site::Platform => None,
    };
    let Ok(pool) = crate::server::pool() else {
        return;
    };
    if let Ok(Some(user)) = crate::auth::current_user().await {
        if crate::auth::is_admin(&user.email) {
            return;
        }
    }
    let device = if ua.contains("Mobi") || ua.contains("Android") || ua.contains("iPhone") {
        "celular"
    } else {
        "escritorio"
    };
    let visit = db::analytics::visit_code(&crate::limits::client_ip(), &ua);
    let detail = detail.and_then(clean_detail);
    let _ = db::analytics::record(
        &pool,
        &visit,
        who,
        step,
        store.as_deref(),
        device,
        detail.as_deref(),
    )
    .await;
}

/// El navegador avisa que la persona llegó a un paso.
#[server]
pub async fn track(step: String, detail: Option<String>) -> Result<(), ServerFnError> {
    record_event(&step, detail.as_deref()).await;
    Ok(())
}

/// Avisa un paso sin esperar la respuesta ni mostrar errores.
pub fn track_client(step: &'static str, detail: Option<String>) {
    leptos::task::spawn_local(async move {
        let _ = track(step.to_string(), detail).await;
    });
}

/// Mira cada página que se abre y la anota como un paso.
#[component]
pub fn TrackPages() -> impl IntoView {
    let path = use_location().pathname;
    Effect::new(move |_| {
        let p = path.get();
        let store = matches!(crate::site::current_site(), domain::Site::Store(_));
        let seg: Vec<&str> = p.trim_matches('/').split('/').collect();
        let step = if store {
            match seg[0] {
                "comprar" => Some("comprar"),
                "pedido" => Some("pedido"),
                "editor" => Some("editor"),
                "terminos" | "privacidad" | "arrepentimiento" | "entrar" | "cuenta" => None,
                _ => Some("tienda"),
            }
        } else {
            match (seg[0], seg.get(2).copied()) {
                ("", _) => Some("inicio"),
                ("entrar", _) => Some("entrar"),
                ("tienda", None) => Some("panel"),
                ("tienda", Some("productos")) => Some("productos"),
                ("tienda", Some("pedidos")) => Some("pedidos"),
                ("tienda", Some("cobros")) => Some("cobros"),
                ("tienda", Some("ventas")) => Some("ventas"),
                ("tienda", Some("datos")) => Some("datos"),
                ("editor", _) => Some("editor"),
                ("cuenta", _) => Some("cuenta"),
                ("mis-fotos", _) => Some("fotos"),
                _ => None,
            }
        };
        if let Some(step) = step {
            track_client(step, None);
        }
    });
}

#[cfg(feature = "ssr")]
async fn require_admin() -> Result<(), ServerFnError> {
    match crate::auth::current_user().await? {
        Some(u) if crate::auth::is_admin(&u.email) => Ok(()),
        _ => Err(ServerFnError::new("Solo el administrador puede ver esta página.")),
    }
}

#[cfg(feature = "ssr")]
fn duration_es(s: i64) -> String {
    match s {
        i64::MIN..=59 => format!("{} s", s.max(0)),
        60..=3599 => format!("{} min", s / 60),
        _ => format!("{} h {} min", s / 3600, (s % 3600) / 60),
    }
}

/// Todo lo del panel: embudos y las últimas visitas de `who`.
#[server]
pub async fn usage_report(
    days: i32,
    who: String,
    only_dropped: bool,
) -> Result<UsageSummary, ServerFnError> {
    use crate::server::{internal, pool};
    require_admin().await?;
    let pool = pool()?;
    let days = days.clamp(1, db::analytics::KEEP_DAYS);
    let who = if who == "owner" { "owner" } else { "buyer" };
    let owner = db::analytics::owner_funnel(&pool, days).await.map_err(internal)?;
    let buyer = db::analytics::buyer_funnel(&pool, days).await.map_err(internal)?;
    let visits = db::analytics::recent_visits(&pool, who, days, only_dropped, 40)
        .await
        .map_err(internal)?
        .into_iter()
        .map(|v| VisitItem {
            code: v.visita,
            last: short(&v.ultimo).to_string(),
            time: duration_es(v.segundos),
            device: if v.dispositivo == "celular" { "Celular" } else { "Escritorio" }.into(),
            done: v.termino,
            ago: domain::ago_es(v.hace),
        })
        .collect();
    Ok(UsageSummary { owner, buyer, visits })
}

/// El paso a paso de una visita.
#[server]
pub async fn usage_visit(code: String, who: String) -> Result<Vec<StepItem>, ServerFnError> {
    use crate::server::{internal, pool};
    require_admin().await?;
    let who = if who == "owner" { "owner" } else { "buyer" };
    let rows = db::analytics::visit_steps(&pool()?, &code, who)
        .await
        .map_err(internal)?;
    Ok(rows
        .into_iter()
        .map(|(s, step, detail)| StepItem {
            at: format!("{}:{:02}", s / 60, s % 60),
            problem: is_problem(&step),
            text: long(&step, detail.as_deref()),
        })
        .collect())
}

const OWNER_LABELS: [&str; 7] = [
    "Cuenta creada",
    "Creó su tienda",
    "Guardó cambios en el editor",
    "Cargó un producto",
    "Publicó la tienda",
    "Configuró cobros",
    "Hizo una venta",
];
const BUYER_LABELS: [&str; 7] = [
    "Visitó la tienda",
    "Abrió un producto",
    "Agregó al carrito",
    "Empezó la compra",
    "Pasó a Entrega",
    "Pasó a Pago",
    "Pedidos pagados",
];

fn href(days: i32, who: &str, dropped: bool, visit: &str) -> String {
    let mut u = format!("/admin/uso?dias={days}&quien={who}");
    if dropped {
        u.push_str("&abandonaron=1");
    }
    if !visit.is_empty() {
        u.push_str(&format!("&v={visit}"));
    }
    u
}

#[component]
fn Funnel(title: &'static str, note: &'static str, labels: [&'static str; 7], values: Vec<i64>) -> impl IntoView {
    let first = values.first().copied().unwrap_or(0);
    let rows = labels
        .iter()
        .enumerate()
        .map(|(i, label)| {
            let n = values.get(i).copied().unwrap_or(0);
            let prev = if i == 0 { n } else { values.get(i - 1).copied().unwrap_or(0) };
            let pct = if first > 0 { n * 100 / first } else { 0 };
            let lost = prev - n;
            let leak = i > 0 && prev >= 5 && lost * 100 / prev.max(1) >= 30;
            view! {
                <li class="grid grid-cols-[minmax(0,9.5rem)_1fr_auto] items-center gap-x-3 gap-y-0.5 py-1">
                    <span class="text-sm">{*label}</span>
                    <span class="h-3 overflow-hidden rounded bg-base-200" aria-hidden="true">
                        <span class="block h-full rounded bg-primary" style=format!("width:{pct}%")></span>
                    </span>
                    <span class="text-sm tabular-nums"><b>{n}</b>{(i > 0 && first > 0).then(|| format!(" · {pct}%"))}</span>
                    {leak.then(|| view! {
                        <span class="col-start-2 col-span-2 text-xs text-error">{format!("Acá se pierden {lost} personas")}</span>
                    })}
                </li>
            }
        })
        .collect_view();
    view! {
        <section class="card border border-base-300 bg-base-100"><div class="card-body gap-1 p-4">
            <h2 class="font-semibold">{title}</h2>
            <p class="text-xs text-base-content/60">{note}</p>
            {if first == 0 {
                view! { <p class="py-4 text-sm text-base-content/60">"Todavía no hay datos en este período."</p> }.into_any()
            } else {
                view! { <ol class="mt-1">{rows}</ol> }.into_any()
            }}
        </div></section>
    }
}

/// `/admin/uso`: embudos y recorridos anónimos. Solo el administrador.
#[component]
pub fn UsagePage() -> impl IntoView {
    if !matches!(crate::site::current_site(), domain::Site::Platform) {
        return view! { <crate::platform::NotAStore/> }.into_any();
    }
    let q = use_query_map();
    let get = move |k: &str| q.with(|m| m.get(k).unwrap_or_default());
    let days = move || match get("dias").as_str() {
        "7" => 7,
        "90" => 90,
        _ => 30,
    };
    let who = move || if get("quien") == "owner" { "owner" } else { "buyer" };
    let dropped = move || get("abandonaron") == "1";
    let visit = move || get("v");
    let report = Resource::new(move || (days(), who(), dropped()), |(d, w, a)| usage_report(d, w.to_string(), a));
    let steps = Resource::new(move || (visit(), who()), |(v, w)| async move {
        if v.is_empty() { Ok(Vec::new()) } else { usage_visit(v, w.to_string()).await }
    });
    let pill = |active: bool| if active { "btn btn-sm btn-primary" } else { "btn btn-sm btn-outline" };
    view! {
        <leptos_meta::Title text="Uso y abandono · MarketMaker"/>
        <div data-theme="light" class="min-h-screen bg-base-200 text-base-content">
            <header class="flex items-center gap-3 border-b border-base-300 bg-base-100 px-6 py-3">
                <a href="/" class="text-lg"><crate::account::Logo/></a>
                <span class="text-sm text-base-content/60">"Administración"</span>
            </header>
            <main class="mx-auto flex max-w-6xl flex-col gap-5 px-4 py-8">
                <div>
                    <h1 class="text-2xl font-bold">"Uso y abandono"</h1>
                    <p class="text-sm text-base-content/70">"Dónde se traba la gente. Cada visita es un código al azar: no se guarda ningún dato personal."</p>
                </div>
                <nav class="flex flex-wrap gap-2" aria-label="Período">
                    {[7, 30, 90].map(|d| view! {
                        <a class=move || pill(days() == d) href=move || href(d, who(), dropped(), "")
                            aria-current=move || (days() == d).then_some("true")>{format!("{d} días")}</a>
                    })}
                </nav>
                <Suspense fallback=|| view! { <ui::PageSkeleton/> }>
                    {move || Suspend::new(async move {
                        match report.await {
                            Err(_) => view! {
                                <p class="text-sm text-base-content/70">"Esta página es solo para la cuenta administradora."</p>
                            }.into_any(),
                            Ok(r) => view! {
                                <div class="grid gap-4 md:grid-cols-2">
                                    <Funnel title="Dueños: de la cuenta a la primera venta"
                                        note="Cuentas nuevas de este período y hasta dónde llegaron."
                                        labels=OWNER_LABELS values=r.owner/>
                                    <Funnel title="Compradores: de la visita al pago"
                                        note="Visitas a las tiendas y hasta qué paso llegaron."
                                        labels=BUYER_LABELS values=r.buyer/>
                                </div>
                                <div class="grid items-start gap-4 lg:grid-cols-[3fr_2fr]">
                                    <section class="card border border-base-300 bg-base-100"><div class="card-body gap-3 p-4">
                                        <h2 class="font-semibold" id="visitas-titulo">"Visitas anónimas"</h2>
                                        <div class="flex flex-wrap gap-2">
                                            <a class=move || pill(who() == "owner") href=move || href(days(), "owner", dropped(), "")>"Dueños"</a>
                                            <a class=move || pill(who() == "buyer") href=move || href(days(), "buyer", dropped(), "")>"Compradores"</a>
                                            <a class=move || pill(dropped()) href=move || href(days(), who(), !dropped(), "")
                                                aria-pressed=move || dropped().to_string()>"Solo las que abandonaron"</a>
                                        </div>
                                        {if r.visits.is_empty() {
                                            view! { <p class="py-4 text-sm text-base-content/60">"Todavía no hay visitas para mostrar."</p> }.into_any()
                                        } else {
                                            view! {
                                                <div tabindex="0" role="region" aria-labelledby="visitas-titulo" class="max-h-[28rem] overflow-auto">
                                                    <table class="table table-xs">
                                                        <thead class="sticky top-0 bg-base-100"><tr>
                                                            <th>"Visita"</th><th>"Llegó hasta"</th><th>"Tiempo"</th><th>"Dispositivo"</th><th>"Cuándo"</th><th>"Estado"</th>
                                                        </tr></thead>
                                                        <tbody>
                                                            {r.visits.into_iter().map(|v| {
                                                                let code = v.code.clone();
                                                                let code2 = v.code.clone();
                                                                view! {
                                                                    <tr class=move || if visit() == code { "bg-base-200" } else { "" }>
                                                                        <th class="whitespace-nowrap font-mono font-medium">
                                                                            <a class="link link-hover" href=move || href(days(), who(), dropped(), &code2)>{v.code}</a>
                                                                        </th>
                                                                        <td class="whitespace-nowrap">{v.last}</td>
                                                                        <td class="whitespace-nowrap">{v.time}</td>
                                                                        <td class="whitespace-nowrap">{v.device}</td>
                                                                        <td class="whitespace-nowrap text-base-content/60">{v.ago}</td>
                                                                        <td class="whitespace-nowrap">
                                                                            {if v.done {
                                                                                view! { <span class="badge badge-success badge-soft badge-sm">"Completó"</span> }.into_any()
                                                                            } else {
                                                                                view! { <span class="badge badge-warning badge-soft badge-sm">"Abandonó"</span> }.into_any()
                                                                            }}
                                                                        </td>
                                                                    </tr>
                                                                }
                                                            }).collect_view()}
                                                        </tbody>
                                                    </table>
                                                </div>
                                            }.into_any()
                                        }}
                                    </div></section>
                                    <section class="card border border-base-300 bg-base-100"><div class="card-body gap-2 p-4" aria-live="polite">
                                        <Suspense fallback=|| ()>
                                            {move || Suspend::new(async move {
                                                let list = steps.await.unwrap_or_default();
                                                if visit().is_empty() || list.is_empty() {
                                                    view! {
                                                        <h2 class="font-semibold">"Paso a paso"</h2>
                                                        <p class="text-sm text-base-content/60">"Elegí una visita para ver su recorrido."</p>
                                                    }.into_any()
                                                } else {
                                                    view! {
                                                        <h2 class="font-semibold">{format!("Visita {}, paso a paso", visit())}</h2>
                                                        <ol class="text-sm">
                                                            {list.into_iter().map(|s| view! {
                                                                <li class=format!("ml-1 flex gap-3 border-l-2 py-1 pl-3 {}", if s.problem { "border-error" } else { "border-base-300" })>
                                                                    <b class="tabular-nums">{s.at}</b><span>{s.text}</span>
                                                                </li>
                                                            }).collect_view()}
                                                        </ol>
                                                    }.into_any()
                                                }
                                            })}
                                        </Suspense>
                                    </div></section>
                                </div>
                            }.into_any(),
                        }
                    })}
                </Suspense>
                <p class="border-t border-base-300 pt-3 text-xs text-base-content/60">
                    "Privacidad: no se guardan nombres, emails, IP ni textos escritos. El código de cada visita cambia todos los días y los detalles se borran a los 90 días. No se guarda nada en el navegador de las personas y se respeta la opción de no ser seguidas (DNT/GPC)."
                </p>
            </main>
        </div>
    }.into_any()
}
