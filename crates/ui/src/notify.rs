//! Avisos de toda la app: un solo lugar para «Producto guardado», «Banner
//! borrado · Deshacer», etc., con la campana que guarda los últimos.
//!
//! - [`Notifier`] vive en el contexto de `App` (ver [`provide_notifier`]);
//!   cualquier pantalla lo pide con [`use_notifier`] y llama a `success`,
//!   `error` o `info`.
//! - [`NoticeHost`] dibuja los avisos que están a la vista (abajo a la
//!   izquierda). Es un `popover="manual"`: queda en la capa superior, por
//!   encima de cualquier ventana.
//! - [`NoticeBell`] es la campana con la lista de los últimos 20 avisos.
//!
//! Los errores de un campo NO van acá: siguen siendo el texto rojo chico
//! debajo del campo.

use std::time::Duration;

use leptos::prelude::*;
use leptos::wasm_bindgen::{JsCast, JsValue};

use crate::Icon;

#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
const STORAGE_KEY: &str = "mk-notices";
const MAX_RECORDS: usize = 20;
const MAX_TOASTS: usize = 3;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum NoticeKind {
    Success,
    Error,
    Info,
}

impl NoticeKind {
    #[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
    fn key(self) -> &'static str {
        match self {
            Self::Success => "ok",
            Self::Error => "error",
            Self::Info => "info",
        }
    }

    #[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
    fn from_key(key: &str) -> Self {
        match key {
            "ok" => Self::Success,
            "error" => Self::Error,
            _ => Self::Info,
        }
    }

    fn icon(self) -> &'static str {
        match self {
            Self::Success => "check",
            Self::Error => "alert-circle",
            Self::Info => "info",
        }
    }

    fn tone(self) -> &'static str {
        match self {
            Self::Success => "text-success",
            Self::Error => "text-error",
            Self::Info => "text-base-content/60",
        }
    }
}

/// Un botón de texto dentro del aviso (por ejemplo «Deshacer»). Solo atajo de
/// algo que ya se puede hacer en otro lado de la pantalla.
#[derive(Clone)]
pub struct NoticeAction {
    label: String,
    run: Callback<()>,
}

impl NoticeAction {
    pub fn new(label: impl Into<String>, run: Callback<()>) -> Self {
        Self {
            label: label.into(),
            run,
        }
    }
}

/// Un aviso a la vista.
#[derive(Clone)]
struct Toast {
    id: u64,
    kind: NoticeKind,
    title: String,
    body: String,
    action: Option<NoticeAction>,
    /// Está saliendo: se anima la salida y después se quita.
    closing: RwSignal<bool>,
}

/// Un aviso en la lista de la campana.
#[derive(Clone, PartialEq, Debug)]
struct Record {
    id: u64,
    kind: NoticeKind,
    title: String,
    body: String,
    at: f64,
    unread: bool,
}

/// Quién muestra y recuerda los avisos. Es `Copy`: se pasa libremente.
#[derive(Clone, Copy)]
pub struct Notifier {
    toasts: RwSignal<Vec<Toast>>,
    records: RwSignal<Vec<Record>>,
    next: StoredValue<u64>,
    /// El mouse o el foco están sobre los avisos: no se van mientras tanto.
    holding: StoredValue<bool>,
    polite: RwSignal<String>,
    assertive: RwSignal<String>,
    /// Estamos dentro del panel (con menú al costado): los avisos se corren
    /// para no tapar el menú.
    panel: RwSignal<bool>,
}

impl Notifier {
    fn new() -> Self {
        let records = load_records();
        let next = records.iter().map(|r| r.id).max().map_or(1, |m| m + 1);
        Self {
            toasts: RwSignal::new(Vec::new()),
            records: RwSignal::new(records),
            next: StoredValue::new(next),
            holding: StoredValue::new(false),
            polite: RwSignal::new(String::new()),
            assertive: RwSignal::new(String::new()),
            panel: RwSignal::new(false),
        }
    }

    /// Algo salió bien: «Producto guardado».
    pub fn success(&self, title: impl Into<String>, body: impl Into<String>) -> u64 {
        self.push(NoticeKind::Success, title.into(), body.into(), None)
    }

    /// Algo no salió: se queda un poco más y queda marcado como sin leer.
    pub fn error(&self, title: impl Into<String>, body: impl Into<String>) -> u64 {
        self.push(NoticeKind::Error, title.into(), body.into(), None)
    }

    /// Información que no es ni éxito ni falla.
    pub fn info(&self, title: impl Into<String>, body: impl Into<String>) -> u64 {
        self.push(NoticeKind::Info, title.into(), body.into(), None)
    }

    /// Muestra un aviso y lo anota en la campana. Devuelve su número para
    /// poder quitarlo con [`Notifier::dismiss_now`].
    pub fn push(
        &self,
        kind: NoticeKind,
        title: String,
        body: String,
        action: Option<NoticeAction>,
    ) -> u64 {
        let id = self.next.get_value();
        self.next.set_value(id + 1);
        let long = kind == NoticeKind::Error || action.is_some();
        self.records.update(|list| {
            list.insert(
                0,
                Record {
                    id,
                    kind,
                    title: title.clone(),
                    body: body.clone(),
                    at: now_ms(),
                    // Solo lo que falló pide atención en la campana.
                    unread: kind == NoticeKind::Error,
                },
            );
            list.truncate(MAX_RECORDS);
        });
        save_records(&self.records.get_untracked());
        let said = if body.is_empty() {
            title.clone()
        } else {
            format!("{title}. {body}")
        };
        let said = if action.is_some() {
            format!("{said}. Acciones disponibles.")
        } else {
            said
        };
        // Vaciar primero para que se repita aunque el texto sea igual.
        let target = if kind == NoticeKind::Error {
            self.assertive
        } else {
            self.polite
        };
        target.set(String::new());
        set_timeout(move || target.set(said), Duration::from_millis(50));
        self.toasts.update(|list| {
            list.push(Toast {
                id,
                kind,
                title,
                body,
                action,
                closing: RwSignal::new(false),
            });
            while list.len() > MAX_TOASTS {
                list.remove(0);
            }
        });
        self.hide_later(id, if long { 9_000 } else { 6_000 });
        id
    }

    /// A los `ms` se va solo, salvo que el mouse o el foco estén sobre los
    /// avisos: en ese caso se vuelve a mirar a los 3 segundos.
    fn hide_later(&self, id: u64, ms: u64) {
        let me = *self;
        set_timeout(
            move || {
                if me.holding.get_value() {
                    me.hide_later(id, 3_000);
                } else {
                    me.dismiss(id);
                }
            },
            Duration::from_millis(ms),
        );
    }

    /// Cierra un aviso con animación de salida.
    pub fn dismiss(&self, id: u64) {
        let Some(closing) = self
            .toasts
            .with_untracked(|l| l.iter().find(|t| t.id == id).map(|t| t.closing))
        else {
            return;
        };
        if closing.get_untracked() {
            return;
        }
        closing.set(true);
        let toasts = self.toasts;
        set_timeout(
            move || toasts.update(|l| l.retain(|t| t.id != id)),
            Duration::from_millis(220),
        );
    }

    /// Lo quita ya, sin animación (lo que decía dejó de ser cierto).
    pub fn dismiss_now(&self, id: u64) {
        self.toasts.update(|l| l.retain(|t| t.id != id));
    }

    /// Dentro del panel los avisos se corren a la derecha del menú.
    pub fn set_in_panel(&self, on: bool) {
        self.panel.set(on);
    }

    fn unread(&self) -> usize {
        self.records.with(|l| l.iter().filter(|r| r.unread).count())
    }

    fn mark_all_read(&self) {
        let any = self.records.with_untracked(|l| l.iter().any(|r| r.unread));
        if any {
            self.records
                .update(|l| l.iter_mut().for_each(|r| r.unread = false));
            save_records(&self.records.get_untracked());
        }
    }

    fn clear(&self) {
        self.records.set(Vec::new());
        save_records(&[]);
    }
}

/// Crea el [`Notifier`] de la app. Se llama una sola vez, en `App`.
pub fn provide_notifier() {
    provide_context(Notifier::new());
}

/// El [`Notifier`] de la app. Fuera de `App` (pruebas) devuelve uno suelto
/// que no se dibuja en ningún lado.
pub fn use_notifier() -> Notifier {
    use_context::<Notifier>().unwrap_or_else(Notifier::new)
}

// ---------------------------------------------------------------------------
// Persistencia y reloj (solo en el navegador).

fn now_ms() -> f64 {
    js_sys::Date::now()
}

fn load_records() -> Vec<Record> {
    #[cfg(target_arch = "wasm32")]
    {
        let raw = leptos::prelude::window()
            .local_storage()
            .ok()
            .flatten()
            .and_then(|s| s.get_item(STORAGE_KEY).ok().flatten());
        let Some(raw) = raw else { return Vec::new() };
        let Ok(serde_json::Value::Array(items)) = serde_json::from_str(&raw) else {
            return Vec::new();
        };
        items
            .iter()
            .filter_map(|v| {
                Some(Record {
                    id: v.get("i")?.as_u64()?,
                    kind: NoticeKind::from_key(v.get("k")?.as_str()?),
                    title: v.get("t")?.as_str()?.to_string(),
                    body: v.get("b")?.as_str()?.to_string(),
                    at: v.get("a")?.as_f64()?,
                    unread: v.get("u")?.as_bool()?,
                })
            })
            .take(MAX_RECORDS)
            .collect()
    }
    #[cfg(not(target_arch = "wasm32"))]
    Vec::new()
}

fn save_records(_records: &[Record]) {
    #[cfg(target_arch = "wasm32")]
    {
        let items: Vec<_> = _records
            .iter()
            .map(|r| {
                serde_json::json!({
                    "i": r.id, "k": r.kind.key(), "t": r.title,
                    "b": r.body, "a": r.at, "u": r.unread,
                })
            })
            .collect();
        if let Some(s) = leptos::prelude::window().local_storage().ok().flatten() {
            _ = s.set_item(STORAGE_KEY, &serde_json::Value::Array(items).to_string());
        }
    }
}

/// Días desde 1970 (en la hora local) de un instante.
fn local_day(ms: f64) -> i64 {
    let d = js_sys::Date::new(&JsValue::from_f64(ms));
    // Medianoche local de ese día, expresada como si fuera UTC.
    let midnight = js_sys::Date::new_with_year_month_day(
        d.get_full_year(),
        d.get_month() as i32,
        d.get_date() as i32,
    );
    let offset_ms = midnight.get_timezone_offset() * 60_000.0;
    ((midnight.get_time() - offset_ms) / 86_400_000.0).round() as i64
}

fn clock(ms: f64) -> String {
    let d = js_sys::Date::new(&JsValue::from_f64(ms));
    format!("{:02}:{:02}", d.get_hours(), d.get_minutes())
}

/// «ahora», «hace 5 min», «10:42», «ayer», «hace 3 días».
fn ago(at: f64) -> String {
    let now = now_ms();
    let secs = ((now - at) / 1000.0).max(0.0);
    if secs < 60.0 {
        return "ahora".into();
    }
    if secs < 3600.0 {
        return format!("hace {} min", (secs / 60.0).floor() as i64);
    }
    match local_day(now) - local_day(at) {
        0 => clock(at),
        1 => "ayer".into(),
        n => format!("hace {n} días"),
    }
}

fn group_of(at: f64) -> u8 {
    match local_day(now_ms()) - local_day(at) {
        0 => 0,
        1 => 1,
        _ => 2,
    }
}

fn call_method(el: &JsValue, name: &str) {
    if let Ok(f) = js_sys::Reflect::get(el, &JsValue::from_str(name)) {
        if let Some(f) = f.dyn_ref::<js_sys::Function>() {
            _ = f.call0(el);
        }
    }
}

fn has_method(el: &JsValue, name: &str) -> bool {
    js_sys::Reflect::get(el, &JsValue::from_str(name))
        .map(|f| f.is_function())
        .unwrap_or(false)
}

// ---------------------------------------------------------------------------
// Los avisos a la vista.

/// Dibuja los avisos que están a la vista. Va una sola vez, en `App`.
#[component]
pub fn NoticeHost() -> impl IntoView {
    let n = use_notifier();
    let host = NodeRef::<leptos::html::Div>::new();
    // De dónde venía el foco antes de entrar a un aviso, para devolvérselo.
    // Va en un `thread_local` y no en un `StoredValue::new_local`: ese envuelve
    // el valor en un `SendWrapper` que hace panic si el servidor lo suelta en
    // otro hilo (tumbaba peticiones con un 502). Solo se usa en el navegador.
    thread_local! {
        static BACK: std::cell::RefCell<Option<leptos::web_sys::HtmlElement>> =
            const { std::cell::RefCell::new(None) };
    }

    // Capa superior: queda por encima de cualquier ventana abierta. Se vuelve
    // a pedir con cada aviso nuevo para quedar sobre lo último que se abrió.
    Effect::new(move |_| {
        let any = n.toasts.with(|l| !l.is_empty());
        let Some(el) = host.get() else { return };
        let js: &JsValue = el.as_ref();
        if !el.has_attribute("popover") {
            if !has_method(js, "showPopover") {
                return;
            }
            _ = el.set_attribute("popover", "manual");
        }
        call_method(js, "hidePopover");
        if any {
            call_method(js, "showPopover");
        }
    });

    let focus_after_close = move || {
        let Some(root) = host.get_untracked() else {
            return;
        };
        let next = root
            .query_selector(".mk-notice:not([data-closing]) [data-close]")
            .ok()
            .flatten()
            .and_then(|e| e.dyn_into::<leptos::web_sys::HtmlElement>().ok());
        let target = next.or_else(|| BACK.with_borrow(Clone::clone));
        if let Some(el) = target {
            _ = el.focus();
        }
    };

    view! {
        <div node_ref=host data-theme="light" class="mk-notices" role="region" aria-label="Avisos"
            data-panel=move || n.panel.get().then_some("")
            on:pointerenter=move |_| n.holding.set_value(true)
            on:pointerleave=move |_| n.holding.set_value(false)
            on:focusin=move |ev: leptos::ev::FocusEvent| {
                n.holding.set_value(true);
                let from = ev.related_target().and_then(|t| t.dyn_into::<leptos::web_sys::HtmlElement>().ok());
                let inside = from.as_ref().zip(host.get_untracked()).is_some_and(|(f, h)| h.contains(Some(f)));
                if !inside {
                    BACK.set(from);
                }
            }
            on:focusout=move |_| n.holding.set_value(false)>
            <div class="sr-only" role="status">{move || n.polite.get()}</div>
            <div class="sr-only" role="alert">{move || n.assertive.get()}</div>
            <For each=move || n.toasts.get() key=|t| t.id let:t>
                {
                    let id = t.id;
                    let closing = t.closing;
                    let action = t.action.clone();
                    view! {
                        <div class="mk-notice flex items-start gap-3 rounded-box border border-base-300 bg-base-100 p-3 pr-2 text-sm text-base-content shadow-xl"
                            data-closing=move || closing.get().then_some("")>
                            <Icon name=t.kind.icon() class=format!("mt-0.5 size-[18px] shrink-0 {}", t.kind.tone())/>
                            <div class="min-w-0 grow">
                                <p class="font-semibold">{t.title.clone()}</p>
                                {(!t.body.is_empty()).then(|| view! { <p class="mt-0.5 text-xs text-base-content/70">{t.body.clone()}</p> })}
                                {action.map(|a| {
                                    let run = a.run;
                                    view! {
                                        <button type="button" class="link link-primary mt-1.5 text-xs font-semibold"
                                            on:click=move |_| {
                                                run.run(());
                                                n.dismiss(id);
                                                focus_after_close();
                                            }>{a.label}</button>
                                    }
                                })}
                            </div>
                            <button type="button" data-close class="btn btn-ghost btn-xs btn-square" aria-label="Cerrar aviso"
                                on:click=move |_| {
                                    n.dismiss(id);
                                    focus_after_close();
                                }>
                                <Icon name="close" class="size-4"/>
                            </button>
                        </div>
                    }
                }
            </For>
        </div>
    }
}

// ---------------------------------------------------------------------------
// La campana.

/// Campana con la lista de los últimos avisos. `id` identifica la lista (si
/// hay dos campanas en pantalla, cada una con el suyo).
#[component]
pub fn NoticeBell(#[prop(into)] id: String, #[prop(into, optional)] class: String) -> impl IntoView {
    let n = use_notifier();
    let panel = NodeRef::<leptos::html::Div>::new();
    // Hasta que carga el navegador no se sabe qué hay guardado: la cuenta
    // aparece recién ahí, así el HTML del servidor coincide.
    let mounted = RwSignal::new(false);
    Effect::new(move |_| mounted.set(true));
    let opened = RwSignal::new(false);
    let count = move || {
        if mounted.get() {
            n.unread()
        } else {
            0
        }
    };

    // La lista se ubica justo debajo de la campana, sin salirse de la pantalla.
    let place = move |ev: leptos::ev::MouseEvent| {
        let Some(btn) = ev
            .current_target()
            .and_then(|t| t.dyn_into::<leptos::web_sys::Element>().ok())
        else {
            return;
        };
        let Some(panel) = panel.get_untracked() else {
            return;
        };
        let r = btn.get_bounding_client_rect();
        let vw = window()
            .inner_width()
            .ok()
            .and_then(|v| v.as_f64())
            .unwrap_or(1024.0);
        let width = 352.0_f64.min(vw - 16.0);
        let left = r.left().min(vw - width - 8.0).max(8.0);
        let style = leptos::web_sys::HtmlElement::style(&panel);
        _ = style.set_property("top", &format!("{}px", r.bottom() + 8.0));
        _ = style.set_property("left", &format!("{left}px"));
    };

    let on_toggle = move |ev: leptos::web_sys::Event| {
        let open = js_sys::Reflect::get(&ev, &JsValue::from_str("newState"))
            .ok()
            .and_then(|v| v.as_string())
            .is_some_and(|s| s == "open");
        if open {
            opened.set(true);
        } else if opened.get_untracked() {
            opened.set(false);
            // Lo que había sin leer se da por visto al cerrar.
            n.mark_all_read();
        }
    };

    let group_rows = move |g: u8| {
        n.records.with(|l| {
            l.iter()
                .filter(|r| group_of(r.at) == g)
                .cloned()
                .collect::<Vec<_>>()
        })
    };
    let section = move |g: u8, title: &'static str| {
        let rows = group_rows(g);
        (!rows.is_empty()).then(|| {
            view! {
                <li class="px-4 pt-3 pb-1 text-[0.7rem] font-semibold tracking-wider text-base-content/60">{title}</li>
                {rows.into_iter().map(|r| view! {
                    <li class="relative flex items-start gap-3 border-t border-base-200 px-4 py-2.5 first-of-type:border-t-0">
                        {r.unread.then(|| view! { <span class="absolute top-4 left-1.5 size-1.5 rounded-full bg-primary" aria-label="Sin leer"></span> })}
                        <Icon name=r.kind.icon() class=format!("mt-0.5 size-4 shrink-0 {}", r.kind.tone())/>
                        <div class="min-w-0 grow">
                            <p class="text-sm font-semibold">{r.title}</p>
                            {(!r.body.is_empty()).then(|| view! { <p class="text-xs text-base-content/70">{r.body}</p> })}
                        </div>
                        <span class="shrink-0 text-xs text-base-content/60">{ago(r.at)}</span>
                    </li>
                }).collect_view()}
            }
        })
    };

    let panel_id = id.clone();
    view! {
        <button type="button" popovertarget=id class=format!("btn btn-ghost btn-square btn-sm relative {class}")
            aria-label="Notificaciones" aria-haspopup="dialog" on:click=place>
            <Icon name="bell" class="size-5"/>
            {move || (count() > 0).then(|| view! {
                <span class="badge badge-primary badge-xs absolute -top-1 -right-1" aria-label=format!("{} sin leer", count())>{count()}</span>
            })}
        </button>
        <div node_ref=panel id=panel_id popover="auto" data-theme="light" class="mk-bellpanel" role="dialog" aria-label="Notificaciones"
            on:toggle=on_toggle>
            <div class="flex items-center justify-between gap-3 border-b border-base-300 px-4 py-3">
                <h2 class="font-bold">"Notificaciones"</h2>
                <button type="button" class="link link-primary text-xs disabled:opacity-40 disabled:no-underline"
                    disabled=move || n.unread() == 0 on:click=move |_| n.mark_all_read()>
                    "Marcar como leídas"
                </button>
            </div>
            {move || mounted.get().then(|| view! {
                <Show when=move || n.records.with(|l| !l.is_empty())
                    fallback=|| view! { <p class="px-4 py-8 text-center text-sm text-base-content/60">"Todavía no hay notificaciones."</p> }>
                    <ul>
                        {move || section(0, "HOY")}
                        {move || section(1, "AYER")}
                        {move || section(2, "ANTERIORES")}
                    </ul>
                    <div class="border-t border-base-300 px-4 py-2.5 text-center">
                        <button type="button" class="link text-xs text-base-content/70" on:click=move |_| n.clear()>"Borrar historial"</button>
                    </div>
                </Show>
            })}
        </div>
    }
}
