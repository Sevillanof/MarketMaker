//! Cajón "Mi tienda": los ajustes de toda la tienda en la columna de la
//! izquierda (en lugar de la lista de bloques).
//!
//! Todo lo que se ve se puede tocar: cada opción es un dibujito de cómo
//! queda, y el cambio se ve al instante en el lienzo. Sin textos de ayuda ni
//! resúmenes; las opciones nuevas se suman como una sección más.

use domain::{
    font_css, Side, StoreLayout, FONT_PAIRS, MAX_PAD, PAD_STEP, STORE_SIZES, WIDTH_PRESETS,
};
use leptos::prelude::*;

use crate::icons::Icon;
use crate::theme_panel::ThemePanel;
use crate::{Actions, EditorState, Ui};

/// Id del botón "Mi tienda" de la barra: vuelve el foco ahí al cerrar.
pub(crate) const STORE_BUTTON_ID: &str = "mk-store-button";

/// Clase de una opción con dibujito.
pub(crate) fn tile(active: bool) -> &'static str {
    if active {
        "flex flex-col items-center gap-1 rounded-field border-2 border-primary bg-primary/10 p-1.5 text-primary"
    } else {
        "flex flex-col items-center gap-1 rounded-field border-2 border-base-300 p-1.5 text-base-content/80 transition-colors hover:border-primary"
    }
}

/// Título de una sección.
#[component]
pub(crate) fn Section(title: &'static str, id: &'static str, children: Children) -> impl IntoView {
    view! {
        <section class="mt-3" aria-labelledby=id>
            <h3 id=id class="mb-1.5 text-[11px] font-semibold uppercase tracking-wide text-base-content/70">{title}</h3>
            {children()}
        </section>
    }
}

/// Cierra el cajón y vuelve el foco al botón "Mi tienda".
pub(crate) fn close_drawer(ui: Ui) {
    ui.store_open.set(false);
    if let Some(b) = document().get_element_by_id(STORE_BUTTON_ID) {
        use wasm_bindgen::JsCast;
        if let Ok(b) = b.dyn_into::<web_sys::HtmlElement>() {
            _ = b.focus();
        }
    }
}

#[component]
pub(crate) fn StoreDrawer() -> impl IntoView {
    let ui = expect_context::<Ui>();
    view! {
        <aside
            class="flex w-80 shrink-0 flex-col border-r border-base-300 bg-base-100"
            aria-labelledby="mk-drawer-title"
            on:keydown=move |ev| {
                if ev.key() == "Escape" {
                    ev.prevent_default();
                    close_drawer(ui);
                }
            }
        >
            <div class="flex h-12 shrink-0 items-center gap-2 border-b border-base-300 px-4">
                <h2 id="mk-drawer-title" tabindex="-1" class="font-bold focus:outline-none">"Mi tienda"</h2>
                <button
                    type="button"
                    class="btn btn-ghost btn-sm btn-square ml-auto"
                    aria-label="Cerrar Mi tienda"
                    title="Cerrar (Esc)"
                    on:click=move |_| close_drawer(ui)
                >
                    <Icon name="close"/>
                </button>
            </div>
            <div class="flex-1 overflow-y-auto px-3 pb-4">
                <ThemePanel/>
                <div class="mt-5 border-t border-base-300"></div>
                <Letters/>
                <Margins/>
                <Width/>
                <ProductCard/>
            </div>
        </aside>
    }
}

#[component]
fn Letters() -> impl IntoView {
    let state = expect_context::<EditorState>();
    let typo = Memo::new(move |_| state.doc.with(|d| d.typography.clone()));
    view! {
        <Section title="Letras" id="mk-dr-letters">
            <div class="grid grid-cols-2 gap-1" role="group" aria-label="Fuentes">
                {FONT_PAIRS.iter().map(|p| {
                    let active = move || typo.with(|t| t.heading == p.heading && t.body == p.body);
                    view! {
                        <button
                            type="button"
                            class="mk-chip"
                            aria-pressed=move || active().to_string()
                            on:click=move |_| state.apply(|d| {
                                d.typography.heading = p.heading.into();
                                d.typography.body = p.body.into();
                            })
                        >
                            <span class="text-base font-semibold leading-none" style=format!("font-family:{}", font_css(p.heading)) aria-hidden="true">"Aa"</span>
                            <span class="min-w-0 truncate text-[11px] font-medium">{p.label}</span>
                        </button>
                    }
                }).collect_view()}
            </div>
            <div class="mk-seg mt-2" role="group" aria-label="Tamaño del texto">
                {STORE_SIZES.iter().map(|(id, name, _)| {
                    let active = move || typo.with(|t| t.size == *id);
                    view! {
                        <button
                            type="button"
                            class:is-active=active
                            aria-pressed=move || active().to_string()
                            on:click=move |_| state.apply(|d| d.typography.size = (*id).into())
                        >{*name}</button>
                    }
                }).collect_view()}
            </div>
        </Section>
    }
}

/// Dibujito de una página: barra, banner y tarjetas, con el margen y el
/// ancho de cada opción.
fn page_icon(pad_px: u8, width_pct: u8) -> impl IntoView {
    let bar = move |h: &'static str, tone: &'static str| {
        view! { <span class=format!("mx-auto {h} rounded-sm {tone}") style=format!("width:{width_pct}%")></span> }
    };
    view! {
        <span
            class="flex h-9 w-full flex-col gap-[2px] rounded-[3px] border border-base-content/15 bg-base-200"
            style=format!("padding:3px {pad_px}px")
            aria-hidden="true"
        >
            {bar("h-1.5", "bg-primary/60")}
            {bar("h-3", "bg-primary/40")}
            {bar("h-2", "bg-primary/25")}
        </span>
    }
}

/// Margen de cada lado: un cuadro con la página en el medio y la medida de
/// cada lado alrededor. Con "Iguales" los cuatro van juntos; con "Cada
/// lado" se elige uno tocando su medida y se cambia con − y +.
#[component]
fn Margins() -> impl IntoView {
    let state = expect_context::<EditorState>();
    let layout = Memo::new(move |_| state.doc.with(|d| d.layout.clone()));
    let linked = RwSignal::new(layout.with_untracked(StoreLayout::pads_equal));
    let side = RwSignal::new(Side::Left);
    // El lado que se cambia con − y + (con "Iguales", todos: se muestra Arriba).
    let shown = move || if linked.get() { Side::Top } else { side.get() };
    let value = move || layout.with(|l| l.pad(shown()));
    let set = move |px: i32| {
        let (all, s) = (linked.get_untracked(), side.get_untracked());
        let group = if all {
            "pad-all".to_string()
        } else {
            format!("pad-{s:?}")
        };
        state.apply_grouped(Some(&group), |d| {
            if all {
                d.layout.set_all_pads(px);
            } else {
                d.layout.set_pad(s, px);
            }
        });
    };
    let side_btn = move |s: Side, place: &'static str| {
        let active = move || linked.get() || side.get() == s;
        view! {
            <button
                type="button"
                class=move || format!(
                    "mk-side absolute {place} {}",
                    if active() { "is-active" } else { "" }
                )
                aria-pressed=move || active().to_string()
                aria-label=move || format!("{}: {} px", s.label(), layout.with(|l| l.pad(s)))
                title=s.label()
                on:click=move |_| {
                    if !linked.get_untracked() {
                        side.set(s);
                    }
                }
            >
                {move || layout.with(|l| l.pad(s))}<small>"px"</small>
            </button>
        }
    };

    view! {
        <Section title="Márgenes" id="mk-dr-margins">
            <div class="relative h-[8.25rem] rounded-field border-2 border-dashed border-primary/40 bg-primary/5">
                <div class="absolute inset-x-16 inset-y-9 flex flex-col gap-1 rounded-[3px] border border-base-content/15 bg-base-100 p-1.5" aria-hidden="true">
                    <span class="h-2 rounded-sm bg-primary/60"></span>
                    <span class="h-5 rounded-sm bg-primary/40"></span>
                    <span class="h-3 rounded-sm bg-primary/25"></span>
                </div>
                {side_btn(Side::Top, "left-1/2 top-1.5 -translate-x-1/2")}
                {side_btn(Side::Bottom, "left-1/2 bottom-1.5 -translate-x-1/2")}
                {side_btn(Side::Left, "left-1.5 top-1/2 -translate-y-1/2")}
                {side_btn(Side::Right, "right-1.5 top-1/2 -translate-y-1/2")}
            </div>
            <div class="mt-1.5 flex items-center gap-1.5">
                <div class="mk-seg flex-1" role="group" aria-label="Márgenes iguales o por lado">
                    <button
                        type="button"
                        class:is-active=move || linked.get()
                        aria-pressed=move || linked.get().to_string()
                        on:click=move |_| {
                            if !linked.get_untracked() {
                                let px = layout.with_untracked(|l| l.pad(side.get_untracked()));
                                linked.set(true);
                                state.apply(|d| d.layout.set_all_pads(i32::from(px)));
                            }
                        }
                    >"Iguales"</button>
                    <button
                        type="button"
                        class:is-active=move || !linked.get()
                        aria-pressed=move || (!linked.get()).to_string()
                        on:click=move |_| linked.set(false)
                    >"Cada lado"</button>
                </div>
                <div class="join" role="group" aria-label=move || format!("Margen: {}", if linked.get() { "todos los lados" } else { shown().label() })>
                    <button
                        type="button"
                        class="btn btn-sm join-item px-2"
                        aria-label="Menos margen"
                        disabled=move || value() == 0
                        on:click=move |_| set(i32::from(value()) - i32::from(PAD_STEP))
                    >"−"</button>
                    <label class="join-item flex h-8 items-center gap-1 border-y border-base-300 px-1.5">
                        <span class="text-[11px] text-primary">{move || if linked.get() { "Todos" } else { shown().short() }}</span>
                        <input
                            type="number"
                            min="0"
                            max=MAX_PAD
                            step=PAD_STEP
                            class="w-9 bg-transparent text-center text-sm font-semibold [appearance:textfield] focus:outline-none [&::-webkit-inner-spin-button]:appearance-none"
                            prop:value=move || value().to_string()
                            on:change=move |ev| {
                                if let Ok(px) = event_target_value(&ev).trim().parse::<i32>() {
                                    set(px);
                                }
                            }
                        />
                    </label>
                    <button
                        type="button"
                        class="btn btn-sm join-item px-2"
                        aria-label="Más margen"
                        disabled=move || MAX_PAD <= value()
                        on:click=move |_| set(i32::from(value()) + i32::from(PAD_STEP))
                    >"+"</button>
                </div>
            </div>
        </Section>
    }
}

#[component]
fn Width() -> impl IntoView {
    let state = expect_context::<EditorState>();
    let max_width = Memo::new(move |_| state.doc.with(|d| d.layout.max_width));
    // Un ancho que no es de la lista (se llegó arrastrando los bordes).
    let custom = move || {
        let w = max_width.get();
        (!WIDTH_PRESETS.iter().any(|(p, _)| *p == w)).then_some(w)
    };
    view! {
        <Section title="Ancho" id="mk-dr-width">
            <div class="grid grid-cols-4 gap-1.5" role="group" aria-labelledby="mk-dr-width">
                {WIDTH_PRESETS.iter().map(|(w, name)| {
                    let active = move || max_width.get() == *w;
                    let pct = match *w { 0 => 100, 1440 => 88, 1120 => 72, _ => 58 };
                    view! {
                        <button
                            type="button"
                            class=move || tile(active())
                            aria-pressed=move || active().to_string()
                            title=if *w == 0 { "Ocupa toda la pantalla" } else { "Ancho máximo del contenido, en píxeles" }
                            on:click=move |_| state.apply(|d| d.layout.max_width = *w)
                        >
                            {page_icon(3, pct)}
                            <span class="text-[11px] font-medium">{*name}</span>
                        </button>
                    }
                }).collect_view()}
            </div>
            {move || custom().map(|w| view! {
                <p class="mt-1.5 flex items-center gap-1.5 rounded-field border-2 border-primary bg-primary/10 px-2 py-1 text-xs font-medium text-primary">
                    <Icon name="check" class="h-3.5 w-3.5"/>
                    {format!("{w} px, elegido arrastrando los bordes")}
                </p>
            })}
        </Section>
    }
}

#[component]
fn ProductCard() -> impl IntoView {
    let ui = expect_context::<Ui>();
    let actions = expect_context::<Actions>();
    view! {
        <button
            type="button"
            class="mt-4 flex w-full items-center gap-3 rounded-field border-2 border-base-300 p-2 text-left transition-colors hover:border-primary"
            on:click=move |_| {
                ui.store_open.set(false);
                actions.open_modal(None);
            }
        >
            <span class="flex h-11 w-14 shrink-0 gap-1 rounded-[3px] border border-base-content/15 bg-base-100 p-1" aria-hidden="true">
                <span class="w-1/2 rounded-sm bg-primary/40"></span>
                <span class="flex flex-1 flex-col gap-1">
                    <span class="h-1.5 rounded-sm bg-base-content/30"></span>
                    <span class="h-1.5 w-2/3 rounded-sm bg-primary/50"></span>
                    <span class="mt-auto h-2.5 rounded-sm bg-primary"></span>
                </span>
            </span>
            <span class="text-sm font-semibold">"Ficha del producto"</span>
            <Icon name="pencil" class="ml-auto h-4 w-4 text-base-content/60"/>
        </button>
    }
}
