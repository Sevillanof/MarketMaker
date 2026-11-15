//! Paleta: los bloques que se pueden agregar, en miniaturas agrupadas y con
//! buscador.

use domain::{allowed_in, Layer, WIDGET_TYPES};
use leptos::ev::PointerEvent;
use leptos::prelude::*;
use wasm_bindgen::JsCast;

use crate::icons::Icon;
use crate::{Actions, EditorState, NewDrag, Ui};

/// Grupo, ícono y descripción breve de cada tipo de bloque, en el orden en
/// que se muestran.
pub(crate) const BLOCKS: &[(&str, &str, &str, &str)] = &[
    (
        "Del producto",
        "ProductImage",
        "image",
        "La foto del producto que se abre",
    ),
    (
        "Del producto",
        "ProductName",
        "type",
        "Su nombre, como título",
    ),
    ("Del producto", "ProductPrice", "tag", "Su precio"),
    (
        "Del producto",
        "ProductDescription",
        "text",
        "El texto que lo describe",
    ),
    (
        "Del producto",
        "ProductButton",
        "button",
        "Un botón para consultar o comprar",
    ),
    ("Encabezado", "Navbar", "navbar", "Logo, nombre y enlaces"),
    ("Encabezado", "Banner", "banner", "Imagen grande con título"),
    (
        "Encabezado",
        "Carousel",
        "carousel",
        "Varias imágenes que pasan solas",
    ),
    ("Productos", "ProductGrid", "grid", "Tus productos en filas"),
    (
        "Productos",
        "FeaturedProduct",
        "featured",
        "Un producto con foto y precio",
    ),
    (
        "Productos",
        "Categories",
        "categories",
        "Accesos a tus categorías",
    ),
    (
        "Productos",
        "Countdown",
        "countdown",
        "Promoción con fecha de fin",
    ),
    ("Contenido", "Text", "text", "Título y párrafo"),
    ("Contenido", "Card", "card", "Imagen, texto y botón"),
    ("Contenido", "Image", "image", "Una imagen sola"),
    (
        "Contenido",
        "HoverGallery",
        "gallery",
        "Fotos que cambian con el mouse",
    ),
    ("Contenido", "Tabs", "tabs", "Información en pestañas"),
    ("Contenido", "Faq", "faq", "Preguntas y respuestas"),
    (
        "Confianza",
        "Testimonials",
        "testimonials",
        "Opiniones de clientes",
    ),
    (
        "Confianza",
        "Benefits",
        "benefits",
        "Envíos, pagos, cambios",
    ),
    ("Confianza", "Logos", "logos", "Marcas que vendés"),
    ("Pie", "Footer", "footer", "Datos y enlaces al final"),
];

const GROUPS: &[&str] = &[
    "Del producto",
    "Encabezado",
    "Productos",
    "Contenido",
    "Confianza",
    "Pie",
];

/// Ícono y descripción de un tipo de bloque.
pub(crate) fn block_info(type_name: &str) -> (&'static str, &'static str) {
    BLOCKS
        .iter()
        .find(|b| b.1 == type_name)
        .map_or(("text", ""), |b| (b.2, b.3))
}

pub(crate) fn label_of(type_name: &str) -> &'static str {
    WIDGET_TYPES
        .iter()
        .find(|(t, _)| *t == type_name)
        .map_or("Bloque", |(_, l)| *l)
}

/// Texto sin mayúsculas ni tildes, para buscar.
fn fold(s: &str) -> String {
    s.to_lowercase()
        .chars()
        .map(|c| match c {
            'á' => 'a',
            'é' => 'e',
            'í' => 'i',
            'ó' => 'o',
            'ú' | 'ü' => 'u',
            c => c,
        })
        .collect()
}

#[component]
pub(crate) fn Palette() -> impl IntoView {
    let actions = expect_context::<Actions>();
    let ui = expect_context::<Ui>();
    let state = expect_context::<EditorState>();
    // Apretar con el mouse prepara un arrastre; el lienzo lo sigue desde ahí
    // (ver `canvas::Canvas`) y decide si fue un clic o un arrastre.
    let on_pointer_down = move |ev: PointerEvent, type_name: &'static str| {
        if ev.button() != 0 || ev.pointer_type() == "touch" {
            return;
        }
        // Sin selección de texto mientras se arrastra; el foco se da a mano.
        ev.prevent_default();
        if let Some(el) = ev
            .current_target()
            .and_then(|t| t.dyn_into::<web_sys::HtmlElement>().ok())
        {
            let opts = web_sys::FocusOptions::new();
            opts.set_prevent_scroll(true);
            _ = el.focus_with_options(&opts);
        }
        let (x, y) = (f64::from(ev.client_x()), f64::from(ev.client_y()));
        ui.swallow_click.set_value(false);
        ui.new_drag.set(Some(NewDrag {
            type_name,
            start_x: x,
            start_y: y,
            x,
            y,
            active: false,
        }));
    };
    let on_click = move |type_name: &'static str| {
        if ui.swallow_click.get_value() {
            ui.swallow_click.set_value(false);
            return;
        }
        actions.add(type_name);
    };
    let query = RwSignal::new(String::new());
    // En la ficha del producto solo se ofrecen los bloques que van ahí.
    let matches = move |type_name: &str, desc: &str| {
        if !allowed_in(type_name, state.layer.get()) {
            return false;
        }
        let q = fold(query.get().trim());
        q.is_empty() || fold(label_of(type_name)).contains(&q) || fold(desc).contains(&q)
    };
    let in_modal = move || state.layer.get() == Layer::ProductModal;

    view! {
        <nav class="flex w-72 shrink-0 flex-col border-r border-base-300 bg-base-100" aria-labelledby="mk-palette-title">
            <div class="flex flex-col gap-2 border-b border-base-300 p-3">
                <h2 id="mk-palette-title" class="font-semibold">
                    {move || if in_modal() { "Agregá a la ficha" } else { "Agregá un bloque" }}
                </h2>
                <p class="text-xs text-base-content/75">
                    {move || if in_modal() {
                        "Bloques para la ficha del producto. Los de \"Del producto\" muestran la foto, el nombre o el precio del producto que se abre."
                    } else {
                        "Hacé clic en uno y se suma al final de la página, o arrastralo al lugar del lienzo donde lo quieras."
                    }}
                </p>
                <label class="input input-sm w-full">
                    <span class="sr-only">"Buscar bloque"</span>
                    <svg class="h-4 w-4 opacity-70" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" aria-hidden="true"><path d="M11 4a7 7 0 1 0 0 14 7 7 0 0 0 0-14M20 20l-4-4"/></svg>
                    <input
                        type="search"
                        placeholder="Buscar bloque…"
                        prop:value=move || query.get()
                        on:input=move |ev| query.set(event_target_value(&ev))
                    />
                </label>
            </div>
            <div class="flex-1 overflow-y-auto p-3">
                {GROUPS
                    .iter()
                    .map(|group| {
                        let group = *group;
                        let any = move || BLOCKS.iter().any(|b| b.0 == group && matches(b.1, b.3));
                        let heading_id = format!("mk-group-{}", fold(group));
                        let heading_for = heading_id.clone();
                        view! {
                            <section class="mb-4" class:hidden=move || !any() aria-labelledby=heading_for>
                                <h3 id=heading_id class="mb-2 text-xs font-semibold uppercase tracking-wide text-base-content/75">{group}</h3>
                                <ul class="grid grid-cols-2 gap-2">
                                    {BLOCKS
                                        .iter()
                                        .filter(|b| b.0 == group)
                                        .map(|&(_, type_name, icon, desc)| view! {
                                            <li class:hidden=move || !matches(type_name, desc)>
                                                <button
                                                    type="button"
                                                    class="flex h-full w-full cursor-grab select-none flex-col items-stretch gap-1.5 rounded-box border border-base-300 bg-base-100 p-2 text-left transition duration-150 hover:border-primary hover:bg-primary/5 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-primary"
                                                    class:opacity-50=move || ui.new_drag.with(|d| d.as_ref().is_some_and(|d| d.active && d.type_name == type_name))
                                                    class:scale-95=move || ui.new_drag.with(|d| d.as_ref().is_some_and(|d| d.active && d.type_name == type_name))
                                                    title=move || if in_modal() { "Clic para agregarlo al final de la ficha, o arrastralo a la ficha" } else { "Clic para agregarlo al final, o arrastralo al lienzo" }
                                                    on:pointerdown=move |ev| on_pointer_down(ev, type_name)
                                                    on:click=move |_| on_click(type_name)
                                                >
                                                    <span class="flex h-14 items-center justify-center rounded-field bg-base-200 text-base-content/70" aria-hidden="true">
                                                        <Icon name=icon class="h-8 w-8"/>
                                                    </span>
                                                    <span class="text-sm font-medium leading-tight">{label_of(type_name)}</span>
                                                    <span class="text-xs leading-snug text-base-content/75">{desc}</span>
                                                </button>
                                            </li>
                                        })
                                        .collect_view()}
                                </ul>
                            </section>
                        }
                    })
                    .collect_view()}
                <p class="text-sm text-base-content/75" class:hidden=move || BLOCKS.iter().any(|b| matches(b.1, b.3))>
                    "No hay bloques con ese nombre."
                </p>
            </div>
        </nav>
    }
}
