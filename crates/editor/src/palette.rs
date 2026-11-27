//! Paleta: una tira angosta y siempre visible con solo miniaturas de los
//! bloques (sin textos). Arriba, un selector con el nombre de la categoría.
//! Al pasar el mouse (o enfocar con el teclado) una miniatura se abre una
//! lupa a su costado con el bloque grande, dibujado de verdad en el tema y las
//! letras de la tienda. Clic lo suma arriba de todo; arrastrarlo lo ubica donde se
//! suelte.

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
        "ProductSheet",
        "image",
        "La ficha completa del producto",
    ),
    (
        "Del producto",
        "ProductButton",
        "button",
        "Un botón para consultar o comprar",
    ),
    (
        "Encabezado",
        "Announcement",
        "announce",
        "Una franja con un mensaje corto",
    ),
    (
        "Encabezado",
        "Coupon",
        "ticket",
        "Un código de descuento para copiar",
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
        "ProductCarousel",
        "rail",
        "Productos en una fila que se desliza",
    ),
    ("Productos", "TagTabs", "grid", "Productos por pestañas"),
    (
        "Productos",
        "FeaturedProduct",
        "featured",
        "Un producto con foto y precio",
    ),
    (
        "Productos",
        "Countdown",
        "countdown",
        "Promoción con fecha de fin",
    ),
    (
        "Productos",
        "Categories",
        "categories",
        "Fotos que llevan a donde quieras",
    ),
    (
        "Filtros y categorías",
        "TagSearch",
        "tags",
        "Buscar y filtrar",
    ),
    (
        "Filtros y categorías",
        "TagFilter",
        "tags",
        "Pastillas para filtrar",
    ),
    (
        "Filtros y categorías",
        "TagList",
        "tags",
        "Casillas al costado",
    ),
    (
        "Filtros y categorías",
        "TagCircles",
        "tags",
        "Círculos con foto",
    ),
    (
        "Filtros y categorías",
        "CategoryTabs",
        "categories",
        "Botones para filtrar por categoría",
    ),
    (
        "Filtros y categorías",
        "CategorySide",
        "categories",
        "Categorías al costado",
    ),
    (
        "Filtros y categorías",
        "CategoryCards",
        "categories",
        "Categorías con foto",
    ),
    (
        "Filtros y categorías",
        "CategoryDrop",
        "categories",
        "Menú desplegable de categorías",
    ),
    (
        "Filtros y categorías",
        "CategoryPage",
        "categories",
        "Categoría con subcategorías",
    ),
    ("Contenido", "Text", "text", "Título y párrafo"),
    ("Contenido", "Steps", "steps", "Cómo comprar, paso a paso"),
    ("Contenido", "Social", "social", "Redes y botón de WhatsApp"),
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
    ("Confianza", "Stats", "stats", "Números que dan confianza"),
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
    ("Filtros y categorías", "tags"),
    ("Contenido", "text"),
    ("Confianza", "benefits"),
    ("Pie", "footer"),
];

/// Lo que muestra la tira: la búsqueda (todas las categorías) o una.
const SEARCH: &str = "";

/// Medidas de cada miniatura de la tira, en píxeles.
const THUMB_W: f64 = 136.0;
const THUMB_H: f64 = 64.0;
/// Medidas de la lupa (el bloque grande que sale al pasar el mouse).
const ZOOM_W: f64 = 440.0;
const ZOOM_H_MAX: f64 = 360.0;
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

/// Escala y alto visible de un bloque dibujado a `tw` píxeles de ancho (con
/// `th_max` de alto máximo: si es más alto se ve la parte de arriba).
fn thumb_size(
    type_name: &str,
    tw: f64,
    th_max: f64,
    in_modal: bool,
) -> Option<(f64, f64, f64, f64)> {
    let widget = Widget::defaults(type_name)?;
    let layer_w = if in_modal {
        Layer::ProductModal.design_width()
    } else {
        Layer::Page(0).design_width()
    };
    let node = Node::new("mk-pal-size", widget);
    let w = (f64::from(node.size.w) * f64::from(layer_w) / f64::from(domain::GRID_COLS))
        .clamp(1.0, THUMB_VIEW_MAX_W);
    let h = f64::from(node.size.h) * f64::from(domain::ROW_PX);
    let scale = tw / w;
    Some((w, h, scale, (h * scale).min(th_max)))
}

/// Qué miniatura tiene la lupa abierta y a qué altura (dentro de la tira).
#[derive(Clone, Copy)]
struct Hover {
    type_name: &'static str,
    /// Altura en la ventana.
    top: f64,
    /// Borde izquierdo del panel: la lupa se abre hacia el lienzo.
    edge: f64,
}

#[component]
pub(crate) fn Palette(products: StoredValue<Vec<Product>>) -> impl IntoView {
    let actions = expect_context::<Actions>();
    let ui = expect_context::<Ui>();
    let state = expect_context::<EditorState>();
    // Categoría elegida (`SEARCH` = buscar en todas).
    let cat = RwSignal::new("Encabezado");
    let menu_open = RwSignal::new(false);
    let hover = RwSignal::new(None::<Hover>);
    // Las miniaturas son los bloques dibujados de verdad: se arman recién en
    // el navegador, así no hay nada que no coincida con lo que vino del servidor.
    let mounted = RwSignal::new(false);
    Effect::new(move |_| mounted.set(true));
    let query = RwSignal::new(String::new());
    let nav_ref = NodeRef::<leptos::html::Nav>::new();
    let head_ref = NodeRef::<leptos::html::Div>::new();
    let trigger_ref = NodeRef::<leptos::html::Button>::new();
    let search_ref = NodeRef::<leptos::html::Input>::new();

    // En la ficha del producto solo se ofrecen los bloques que van ahí.
    let allowed = move |type_name: &str| allowed_in(type_name, state.layer.get());
    let count_of = move |group: &str| {
        BLOCKS
            .iter()
            .filter(|b| b.0 == group && allowed(b.1))
            .count()
    };
    let in_modal = move || state.layer.get() == Layer::ProductModal;

    // Si la categoría elegida no tiene bloques acá (por ejemplo, al pasar a
    // la ficha del producto), se pasa a la primera que sí.
    Effect::new(move |_| {
        let current = cat.get();
        if current != SEARCH && count_of(current) == 0 {
            if let Some(&(g, _)) = GROUPS.iter().find(|(g, _)| count_of(g) > 0) {
                cat.set(g);
            }
        }
    });
    // Al empezar a arrastrar se cierra la lupa, para ver dónde cae el bloque.
    Effect::new(move |_| {
        if ui.new_drag.with(|d| d.as_ref().is_some_and(|d| d.active)) {
            hover.set(None);
            menu_open.set(false);
        }
    });
    // Tocar fuera del selector de categorías lo cierra.
    let outside = window_event_listener(leptos::ev::pointerdown, move |ev| {
        if !menu_open.get_untracked() {
            return;
        }
        let inside = ev
            .target()
            .and_then(|t| t.dyn_into::<web_sys::Node>().ok())
            .zip(head_ref.get_untracked())
            .is_some_and(|(t, n)| n.contains(Some(&t)));
        if !inside {
            menu_open.set(false);
        }
    });
    on_cleanup(move || outside.remove());

    let choose = move |g: &'static str| {
        cat.set(g);
        menu_open.set(false);
        hover.set(None);
        if g == SEARCH {
            request_animation_frame(move || {
                if let Some(el) = search_ref.get_untracked() {
                    _ = el.focus();
                }
            });
        } else if let Some(el) = trigger_ref.get_untracked() {
            _ = el.focus();
        }
    };

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
        hover.set(None);
        actions.add(type_name);
    };
    // Abre la lupa a la altura de la miniatura (mouse o foco del teclado).
    let show_zoom = move |target: Option<web_sys::HtmlElement>, type_name: &'static str| {
        if ui.new_drag.with(|d| d.is_some()) {
            return;
        }
        let (Some(el), Some(nav)) = (target, nav_ref.get_untracked()) else {
            return;
        };
        let r = el.get_bounding_client_rect();
        let n = nav.get_bounding_client_rect();
        let vh = window()
            .inner_height()
            .ok()
            .and_then(|v| v.as_f64())
            .unwrap_or(800.0);
        let card_h = thumb_size(type_name, ZOOM_W, ZOOM_H_MAX, state.in_modal_untracked())
            .map_or(200.0, |(_, _, _, h)| h)
            + 40.0
            + 56.0;
        let want = r.top() + r.height() / 2.0 - card_h / 2.0;
        let top = want.clamp(8.0, (vh - card_h - 8.0).max(8.0));
        hover.set(Some(Hover {
            type_name,
            top,
            edge: n.left(),
        }));
    };
    let matches = move |type_name: &str, desc: &str, group: &str| {
        if !allowed(type_name) {
            return false;
        }
        let current = cat.get();
        // La búsqueda filtra solo en "Buscar"; una categoría muestra las suyas.
        if current == SEARCH {
            let q = fold(query.get().trim());
            return q.is_empty()
                || fold(label_of(type_name)).contains(&q)
                || fold(desc).contains(&q)
                || keywords(type_name)
                    .split(' ')
                    .any(|k| k.starts_with(&q) || (q.len() > 3 && q.contains(k)));
        }
        group == current
    };
    let on_key = move |ev: web_sys::KeyboardEvent| {
        if ev.key() == "Escape" && (menu_open.get_untracked() || hover.get_untracked().is_some()) {
            ev.stop_propagation();
            ev.prevent_default();
            if menu_open.get_untracked() {
                menu_open.set(false);
                if let Some(el) = trigger_ref.get_untracked() {
                    _ = el.focus();
                }
            }
            hover.set(None);
        }
    };

    // Una miniatura de la tira, con su nombre y para qué sirve debajo (la
    // lupa la muestra en grande al pasar el mouse).
    let card = move |type_name: &'static str, desc: &'static str| {
        view! {
            <li>
                <button
                    type="button"
                    class="group block w-full cursor-grab select-none rounded-field outline-primary transition hover:outline-2 hover:outline-offset-2 focus-visible:outline-2 focus-visible:outline-offset-2"
                    class:opacity-50=move || ui.new_drag.with(|d| d.as_ref().is_some_and(|d| d.active && d.type_name == type_name))
                    aria-label=format!("{}: {}", label_of(type_name), desc)
                    on:pointerdown=move |ev| on_pointer_down(ev, type_name)
                    on:click=move |_| on_click(type_name)
                    on:pointerenter=move |ev| show_zoom(ev.current_target().and_then(|t| t.dyn_into::<web_sys::HtmlElement>().ok()), type_name)
                    on:pointerleave=move |_| hover.set(None)
                    on:focus=move |ev| show_zoom(ev.current_target().and_then(|t| t.dyn_into::<web_sys::HtmlElement>().ok()), type_name)
                    on:blur=move |_| hover.set(None)
                >
                    <span class=STAGE style=format!("height:{}px", THUMB_H + 12.0) aria-hidden="true">
                        <Thumb type_name products tw=THUMB_W th_max=THUMB_H/>
                    </span>
                    <span class="mt-1 block px-0.5 pb-1.5 text-left leading-tight" aria-hidden="true">
                        <span class="block text-[13px] font-semibold">{label_of(type_name)}</span>
                        <span class="mt-0.5 block text-[11.5px] text-base-content/65">{desc}</span>
                    </span>
                </button>
            </li>
        }
    };

    view! {
        <nav
            node_ref=nav_ref
            class="relative z-30 flex min-h-0 w-full flex-1 flex-col bg-base-100"
            aria-label="Agregar bloques"
            on:keydown=on_key
        >
            <div node_ref=head_ref class="relative border-b border-base-300 p-2.5">
                <button
                    node_ref=trigger_ref
                    type="button"
                    id="mk-cat-trigger"
                    class="btn btn-sm w-full justify-start gap-2 border-base-300 bg-base-100 font-semibold"
                    aria-haspopup="menu"
                    aria-expanded=move || menu_open.get().to_string()
                    aria-controls="mk-cat-menu"
                    on:click=move |_| menu_open.update(|o| *o = !*o)
                >
                    {move || view! { <Icon name=cat_icon(cat.get()) class="h-4 w-4"/> }}
                    <span class="flex-1 truncate text-left">{move || cat_label(cat.get())}</span>
                    <Icon name="chevron-down" class="h-4 w-4 opacity-70"/>
                </button>
                {move || menu_open.get().then(|| view! {
                    <div
                        id="mk-cat-menu"
                        role="menu"
                        aria-label="Categorías de bloques"
                        class="absolute inset-x-2.5 top-full z-10 mt-1 rounded-box border border-base-300 bg-base-100 p-1 shadow-xl"
                    >
                        {std::iter::once((SEARCH, "search"))
                            .chain(GROUPS.iter().copied())
                            .map(|(g, icon)| {
                                let on = move || cat.get() == g;
                                view! {
                                    <button
                                        type="button"
                                        role="menuitemradio"
                                        aria-checked=move || on().to_string()
                                        class="flex w-full items-center gap-2.5 rounded-field px-2.5 py-2 text-left text-sm hover:bg-base-200 focus-visible:outline-2 focus-visible:outline-primary"
                                        class=("bg-primary/10", on)
                                        class=("text-primary", on)
                                        class=("font-semibold", on)
                                        class:hidden=move || g != SEARCH && count_of(g) == 0
                                        on:click=move |_| choose(g)
                                    >
                                        <Icon name=icon class="h-5 w-5"/>
                                        <span class="flex-1">{cat_label(g)}</span>
                                        <span class="text-xs text-base-content/60">
                                            {move || if g == SEARCH { String::new() } else { count_of(g).to_string() }}
                                        </span>
                                    </button>
                                }
                            })
                            .collect_view()}
                    </div>
                })}
                {move || (cat.get() == SEARCH).then(|| view! {
                    <label class="input input-sm mt-2 w-full">
                        <span class="sr-only">"Buscar bloque"</span>
                        <Icon name="search" class="h-4 w-4 opacity-70"/>
                        <input
                            node_ref=search_ref
                            type="search"
                            placeholder="Buscar…"
                            prop:value=move || query.get()
                            on:input=move |ev| query.set(event_target_value(&ev))
                        />
                    </label>
                })}
            </div>
            <ul class="flex min-h-0 flex-1 flex-col gap-2.5 overflow-y-auto p-3.5">
                // Solo se dibujan los bloques de la categoría elegida (o de la
                // búsqueda): cada miniatura es el bloque entero.
                {move || {
                    if !mounted.get() {
                        return ().into_any();
                    }
                    let shown: Vec<_> = BLOCKS.iter().filter(|b| matches(b.1, b.3, b.0)).collect();
                    if shown.is_empty() {
                        view! { <li class="text-sm text-base-content/75">"Todavía no hay un bloque así. Probá con «texto», «imagen», «productos» o «botón»."</li> }.into_any()
                    } else {
                        shown
                            .into_iter()
                            .map(|&(_, type_name, _, desc)| card(type_name, desc))
                            .collect_view()
                            .into_any()
                    }
                }}
            </ul>
            <p class="border-t border-base-300 px-3.5 py-2 text-xs text-base-content/75">
                {move || if in_modal() { "Clic o arrastrá a la ficha" } else { "Clic o arrastrá al lienzo" }}
            </p>
            {move || hover.get().map(|h| {
                let (_, desc) = block_info(h.type_name);
                view! {
                    <div
                        class="pointer-events-none fixed z-40 overflow-hidden rounded-box border border-base-300 bg-base-100 shadow-2xl"
                        style=format!("top:{}px; left:{}px; width:{}px", h.top, h.edge - 8.0 - (ZOOM_W + 80.0), ZOOM_W + 80.0)
                        aria-hidden="true"
                    >
                        <div class=ZOOM_STAGE>
                            <Thumb type_name=h.type_name products tw=ZOOM_W th_max=ZOOM_H_MAX/>
                        </div>
                        <div class="flex items-baseline gap-3 px-4 py-2.5">
                            <b class="text-sm">{label_of(h.type_name)}</b>
                            <span class="min-w-0 flex-1 truncate text-xs text-base-content/70">{desc}</span>
                            <span class="shrink-0 text-xs text-base-content/60">"Clic para agregar"</span>
                        </div>
                    </div>
                }
            })}
        </nav>
    }
}

/// Fondo neutro punteado detrás de cada miniatura: así el bloque, aunque
/// la tienda sea oscura, no tiñe la tira.
const STAGE: &str = "grid place-items-center rounded-field bg-base-200 [background-image:radial-gradient(var(--color-base-300)_1px,transparent_1px)] [background-size:12px_12px]";
const ZOOM_STAGE: &str = "grid place-items-center bg-base-200 p-5 [background-image:radial-gradient(var(--color-base-300)_1px,transparent_1px)] [background-size:12px_12px]";

/// Otras palabras con que se busca cada bloque (sin tildes, en minúscula):
/// así "slider", "cupón" o "reseñas" encuentran el bloque aunque se llame
/// distinto.
fn keywords(type_name: &str) -> &'static str {
    match type_name {
        "Announcement" => "anuncio barra franja aviso envio gratis promo mensaje",
        "Coupon" => "cupon descuento codigo promo oferta rebaja",
        "Navbar" => "menu encabezado header logo navegacion barra",
        "Banner" => "portada hero imagen grande titulo foto principal",
        "Carousel" => "slider slideshow diapositivas presentacion fotos rotan",
        "ProductGrid" => "productos catalogo tienda grilla lista articulos",
        "ProductCarousel" => "slider productos fila deslizar destacados",
        "TagFilter" | "TagList" | "TagCircles" | "TagSearch" | "TagTabs" => {
            "etiquetas filtro filtrar buscar"
        }
        "CategoryTabs" | "CategorySide" | "CategoryCards" | "CategoryDrop" | "CategoryPage"
        | "Categories" => "categorias secciones rubros filtro",
        "FeaturedProduct" => "destacado producto estrella oferta",
        "Countdown" => "cuenta regresiva reloj temporizador oferta black hot sale cyber",
        "Text" => "texto parrafo titulo escribir contenido",
        "Steps" => "pasos como comprar proceso",
        "Social" => "redes whatsapp instagram facebook tiktok",
        "Card" => "tarjeta imagen texto boton",
        "Image" => "imagen foto picture",
        "HoverGallery" => "galeria fotos imagenes",
        "Tabs" => "pestañas informacion",
        "Faq" => "preguntas frecuentes ayuda dudas",
        "Testimonials" => "testimonios reseñas opiniones comentarios clientes resenas",
        "Benefits" => "beneficios ventajas envio cuotas garantia",
        "Stats" => "cifras numeros estadisticas",
        "Logos" => "logos marcas",
        "Contact" => "contacto mapa direccion telefono email formulario horario",
        "Footer" => "pie footer legal",
        _ => "",
    }
}

fn cat_label(g: &str) -> &str {
    if g == SEARCH {
        "Buscar"
    } else {
        g
    }
}

fn cat_icon(g: &str) -> &'static str {
    if g == SEARCH {
        "search"
    } else {
        GROUPS
            .iter()
            .find(|(n, _)| *n == g)
            .map_or("text", |(_, i)| *i)
    }
}

/// El bloque con sus valores de fábrica, dibujado de verdad en el tema y
/// las letras de la tienda y achicado (o agrandado) a `tw` píxeles de ancho.
#[component]
pub(crate) fn Thumb(
    type_name: &'static str,
    products: StoredValue<Vec<Product>>,
    tw: f64,
    th_max: f64,
) -> impl IntoView {
    let state = expect_context::<EditorState>();
    let in_modal = state.in_modal_untracked();
    let Some((w, h, scale, vis_h)) = thumb_size(type_name, tw, th_max, in_modal) else {
        return ().into_any();
    };
    let Some(widget) = Widget::defaults(type_name) else {
        return ().into_any();
    };
    let products = products.get_value();
    let mut node = Node::new(format!("mk-pal-{type_name}"), widget);
    // Un producto destacado sin elegir mostraría un aviso: se ve con el primero.
    if let Widget::FeaturedProduct(p) = &mut node.widget {
        if let Some(first) = products.first() {
            p.product_id = first.id.clone();
        }
    }
    let product = in_modal.then(|| products.first().cloned().unwrap_or_else(sample_product));
    let ctx = RenderCtx {
        products,
        editing: true,
        product,
    };
    let theme = move || state.doc.with(|d| d.theme.clone());
    let store_vars = move || state.doc.with(|d| d.typography.css_vars());
    let outer = move || format!("{}; width:{tw}px; height:{vis_h}px", store_vars());
    let inner = format!(
        "width:{w}px; height:{h}px; transform: scale({scale}); transform-origin: 0 0; padding: 8px; box-sizing: border-box;"
    );
    view! {
        <span
            data-theme=theme
            class="mk-store relative block overflow-hidden rounded-sm shadow-sm ring-1 ring-black/10"
            style=outer
            aria-hidden="true"
        >
            <span class="pointer-events-none block select-none" style=inner inert>
                {render_widget(&node, &ctx)}
            </span>
        </span>
    }
    .into_any()
}
