//! Marketplace final: recibe un [`PageView`] y lo renderiza.

use domain::PageView;
use leptos::prelude::*;
use widgets::{node_style, render_widget, RenderCtx};

#[component]
pub fn StorefrontPage(view: PageView) -> impl IntoView {
    let PageView { doc, products, .. } = view;
    let ctx = RenderCtx {
        products,
        editing: false,
    };
    // Orden de lectura (y, x): en pantallas angostas los nodos se apilan así.
    let mut nodes = doc.nodes;
    nodes.sort_by_key(|n| (n.pos.y, n.pos.x));

    view! {
        <div data-theme=doc.theme class="mk-store min-h-screen bg-base-200 text-base-content" style=doc.typography.css_vars()>
            <div class="mx-auto max-w-6xl p-4">
                <div class="mk-grid">
                    {nodes
                        .into_iter()
                        .map(|node| {
                            let style = node_style(&node);
                            let content = render_widget(&node, &ctx);
                            view! { <div class="mk-node" style=style>{content}</div> }
                        })
                        .collect_view()}
                </div>
            </div>
        </div>
    }
}
