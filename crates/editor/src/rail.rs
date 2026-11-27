//! Riel angosto de la izquierda del editor: las cuatro herramientas
//! (Páginas, Bloques, Agregar y Estilo). Cada una cambia lo que muestra el
//! panel de la derecha; el lienzo no se mueve.

use leptos::prelude::*;

use crate::icons::Icon;
use crate::store_drawer::STORE_BUTTON_ID;
use crate::{EditorState, Tool, Ui};

#[component]
pub(crate) fn Rail() -> impl IntoView {
    let ui = expect_context::<Ui>();
    let state = expect_context::<EditorState>();
    // En la ficha del producto no hay páginas para recorrer.
    let item = move |tool: Tool, id: &'static str, icon: &'static str, label: &'static str| {
        let on = move || ui.tool.get() == tool;
        view! {
            <button
                type="button"
                id=id
                class="flex w-full flex-col items-center gap-1 rounded-box px-1 py-2.5 text-[11px] font-semibold text-base-content/70 transition-colors hover:bg-base-200 focus-visible:outline-2 focus-visible:outline-primary"
                class=("bg-primary/10", on)
                class=("text-primary", on)
                aria-pressed=move || on().to_string()
                on:click=move |_| ui.tool.set(tool)
            >
                <Icon name=icon class="h-5 w-5"/>
                {label}
            </button>
        }
    };
    view! {
        <nav class="z-30 flex w-[4.5rem] shrink-0 flex-col gap-1.5 border-r border-base-300 bg-base-100 p-2" aria-label="Herramientas del editor">
            {item(Tool::Pages, "mk-rail-pages", "page", "Páginas")}
            {item(Tool::Blocks, "mk-rail-blocks", "grid", "Bloques")}
            {item(Tool::Add, "mk-rail-add", "plus", "Agregar")}
            {item(Tool::Style, STORE_BUTTON_ID, "contrast", "Estilo")}
            {move || state.in_modal().then(|| view! { <span class="sr-only">"Editando la ficha del producto"</span> })}
        </nav>
    }
}
