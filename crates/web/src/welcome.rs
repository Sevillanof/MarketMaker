//! Bienvenida de la primera vez: globos que señalan, en el panel, dónde está
//! cada cosa (Productos, Cobros, Entregas, Diseño y los Primeros pasos).
//!
//! Aparece una sola vez por cuenta nueva (queda guardado en la cuenta, no en
//! el navegador) y solo en pantallas donde el menú está a la vista. Con
//! `?bienvenida=1` en la dirección se puede ver de nuevo sin gastar el aviso.
//!
//! Los globos son un diálogo sin bloqueo anclado al elemento real (`data-tour`),
//! con el foco puesto adentro y Esc para salir.

use crate::panel::Section;
use leptos::prelude::*;
use std::time::Duration;

/// Cada paso: el `data-tour` del elemento, el título y el texto.
const STEPS: [(&str, &str, &str); 5] = [
    (
        "Productos",
        "Acá cargás tus productos",
        "Cada producto lleva foto, nombre y precio.",
    ),
    (
        "Cobros",
        "Acá elegís cómo te pagan",
        "Mercado Pago, transferencia o coordinar con tu cliente.",
    ),
    (
        "Entregas",
        "Acá elegís cómo entregás",
        "Retiro en el local o envío con Andreani.",
    ),
    (
        "Diseño",
        "Acá armás tu tienda",
        "Elegís bloques, colores y textos, y después la publicás.",
    ),
    (
        "Primeros pasos",
        "Seguí estos pasos",
        "Te dejamos lo que falta para vender. Con \"Ver cómo\" ves cada paso en una película.",
    ),
];

type Rect = (f64, f64, f64, f64);

/// ¿Falta mostrarle la bienvenida a quien entró?
#[server]
pub async fn welcome_pending() -> Result<bool, ServerFnError> {
    use crate::server::{internal, pool};
    let Some(user) = crate::auth::current_user().await? else {
        return Ok(false);
    };
    db::auth::welcome_pending(&pool()?, user.id)
        .await
        .map_err(internal)
}

/// Anota que ya vio la bienvenida.
#[server]
pub async fn welcome_done() -> Result<(), ServerFnError> {
    use crate::server::{internal, pool};
    let Some(user) = crate::auth::current_user().await? else {
        return Ok(());
    };
    db::auth::mark_welcome_seen(&pool()?, user.id)
        .await
        .map_err(internal)
}

fn viewport() -> (f64, f64) {
    let w = window();
    let get = |v: Result<leptos::wasm_bindgen::JsValue, _>| {
        v.ok().and_then(|v| v.as_f64()).unwrap_or(0.0)
    };
    (get(w.inner_width()), get(w.inner_height()))
}

/// Dónde está en pantalla el elemento `data-tour`, si se ve.
fn target_rect(name: &str) -> Option<Rect> {
    let el = document()
        .query_selector(&format!("[data-tour=\"{name}\"]"))
        .ok()??;
    let r = el.get_bounding_client_rect();
    let (vw, vh) = viewport();
    (r.width() > 0.0
        && r.height() > 0.0
        && r.right() > 0.0
        && r.left() < vw
        && r.bottom() > 0.0
        && r.top() < vh)
        .then(|| (r.left(), r.top(), r.width(), r.height()))
}

/// Muestra el primer paso desde `from` cuyo elemento se ve. `false` si no queda ninguno.
fn show(from: usize, step: RwSignal<Option<usize>>, rect: RwSignal<Rect>) -> bool {
    for i in from..STEPS.len() {
        if let Ok(Some(el)) = document().query_selector(&format!("[data-tour=\"{}\"]", STEPS[i].0))
        {
            el.scroll_into_view();
        }
        if let Some(r) = target_rect(STEPS[i].0) {
            rect.set(r);
            step.set(Some(i));
            request_animation_frame(|| {
                if let Some(b) = document().get_element_by_id("tour-next") {
                    use leptos::wasm_bindgen::JsCast;
                    if let Ok(b) = b.dyn_into::<leptos::web_sys::HtmlElement>() {
                        _ = b.focus();
                    }
                }
            });
            return true;
        }
    }
    false
}

/// Espera a que el menú esté dibujado y arranca.
fn try_start(tries: u32, step: RwSignal<Option<usize>>, rect: RwSignal<Rect>) {
    if viewport().0 < 1024.0 {
        return;
    }
    if target_rect(STEPS[0].0).is_some() {
        show(0, step, rect);
    } else if tries < 20 {
        set_timeout(
            move || try_start(tries + 1, step, rect),
            Duration::from_millis(300),
        );
    }
}

#[component]
pub fn WelcomeTour(section: Signal<Section>) -> impl IntoView {
    let pending = Resource::new(|| (), |_| welcome_pending());
    let step = RwSignal::new(None::<usize>);
    let rect = RwSignal::new((0.0, 0.0, 0.0, 0.0));
    let started = RwSignal::new(false);
    let forced = RwSignal::new(false);

    // Arranca en el Inicio, cuando falta (o se pide con ?bienvenida=1).
    Effect::new(move |_| {
        let want = pending.get().and_then(|r| r.ok()).unwrap_or(false);
        let force = window()
            .location()
            .search()
            .unwrap_or_default()
            .contains("bienvenida=1");
        if (want || force) && section.get() == Section::Home && !started.get_untracked() {
            started.set(true);
            forced.set(!want);
            try_start(0, step, rect);
        }
    });

    let finish = move || {
        step.set(None);
        if !forced.get_untracked() {
            leptos::task::spawn_local(async {
                _ = welcome_done().await;
            });
        }
    };
    let next = move || {
        let i = step.get_untracked().map_or(0, |i| i + 1);
        if !show(i, step, rect) {
            finish();
        }
    };
    // Si cambia el tamaño de la ventana o se mueve la página, el globo sigue al elemento.
    let follow = move || {
        if let Some(i) = step.get_untracked() {
            if let Some(r) = target_rect(STEPS[i].0) {
                rect.set(r);
            }
        }
    };
    let _ = window_event_listener(leptos::ev::resize, move |_| follow());
    let _ = window_event_listener(leptos::ev::scroll, move |_| follow());

    view! {
        {move || step.get().map(|i| {
            let (x, y, w, h) = rect.get();
            let (vw, vh) = viewport();
            let (_, title, text) = STEPS[i];
            // Los elementos angostos (el menú) llevan el globo al costado; los anchos, abajo o arriba.
            let (tx, ty) = if w < 400.0 {
                ((x + w + 18.0).min(vw - 300.0).max(12.0), (y - 6.0).min(vh - 190.0).max(12.0))
            } else if y + h + 200.0 < vh {
                ((x + 16.0).min(vw - 300.0).max(12.0), y + h + 14.0)
            } else {
                ((x + 16.0).min(vw - 300.0).max(12.0), (y - 190.0).max(12.0))
            };
            let last = i + 1 == STEPS.len();
            view! {
                // Tapa la pantalla para que no se toque nada por detrás mientras dura.
                <div class="fixed inset-0 z-[70]" aria-hidden="true"></div>
                <div class="pointer-events-none fixed z-[71] rounded-box outline outline-[3px] outline-primary transition-all duration-500 motion-reduce:transition-none"
                    style=format!("left:{}px;top:{}px;width:{}px;height:{}px;box-shadow:0 0 0 100vmax rgb(15 17 30 / .55)", x - 6.0, y - 4.0, w + 12.0, h + 8.0)
                    aria-hidden="true"></div>
                <div role="dialog" aria-labelledby="tour-title" data-theme="light"
                    class="fixed z-[72] w-72 rounded-box bg-base-100 p-4 text-base-content shadow-2xl transition-all duration-500 motion-reduce:transition-none"
                    style=format!("left:{tx}px;top:{ty}px")
                    on:keydown=move |ev| {
                        if ev.key() == "Escape" {
                            ev.prevent_default();
                            finish();
                        }
                    }>
                    <h2 id="tour-title" class="text-base font-bold">{title}</h2>
                    <p class="mt-1 mb-3 text-sm text-base-content/70">{text}</p>
                    <div class="flex items-center gap-2">
                        <span class="grow text-xs text-base-content/60">{format!("{} de {}", i + 1, STEPS.len())}</span>
                        {(!last).then(|| view! {
                            <button type="button" class="btn btn-ghost btn-sm" on:click=move |_| finish()>"Saltar"</button>
                        })}
                        <button type="button" id="tour-next" class="btn btn-primary btn-sm" on:click=move |_| next()>
                            {if last { "Listo" } else { "Siguiente" }}
                        </button>
                    </div>
                </div>
            }
        })}
    }
}
