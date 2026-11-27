//! Paneles de la derecha para las herramientas Páginas y Bloques del riel.
//!
//! - Páginas: la tienda como un mapa. Cada página se despliega y muestra sus
//!   bloques en el orden en que se ven.
//! - Bloques: los bloques de la página que se edita.
//!
//! Cada bloque es una fila con su miniatura, su nombre y su título actual.
//! Tocar el nombre abre sus ajustes avanzados; la flecha abre, ahí mismo,
//! los rápidos: los pocos campos que más se cambian en ese bloque.

use domain::{Field, FieldKind, Layer, Product, Widget};
use leptos::prelude::*;

use crate::icons::Icon;
use crate::look::DesignPicker;
use crate::inspector::{group_fields, ShownField};
use crate::palette::{block_info, label_of, Thumb};
use crate::topbar::{NewPageMenu, PageMenu};
use crate::{Actions, EditorState, Tool, Ui};

/// Un bloque del mapa: su id y el nombre de su tipo.
type Row = (String, &'static str);

/// Los ajustes rápidos de cada tipo de bloque. Los demás muestran sus dos
/// primeros campos.
fn quick_keys(type_name: &str) -> Option<&'static [&'static str]> {
    match type_name {
        "Announcement" => Some(&["message", "show_tag", "tag"]),
        "Navbar" => Some(&["brand", "logo_url", "search"]),
        "Banner" => Some(&["title", "image_url"]),
        "Benefits" => Some(&["items"]),
        "Categories" => Some(&["title"]),
        "ProductGrid" => Some(&["title", "limit"]),
        "Card" => Some(&["title", "image_url"]),
        "Testimonials" => Some(&["items"]),
        "Faq" => Some(&["title"]),
        "Footer" => Some(&["brand"]),
        "Text" => Some(&["heading", "body"]),
        _ => None,
    }
}

/// Campos de los ajustes rápidos de un bloque.
fn quick_fields(widget: &Widget) -> Vec<&'static Field> {
    let all = widget.fields();
    if let Some(keys) = quick_keys(widget.type_name()) {
        let found: Vec<_> = keys
            .iter()
            .filter_map(|k| all.iter().find(|f| f.key == *k))
            .collect();
        if !found.is_empty() {
            return found;
        }
    }
    group_fields(all).basic.into_iter().take(2).collect()
}

/// Bloques de una página, en el orden en que se ven (de arriba a abajo).
fn rows_of(index: Signal<usize>) -> Memo<Vec<Row>> {
    let state = expect_context::<EditorState>();
    Memo::new(move |_| {
        let layer = Layer::Page(index.get());
        state.doc.with(|d| {
            let mut nodes: Vec<_> = d.layer(layer).iter().collect();
            nodes.sort_by_key(|n| (n.pos.y, n.pos.x));
            nodes
                .into_iter()
                .map(|n| (n.id.clone(), n.widget.type_name()))
                .collect::<Vec<Row>>()
        })
    })
}

#[component]
pub(crate) fn PageMap(products: StoredValue<Vec<Product>>) -> impl IntoView {
    let ui = expect_context::<Ui>();
    move || match ui.tool.get() {
        Tool::Blocks => view! { <BlocksPanel products/> }.into_any(),
        _ => view! { <PagesTree products/> }.into_any(),
    }
}

/// Herramienta Bloques: la página que se edita.
#[component]
fn BlocksPanel(products: StoredValue<Vec<Product>>) -> impl IntoView {
    let state = expect_context::<EditorState>();
    let ui = expect_context::<Ui>();
    let index = Signal::derive(move || state.page.get());
    let rows = rows_of(index);
    let name = move || {
        state
            .doc
            .with(|d| d.page_name(state.page.get()).to_string())
    };
    view! {
        <div class="flex flex-col gap-4">
            <div class="flex items-start gap-2">
                <div class="min-w-0 flex-1">
                    <h2 id="mk-inspector-title" tabindex="-1" class="text-lg font-bold leading-tight">"Tu página"</h2>
                    <p class="text-sm text-base-content/75">{name}" · "{move || rows.with(Vec::len)}" bloques"</p>
                </div>
                <button type="button" class="btn btn-sm" on:click=move |_| ui.tool.set(Tool::Pages)>"Páginas"</button>
            </div>
            <BlockList rows products/>
            <button type="button" class="btn btn-outline btn-sm w-full border-base-300" on:click=move |_| ui.tool.set(Tool::Add)>
                <Icon name="plus" class="h-4 w-4"/>"Agregar bloque"
            </button>
            <p class="text-xs text-base-content/70">"Elegí un bloque para cambiarlo. Con el nombre abrís todos sus ajustes."</p>
            <crate::page_map::Shortcuts/>
        </div>
    }
}

/// Herramienta Páginas: todas las páginas, cada una con sus bloques.
#[component]
fn PagesTree(products: StoredValue<Vec<Product>>) -> impl IntoView {
    let state = expect_context::<EditorState>();
    let actions = expect_context::<Actions>();
    // Páginas desplegadas: cada una se abre y se pliega por su cuenta (se
    // pueden comparar dos). Al pasar a una página, se despliega sola.
    let expanded = RwSignal::new(vec![state.page.get_untracked()]);
    Effect::new(move |_| {
        let page = state.page.get();
        expanded.update(|e| {
            if !e.contains(&page) {
                e.push(page);
            }
        });
    });
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
                    <h2 id="mk-inspector-title" tabindex="-1" class="text-lg font-bold leading-tight">"Tus páginas"</h2>
                    <p class="text-sm text-base-content/75">"Cada página con sus bloques."</p>
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
        </div>
    }
}

/// Una página: su nombre, si es la actual y, desplegada, sus bloques.
#[component]
fn PageItem(
    index: usize,
    name: String,
    expanded: RwSignal<Vec<usize>>,
    products: StoredValue<Vec<Product>>,
    actions: Actions,
) -> impl IntoView {
    let state = expect_context::<EditorState>();
    let is_open = move || expanded.with(|e| e.contains(&index));
    let is_current = move || state.page.get() == index;
    let body_id = format!("mk-map-page-{index}");
    let rows = rows_of(Signal::stored(index));
    view! {
        <li class="rounded-box border border-base-300 bg-base-100" class=("border-primary", is_current)>
            <div class="flex items-center gap-1 pr-2">
                <button
                    type="button"
                    class="flex min-w-0 flex-1 items-center gap-2 rounded-box py-2.5 pl-3 pr-1 text-left text-sm font-semibold"
                    aria-expanded=move || is_open().to_string()
                    aria-controls=body_id.clone()
                    on:click=move |_| expanded.update(|e| {
                        if let Some(at) = e.iter().position(|p| *p == index) {
                            e.remove(at);
                        } else {
                            e.push(index);
                        }
                    })
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
                <BlockList rows products/>
            </div>
        </li>
    }
}

/// Lista de bloques. Un Memo: se arma de nuevo solo si cambian los bloques
/// o su orden, no con cada tecla (perdería el foco de los campos).
#[component]
fn BlockList(rows: Memo<Vec<Row>>, products: StoredValue<Vec<Product>>) -> impl IntoView {
    view! {
        <ol class="flex flex-col gap-1.5">
            <For each=move || rows.get() key=|r| r.0.clone() let:row>
                <BlockItem id=row.0 type_name=row.1 products/>
            </For>
        </ol>
    }
}

/// Miniatura del bloque, dibujado de verdad y achicado.
#[component]
pub(crate) fn Mini(type_name: &'static str, products: StoredValue<Vec<Product>>) -> impl IntoView {
    // Se arma recién en el navegador, así no hay nada que no coincida con lo
    // que vino del servidor.
    let mounted = RwSignal::new(false);
    Effect::new(move |_| mounted.set(true));
    view! {
        <span class="grid h-11 w-14 shrink-0 place-items-center overflow-hidden rounded-field bg-base-200" aria-hidden="true">
            {move || mounted.get().then(|| view! { <Thumb type_name products tw=56.0 th_max=44.0/> })}
        </span>
    }
}

/// Un bloque del mapa. Elegido, muestra sus ajustes rápidos.
#[component]
fn BlockItem(
    id: String,
    type_name: &'static str,
    products: StoredValue<Vec<Product>>,
) -> impl IntoView {
    let state = expect_context::<EditorState>();
    let actions = expect_context::<Actions>();
    let ui = expect_context::<Ui>();
    let sid = StoredValue::new(id.clone());
    let (_, description) = block_info(type_name);
    let label = label_of(type_name);
    let selected = move || {
        state
            .selected
            .with(|s| s.as_deref() == Some(sid.get_value().as_str()))
    };
    let (body_id, name_id) = (format!("mk-map-block-{id}"), format!("mk-map-name-{id}"));
    let widget = state.node(&id).map(|n| n.widget);
    let fields = widget.as_ref().map(quick_fields).unwrap_or_default();
    // El título actual del bloque, como en la tienda; si no tiene, qué es.
    let title_key = widget
        .as_ref()
        .and_then(|w| {
            w.fields()
                .iter()
                .find(|f| matches!(f.kind, FieldKind::Text { .. }))
        })
        .map(|f| f.key);
    let subtitle = move || {
        title_key
            .and_then(|key| {
                state.doc.with(|d| {
                    sid.with_value(|id| crate::fields::field_value(d, id, key))
                        .as_str()
                        .map(str::trim)
                        .filter(|t| !t.is_empty())
                        .map(str::to_string)
                })
            })
            .unwrap_or_else(|| description.to_string())
    };
    let opens_modal = matches!(type_name, "ProductGrid" | "TagTabs" | "FeaturedProduct");
    let has_links = matches!(type_name, "Navbar" | "Footer");
    let quick_id = id.clone();
    let has_designs = widget.as_ref().is_some_and(Widget::is_new_design);
    let open_advanced = move || {
        request_animation_frame(move || {
            ui.advanced.set(true);
            request_animation_frame(|| crate::focus_selector("#mk-inspector-title"));
        });
    };
    view! {
        <li class="rounded-field border border-base-300 bg-base-100" class=("border-primary", selected) class=("bg-primary/5", selected)>
            <div class="flex items-center">
                <button
                    type="button"
                    id=name_id
                    class="flex min-w-0 flex-1 items-center gap-2.5 rounded-field p-2 text-left"
                    title="Abrir todos los ajustes"
                    on:click=move |_| {
                        actions.go_to(&sid.get_value());
                        open_advanced();
                    }
                >
                    <Mini type_name products/>
                    <span class="flex min-w-0 flex-1 flex-col leading-tight">
                        <span class="truncate text-sm font-semibold">{label}</span>
                        <span class="truncate text-xs text-base-content/70">{subtitle}</span>
                    </span>
                </button>
                <button
                    type="button"
                    class="mr-1 grid size-8 shrink-0 place-items-center rounded-field hover:bg-base-200"
                    aria-expanded=move || selected().to_string()
                    aria-controls=body_id.clone()
                    aria-label=format!("Ajustes rápidos de {}", label.to_lowercase())
                    title="Ajustes rápidos"
                    on:click=move |_| {
                        if selected() {
                            state.selected.set(None);
                        } else {
                            actions.go_to(&sid.get_value());
                        }
                    }
                >
                    <span class="transition-transform" class=("rotate-180", selected)>
                        <Icon name="chevron-down" class="h-4 w-4"/>
                    </span>
                </button>
            </div>
            <div id=body_id class="flex flex-col gap-4 border-t border-base-300 p-3" class:hidden=move || !selected()>
                <h3 class="text-xs font-bold uppercase tracking-wide text-base-content/70">"Ajustes rápidos"</h3>
                {has_designs.then(|| view! { <DesignPicker id=quick_id.clone() visible=selected products/> })}
                {fields
                    .into_iter()
                    .map(|f| view! { <ShownField products id=quick_id.clone() field=f/> })
                    .collect_view()}
                {opens_modal.then(|| view! {
                    <button type="button" class="btn btn-link btn-sm h-auto min-h-0 justify-start p-0 no-underline" on:click=move |_| actions.open_modal(None)>
                        "Diseñar ficha de producto ↗"
                    </button>
                })}
                {has_links.then(|| view! {
                    <button type="button" class="btn btn-link btn-sm h-auto min-h-0 justify-start p-0 no-underline" on:click=move |_| open_advanced()>
                        "Editar enlaces →"
                    </button>
                })}
                <button type="button" class="btn btn-outline btn-sm self-start border-base-300" on:click=move |_| open_advanced()>
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
