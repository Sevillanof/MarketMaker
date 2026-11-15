//! Marketplace final: recibe un [`PageView`] y renderiza una de sus páginas.

use domain::{Layer, PageView};
use leptos::prelude::*;
use widgets::{
    node_style, render_widget, Cart, CartBar, CartDrawer, OpenProduct, ProductDialog, RenderCtx,
};

/// La página de la tienda que está en la dirección `path` (`/` es Inicio).
/// Si no hay ninguna ahí, un aviso con un enlace a Inicio.
#[component]
pub fn StorefrontPage(view: PageView, #[prop(into)] path: String) -> impl IntoView {
    let PageView { doc, products, .. } = view;
    let Some(page) = doc.page_index(&path) else {
        return view! {
            <div data-theme=doc.theme class="mk-store min-h-screen bg-base-200 text-base-content" style=doc.typography.css_vars()>
                <div class="mx-auto flex max-w-md flex-col items-center gap-3 p-16 text-center">
                    <h1 class="mk-h text-2xl font-bold">"No encontramos esta página"</h1>
                    <p class="text-base-content/75">"Puede que la dirección esté mal escrita o que la página ya no exista."</p>
                    <a class="btn btn-primary" href="/">"Ir al inicio"</a>
                </div>
            </div>
        }
        .into_any();
    };
    // Tocar un producto abre su ficha (ver `widgets::ProductDialog`).
    let open = RwSignal::new(None::<String>);
    provide_context(OpenProduct(open));
    // Carrito del comprador, guardado en su navegador (ver `widgets::cart`).
    Cart::provide(products.clone());
    let ctx = RenderCtx {
        products: products.clone(),
        editing: false,
        product: None,
    };
    // Orden de lectura (y, x): en pantallas angostas los nodos se apilan así.
    let mut nodes = doc.layer(Layer::Page(page)).clone();
    nodes.sort_by_key(|n| (n.pos.y, n.pos.x));

    view! {
        <div data-theme=doc.theme class="mk-store min-h-screen bg-base-200 text-base-content" style=format!("{}; {}", doc.typography.css_vars(), doc.style.css_vars())>
            <div class="mk-wrap" style=doc.layout.css_vars()>
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
            <ProductDialog modal=doc.product_modal products open/>
            <CartBar/>
            <CartDrawer/>
        </div>
    }
    .into_any()
}
