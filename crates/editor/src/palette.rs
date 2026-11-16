//! Paleta: un riel angosto con las categorías de bloques. Al elegir una se
//! abre, encima del lienzo, una galería con cada bloque dibujado de verdad
//! (en el tema y las letras de la tienda), para ver cómo queda antes de
//! agregarlo. Clic lo suma al final; arrastrarlo lo ubica donde se suelte.

use domain::{allowed_in, Layer, Node, Product, Widget, WIDGET_TYPES};
use leptos::ev::PointerEvent;
use leptos::prelude::*;
use wasm_bindgen::JsCast;
use widgets::{render_widget, sample_product, RenderCtx};

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
    ("Productos", "TagFilter", "tags", "Pastillas para filtrar"),
    ("Productos", "TagList", "tags", "Casillas al costado"),
    ("Productos", "TagCircles", "tags", "Círculos con foto"),
    ("Productos", "TagSearch", "tags", "Buscar y filtrar"),
    ("Productos", "TagTabs", "grid", "Productos por pestañas"),
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
    (
        "Contenido",
        "Contact",
        "contact",
        "WhatsApp, Instagram, email y horario",
    ),
    ("Pie", "Footer", "footer", "Datos y enlaces al final"),
];

/// Categorías del riel, con su ícono.
const GROUPS: &[(&str, &str)] = &[
    ("Del producto", "modal"),
    ("Encabezado", "banner"),
    ("Productos", "grid"),
    ("Contenido", "text"),
    ("Confianza", "benefits"),
    ("Pie", "footer"),
];

/// Lo que muestra la galería: la búsqueda (todas las categorías) o una.
const SEARCH: &str = "";

/// Medidas de cada miniatura de la galería, en píxeles.
const THUMB_W: f64 = 188.0;
const THUMB_H: f64 = 120.0;
/// Ancho máximo del bloque que entra en una miniatura: los bloques anchos
/// se dibujan a este ancho (se acomodan solos) para que se lean.
const THUMB_VIEW_MAX_W: f64 = 600.0;

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
pub(crate) fn Palette(products: StoredValue<Vec<Product>>) -> impl IntoView {
    let actions = expect_context::<Actions>();
    let ui = expect_context::<Ui>();
    let state = expect_context::<EditorState>();
    // Categoría abierta en la galería (`SEARCH` = buscar en todas).
    let open = RwSignal::new(None::<&'static str>);
    let query = RwSignal::new(String::new());
    let nav_ref = NodeRef::<leptos::html::Nav>::new();
    let search_ref = NodeRef::<leptos::html::Input>::new();

    // Cierra la galería y devuelve el foco a la categoría que la abrió.
    let close = move |refocus: bool| {
        let was = open.get_untracked();
        open.set(None);
        if refocus {
            if let Some(g) = was {
                if let Some(el) = document()
                    .get_element_by_id(&rail_id(g))
                    .and_then(|e| e.dyn_into::<web_sys::HtmlElement>().ok())
                {
                    _ = el.focus();
                }
            }
        }
    };
    // Al empezar a arrastrar se cierra, para ver dónde cae el bloque.
    Effect::new(move |_| {
        if ui.new_drag.with(|d| d.as_ref().is_some_and(|d| d.active)) {
            open.set(None);
        }
    });
    // Tocar fuera del riel y la galería la cierra.
    let outside = window_event_listener(leptos::ev::pointerdown, move |ev| {
        if open.get_untracked().is_none() {
            return;
        }
        let inside = ev
            .target()
            .and_then(|t| t.dyn_into::<web_sys::Node>().ok())
            .zip(nav_ref.get_untracked())
            .is_some_and(|(t, n)| n.contains(Some(&t)));
        if !inside {
            open.set(None);
        }
    });
    on_cleanup(move || outside.remove());
    // Al pasar a la ficha del producto (o volver) cambia lo que se ofrece.
    Effect::new(move |prev: Option<Layer>| {
        let layer = state.layer.get();
        if prev.is_some_and(|p| p != layer) {
            open.set(None);
        }
        layer
    });

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
        open.set(None);
        actions.add(type_name);
    };
    // En la ficha del producto solo se ofrecen los bloques que van ahí.
    let allowed = move |type_name: &str| allowed_in(type_name, state.layer.get());
    let matches = move |type_name: &str, desc: &str| {
        if !allowed(type_name) {
            return false;
        }
        // La búsqueda filtra solo en "Buscar"; una categoría muestra todos.
        if open.get() != Some(SEARCH) {
            return true;
        }
        let q = fold(query.get().trim());
        q.is_empty() || fold(label_of(type_name)).contains(&q) || fold(desc).contains(&q)
    };
    let in_modal = move || state.layer.get() == Layer::ProductModal;
    let toggle = move |g: &'static str| {
        if open.get_untracked() == Some(g) {
            open.set(None);
        } else {
            open.set(Some(g));
            if g == SEARCH {
                request_animation_frame(move || {
                    if let Some(el) = search_ref.get_untracked() {
                        _ = el.focus();
                    }
                });
            }
        }
    };
    let on_key = move |ev: web_sys::KeyboardEvent| {
        if ev.key() == "Escape" && open.get_untracked().is_some() {
            ev.stop_propagation();
            ev.prevent_default();
            close(true);
        }
    };

    let theme = move || state.doc.with(|d| d.theme.clone());
    let store_vars = move || state.doc.with(|d| d.typography.css_vars());

    // Un bloque de la galería: su miniatura real, nombre y descripción.
    let card = move |type_name: &'static str, desc: &'static str| {
        view! {
            <li class:hidden=move || !matches(type_name, desc)>
                <button
                    type="button"
                    class="group flex h-full w-full cursor-grab select-none flex-col items-stretch overflow-hidden rounded-box border border-base-300 bg-base-100 text-left transition duration-150 hover:border-primary hover:shadow-md focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-primary"
                    class:opacity-50=move || ui.new_drag.with(|d| d.as_ref().is_some_and(|d| d.active && d.type_name == type_name))
                    title=move || if in_modal() { "Clic para agregarlo al final de la ficha, o arrastralo a la ficha" } else { "Clic para agregarlo al final, o arrastralo al lienzo" }
                    on:pointerdown=move |ev| on_pointer_down(ev, type_name)
                    on:click=move |_| on_click(type_name)
                >
                    <div
                        data-theme=theme
                        class="mk-store relative overflow-hidden border-b border-base-300 bg-base-200"
                        style=move || format!("{}; height:{THUMB_H}px", store_vars())
                        aria-hidden="true"
                    >
                        <Thumb type_name products/>
                        <span class="badge badge-primary badge-sm absolute right-1.5 top-1.5 gap-0.5 font-sans opacity-0 shadow transition group-hover:opacity-100 group-focus-visible:opacity-100">
                            <Icon name="plus" class="h-3 w-3"/>
                            "Agregar"
                        </span>
                    </div>
                    <span class="flex flex-col gap-0.5 p-2">
                        <span class="text-sm font-medium leading-tight">{label_of(type_name)}</span>
                        <span class="text-xs leading-snug text-base-content/75">{desc}</span>
                    </span>
                </button>
            </li>
        }
    };

    view! {
        <nav
            node_ref=nav_ref
            class="relative z-30 flex w-[76px] shrink-0 flex-col border-r border-base-300 bg-base-100"
            aria-label="Agregar bloques"
            on:keydown=on_key
        >
            <ul class="flex flex-col items-center gap-1 overflow-y-auto px-1.5 py-2">
                <li class="w-full">
                    <RailButton group=SEARCH icon="search" label="Buscar" open toggle/>
                </li>
                <li class="mx-auto my-1 w-10 border-t border-base-300" aria-hidden="true"></li>
                {GROUPS
                    .iter()
                    .map(|&(group, icon)| {
                        let any = move || BLOCKS.iter().any(|b| b.0 == group && allowed(b.1));
                        view! {
                            <li class="w-full" class:hidden=move || !any()>
                                <RailButton group icon label=group open toggle/>
                            </li>
                        }
                    })
                    .collect_view()}
            </ul>
            {move || open.get().map(|current| {
                let title = if current == SEARCH {
                    if in_modal() { "Agregá a la ficha" } else { "Agregá un bloque" }
                } else {
                    current
                };
                let count = move || BLOCKS.iter().filter(|b| (current == SEARCH || b.0 == current) && matches(b.1, b.3)).count();
                view! {
                    <div
                        id="mk-gallery"
                        class="absolute inset-y-0 left-full flex w-[416px] flex-col border-r border-base-300 bg-base-100 shadow-2xl"
                        role="region"
                        aria-labelledby="mk-gallery-title"
                    >
                        <div class="flex flex-col gap-2 border-b border-base-300 p-3">
                            <div class="flex items-center gap-2">
                                <h2 id="mk-gallery-title" class="font-semibold">{title}</h2>
                                <span class="text-xs text-base-content/75">
                                    {move || match count() { 1 => "1 bloque".to_string(), n => format!("{n} bloques") }}
                                </span>
                                <button
                                    type="button"
                                    class="btn btn-ghost btn-sm btn-square ml-auto"
                                    aria-label="Cerrar"
                                    title="Cerrar (Esc)"
                                    on:click=move |_| close(true)
                                >
                                    <Icon name="close"/>
                                </button>
                            </div>
                            {(current == SEARCH).then(|| view! {
                                <label class="input input-sm w-full">
                                    <span class="sr-only">"Buscar bloque"</span>
                                    <Icon name="search" class="h-4 w-4 opacity-70"/>
                                    <input
                                        node_ref=search_ref
                                        type="search"
                                        placeholder="Buscar bloque…"
                                        prop:value=move || query.get()
                                        on:input=move |ev| query.set(event_target_value(&ev))
                                    />
                                </label>
                            })}
                            <p class="text-xs text-base-content/75">
                                {move || if in_modal() {
                                    "Clic para sumarlo al final de la ficha, o arrastralo al lugar donde lo quieras."
                                } else {
                                    "Clic para sumarlo al final de la página, o arrastralo al lugar del lienzo donde lo quieras."
                                }}
                            </p>
                        </div>
                        <div class="flex-1 overflow-y-auto p-3">
                            {GROUPS
                                .iter()
                                .filter(|g| current == SEARCH || g.0 == current)
                                .map(|&(group, _)| {
                                    let any = move || BLOCKS.iter().any(|b| b.0 == group && matches(b.1, b.3));
                                    let heading_id = format!("mk-group-{}", fold(group).replace(' ', "-"));
                                    view! {
                                        <section class="mb-4 last:mb-0" class:hidden=move || !any() aria-labelledby=heading_id.clone()>
                                            <h3
                                                id=heading_id.clone()
                                                class="mb-2 text-xs font-semibold uppercase tracking-wide text-base-content/75"
                                                class:sr-only=move || current != SEARCH
                                            >
                                                {group}
                                            </h3>
                                            <ul class="grid grid-cols-2 gap-3">
                                                {BLOCKS
                                                    .iter()
                                                    .filter(|b| b.0 == group)
                                                    .map(|&(_, type_name, _, desc)| card(type_name, desc))
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
                    </div>
                }
            })}
        </nav>
    }
}

fn rail_id(group: &str) -> String {
    if group.is_empty() {
        "mk-rail-buscar".to_string()
    } else {
        format!("mk-rail-{}", fold(group).replace(' ', "-"))
    }
}

/// Un botón del riel: abre o cierra su categoría en la galería.
#[component]
fn RailButton(
    group: &'static str,
    icon: &'static str,
    label: &'static str,
    open: RwSignal<Option<&'static str>>,
    toggle: impl Fn(&'static str) + Copy + Send + Sync + 'static,
) -> impl IntoView {
    let on = move || open.get() == Some(group);
    view! {
        <button
            type="button"
            id=rail_id(group)
            class="flex w-full flex-col items-center gap-1 rounded-field px-1 py-2 text-center text-[11px] leading-tight text-base-content/80 transition hover:bg-base-200 focus-visible:outline-2 focus-visible:outline-offset-1 focus-visible:outline-primary"
            class=("bg-primary/10", on)
            class=("text-primary", on)
            class=("font-semibold", on)
            aria-expanded=move || on().to_string()
            aria-controls="mk-gallery"
            on:click=move |_| toggle(group)
        >
            <Icon name=icon class="h-5 w-5"/>
            {label}
        </button>
    }
}

/// El bloque recién agregado, con sus valores de fábrica, achicado entero a
/// la miniatura y centrado.
#[component]
fn Thumb(type_name: &'static str, products: StoredValue<Vec<Product>>) -> impl IntoView {
    let state = expect_context::<EditorState>();
    let Some(widget) = Widget::defaults(type_name) else {
        return ().into_any();
    };
    let in_modal = state.in_modal_untracked();
    let layer_w = if in_modal {
        Layer::ProductModal.design_width()
    } else {
        Layer::Page(0).design_width()
    };
    let products = products.get_value();
    let mut node = Node::new(format!("mk-pal-{type_name}"), widget);
    // Un producto destacado sin elegir mostraría un aviso: se ve con el primero.
    if let Widget::FeaturedProduct(p) = &mut node.widget {
        if let Some(first) = products.first() {
            p.product_id = first.id.clone();
        }
    }
    let w = (f64::from(node.size.w) * f64::from(layer_w) / f64::from(domain::GRID_COLS))
        .clamp(1.0, THUMB_VIEW_MAX_W);
    let h = f64::from(node.size.h) * f64::from(domain::ROW_PX);
    // Entero si entra; si es muy alto, se ve la parte de arriba.
    let scale = THUMB_W / w;
    let top = ((THUMB_H - h * scale) / 2.0).max(0.0);
    let product = in_modal.then(|| products.first().cloned().unwrap_or_else(sample_product));
    let ctx = RenderCtx {
        products,
        editing: true,
        product,
    };
    let style = format!(
        "width:{w}px; height:{h}px; transform: translate(0, {top}px) scale({scale}); transform-origin: 0 0; padding: 8px; box-sizing: border-box;"
    );
    view! {
        <div class="pointer-events-none select-none" style=style inert>
            {render_widget(&node, &ctx)}
        </div>
    }
    .into_any()
}
