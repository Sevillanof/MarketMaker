//! Render de cada widget. Un componente por variante de [`Widget`].
//!
//! Los widgets usan solo colores semánticos de daisyUI y se adaptan al ancho
//! y alto que les toca en la grilla (con container queries `@container`).
//! Los textos se muestran siempre como texto, nunca como HTML.

use domain::{
    BannerProps, BenefitsProps, CardProps, CarouselProps, CategoriesProps, CountdownProps,
    FaqProps, FeaturedProductProps, FooterProps, HoverGalleryProps, ImageProps, LogosProps,
    NavbarProps, Node, Product, ProductButtonProps, ProductGridProps, ProductImageProps,
    ProductModal, TabsProps, TestimonialsProps, TextProps, Widget,
};
use leptos::html::Div;
use leptos::prelude::*;
use ui::{Button, Card, Icon, Image, Picture, Price, Skeleton};

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
    }
}

/// Producto abierto en la ficha de la tienda, por contexto. Si está, las
/// tarjetas de producto se pueden tocar para abrir su ficha.
#[derive(Clone, Copy)]
pub struct OpenProduct(pub RwSignal<Option<String>>);

/// Dibuja un nodo dentro de su envoltorio de letras (`.mk-type`, ver
/// [`domain::TextStyle`] y `style/tailwind.css`).
pub fn render_widget(node: &Node, ctx: &RenderCtx) -> AnyView {
    let class = format!("{} h-full", node.style.classes());
    let style = node.style.css_vars();
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
        Widget::FeaturedProduct(p) => {
            view! { <FeaturedProductWidget props=p.clone() products=ctx.products.clone()/> }
                .into_any()
        }
        Widget::Navbar(p) => view! { <NavbarWidget props=p.clone() editing/> }.into_any(),
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
                <div class="flex h-full items-start overflow-hidden">
                    <h2 class="text-2xl font-bold leading-tight line-clamp-2">{name}</h2>
                </div>
            }
            .into_any()
        }
        Widget::ProductPrice(_) => {
            let cents = ctx.product.as_ref().map_or(0, |p| p.price_cents);
            view! {
                <div class="flex h-full items-start overflow-hidden text-xl text-primary">
                    <Price cents/>
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
                <div class="h-full overflow-y-auto">
                    {if empty && editing {
                        view! { <p class="text-base-content/60 italic">"Este producto todavía no tiene descripción."</p> }.into_any()
                    } else {
                        view! { <p class="whitespace-pre-line text-base-content/80">{text}</p> }.into_any()
                    }}
                </div>
            }
            .into_any()
        }
        Widget::ProductButton(p) => {
            view! { <ProductButtonWidget props=p.clone() editing/> }.into_any()
        }
    }
}

#[component]
pub fn ProductImageWidget(props: ProductImageProps, product: Option<Product>) -> impl IntoView {
    let (src, alt) = product.map_or_else(Default::default, |p| (p.image_url, p.name));
    view! {
        <div class="h-full overflow-hidden rounded-box bg-base-200">
            <Image src alt contain=props.fit == "contain"/>
        </div>
    }
}

#[component]
pub fn ProductButtonWidget(props: ProductButtonProps, editing: bool) -> impl IntoView {
    view! {
        <div class="flex h-full items-start">
            <Button label=props.label href=props.href disabled=editing/>
        </div>
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
    let content_class = if has_image {
        "hero-content relative text-center text-neutral-content"
    } else {
        "hero-content relative text-center text-base-content"
    };
    view! {
        <div class="@container hero relative h-full overflow-hidden rounded-box bg-base-200">
            {has_image.then(|| view! {
                <div class="absolute inset-0">
                    <Image src=props.image_url.clone() alt=props.title.clone()/>
                </div>
                <div class="hero-overlay absolute inset-0"></div>
            })}
            <div class=content_class>
                <div class="max-w-xl">
                    <h1 class="text-xl font-bold line-clamp-2 @xl:text-3xl">{props.title.clone()}</h1>
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
        "center" => "h-full overflow-hidden rounded-box bg-base-100 p-4 text-center",
        "right" => "h-full overflow-hidden rounded-box bg-base-100 p-4 text-right",
        _ => "h-full overflow-hidden rounded-box bg-base-100 p-4 text-left",
    };
    view! {
        <div class=class>
            {(!props.heading.is_empty()).then(|| view! {
                <h2 class="text-xl font-bold mb-2 truncate">{props.heading.clone()}</h2>
            })}
            <p class="whitespace-pre-line">{props.body}</p>
        </div>
    }
}

#[component]
pub fn ProductGridWidget(props: ProductGridProps, products: Vec<Product>) -> impl IntoView {
    let limit = usize::try_from(props.limit).unwrap_or(0);
    let shown: Vec<Product> = products.into_iter().take(limit).collect();
    let empty = shown.is_empty();
    view! {
        <div class="@container flex h-full flex-col gap-3 overflow-hidden">
            {(!props.title.is_empty()).then(|| view! {
                <h2 class="text-xl font-bold truncate">{props.title.clone()}</h2>
            })}
            {empty.then(|| view! {
                <div class="alert">"Todavía no hay productos."</div>
            })}
            <div class="grid min-h-0 flex-1 auto-rows-fr grid-cols-2 gap-3 @lg:grid-cols-3 @3xl:grid-cols-4 @5xl:grid-cols-6">
                {shown.into_iter().map(|p| view! { <ProductCard product=p show_price=true/> }).collect_view()}
            </div>
        </div>
    }
}

#[component]
pub fn FeaturedProductWidget(props: FeaturedProductProps, products: Vec<Product>) -> impl IntoView {
    let product = products.into_iter().find(|p| p.id == props.product_id);
    match product {
        Some(p) => view! { <ProductCard product=p show_price=props.show_price/> }.into_any(),
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
fn ProductCard(product: Product, show_price: bool) -> impl IntoView {
    let open = use_context::<OpenProduct>();
    let id = product.id.clone();
    let name = product.name.clone();
    view! {
        <div class="relative h-full" data-product-id=product.id.clone()>
        <Card>
            <div class="relative min-h-0 flex-1">
                <div class="absolute inset-0">
                    <Image src=product.image_url.clone() alt=product.name.clone()/>
                </div>
            </div>
            <div class="card-body grow-0 shrink-0 gap-0">
                <h3 class="card-title text-sm truncate block">{product.name.clone()}</h3>
                {show_price.then(|| view! { <Price cents=product.price_cents/> })}
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

#[component]
pub fn NavbarWidget(props: NavbarProps, editing: bool) -> impl IntoView {
    let has_button = !props.button_label.is_empty();
    view! {
        <nav class="navbar h-full min-h-0 gap-4 rounded-box bg-base-100 px-4 shadow-sm">
            <MaybeLink href="/" editing class="flex shrink-0 items-center gap-2">
                {(!props.logo_url.is_empty()).then(|| view! {
                    <img class="h-8 w-8 object-contain" src=props.logo_url.clone() alt="" draggable="false"/>
                })}
                <span class="mk-h text-lg font-bold whitespace-nowrap">{props.brand.clone()}</span>
            </MaybeLink>
            <ul class="menu menu-horizontal min-w-0 flex-1 flex-nowrap justify-end overflow-x-auto px-0">
                {props
                    .links
                    .into_iter()
                    .map(|l| view! {
                        <li>
                            <MaybeLink href=l.href editing class="whitespace-nowrap">{l.label}</MaybeLink>
                        </li>
                    })
                    .collect_view()}
            </ul>
            {has_button.then(|| view! {
                <Button label=props.button_label.clone() href=props.button_href.clone() disabled=editing/>
            })}
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
        <div class="@container relative h-full overflow-hidden rounded-box bg-base-200">
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
                                <div class="absolute inset-0 bg-gradient-to-r from-black/60 via-black/25 to-transparent"></div>
                                <div class="absolute inset-y-0 left-0 flex max-w-lg flex-col justify-center gap-2 px-14 py-6 text-white @2xl:gap-3">
                                    <h2 class="text-xl font-bold line-clamp-2 @2xl:text-3xl">{s.title.clone()}</h2>
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
                    class="btn btn-circle btn-sm absolute left-3 top-1/2 -translate-y-1/2 border-0 bg-base-100/80"
                    aria-label="Anterior"
                    on:click=move |_| go((current.get_untracked() + count - 1) % count)
                >
                    "❮"
                </button>
                <button
                    type="button"
                    class="btn btn-circle btn-sm absolute right-3 top-1/2 -translate-y-1/2 border-0 bg-base-100/80"
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
                                class="h-2 rounded-full bg-white transition-all"
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
    view! {
        <div class="@container flex h-full flex-col gap-3 overflow-hidden">
            {(!props.title.is_empty()).then(|| view! {
                <h2 class="text-xl font-bold truncate">{props.title.clone()}</h2>
            })}
            <div class="grid min-h-0 flex-1 auto-rows-fr grid-cols-2 gap-3 @xl:grid-cols-3 @3xl:grid-cols-4 @5xl:grid-cols-6">
                {props
                    .items
                    .into_iter()
                    .map(|c| view! {
                        <MaybeLink href=c.href editing class="group relative block min-h-0 overflow-hidden rounded-box">
                            <div class="absolute inset-0 transition-transform duration-300 group-hover:scale-105">
                                <Image src=c.image_url.clone() alt=c.label.clone()/>
                            </div>
                            <div class="absolute inset-x-0 bottom-0 bg-gradient-to-t from-black/70 to-transparent p-3">
                                <span class="mk-h font-semibold text-white">{c.label.clone()}</span>
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
        <div class="h-full overflow-hidden rounded-box bg-base-100 px-4 py-2">
            <div class="grid h-full auto-cols-[minmax(13rem,1fr)] grid-flow-col items-center gap-4 overflow-x-auto">
                {props
                    .items
                    .into_iter()
                    .map(|b| view! {
                        <div class="flex min-w-0 items-center gap-3">
                            <div class="flex h-12 w-12 shrink-0 items-center justify-center rounded-full bg-primary/10 text-primary">
                                <Icon name=b.icon class="h-6 w-6"/>
                            </div>
                            <div class="min-w-0">
                                <p class="mk-h font-semibold truncate">{b.title}</p>
                                <p class="text-sm opacity-70 line-clamp-2">{b.text}</p>
                            </div>
                        </div>
                    })
                    .collect_view()}
            </div>
        </div>
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
        <div class="@container flex h-full flex-col items-center justify-center gap-3 overflow-hidden rounded-box bg-primary p-4 text-center text-primary-content">
            <h2 class="text-2xl font-bold line-clamp-1 @xl:text-3xl">{props.title.clone()}</h2>
            {(!props.subtitle.is_empty()).then(|| view! {
                <p class="hidden line-clamp-2 opacity-90 @xl:block">{props.subtitle.clone()}</p>
            })}
            {move || match left.get() {
                None => view! {
                    <div class="flex gap-3">
                        {(0..4).map(|_| view! { <Skeleton class="h-16 w-16 bg-primary-content/20"/> }).collect_view()}
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
                                        <div class="flex flex-col items-center rounded-box bg-primary-content/15 px-3 py-2">
                                            <span class="countdown font-mono text-3xl">
                                                <span style=style aria-live="polite" aria-label=v.to_string()>{v.to_string()}</span>
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
                    class="btn btn-sm bg-primary-content text-primary border-0"
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
            "card card-side card-sm h-full overflow-hidden border border-base-300 bg-base-100 shadow-sm",
            "relative w-2/5 shrink-0",
        ),
        "overlay" => (
            "card image-full card-sm h-full overflow-hidden shadow-sm",
            "relative",
        ),
        _ => (
            "card card-sm h-full overflow-hidden border border-base-300 bg-base-100 shadow-sm",
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
                <h2 class="card-title line-clamp-2">{props.title.clone()}</h2>
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
        <div class="tabs tabs-box h-full content-start overflow-hidden bg-base-200">
            {props
                .tabs
                .into_iter()
                .enumerate()
                .map(|(i, t)| view! {
                    <input type="radio" name=group.clone() class="tab" aria-label=t.label checked=i == 0/>
                    <div class="tab-content h-[calc(100%-3rem)] overflow-auto rounded-box border-base-300 bg-base-100 p-4">
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
        <div class="flex h-full flex-col gap-3 overflow-hidden">
            {(!props.title.is_empty()).then(|| view! {
                <h2 class="text-xl font-bold truncate">{props.title.clone()}</h2>
            })}
            <div class="join join-vertical min-h-0 w-full flex-1 overflow-y-auto">
                {props
                    .items
                    .into_iter()
                    .map(|q| view! {
                        <details class="collapse collapse-arrow join-item border border-base-300 bg-base-100">
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
        <MaybeLink href=props.href.clone() editing class="card card-sm h-full overflow-hidden border border-base-300 bg-base-100 shadow-sm">
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
                <h3 class="card-title text-sm truncate block">{props.title.clone()}</h3>
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
        <div class="@container flex h-full flex-col gap-3 overflow-hidden">
            {(!props.title.is_empty()).then(|| view! {
                <h2 class="text-xl font-bold truncate">{props.title.clone()}</h2>
            })}
            <div class="grid min-h-0 flex-1 auto-cols-[minmax(16rem,1fr)] grid-flow-col gap-3 overflow-x-auto">
                {props
                    .items
                    .into_iter()
                    .map(|t| {
                        let initial = t.name.chars().next().map(String::from).unwrap_or_default();
                        view! {
                            <div class="card card-sm min-h-0 overflow-hidden border border-base-300 bg-base-100">
                                <div class="card-body">
                                    <Stars rating=t.rating/>
                                    <p class="line-clamp-4 italic">{format!("“{}”", t.text)}</p>
                                    <div class="flex items-center gap-2">
                                        <div class="avatar avatar-placeholder">
                                            <div class="w-8 rounded-full bg-neutral text-neutral-content">
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
        <div class="flex h-full flex-col justify-center gap-2 overflow-hidden rounded-box bg-base-100 px-4 py-3">
            {(!props.title.is_empty()).then(|| view! {
                <h2 class="text-center text-xs font-semibold uppercase tracking-wide opacity-60 truncate">{props.title.clone()}</h2>
            })}
            <div class="flex items-center justify-center-safe gap-8 overflow-x-auto">
                {props
                    .items
                    .into_iter()
                    .map(|l| view! {
                        <MaybeLink href=l.href editing class="block shrink-0">
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
        <MaybeLink href=props.href.clone() editing class="block h-full overflow-hidden rounded-box">
            <Image src=props.image_url.clone() alt=props.alt.clone() contain=props.fit == "contain"/>
        </MaybeLink>
    }
}

#[component]
pub fn FooterWidget(props: FooterProps, editing: bool) -> impl IntoView {
    view! {
        <footer class="@container flex h-full flex-col justify-between gap-3 overflow-hidden rounded-box bg-neutral px-6 py-4 text-neutral-content">
            <div class="flex flex-col gap-4 @2xl:flex-row @2xl:justify-between">
                <aside class="max-w-sm">
                    <p class="mk-h text-lg font-bold">{props.brand.clone()}</p>
                    <p class="text-sm opacity-80 line-clamp-2">{props.description.clone()}</p>
                </aside>
                <nav class="flex flex-wrap content-start gap-x-6 gap-y-1 text-sm @2xl:max-w-md @2xl:justify-end">
                    {props
                        .links
                        .into_iter()
                        .map(|l| view! {
                            <MaybeLink href=l.href editing class="link link-hover truncate">{l.label}</MaybeLink>
                        })
                        .collect_view()}
                </nav>
            </div>
            {(!props.legal.is_empty()).then(|| view! {
                <p class="border-t border-neutral-content/20 pt-3 text-xs opacity-70 truncate">{props.legal.clone()}</p>
            })}
        </footer>
    }
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
