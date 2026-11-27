//! Mapa de la tienda: el panel de la derecha cuando no hay un bloque en
//! "Ajustes avanzados". Lista las páginas (cada una se despliega) con sus
//! bloques en el orden en que se ven; al elegir un bloque, acá mismo
//! aparecen sus ajustes rápidos y el paso a los avanzados.

use domain::{Layer, Product};
use leptos::prelude::*;

use crate::icons::Icon;
use crate::inspector::{group_fields, ShownField};
use crate::palette::{block_info, label_of};
use crate::topbar::{NewPageMenu, PageMenu};
use crate::{Actions, EditorState, Ui};

/// Cuántos campos del bloque se muestran como ajustes rápidos.
const QUICK_FIELDS: usize = 2;

/// Un bloque del mapa: su id y el nombre de su tipo.
type Row = (String, &'static str);

#[component]
pub(crate) fn PageMap(products: StoredValue<Vec<Product>>) -> impl IntoView {
    let state = expect_context::<EditorState>();
    let actions = expect_context::<Actions>();
    // Página desplegada. Sigue a la que se edita; se puede plegar a mano.
    let expanded = RwSignal::new(Some(state.page.get_untracked()));
    Effect::new(move |_| expanded.set(Some(state.page.get())));
    let pages = Memo::new(move |_| {
        state.doc.with(|d| {
            (0..d.page_count())
                .map(|i| (i, d.page_name(i).to_string()))
                .collect::<Vec<_>>()
        })
    });

    view! {
        <div class="flex flex-col gap-4">
            <div class="flex items-start gap-2">
                <div class="min-w-0 flex-1">
                    <h2 id="mk-inspector-title" tabindex="-1" class="text-lg font-bold leading-tight">"Tu tienda"</h2>
                    <p class="text-sm text-base-content/75">"Tus páginas y sus bloques. Elegí uno para cambiarlo."</p>
                </div>
                <NewPageMenu/>
            </div>
            <ul class="flex flex-col gap-2" aria-label="Páginas de la tienda">
                <For each=move || pages.get() key=|(i, n)| (*i, n.clone()) let:page>
                    <PageItem index=page.0 name=page.1 expanded products actions/>
                </For>
            </ul>
            <p class="text-xs text-base-content/70">
                {move || if state.doc.with(|d| d.pages.is_empty()) {
                    "Tocá un enlace del menú superior (Ofertas, Categorías…) para crear y editar esa página."
                } else {
                    "Tocá un enlace del menú para ir a esa página, o elegila acá."
                }}
            </p>
            <Shortcuts/>
        </div>
    }
}

/// Una página: su nombre, si es la actual y, desplegada, sus bloques.
#[component]
fn PageItem(
    index: usize,
    name: String,
    expanded: RwSignal<Option<usize>>,
    products: StoredValue<Vec<Product>>,
    actions: Actions,
) -> impl IntoView {
    let state = expect_context::<EditorState>();
    let is_open = move || expanded.get() == Some(index);
    let is_current = move || state.page.get() == index;
    let body_id = format!("mk-map-page-{index}");
    // Un Memo: la lista se arma de nuevo solo si cambian los bloques o su
    // orden, no con cada tecla (perdería el foco de los campos).
    let rows = Memo::new(move |_| {
        state.doc.with(|d| {
            let mut nodes: Vec<_> = d.layer(Layer::Page(index)).iter().collect();
            nodes.sort_by_key(|n| (n.pos.y, n.pos.x));
            nodes
                .into_iter()
                .map(|n| (n.id.clone(), n.widget.type_name()))
                .collect::<Vec<Row>>()
        })
    });
    view! {
        <li class="rounded-box border border-base-300 bg-base-100" class=("border-primary", is_current)>
            <div class="flex items-center gap-1 pr-2">
                <button
                    type="button"
                    class="flex min-w-0 flex-1 items-center gap-2 rounded-box py-2.5 pl-3 pr-1 text-left text-sm font-semibold"
                    aria-expanded=move || is_open().to_string()
                    aria-controls=body_id.clone()
                    on:click=move |_| expanded.set(if is_open() { None } else { Some(index) })
                >
                    <span class="shrink-0 transition-transform" class=("-rotate-90", move || !is_open())>
                        <Icon name="chevron-down" class="h-4 w-4"/>
                    </span>
                    <span class="truncate">{name}</span>
                    {move || is_current().then(|| view! {
                        <span class="badge badge-sm badge-primary badge-soft shrink-0">"Actual"</span>
                    })}
                </button>
                {move || (is_current() && index > 0).then(|| view! { <PageMenu/> })}
            </div>
            <div id=body_id class="flex flex-col gap-2 border-t border-base-300 p-2" class:hidden=move || !is_open()>
                {move || (!is_current()).then(|| view! {
                    <button type="button" class="btn btn-ghost btn-sm self-start text-primary"
                        on:click=move |_| actions.open_page(index, true)>
                        "Ver página"<Icon name="arrow-right" class="h-3.5 w-3.5"/>
                    </button>
                })}
                <ol class="flex flex-col gap-1.5">
                    <For each=move || rows.get() key=|r| r.0.clone() let:row>
                        <BlockItem id=row.0 type_name=row.1 products/>
                    </For>
                </ol>
            </div>
        </li>
    }
}

/// Un bloque del mapa. Elegido, muestra sus ajustes rápidos.
#[component]
fn BlockItem(id: String, type_name: &'static str, products: StoredValue<Vec<Product>>) -> impl IntoView {
    let state = expect_context::<EditorState>();
    let actions = expect_context::<Actions>();
    let ui = expect_context::<Ui>();
    let sid = StoredValue::new(id.clone());
    let (icon, description) = block_info(type_name);
    let label = label_of(type_name);
    let selected = move || state.selected.with(|s| s.as_deref() == Some(sid.get_value().as_str()));
    let body_id = format!("mk-map-block-{id}");
    let fields = state
        .node(&id)
        .map(|n| group_fields(n.widget.fields()))
        .map(|g| g.basic.into_iter().take(QUICK_FIELDS).collect::<Vec<_>>())
        .unwrap_or_default();
    let quick_id = id.clone();
    view! {
        <li class="rounded-field border border-base-300 bg-base-100" class=("border-primary", selected) class=("bg-primary/5", selected)>
            <div class="flex items-center">
                <button
                    type="button"
                    class="flex min-w-0 flex-1 items-center gap-2.5 rounded-field p-2 text-left"
                    aria-expanded=move || selected().to_string()
                    aria-controls=body_id.clone()
                    on:click=move |_| {
                        if selected() {
                            state.selected.set(None);
                        } else {
                            actions.go_to(&sid.get_value());
                        }
                    }
                >
                    <span class="grid size-9 shrink-0 place-items-center rounded-field bg-primary/10 text-primary" aria-hidden="true">
                        <Icon name=icon class="h-4.5 w-4.5"/>
                    </span>
                    <span class="flex min-w-0 flex-1 flex-col leading-tight">
                        <span class="truncate text-sm font-semibold">{label}</span>
                        <span class="truncate text-xs text-base-content/70">{description}</span>
                    </span>
                    <span class="shrink-0 transition-transform" class=("rotate-180", selected)>
                        <Icon name="chevron-down" class="h-4 w-4"/>
                    </span>
                </button>
            </div>
            <div id=body_id class="flex flex-col gap-4 border-t border-base-300 p-3" class:hidden=move || !selected()>
                <h3 class="text-xs font-bold uppercase tracking-wide text-base-content/70">"Ajustes rápidos"</h3>
                {fields
                    .into_iter()
                    .map(|f| view! { <ShownField products id=quick_id.clone() field=f/> })
                    .collect_view()}
                <button
                    type="button"
                    class="btn btn-outline btn-sm self-start border-base-300"
                    on:click=move |_| {
                        ui.advanced.set(true);
                        request_animation_frame(|| crate::focus_selector("#mk-inspector-title"));
                    }
                >
                    "Ajustes avanzados"<Icon name="arrow-right" class="h-3.5 w-3.5"/>
                </button>
            </div>
        </li>
    }
}

/// Atajos de teclado, plegados al pie del panel.
#[component]
pub(crate) fn Shortcuts() -> impl IntoView {
    let shortcuts: &[(&str, &str)] = &[
        ("Ctrl/⌘ + Z", "Deshacer"),
        ("Ctrl/⌘ + Mayús + Z", "Rehacer"),
        ("Ctrl/⌘ + S", "Guardar ahora"),
        ("Tab", "Ir de un bloque a otro"),
        ("Enter", "Editar el bloque enfocado"),
        ("Flechas", "Mover el bloque"),
        ("Mayús + flechas", "Cambiar su tamaño"),
        ("Suprimir", "Borrar el bloque"),
        ("Esc", "Soltar el bloque elegido"),
    ];
    view! {
        <details class="text-sm">
            <summary class="cursor-pointer font-semibold">"⌨ Atajos de teclado"</summary>
            <dl class="mt-2 grid grid-cols-[auto_1fr] items-center gap-x-3 gap-y-1.5">
                {shortcuts.iter().map(|(k, d)| view! {
                    <dt><kbd class="kbd kbd-sm">{*k}</kbd></dt>
                    <dd>{*d}</dd>
                }).collect_view()}
            </dl>
        </details>
    }
}
