#![recursion_limit = "256"]

use domain::PageDoc;
use editor::{Editor, PublishAction, SaveAction};
use leptos::prelude::*;
use leptos_meta::{provide_meta_context, MetaTags, Title};
use leptos_router::{
    components::{ParentRoute, Redirect, Route, Router, Routes},
    hooks::use_location,
    path, SsrMode, StaticSegment, WildcardSegment,
};
use storefront::StorefrontPage;

mod account;
mod analytics;
#[cfg(feature = "ssr")]
pub mod andreani;
pub mod auth;
mod checkout;
mod dev;
#[cfg(feature = "ssr")]
pub mod email;
mod landing;
mod legal;
#[cfg(feature = "ssr")]
pub mod limits;
#[cfg(feature = "ssr")]
pub mod media;
#[cfg(feature = "ssr")]
pub mod mp;
mod orders;
mod panel;
mod payments;
mod photos;
mod platform;
mod products;
mod regret;
mod sales;
#[cfg(feature = "ssr")]
pub mod shipments;
mod store_info;
pub mod server;
pub mod site;
pub mod stores;
mod whitelist;

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
                <leptos_meta::HashedStylesheet options=options.clone() id="leptos"/>
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
    // Uso y abandono: el carrito y las fichas avisan por acá (ver `analytics`).
    provide_context(widgets::UsageHook(Callback::new(|step: &'static str| {
        analytics::track_client(step, None)
    })));

    provide_context(panel::NavCache(RwSignal::new(None)));

    view! {
        <Title text="Marketplace"/>

        <Router>
            <analytics::TrackPages/>
            <main>
                <Routes fallback=|| view! { <platform::NotAStore/> }>
                    <Route path=StaticSegment("") view=Home ssr=SsrMode::Async/>
                    // El editor se baja aparte (`cargo leptos build --split`): quien
                    // compra en una tienda no carga su código.
                    <Route path=path!("/editor") view={leptos_router::Lazy::<EditorRoute>::new()} ssr=SsrMode::Async/>
                    <Route path=path!("/entrar") view=account::SignInPage ssr=SsrMode::Async/>
                    <Route path=path!("/cuenta") view=account::AccountPage ssr=SsrMode::Async/>
                    <Route path=path!("/mis-fotos") view=photos::MyPhotosPage ssr=SsrMode::Async/>
                    // El menú del panel es el marco de estas páginas: queda fijo y
                    // solo cambia el contenido.
                    <ParentRoute path=path!("/tienda/:slug") view=panel::PanelLayout ssr=SsrMode::Async>
                        <Route path=path!("") view=panel::StoreHomePage ssr=SsrMode::Async/>
                        <Route path=path!("productos") view=products::ProductsPage ssr=SsrMode::Async/>
                        <Route path=path!("productos/:id") view=products::ProductEditPage ssr=SsrMode::Async/>
                        <Route path=path!("pedidos") view=orders::OrdersPage ssr=SsrMode::Async/>
                        <Route path=path!("cobros") view=payments::PaymentsPage ssr=SsrMode::Async/>
                        <Route path=path!("ventas") view=sales::SalesPage ssr=SsrMode::Async/>
                        <Route path=path!("datos") view=store_info::StoreInfoPage ssr=SsrMode::Async/>
                    </ParentRoute>
                    <Route path=path!("/admin/uso") view=analytics::UsagePage ssr=SsrMode::Async/>
                    <Route path=path!("/comprar") view=checkout::CheckoutPage ssr=SsrMode::Async/>
                    <Route path=path!("/terminos") view=legal::TermsPage/>
                    <Route path=path!("/privacidad") view=legal::PrivacyPage/>
                    <Route path=path!("/arrepentimiento") view=regret::RegretPage ssr=SsrMode::Async/>
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

/// Responde con el código 404 (la página no existe), para los buscadores.
pub(crate) fn not_found_status() {
    #[cfg(feature = "ssr")]
    if let Some(r) = use_context::<leptos_axum::ResponseOptions>() {
        r.set_status(http::StatusCode::NOT_FOUND);
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
                    Ok(view) if view.published == 0 => view! {
                        <account::OwnerBar me/>
                        <StoreMeta view=view.clone()/>
                        <StoreSoon name=view.store_name.clone()/>
                    }
                    .into_any(),
                    Ok(view) => {
                        if view.doc.page_index(&location.pathname.get_untracked()).is_none() {
                            not_found_status();
                        }
                        view! {
                        <account::OwnerBar me/>
                        <StoreMeta view=view.clone()/>
                        {move || {
                            let path = location.pathname.get();
                            view! { <StorefrontPage view=view.clone() path/> }
                        }}
                    }
                    .into_any()
                    }
                    Err(_) => view! { <platform::StoreNotFound/> }.into_any(),
                }
            })}
        </Suspense>
    }
}

/// La tienda todavía no se publicó: un aviso amable en lugar de una página
/// vacía.
#[component]
fn StoreSoon(name: String) -> impl IntoView {
    view! {
        <div class="grid min-h-screen place-items-center bg-base-200 px-4">
            <div class="flex max-w-md flex-col items-center gap-3 text-center">
                <h1 class="text-3xl font-bold">{name}</h1>
                <p class="text-lg text-base-content/75">"Estamos preparando la tienda. Volvé pronto."</p>
            </div>
        </div>
    }
}

/// Título de la pestaña y lo que se ve al compartir el enlace (WhatsApp,
/// redes): el nombre de la tienda, una frase y una foto.
#[component]
fn StoreMeta(view: domain::PageView) -> impl IntoView {
    let name = view.store_name.clone();
    let banner = view.doc.nodes.iter().find_map(|n| match &n.widget {
        domain::Widget::Banner(b) => Some(b.clone()),
        _ => None,
    });
    let description = banner
        .as_ref()
        .map(|b| b.subtitle.trim().to_string())
        .filter(|s| !s.is_empty() && !domain::STARTER_PLACEHOLDERS.contains(&s.as_str()))
        .unwrap_or_else(|| format!("Comprá online en {name}."));
    let image = banner
        .map(|b| b.image_url)
        .filter(|u| !u.is_empty())
        .or_else(|| view.products.iter().map(|p| p.image_url.clone()).find(|u| !u.is_empty()))
        .map(|u| {
            if u.starts_with('/') {
                format!("{}://{}{u}", site::scheme(), site::current_host())
            } else {
                u
            }
        });
    view! {
        <Title text=name.clone()/>
        <leptos_meta::Meta name="description" content=description.clone()/>
        <leptos_meta::Meta property="og:type" content="website"/>
        <leptos_meta::Meta property="og:title" content=name/>
        <leptos_meta::Meta property="og:description" content=description/>
        {image.map(|i| view! { <leptos_meta::Meta property="og:image" content=i/> })}
    }
}

/// `/editor` como ruta que se carga aparte.
#[derive(Debug)]
struct EditorRoute;

#[leptos_router::lazy_route]
impl leptos_router::LazyRoute for EditorRoute {
    fn data() -> Self {
        EditorRoute
    }

    fn view(_this: Self) -> AnyView {
        view! { <EditorPage/> }.into_any()
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
    if let domain::Site::Store(slug) = site::current_site() {
        provide_context(editor::PanelLink(format!(
            "{}/tienda/{slug}",
            site::platform_url()
        )));
    }
    if site::current_site() == domain::Site::Platform {
        // En la plataforma no hay editor: cada tienda tiene el suyo.
        return view! { <Redirect path="/"/> }.into_any();
    }
    let panel = match site::current_site() {
        domain::Site::Store(slug) => format!("{}/tienda/{slug}", site::platform_url()),
        domain::Site::Platform => "/".into(),
    };
    view! {
        <Title text="Editor · MarketMaker"/>
        // El editor es solo para computadora; en el celular, un aviso.
        <EditorOnDesktop panel/>
        <div class="max-[799px]:hidden">
        <Suspense fallback=|| view! { <ui::EditorSkeleton/> }>
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
        </div>
    }
    .into_any()
}

/// En pantallas de menos de 800 px: el diseño se edita en una computadora; Productos y
/// Pedidos sí se pueden usar desde el celular.
#[component]
fn EditorOnDesktop(panel: String) -> impl IntoView {
    view! {
        <div data-theme="light" class="min-h-screen bg-base-200 px-4 py-12 text-base-content min-[800px]:hidden">
            <div class="card mx-auto max-w-md bg-base-100 shadow-sm"><div class="card-body gap-3">
                <h1 class="card-title text-2xl">"Para diseñar tu tienda, usá una computadora"</h1>
                <p class="text-base-content/75">"El editor necesita una pantalla grande. Desde el celular podés cargar productos y atender tus pedidos."</p>
                <div class="flex flex-wrap gap-2">
                    <a class="btn btn-primary" href=format!("{panel}/pedidos")>"Ver pedidos"</a>
                    <a class="btn" href=format!("{panel}/productos")>"Ver productos"</a>
                </div>
            </div></div>
        </div>
    }
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
