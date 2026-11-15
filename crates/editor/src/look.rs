//! Sección "Diseño y color" de la pestaña Diseño del inspector.
//!
//! Los cinco diseños se eligen mirando el bloque mismo: cada opción es una
//! miniatura del bloque dibujado con ese diseño, en el tema y con el color
//! de la tienda. Son radios nativos (flechas para moverse, un solo punto de
//! tabulación) con la marca de elegido a la vista, no solo por color.

use domain::{Layer, Look, Node, Product, DESIGNS, THEME_COLORS};
use leptos::prelude::*;
use widgets::{render_widget, sample_product, RenderCtx};

use crate::icons::Icon;
use crate::EditorState;

/// Ancho de cada miniatura, en píxeles (dos por fila en el panel).
const THUMB_W: f64 = 156.0;
/// Alto máximo de una miniatura; lo que sobra del bloque se recorta.
const THUMB_MAX_H: f64 = 104.0;
/// Ancho máximo del bloque que entra en una miniatura, en píxeles.
const THUMB_VIEW_MAX_W: f64 = 900.0;

#[component]
pub(crate) fn BlockLook(
    id: String,
    /// Si la pestaña Diseño está a la vista: las miniaturas se dibujan solo
    /// entonces, para no redibujarlas con cada tecla en Contenido.
    #[prop(into)]
    visible: Signal<bool>,
    products: StoredValue<Vec<Product>>,
) -> impl IntoView {
    let state = expect_context::<EditorState>();
    let sid = StoredValue::new(id.clone());
    let node = Memo::new(move |_| state.doc.with(|d| sid.with_value(|id| d.node(id).cloned())));
    let look =
        Memo::new(move |_| node.with(|n| n.as_ref().map(|n| n.look.clone()).unwrap_or_default()));
    let set = move |group: Option<&str>, f: Box<dyn FnOnce(&mut Look)>| {
        sid.with_value(|id| {
            state.apply_grouped(group, |d| {
                if let Some(n) = d.node_mut(id) {
                    f(&mut n.look);
                }
            })
        })
    };
    let theme = move || state.doc.with(|d| d.theme.clone());
    let store_vars = move || state.doc.with(|d| d.typography.css_vars());
    let group_name = format!("mk-design-{id}");
    let color_name = format!("mk-color-{id}");
    let custom_id = format!("mk-color-custom-{id}");
    let is_custom = move || look.with(|l| l.color.starts_with('#'));
    let custom_value = move || {
        look.with(|l| {
            if l.color.starts_with('#') {
                l.color.clone()
            } else {
                "#3355ff".to_string()
            }
        })
    };

    view! {
        <section class="flex flex-col gap-4" aria-labelledby="mk-look-title">
            <div>
                <h3 id="mk-look-title" class="text-sm font-semibold">"Diseño y color"</h3>
                <p class="text-xs text-base-content/75">"Cada diseño cambia cómo se ve el bloque. Tus textos e imágenes no cambian."</p>
            </div>
            <fieldset>
                <legend class="mb-2 text-sm font-medium">"Diseño"</legend>
                <div class="grid grid-cols-2 gap-2">
                    {DESIGNS.iter().map(|(n, name, hint)| {
                        let n = *n;
                        let checked = move || look.with(|l| l.design == n);
                        view! {
                            <label class="group relative flex cursor-pointer flex-col gap-1 rounded-box p-1.5 outline-2 outline-offset-0 outline-transparent hover:bg-base-200 has-[:checked]:bg-primary/5 has-[:checked]:outline-primary has-[:focus-visible]:outline-offset-2 has-[:focus-visible]:outline-base-content">
                                <input
                                    type="radio"
                                    class="sr-only"
                                    name=group_name.clone()
                                    value=n.to_string()
                                    prop:checked=checked
                                    on:change=move |_| set(None, Box::new(move |l| l.design = n))
                                />
                                <div
                                    data-theme=theme
                                    class="mk-store overflow-hidden rounded-field border border-base-300 bg-base-200"
                                    style=move || format!("{}; height:{THUMB_MAX_H}px", store_vars())
                                    aria-hidden="true"
                                >
                                    {move || visible.get().then(|| node.get().map(|node| {
                                        view! { <Thumb node design=n products/> }
                                    }))}
                                </div>
                                <span class="flex items-center gap-1 text-sm font-medium">
                                    {move || checked().then(|| view! { <Icon name="check" class="h-4 w-4 text-primary"/> })}
                                    {*name}
                                </span>
                                <span class="text-xs leading-tight text-base-content/75">{*hint}</span>
                            </label>
                        }
                    }).collect_view()}
                </div>
            </fieldset>
            <fieldset>
                <legend class="mb-2 text-sm font-medium">"Color"</legend>
                <div data-theme=theme class="grid grid-cols-3 gap-1.5 rounded-box bg-base-100 p-1 text-base-content">
                    {THEME_COLORS.iter().map(|(cid, name)| {
                        let cid: &'static str = cid;
                        let checked = move || look.with(|l| l.color == cid);
                        let swatch = swatch_class(cid);
                        view! {
                            <label class="flex cursor-pointer items-center gap-1.5 rounded-field border border-base-300 px-1.5 py-1 text-xs hover:bg-base-200 has-[:checked]:border-base-content has-[:checked]:font-semibold has-[:focus-visible]:outline-2 has-[:focus-visible]:outline-offset-1 has-[:focus-visible]:outline-base-content">
                                <input
                                    type="radio"
                                    class="sr-only"
                                    name=color_name.clone()
                                    value=cid
                                    prop:checked=checked
                                    on:change=move |_| set(None, Box::new(move |l| l.color = cid.to_string()))
                                />
                                <span class=format!("{swatch} grid h-5 w-5 shrink-0 place-items-center rounded-full ring-1 ring-base-content/20") aria-hidden="true">
                                    {move || checked().then(|| view! { <span class="h-2 w-2 rounded-full bg-base-100 ring-1 ring-base-content/40"></span> })}
                                </span>
                                <span class="truncate">{*name}</span>
                            </label>
                        }
                    }).collect_view()}
                    <label
                        for=custom_id.clone()
                        class="flex cursor-pointer items-center gap-1.5 rounded-field border border-base-300 px-1.5 py-1 text-xs hover:bg-base-200 has-[:focus-visible]:outline-2 has-[:focus-visible]:outline-offset-1 has-[:focus-visible]:outline-base-content"
                        class=("border-base-content", is_custom)
                        class=("font-semibold", is_custom)
                    >
                        <input
                            type="color"
                            id=custom_id.clone()
                            class="mk-color-input h-5 w-5 shrink-0 cursor-pointer rounded-full"
                            aria-describedby="mk-color-custom-help"
                            prop:value=custom_value
                            on:input=move |ev| {
                                let v = event_target_value(&ev);
                                set(Some("look-color"), Box::new(move |l| l.color = v));
                            }
                            on:change=move |_| state.end_group()
                        />
                        <span class="truncate">"Otro color"</span>
                    </label>
                </div>
                <p id="mk-color-custom-help" class="mt-1.5 text-xs text-base-content/75">
                    "Los colores de la lista combinan con el tema. Con otro color, el texto encima se pone negro o blanco para que se lea bien."
                </p>
            </fieldset>
            {move || (!look.with(Look::is_default)).then(|| view! {
                <button
                    type="button"
                    class="btn btn-sm btn-ghost self-start"
                    on:click=move |_| set(None, Box::new(|l| *l = Look::default()))
                >
                    <Icon name="undo"/>
                    "Volver al diseño clásico"
                </button>
            })}
        </section>
    }
}

/// Clase de la muestra de cada color del tema (literal, para Tailwind).
fn swatch_class(id: &str) -> &'static str {
    match id {
        "secondary" => "bg-secondary",
        "accent" => "bg-accent",
        "neutral" => "bg-neutral",
        "info" => "bg-info",
        "success" => "bg-success",
        "warning" => "bg-warning",
        "error" => "bg-error",
        _ => "bg-primary",
    }
}

/// El bloque dibujado con el diseño `design`, achicado al ancho de la miniatura.
#[component]
fn Thumb(node: Node, design: u8, products: StoredValue<Vec<Product>>) -> impl IntoView {
    let state = expect_context::<EditorState>();
    let in_modal = state.in_modal_untracked();
    let layer_w = if in_modal {
        Layer::ProductModal.design_width()
    } else {
        Layer::Page(0).design_width()
    };
    let w = f64::from(node.size.w) * f64::from(layer_w) / f64::from(domain::GRID_COLS);
    let h = f64::from(node.size.h) * f64::from(domain::ROW_PX);
    // Los bloques anchos se muestran desde la izquierda, sin achicarlos
    // tanto que no se distinga el diseño.
    let scale = THUMB_W / w.clamp(1.0, THUMB_VIEW_MAX_W);
    let products = products.get_value();
    let product = in_modal.then(|| products.first().cloned().unwrap_or_else(sample_product));
    let ctx = RenderCtx {
        products,
        editing: true,
        product,
    };
    let mut node = node;
    node.look.design = design;
    // Otro id que el del lienzo: así las pestañas de la miniatura no forman
    // un grupo con las del bloque y no le cambian la pestaña abierta.
    node.id = format!("{}-thumb{design}", node.id);
    // Centrado a lo alto si entra; si no, se ve la parte de arriba.
    let top = ((THUMB_MAX_H - h * scale) / 2.0).max(0.0);
    let style = format!(
        "width:{w}px; height:{h}px; transform: translate(0, {top}px) scale({scale}); transform-origin: 0 0; padding: 8px; box-sizing: border-box;"
    );
    view! {
        <div class="pointer-events-none select-none" style=style inert>
            {render_widget(&node, &ctx)}
        </div>
    }
}
