//! Ruta de desarrollo `/dev/widgets`: cada widget en su tamaño mínimo, por
//! defecto y máximo.
//! Solo responde en compilaciones de desarrollo (`cargo leptos watch`); en
//! `--release` muestra "Página no encontrada".

use domain::{Node, Product, Size, Widget, WIDGET_TYPES};
use leptos::prelude::*;
use widgets::{node_style, render_widget, RenderCtx};

fn sample_products() -> Vec<Product> {
    let names = [
        "Auriculares inalámbricos",
        "Mochila urbana",
        "Lámpara de escritorio",
        "Taza de cerámica",
        "Zapatillas running",
        "Reloj analógico",
        "Cuaderno de tapa dura",
        "Parlante portátil",
    ];
    names
        .iter()
        .enumerate()
        .map(|(i, name)| Product {
            id: format!("p{i}"),
            name: (*name).to_string(),
            price_cents: 1_234_500 + i as i64 * 250_050,
            image_url: format!("/img/product-{}.webp", i + 1),
            description: format!("{name} de ejemplo, con envío a todo el país."),
        })
        .collect()
}

#[component]
pub fn DevWidgetsPage() -> impl IntoView {
    if !cfg!(debug_assertions) {
        return view! { "Página no encontrada." }.into_any();
    }
    let products = sample_products();
    let first_id = products[0].id.clone();
    let ctx = RenderCtx {
        products: products.clone(),
        editing: false,
        product: Some(products[0].clone()),
    };

    let cases: Vec<(String, Node)> = WIDGET_TYPES
        .iter()
        .flat_map(|(type_name, label)| {
            let mut widget = Widget::defaults(type_name).expect("tipo conocido");
            if let Widget::FeaturedProduct(p) = &mut widget {
                p.product_id = first_id.clone();
            }
            let l = widget.limits();
            [
                (
                    "mínimo",
                    Size {
                        w: l.min_w,
                        h: l.min_h,
                    },
                ),
                ("por defecto", l.default),
                (
                    "máximo",
                    Size {
                        w: l.max_w,
                        h: l.max_h,
                    },
                ),
            ]
            .into_iter()
            .map(|(name, size)| {
                let mut node = Node::new(format!("{type_name}-{name}"), widget.clone());
                node.size = size;
                (format!("{label} · {name} ({}×{})", size.w, size.h), node)
            })
            .collect::<Vec<_>>()
        })
        .collect();

    view! {
        <div data-theme="light" class="min-h-screen bg-base-200 p-6 text-base-content">
            <h1 class="text-2xl font-bold mb-6">"Widgets: " {cases.len()} " combinaciones"</h1>
            {cases
                .into_iter()
                .map(|(title, node)| {
                    let style = node_style(&node);
                    let content = render_widget(&node, &ctx);
                    view! {
                        <section class="mb-10">
                            <h2 class="text-lg font-semibold mb-2">{title}</h2>
                            <div class="mk-grid rounded-box bg-base-100 p-4">
                                <div class="mk-node outline outline-1 outline-dashed outline-base-300" style=style>
                                    {content}
                                </div>
                            </div>
                        </section>
                    }
                })
                .collect_view()}
        </div>
    }
    .into_any()
}

/// Ruta de desarrollo `/dev/designs`: cada widget en sus cinco diseños, cada
/// uno con otro color. `?theme=dark` (o cualquier tema) lo dibuja en ese tema.
#[component]
pub fn DevDesignsPage() -> impl IntoView {
    if !cfg!(debug_assertions) {
        return view! { "Página no encontrada." }.into_any();
    }
    let theme = leptos_router::hooks::use_query_map()
        .with_untracked(|q| q.get("theme"))
        .filter(|t| domain::THEMES.contains(&t.as_str()))
        .unwrap_or_else(|| "light".to_string());
    let only = leptos_router::hooks::use_query_map().with_untracked(|q| q.get("only"));
    let products = sample_products();
    let first_id = products[0].id.clone();
    let ctx = RenderCtx {
        products: products.clone(),
        editing: false,
        product: Some(products[0].clone()),
    };
    let colors = ["", "secondary", "accent", "neutral", "#e85d04"];
    let rows: Vec<(String, Vec<(String, Node)>)> = WIDGET_TYPES
        .iter()
        .filter(|(t, _)| {
            only.as_deref()
                .is_none_or(|o| o.split(',').any(|o| o == *t))
        })
        .map(|(type_name, label)| {
            let mut widget = Widget::defaults(type_name).expect("tipo conocido");
            if let Widget::FeaturedProduct(p) = &mut widget {
                p.product_id = first_id.clone();
            }
            let cells = domain::DESIGNS
                .iter()
                .zip(colors)
                .map(|((n, name, _), color)| {
                    let mut node = Node::new(format!("{type_name}-{n}"), widget.clone());
                    node.size.w = node.size.w.min(48);
                    node.look = domain::Look {
                        design: *n,
                        color: color.to_string(),
                    };
                    let shown = if color.is_empty() { "principal" } else { color };
                    (format!("{n} · {name} · {shown}"), node)
                })
                .collect();
            ((*label).to_string(), cells)
        })
        .collect();

    view! {
        <div data-theme=theme class="mk-store min-h-screen bg-base-200 p-6 text-base-content">
            {rows
                .into_iter()
                .map(|(label, cells)| view! {
                    <section class="mb-10">
                        <h2 class="mb-3 text-xl font-bold">{label}</h2>
                        {cells
                            .into_iter()
                            .map(|(title, mut node)| {
                                node.pos = domain::Pos { x: 0, y: 0 };
                                let style = node_style(&node);
                                let content = render_widget(&node, &ctx);
                                view! {
                                    <h3 class="mt-3 mb-1 text-sm font-semibold opacity-75">{title}</h3>
                                    <div class="mk-grid" style="width:1136px">
                                        <div class="mk-node" style=style>{content}</div>
                                    </div>
                                }
                            })
                            .collect_view()}
                    </section>
                })
                .collect_view()}
        </div>
    }
    .into_any()
}
