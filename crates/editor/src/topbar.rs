//! Piezas de la barra superior del editor: el menú "Mi tienda" (todo lo que
//! vale para la tienda entera) y el selector de la página que se edita.

use domain::{StoreLayout, MARGINS, THEMES, THEME_LABELS, WIDTH_PRESETS};
use leptos::prelude::*;
use wasm_bindgen::JsCast;

use crate::icons::Icon;
use crate::typography::StoreTypographyFields;
use crate::{choice_class, Actions, EditorState};

/// Id del panel emergente de "Mi tienda".
const PANEL_ID: &str = "mk-store-panel";

/// Cierra el panel de "Mi tienda", si está abierto.
fn hide_panel() {
    if let Some(el) = document()
        .get_element_by_id(PANEL_ID)
        .and_then(|e| e.dyn_into::<web_sys::HtmlElement>().ok())
    {
        _ = el.hide_popover();
    }
}

/// Botón "Mi tienda" y su panel con los ajustes de toda la tienda: colores,
/// letras, ficha del producto y el enlace a la tienda publicada.
///
/// Usa la API `popover` del navegador: se cierra con Esc o con un clic
/// afuera, y devuelve el foco al botón.
#[component]
pub(crate) fn StoreMenu() -> impl IntoView {
    let state = expect_context::<EditorState>();
    let actions = expect_context::<Actions>();
    let theme = move || state.doc.with(|d| d.theme.clone());
    let live_href = move || {
        state
            .doc
            .with(|d| d.page_path(state.page.get()).to_string())
    };

    view! {
        <button
            type="button"
            class="btn h-11 gap-2.5 border-base-300 bg-base-100 pl-1.5 pr-2.5 font-normal shadow-none hover:border-primary"
            popovertarget=PANEL_ID
            aria-haspopup="dialog"
            title="Colores, letras, márgenes y ficha de toda la tienda"
        >
            <span data-theme=theme class="flex h-8 w-8 items-center justify-center gap-0.5 rounded-field border border-base-content/15 bg-base-100" aria-hidden="true">
                <span class="h-1.5 w-1.5 rounded-full bg-primary"></span>
                <span class="h-1.5 w-1.5 rounded-full bg-secondary"></span>
                <span class="h-1.5 w-1.5 rounded-full bg-accent"></span>
            </span>
            <span class="flex flex-col items-start leading-tight">
                <span class="text-[0.9375rem] font-bold">"Mi tienda"</span>
                <span class="text-xs text-base-content/75">"Colores, letras y márgenes"</span>
            </span>
            <Icon name="chevron-down" class="h-3.5 w-3.5"/>
        </button>
        <div
            id=PANEL_ID
            popover="auto"
            role="dialog"
            aria-labelledby="mk-store-title"
            class="mk-popover mk-store-panel w-[40rem] rounded-box border border-base-300 bg-base-100 p-5 text-base-content shadow-xl"
        >
            <div class="flex items-start justify-between gap-3">
                <div>
                    <h2 id="mk-store-title" class="text-lg font-bold">"Ajustes de toda la tienda"</h2>
                    <p class="text-sm text-base-content/75">"Valen para todas las páginas. Cada bloque se puede cambiar aparte desde el panel de la derecha."</p>
                </div>
                <button
                    type="button"
                    class="btn btn-ghost btn-sm btn-square"
                    popovertarget=PANEL_ID
                    popovertargetaction="hide"
                    aria-label="Cerrar"
                >
                    <Icon name="close"/>
                </button>
            </div>

            <section class="mt-4" aria-labelledby="mk-colors-title">
                <h3 id="mk-colors-title" class="mb-2 text-xs font-semibold uppercase tracking-wide text-base-content/75">"Colores"</h3>
                <div class="grid grid-cols-6 gap-2">
                    {THEMES.iter().zip(THEME_LABELS).map(|(t, l)| {
                        let active = move || state.doc.with(|d| d.theme == *t);
                        view! {
                            <button
                                type="button"
                                class=move || if active() {
                                    "flex flex-col gap-1.5 rounded-field border-2 border-primary bg-primary/10 p-1.5 text-left text-sm font-medium"
                                } else {
                                    "flex flex-col gap-1.5 rounded-field border-2 border-base-300 p-1.5 text-left text-sm font-medium transition-colors hover:border-primary"
                                }
                                aria-pressed=move || active().to_string()
                                on:click=move |_| state.apply(|d| d.theme = (*t).to_string())
                            >
                                <span data-theme=*t class="flex h-11 items-end gap-1 rounded-field border border-base-content/10 bg-base-100 p-2" aria-hidden="true">
                                    <span class="h-3 w-3 rounded-full bg-primary"></span>
                                    <span class="h-3 w-3 rounded-full bg-secondary"></span>
                                    <span class="h-3 w-3 rounded-full bg-accent"></span>
                                </span>
                                <span class="flex items-center justify-between gap-1 px-0.5">
                                    {*l}
                                    {move || active().then(|| view! { <Icon name="check" class="h-3.5 w-3.5 text-primary"/> })}
                                </span>
                            </button>
                        }
                    }).collect_view()}
                </div>
            </section>

            <div class="mt-5">
                <StoreTypographyFields/>
            </div>

            <div class="mt-5">
                <StoreLayoutFields/>
            </div>

            <div class="mt-5 flex gap-2 border-t border-base-300 pt-4">
                <button
                    type="button"
                    class="btn btn-sm flex-1 justify-start"
                    on:click=move |_| {
                        hide_panel();
                        actions.open_modal(None);
                    }
                >
                    <Icon name="modal"/>
                    "Editar la ficha del producto"
                </button>
                <a class="btn btn-sm flex-1 justify-start" href=live_href target="_blank" rel="noopener">
                    <Icon name="external"/>
                    "Ver tienda publicada"
                    <span class="sr-only">"(se abre en otra pestaña)"</span>
                </a>
            </div>
        </div>
    }
}

/// Margen (en px de la miniatura) con que se dibuja cada opción de
/// [`MARGINS`]: exagerado, para que la diferencia se vea en chico.
fn thumb_pad(px: u16) -> u16 {
    match px {
        0..=8 => 3,
        9..=24 => 7,
        _ => 16,
    }
}

/// Márgenes de toda la tienda (tres opciones con dibujo) y ancho máximo del
/// contenido. Va dentro del menú "Mi tienda". El ancho también se cambia
/// arrastrando los bordes de la página en el lienzo.
#[component]
fn StoreLayoutFields() -> impl IntoView {
    let state = expect_context::<EditorState>();
    let layout = Memo::new(move |_| state.doc.with(|d| d.layout.clone()));
    let set = move |f: Box<dyn FnOnce(&mut StoreLayout)>| state.apply(|d| f(&mut d.layout));
    // Un ancho que no es de la lista (se llegó arrastrando los bordes).
    let custom = move || {
        layout.with(|l| {
            (!WIDTH_PRESETS.iter().any(|(w, _)| *w == l.max_width)).then_some(l.max_width)
        })
    };
    let card = "flex flex-col gap-1.5 rounded-field border-2 p-1.5 text-left text-sm";

    view! {
        <section aria-labelledby="mk-layout-title">
            <div class="mb-2 flex items-baseline justify-between gap-3">
                <h3 id="mk-layout-title" class="text-xs font-semibold uppercase tracking-wide text-base-content/75">"Márgenes y ancho"</h3>
                <p class="text-xs text-base-content/75">"También podés arrastrar los bordes de la página."</p>
            </div>
            <div class="grid grid-cols-3 gap-2" role="group" aria-label="Margen alrededor de la tienda">
                {MARGINS.iter().map(|(id, name, px, hint)| {
                    let active = move || layout.with(|l| l.margin == *id);
                    let pad = thumb_pad(*px);
                    view! {
                        <button
                            type="button"
                            class=move || if active() {
                                format!("{card} border-primary bg-primary/10")
                            } else {
                                format!("{card} border-base-300 transition-colors hover:border-primary")
                            }
                            aria-pressed=move || active().to_string()
                            on:click=move |_| set(Box::new(move |l| l.margin = (*id).into()))
                        >
                            <span class="mk-margin-thumb" style=format!("padding:{pad}px") aria-hidden="true">
                                <span class="mk-mt-bar h-2"></span>
                                <span class="mk-mt-bar h-5"></span>
                                <span class="mk-mt-row">
                                    <span class="mk-mt-bar h-4"></span>
                                    <span class="mk-mt-bar h-4"></span>
                                    <span class="mk-mt-bar h-4"></span>
                                </span>
                            </span>
                            <span class="flex items-center justify-between gap-1 px-0.5">
                                <span class="font-medium">{*name}</span>
                                {move || active().then(|| view! { <Icon name="check" class="h-3.5 w-3.5 text-primary"/> })}
                            </span>
                            <span class="px-0.5 text-xs text-base-content/75">{*hint}</span>
                        </button>
                    }
                }).collect_view()}
            </div>
            <div class="mt-3 flex flex-wrap items-center gap-x-3 gap-y-2">
                <span id="mk-width-title" class="text-sm font-medium">"Ancho máximo"</span>
                <div class="join" role="group" aria-labelledby="mk-width-title">
                    {WIDTH_PRESETS.iter().map(|(w, name)| {
                        let active = move || layout.with(|l| l.max_width == *w);
                        let title = match *w {
                            0 => "Ocupa toda la pantalla, de borde a borde",
                            1120 => "Como viene",
                            _ => "En pantallas más anchas, la tienda queda centrada",
                        };
                        view! {
                            <button
                                type="button"
                                class=move || format!("{} join-item", choice_class(active()))
                                aria-pressed=move || active().to_string()
                                title=title
                                on:click=move |_| set(Box::new(move |l| l.max_width = *w))
                            >{*name}</button>
                        }
                    }).collect_view()}
                    {move || custom().map(|w| view! {
                        <span class="btn btn-sm join-item pointer-events-none border-primary bg-primary/10 text-primary" title="El ancho que elegiste arrastrando los bordes">{format!("{w} px")}</span>
                    })}
                </div>
            </div>
        </section>
    }
}

/// Selector de la página que se está editando, con su dirección y el botón
/// para borrarla (Inicio no se borra).
#[component]
pub(crate) fn PageSwitcher() -> impl IntoView {
    let state = expect_context::<EditorState>();
    let actions = expect_context::<Actions>();
    let pages = move || {
        state.doc.with(|d| {
            (0..d.page_count())
                .map(|i| (i, d.page_name(i).to_string()))
                .collect::<Vec<_>>()
        })
    };
    let hint = move || {
        if state.doc.with(|d| d.pages.is_empty()) {
            "Tocá un enlace del menú superior (Ofertas, Categorías…) para crear y editar esa página."
        } else {
            "Tocá un enlace del menú para ir a esa página, o elegila acá."
        }
    };

    view! {
        <div class="flex items-center gap-1" role="group" aria-label="Páginas de la tienda">
            <label class="select select-sm h-9 w-48 gap-2" title=hint>
                <Icon name="page" class="h-4 w-4 text-base-content/70"/>
                <span class="sr-only">"Página"</span>
                <select
                    id="mk-page-select"
                    class="font-semibold"
                    prop:value=move || state.page.get().to_string()
                    on:change=move |ev| {
                        if let Ok(i) = event_target_value(&ev).parse::<usize>() {
                            actions.open_page(i, true);
                        }
                    }
                >
                    {move || pages()
                        .into_iter()
                        .map(|(i, name)| view! { <option value=i.to_string() selected=move || state.page.get() == i>{name}</option> })
                        .collect_view()}
                </select>
            </label>
            <code class="px-1.5 text-xs text-base-content/75" title="Dirección de la página en tu tienda">
                {move || state.doc.with(|d| d.page_path(state.page.get()).to_string())}
            </code>
            {move || (state.page.get() > 0).then(|| view! {
                <button
                    type="button"
                    class="btn btn-ghost btn-sm btn-square text-red-700"
                    aria-label="Borrar esta página"
                    title="Borrar esta página"
                    on:click=move |_| actions.delete_page(state.page.get_untracked())
                >
                    <Icon name="trash"/>
                </button>
            })}
        </div>
    }
}
