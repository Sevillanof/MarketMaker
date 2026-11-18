#![recursion_limit = "256"]

//! Render de cada widget. Un componente por variante de [`Widget`].
//!
//! Los widgets usan solo colores semánticos de daisyUI y se adaptan al ancho
//! y alto que les toca en la grilla (con container queries `@container`).
//! Los textos se muestran siempre como texto, nunca como HTML.

use domain::{
    AnnouncementProps, ProductCarouselProps, SocialProps, StatsProps, StepsProps,
    BannerProps, BenefitsProps, CardProps, ContactProps, CarouselProps, CategoriesProps, CountdownProps,
    FaqProps, FeaturedProductProps, FooterProps, HoverGalleryProps, ImageProps, LogosProps,
    NavbarProps, Node, Product, ProductButtonProps, ProductGridProps, ProductImageProps,
    ProductModal, TabsProps, TestimonialsProps, TextProps, Widget,
};
use leptos::html::Div;
use leptos::prelude::*;
use ui::{Button, Card, Icon, Image, Picture, PriceWas, Skeleton};

pub mod account;
pub mod cart;
pub mod tags;
pub use account::{Account, AccountNavButton, AccountState, BuyerAvatar};
pub use cart::{AddToCart, CardCartButton, Cart, CartBar, CartDrawer, CartNavButton};
pub use tags::{FilteredProducts, ProductFilter};

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

/// Producto abierto en la ficha de la tienda, por contexto. Si está, las
/// tarjetas de producto se pueden tocar para abrir su ficha.
#[derive(Clone, Copy)]
pub struct OpenProduct(pub RwSignal<Option<String>>);

/// Dibuja un nodo dentro de su envoltorio de letras (`.mk-type`, ver
/// [`domain::TextStyle`] y `style/tailwind.css`).
pub fn render_widget(node: &Node, ctx: &RenderCtx) -> AnyView {
    let class = format!("{} {} h-full", node.style.classes(), node.look.classes());
    let style = format!("{}; {}", node.style.css_vars(), node.look.css_vars());
    let content = render_content(node, ctx);
    view! { <div class=class style=style>{content}</div> }.into_any()
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
            view! { <ProductGridWidget props=p.clone() products=ctx.products.clone()/> }.into_any()
        }
        Widget::TagFilter(_)
        | Widget::TagList(_)
        | Widget::TagCircles(_)
        | Widget::TagSearch(_)
        | Widget::TagTabs(_) => tags::render(&node.widget, ctx),
        Widget::FeaturedProduct(p) => {
            view! { <FeaturedProductWidget props=p.clone() products=ctx.products.clone()/> }
                .into_any()
        }
        Widget::Navbar(p) => {
            view! { <NavbarWidget props=p.clone() editing uid=format!("nav-{}", node.id)/> }
                .into_any()
        }
        Widget::Announcement(p) => {
            view! { <AnnouncementWidget props=p.clone() editing/> }.into_any()
        }
        Widget::Stats(p) => view! { <StatsWidget props=p.clone()/> }.into_any(),
        Widget::Steps(p) => view! { <StepsWidget props=p.clone()/> }.into_any(),
        Widget::ProductCarousel(p) => view! {
            <ProductCarouselWidget props=p.clone() products=ctx.products.clone() editing/>
        }
        .into_any(),
        Widget::Social(p) => view! { <SocialWidget props=p.clone() editing/> }.into_any(),
        Widget::Carousel(p) => view! { <CarouselWidget props=p.clone() editing/> }.into_any(),
        Widget::Categories(p) => view! { <CategoriesWidget props=p.clone() editing/> }.into_any(),
        Widget::Benefits(p) => view! { <BenefitsWidget props=p.clone()/> }.into_any(),
        Widget::Countdown(p) => view! { <CountdownWidget props=p.clone() editing/> }.into_any(),
        Widget::Card(p) => view! { <CardWidget props=p.clone() editing/> }.into_any(),
        Widget::Tabs(p) => {
            view! { <TabsWidget props=p.clone() group=format!("tabs-{}", node.id)/> }.into_any()
        }
        Widget::Faq(p) => view! { <FaqWidget props=p.clone()/> }.into_any(),
        Widget::HoverGallery(p) => {
            view! { <HoverGalleryWidget props=p.clone() editing/> }.into_any()
        }
        Widget::Testimonials(p) => view! { <TestimonialsWidget props=p.clone()/> }.into_any(),
        Widget::Logos(p) => view! { <LogosWidget props=p.clone() editing/> }.into_any(),
        Widget::Image(p) => view! { <ImageWidget props=p.clone() editing/> }.into_any(),
        Widget::Contact(p) => view! { <ContactWidget props=p.clone() editing/> }.into_any(),
        Widget::Footer(p) => view! { <FooterWidget props=p.clone() editing/> }.into_any(),
        Widget::ProductImage(p) => {
            view! { <ProductImageWidget props=p.clone() product=ctx.product.clone()/> }.into_any()
        }
        Widget::ProductName(_) => {
            let name = ctx
                .product
                .as_ref()
                .map(|p| p.name.clone())
                .unwrap_or_default();
            view! {
                <div class="mk-box mk-pad mk-pname flex h-full items-start overflow-hidden rounded-box">
                    <h2 class="mk-title text-2xl font-bold leading-tight line-clamp-2">{name}</h2>
                </div>
            }
            .into_any()
        }
        Widget::ProductPrice(_) => {
            let (cents, was, off) = ctx.product.as_ref().map_or((0, 0, None), |p| {
                (p.price_cents, p.compare_at_cents, p.discount_pct())
            });
            view! {
                <div class="mk-pprice flex h-full flex-wrap items-start gap-2 overflow-hidden text-xl">
                    <span class="mk-pricetag"><PriceWas cents was/></span>
                    {off.map(|n| view! { <span class="badge badge-error">{format!("-{n}%")}</span> })}
                </div>
            }
            .into_any()
        }
        Widget::ProductDescription(_) => {
            let text = ctx
                .product
                .as_ref()
                .map(|p| p.description.clone())
                .unwrap_or_default();
            let empty = text.is_empty();
            view! {
                <div class="mk-box mk-pad mk-pdesc h-full overflow-y-auto rounded-box">
                    {if empty && editing {
                        view! { <p class="text-base-content/60 italic">"Este producto todavía no tiene descripción."</p> }.into_any()
                    } else {
                        view! { <p class="whitespace-pre-line opacity-85">{text}</p> }.into_any()
                    }}
                </div>
            }
            .into_any()
        }
        Widget::ProductButton(p) => {
            view! { <ProductButtonWidget props=p.clone() editing product=ctx.product.clone()/> }
                .into_any()
        }
    }
}

#[component]
pub fn ProductImageWidget(props: ProductImageProps, product: Option<Product>) -> impl IntoView {
    let (photos, alt, fits) =
        product.map_or_else(Default::default, |p| (p.all_photos(), p.name, p.photo_fits));
    let contain = props.fit == "contain";
    if photos.len() < 2 {
        let src = photos.into_iter().next().unwrap_or_default();
        let fit = fits.get(&src).copied();
        return view! {
            <div class="mk-box mk-image h-full overflow-hidden rounded-box bg-base-200">
                <Image src alt contain fit/>
            </div>
        }
        .into_any();
    }
    // Varias fotos: la grande arriba y las miniaturas abajo para cambiarla.
    let shown = RwSignal::new(0usize);
    let photos = StoredValue::new(photos);
    let fits = StoredValue::new(fits);
    let total = photos.with_value(Vec::len);
    view! {
        <div class="mk-image flex h-full flex-col gap-2">
            <div class="mk-box relative min-h-0 flex-1 overflow-hidden rounded-box bg-base-200">
                {move || {
                    let i = shown.get();
                    let src = photos.with_value(|p| p[i].clone());
                    let fit = fits.with_value(|f| f.get(&src).copied());
                    view! { <Image src alt=format!("{alt}, foto {} de {total}", i + 1) contain fit/> }
                }}
            </div>
            <div class="flex shrink-0 gap-2 overflow-x-auto" role="group" aria-label="Fotos del producto">
                {(0..total).map(|i| {
                    let src = photos.with_value(|p| p[i].clone());
                    let fit = fits.with_value(|f| f.get(&src).copied());
                    view! {
                        <button type="button"
                            class="h-14 w-14 shrink-0 overflow-hidden rounded-field -outline-offset-2 outline-primary"
                            class:outline-2=move || shown.get() == i
                            aria-label=format!("Ver foto {}", i + 1)
                            aria-pressed=move || (shown.get() == i).to_string()
                            on:click=move |_| shown.set(i)>
                            <Image src alt="" fit/>
                        </button>
                    }
                }).collect_view()}
            </div>
        </div>
    }
    .into_any()
}

/// Texto del botón de la ficha. Las fichas viejas decían "Consultar por este
/// producto" y llevaban a Contacto; ahora el botón agrega al carrito.
pub fn product_button_label(label: &str) -> String {
    match label.trim() {
        "" | "Consultar por este producto" => "Agregar al carrito".to_string(),
        other => other.to_string(),
    }
}

#[component]
pub fn ProductButtonWidget(
    props: ProductButtonProps,
    editing: bool,
    /// El producto de la ficha. Sin stock: el botón dice "Agotado".
    product: Option<Product>,
) -> impl IntoView {
    if product.as_ref().is_some_and(|p| !p.in_stock) {
        return view! {
            <div class="mk-pbtn flex h-full items-start">
                <button type="button" class="btn btn-primary mk-btn" disabled>"Agotado"</button>
            </div>
        }
        .into_any();
    }
    view! { <AddToCart product label=product_button_label(&props.label) editing/> }.into_any()
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
    let products = StoredValue::new(products);
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
            class="modal mk-dialog"
            aria-label=move || product.get().map(|p| p.name).unwrap_or_default()
            on:close=move |_| open.set(None)
        >
            <div class="modal-box mk-dialog-box">
                <form method="dialog" class="absolute right-2 top-2 z-10">
                    <button class="btn btn-sm btn-circle btn-ghost" aria-label="Cerrar">
                        <svg class="h-4 w-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" aria-hidden="true"><path d="M6 6l12 12M18 6L6 18"/></svg>
                    </button>
                </form>
                {move || product.get().map(|p| {
                    let ctx = RenderCtx {
                        products: products.get_value(),
                        editing,
                        product: Some(p),
                    };
                    view! {
                        <div class="mk-grid">
                            {nodes
                                .get_value()
                                .into_iter()
                                .map(|node| view! {
                                    <div class="mk-node" style=node_style(&node)>{render_widget(&node, &ctx)}</div>
                                })
                                .collect_view()}
                        </div>
                    }
                })}
            </div>
            <form method="dialog" class="modal-backdrop">
                <button tabindex="-1">"Cerrar"</button>
            </form>
        </dialog>
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

#[component]
pub fn BannerWidget(props: BannerProps, editing: bool) -> impl IntoView {
    let has_image = !props.image_url.is_empty();
    let has_button = !props.button_label.is_empty();
    let root_class = if has_image {
        "mk-box mk-banner mk-has-img @container hero relative h-full overflow-hidden rounded-box bg-base-200"
    } else {
        "mk-box mk-banner @container hero relative h-full overflow-hidden rounded-box bg-base-200"
    };
    view! {
        <div class=root_class>
            {has_image.then(|| view! {
                <div class="mk-banner-img absolute inset-0">
                    <Image src=props.image_url.clone() alt=props.title.clone() eager=true/>
                </div>
                <div class="mk-banner-ov hero-overlay absolute inset-0"></div>
            })}
            <div class="mk-banner-body hero-content relative text-center">
                <div class="mk-banner-panel max-w-xl">
                    <h1 class="mk-title text-xl font-bold line-clamp-2 @xl:text-3xl">{props.title.clone()}</h1>
                    {(!props.subtitle.is_empty()).then(|| view! {
                        <p class="py-3 line-clamp-2 text-sm @xl:text-base">{props.subtitle.clone()}</p>
                    })}
                    {has_button.then(|| view! {
                        <Button label=props.button_label.clone() href=props.button_href.clone() disabled=editing/>
                    })}
                </div>
            </div>
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
                <h2 class="mk-title text-xl font-bold mb-2 truncate">{props.heading.clone()}</h2>
            })}
            <p class="whitespace-pre-line">{props.body}</p>
        </div>
    }
}

#[component]
pub fn ProductGridWidget(props: ProductGridProps, products: Vec<Product>) -> impl IntoView {
    let limit = usize::try_from(props.limit).unwrap_or(0);
    let filter = use_context::<ProductFilter>();
    view! {
        <div class="mk-box mk-pad @container flex h-full flex-col gap-3 overflow-hidden rounded-box">
            {(!props.title.is_empty()).then(|| view! {
                <h2 class="mk-title text-xl font-bold truncate">{props.title.clone()}</h2>
            })}
            <FilteredProducts products limit filter/>
        </div>
    }
}

#[component]
pub fn FeaturedProductWidget(props: FeaturedProductProps, products: Vec<Product>) -> impl IntoView {
    let product = products.into_iter().find(|p| p.id == props.product_id);
    match product {
        Some(p) => view! {
            <div class="mk-box mk-pad h-full rounded-box">
                <ProductCard product=p show_price=props.show_price/>
            </div>
        }
        .into_any(),
        None => view! {
            <div class="flex h-full items-center justify-center rounded-box border border-dashed border-base-300 bg-base-200 p-4 text-center">
                "Elegí un producto"
            </div>
        }
        .into_any(),
    }
}

/// Tarjeta de producto: la imagen ocupa todo el alto que deja el texto.
/// En la tienda (con [`OpenProduct`]) se puede tocar para abrir su ficha.
#[component]
pub fn ProductCard(product: Product, show_price: bool) -> impl IntoView {
    let open = use_context::<OpenProduct>();
    let id = product.id.clone();
    let name = product.name.clone();
    let sold_out = !product.in_stock;
    let few_left = product.few_left();
    let off = show_price.then(|| product.discount_pct()).flatten();
    let cart_product = product.clone();
    let fit = product.fit(&product.image_url);
    view! {
        <div class="relative h-full" data-product-id=product.id.clone()>
        <Card>
            <div class="mk-card-img relative min-h-0 flex-1">
                <div class="absolute inset-0">
                    <Image src=product.image_url.clone() alt=product.name.clone() fit/>
                </div>
                <div class="absolute left-2 top-2 flex flex-col items-start gap-1">
                    {sold_out.then(|| view! { <span class="badge badge-neutral">"Agotado"</span> })}
                    {few_left.map(|t| view! { <span class="badge badge-warning">{t}</span> })}
                    {off.map(|n| view! {
                        <span class="badge badge-error">{format!("-{n}%")}</span>
                    })}
                </div>
            </div>
            <div class="card-body grow-0 shrink-0 gap-0">
                <h3 class="mk-sub card-title text-sm truncate block">{product.name.clone()}</h3>
                {show_price.then(|| view! { <PriceWas cents=product.price_cents was=product.compare_at_cents/> })}
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
    let view_w = win.inner_width().ok().and_then(|w| w.as_f64()).unwrap_or(1024.0);
    let anchor = if wide {
        btn.closest("nav").ok().flatten().unwrap_or_else(|| btn.clone().into())
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

#[component]
pub fn NavbarWidget(props: NavbarProps, editing: bool, uid: String) -> impl IntoView {
    let has_button = !props.button_label.is_empty();
    // Vacíos, usan el nombre y el logo de la tienda (ver "Datos de la tienda").
    let store = use_context::<domain::StoreInfo>().unwrap_or_default();
    let brand = or_store(&props.brand, &store.name);
    let logo_url = or_store(&props.logo_url, &store.logo_url);
    let wide = props.menu == "panel";
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
            let children = l.children.clone();
            let image = l.image_url.clone();
            let all_href = l.href.clone();
            let label = l.label.clone();
            let label_btn = l.label.clone();
            let panel = if wide {
                view! {
                    <div class="grid gap-6 p-5 @3xl:grid-cols-[1fr_1fr_1.2fr]">
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
                        {(!image.is_empty()).then(|| view! {
                            <MaybeLink href=all_href.clone() editing class="relative block overflow-hidden rounded-box">
                                <div class="aspect-[16/9] w-full"><Image src=image.clone() alt=""/></div>
                                <span class="absolute inset-x-0 bottom-0 bg-gradient-to-t from-black/70 to-transparent p-3 text-sm font-semibold text-white">{format!("{label} →")}</span>
                            </MaybeLink>
                        })}
                    </div>
                }
                .into_any()
            } else {
                view! {
                    <ul class="menu w-52 p-1">
                        {children.into_iter().map(|c| view! {
                            <li><MaybeLink href=c.href editing class="mk-link">{c.label}</MaybeLink></li>
                        }).collect_view()}
                        {(!all_href.is_empty()).then(|| view! {
                            <li class="mt-1 border-t border-base-300 pt-1"><MaybeLink href=all_href.clone() editing class="mk-link font-semibold">"Ver todo"</MaybeLink></li>
                        })}
                    </ul>
                }
                .into_any()
            };
            let pop_class = if wide { "mk-navpop mk-navpop-wide @container" } else { "mk-navpop" };
            view! {
                <li>
                    <button type="button" class="mk-link btn btn-ghost btn-sm font-normal whitespace-nowrap"
                        popovertarget=pid.clone()
                        on:click=move |ev| place_popover(ev, &pid_click, wide)>
                        {label_btn}
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
        .map(|l| {
            if l.children.is_empty() {
                view! {
                    <li><MaybeLink href=l.href editing class="mk-link">{l.label}</MaybeLink></li>
                }
                .into_any()
            } else {
                view! {
                    <li>
                        <details>
                            <summary>{l.label}</summary>
                            <ul>
                                {l.children.into_iter().map(|c| view! {
                                    <li><MaybeLink href=c.href editing class="mk-link">{c.label}</MaybeLink></li>
                                }).collect_view()}
                            </ul>
                        </details>
                    </li>
                }
                .into_any()
            }
        })
        .collect_view();
    view! {
        <nav class="mk-box mk-nav navbar @container h-full min-h-0 gap-4 rounded-box bg-base-100 px-4 shadow-sm">
            <MaybeLink href="/" editing class="flex shrink-0 items-center gap-2">
                {(!logo_url.is_empty()).then(|| view! {
                    <img class="h-8 w-8 object-contain" src=logo_url.clone() alt="" draggable="false"/>
                })}
                <span class="mk-brand mk-h text-lg font-bold whitespace-nowrap">{brand}</span>
            </MaybeLink>
            <ul class="menu menu-horizontal hidden min-w-0 flex-1 flex-nowrap items-center justify-end px-0 @2xl:flex">
                {desktop}
            </ul>
            <div class="flex-1 @2xl:hidden"></div>
            {has_button.then(|| view! {
                <Button label=props.button_label.clone() href=props.button_href.clone() disabled=editing/>
            })}
            <AccountNavButton/>
            <CartNavButton/>
            <button type="button" class="btn btn-ghost btn-square @2xl:hidden" aria-label="Abrir el menú"
                popovertarget=mobile_id.clone()
                on:click=move |ev| place_popover(ev, &mobile_click, true)>
                <Icon name="menu" class="h-6 w-6"/>
            </button>
            <div id=mobile_id popover="auto" class="mk-navpop mk-navpop-wide">
                <ul class="menu w-full p-2">{mobile}</ul>
            </div>
        </nav>
    }
}

#[component]
pub fn CarouselWidget(props: CarouselProps, editing: bool) -> impl IntoView {
    let track = NodeRef::<Div>::new();
    let current = RwSignal::new(0usize);
    let count = props.slides.len();

    let go = move |i: usize| {
        if let Some(el) = track.get_untracked() {
            let w = el.client_width();
            el.set_scroll_left(w * i as i32);
        }
    };

    // Avance automático (nunca en el editor). Solo corre en el navegador.
    let autoplay = props.autoplay;
    if !editing && autoplay > 0 && count > 1 {
        Effect::new(move |_| {
            let handle = set_interval_with_handle(
                move || go((current.get_untracked() + 1) % count),
                std::time::Duration::from_secs(autoplay as u64),
            );
            on_cleanup(move || {
                if let Ok(h) = handle {
                    h.clear();
                }
            });
        });
    }

    view! {
        <div class="mk-box mk-carousel @container relative h-full overflow-hidden rounded-box bg-base-200">
            <div
                node_ref=track
                class="carousel h-full w-full"
                on:scroll=move |_| {
                    if let Some(el) = track.get_untracked() {
                        let w = el.client_width().max(1);
                        let i = ((el.scroll_left() + w / 2) / w) as usize;
                        if current.get_untracked() != i {
                            current.set(i);
                        }
                    }
                }
            >
                {props
                    .slides
                    .into_iter()
                    .map(|s| {
                        let has_button = !s.button_label.is_empty();
                        view! {
                            <div class="carousel-item relative h-full w-full">
                                <Image src=s.image_url.clone() alt=s.title.clone()/>
                                <div class="mk-car-ov absolute inset-0 bg-gradient-to-r from-black/60 via-black/25 to-transparent"></div>
                                <div class="mk-car-body absolute inset-y-0 left-0 flex max-w-lg flex-col justify-center gap-2 px-14 py-6 text-white @2xl:gap-3">
                                    <h2 class="mk-title text-xl font-bold line-clamp-2 @2xl:text-3xl">{s.title.clone()}</h2>
                                    {(!s.subtitle.is_empty()).then(|| view! {
                                        <p class="text-sm line-clamp-2 @2xl:text-base">{s.subtitle.clone()}</p>
                                    })}
                                    {has_button.then(|| view! {
                                        <div>
                                            <Button label=s.button_label.clone() href=s.button_href.clone() disabled=editing/>
                                        </div>
                                    })}
                                </div>
                            </div>
                        }
                    })
                    .collect_view()}
            </div>
            {(count > 1).then(|| view! {
                <button
                    type="button"
                    class="mk-car-arrow btn btn-circle btn-sm absolute left-3 top-1/2 -translate-y-1/2 border-0 bg-base-100/80"
                    aria-label="Anterior"
                    on:click=move |_| go((current.get_untracked() + count - 1) % count)
                >
                    "❮"
                </button>
                <button
                    type="button"
                    class="mk-car-arrow btn btn-circle btn-sm absolute right-3 top-1/2 -translate-y-1/2 border-0 bg-base-100/80"
                    aria-label="Siguiente"
                    on:click=move |_| go((current.get_untracked() + 1) % count)
                >
                    "❯"
                </button>
                <div class="absolute bottom-3 left-1/2 flex -translate-x-1/2 gap-2">
                    {(0..count)
                        .map(|i| view! {
                            <button
                                type="button"
                                class="mk-car-dot h-2 rounded-full bg-white transition-all"
                                class=("w-6", move || current.get() == i)
                                class=("w-2", move || current.get() != i)
                                class=("opacity-50", move || current.get() != i)
                                aria-label=format!("Diapositiva {}", i + 1)
                                on:click=move |_| go(i)
                            ></button>
                        })
                        .collect_view()}
                </div>
            })}
        </div>
    }
}

#[component]
pub fn CategoriesWidget(props: CategoriesProps, editing: bool) -> impl IntoView {
    let cols = tags::fill_cols(props.items.len());
    view! {
        <div class="mk-box mk-pad mk-cats @container flex h-full flex-col gap-3 overflow-hidden rounded-box">
            {(!props.title.is_empty()).then(|| view! {
                <h2 class="mk-title text-xl font-bold truncate">{props.title.clone()}</h2>
            })}
            <div class=format!("grid min-h-0 flex-1 auto-rows-fr gap-3 {}", cols)>
                {props
                    .items
                    .into_iter()
                    .map(|c| view! {
                        <MaybeLink href=c.href editing class="mk-cat group relative block min-h-0 overflow-hidden rounded-box">
                            <div class="absolute inset-0 transition-transform duration-300 group-hover:scale-105">
                                <Image src=c.image_url.clone() alt=c.label.clone()/>
                            </div>
                            <div class="mk-cat-label absolute inset-x-0 bottom-0 bg-gradient-to-t from-black/70 to-transparent p-3 text-white">
                                <span class="mk-h font-semibold">{c.label.clone()}</span>
                            </div>
                        </MaybeLink>
                    })
                    .collect_view()}
            </div>
        </div>
    }
}

#[component]
pub fn BenefitsWidget(props: BenefitsProps) -> impl IntoView {
    view! {
        <div class="mk-box mk-benefits h-full overflow-hidden rounded-box bg-base-100 px-4 py-2">
            <div class="grid h-full auto-cols-[minmax(13rem,1fr)] grid-flow-col items-center gap-4 overflow-x-auto">
                {props
                    .items
                    .into_iter()
                    .map(|b| view! {
                        <div class="mk-benefit flex min-w-0 items-center gap-3">
                            <div class="mk-icon flex h-12 w-12 shrink-0 items-center justify-center rounded-full">
                                <Icon name=b.icon class="h-6 w-6"/>
                            </div>
                            <div class="min-w-0">
                                <p class="mk-sub mk-h font-semibold truncate">{b.title}</p>
                                <p class="text-sm opacity-70 line-clamp-2">{b.text}</p>
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

/// Columnas de una fila de `n` elementos desde el ancho medio (`@xl`).
fn cols_xl(n: usize) -> &'static str {
    match n {
        0..=2 => "@xl:grid-cols-2",
        3 => "@xl:grid-cols-3",
        _ => "@xl:grid-cols-4",
    }
}

/// Columnas de una fila de `n` elementos desde el ancho grande (`@2xl`).
fn cols_2xl(n: usize) -> &'static str {
    match n {
        0..=2 => "@2xl:grid-cols-2",
        3 => "@2xl:grid-cols-3",
        4 => "@2xl:grid-cols-4",
        _ => "@2xl:grid-cols-5",
    }
}

#[component]
pub fn AnnouncementWidget(props: AnnouncementProps, editing: bool) -> impl IntoView {
    let variant = props.variant.clone();
    let count = props.items.len().max(1);
    let items = StoredValue::new(props.items);
    let current = RwSignal::new(0usize);
    let paused = RwSignal::new(false);
    let closed = RwSignal::new(false);
    if variant == "b" && !editing && count > 1 {
        Effect::new(move |_| {
            let handle = set_interval_with_handle(
                move || {
                    if !paused.get_untracked() {
                        current.update(|c| *c = (*c + 1) % count);
                    }
                },
                std::time::Duration::from_secs(6),
            );
            on_cleanup(move || {
                if let Ok(h) = handle {
                    h.clear();
                }
            });
        });
    }
    let rotating = variant == "b";
    let closable = variant == "c";
    let message = move || {
        let i = if rotating { current.get() } else { 0 };
        items.with_value(|v| v.get(i).cloned())
    };
    let step = move |d: isize| {
        current.update(|c| *c = ((*c as isize + d).rem_euclid(count as isize)) as usize);
    };
    view! {
        <div class="mk-box mk-ann flex h-full items-center gap-2 overflow-hidden rounded-box bg-primary px-2 text-sm text-primary-content"
            class:hidden=move || closed.get()
            on:mouseenter=move |_| paused.set(true)
            on:mouseleave=move |_| paused.set(false)
            on:focusin=move |_| paused.set(true)
            on:focusout=move |_| paused.set(false)>
            {rotating.then(|| view! {
                <button type="button" class="btn btn-ghost btn-xs btn-square" aria-label="Mensaje anterior" on:click=move |_| step(-1)>
                    <Icon name="chevron-left" class="h-4 w-4"/>
                </button>
            })}
            {(!rotating && !closable).then(|| view! { <span class="w-0 flex-1"></span> })}
            <p class="min-w-0 flex-1 text-center text-balance">
                {move || message().map(|m| view! {
                    <span>{m.text}</span>
                    {(!m.link_label.is_empty()).then(|| view! {
                        " · "
                        <MaybeLink href=m.link_href.clone() editing class="mk-link inline font-semibold underline underline-offset-2">{m.link_label.clone()}</MaybeLink>
                    })}
                })}
            </p>
            {rotating.then(|| view! {
                <button type="button" class="btn btn-ghost btn-xs btn-square" aria-label="Mensaje siguiente" on:click=move |_| step(1)>
                    <Icon name="chevron-right" class="h-4 w-4"/>
                </button>
            })}
            {closable.then(|| view! {
                <button type="button" class="btn btn-ghost btn-xs btn-square" aria-label="Cerrar aviso" on:click=move |_| closed.set(true)>
                    <Icon name="close" class="h-4 w-4"/>
                </button>
            })}
            {(!rotating && !closable).then(|| view! { <span class="w-0 flex-1"></span> })}
        </div>
    }
}

#[component]
pub fn StatsWidget(props: StatsProps) -> impl IntoView {
    let n = props.items.len();
    match props.variant.as_str() {
        "b" => view! {
            <div class=format!("mk-box mk-pad mk-stats @container grid h-full grid-cols-1 content-center gap-6 overflow-hidden rounded-box bg-base-100 px-6 py-4 text-center {}", cols_xl(n))>
                {props.items.into_iter().map(|s| view! {
                    <div class="min-w-0">
                        <p class="mk-title mk-h text-4xl font-extrabold tracking-tight @xl:text-5xl">{s.value}</p>
                        <p class="mx-auto mt-2 h-0.5 w-8 bg-current opacity-60"></p>
                        <p class="mt-2 text-sm opacity-70">{s.label}</p>
                    </div>
                }).collect_view()}
            </div>
        }
        .into_any(),
        "c" => view! {
            <div class=format!("mk-stats @container grid h-full grid-cols-1 content-center gap-4 overflow-hidden {}", cols_xl(n))>
                {props.items.into_iter().map(|s| view! {
                    <div class="mk-item mk-box flex min-w-0 items-center gap-4 rounded-box border border-base-300 bg-base-100 p-5">
                        <span class="mk-icon flex h-12 w-12 shrink-0 items-center justify-center rounded-full">
                            <Icon name=s.icon class="h-6 w-6"/>
                        </span>
                        <div class="min-w-0">
                            <p class="mk-h text-2xl font-bold leading-none">{s.value}</p>
                            <p class="mt-1 text-sm opacity-70 line-clamp-2">{s.label}</p>
                        </div>
                    </div>
                }).collect_view()}
            </div>
        }
        .into_any(),
        _ => view! {
            <div class="mk-stats @container h-full overflow-hidden">
                <div class="mk-box stats stats-vertical @xl:stats-horizontal h-full w-full overflow-hidden rounded-box bg-base-100 shadow-sm">
                    {props.items.into_iter().map(|s| view! {
                        <div class="stat place-items-center">
                            <div class="stat-value mk-h text-primary">{s.value}</div>
                            <div class="stat-title whitespace-normal text-center">{s.label}</div>
                        </div>
                    }).collect_view()}
                </div>
            </div>
        }
        .into_any(),
    }
}

#[component]
pub fn StepsWidget(props: StepsProps) -> impl IntoView {
    let n = props.items.len();
    let title = (!props.title.is_empty()).then(|| view! {
        <h2 class="mk-title mk-h mb-4 text-center text-xl font-bold">{props.title.clone()}</h2>
    });
    match props.variant.as_str() {
        "b" => view! {
            <div class="mk-box mk-steps @container flex h-full flex-col justify-center overflow-hidden rounded-box bg-base-100 p-6 shadow-sm">
                {title}
                <ul class="steps steps-vertical @2xl:steps-horizontal w-full">
                    {props.items.into_iter().map(|s| view! {
                        <li class="step step-primary"><span class="mk-sub mk-h font-medium">{s.title}</span></li>
                    }).collect_view()}
                </ul>
            </div>
        }
        .into_any(),
        "c" => view! {
            <div class="mk-steps @container flex h-full flex-col justify-center overflow-hidden">
                {title}
                <ol class=format!("grid grid-cols-1 gap-4 {}", cols_xl(n))>
                    {props.items.into_iter().enumerate().map(|(i, s)| view! {
                        <li class="mk-item mk-box min-w-0 rounded-box border border-base-300 bg-base-100 p-5">
                            <p class="mk-h text-5xl font-extrabold opacity-25">{format!("{:02}", i + 1)}</p>
                            <p class="mk-sub mk-h mt-2 font-semibold">{s.title}</p>
                            <p class="text-sm opacity-70">{s.text}</p>
                        </li>
                    }).collect_view()}
                </ol>
            </div>
        }
        .into_any(),
        _ => view! {
            <div class="mk-box mk-steps @container flex h-full flex-col justify-center overflow-hidden rounded-box bg-base-100 p-6 shadow-sm">
                {title}
                <ol class=format!("grid grid-cols-1 gap-6 {}", cols_2xl(n))>
                    {props.items.into_iter().enumerate().map(|(i, s)| view! {
                        <li class="flex min-w-0 flex-col items-center gap-2 text-center">
                            <span class="mk-icon mk-h flex h-10 w-10 items-center justify-center rounded-full bg-primary font-bold text-primary-content">{i + 1}</span>
                            <p class="mk-sub mk-h font-semibold">{s.title}</p>
                            <p class="text-sm opacity-70">{s.text}</p>
                        </li>
                    }).collect_view()}
                </ol>
            </div>
        }
        .into_any(),
    }
}

#[component]
pub fn ProductCarouselWidget(
    props: ProductCarouselProps,
    products: Vec<Product>,
    editing: bool,
) -> impl IntoView {
    let variant = props.variant.clone();
    let limit = usize::try_from(props.limit).unwrap_or(0).max(1);
    let mut products = products;
    // Los agotados van al final.
    products.sort_by_key(|p| !p.in_stock);
    products.truncate(limit);
    let total = products.len();
    let track = NodeRef::<Div>::new();
    let at_start = RwSignal::new(true);
    let at_end = RwSignal::new(false);
    let measure = move || {
        if let Some(el) = track.get_untracked() {
            let (left, view_w, all) = (el.scroll_left(), el.client_width(), el.scroll_width());
            at_start.set(left <= 1);
            at_end.set(left + view_w >= all - 1);
        }
    };
    Effect::new(move |_| measure());
    let go = move |dir: i32| {
        if let Some(el) = track.get_untracked() {
            let step = f64::from(el.client_width()) * 0.8;
            el.scroll_by_with_x_and_y(f64::from(dir) * step, 0.0);
        }
    };
    let title = props.title.clone();
    let link = (!props.link_label.is_empty()).then(|| {
        view! {
            <MaybeLink href=props.link_href.clone() editing class="mk-link link link-primary shrink-0 text-sm">{props.link_label.clone()}</MaybeLink>
        }
    });
    let arrows = move |class: &'static str| {
        view! {
            <div class=class>
                <button type="button" class="btn btn-circle btn-sm" aria-label="Ver los anteriores"
                    disabled=move || at_start.get() on:click=move |_| go(-1)>
                    <Icon name="chevron-left" class="h-4 w-4"/>
                </button>
                <button type="button" class="btn btn-circle btn-sm btn-primary mk-btn" aria-label="Ver los siguientes"
                    disabled=move || at_end.get() on:click=move |_| go(1)>
                    <Icon name="chevron-right" class="h-4 w-4"/>
                </button>
            </div>
        }
    };
    let rail = {
        let title = title.clone();
        view! {
            <div class="relative min-h-0 flex-1">
                <div node_ref=track
                    class="mk-rail carousel h-full w-full gap-4 [scrollbar-width:none] [&::-webkit-scrollbar]:hidden"
                    tabindex="0" role="region" aria-roledescription="carousel" aria-label=title
                    on:scroll=move |_| measure()>
                    {products.into_iter().enumerate().map(|(i, p)| view! {
                        <div class="carousel-item h-full w-[clamp(9.5rem,24cqw,15rem)]"
                            role="group" aria-roledescription="slide" aria-label=format!("{} de {total}", i + 1)>
                            <div class="h-full w-full"><ProductCard product=p show_price=true/></div>
                        </div>
                    }).collect_view()}
                </div>
                {(total == 0).then(|| view! { <div class="alert">"Todavía no hay productos."</div> })}
                {(variant == "b").then(|| view! {
                    <div class="pointer-events-none absolute inset-x-1 top-1/2 flex -translate-y-1/2 justify-between">
                        <button type="button" class="btn btn-circle pointer-events-auto shadow" aria-label="Ver los anteriores"
                            disabled=move || at_start.get() on:click=move |_| go(-1)>
                            <Icon name="chevron-left" class="h-5 w-5"/>
                        </button>
                        <button type="button" class="btn btn-circle pointer-events-auto shadow" aria-label="Ver los siguientes"
                            disabled=move || at_end.get() on:click=move |_| go(1)>
                            <Icon name="chevron-right" class="h-5 w-5"/>
                        </button>
                    </div>
                })}
            </div>
        }
    };
    match variant.as_str() {
        "b" => view! {
            <div class="mk-box mk-pad mk-pcar @container flex h-full flex-col gap-3 overflow-hidden rounded-box bg-base-100 p-5 shadow-sm">
                <div class="flex items-end justify-between gap-3">
                    <h2 class="mk-title mk-h truncate text-xl font-bold">{title}</h2>
                    {link}
                </div>
                {rail}
            </div>
        }
        .into_any(),
        "c" => view! {
            <div class="mk-box mk-pad mk-pcar @container flex h-full flex-col gap-3 overflow-hidden rounded-box">
                <div class="flex items-end justify-between gap-3">
                    <h2 class="mk-title mk-h truncate text-xl font-bold">{title}</h2>
                    {link}
                </div>
                {rail}
                {arrows("flex justify-center gap-2")}
            </div>
        }
        .into_any(),
        _ => view! {
            <div class="mk-box mk-pad mk-pcar @container flex h-full flex-col gap-3 overflow-hidden rounded-box bg-base-100 p-5 shadow-sm">
                <div class="flex items-center justify-between gap-3">
                    <h2 class="mk-title mk-h truncate text-xl font-bold">{title}</h2>
                    <div class="flex items-center gap-4">
                        {link}
                        {arrows("flex gap-2")}
                    </div>
                </div>
                {rail}
            </div>
        }
        .into_any(),
    }
}

#[component]
pub fn SocialWidget(props: SocialProps, editing: bool) -> impl IntoView {
    let store = use_context::<domain::StoreInfo>().unwrap_or_default();
    let mut channels = store_channels(&store);
    // WhatsApp va aparte: es el botón flotante.
    let whatsapp = channels
        .iter()
        .position(|c| c.icon == "whatsapp")
        .map(|i| channels.remove(i));
    channels.retain(|c| c.icon == "instagram");
    if !props.facebook.trim().is_empty() {
        channels.push(Channel {
            icon: "facebook",
            label: "Facebook",
            value: props.facebook.trim().trim_start_matches("https://").trim_start_matches("www.").to_string(),
            href: props.facebook.trim().to_string(),
            external: true,
        });
    }
    if !props.tiktok.trim().is_empty() {
        channels.push(Channel {
            icon: "tiktok",
            label: "TikTok",
            value: props.tiktok.trim().trim_start_matches("https://").trim_start_matches("www.").to_string(),
            href: props.tiktok.trim().to_string(),
            external: true,
        });
    }
    let float = |circle: bool| {
        let label = props.label.clone();
        whatsapp.as_ref().map(|w| {
            let class = if circle {
                "mk-wa-float btn btn-circle btn-lg border-0 text-white shadow-lg"
            } else {
                "mk-wa-float btn rounded-full border-0 px-5 text-white shadow-lg"
            };
            let pos = if editing { "absolute bottom-2 right-2" } else { "fixed bottom-6 right-4 z-[45]" };
            view! {
                <ChannelLink href=w.href.clone() external=true editing class=format!("{class} {pos}") label="Escribinos por WhatsApp">
                    <Icon name="whatsapp" class="h-7 w-7"/>
                    {(!circle).then(|| view! { <span>{label.clone()}</span> })}
                </ChannelLink>
            }
        })
    };
    let hint = (editing && whatsapp.is_none() && props.variant != "c").then(|| view! {
        <p class="text-sm opacity-70">"Cargá tu WhatsApp en «Datos de la tienda» y acá aparece el botón."</p>
    });
    match props.variant.as_str() {
        "b" => view! {
            <div class="mk-box mk-pad mk-social relative h-full overflow-hidden rounded-box">
                {hint}
                {float(false)}
            </div>
        }
        .into_any(),
        "c" => view! {
            <div class="mk-social @container grid h-full grid-cols-1 content-center gap-3 overflow-hidden @xl:grid-cols-3">
                {channels.into_iter().chain(whatsapp).map(|c| view! {
                    <ChannelLink href=c.href external=c.external editing class="mk-item mk-box flex min-h-14 min-w-0 items-center gap-3 rounded-box border border-base-300 bg-base-100 p-3">
                        <span class="mk-icon flex h-10 w-10 shrink-0 items-center justify-center rounded-full">
                            <Icon name=c.icon class="h-5 w-5"/>
                        </span>
                        <span class="min-w-0 leading-tight">
                            <span class="block font-semibold">{c.label}</span>
                            <span class="block truncate text-sm opacity-70">{c.value}</span>
                        </span>
                    </ChannelLink>
                }).collect_view()}
            </div>
        }
        .into_any(),
        _ => view! {
            <div class="mk-box mk-pad mk-social relative flex h-full items-center gap-3 overflow-hidden rounded-box">
                {(!props.title.is_empty()).then(|| view! { <span class="mk-title mk-h text-sm font-semibold">{props.title.clone()}</span> })}
                {channels.into_iter().map(|c| view! {
                    <ChannelLink href=c.href external=c.external editing class="mk-btn btn btn-circle btn-outline btn-sm" label=c.label>
                        <Icon name=c.icon class="h-4 w-4"/>
                    </ChannelLink>
                }).collect_view()}
                {hint}
                {float(true)}
            </div>
        }
        .into_any(),
    }
}

/// Tiempo que falta hasta `ends_at`, en segundos. Usa la hora local del navegador.
#[cfg(target_arch = "wasm32")]
fn seconds_left(ends_at: &str) -> Option<i64> {
    let end = js_sys::Date::new(&wasm_bindgen::JsValue::from_str(ends_at)).get_time();
    if end.is_nan() {
        return None;
    }
    Some(((end - js_sys::Date::now()) / 1000.0).max(0.0) as i64)
}

#[cfg(not(target_arch = "wasm32"))]
fn seconds_left(_ends_at: &str) -> Option<i64> {
    None
}

/// Separa segundos en días, horas, minutos y segundos.
pub fn split_seconds(total: i64) -> [i64; 4] {
    let t = total.max(0);
    [t / 86_400, t % 86_400 / 3600, t % 3600 / 60, t % 60]
}

#[component]
pub fn CountdownWidget(props: CountdownProps, editing: bool) -> impl IntoView {
    // `None` hasta que corre en el navegador: el servidor y la hidratación
    // dibujan el skeleton, y después el reloj.
    let left = RwSignal::new(None::<i64>);
    let ends_at = props.ends_at.clone();
    Effect::new(move |_| {
        left.set(seconds_left(&ends_at));
        let ends_at = ends_at.clone();
        let handle = set_interval_with_handle(
            move || left.set(seconds_left(&ends_at)),
            std::time::Duration::from_secs(1),
        );
        on_cleanup(move || {
            if let Ok(h) = handle {
                h.clear();
            }
        });
    });
    let has_button = !props.button_label.is_empty();
    let ended_text = props.ended_text.clone();
    let units = ["días", "horas", "min", "seg"];

    view! {
        <div class="mk-box mk-countdown @container flex h-full flex-col items-center justify-center gap-3 overflow-hidden rounded-box p-4 text-center">
            <h2 class="mk-title text-2xl font-bold line-clamp-1 @xl:text-3xl">{props.title.clone()}</h2>
            {(!props.subtitle.is_empty()).then(|| view! {
                <p class="hidden line-clamp-2 opacity-90 @xl:block">{props.subtitle.clone()}</p>
            })}
            {move || match left.get() {
                None => view! {
                    <div class="flex gap-3">
                        {(0..4).map(|_| view! { <Skeleton class="h-16 w-16 bg-current/20"/> }).collect_view()}
                    </div>
                }
                .into_any(),
                Some(0) => view! { <p class="text-xl font-semibold">{ended_text.clone()}</p> }.into_any(),
                Some(s) => {
                    let parts = split_seconds(s);
                    view! {
                        <div class="grid auto-cols-max grid-flow-col gap-3">
                            {parts
                                .iter()
                                .zip(units)
                                .map(|(v, unit)| {
                                    let style = format!("--value:{v}; --digits:2;");
                                    view! {
                                        <div class="mk-unit flex flex-col items-center rounded-box px-3 py-2">
                                            <span class="countdown font-mono text-3xl">
                                                <span style=style aria-live=(!editing).then_some("polite") aria-label=v.to_string()>{v.to_string()}</span>
                                            </span>
                                            <span class="text-xs">{unit}</span>
                                        </div>
                                    }
                                })
                                .collect_view()}
                        </div>
                    }
                    .into_any()
                }
            }}
            {has_button.then(|| view! {
                <a
                    class="mk-btn btn btn-sm"
                    href=(!editing).then(|| props.button_href.clone())
                    data-href=(editing && !props.button_href.is_empty()).then(|| props.button_href.clone())
                >
                    {props.button_label.clone()}
                </a>
            })}
        </div>
    }
}

#[component]
pub fn CardWidget(props: CardProps, editing: bool) -> impl IntoView {
    let (card_class, figure_class) = match props.layout.as_str() {
        "side" => (
            "mk-box mk-card card card-side card-sm h-full overflow-hidden border border-base-300 bg-base-100 shadow-sm",
            "relative w-2/5 shrink-0",
        ),
        "overlay" => (
            "mk-box mk-card mk-card-overlay card image-full card-sm h-full overflow-hidden shadow-sm",
            "relative",
        ),
        _ => (
            "mk-box mk-card card card-sm h-full overflow-hidden border border-base-300 bg-base-100 shadow-sm",
            "relative min-h-0 flex-1",
        ),
    };
    let has_image = !props.image_url.is_empty();
    let has_button = !props.button_label.is_empty();
    let body_class = if props.layout == "vertical" {
        "card-body grow-0 shrink-0"
    } else {
        "card-body min-w-0 justify-center"
    };
    view! {
        <div class=card_class>
            {has_image.then(|| view! {
                <figure class=figure_class>
                    <div class="absolute inset-0">
                        <Image src=props.image_url.clone() alt=props.title.clone()/>
                    </div>
                </figure>
            })}
            <div class=body_class>
                <h2 class="mk-title card-title line-clamp-2">{props.title.clone()}</h2>
                {(!props.body.is_empty()).then(|| view! {
                    <p class="line-clamp-3 grow-0">{props.body.clone()}</p>
                })}
                {has_button.then(|| view! {
                    <div class="card-actions">
                        <Button label=props.button_label.clone() href=props.button_href.clone() disabled=editing/>
                    </div>
                })}
            </div>
        </div>
    }
}

#[component]
pub fn TabsWidget(props: TabsProps, group: String) -> impl IntoView {
    view! {
        <div class="mk-box mk-tabs tabs tabs-box h-full content-start overflow-hidden bg-base-200">
            {props
                .tabs
                .into_iter()
                .enumerate()
                .map(|(i, t)| view! {
                    <input type="radio" name=group.clone() class="tab" aria-label=t.label checked=i == 0/>
                    <div class="mk-item tab-content h-[calc(100%-3rem)] overflow-auto rounded-box border-base-300 bg-base-100 p-4">
                        <p class="whitespace-pre-line">{t.body}</p>
                    </div>
                })
                .collect_view()}
        </div>
    }
}

#[component]
pub fn FaqWidget(props: FaqProps) -> impl IntoView {
    view! {
        <div class="mk-box mk-pad mk-faq flex h-full flex-col gap-3 overflow-hidden rounded-box">
            {(!props.title.is_empty()).then(|| view! {
                <h2 class="mk-title text-xl font-bold truncate">{props.title.clone()}</h2>
            })}
            <div class="join join-vertical min-h-0 w-full flex-1 overflow-y-auto">
                {props
                    .items
                    .into_iter()
                    .map(|q| view! {
                        <details class="mk-item collapse collapse-arrow join-item border border-base-300 bg-base-100">
                            <summary class="collapse-title font-semibold">{q.question}</summary>
                            <div class="collapse-content text-sm">
                                <p class="whitespace-pre-line">{q.answer}</p>
                            </div>
                        </details>
                    })
                    .collect_view()}
            </div>
        </div>
    }
}

#[component]
pub fn HoverGalleryWidget(props: HoverGalleryProps, editing: bool) -> impl IntoView {
    let title = props.title.clone();
    view! {
        <MaybeLink href=props.href.clone() editing class="mk-box mk-hgal card card-sm h-full overflow-hidden border border-base-300 bg-base-100 shadow-sm">
            <div class="skeleton relative min-h-0 flex-1 rounded-none">
                <figure class="hover-gallery absolute inset-0 h-full">
                    // Las fotos que aparecen al pasar el mouse son variantes de
                    // la primera: solo la primera se anuncia.
                    {props
                        .images
                        .into_iter()
                        .enumerate()
                        .map(|(i, img)| {
                            let alt = if i == 0 { title.clone() } else { String::new() };
                            view! { <Picture src=img.image_url alt class="block h-full w-full object-cover"/> }
                        })
                        .collect_view()}
                </figure>
            </div>
            <div class="card-body grow-0 shrink-0 gap-0">
                <h3 class="mk-sub card-title text-sm truncate block">{props.title.clone()}</h3>
                {(!props.subtitle.is_empty()).then(|| view! {
                    <p class="text-xs opacity-70 truncate">{props.subtitle.clone()}</p>
                })}
            </div>
        </MaybeLink>
    }
}

#[component]
fn Stars(rating: i64) -> impl IntoView {
    let label = format!("{rating} de 5 estrellas");
    view! {
        <div class="rating rating-xs" role="img" aria-label=label>
            {(1..=5)
                .map(|i| view! {
                    <div class="mask mask-star-2 bg-warning" aria-current=(i == rating).then_some("true")></div>
                })
                .collect_view()}
        </div>
    }
}

#[component]
pub fn TestimonialsWidget(props: TestimonialsProps) -> impl IntoView {
    view! {
        <div class="mk-box mk-pad mk-testi @container flex h-full flex-col gap-3 overflow-hidden rounded-box">
            {(!props.title.is_empty()).then(|| view! {
                <h2 class="mk-title text-xl font-bold truncate">{props.title.clone()}</h2>
            })}
            <div class="grid min-h-0 flex-1 auto-cols-[minmax(16rem,1fr)] grid-flow-col gap-3 overflow-x-auto">
                {props
                    .items
                    .into_iter()
                    .map(|t| {
                        let initial = t.name.chars().next().map(String::from).unwrap_or_default();
                        view! {
                            <div class="mk-item card card-sm min-h-0 overflow-hidden border border-base-300 bg-base-100">
                                <div class="card-body">
                                    <Stars rating=t.rating/>
                                    <p class="line-clamp-4 italic">{format!("“{}”", t.text)}</p>
                                    <div class="flex items-center gap-2">
                                        <div class="avatar avatar-placeholder">
                                            <div class="mk-avatar w-8 rounded-full">
                                                <span class="text-xs">{initial}</span>
                                            </div>
                                        </div>
                                        <span class="text-sm font-semibold truncate">{t.name}</span>
                                    </div>
                                </div>
                            </div>
                        }
                    })
                    .collect_view()}
            </div>
        </div>
    }
}

#[component]
pub fn LogosWidget(props: LogosProps, editing: bool) -> impl IntoView {
    view! {
        <div class="mk-box mk-logos flex h-full flex-col justify-center gap-2 overflow-hidden rounded-box bg-base-100 px-4 py-3">
            {(!props.title.is_empty()).then(|| view! {
                <h2 class="mk-sub text-center text-xs font-semibold uppercase tracking-wide opacity-75 truncate">{props.title.clone()}</h2>
            })}
            <div class="flex items-center justify-center-safe gap-8 overflow-x-auto">
                {props
                    .items
                    .into_iter()
                    .map(|l| view! {
                        <MaybeLink href=l.href editing class="mk-logo block shrink-0">
                            <img
                                class="h-8 w-auto max-w-40 shrink-0 object-contain opacity-70 grayscale transition hover:opacity-100 hover:grayscale-0"
                                src=l.image_url
                                alt=l.alt
                                draggable="false"
                                loading="lazy"
                                decoding="async"
                            />
                        </MaybeLink>
                    })
                    .collect_view()}
            </div>
        </div>
    }
}

#[component]
pub fn ImageWidget(props: ImageProps, editing: bool) -> impl IntoView {
    view! {
        <MaybeLink href=props.href.clone() editing class="mk-box mk-image block h-full overflow-hidden rounded-box">
            <Image src=props.image_url.clone() alt=props.alt.clone() contain=props.fit == "contain"/>
        </MaybeLink>
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
    let whatsapp = (channels.first().is_some_and(|c| c.icon == "whatsapp")).then(|| channels.remove(0));
    let has_channels = whatsapp.is_some() || !channels.is_empty();
    let others_title = if whatsapp.is_some() { "También podés" } else { "Escribinos" };
    let hours = props.hours.trim().to_string();
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

#[component]
pub fn FooterWidget(props: FooterProps, editing: bool) -> impl IntoView {
    let store = use_context::<domain::StoreInfo>().unwrap_or_default();
    let brand = or_store(&props.brand, &store.name);
    let legal = if props.legal.is_empty() && !store.name.is_empty() {
        format!("© {}", store.name)
    } else {
        props.legal.clone()
    };
    // Abajo, el texto legal y la frase de la tienda en una sola línea.
    let bottom: Vec<String> = [legal, props.description.trim().to_string()]
        .into_iter()
        .filter(|t| !t.is_empty())
        .collect();
    let bottom = bottom.join(" · ");
    let logo = store.logo_url.clone();
    let channels = store_channels(&store);
    let has_links = !props.links.is_empty();
    view! {
        <footer class="mk-box mk-footer @container flex h-full flex-col justify-center gap-4 overflow-hidden rounded-box px-6 py-5">
            <div class="mk-foot-row grid grid-cols-[minmax(0,1fr)_auto] items-center gap-x-8 gap-y-3 @3xl:grid-cols-[auto_minmax(0,1fr)_auto]">
                <div class="flex min-w-0 items-center gap-3">
                    {(!logo.is_empty()).then(|| view! {
                        <img src=logo.clone() alt="" class="h-9 w-9 shrink-0 rounded-full object-cover"/>
                    })}
                    <p class="mk-brand mk-h truncate text-lg font-bold">{brand}</p>
                </div>
                {has_links.then(|| view! {
                    <nav aria-label="Pie de página" class="col-span-2 row-start-2 flex flex-wrap gap-x-6 gap-y-1 text-sm @3xl:col-span-1 @3xl:col-start-2 @3xl:row-start-1">
                        {props
                            .links
                            .into_iter()
                            .map(|l| view! {
                                <MaybeLink href=l.href editing class="mk-link link link-hover">{l.label}</MaybeLink>
                            })
                            .collect_view()}
                    </nav>
                })}
                {(!channels.is_empty()).then(|| view! {
                    <div class="flex gap-2 justify-self-end @3xl:col-start-3 @3xl:row-start-1">
                        {channels
                            .into_iter()
                            .map(|c| view! {
                                <ChannelLink href=c.href external=c.external editing label=c.label class="mk-social grid size-10 place-items-center rounded-full border border-current/25 pointer-coarse:size-11">
                                    <Icon name=c.icon class="h-[18px] w-[18px]"/>
                                </ChannelLink>
                            })
                            .collect_view()}
                    </div>
                })}
            </div>
            {(!bottom.is_empty()).then(|| view! {
                <p class="border-t border-current/20 pt-3 text-xs opacity-70">{bottom}</p>
            })}
        </footer>
    }
}

/// El valor del bloque, o el de la tienda si el bloque lo dejó vacío.
fn or_store(own: &str, store: &str) -> String {
    if own.trim().is_empty() { store } else { own }.to_string()
}

#[cfg(test)]
mod tests {
    use super::split_seconds;

    #[test]
    fn splits_seconds_into_units() {
        assert_eq!(split_seconds(0), [0, 0, 0, 0]);
        assert_eq!(split_seconds(90_061), [1, 1, 1, 1]);
        assert_eq!(split_seconds(-5), [0, 0, 0, 0]);
    }
}
