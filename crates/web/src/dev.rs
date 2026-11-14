//! Ruta de desarrollo `/dev/widgets`: cada widget en cada uno de sus tamaños.

use domain::{Node, Pos, Product, Widget, WIDGET_TYPES};
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
            image_url: format!("https://picsum.photos/seed/mk-product-{}/600/400", i + 1),
        })
        .collect()
}

#[component]
pub fn DevWidgetsPage() -> impl IntoView {
    let products = sample_products();
    let first_id = products[0].id.clone();
    let ctx = RenderCtx {
        products,
        editing: false,
    };

    let cases: Vec<(String, Node)> = WIDGET_TYPES
        .iter()
        .flat_map(|(type_name, label)| {
            let mut widget = Widget::defaults(type_name).expect("tipo conocido");
            if let Widget::FeaturedProduct(p) = &mut widget {
                p.product_id = first_id.clone();
            }
            widget
                .sizes()
                .iter()
                .map(|s| {
                    let node = Node {
                        id: format!("{type_name}-{}", s.id),
                        widget: widget.clone(),
                        pos: Pos { x: 0, y: 0 },
                        size: s.id.to_string(),
                    };
                    (format!("{label} · {} ({}×{})", s.id, s.w, s.h), node)
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
                    let content = render_widget(&node.widget, &ctx);
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
}
