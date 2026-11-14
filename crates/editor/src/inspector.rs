//! Inspector: tamaño, orden y campos del bloque elegido.

use domain::{Field, Product, Widget, GRID_COLS, ROW_PX};
use leptos::prelude::*;

use crate::fields::PropField;
use crate::icons::Icon;
use crate::palette::block_info;
use crate::{Actions, EditorState};

/// Anchos con nombre, en columnas de 48.
const WIDTHS: &[(u8, &str)] = &[
    (48, "Completo"),
    (32, "2/3"),
    (24, "Mitad"),
    (16, "1/3"),
    (12, "1/4"),
];

/// Alto en píxeles de `h` filas.
fn rows_to_px(h: u8) -> u32 {
    u32::from(h) * u32::from(ROW_PX)
}

/// Filas más cercanas a `px` píxeles.
fn px_to_rows(px: f64) -> i32 {
    (px / f64::from(ROW_PX)).round() as i32
}

#[component]
pub(crate) fn Inspector(products: StoredValue<Vec<Product>>) -> impl IntoView {
    let state = expect_context::<EditorState>();
    // Se redibuja solo cuando cambia el nodo seleccionado (o deja de existir),
    // no con cada tecla, para no perder el foco de los campos.
    let current = Memo::new(move |_| {
        state
            .selected
            .get()
            .filter(|id| state.doc.with(|d| d.nodes.iter().any(|n| &n.id == id)))
    });

    view! {
        <aside class="w-80 shrink-0 overflow-y-auto border-l border-base-300 bg-base-100 p-4" aria-label="Propiedades del bloque">
            {move || match current.get() {
                None => view! { <EmptyInspector/> }.into_any(),
                Some(id) => match state.node(&id) {
                    Some(node) => view! { <NodeInspector products id widget=node.widget/> }.into_any(),
                    None => ().into_any(),
                },
            }}
        </aside>
    }
}

#[component]
fn EmptyInspector() -> impl IntoView {
    let shortcuts: &[(&str, &str)] = &[
        ("Ctrl/⌘ + Z", "Deshacer"),
        ("Ctrl/⌘ + Mayús + Z", "Rehacer"),
        ("Ctrl/⌘ + S", "Guardar"),
        ("Tab", "Ir de un bloque a otro"),
        ("Enter", "Editar el bloque enfocado"),
        ("Flechas", "Mover el bloque"),
        ("Mayús + flechas", "Cambiar su tamaño"),
        ("Suprimir", "Borrar el bloque"),
        ("Esc", "Soltar el bloque elegido"),
    ];
    view! {
        <div class="flex flex-col gap-4">
            <h2 id="mk-inspector-title" tabindex="-1" class="text-lg font-bold">"Ningún bloque elegido"</h2>
            <p class="text-sm">"Hacé clic en un bloque del lienzo para cambiar sus textos, imágenes y tamaño aquí."</p>
            <div>
                <h3 class="mb-2 text-sm font-semibold">"Atajos de teclado"</h3>
                <dl class="grid grid-cols-[auto_1fr] items-center gap-x-3 gap-y-1.5 text-sm">
                    {shortcuts.iter().map(|(k, d)| view! {
                        <dt><kbd class="kbd kbd-sm">{*k}</kbd></dt>
                        <dd>{*d}</dd>
                    }).collect_view()}
                </dl>
            </div>
        </div>
    }
}

#[component]
fn NodeInspector(products: StoredValue<Vec<Product>>, id: String, widget: Widget) -> impl IntoView {
    let state = expect_context::<EditorState>();
    let actions = expect_context::<Actions>();
    let label = widget.label();
    let (icon, description) = block_info(widget.type_name());
    let limits = widget.limits();
    let sid = StoredValue::new(id.clone());
    let size = move || {
        state.doc.with(|d| {
            sid.with_value(|id| {
                d.nodes
                    .iter()
                    .find(|n| &n.id == id)
                    .map(|n| n.size)
                    .unwrap_or(limits.default)
            })
        })
    };
    let resize = move |w: i32, h: i32| {
        sid.with_value(|id| state.apply(|d| grid::resize_node(&mut d.nodes, id, w, h)));
    };
    let basic: Vec<&'static Field> = widget.fields().iter().filter(|f| !f.advanced).collect();
    let advanced: Vec<&'static Field> = widget.fields().iter().filter(|f| f.advanced).collect();
    let has_advanced = !advanced.is_empty();
    let widths: Vec<(u8, &str)> = WIDTHS
        .iter()
        .copied()
        .filter(|(w, _)| (limits.min_w..=limits.max_w).contains(w))
        .collect();
    let (basic_id, advanced_id) = (id.clone(), id.clone());

    view! {
        <div class="flex flex-col gap-5">
            <div class="flex items-start gap-3">
                <span class="rounded-field bg-primary/10 p-2 text-primary" aria-hidden="true"><Icon name=icon class="h-5 w-5"/></span>
                <div>
                    <h2 id="mk-inspector-title" tabindex="-1" class="text-lg font-bold leading-tight">{label}</h2>
                    <p class="text-sm text-base-content/75">{description}</p>
                </div>
            </div>
            <div class="grid grid-cols-3 gap-2" role="group" aria-label="Orden y copia">
                <button type="button" class="btn btn-sm" on:click=move |_| sid.with_value(|id| actions.move_up(id))>
                    <Icon name="up"/>"Subir"
                </button>
                <button type="button" class="btn btn-sm" on:click=move |_| sid.with_value(|id| actions.move_down(id))>
                    <Icon name="down"/>"Bajar"
                </button>
                <button type="button" class="btn btn-sm" on:click=move |_| sid.with_value(|id| actions.duplicate(id))>
                    <Icon name="copy"/>"Duplicar"
                </button>
            </div>
            <fieldset class="flex flex-col gap-2">
                <legend class="mb-1 text-sm font-medium">"Ancho"</legend>
                <div class="join w-full">
                    {widths.into_iter().map(|(w, name)| view! {
                        <button
                            type="button"
                            class="btn btn-sm join-item flex-1"
                            class:btn-primary=move || size().w == w
                            aria-pressed=move || (size().w == w).to_string()
                            on:click=move |_| resize(i32::from(w), i32::from(size().h))
                        >{name}</button>
                    }).collect_view()}
                </div>
            </fieldset>
            <div class="grid grid-cols-2 gap-2">
                <label class="flex flex-col gap-1">
                    <span class="text-sm font-medium">"Ancho exacto"</span>
                    <input
                        type="number"
                        class="input input-sm w-full"
                        min=limits.min_w
                        max=limits.max_w
                        aria-describedby="mk-size-w-help"
                        prop:value=move || size().w.to_string()
                        on:change=move |ev| {
                            if let Ok(w) = event_target_value(&ev).parse::<i32>() {
                                resize(w, i32::from(size().h));
                            }
                        }
                    />
                    <span id="mk-size-w-help" class="text-xs text-base-content/75">{format!("De {} a {} columnas ({GRID_COLS} = todo el ancho)", limits.min_w, limits.max_w)}</span>
                </label>
                <label class="flex flex-col gap-1">
                    <span class="text-sm font-medium">"Alto (px)"</span>
                    <input
                        type="number"
                        class="input input-sm w-full"
                        min=rows_to_px(limits.min_h)
                        max=rows_to_px(limits.max_h)
                        step=ROW_PX
                        aria-describedby="mk-size-h-help"
                        prop:value=move || rows_to_px(size().h).to_string()
                        on:change=move |ev| {
                            if let Ok(px) = event_target_value(&ev).parse::<f64>() {
                                resize(i32::from(size().w), px_to_rows(px));
                            }
                        }
                    />
                    <span id="mk-size-h-help" class="text-xs text-base-content/75">{format!("De {} a {} píxeles, de {ROW_PX} en {ROW_PX}", rows_to_px(limits.min_h), rows_to_px(limits.max_h))}</span>
                </label>
            </div>
            {basic
                .into_iter()
                .map(|f| view! { <PropField products id=basic_id.clone() field=f/> })
                .collect_view()}
            {has_advanced.then(|| view! {
                <details class="collapse collapse-arrow border border-base-300">
                    <summary class="collapse-title text-sm font-medium">"Más opciones"</summary>
                    <div class="collapse-content flex flex-col gap-4">
                        {advanced
                            .into_iter()
                            .map(|f| view! { <PropField products id=advanced_id.clone() field=f/> })
                            .collect_view()}
                    </div>
                </details>
            })}
            <button
                type="button"
                class="btn btn-sm btn-outline border-red-700 text-red-700 hover:bg-red-700 hover:text-white"
                on:click=move |_| sid.with_value(|id| actions.delete(id))
            >
                <Icon name="trash"/>
                {format!("Borrar {}", label.to_lowercase())}
            </button>
        </div>
    }
}
