//! Sección "Estilo y color" del inspector.
//!
//! Los cinco diseños se eligen mirando el bloque mismo: cada opción es una
//! miniatura del bloque dibujado con ese diseño, en el tema y con el color
//! de la tienda, y los cinco entran en una sola fila. Son radios nativos
//! (flechas para moverse, un solo punto de tabulación) con la marca de
//! elegido a la vista, no solo por color. Los colores son muestras redondas
//! con su nombre para lectores de pantalla y debajo, escrito.

use domain::{Layer, Look, Node, Product, DESIGNS, THEME_COLORS};
use leptos::prelude::*;
use widgets::{render_widget, sample_product, RenderCtx};

use crate::icons::Icon;
use crate::EditorState;

/// Ancho de cada miniatura, en píxeles (cinco en una fila del panel).
const THUMB_W: f64 = 58.0;
/// Alto máximo de una miniatura; lo que sobra del bloque se recorta.
const THUMB_MAX_H: f64 = 46.0;
/// Ancho máximo del bloque que entra en una miniatura, en píxeles.
const THUMB_VIEW_MAX_W: f64 = 900.0;

#[component]
pub(crate) fn BlockLook(
    id: String,
    /// Si la sección está abierta: las miniaturas se dibujan solo entonces,
    /// para no redibujarlas con cada tecla en Contenido.
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
    let mobile_id = format!("mk-design-mobile-{id}");
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
        <section class="flex flex-col gap-4">
            <fieldset>
                <legend class="mb-2 flex items-center gap-1 text-sm font-medium">
                    "Estilo"
                    <ui::HelpTip id=format!("tip-look-{id}") label="Estilo" text="Cada estilo cambia cómo se ve el bloque. Tus textos e imágenes no cambian."/>
                </legend>
                <div class="grid grid-cols-5 gap-1">
                    {DESIGNS.iter().map(|(n, name, hint)| {
                        let n = *n;
                        let checked = move || look.with(|l| l.design == n);
                        view! {
                            <label title=*hint class="group relative flex cursor-pointer flex-col items-stretch gap-1 rounded-field p-1 outline-2 outline-offset-0 outline-transparent hover:bg-base-200 has-[:checked]:bg-primary/5 has-[:checked]:outline-primary has-[:focus-visible]:outline-offset-2 has-[:focus-visible]:outline-base-content">
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
                                    class="mk-store overflow-hidden rounded-md border border-base-300 bg-base-200"
                                    style=move || format!("{}; height:{THUMB_MAX_H}px", store_vars())
                                    aria-hidden="true"
                                >
                                    {move || visible.get().then(|| node.get().map(|node| {
                                        view! { <Thumb node design=n products/> }
                                    }))}
                                </div>
                                <span class="flex items-center justify-center gap-0.5 text-center text-[11px] font-medium leading-tight" class=("text-primary", checked)>
                                    {move || checked().then(|| view! { <Icon name="check" class="h-3 w-3 shrink-0"/> })}
                                    {*name}
                                </span>
                            </label>
                        }
                    }).collect_view()}
                </div>
            </fieldset>
            <div class="flex flex-col gap-1">
                <label for=mobile_id.clone() class="flex items-center gap-1 text-sm font-medium">
                    "Diseño en celular"
                    <ui::HelpTip
                        id=format!("tip-mobile-{id}")
                        label="Diseño en celular"
                        text="Por defecto el celular usa el mismo diseño que escritorio. Elegí otro si querés que en pantallas angostas se vea distinto."
                    />
                </label>
                <select
                    id=mobile_id.clone()
                    class="select select-sm w-full"
                    prop:value=move || look.with(|l| l.design_m.to_string())
                    on:change=move |ev| {
                        let n = event_target_value(&ev).parse::<u8>().unwrap_or(0);
                        set(None, Box::new(move |l| l.design_m = n));
                    }
                >
                    <option value="0">"Igual que escritorio"</option>
                    {DESIGNS.iter().map(|(n, name, _)| view! { <option value=n.to_string()>{*name}</option> }).collect_view()}
                </select>
            </div>
            <fieldset>
                <legend class="mb-2 flex items-center gap-1 text-sm font-medium">
                    "Color"
                    <ui::HelpTip
                        id=format!("tip-color-{id}")
                        label="Color"
                        text="«Tienda», «Tienda 2» y «Tienda 3» son los colores que elegiste en Mi tienda. Con otro color, el texto encima se pone negro o blanco para que se lea bien."
                    />
                </legend>
                <div data-theme=theme class="flex flex-wrap gap-2.5 rounded-box bg-base-100 p-1 text-base-content">
                    {THEME_COLORS.iter().map(|(cid, name)| {
                        let cid: &'static str = cid;
                        let checked = move || look.with(|l| l.color == cid);
                        let swatch = swatch_class(cid);
                        view! {
                            <label
                                title=*name
                                class=format!("{swatch} grid h-6 w-6 cursor-pointer place-items-center rounded-full ring-1 ring-base-content/20 ring-offset-2 ring-offset-base-100 has-[:checked]:ring-2 has-[:checked]:ring-base-content has-[:focus-visible]:ring-2 has-[:focus-visible]:ring-base-content")
                            >
                                <input
                                    type="radio"
                                    class="sr-only"
                                    name=color_name.clone()
                                    value=cid
                                    aria-label=*name
                                    prop:checked=checked
                                    on:change=move |_| set(None, Box::new(move |l| l.color = cid.to_string()))
                                />
                                {move || checked().then(|| view! { <Icon name="check" class="h-3.5 w-3.5 text-base-100 drop-shadow"/> })}
                            </label>
                        }
                    }).collect_view()}
                    <label
                        for=custom_id.clone()
                        title="Otro color"
                        class="relative grid h-6 w-6 cursor-pointer place-items-center overflow-hidden rounded-full ring-1 ring-base-content/20 ring-offset-2 ring-offset-base-100 has-[:focus-visible]:ring-2 has-[:focus-visible]:ring-base-content"
                        class=("ring-2", is_custom)
                        class=("ring-base-content", is_custom)
                        style=move || if is_custom() {
                            format!("background:{}", custom_value())
                        } else {
                            "background:conic-gradient(#ef4444,#eab308,#22c55e,#06b6d4,#3b82f6,#d946ef,#ef4444)".to_string()
                        }
                    >
                        <input
                            type="color"
                            id=custom_id.clone()
                            class="mk-color-input absolute inset-0 h-full w-full cursor-pointer opacity-0"
                            aria-label="Otro color"
                            prop:value=custom_value
                            on:input=move |ev| {
                                let v = event_target_value(&ev);
                                set(Some("look-color"), Box::new(move |l| l.color = v));
                            }
                            on:change=move |_| state.end_group()
                        />
                    </label>
                </div>
                <p class="mt-1.5 text-xs text-base-content/75" aria-live="polite">
                    "Elegido: "
                    <span class="font-medium text-base-content">{move || look.with(|l| color_name_of(&l.color))}</span>
                </p>
                // En Clásico y Minimal el color casi no se nota si el bloque
                // no tiene botón: se avisa para que no parezca que no anda.
                {move || match look.with(|l| l.design) {
                    1 => Some("En el diseño Clásico el color se usa en el botón. Si el bloque no tiene botón, probá «Suave» o «Intenso»."),
                    5 => Some("En el diseño Minimal el color se usa en el botón y en la línea de los títulos. Para que se note más, probá «Suave» o «Intenso»."),
                    _ => None,
                }.map(|t| view! {
                    <p class="mt-1.5 rounded-field bg-base-200 px-2 py-1.5 text-xs text-base-content/80" role="note">{t}</p>
                })}
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

/// Nombre del color elegido: el del tema o "Otro color".
fn color_name_of(color: &str) -> String {
    THEME_COLORS
        .iter()
        .find(|(id, _)| *id == color)
        .map_or_else(|| "Otro color".to_string(), |(_, name)| (*name).to_string())
}

/// Clase de la muestra de cada color del tema (literal, para Tailwind).
pub(crate) fn swatch_class(id: &str) -> &'static str {
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
        // El ancho de la tienda (con "Ancho completo", el de siempre).
        state.doc.with_untracked(|d| match d.layout.max_width {
            0 => Layer::Page(0).design_width(),
            w => w,
        })
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
    node.look.design_m = 0;
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
