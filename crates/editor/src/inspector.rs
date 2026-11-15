//! Inspector: el panel de la derecha con los campos del bloque elegido.
//!
//! Dos pestañas: "Contenido" (textos, imágenes, listas y el botón) y
//! "Diseño" (diseño y color, tamaño y letras). Al elegir un bloque se abre en Contenido,
//! que es lo que la persona viene a cambiar casi siempre.

use domain::{Field, Product, Widget, GRID_COLS, ROW_PX};
use leptos::prelude::*;

use crate::fields::{ButtonGroup, PropField};
use crate::icons::Icon;
use crate::look::BlockLook;
use crate::palette::block_info;
use crate::typography::BlockTypography;
use crate::{choice_class, focus_selector, Actions, EditorState, Ui, WIDTHS};

/// Filas que suma o resta cada botón de alto (48 px).
const HEIGHT_STEP: i32 = 6;

/// Alto en píxeles de `h` filas.
fn rows_to_px(h: u8) -> u32 {
    u32::from(h) * u32::from(ROW_PX)
}

/// Filas más cercanas a `px` píxeles.
fn px_to_rows(px: f64) -> i32 {
    (px / f64::from(ROW_PX)).round() as i32
}

/// Campos de un bloque repartidos para el panel.
struct FieldGroups {
    /// A la vista en Contenido.
    basic: Vec<&'static Field>,
    /// Texto y enlace del botón, si el bloque tiene uno.
    button: Option<(&'static Field, &'static Field)>,
    /// Plegados en "Más opciones".
    advanced: Vec<&'static Field>,
}

fn group_fields(fields: &'static [Field]) -> FieldGroups {
    let find = |key: &str| fields.iter().find(|f| f.key == key);
    let button = find("button_label").zip(find("button_href"));
    let is_button = |f: &Field| button.is_some() && matches!(f.key, "button_label" | "button_href");
    FieldGroups {
        basic: fields
            .iter()
            .filter(|f| !f.advanced && !is_button(f))
            .collect(),
        button,
        advanced: fields
            .iter()
            .filter(|f| f.advanced && !is_button(f))
            .collect(),
    }
}

#[component]
pub(crate) fn Inspector(products: StoredValue<Vec<Product>>) -> impl IntoView {
    let state = expect_context::<EditorState>();
    // Se redibuja solo cuando cambia el nodo seleccionado (o deja de existir),
    // no con cada tecla, para no perder el foco de los campos.
    let current = Memo::new(move |_| {
        state.selected.get().filter(|id| {
            let layer = state.layer.get();
            state.doc.with(|d| d.layer_of(id)) == Some(layer)
        })
    });

    view! {
        <aside class="w-[23rem] shrink-0 overflow-y-auto border-l border-base-300 bg-base-100 p-4" aria-label="Propiedades del bloque">
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
    let state = expect_context::<EditorState>();
    let actions = expect_context::<Actions>();
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
            {move || if state.in_modal() {
                view! {
                    <p class="text-sm">"Estás en la ficha del producto. Hacé clic en uno de sus bloques para cambiarlo aquí, o sumá bloques desde la izquierda."</p>
                    <button type="button" class="btn btn-sm self-start" on:click=move |_| actions.close_modal()>"Volver a la página"</button>
                }.into_any()
            } else {
                view! {
                    <p class="text-sm">"Hacé clic en un bloque del lienzo para cambiar sus textos, imágenes y tamaño aquí."</p>
                }.into_any()
            }}
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

#[derive(Clone, Copy, PartialEq, Eq)]
enum Tab {
    Content,
    Design,
}

impl Tab {
    fn ids(self) -> (&'static str, &'static str) {
        match self {
            Tab::Content => ("mk-tab-content", "mk-tabpanel-content"),
            Tab::Design => ("mk-tab-design", "mk-tabpanel-design"),
        }
    }

    fn other(self) -> Tab {
        match self {
            Tab::Content => Tab::Design,
            Tab::Design => Tab::Content,
        }
    }
}

#[component]
fn NodeInspector(products: StoredValue<Vec<Product>>, id: String, widget: Widget) -> impl IntoView {
    let ui = expect_context::<Ui>();
    let actions = expect_context::<Actions>();
    let label = widget.label();
    let (icon, description) = block_info(widget.type_name());
    let sid = StoredValue::new(id.clone());
    let tab = RwSignal::new(Tab::Content);
    // Al intentar guardar con errores, los campos están en Contenido.
    Effect::new(move |_| {
        if ui.show_errors.get() {
            tab.set(Tab::Content);
        }
    });
    let FieldGroups {
        basic,
        button,
        advanced,
    } = group_fields(widget.fields());
    let has_advanced = !advanced.is_empty();
    let no_fields = basic.is_empty() && button.is_none() && !has_advanced;
    let opens_modal = matches!(widget, Widget::ProductGrid(_) | Widget::FeaturedProduct(_));
    let (basic_id, advanced_id, button_id) = (id.clone(), id.clone(), id.clone());

    // Pestañas con el patrón de ARIA: flechas, Inicio y Fin cambian de
    // pestaña y llevan el foco a la nueva.
    let on_tab_key = move |ev: leptos::ev::KeyboardEvent| {
        let next = match ev.key().as_str() {
            "ArrowLeft" | "ArrowRight" => tab.get_untracked().other(),
            "Home" => Tab::Content,
            "End" => Tab::Design,
            _ => return,
        };
        ev.prevent_default();
        tab.set(next);
        focus_selector(&format!("#{}", next.ids().0));
    };
    let tab_button = move |t: Tab, text: &'static str| {
        let (tab_id, panel_id) = t.ids();
        view! {
            <button
                type="button"
                role="tab"
                id=tab_id
                class="tab font-medium"
                class:tab-active=move || tab.get() == t
                aria-selected=move || (tab.get() == t).to_string()
                aria-controls=panel_id
                tabindex=move || if tab.get() == t { "0" } else { "-1" }
                on:click=move |_| tab.set(t)
            >{text}</button>
        }
    };

    view! {
        <div class="flex flex-col gap-4">
            <div class="flex items-start gap-3">
                <span class="rounded-field bg-primary/10 p-2 text-primary" aria-hidden="true"><Icon name=icon class="h-5 w-5"/></span>
                <div class="min-w-0 flex-1">
                    <h2 id="mk-inspector-title" tabindex="-1" class="text-lg font-bold leading-tight">{label}</h2>
                    <p class="text-sm text-base-content/75">{description}</p>
                </div>
                <div class="flex shrink-0 gap-1" role="group" aria-label="Acciones del bloque">
                    <button
                        type="button"
                        class="btn btn-ghost btn-sm btn-square"
                        aria-label=format!("Duplicar {}", label.to_lowercase())
                        title="Duplicar"
                        on:click=move |_| sid.with_value(|id| actions.duplicate(id))
                    ><Icon name="copy"/></button>
                    <button
                        type="button"
                        class="btn btn-ghost btn-sm btn-square text-red-700 hover:bg-red-700 hover:text-white"
                        aria-label=format!("Borrar {}", label.to_lowercase())
                        title="Borrar (Suprimir)"
                        on:click=move |_| sid.with_value(|id| actions.delete(id))
                    ><Icon name="trash"/></button>
                </div>
            </div>

            <div role="tablist" aria-label="Qué querés cambiar" class="tabs tabs-box tabs-sm grid grid-cols-2" on:keydown=on_tab_key>
                {tab_button(Tab::Content, "Contenido")}
                {tab_button(Tab::Design, "Diseño")}
            </div>

            <div
                role="tabpanel"
                id=Tab::Content.ids().1
                aria-labelledby=Tab::Content.ids().0
                class="flex flex-col gap-5"
                class:hidden=move || tab.get() != Tab::Content
            >
                {basic
                    .into_iter()
                    .map(|f| view! { <PropField products id=basic_id.clone() field=f/> })
                    .collect_view()}
                {button.map(|(label_field, href_field)| view! {
                    <ButtonGroup products id=button_id.clone() label_field href_field/>
                })}
                {no_fields.then(|| view! {
                    <p class="text-sm text-base-content/75">"Este bloque muestra el dato del producto que se abre, así que no hay nada para escribir. Su diseño, color, tamaño y letras se cambian en Diseño."</p>
                })}
                {move || {
                    let state = expect_context::<EditorState>();
                    let pages = state.doc.with(|d| {
                        d.node(&sid.get_value())
                            .map_or(0, |n| d.shared_pages(&n.shared))
                    });
                    (pages > 1).then(|| view! {
                        <section class="flex flex-col gap-2 rounded-box border border-primary/30 bg-primary/5 p-3" aria-labelledby="mk-shared-title">
                            <h3 id="mk-shared-title" class="flex items-center gap-2 text-sm font-semibold">
                                <Icon name="page" class="h-4 w-4 text-primary"/>
                                {format!("Compartido en {pages} páginas")}
                            </h3>
                            <p class="text-sm text-base-content/75">"Lo que cambies acá (textos, enlaces, letras) se ve igual en todas tus páginas. El lugar y el tamaño son de cada página."</p>
                            <button type="button" class="btn btn-ghost btn-sm self-start" on:click=move |_| sid.with_value(|id| actions.unshare(id))>
                                "Cambiarlo solo en esta página"
                            </button>
                        </section>
                    })
                }}
                {opens_modal.then(|| view! {
                    <section class="flex flex-col gap-2 rounded-box border border-base-300 p-3" aria-labelledby="mk-modal-help-title">
                        <h3 id="mk-modal-help-title" class="text-sm font-semibold">"Ficha del producto"</h3>
                        <p class="text-sm text-base-content/75">"En tu tienda, tocar un producto abre su ficha con el fondo oscurecido. Es la misma para todos tus productos. También podés abrirla haciendo clic en un producto del lienzo."</p>
                        <button type="button" class="btn btn-sm self-start" on:click=move |_| actions.open_modal(None)>
                            <Icon name="featured"/>
                            "Diseñar la ficha"
                        </button>
                    </section>
                })}
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
            </div>

            <div
                role="tabpanel"
                id=Tab::Design.ids().1
                aria-labelledby=Tab::Design.ids().0
                class="flex flex-col gap-5"
                class:hidden=move || tab.get() != Tab::Design
            >
                <BlockLook id=id.clone() visible=Signal::derive(move || tab.get() == Tab::Design) products/>
                <BlockSize id=id.clone() widget=widget.clone()/>
                <BlockTypography id=id.clone()/>
            </div>
        </div>
    }
}

/// Sección "Tamaño" de la pestaña Diseño: ancho con nombres, alto con dos
/// botones y los números exactos plegados para quien los quiera.
#[component]
fn BlockSize(id: String, widget: Widget) -> impl IntoView {
    let state = expect_context::<EditorState>();
    let limits = widget.limits();
    let sid = StoredValue::new(id);
    let size = move || {
        state
            .doc
            .with(|d| sid.with_value(|id| d.node(id).map(|n| n.size).unwrap_or(limits.default)))
    };
    let resize = move |w: i32, h: i32| {
        sid.with_value(|id| state.apply_nodes(|nodes| grid::resize_node(nodes, id, w, h)));
    };
    let widths: Vec<(u8, &str)> = WIDTHS
        .iter()
        .copied()
        .filter(|(w, _)| (limits.min_w..=limits.max_w).contains(w))
        .collect();
    let custom = move || !WIDTHS.iter().any(|(w, _)| *w == size().w);
    let change_h = move |rows: i32| resize(i32::from(size().w), i32::from(size().h) + rows);

    view! {
        <section class="flex flex-col gap-4" aria-labelledby="mk-size-title">
            <h3 id="mk-size-title" class="text-sm font-semibold">"Tamaño"</h3>
            <fieldset class="fieldset gap-1 p-0">
                <legend class="fieldset-legend pt-0 text-sm font-medium">"Ancho"</legend>
                <div class="join w-full">
                    {widths.into_iter().map(|(w, name)| view! {
                        <button
                            type="button"
                            class=move || format!("{} join-item flex-1", choice_class(size().w == w))
                            aria-pressed=move || (size().w == w).to_string()
                            on:click=move |_| resize(i32::from(w), i32::from(size().h))
                        >{name}</button>
                    }).collect_view()}
                </div>
                {move || custom().then(|| view! {
                    <p class="label text-xs text-base-content/75">"Ahora tiene un ancho a medida."</p>
                })}
            </fieldset>
            <fieldset class="fieldset gap-1 p-0">
                <legend class="fieldset-legend pt-0 text-sm font-medium">"Alto"</legend>
                <div class="flex items-center gap-2">
                    <button
                        type="button"
                        class="btn btn-sm flex-1"
                        disabled={move || size().h <= limits.min_h}
                        on:click=move |_| change_h(-HEIGHT_STEP)
                    >"Más bajo"</button>
                    <span class="min-w-16 text-center text-sm tabular-nums" aria-live="polite">
                        {move || format!("{} px", rows_to_px(size().h))}
                    </span>
                    <button
                        type="button"
                        class="btn btn-sm flex-1"
                        disabled={move || size().h >= limits.max_h}
                        on:click=move |_| change_h(HEIGHT_STEP)
                    >"Más alto"</button>
                </div>
            </fieldset>
            <details class="collapse collapse-arrow border border-base-300">
                <summary class="collapse-title text-sm font-medium">"Ajustar a mano"</summary>
                <div class="collapse-content grid grid-cols-2 gap-2">
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
            </details>
        </section>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn button_fields_get_their_own_group() {
        let g = group_fields(Widget::defaults("Banner").unwrap().fields());
        let keys = |v: &[&Field]| v.iter().map(|f| f.key).collect::<Vec<_>>();
        assert_eq!(keys(&g.basic), ["title", "subtitle", "image_url"]);
        let (label, href) = g.button.expect("el banner tiene botón");
        assert_eq!((label.key, href.key), ("button_label", "button_href"));
        assert!(g.advanced.is_empty());
    }

    #[test]
    fn other_advanced_fields_stay_folded() {
        let g = group_fields(Widget::defaults("Countdown").unwrap().fields());
        assert!(g.button.is_some());
        assert_eq!(
            g.advanced.iter().map(|f| f.key).collect::<Vec<_>>(),
            ["ended_text"]
        );

        let g = group_fields(Widget::defaults("Image").unwrap().fields());
        assert!(g.button.is_none());
        assert_eq!(
            g.advanced.iter().map(|f| f.key).collect::<Vec<_>>(),
            ["href", "fit"]
        );
    }
}
