#![recursion_limit = "256"]

//! Render de cada widget. Un componente por variante de [`Widget`].
//!
//! Los widgets usan solo colores semánticos de daisyUI y se adaptan al ancho
//! y alto que les toca en la grilla (con container queries `@container`).
//! Los textos se muestran siempre como texto, nunca como HTML.

use domain::{
    AnnouncementProps, BannerProps, BenefitsProps, CardProps, CategoriesProps, ContactProps,
    FaqProps, FooterProps, LinkItem, NavbarProps, Node, Product, ProductGridProps, ProductModal,
    TestimonialsProps, TextProps, Widget,
};
use leptos::html::Div;
use leptos::prelude::*;
use ui::{Button, Card, Icon, Image, PriceWas};

pub mod account;
pub mod anim;
pub mod cart;
pub mod categories;
pub mod product_page;
pub mod tags;
pub use account::{
    Account, AccountFooterLink, AccountNavButton, AccountState, BuyerAvatar, FavRequest,
    FavoriteHeart, HeartIcon,
};
pub use anim::{AnimText, TextAnim};
pub use cart::{AddToCart, CardCartButton, Cart, CartBar, CartDrawer, CartNavButton, ItemName};
pub use tags::{FilteredProducts, NwGrid, ProductFilter};

/// Contexto que recibe todo widget al dibujarse.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct RenderCtx {
    pub products: Vec<Product>,
    /// En el editor: desactiva enlaces, botones y animaciones automáticas.
    pub editing: bool,
    /// Producto que muestran los bloques de la ficha del producto.
    pub product: Option<Product>,
}

/// Producto que se muestra en la ficha cuando la tienda todavía no tiene
/// productos (solo en el editor).
pub fn sample_product() -> Product {
    Product {
        id: String::new(),
        name: "Producto de ejemplo".to_string(),
        price_cents: 1_999_900,
        image_url: "/img/product-1.webp".to_string(),
        description: "Así se va a ver la descripción de cada producto.".to_string(),
        ..Product::default()
    }
}

/// Aviso de uso para quien arma la página (la app): "agregó al carrito",
/// "abrió un producto". Si no está en el contexto, no se avisa nada.
#[derive(Clone, Copy)]
pub struct UsageHook(pub Callback<&'static str>);

/// Las páginas y las categorías de productos que tiene la tienda, por
/// contexto en la tienda publicada (ver `domain::category_tile_href`).
#[derive(Clone)]
pub struct StoreLinks {
    pub pages: Vec<String>,
    pub categories: Vec<String>,
}

/// La página tiene pie: los enlaces obligatorios («Ingresar a mi cuenta» y
/// «Botón de arrepentimiento») van adentro del pie y no sueltos debajo.
#[derive(Clone, Copy)]
pub struct LegalInFooter;

/// Producto abierto en la ficha de la tienda, por contexto. Si está, las
/// tarjetas de producto se pueden tocar para abrir su ficha.
#[derive(Clone, Copy)]
pub struct OpenProduct(pub RwSignal<Option<String>>);

/// Dibuja un nodo dentro de su envoltorio de letras (`.mk-type`, ver
/// [`domain::TextStyle`] y `style/tailwind.css`).
pub fn render_widget(node: &Node, ctx: &RenderCtx) -> AnyView {
    let nwl = if node.widget.is_new_design() {
        " mk-nwl"
    } else {
        ""
    };
    let class = format!(
        "{} {}{nwl} h-full",
        node.style.classes(),
        node.look.classes_max(node.widget.designs().len() as u8)
    );
    let style = format!("{}; {}", node.style.css_vars(), node.look.css_vars());
    // Las tarjetas de producto leen su estilo del bloque que las contiene.
    let cards = match &node.widget {
        Widget::ProductGrid(p) => Some(p.cards.clone()),
        _ => None,
    };
    let inner = {
        let (node, ctx) = (node.clone(), ctx.clone());
        move || {
            match cards {
            Some(cards) => {
                view! { <leptos::context::Provider value=cards>{render_content(&node, &ctx)}</leptos::context::Provider> }.into_any()
            }
            None => render_content(&node, &ctx),
        }
        }
    };
    // La animación del texto la leen los títulos del bloque (`AnimText`).
    let content = match domain::text_animation(&node.style.animation) {
        Some((id, _, _)) => {
            let anim = TextAnim {
                id: (*id).to_string(),
                extra: node.style.anim_text.trim().to_string(),
            };
            view! { <leptos::context::Provider value=anim>{inner()}</leptos::context::Provider> }
                .into_any()
        }
        None => inner(),
    };
    // Cada bloque se puede nombrar en un enlace (`#w-<id>`).
    let anchor = format!("w-{}", node.id);
    view! { <div id=anchor class=class style=style>{content}</div> }.into_any()
}

fn render_content(node: &Node, ctx: &RenderCtx) -> AnyView {
    let editing = ctx.editing;
    match &node.widget {
        Widget::Banner(p) => view! { <BannerWidget props=p.clone() editing/> }.into_any(),
        Widget::Text(p) => {
            // La alineación del estilo del bloque manda sobre la vieja del texto.
            let mut props = p.clone();
            if !node.style.align.is_empty() {
                props.align = node.style.align.clone();
            }
            view! { <TextWidget props/> }.into_any()
        }
        Widget::ProductGrid(p) => {
            view! { <ProductGridWidget props=p.clone() products=ctx.products.clone() editing/> }
                .into_any()
        }
        Widget::Navbar(p) => {
            view! { <NavbarWidget props=p.clone() editing uid=format!("nav-{}", node.id)/> }
                .into_any()
        }
        Widget::Announcement(p) => {
            view! { <AnnouncementWidget props=p.clone() editing/> }.into_any()
        }
        Widget::Categories(p) => view! { <CategoriesWidget props=p.clone() editing/> }.into_any(),
        Widget::Benefits(p) => view! { <BenefitsWidget props=p.clone()/> }.into_any(),
        Widget::Card(p) => view! { <CardWidget props=p.clone() editing/> }.into_any(),
        Widget::Faq(p) => view! { <FaqWidget props=p.clone() editing/> }.into_any(),
        Widget::Testimonials(p) => view! { <TestimonialsWidget props=p.clone()/> }.into_any(),
        Widget::Contact(p) => view! { <ContactWidget props=p.clone() editing/> }.into_any(),
        Widget::Footer(p) => view! { <FooterWidget props=p.clone() editing/> }.into_any(),
        Widget::CategoryPage(_) => categories::render(&node.widget, ctx),
        Widget::ProductSheet(p) => view! {
            <product_page::ProductSheetWidget props=p.clone() product=ctx.product.clone() editing/>
        }
        .into_any(),
    }
}

/// Texto del botón de la ficha. Las fichas viejas decían "Consultar por este
/// producto" y llevaban a Contacto; ahora el botón agrega al carrito.
pub fn product_button_label(label: &str) -> String {
    match label.trim() {
        "" | "Consultar por este producto" => "Agregar al carrito".to_string(),
        other => other.to_string(),
    }
}

/// Ficha del producto en la tienda: un `<dialog>` modal con el diseño de
/// [`ProductModal`], para el producto de `open`. Se abre con `showModal()`
/// (fondo inerte, foco dentro, Esc la cierra y el foco vuelve a la tarjeta)
/// y también se cierra tocando afuera o con el botón Cerrar.
#[component]
pub fn ProductDialog(
    modal: ProductModal,
    products: Vec<Product>,
    open: RwSignal<Option<String>>,
    /// En la vista previa del editor: sin enlaces que naveguen.
    #[prop(optional)]
    editing: bool,
) -> impl IntoView {
    let dialog_ref = NodeRef::<leptos::html::Dialog>::new();
    let box_ref = NodeRef::<Div>::new();
    let products = StoredValue::new(products);
    let open_class = match modal.open.as_str() {
        "drawer" => "mk-open-drawer",
        "sheet" => "mk-open-sheet",
        "auto" => "mk-open-auto",
        _ => "mk-open-center",
    };
    let mut nodes = modal.nodes;
    nodes.sort_by_key(|n| (n.pos.y, n.pos.x));
    let nodes = StoredValue::new(nodes);
    let product = Memo::new(move |_| {
        let id = open.get()?;
        products.with_value(|ps| ps.iter().find(|p| p.id == id).cloned())
    });

    Effect::new(move |_| {
        let Some(dialog) = dialog_ref.get() else {
            return;
        };
        if product.get().is_some() {
            if !dialog.open() {
                _ = dialog.show_modal();
            }
        } else if dialog.open() {
            dialog.close();
        }
    });

    view! {
        <dialog
            node_ref=dialog_ref
            class=format!("modal mk-dialog {open_class}")
            aria-label=move || product.get().map(|p| p.name).unwrap_or_default()
            on:close=move |_| open.set(None)
        >
            <div class="modal-box mk-dialog-box" node_ref=box_ref>
                <form method="dialog" class="absolute right-2 top-2 z-10">
                    <button class="btn btn-sm btn-circle btn-ghost" aria-label="Cerrar">
                        <svg class="h-4 w-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" aria-hidden="true"><path d="M6 6l12 12M18 6L6 18"/></svg>
                    </button>
                </form>
                {move || product.get().map(|p| {
                    // Color y talle elegidos, compartidos por la foto y el botón.
                    provide_context(product_page::VariantPick::new());
                    let ctx = RenderCtx {
                        products: products.get_value(),
                        editing,
                        product: Some(p),
                    };
                    view! { <div class="mk-sheetflow">{flow_nodes(nodes.get_value(), &ctx)}</div> }
                })}
            </div>
            <form method="dialog" class="modal-backdrop">
                <button tabindex="-1">"Cerrar"</button>
            </form>
        </dialog>
    }
}

/// La ficha de un producto tal como se ve en la tienda, sin la ventana ni
/// botones que compren o naveguen: la vista previa de Productos mientras se
/// escribe. Va dentro de un contenedor con el tema de la tienda (`mk-store`).
#[component]
pub fn ProductSheetPreview(
    modal: ProductModal,
    #[prop(into)] product: Signal<Option<Product>>,
) -> impl IntoView {
    let mut nodes = modal.nodes;
    nodes.sort_by_key(|n| (n.pos.y, n.pos.x));
    let nodes = StoredValue::new(nodes);
    move || {
        product.get().map(|p| {
            provide_context(product_page::VariantPick::new());
            let ctx = RenderCtx {
                products: vec![p.clone()],
                editing: true,
                product: Some(p),
            };
            view! { <div class="mk-sheetflow">{flow_nodes(nodes.get_value(), &ctx)}</div> }
        })
    }
}

/// La ficha nueva y lo que se le sume debajo, uno tras otro a todo el ancho.
fn flow_nodes(mut nodes: Vec<Node>, ctx: &RenderCtx) -> impl IntoView {
    nodes.sort_by_key(|n| (n.pos.y, n.pos.x));
    nodes
        .into_iter()
        .map(|node| view! { <div class="mk-sheetnode">{render_widget(&node, ctx)}</div> })
        .collect_view()
}

/// Dónde va el botón Compartir de la ficha (ver `ProductModal::share`).
#[derive(Clone, Copy)]
pub struct SharePlace {
    pub on_photo: Signal<bool>,
}

/// «Compartir»: en el celular abre el menú de compartir del teléfono; en la
/// computadora copia el enlace del producto y avisa «Enlace copiado». Arriba
/// de la ficha lleva texto; sobre la foto es un botón redondo.
#[component]
pub fn ShareButton(
    product: Product,
    on_photo: bool,
    /// En el flujo de la ficha nueva (sin posición absoluta).
    #[prop(optional)]
    inline: bool,
) -> impl IntoView {
    let id = StoredValue::new(product.id.clone());
    let name = StoredValue::new(product.name.clone());
    // None: nada que avisar; Some(true): copiado; Some(false): no se pudo.
    let state = RwSignal::new(None::<bool>);
    let generation = StoredValue::new(0u32);
    let share = move |_| {
        let origin = window().location().origin().unwrap_or_default();
        let url = format!("{origin}{}", domain::product_href(&id.get_value()));
        if native_share(&name.get_value(), &url) {
            return;
        }
        generation.update_value(|g| *g += 1);
        let mine = generation.get_value();
        state.set(None);
        copy_text(&url, move |ok| {
            _ = state.try_set(Some(ok));
            set_timeout(
                move || {
                    if generation.try_get_value() == Some(mine) {
                        _ = state.try_set(None);
                    }
                },
                std::time::Duration::from_secs(2),
            );
        });
    };
    let note = move || match state.get() {
        Some(true) => "Enlace copiado",
        Some(false) => "No pudimos copiar el enlace",
        None => "",
    };
    let icon = view! {
        <svg class="h-[18px] w-[18px]" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
            <circle cx="18" cy="5" r="3"/><circle cx="6" cy="12" r="3"/><circle cx="18" cy="19" r="3"/><path d="M8.6 10.5l6.8-4M8.6 13.5l6.8 4"/>
        </svg>
    };
    if on_photo {
        view! {
            <button type="button" class="mk-fav mk-share-photo absolute right-2 z-10" aria-label="Compartir" on:click=share>{icon}</button>
            <span role="status" class="mk-share-note absolute right-12 z-10 rounded-field bg-base-100 px-2 py-1 text-xs shadow empty:hidden">{note}</span>
        }
        .into_any()
    } else if inline {
        view! {
            <span class="nw-pdp-share">
                <span role="status" class="text-xs">{note}</span>
                <button type="button" class="nw-pdp-sharebtn" on:click=share>{icon}"Compartir"</button>
            </span>
        }
        .into_any()
    } else {
        view! {
            <div class="absolute right-12 top-2 z-10 flex items-center gap-2">
                <span role="status" class="text-xs text-base-content/75">{note}</span>
                <button type="button" class="btn btn-sm btn-ghost gap-2 font-medium" on:click=share>{icon}"Compartir"</button>
            </div>
        }
        .into_any()
    }
}

/// Abre el menú de compartir del teléfono, si el navegador lo tiene. En la
/// computadora devuelve `false`: ahí conviene copiar el enlace.
fn native_share(title: &str, url: &str) -> bool {
    #[cfg(target_arch = "wasm32")]
    {
        use wasm_bindgen::{JsCast, JsValue};
        let touch = window()
            .match_media("(pointer: coarse)")
            .ok()
            .flatten()
            .is_some_and(|m| m.matches());
        let nav: JsValue = window().navigator().into();
        let Ok(f) = js_sys::Reflect::get(&nav, &JsValue::from_str("share")) else {
            return false;
        };
        let Some(f) = f.dyn_ref::<js_sys::Function>() else {
            return false;
        };
        if !touch {
            return false;
        }
        let data = js_sys::Object::new();
        _ = js_sys::Reflect::set(
            &data,
            &JsValue::from_str("title"),
            &JsValue::from_str(title),
        );
        _ = js_sys::Reflect::set(&data, &JsValue::from_str("url"), &JsValue::from_str(url));
        // Si la persona cierra el menú sin compartir, la promesa falla: no
        // hay nada que avisar.
        if let Ok(p) = f.call1(&nav, &data) {
            if let Ok(p) = p.dyn_into::<js_sys::Promise>() {
                let quiet = wasm_bindgen::closure::Closure::once(|_: JsValue| {});
                _ = p.catch(&quiet);
                quiet.forget();
            }
        }
        true
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = (title, url);
        false
    }
}

/// Variables CSS de posición de un nodo para `.mk-node`: `--x:1; --y:49; --w:48; --h:60`.
/// `--x` y `--y` suman 1 a `pos` porque las líneas de la grilla CSS empiezan en 1.
pub fn node_style(node: &Node) -> String {
    format!(
        "--x:{}; --y:{}; --w:{}; --h:{}",
        u32::from(node.pos.x) + 1,
        u32::from(node.pos.y) + 1,
        node.size.w,
        node.size.h
    )
}

/// Clase y estilo de cada nodo de una página, en el mismo orden. Un menú
/// flotante ([`NavbarProps::floats`]) se monta sobre el bloque que tiene
/// justo abajo (la portada): ese bloque sube hasta la fila del menú y crece
/// lo mismo, y el menú queda encima. En el celular, apilados, el bloque sube
/// con un margen negativo (ver `.mk-node-under`).
pub fn page_nodes(nodes: &[Node]) -> Vec<(&'static str, String)> {
    nodes.iter().map(|n| ("mk-node", node_style(n))).collect()
}

/// Enlace si hay `href` y no se está editando; si no, un `div` con la misma
/// clase. Mientras se edita, el `div` lleva el enlace en `data-href`: el
/// editor lo usa para ir a esa página al tocarlo.
#[component]
fn MaybeLink(
    #[prop(into)] href: String,
    editing: bool,
    #[prop(into)] class: String,
    children: Children,
) -> impl IntoView {
    if href.is_empty() {
        view! { <div class=class>{children()}</div> }.into_any()
    } else if editing {
        view! { <div class=class data-href=href>{children()}</div> }.into_any()
    } else {
        view! { <a class=class href=href>{children()}</a> }.into_any()
    }
}

/// Texto de un botón de widget nuevo: negro o blanco, el que más contraste da.
fn nw_on_color(_color: &str) -> String {
    // Los colores salen del tema de la tienda; los que guardaron los
    // bloques viejos ya no se aplican.
    String::new()
}

/// Portada (diseños: marco editorial, fotografía inmersiva, arco).
#[component]
pub fn BannerWidget(props: BannerProps, editing: bool) -> impl IntoView {
    // Sin adónde llevar, el botón no se muestra en la tienda (en el editor sí,
    // con un aviso en el panel).
    // El botón se muestra si tiene texto (vacío = sin botón).
    let has_button = !props.button_label.is_empty()
        && (editing || !props.button_href.trim().is_empty());
    let copy_style = if props.align == "center" {
        "align-items:center;text-align:center;"
    } else {
        ""
    };
    let root_style = format!(
        "{}--nw-veil:{};",
        nw_colors(&props.bg, &props.ink),
        f64::from(props.veil.clamp(60, 85) as i32) / 100.0
    );
    view! {
        <div class="mk-nw h-full overflow-hidden">
            <div class="nw-hero-wrap"><section class="nw-hero nw-veil" style=root_style>
                <div class="nw-hero-copy" style=copy_style>
                    {(!props.kicker.is_empty()).then(|| view! {
                        <span class="nw-kicker">{props.kicker.clone()}</span>
                    })}
                    <h4><AnimText text=props.title.clone()/></h4>
                    {(!props.subtitle.is_empty()).then(|| view! { <p>{props.subtitle.clone()}</p> })}
                    {has_button.then(|| view! {
                        <span style=nw_on_color(&props.button_color)>
                            <MaybeLink href=props.button_href.clone() editing class="nw-action">
                                {format!("{} ↗", props.button_label)}
                            </MaybeLink>
                        </span>
                    })}
                </div>
                <div class="nw-hero-photo">
                    <Image src=props.image_url.clone() alt=props.image_alt.clone() eager=true/>
                </div>
            </section></div>
        </div>
    }
}

#[component]
pub fn TextWidget(props: TextProps) -> impl IntoView {
    let class = match props.align.as_str() {
        "center" => "mk-box mk-text h-full overflow-hidden rounded-box bg-base-100 p-4 text-center",
        "right" => "mk-box mk-text h-full overflow-hidden rounded-box bg-base-100 p-4 text-right",
        _ => "mk-box mk-text h-full overflow-hidden rounded-box bg-base-100 p-4 text-left",
    };
    view! {
        <div class=class>
            {(!props.heading.is_empty()).then(|| view! {
                <h2 class="mk-title text-xl font-bold mb-2 truncate"><AnimText text={props.heading.clone()}/></h2>
            })}
            <p class="whitespace-pre-line">{props.body}</p>
        </div>
    }
}

/// Encabezado de las secciones nuevas: antetítulo, título y enlace a la derecha.
#[component]
fn NwHead(
    kicker: String,
    title: String,
    show_all: bool,
    all_label: String,
    all_href: String,
    editing: bool,
) -> impl IntoView {
    let link = show_all && !all_label.is_empty();
    view! {
        <div class="nw-section-head">
            <div>
                {(!kicker.is_empty()).then(|| view! { <div class="nw-kicker">{kicker.clone()}</div> })}
                {(!title.is_empty()).then(|| view! { <h4><AnimText text=title.clone()/></h4> })}
            </div>
            {link.then(|| view! {
                <MaybeLink href=all_href editing class="nw-section-link">{format!("{all_label} ↗")}</MaybeLink>
            })}
        </div>
    }
}

/// Cuadrícula de productos (diseños: lateral, enmarcada, flotante translúcida).
#[component]
pub fn ProductGridWidget(
    props: ProductGridProps,
    products: Vec<Product>,
    editing: bool,
) -> impl IntoView {
    let limit = usize::try_from(props.limit).unwrap_or(0);
    let filter = use_context::<ProductFilter>();
    let grid = NwGrid {
        cols: props.cols.clamp(2, 6),
        show_category: props.show_category,
        show_installments: props.show_installments,
        favorite: props.cards.card_favorite,
        cart_action: if props.cart_action == "plus" { "plus" } else { "text" },
    };
    let style = format!(
        "{}--radius:{}px;--panel-a:{}%;",
        nw_colors(&props.bg, &props.ink),
        props.radius.clamp(0, 24),
        props.panel_opacity.clamp(75, 100)
    );
    view! {
        <div class="mk-nw h-full overflow-hidden">
            <section class="nw-section" style=style>
                <NwHead kicker=props.kicker.clone() title=props.title.clone() show_all=props.show_all
                    all_label=props.all_label.clone() all_href=props.all_href.clone() editing/>
                <FilteredProducts products limit filter paged=true nw=Some(grid)/>
            </section>
        </div>
    }
}

/// Tarjeta de producto: la imagen ocupa todo el alto que deja el texto.
/// En la tienda (con [`OpenProduct`]) se puede tocar para abrir su ficha.
#[component]
pub fn ProductCard(product: Product, show_price: bool) -> impl IntoView {
    let look = use_context::<domain::CardOptions>().unwrap_or_default();
    match look.card.as_str() {
        "full" => return product_page::FullCard(product, show_price, &look).into_any(),
        "clean" => {
            return product_page::CleanCard(
                product,
                show_price,
                look.card_favorite,
                look.card_colors,
            )
            .into_any()
        }
        _ => {}
    }
    let open = use_context::<OpenProduct>();
    let id = product.id.clone();
    let name = product.name.clone();
    let sold_out = !product.in_stock;
    let few_left = product.few_left();
    let off = show_price.then(|| product.discount_pct()).flatten();
    let cart_product = product.clone();
    let dots_product = product.clone();
    let fit = product.fit(&product.image_url);
    let heart = look
        .card_favorite
        .then(|| (product.id.clone(), product.name.clone()));
    view! {
        <div class="relative h-full" data-product-id=product.id.clone()>
        <Card>
            <div class="mk-card-img relative min-h-0 flex-1">
                <div class="absolute inset-0">
                    <Image src=product.image_url.clone() alt=product.name.clone() fit initial=true/>
                </div>
                {heart.map(|(product_id, name)| view! { <FavoriteHeart product_id name/> })}
                <div class="absolute left-2 top-2 flex flex-col items-start gap-1">
                    {sold_out.then(|| view! { <span class="badge badge-neutral">"Agotado"</span> })}
                    {few_left.map(|t| view! { <span class="badge badge-warning">{t}</span> })}
                    {off.map(|n| view! {
                        <span class="badge badge-error">{format!("-{n}%")}</span>
                    })}
                </div>
            </div>
            <div class="card-body grow-0 shrink-0 gap-0">
                <h3 class="mk-sub mk-pcard-name card-title truncate block text-[0.8125rem] leading-snug sm:text-sm" title=product.name.clone()>{product.name.clone()}</h3>
                {show_price.then(|| view! { <PriceWas cents=product.price_cents was=product.compare_at_cents/> })}
                {look.card_colors.then(|| view! { <product_page::ColorDots product=dots_product.clone()/> })}
                <CardCartButton product=cart_product/>
            </div>
        </Card>
        {open.map(|OpenProduct(open)| view! {
            <button
                type="button"
                class="absolute inset-0 cursor-pointer rounded-box focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-primary"
                aria-haspopup="dialog"
                aria-label=format!("Ver {name}")
                on:click=move |_| open.set(Some(id.clone()))
            ></button>
        })}
        </div>
    }
    .into_any()
}

/// Dónde poner un desplegable: justo debajo de su botón (o del menú entero,
/// si es el panel ancho). El desplegable está en la capa superior del
/// navegador (`popover`), así que ningún bloque lo recorta.
fn place_popover(ev: leptos::ev::MouseEvent, pop_id: &str, wide: bool) {
    use leptos::wasm_bindgen::JsCast;
    let Some(btn) = ev
        .current_target()
        .and_then(|t| t.dyn_into::<web_sys::HtmlElement>().ok())
    else {
        return;
    };
    let Some(pop) = document()
        .get_element_by_id(pop_id)
        .and_then(|e| e.dyn_into::<web_sys::HtmlElement>().ok())
    else {
        return;
    };
    let win = window();
    let (sx, sy) = (win.scroll_x().unwrap_or(0.0), win.scroll_y().unwrap_or(0.0));
    let view_w = win
        .inner_width()
        .ok()
        .and_then(|w| w.as_f64())
        .unwrap_or(1024.0);
    let anchor = if wide {
        btn.closest("nav")
            .ok()
            .flatten()
            .unwrap_or_else(|| btn.clone().into())
    } else {
        btn.clone().into()
    };
    let r = anchor.get_bounding_client_rect();
    let b = btn.get_bounding_client_rect();
    let style = pop.style();
    _ = style.set_property("top", &format!("{}px", b.bottom() + sy + 6.0));
    if wide {
        _ = style.set_property("left", &format!("{}px", r.left() + sx));
        _ = style.set_property("width", &format!("{}px", r.width()));
    } else {
        // Que no se salga por la derecha.
        let left = b.left().min(view_w - 232.0).max(8.0);
        _ = style.set_property("left", &format!("{}px", left + sx));
    }
}

/// El contenido de un desplegable «En columnas»: cada opción con sus propias
/// opciones es una columna con título; las opciones sueltas van juntas en la
/// primera columna. La foto, si hay, va a la derecha.
fn mega_panel(l: &LinkItem, editing: bool) -> AnyView {
    // Las columnas siguen el orden de los enlaces: las opciones sueltas
    // seguidas forman una columna en su lugar.
    enum Col {
        Loose(Vec<LinkItem>),
        Titled(LinkItem),
    }
    let mut groups: Vec<Col> = Vec::new();
    for c in l.children.iter().cloned() {
        if !c.children.is_empty() {
            groups.push(Col::Titled(c));
        } else if let Some(Col::Loose(v)) = groups.last_mut() {
            v.push(c);
        } else {
            groups.push(Col::Loose(vec![c]));
        }
    }
    let cols = groups.len();
    let has_image = !l.image_url.is_empty();
    let class = if has_image {
        "mk-mega mk-mega-img"
    } else {
        "mk-mega"
    };
    let all_href = l.href.clone();
    let label = l.label.clone();
    let image = l.image_url.clone();
    let see_all = (!has_image && !all_href.is_empty()).then(|| all_href.clone());
    view! {
        <div class=class style=format!("--cols:{}", cols.max(1))>
            {groups.into_iter().map(|g| match g {
                Col::Loose(items) => view! {
                    <ul class="mk-mega-col">
                        {items.into_iter().map(|c| view! {
                            <li><MaybeLink href=c.href editing class="mk-link mk-mega-lead">{c.label}</MaybeLink></li>
                        }).collect_view()}
                    </ul>
                }.into_any(),
                Col::Titled(c) => view! {
                    <div class="mk-mega-col">
                        <MaybeLink href=c.href editing class="mk-link mk-mega-h">{c.label}</MaybeLink>
                        <ul>
                            {c.children.into_iter().map(|g| view! {
                                <li><MaybeLink href=g.href editing class="mk-link">{g.label}</MaybeLink></li>
                            }).collect_view()}
                        </ul>
                    </div>
                }.into_any(),
            }).collect_view()}
            {has_image.then(|| view! {
                <MaybeLink href=all_href editing class="mk-mega-pic relative block overflow-hidden">
                    <div class="aspect-[16/10] w-full"><Image src=image alt=""/></div>
                    <span class="absolute inset-x-0 bottom-0 bg-gradient-to-t from-black/70 to-transparent p-3 text-sm font-semibold text-white">{format!("{label} →")}</span>
                </MaybeLink>
            })}
            // Como en la lista simple: "Ver todo" si el enlace lleva a algún lado
            // (con foto, la foto ya lleva ahí).
            {see_all.map(|href| view! {
                <div class="col-span-full border-t border-base-300 pt-3">
                    <MaybeLink href=href editing class="mk-link text-sm font-semibold">"Ver todo"</MaybeLink>
                </div>
            })}
        </div>
    }
    .into_any()
}

/// Buscador del menú: escribe en el filtro de productos de la página (ver
/// `tags::ProductFilter`). Si la página no muestra productos, al apretar
/// Enter va al inicio de la tienda con la búsqueda (`/?buscar=`).
#[component]
fn NavSearch(editing: bool, #[prop(into)] class: String) -> impl IntoView {
    let filter = use_context::<crate::tags::ProductFilter>();
    view! {
        <form role="search" action="/" method="get" class=format!("mk-navsearch {class}")
            on:submit=move |ev| {
                if editing {
                    ev.prevent_default();
                    return;
                }
                let list = document().query_selector(".mk-filtered, .mk-pgrid").ok().flatten();
                if let Some(list) = list {
                    ev.prevent_default();
                    let opts = web_sys::ScrollIntoViewOptions::new();
                    opts.set_block(web_sys::ScrollLogicalPosition::Center);
                    list.scroll_into_view_with_scroll_into_view_options(&opts);
                }
            }>
            <label class="input input-sm mk-navsearch-box">
                <Icon name="search" class="h-4 w-4 opacity-70"/>
                <input type="search" name="buscar" placeholder="¿Qué estás buscando?"
                    aria-label="Buscar productos" autocomplete="off" enterkeyhint="search"
                    tabindex=editing.then_some("-1")
                    prop:value=move || filter.map(|f| f.query.get()).unwrap_or_default()
                    on:input=move |ev| { if let Some(f) = filter { f.query.set(event_target_value(&ev)); } }/>
            </label>
        </form>
    }
}

/// Un enlace del menú del celular: si tiene opciones, un botón que las
/// muestra u oculta debajo (`aria-expanded`), con un nivel más para las
/// columnas del desplegable «En columnas».
fn mobile_link(l: LinkItem, editing: bool, uid: &str) -> AnyView {
    if l.children.is_empty() {
        return view! {
            <li><MaybeLink href=l.href editing class="mk-link">{l.label}</MaybeLink></li>
        }
        .into_any();
    }
    let open = RwSignal::new(false);
    let id = format!("{uid}-sub");
    view! {
        <li>
            <button type="button" class="mk-link justify-between" aria-controls=id.clone()
                aria-expanded=move || open.get().to_string()
                on:click=move |_| open.update(|o| *o = !*o)>
                {l.label}
                <Icon name="chevron-down" class="h-4 w-4 transition-transform in-aria-expanded:rotate-180"/>
            </button>
            <ul id=id hidden=move || !open.get()>
                {l.children.into_iter().enumerate().map(|(j, c)| mobile_link(c, editing, &format!("{uid}-{j}"))).collect_view()}
                {(!l.href.is_empty()).then(|| view! {
                    <li><MaybeLink href=l.href.clone() editing class="mk-link font-semibold">"Ver todo"</MaybeLink></li>
                })}
            </ul>
        </li>
    }
    .into_any()
}

#[component]
pub fn NavbarWidget(props: NavbarProps, editing: bool, uid: String) -> impl IntoView {
    // Sin adónde llevar, el botón no se muestra en la tienda (en el editor sí,
    // con un aviso en el panel).
    let has_button =
        !props.button_label.is_empty() && (editing || !props.button_href.trim().is_empty());
    // Vacíos, usan el nombre y el logo de la tienda (ver "Datos de la tienda").
    let store = use_context::<domain::StoreInfo>().unwrap_or_default();
    let brand = or_store(&props.brand, &store.name);
    let logo_url = or_store(&props.logo_url, &store.logo_url);
    let columns = props.menu == "columns";
    let wide = props.menu == "panel" || columns;
    let links = props.links;
    let desktop = links
        .clone()
        .into_iter()
        .enumerate()
        .map(|(i, l)| {
            if l.children.is_empty() {
                return view! {
                    <li>
                        <MaybeLink href=l.href editing class="mk-link whitespace-nowrap">{l.label}</MaybeLink>
                    </li>
                }
                .into_any();
            }
            let pid = format!("{uid}-m{i}");
            let pid_click = pid.clone();
            // "Panel ancho con foto" sin foto: un panel del tamaño de sus
            // enlaces, no uno enorme y vacío.
            let pop_wide = columns || (wide && !l.image_url.is_empty());
            let panel = if columns {
                mega_panel(&l, editing)
            } else if wide {
                let children = l.children.clone();
                let image = l.image_url.clone();
                let all_href = l.href.clone();
                let label = l.label.clone();
                let has_image = !image.is_empty();
                let see_all = (!has_image && !all_href.is_empty()).then(|| all_href.clone());
                let grid = if has_image {
                    "grid gap-6 p-5 @3xl:grid-cols-[1fr_1fr_1.2fr]"
                } else {
                    "grid grid-flow-col auto-cols-max gap-x-8 gap-y-3 p-5"
                };
                view! {
                    <div class=grid>
                        {children
                            .chunks(3)
                            .map(|col| col.to_vec())
                            .collect::<Vec<_>>()
                            .into_iter()
                            .map(|col| view! {
                                <ul class="flex flex-col gap-1.5">
                                    {col.into_iter().map(|c| view! {
                                        <li><MaybeLink href=c.href editing class="mk-link link link-hover">{c.label}</MaybeLink></li>
                                    }).collect_view()}
                                </ul>
                            })
                            .collect_view()}
                        {has_image.then(|| view! {
                            <MaybeLink href=all_href.clone() editing class="relative block overflow-hidden rounded-box">
                                <div class="aspect-[16/9] w-full"><Image src=image.clone() alt=""/></div>
                                <span class="absolute inset-x-0 bottom-0 bg-gradient-to-t from-black/70 to-transparent p-3 text-sm font-semibold text-white">{format!("{label} →")}</span>
                            </MaybeLink>
                        })}
                    </div>
                    {see_all.map(|href| view! {
                        <div class="border-t border-base-300 px-5 py-3">
                            <MaybeLink href=href editing class="mk-link text-sm font-semibold">"Ver todo"</MaybeLink>
                        </div>
                    })}
                }
                .into_any()
            } else {
                let all_href = l.href.clone();
                view! {
                    <ul class="menu w-52 p-1">
                        {l.children.clone().into_iter().map(|c| view! {
                            <li><MaybeLink href=c.href editing class="mk-link">{c.label}</MaybeLink></li>
                        }).collect_view()}
                        {(!all_href.is_empty()).then(|| view! {
                            <li class="mt-1 border-t border-base-300 pt-1"><MaybeLink href=all_href.clone() editing class="mk-link font-semibold">"Ver todo"</MaybeLink></li>
                        })}
                    </ul>
                }
                .into_any()
            };
            let pop_class = if pop_wide {
                "mk-navpop mk-navpop-wide @container"
            } else {
                "mk-navpop"
            };
            view! {
                <li>
                    <button type="button" class="mk-link nw-linkbtn whitespace-nowrap"
                        popovertarget=pid.clone()
                        on:click=move |ev| place_popover(ev, &pid_click, pop_wide)>
                        {l.label}
                        <Icon name="chevron-down" class="h-4 w-4"/>
                    </button>
                    <div id=pid popover="auto" class=pop_class>{panel}</div>
                </li>
            }
            .into_any()
        })
        .collect_view();
    let mobile_id = format!("{uid}-mobile");
    let mobile_click = mobile_id.clone();
    let mobile = links
        .into_iter()
        .enumerate()
        .map(|(i, l)| mobile_link(l, editing, &format!("{uid}-mm{i}")))
        .collect_view();
    // «nido.»: un punto al final del nombre va en el color de acento.
    let (brand_main, brand_dot) = match brand.strip_suffix('.') {
        Some(main) => (main.to_string(), true),
        None => (brand.clone(), false),
    };
    let brand_style = format!("font-size:{}px", props.size.clamp(18, 40));
    let links_class = if props.upper {
        "nw-nav-links nw-upper"
    } else {
        "nw-nav-links"
    };
    let search = props.search;
    let mobile_pop = view! {
        <div id=mobile_id.clone() popover="auto" class="mk-navpop">
            {has_button.then(|| view! {
                <div class="p-2 pb-0 [&_.mk-btn]:w-full">
                    <Button label=props.button_label.clone() href=props.button_href.clone() disabled=editing/>
                </div>
            })}
            {search.then(|| view! { <NavSearch editing class="p-2 pb-0"/> })}
            <ul class="menu w-64 p-2">{mobile}</ul>
        </div>
    };
    view! {
        <div class="mk-nw h-full">
            <header class="nw-navigation" style=nw_colors(&props.bg, &props.ink)>
                <div class="nw-nav-row">
                    <MaybeLink href="/" editing class="nw-brand flex min-w-0 items-center gap-2">
                        {(!logo_url.is_empty()).then(|| view! {
                            <img class="h-8 w-auto max-w-32 min-w-8 shrink-0 object-contain object-left" src=logo_url.clone() alt="" draggable="false"/>
                        })}
                        <span class="truncate" style=brand_style>
                            {brand_main}
                            {brand_dot.then(|| view! { <i>"."</i> })}
                        </span>
                    </MaybeLink>
                    <ul class=links_class>{desktop}</ul>
                    <div class="nw-nav-icons">
                        {search.then(|| view! { <NavSearch editing class="nw-navsearch"/> })}
                        {has_button.then(|| view! {
                            <span class="nw-nav-button">
                                <Button label=props.button_label.clone() href=props.button_href.clone() disabled=editing/>
                            </span>
                        })}
                        <AccountNavButton/>
                        <CartNavButton/>
                    </div>
                    <div class="nw-nav-mobile">
                        <button type="button" class="nw-nav-mobile-btn" aria-label="Abrir el menú"
                            popovertarget=mobile_id
                            on:click=move |ev| place_popover(ev, &mobile_click, false)>
                            <Icon name="menu" class="h-5 w-5"/>
                            "Menú"
                        </button>
                    </div>
                    {mobile_pop}
                </div>
            </header>
        </div>
    }
}

#[component]
pub fn CategoriesWidget(props: CategoriesProps, editing: bool) -> impl IntoView {
    // En la tienda, los enlaces viejos a páginas que no existen se arreglan.
    let links = use_context::<StoreLinks>();
    let mut props = props;
    if let Some(l) = &links {
        for c in &mut props.items {
            let has_page = l
                .pages
                .iter()
                .any(|p| Some(p) == domain::normalize_path(&c.href).as_ref());
            c.href = domain::category_tile_href(&c.href, &c.label, has_page, &l.categories);
        }
    }
    let ratio = match props.ratio.as_str() {
        "1" | "0.8" | "1.78" => props.ratio.clone(),
        _ => "1.25".to_string(),
    };
    let style = format!(
        "{}--nw-ratio:{ratio};--nw-rad:{}px;",
        nw_colors(&props.bg, &props.ink),
        props.radius.clamp(0, 24)
    );
    let n = props.items.len().clamp(1, 4);
    let arrow = props.arrow;
    view! {
        <div class="mk-nw h-full overflow-hidden">
            <section class="nw-section" style=style>
                <NwHead kicker=props.kicker.clone() title=props.title.clone() show_all=props.show_all
                    all_label=props.all_label.clone() all_href=props.all_href.clone() editing/>
                <div class="nw-categories" style=format!("--n:{n}")>
                    {props
                        .items
                        .into_iter()
                        .map(|c| view! {
                            <MaybeLink href=c.href editing class="nw-category">
                                <div class="nw-img"><Image src=c.image_url.clone() alt=c.label.clone()/></div>
                                <div class="nw-category-label">
                                    <span>{c.label.clone()}</span>
                                    {arrow.then(|| view! { <span aria-hidden="true">"↗"</span> })}
                                </div>
                            </MaybeLink>
                        })
                        .collect_view()}
                </div>
            </section>
        </div>
    }
}

#[component]
pub fn BenefitsWidget(props: BenefitsProps) -> impl IntoView {
    let n = props.items.len().max(1);
    let icon_style = String::new();
    let show_text = props.show_text;
    view! {
        <div class="mk-nw h-full overflow-hidden">
            <div class="nw-benefits" style=format!("{}--n:{n}", nw_colors(&props.bg, &props.ink))>
                {props
                    .items
                    .into_iter()
                    .map(|b| view! {
                        <div class="nw-benefit">
                            <span class="nw-benefit-icon" style=icon_style.clone()>
                                <Icon name=b.icon class="h-5 w-5"/>
                            </span>
                            <div>
                                <b>{b.title}</b>
                                {(show_text && !b.text.is_empty()).then(|| view! { <small>{b.text.clone()}</small> })}
                            </div>
                        </div>
                    })
                    .collect_view()}
            </div>
        </div>
    }
}

// ---------------------------------------------------------------------------
// Widgets con estilos (a, b y c): barra de anuncio, cifras, pasos, carrusel de
// productos y redes.

/// Copia `text` al portapapeles y avisa con `done(true)` o `done(false)`.
pub fn copy_text(text: &str, done: impl Fn(bool) + 'static) {
    #[cfg(target_arch = "wasm32")]
    {
        use wasm_bindgen::{closure::Closure, JsValue};
        let promise = window().navigator().clipboard().write_text(text);
        let done = std::rc::Rc::new(done);
        let d2 = done.clone();
        let ok = Closure::once(move |_: JsValue| done(true));
        let err = Closure::once(move |_: JsValue| d2(false));
        let _ = promise.then2(&ok, &err);
        // Viven hasta que el navegador responde (una vez por clic).
        ok.forget();
        err.forget();
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = text;
        done(false);
    }
}

/// Los códigos de los cupones que sirven hoy. La tienda lo provee: un bloque
/// Cupón con otro código no se muestra. Sin este contexto se muestra siempre.
#[derive(Clone, Debug)]
pub struct ActiveCoupons(pub Vec<String>);

/// Colores propios de un widget nuevo: fondo y texto (`#rrggbb`). Vacío:
/// los del tema de la tienda.
fn nw_colors(_bg: &str, _ink: &str) -> String {
    String::new()
}

/// Barra de anuncio (diseños: franja directa, etiqueta suave, nota enmarcada).
#[component]
pub fn AnnouncementWidget(props: AnnouncementProps, editing: bool) -> impl IntoView {
    let style = format!(
        "{}padding-block:{}px;text-align:{};",
        nw_colors(&props.bg, &props.ink),
        props.pad.clamp(6, 28),
        if props.align == "left" {
            "left"
        } else {
            "center"
        }
    );
    let href = props.href.trim().to_string();
    view! {
        <div class="mk-nw h-full">
            <div class="nw-ann" style=style>
                {if props.link_label.trim().is_empty() {
                    // Barras de antes: todo el mensaje es el enlace.
                    view! {
                        <MaybeLink href=href editing class="nw-ann-inner">
                            {(props.show_tag && !props.tag.is_empty()).then(|| view! {
                                <span class="nw-ann-mark">{props.tag.clone()}</span>
                            })}
                            <span>{props.message.clone()}</span>
                        </MaybeLink>
                    }.into_any()
                } else {
                    view! {
                        <div class="nw-ann-inner">
                            {(props.show_tag && !props.tag.is_empty()).then(|| view! {
                                <span class="nw-ann-mark">{props.tag.clone()}</span>
                            })}
                            <span>{props.message.clone()}</span>
                            <MaybeLink href=href editing class="nw-ann-link">
                                {props.link_label.clone()}
                            </MaybeLink>
                        </div>
                    }.into_any()
                }}
            </div>
        </div>
    }
}

/// Separa segundos en días, horas, minutos y segundos.
pub fn split_seconds(total: i64) -> [i64; 4] {
    let t = total.max(0);
    [t / 86_400, t % 86_400 / 3600, t % 3600 / 60, t % 60]
}

/// Historia destacada (diseños: lado a lado, panorámica, retrato).
#[component]
pub fn CardWidget(props: CardProps, editing: bool) -> impl IntoView {
    // Sin adónde llevar, el botón no se muestra en la tienda (en el editor sí,
    // con un aviso en el panel).
    let has_button = !props.button_label.is_empty()
        && (editing || !props.button_href.trim().is_empty());
    let class = "nw-story";
    view! {
        <div class="mk-nw h-full overflow-hidden">
            <section class="nw-section">
                <div class=class style=nw_colors(&props.bg, &props.ink)>
                    {(!props.image_url.is_empty()).then(|| view! {
                        <div class="nw-img"><Image src=props.image_url.clone() alt=if props.image_alt.is_empty() { props.title.clone() } else { props.image_alt.clone() }/></div>
                    })}
                    <div class="nw-story-copy">
                        <h4><AnimText text=props.title.clone()/></h4>
                        {(!props.body.is_empty()).then(|| view! { <p>{props.body.clone()}</p> })}
                        {has_button.then(|| view! {
                            <span style=nw_on_color(&props.button_color)>
                                <MaybeLink href=props.button_href.clone() editing class="nw-action">
                                    {format!("{} ↗", props.button_label)}
                                </MaybeLink>
                            </span>
                        })}
                    </div>
                </div>
            </section>
        </div>
    }
}

/// Preguntas frecuentes (diseños: líneas simples, acordeones enmarcados, índice lateral).
#[component]
pub fn FaqWidget(props: FaqProps, #[prop(default = false)] editing: bool) -> impl IntoView {
    let _ = editing;
    let open_first = props.open_first;
    view! {
        <div class="mk-nw h-full overflow-hidden">
            <section class="nw-section nw-faq" style=nw_colors(&props.bg, &props.ink)>
                <div class="nw-section-head">
                    <div>
                        {(!props.kicker.is_empty()).then(|| view! { <div class="nw-kicker">{props.kicker.clone()}</div> })}
                        {(!props.title.is_empty()).then(|| view! { <h4><AnimText text=props.title.clone()/></h4> })}
                    </div>
                </div>
                <div class="nw-faq-wrap">
                    {props
                        .items
                        .into_iter()
                        .enumerate()
                        .map(|(i, it)| view! {
                            <details open=open_first && i == 0>
                                <summary><span>{it.question}</span></summary>
                                <p>{it.answer}</p>
                            </details>
                        })
                        .collect_view()}
                </div>
            </section>
        </div>
    }
}

/// Cita de clientes (diseños: editorial, enmarcada, cita lateral).
#[component]
pub fn TestimonialsWidget(props: TestimonialsProps) -> impl IntoView {
    let colors = nw_colors(&props.bg, &props.ink);
    let star_style = String::new();
    let show_stars = props.show_stars;
    view! {
        <div class="mk-nw h-full overflow-hidden" style=colors>
            {props
                .items
                .into_iter()
                .map(|t| {
                    let n = t.rating.clamp(1, 5) as usize;
                    let stars = format!("{}{}", "★".repeat(n), "☆".repeat(5 - n));
                    let who = [t.note.clone(), t.name.clone()]
                        .into_iter()
                        .filter(|x| !x.is_empty())
                        .collect::<Vec<_>>()
                        .join(" · ");
                    view! {
                        <section class="nw-testimonial">
                            <div class="nw-testimonial-inner">
                                <span class="nw-quote-mark" aria-hidden="true">"“"</span>
                                <div>
                                    {show_stars.then(|| view! {
                                        <span class="nw-stars" style=star_style.clone() role="img" aria-label=format!("{n} de 5 estrellas")>{stars.clone()}</span>
                                    })}
                                    <blockquote>{format!("“{}”", t.text)}</blockquote>
                                    <small>{who}</small>
                                </div>
                            </div>
                        </section>
                    }
                })
                .collect_view()}
        </div>
    }
}

/// Un medio de contacto de la tienda: ícono, nombre, lo que se ve
/// (número, usuario o email) y el enlace.
struct Channel {
    icon: &'static str,
    label: &'static str,
    value: String,
    href: String,
    /// WhatsApp e Instagram abren en otra pestaña; el email, en el programa
    /// de correo.
    external: bool,
}

/// Los medios de contacto que el dueño cargó en "Datos de la tienda", en
/// este orden: WhatsApp, Instagram, email.
fn store_channels(store: &domain::StoreInfo) -> Vec<Channel> {
    let mut out = Vec::new();
    if let Some(href) = store.whatsapp_url() {
        out.push(Channel {
            icon: "whatsapp",
            label: "WhatsApp",
            value: store.whatsapp.clone(),
            href,
            external: true,
        });
    }
    if let Some(href) = store.instagram_url() {
        out.push(Channel {
            icon: "instagram",
            label: "Instagram",
            value: format!("@{}", store.instagram),
            href,
            external: true,
        });
    }
    if !store.email.is_empty() {
        out.push(Channel {
            icon: "mail",
            label: "Email",
            value: store.email.clone(),
            href: format!("mailto:{}", store.email),
            external: false,
        });
    }
    out
}

/// Enlace a un medio de contacto. Mientras se edita es un `div` sin
/// `data-href`: son direcciones de afuera, no páginas de la tienda.
#[component]
fn ChannelLink(
    href: String,
    external: bool,
    editing: bool,
    #[prop(into)] class: String,
    #[prop(into, optional)] label: Option<String>,
    children: Children,
) -> impl IntoView {
    if editing {
        view! { <div class=class aria-label=label>{children()}</div> }.into_any()
    } else {
        let (target, rel) = if external {
            (Some("_blank"), Some("noopener"))
        } else {
            (None, None)
        };
        view! { <a class=class href=href target=target rel=rel aria-label=label>{children()}</a> }
            .into_any()
    }
}

#[component]
pub fn ContactWidget(props: ContactProps, editing: bool) -> impl IntoView {
    let store = use_context::<domain::StoreInfo>().unwrap_or_default();
    let mut channels = store_channels(&store);
    // WhatsApp va como botón principal; el resto, como filas.
    let whatsapp =
        (channels.first().is_some_and(|c| c.icon == "whatsapp")).then(|| channels.remove(0));
    let has_channels = whatsapp.is_some() || !channels.is_empty();
    let others_title = if whatsapp.is_some() {
        "También podés"
    } else {
        "Escribinos"
    };
    // Los horarios del bloque; si sigue el texto de ejemplo, los de Datos de
    // la tienda.
    let hours = match props.hours.trim() {
        h if (h.is_empty() || domain::is_placeholder(h)) && !store.hours.is_empty() => {
            store.hours.clone()
        }
        h => h.to_string(),
    };
    let address = props.address.trim().to_string();
    let root = if has_channels || editing {
        "mk-box mk-contact @container h-full overflow-hidden rounded-box bg-base-100"
    } else {
        "mk-box mk-contact mk-contact-solo @container h-full overflow-hidden rounded-box bg-base-100"
    };
    view! {
        <section class=root>
            <div class="mk-contact-grid grid h-full">
                <div class="mk-contact-side flex flex-col justify-center gap-6 p-6 @3xl:p-10">
                    <div>
                        <h2 class="mk-contact-title mk-h text-3xl font-bold leading-tight text-balance @3xl:text-4xl">{props.title.clone()}</h2>
                        {(!props.text.trim().is_empty()).then(|| view! {
                            <p class="mt-3 max-w-md text-pretty opacity-90">{props.text.clone()}</p>
                        })}
                    </div>
                    {(!hours.is_empty() || !address.is_empty()).then(|| view! {
                        <dl class="grid gap-4">
                            {(!hours.is_empty()).then(|| view! {
                                <div class="flex gap-3">
                                    <Icon name="clock" class="mt-0.5 h-5 w-5 shrink-0"/>
                                    <div>
                                        <dt class="font-semibold">"Horario de atención"</dt>
                                        <dd class="whitespace-pre-line opacity-85">{hours.clone()}</dd>
                                    </div>
                                </div>
                            })}
                            {(!address.is_empty()).then(|| view! {
                                <div class="flex gap-3">
                                    <Icon name="pin" class="mt-0.5 h-5 w-5 shrink-0"/>
                                    <div>
                                        <dt class="font-semibold">"Dónde estamos"</dt>
                                        <dd class="opacity-85">{address.clone()}</dd>
                                    </div>
                                </div>
                            })}
                        </dl>
                    })}
                </div>
                {(has_channels || editing).then(|| view! {
                    <div class="mk-contact-ways flex flex-col justify-center gap-3 p-6 @3xl:p-10">
                        {whatsapp.map(|c| view! {
                            <p class="text-sm font-semibold opacity-70">"La forma más rápida"</p>
                            <ChannelLink href=c.href external=c.external editing class="mk-wa flex min-h-12 items-center gap-3 rounded-field px-4 py-3 font-semibold">
                                <Icon name="whatsapp" class="h-5 w-5 shrink-0"/>
                                <span class="min-w-0 flex-1">"Escribinos por WhatsApp"</span>
                                <Icon name="arrow-right" class="h-5 w-5 shrink-0"/>
                            </ChannelLink>
                        })}
                        {(!channels.is_empty()).then(|| view! {
                            <p class="mt-2 text-sm font-semibold opacity-70">{others_title}</p>
                            {channels
                                .into_iter()
                                .map(|c| view! {
                                    <ChannelLink href=c.href external=c.external editing class="mk-item mk-contact-way flex min-h-12 items-center gap-3 rounded-field border border-base-content/15 px-3 py-2.5">
                                        <span class="mk-icon flex h-9 w-9 shrink-0 items-center justify-center rounded-full">
                                            <Icon name=c.icon class="h-5 w-5"/>
                                        </span>
                                        <span class="min-w-0 flex-1">
                                            <span class="block font-semibold">{c.label}</span>
                                            <span class="block truncate text-sm opacity-70">{c.value}</span>
                                        </span>
                                        <Icon name="arrow-right" class="h-4 w-4 shrink-0 opacity-50"/>
                                    </ChannelLink>
                                })
                                .collect_view()}
                        })}
                        {(!has_channels).then(|| view! {
                            <p class="rounded-field border border-dashed border-base-content/30 p-4 text-sm opacity-80">
                                "Cargá tu WhatsApp, Instagram o email en Datos de la tienda y aparecen acá."
                            </p>
                        })}
                    </div>
                })}
            </div>
        </section>
    }
}

/// Pie de página (diseños: columnas, marca protagonista, bloque contenido).
#[component]
pub fn FooterWidget(props: FooterProps, editing: bool) -> impl IntoView {
    let store = use_context::<domain::StoreInfo>().unwrap_or_default();
    let brand = or_store(&props.brand, &store.name);
    let legal = if (props.legal.trim().is_empty() || props.legal.ends_with("Nombre de tu tienda"))
        && !store.name.is_empty()
    {
        format!("© {}", store.name)
    } else {
        props.legal.clone()
    };
    // La frase del bloque o, si sigue la de ejemplo, la de Datos de la tienda.
    let own = props.description.trim();
    let phrase = if own.is_empty() || domain::is_placeholder(own) && !store.tagline.is_empty() {
        store.tagline.clone()
    } else {
        own.to_string()
    };
    // Abajo, en una sola línea: el texto legal y los horarios.
    let bottom: Vec<String> = [legal, store.hours.clone()]
        .into_iter()
        .filter(|t| !t.is_empty())
        .collect();
    let bottom = bottom.join(" · ");
    let logo = store.logo_url.clone();
    let channels = store_channels(&store);
    // Cada enlace con "hijos" es una columna con título; los enlaces sueltos
    // de los documentos viejos van todos juntos en una columna.
    let (groups, loose): (Vec<_>, Vec<_>) = props
        .links
        .into_iter()
        .partition(|l| !l.children.is_empty());
    let mut columns: Vec<(String, String, Vec<domain::LinkItem>)> = groups
        .into_iter()
        .map(|g| (g.label, g.href, g.children))
        .collect();
    if !loose.is_empty() {
        columns.push((String::new(), String::new(), loose));
    }
    let n = columns.len().max(1);
    // En la tienda, «Ingresar a mi cuenta» y el botón de arrepentimiento van
    // acá adentro (ver `storefront::StorefrontPage`).
    let legal_links = !editing && use_context::<LegalInFooter>().is_some();
    let show_desc = props.show_description && !phrase.is_empty();
    view! {
        <div class="mk-nw h-full overflow-hidden">
            <footer class="nw-footer" style=nw_colors(&props.bg, &props.ink)>
                <div class="nw-footer-grid" style=format!("--fc:{n}")>
                    <div class="nw-footer-brand">
                        <div class="nw-brand">
                            {(!logo.is_empty()).then(|| view! {
                                <img src=logo.clone() alt="" class="mr-2 inline-block h-8 w-auto max-w-32 object-contain align-middle"/>
                            })}
                            {brand}
                            {props.show_dot.then(|| view! { <i>"."</i> })}
                        </div>
                        {show_desc.then(|| view! { <p>{phrase.clone()}</p> })}
                        {(!channels.is_empty()).then(|| view! {
                            <div class="nw-foot-icons">
                                {channels
                                    .into_iter()
                                    .map(|c| view! {
                                        <ChannelLink href=c.href external=c.external editing label=c.label class="">
                                            <Icon name=c.icon class="h-4 w-4"/>
                                        </ChannelLink>
                                    })
                                    .collect_view()}
                            </div>
                        })}
                    </div>
                    {columns
                        .into_iter()
                        .map(|(title, href, links)| view! {
                            <div>
                                {(!title.is_empty()).then(|| view! {
                                    <b>
                                        <MaybeLink href=href editing class="nw-foot-link">{title.clone()}</MaybeLink>
                                    </b>
                                })}
                                {links.into_iter().map(|l| view! {
                                    <MaybeLink href=l.href editing class="nw-foot-link">{l.label}</MaybeLink>
                                }).collect_view()}
                            </div>
                        })
                        .collect_view()}
                </div>
                {(!bottom.is_empty() || legal_links).then(|| view! {
                    <p class="nw-footer-legal">
                        <span>{bottom}</span>
                        {legal_links.then(|| view! {
                            <span class="flex flex-wrap items-center gap-x-2">
                                <crate::account::AccountFooterLink/>
                                <span aria-hidden="true">"·"</span>
                                <a class="nw-foot-link underline" href="/arrepentimiento">"Botón de arrepentimiento"</a>
                            </span>
                        })}
                    </p>
                })}
            </footer>
        </div>
    }
}

/// El valor del bloque, o el de la tienda si el bloque lo dejó vacío.
fn or_store(own: &str, store: &str) -> String {
    // El texto de ejemplo de bloques viejos también cede al dato de la tienda.
    let placeholder = own.trim().is_empty() || own.trim() == "Nombre de tu tienda";
    if placeholder && !store.is_empty() {
        store
    } else {
        own
    }
    .to_string()
}

/// Cambia los datos de la dirección (lo que va después de `?`) sin recargar
/// ni mover la pantalla. Con `push`, queda un paso nuevo para el botón Atrás;
/// si no, reemplaza la dirección actual. Si no cambia nada, no hace nada.
pub(crate) fn edit_url(push: bool, change: impl FnOnce(&web_sys::UrlSearchParams)) {
    let Some(w) = web_sys::window() else {
        return;
    };
    let loc = w.location();
    let (Ok(path), Ok(search), Ok(hash)) = (loc.pathname(), loc.search(), loc.hash()) else {
        return;
    };
    let Ok(params) = web_sys::UrlSearchParams::new_with_str(&search) else {
        return;
    };
    change(&params);
    let query: String = params.to_string().into();
    if query == search.trim_start_matches('?') {
        return;
    }
    let url = if query.is_empty() {
        format!("{path}{hash}")
    } else {
        format!("{path}?{query}{hash}")
    };
    let Ok(history) = w.history() else {
        return;
    };
    // Se conserva lo que ya guardaba esta entrada (por ejemplo, el router).
    let state = history.state().unwrap_or(wasm_bindgen::JsValue::NULL);
    _ = if push {
        history.push_state_with_url(&state, "", Some(&url))
    } else {
        history.replace_state_with_url(&state, "", Some(&url))
    };
}

/// El producto de `?ver=<producto>` en la dirección, si hay.
fn product_in_url() -> Option<String> {
    let search = web_sys::window()
        .and_then(|w| w.location().search().ok())
        .unwrap_or_default();
    search
        .trim_start_matches('?')
        .split('&')
        .find_map(|kv| kv.strip_prefix("ver="))
        .filter(|v| {
            !v.is_empty() && v.len() <= 36 && v.bytes().all(|b| b.is_ascii_hexdigit() || b == b'-')
        })
        .map(str::to_string)
}

/// La ficha y la dirección van juntas. Si la dirección trae `?ver=<producto>`
/// (un enlace compartido, «Mis favoritos»), abre esa ficha. Al abrir una
/// ficha la dirección pasa a `?ver=<producto>`, así se puede copiar y mandar,
/// y queda un paso en el historial: el botón Atrás cierra la ficha en lugar
/// de sacar al comprador de la página. Solo en el navegador.
pub fn open_from_query(open: RwSignal<Option<String>>) {
    // ¿La entrada actual del historial la sumó una ficha abierta acá?
    let pushed = StoredValue::new(false);
    Effect::new(move |_| {
        if let Some(id) = product_in_url() {
            open.set(Some(id));
        }
        // Atrás o Adelante: la ficha sigue a la dirección.
        let handle = window_event_listener(leptos::ev::popstate, move |_| {
            let now = product_in_url();
            if now.is_none() {
                pushed.set_value(false);
            }
            if open.get_untracked() != now {
                open.set(now);
            }
        });
        on_cleanup(move || handle.remove());
    });
    Effect::new(move |_| {
        let (want, now) = (open.get(), product_in_url());
        if want == now {
            return;
        }
        match (want, now) {
            // De una ficha a otra: es el mismo paso del historial.
            (Some(id), Some(_)) => edit_url(false, |p| p.set("ver", &id)),
            (Some(id), None) => {
                edit_url(true, |p| p.set("ver", &id));
                pushed.set_value(true);
            }
            // Se cerró con la ✕, Esc o tocando afuera: se deshace el paso.
            (None, _) if pushed.get_value() => {
                pushed.set_value(false);
                if let Some(h) = web_sys::window().and_then(|w| w.history().ok()) {
                    _ = h.back();
                }
            }
            // Entró directo con el enlace: no hay paso que deshacer.
            (None, _) => edit_url(false, |p| p.delete("ver")),
        }
    });
}
