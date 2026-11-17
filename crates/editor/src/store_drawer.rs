//! Cajón "Mi tienda": los ajustes de toda la tienda en la columna de la
//! izquierda (en lugar de la lista de bloques).
//!
//! Todo lo que se ve se puede tocar: cada opción es un dibujito de cómo
//! queda, y el cambio se ve al instante en el lienzo. Sin textos de ayuda ni
//! resúmenes; las opciones nuevas se suman como una sección más.

use domain::{font_css, Side, StoreLayout, FONT_PAIRS, MAX_PAD, PAD_STEP, STORE_SIZES};
use leptos::prelude::*;

use crate::icons::Icon;
use crate::theme_panel::ThemePanel;
use crate::{EditorState, Ui};

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
            </div>
        </aside>
    }
}

/// Letras de la tienda: un desplegable con las tipografías (cada opción se
/// ve en su letra) y el tamaño del texto.
#[component]
pub(crate) fn Letters() -> impl IntoView {
    let state = expect_context::<EditorState>();
    let typo = Memo::new(move |_| state.doc.with(|d| d.typography.clone()));
    let current = move || {
        typo.with(|t| {
            FONT_PAIRS
                .iter()
                .position(|p| p.heading == t.heading && p.body == t.body)
        })
    };
    view! {
        <Section title="Letras" id="mk-dr-letters">
            <select
                class="mk-fontselect"
                aria-labelledby="mk-dr-letters"
                style=move || current().map(|i| format!("font-family:{}", font_css(FONT_PAIRS[i].heading))).unwrap_or_default()
                on:change=move |ev| {
                    if let Ok(i) = event_target_value(&ev).parse::<usize>() {
                        if let Some(p) = FONT_PAIRS.get(i) {
                            state.apply(|d| {
                                d.typography.heading = p.heading.into();
                                d.typography.body = p.body.into();
                            });
                        }
                    }
                }
            >
                {FONT_PAIRS.iter().enumerate().map(|(i, p)| view! {
                    <option
                        value=i.to_string()
                        selected=move || current() == Some(i)
                        style=format!("font-family:{}", font_css(p.heading))
                    >{p.label}</option>
                }).collect_view()}
            </select>
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

/// Margen de cada lado (entre el borde de la pantalla y los bloques).
/// Con "Iguales" un solo deslizador mueve los cuatro; con "Cada lado" hay un
/// campo por lado.
#[component]
pub(crate) fn Margins() -> impl IntoView {
    let state = expect_context::<EditorState>();
    let layout = Memo::new(move |_| state.doc.with(|d| d.layout.clone()));
    let linked = RwSignal::new(layout.with_untracked(StoreLayout::pads_equal));
    let set_all = move |px: i32| {
        state.apply_grouped(Some("pad-all"), |d| d.layout.set_all_pads(px));
    };
    view! {
        <div class="mt-2.5">
            <p class="mb-1 flex items-baseline gap-1.5">
                <span class="text-xs font-semibold" id="mk-dr-margins">"Márgenes"</span>
                <span class="text-[10.5px] text-base-content/55">"borde de la pantalla"</span>
            </p>
            <div class="mk-seg" role="group" aria-labelledby="mk-dr-margins">
                <button
                    type="button"
                    class:is-active=move || linked.get()
                    aria-pressed=move || linked.get().to_string()
                    on:click=move |_| {
                        if !linked.get_untracked() {
                            let px = layout.with_untracked(|l| l.pad_top);
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
            <Show
                when=move || linked.get()
                fallback=move || view! {
                    <div class="mt-1.5 grid grid-cols-4 gap-1">
                        {Side::ALL.iter().map(|s| {
                            let s = *s;
                            view! {
                                <label class="min-w-0">
                                    <span class="block truncate text-[10px] text-base-content/60">{s.label()}</span>
                                    <input
                                        type="number"
                                        min="0"
                                        max=MAX_PAD
                                        step=PAD_STEP
                                        class="mk-num"
                                        prop:value=move || layout.with(|l| l.pad(s)).to_string()
                                        on:change=move |ev| {
                                            if let Ok(px) = event_target_value(&ev).trim().parse::<i32>() {
                                                state.apply(|d| d.layout.set_pad(s, px));
                                            }
                                        }
                                    />
                                </label>
                            }
                        }).collect_view()}
                    </div>
                }
            >
                <div class="mt-1.5 flex items-center gap-3 rounded-box bg-base-200/70 px-3 py-1.5">
                    <input
                        type="range"
                        min="0"
                        max=MAX_PAD
                        step=PAD_STEP
                        class="range range-xs min-w-0 flex-1"
                        aria-label="Margen de todos los lados"
                        aria-valuetext=move || format!("{} píxeles", layout.with(|l| l.pad_top))
                        prop:value=move || layout.with(|l| l.pad_top).to_string()
                        on:input=move |ev| {
                            if let Ok(px) = event_target_value(&ev).parse::<i32>() {
                                set_all(px);
                            }
                        }
                    />
                    <span class="w-12 shrink-0 text-right text-sm font-bold leading-none">
                        {move || layout.with(|l| l.pad_top)}
                        <span class="ml-0.5 text-[10px] font-semibold text-base-content/55">"px"</span>
                    </span>
                </div>
            </Show>
        </div>
    }
}
