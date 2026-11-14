//! Render de cada widget. Un componente por variante de [`Widget`].
//!
//! Los widgets usan solo colores semánticos de daisyUI y se adaptan al alto
//! fijo de la opción de tamaño elegida. Los textos se muestran siempre como
//! texto, nunca como HTML.

use domain::{
    BannerProps, FeaturedProductProps, Node, Product, ProductGridProps, TextProps, Widget,
};
use leptos::prelude::*;
use ui::{Button, Card, Image, Price};

/// Contexto que recibe todo widget al dibujarse.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct RenderCtx {
    pub products: Vec<Product>,
    /// En el editor: desactiva enlaces y botones del widget.
    pub editing: bool,
}

pub fn render_widget(widget: &Widget, ctx: &RenderCtx) -> AnyView {
    match widget {
        Widget::Banner(p) => {
            view! { <BannerWidget props=p.clone() editing=ctx.editing/> }.into_any()
        }
        Widget::Text(p) => view! { <TextWidget props=p.clone()/> }.into_any(),
        Widget::ProductGrid(p) => {
            view! { <ProductGridWidget props=p.clone() products=ctx.products.clone()/> }.into_any()
        }
        Widget::FeaturedProduct(p) => {
            view! { <FeaturedProductWidget props=p.clone() products=ctx.products.clone()/> }
                .into_any()
        }
    }
}

/// Variables CSS de posición de un nodo para `.mk-node`: `--x:1; --y:5; --w:12; --h:5`.
/// `--x` y `--y` suman 1 a `pos` porque las líneas de la grilla CSS empiezan en 1.
pub fn node_style(node: &Node) -> String {
    let preset = node.preset().unwrap_or(&node.widget.sizes()[0]);
    format!(
        "--x:{}; --y:{}; --w:{}; --h:{}",
        u32::from(node.pos.x) + 1,
        u32::from(node.pos.y) + 1,
        preset.w,
        preset.h
    )
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
        <div class="hero relative h-full overflow-hidden rounded-box bg-base-200">
            {has_image.then(|| view! {
                <div class="absolute inset-0">
                    <Image src=props.image_url.clone() alt=props.title.clone()/>
                </div>
                <div class="hero-overlay absolute inset-0"></div>
            })}
            <div class=content_class>
                <div class="max-w-xl">
                    <h1 class="text-3xl font-bold line-clamp-2">{props.title.clone()}</h1>
                    {(!props.subtitle.is_empty()).then(|| view! {
                        <p class="py-3 line-clamp-2">{props.subtitle.clone()}</p>
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

/// Tarjeta de producto que reparte el alto entre la imagen y el texto.
#[component]
fn ProductCard(product: Product, show_price: bool) -> impl IntoView {
    view! {
        <Card>
            <figure class="min-h-0 flex-1">
                <Image src=product.image_url.clone() alt=product.name.clone()/>
            </figure>
            <div class="card-body shrink-0 gap-0">
                <h3 class="card-title text-sm truncate block">{product.name.clone()}</h3>
                {show_price.then(|| view! { <Price cents=product.price_cents/> })}
            </div>
        </Card>
    }
}
