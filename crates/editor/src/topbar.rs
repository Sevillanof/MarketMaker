//! Piezas de la barra superior del editor: el menú "Mi tienda" (todo lo que
//! vale para la tienda entera) y el selector de la página que se edita.

use leptos::prelude::*;

use crate::icons::Icon;
use crate::store_drawer::{close_drawer, STORE_BUTTON_ID};
use crate::{Actions, EditorState, Ui};

/// Botón "Mi tienda": abre y cierra el cajón con los ajustes de toda la
/// tienda (ver [`crate::store_drawer`]), en lugar de la lista de bloques.
#[component]
pub(crate) fn StoreMenu() -> impl IntoView {
    let state = expect_context::<EditorState>();
    let ui = expect_context::<Ui>();
    let theme = move || state.doc.with(|d| d.theme.clone());
    let open = move || ui.store_open.get();

    view! {
        <button
            type="button"
            id=STORE_BUTTON_ID
            class=move || format!(
                "btn h-11 gap-2.5 pl-1.5 pr-2.5 font-normal shadow-none {}",
                if open() { "border-primary bg-primary/10" } else { "border-base-300 bg-base-100 hover:border-primary" }
            )
            aria-expanded=move || open().to_string()
            aria-controls="mk-drawer-title"
            title="Colores, letras, márgenes y más de toda la tienda"
            on:click=move |_| {
                if ui.store_open.get_untracked() {
                    close_drawer(ui);
                } else {
                    ui.preview.set(false);
                    ui.store_open.set(true);
                }
            }
        >
            <span data-theme=theme class="flex h-8 w-8 items-center justify-center gap-0.5 rounded-field border border-base-content/15 bg-base-100" aria-hidden="true">
                <span class="h-1.5 w-1.5 rounded-full bg-primary"></span>
                <span class="h-1.5 w-1.5 rounded-full bg-secondary"></span>
                <span class="h-1.5 w-1.5 rounded-full bg-accent"></span>
            </span>
            <span class="flex flex-col items-start leading-tight">
                <span class="text-[0.9375rem] font-bold">"Mi tienda"</span>
                <span class="text-xs text-base-content/75 max-[1699px]:hidden">"Ajustes de toda la tienda"</span>
            </span>
            {move || if open() {
                view! { <Icon name="close" class="h-3.5 w-3.5"/> }
            } else {
                view! { <Icon name="chevron-down" class="h-3.5 w-3.5"/> }
            }}
        </button>
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
            <label class="select select-sm h-9 w-48 gap-2 max-[1399px]:w-36" title=hint>
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
            <NewPageMenu/>
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

/// "+ Nueva página": nombre, dirección y con qué empieza (en blanco o una
/// de las páginas modelo), con el menú y el pie de la actual.
#[component]
fn NewPageMenu() -> impl IntoView {
    let actions = expect_context::<Actions>();
    let name = RwSignal::new(String::new());
    // `None`: en blanco; `Some(i)`: la página modelo `i`.
    let choice = RwSignal::new(None::<usize>);
    // Los avisos aparecen recién al tocar "Crear página".
    let tried = RwSignal::new(false);
    let problem = move || {
        name.track();
        actions.new_page_problem(&name.get())
    };
    let slug = move || {
        let s = domain::slug_from_name(&name.get());
        if s.is_empty() {
            "nombre-de-la-pagina".to_string()
        } else {
            s
        }
    };
    let menu = NodeRef::<leptos::html::Details>::new();
    // Se cierra al tocar afuera, como cualquier menú.
    let outside = window_event_listener(leptos::ev::pointerdown, move |ev| {
        use wasm_bindgen::JsCast;
        let Some(d) = menu.get_untracked() else {
            return;
        };
        let inside = ev
            .target()
            .and_then(|t| t.dyn_into::<web_sys::Node>().ok())
            .is_some_and(|t| d.contains(Some(&t)));
        if d.open() && !inside {
            d.set_open(false);
        }
    });
    on_cleanup(move || outside.remove());
    view! {
        <details node_ref=menu class="dropdown"
            // Y con Escape, devolviendo el foco al botón.
            on:keydown=move |ev| {
                if ev.key() == "Escape" {
                    if let Some(d) = menu.get_untracked() {
                        if d.open() {
                            ev.prevent_default();
                            ev.stop_propagation();
                            d.set_open(false);
                            if let Ok(Some(s)) = d.query_selector("summary") {
                                use wasm_bindgen::JsCast;
                                if let Ok(s) = s.dyn_into::<web_sys::HtmlElement>() {
                                    _ = s.focus();
                                }
                            }
                        }
                    }
                }
            }>
            <summary class="btn btn-ghost btn-sm gap-1" title="Crear una página nueva">
                <Icon name="plus" class="h-4 w-4"/><span class="sr-only">"Nueva página"</span>
            </summary>
            <form
                class="dropdown-content z-50 mt-1 w-80 space-y-3 rounded-box border border-base-300 bg-base-100 p-4 shadow-lg"
                on:submit=move |ev| {
                    ev.prevent_default();
                    tried.set(true);
                    if actions.add_named_page(&name.get_untracked(), choice.get_untracked()) {
                        if let Some(d) = menu.get_untracked() {
                            d.set_open(false);
                        }
                        name.set(String::new());
                        choice.set(None);
                        tried.set(false);
                    }
                }
            >
                <div>
                    <label class="mb-1 block text-sm font-medium" for="mk-new-page-name">"Nombre de la página"</label>
                    <input
                        id="mk-new-page-name"
                        type="text"
                        class="input input-sm w-full"
                        maxlength="40"
                        placeholder="Por ejemplo: Ofertas de verano"
                        aria-describedby="mk-new-page-hint"
                        prop:value=move || name.get()
                        on:input=move |ev| name.set(event_target_value(&ev))
                    />
                    {move || match problem() {
                        Some(p) if tried.get() => view! {
                            <p id="mk-new-page-hint" class="mt-1 text-xs text-error" role="alert">{p}</p>
                        }.into_any(),
                        _ => view! {
                            <p id="mk-new-page-hint" class="mt-1 text-xs text-base-content/70 break-all">
                                "Va a quedar en …/"<span class="font-semibold">{slug}</span>
                            </p>
                        }.into_any(),
                    }}
                </div>
                <fieldset class="space-y-1.5">
                    <legend class="mb-1 text-sm font-medium">"Empezar con"</legend>
                    {std::iter::once((None, "En blanco", "Solo el menú y el pie de tu tienda"))
                        .chain(domain::PAGE_TEMPLATES.iter().enumerate().map(|(i, (_, n))| {
                            (Some(i), *n, "Con bloques de ejemplo para completar")
                        }))
                        .map(|(value, title, hint)| view! {
                            <label class="flex cursor-pointer items-start gap-2.5 rounded-lg border border-base-300 px-3 py-2 has-[:checked]:border-primary has-[:checked]:bg-primary/5">
                                <input
                                    type="radio"
                                    name="mk-new-page-start"
                                    class="radio radio-primary radio-sm mt-0.5"
                                    prop:checked=move || choice.get() == value
                                    on:change=move |_| {
                                        // El nombre sigue a la página modelo mientras el
                                        // dueño no haya escrito uno propio.
                                        let current = name.get_untracked();
                                        let untouched = current.trim().is_empty()
                                            || domain::PAGE_TEMPLATES.iter().any(|(_, n)| *n == current);
                                        if untouched {
                                            name.set(if value.is_some() { title.to_string() } else { String::new() });
                                        }
                                        choice.set(value);
                                    }
                                />
                                <span>
                                    <span class="block text-sm font-medium">{title}</span>
                                    <span class="block text-xs text-base-content/70">{hint}</span>
                                </span>
                            </label>
                        })
                        .collect_view()}
                </fieldset>
                <button type="submit" class="btn btn-primary btn-sm w-full">"Crear página"</button>
            </form>
        </details>
    }
}
