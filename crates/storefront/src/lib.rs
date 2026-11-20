//! Marketplace final: recibe un [`PageView`] y renderiza una de sus páginas.

use domain::{Layer, PageView};
use leptos::prelude::*;
use widgets::{
    render_widget, Cart, CartBar, CartDrawer, OpenProduct, ProductDialog, ProductFilter, RenderCtx,
};

/// La página de la tienda que está en la dirección `path` (`/` es Inicio).
/// Si no hay ninguna ahí, un aviso con un enlace a Inicio, entre el menú y el
/// pie de la tienda.
#[component]
pub fn StorefrontPage(view: PageView, #[prop(into)] path: String) -> impl IntoView {
    let PageView {
        doc,
        products,
        store,
        coupons,
        ..
    } = view;
    // El bloque Cupón se ve solo con un cupón que sirve hoy.
    provide_context(widgets::ActiveCoupons(
        coupons.into_iter().map(|c| c.code).collect(),
    ));
    // Nombre, logo y contacto: el menú y el pie los usan si quedan vacíos.
    provide_context(store);
    let page = doc.page_index(&path);
    let mut categories: Vec<String> = products
        .iter()
        .map(|p| p.category.clone())
        .filter(|c| !c.is_empty())
        .collect();
    categories.sort();
    categories.dedup();
    provide_context(widgets::StoreLinks {
        pages: doc.pages.iter().map(|p| p.path.clone()).collect(),
        categories,
    });
    // Tocar un producto abre su ficha (ver `widgets::ProductDialog`).
    let open = RwSignal::new(None::<String>);
    provide_context(OpenProduct(open));
    widgets::open_from_query(open);
    if let Some(hook) = use_context::<widgets::UsageHook>() {
        Effect::new(move |_| {
            if open.get().is_some() {
                hook.0.run("producto");
            }
        });
    }
    // Lo que el comprador eligió en los filtros por etiqueta (ver `widgets::tags`).
    ProductFilter::provide_from_url();
    // Carrito del comprador, guardado en su navegador (ver `widgets::cart`).
    Cart::provide(products.clone());
    let ctx = RenderCtx {
        products: products.clone(),
        editing: false,
        product: None,
    };
    let theme = doc.theme.clone();
    let style = format!("{}; {}", doc.typography.css_vars(), doc.style.css_vars());
    let layout = doc.layout.css_vars();
    // Los bloques de una grilla, en orden de lectura (y, x): en pantallas
    // angostas se apilan así.
    let grid = move |mut nodes: Vec<domain::Node>| {
        nodes.sort_by_key(|n| (n.pos.y, n.pos.x));
        view! {
            <div class="mk-wrap" style=layout.clone()>
                <div class="mk-grid">
                    {widgets::page_nodes(&nodes)
                        .into_iter()
                        .zip(nodes)
                        .map(|((class, style), node)| {
                            let content = render_widget(&node, &ctx);
                            view! { <div class=class style=style>{content}</div> }
                        })
                        .collect_view()}
                </div>
            </div>
        }
    };
    // Obligatorio en las tiendas online de Argentina (Res. 424/2020). Si la
    // página tiene pie, estos enlaces van adentro (ver `FooterWidget`).
    let legal = |has_footer: bool| {
        (!has_footer).then(|| view! {
            <p class="mk-legal flex flex-wrap items-center justify-center gap-x-6 gap-y-2 py-6 text-center text-sm text-base-content/75">
                <widgets::AccountFooterLink/>
                <a class="link link-hover" href="/arrepentimiento">"Botón de arrepentimiento"</a>
            </p>
        })
    };
    let is_footer = |n: &domain::Node| matches!(n.widget, domain::Widget::Footer(_));
    let Some(page) = page else {
        // La página no existe: el aviso entre el menú y el pie del Inicio,
        // así el comprador sigue dentro de la tienda (menú, carrito, cuenta).
        let home = doc.layer(Layer::Page(0)).clone();
        let top = |mut n: domain::Node| {
            n.pos.y = 0;
            n
        };
        let navs: Vec<_> = home
            .iter()
            .filter(|n| matches!(n.widget, domain::Widget::Navbar(_)))
            .take(1)
            .cloned()
            .map(top)
            .collect();
        let foots: Vec<_> = home
            .iter()
            .filter(|n| is_footer(n))
            .take(1)
            .cloned()
            .map(top)
            .collect();
        let has_footer = !foots.is_empty();
        if has_footer {
            provide_context(widgets::LegalInFooter);
        }
        return view! {
            <div data-theme=theme class="mk-store flex min-h-screen flex-col bg-base-200 text-base-content" style=style>
                {(!navs.is_empty()).then(|| grid(navs))}
                <main class="mx-auto flex max-w-md flex-1 flex-col items-center justify-center gap-3 px-6 py-16 text-center">
                    <h1 class="mk-h text-2xl font-bold">"No encontramos esta página"</h1>
                    <p class="text-base-content/75">"Puede que la dirección esté mal escrita o que la página ya no exista."</p>
                    <div class="mt-2 flex flex-wrap justify-center gap-2">
                        <a class="btn btn-primary" href="/">"Ir al inicio"</a>
                    </div>
                </main>
                {has_footer.then(|| grid(foots))}
                {legal(has_footer)}
                <CartBar/>
                <CartDrawer/>
            </div>
        }
        .into_any();
    };
    let nodes = doc.layer(Layer::Page(page)).clone();
    let has_footer = nodes.iter().any(is_footer);
    if has_footer {
        provide_context(widgets::LegalInFooter);
    }

    view! {
        <div data-theme=theme class="mk-store min-h-screen bg-base-200 text-base-content" style=style>
            {grid(nodes)}
            {legal(has_footer)}
            <ProductDialog modal=doc.product_modal products open/>
            <CartBar/>
            <CartDrawer/>
        </div>
    }
    .into_any()
}
