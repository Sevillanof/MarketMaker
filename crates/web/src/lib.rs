#![recursion_limit = "256"]

use domain::PageDoc;
use editor::{Editor, PublishAction, SaveAction};
use leptos::prelude::*;
use leptos_meta::{provide_meta_context, MetaTags, Stylesheet, Title};
use leptos_router::{
    components::{Redirect, Route, Router, Routes},
    hooks::use_location,
    path, SsrMode, StaticSegment, WildcardSegment,
};
use storefront::StorefrontPage;

mod account;
pub mod auth;
mod checkout;
mod dev;
#[cfg(feature = "ssr")]
pub mod media;
mod photos;
mod platform;
mod orders;
mod products;
mod sales;
pub mod server;
pub mod site;
pub mod stores;

pub use auth::*;
pub use server::*;
pub use stores::*;

pub fn shell(options: LeptosOptions) -> impl IntoView {
    view! {
        <!DOCTYPE html>
        <html lang="es">
            <head>
                <meta charset="utf-8"/>
                <meta name="viewport" content="width=device-width, initial-scale=1"/>
                <AutoReload options=options.clone()/>
                <HydrationScripts options/>
                <MetaTags/>
            </head>
            <body>
                <App/>
            </body>
        </html>
    }
}

#[component]
pub fn App() -> impl IntoView {
    // Provides context that manages stylesheets, titles, meta tags, etc.
    provide_meta_context();

    view! {
        <Stylesheet id="leptos" href="/pkg/marketplace.css"/>
        <Title text="Marketplace"/>

        <Router>
            <main>
                <Routes fallback=|| "Página no encontrada.".into_view()>
                    <Route path=StaticSegment("") view=Home ssr=SsrMode::Async/>
                    <Route path=path!("/editor") view=EditorPage ssr=SsrMode::Async/>
                    <Route path=path!("/entrar") view=account::SignInPage ssr=SsrMode::Async/>
                    <Route path=path!("/mis-fotos") view=photos::MyPhotosPage ssr=SsrMode::Async/>
                    <Route path=path!("/tienda/:slug/productos") view=products::ProductsPage ssr=SsrMode::Async/>
                    <Route path=path!("/tienda/:slug/productos/:id") view=products::ProductEditPage ssr=SsrMode::Async/>
                    <Route path=path!("/tienda/:slug/pedidos") view=orders::OrdersPage ssr=SsrMode::Async/>
                    <Route path=path!("/tienda/:slug/ventas") view=sales::SalesPage ssr=SsrMode::Async/>
                    <Route path=path!("/comprar") view=checkout::CheckoutPage ssr=SsrMode::Async/>
                    <Route path=path!("/pedido/:id") view=checkout::OrderPage ssr=SsrMode::Async/>
                    <Route path=path!("/entrar/pase") view=platform::PassPage ssr=SsrMode::Async/>
                    <Route path=path!("/dev/widgets") view=dev::DevWidgetsPage/>
                    <Route path=path!("/dev/designs") view=dev::DevDesignsPage/>
                    // Las demás páginas de la tienda (/ofertas, /categorias…).
                    <Route path=WildcardSegment("any") view=Home ssr=SsrMode::Async/>
                </Routes>
            </main>
        </Router>
    }
}

/// `localhost:3000` es la plataforma (tus tiendas); `nombre.localhost:3000`
/// es la tienda `nombre`.
#[component]
fn Home() -> impl IntoView {
    let platform_root = move || use_location().pathname.get_untracked() == "/";
    match site::current_site() {
        domain::Site::Platform if platform_root() => view! { <platform::PlatformHome/> }.into_any(),
        domain::Site::Platform => view! { <platform::NotAStore/> }.into_any(),
        domain::Site::Store(_) => view! { <HomePage/> }.into_any(),
    }
}

/// Marketplace: la última versión publicada de la tienda, en la página de la
/// dirección actual (`/` es Inicio, `/ofertas` es Ofertas…). Toda la tienda
/// se guarda en un solo documento, el de `home`.
#[component]
fn HomePage() -> impl IntoView {
    let page = Resource::new(|| (), |_| load_page("home".to_string()));
    let me = Resource::new(|| (), |_| who_am_i());
    let location = use_location();
    view! {
        <Suspense fallback=|| view! { <ui::PageSkeleton/> }>
            {move || Suspend::new(async move {
                let me = me.await.ok().flatten();
                match page.await {
                    Ok(view) => view! {
                        <account::OwnerBar me/>
                        {move || {
                            let path = location.pathname.get();
                            view! { <StorefrontPage view=view.clone() path/> }
                        }}
                    }
                    .into_any(),
                    Err(_) => view! { <platform::StoreNotFound/> }.into_any(),
                }
            })}
        </Suspense>
    }
}

/// Editor en `/editor`: arranca desde el borrador (lo último guardado) de `home`.
#[component]
fn EditorPage() -> impl IntoView {
    let page = Resource::new(|| (), |_| load_draft("home".to_string()));
    let publish: PublishAction =
        Action::new(|version: &u32| publish_page("home".to_string(), *version));
    let save: SaveAction = Action::new(|(doc, base_version): &(PageDoc, u32)| {
        save_page("home".to_string(), doc.clone(), *base_version)
    });
    let me = Resource::new(|| (), |_| who_am_i());
    if site::current_site() == domain::Site::Platform {
        // En la plataforma no hay editor: cada tienda tiene el suyo.
        return view! { <Redirect path="/"/> }.into_any();
    }
    view! {
        <Title text="Editor · Marketplace"/>
        <Suspense fallback=|| view! { <ui::PageSkeleton/> }>
            {move || Suspend::new(async move {
                // Sin cuenta, a Entrar; con una cuenta que no es dueña, un aviso.
                // Esto es para orientar: el permiso real lo revisa `save_page`.
                match me.await {
                    Ok(Some(me)) if me.can_edit => {}
                    Ok(Some(me)) => return view! { <NotYourStore me/> }.into_any(),
                    _ => return view! { <Redirect path="/entrar?volver=/editor"/> }.into_any(),
                }
                match page.await {
                    Ok(initial) => view! { <Editor initial save publish/> }.into_any(),
                    Err(e) => view! {
                        <div class="p-8">
                            <div class="alert alert-error">{format!("No se pudo cargar la página: {e}")}</div>
                        </div>
                    }
                    .into_any(),
                }
            })}
        </Suspense>
    }
    .into_any()
}

/// `/editor` con una cuenta que no es dueña de esta tienda.
#[component]
fn NotYourStore(me: domain::Me) -> impl IntoView {
    view! {
        <div data-theme="light" class="min-h-screen bg-base-200 flex items-start justify-center px-4 py-16">
            <div role="alert" class="card bg-base-100 shadow-sm w-full max-w-md">
                <div class="card-body gap-3">
                    <h1 class="card-title text-2xl">"Esta tienda no es tuya"</h1>
                    <p>"Entraste como " <strong>{me.email}</strong> ", y esa cuenta no puede editar esta tienda."</p>
                    <div class="flex gap-2">
                        <a class="btn btn-primary" href="/">"Ver la tienda"</a>
                        <a class="btn btn-ghost" href="/entrar">"Cambiar de cuenta"</a>
                    </div>
                </div>
            </div>
        </div>
    }
}
