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
mod buyer;
mod buyer_ui;
mod checkout;
mod coupons;
mod dev;
#[cfg(feature = "ssr")]
pub mod email;
mod help;
mod help_editor;
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
pub mod reports;
mod sales;
pub mod seo;
pub mod server;
#[cfg(feature = "ssr")]
pub mod shipments;
pub mod site;
mod store_info;
pub mod stores;
mod team;
mod variants_field;
mod welcome;
mod whitelist;

pub use auth::*;
pub use server::*;
pub use stores::*;

/// Guarda los toques hechos antes de que la página esté viva. Con conexión
/// lenta la tienda se ve varios segundos antes de que sus botones funcionen;
/// sin esto, lo que se toca en ese rato se pierde. El botón tocado queda
/// marcado (`data-mk-wait`, ver `tailwind.css`) y, apenas tiene quien lo
/// atienda, se le repite el toque una sola vez.
///
/// Solo cambia atributos: agregar un nodo rompería la hidratación. Para saber
/// si un botón ya está vivo anota a quién se le pone un manejador de `click`
/// (envuelve `addEventListener` hasta que `App` avisa con `__mkReady`). Los
/// botones que envían un formulario o abren algo sin programa (popover,
/// `commandfor`) ya funcionan solos y no se tocan.
const KEEP_TAPS: &str = r#"(function(){var q=[],hot=new WeakSet,t0=0,n=0,P=EventTarget.prototype,add=P.addEventListener;
P.addEventListener=function(t){if(window.__mkReady&&!q.length)P.addEventListener=add;else if(t=='click'&&this.nodeType==1)hot.add(this);return add.apply(this,arguments)};
function free(e){e.removeAttribute('data-mk-wait');e.removeAttribute('aria-busy')}
function tick(){var late=window.__mkReady&&++n>25;
q=q.filter(function(e){if(!e.isConnected)return 0;if(hot.has(e)||late){free(e);e.click();return 0}return 1});
if(window.__mkReady&&!q.length)P.addEventListener=add;
if(!q.length)return;
if(Date.now()-t0>60000){q.forEach(free);q=[];return}
setTimeout(tick,200)}
document.addEventListener('click',function(ev){if(window.__mkReady||!ev.isTrusted||!ev.target.closest)return;
var e=ev.target.closest('button');
if(!e||e.disabled||(e.form&&e.type=='submit')||e.hasAttribute('popovertarget')||e.hasAttribute('commandfor')||hot.has(e)||q.indexOf(e)>=0)return;
e.setAttribute('data-mk-wait','');e.setAttribute('aria-busy','true');q.push(e);
if(q.length==1){t0=Date.now();n=0;tick()}},true)})()"#;

pub fn shell(options: LeptosOptions) -> impl IntoView {
    view! {
        <!DOCTYPE html>
        <html lang="es">
            <head>
                <meta charset="utf-8"/>
                <meta name="viewport" content="width=device-width, initial-scale=1"/>
                <AutoReload options=options.clone()/>
                <leptos_meta::HashedStylesheet options=options.clone() id="leptos"/>
                <script inner_html=KEEP_TAPS></script>
                <HydrationScripts options=options.clone()/>
                {editor_preload(&options)}
                <MetaTags/>
            </head>
            <body>
                <App/>
                // Reportar un problema: botón, selección de área y envío (ver `reports`).
                <script src="/js/reportar.js?v=5" defer></script>
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
    // Una sola lista de fotos y una sola bandeja de subidas para toda la app.
    editor::provide_photos();
    // La página ya atiende sus botones: `KEEP_TAPS` deja de guardar toques.
    Effect::new(|| {
        if let Some(w) = web_sys::window() {
            _ = js_sys::Reflect::set(&w, &"__mkReady".into(), &true.into());
        }
    });

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
                    <Route path=path!("/entrar") view={leptos_router::Lazy::<SignInRoute>::new()} ssr=SsrMode::Async/>
                    <Route path=path!("/cuenta") view={leptos_router::Lazy::<AccountRoute>::new()} ssr=SsrMode::Async/>
                    <Route path=path!("/mis-fotos") view={leptos_router::Lazy::<PhotosRoute>::new()} ssr=SsrMode::Async/>
                    // El menú del panel es el marco de estas páginas: queda fijo y
                    // solo cambia el contenido.
                    <ParentRoute path=path!("/tienda/:slug") view={leptos_router::Lazy::<PanelLayoutRoute>::new()} ssr=SsrMode::Async>
                        <Route path=path!("") view={leptos_router::Lazy::<HomeRoute>::new()} ssr=SsrMode::Async/>
                        <Route path=path!("productos") view={leptos_router::Lazy::<ProductsRoute>::new()} ssr=SsrMode::Async/>
                        <Route path=path!("productos/:id") view={leptos_router::Lazy::<ProductEditRoute>::new()} ssr=SsrMode::Async/>
                        <Route path=path!("pedidos") view={leptos_router::Lazy::<OrdersRoute>::new()} ssr=SsrMode::Async/>
                        <Route path=path!("cobros") view={leptos_router::Lazy::<PaymentsRoute>::new()} ssr=SsrMode::Async/>
                        <Route path=path!("cupones") view={leptos_router::Lazy::<CouponsRoute>::new()} ssr=SsrMode::Async/>
                        <Route path=path!("ventas") view={leptos_router::Lazy::<SalesRoute>::new()} ssr=SsrMode::Async/>
                        <Route path=path!("datos") view={leptos_router::Lazy::<StoreInfoRoute>::new()} ssr=SsrMode::Async/>
                    </ParentRoute>
                    <Route path=path!("/admin/uso") view={leptos_router::Lazy::<UsageRoute>::new()} ssr=SsrMode::Async/>
                    <Route path=path!("/comprar") view=checkout::CheckoutPage ssr=SsrMode::Async/>
                    <Route path=path!("/terminos") view=legal::TermsPage/>
                    <Route path=path!("/privacidad") view=legal::PrivacyPage/>
                    <Route path=path!("/arrepentimiento") view=regret::RegretPage ssr=SsrMode::Async/>
                    <Route path=path!("/pedido/:id") view=checkout::OrderPage ssr=SsrMode::Async/>
                    // La cuenta a secas abre los pedidos.
                    <Route path=path!("/mi-cuenta") view=|| view! { <Redirect path="/mi-cuenta/pedidos"/> } ssr=SsrMode::Async/>
                    <Route path=path!("/mi-cuenta/entrar") view=buyer_ui::BuyerEnterPage ssr=SsrMode::Async/>
                    <Route path=path!("/mi-cuenta/pedidos") view=buyer_ui::BuyerOrdersPage ssr=SsrMode::Async/>
                    <Route path=path!("/mi-cuenta/pedidos/:id") view=buyer_ui::BuyerOrderPage ssr=SsrMode::Async/>
                    <Route path=path!("/mi-cuenta/favoritos") view=buyer_ui::BuyerFavoritesPage ssr=SsrMode::Async/>
                    <Route path=path!("/mi-cuenta/datos") view=|| view! { <buyer_ui::BuyerProfilePage section="datos"/> } ssr=SsrMode::Async/>
                    <Route path=path!("/mi-cuenta/direcciones") view=|| view! { <buyer_ui::BuyerProfilePage section="direcciones"/> } ssr=SsrMode::Async/>
                    <Route path=path!("/entrar/pase") view=platform::PassPage ssr=SsrMode::Async/>
                    <Route path=path!("/dev/widgets") view=dev::DevWidgetsPage/>
                    <Route path=path!("/dev/designs") view=dev::DevDesignsPage/>
                    <Route path=path!("/dev/ficha") view=dev::DevProductStylesPage/>
                    <Route path=path!("/dev/menus") view=dev::DevMenusPage/>
                    // Lo guardado sin publicar, para la vista de celular del editor.
                    <Route path=path!("/vista-previa") view=DraftPreviewPage ssr=SsrMode::Async/>
                    // Las demás páginas de la tienda (/ofertas, /categorias…).
                    <Route path=WildcardSegment("any") view=Home ssr=SsrMode::Async/>
                </Routes>
            </main>
            <editor::UploadTray/>
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
                    Ok(view) if view.published == 0 => {
                        let owner = me.as_ref().is_some_and(|m| m.can_edit);
                        view! {
                            <account::OwnerBar me/>
                            <StoreMeta view=view.clone()/>
                            <StoreSoon name=view.store_name.clone() owner/>
                        }
                    }
                    .into_any(),
                    // Pausada: quien compra ve el aviso; quien la edita, la tienda.
                    Ok(view) if view.paused && !me.as_ref().is_some_and(|m| m.can_edit) => view! {
                        <StoreMeta view=view.clone()/>
                        <StoreSoon name=view.store_name.clone() owner=false/>
                    }
                    .into_any(),
                    Ok(view) => {
                        if view.doc.page_index(&location.pathname.get_untracked()).is_none() {
                            not_found_status();
                        }
                        view! {
                        <account::OwnerBar me/>
                        <StoreMeta view=view.clone()/>
                        {view.test_store.then(|| view! { <TestStoreTag/> })}
                        <buyer_ui::BuyerScope store_name=view.store_name.clone()>
                            {move || {
                                let path = location.pathname.get();
                                // Cada página con su nombre en la pestaña y
                                // en el historial: «Contacto · Casa Alma».
                                let title = match view.doc.page_index(&path) {
                                    Some(0) => view.store_name.clone(),
                                    Some(i) => format!("{} · {}", view.doc.pages[i - 1].name, view.store_name),
                                    None => format!("Página no encontrada · {}", view.store_name),
                                };
                                view! {
                                    <Title text=title/>
                                    <StorefrontPage view=view.clone() path/>
                                }
                            }}
                        </buyer_ui::BuyerScope>
                    }
                    .into_any()
                    }
                    Err(_) => view! { <platform::StoreNotFound/> }.into_any(),
                }
            })}
        </Suspense>
    }
}

/// El borrador de la tienda tal como lo vería un comprador, en la página
/// `?ruta=` (Inicio si falta). La usa la vista de celular del editor, dentro
/// de un marco del ancho de un teléfono. Solo para quien puede editar.
#[component]
fn DraftPreviewPage() -> impl IntoView {
    // El editor la muestra en un marco: se permite solo desde el mismo sitio
    // (las demás páginas no se pueden enmarcar, ver `server::main`).
    #[cfg(feature = "ssr")]
    if let Some(r) = use_context::<leptos_axum::ResponseOptions>() {
        r.insert_header(
            http::header::X_FRAME_OPTIONS,
            http::HeaderValue::from_static("SAMEORIGIN"),
        );
        r.insert_header(
            http::header::CONTENT_SECURITY_POLICY,
            http::HeaderValue::from_static(
                "frame-ancestors 'self'; base-uri 'self'; object-src 'none'",
            ),
        );
    }
    let query = leptos_router::hooks::use_query_map();
    let path = query
        .with_untracked(|q| q.get("ruta"))
        .filter(|r| r.starts_with('/'))
        .unwrap_or_else(|| "/".to_string());
    let page = Resource::new(|| (), |_| server::load_preview("home".to_string()));
    view! {
        <leptos_meta::Meta name="robots" content="noindex"/>
        <Suspense fallback=|| view! { <ui::PageSkeleton/> }>
            {move || {
                let path = path.clone();
                Suspend::new(async move {
                    match page.await {
                        Ok(view) => view! {
                            <buyer_ui::BuyerScope store_name=view.store_name.clone()>
                                <StorefrontPage view=view.clone() path=path.clone()/>
                            </buyer_ui::BuyerScope>
                        }
                        .into_any(),
                        Err(_) => view! {
                            <p class="p-8 text-center text-base-content/75">"Entrá con tu cuenta para ver la vista previa."</p>
                        }
                        .into_any(),
                    }
                })
            }}
        </Suspense>
    }
}

/// La tienda todavía no se publicó: un aviso amable en lugar de una página
/// vacía. A quien puede editarla, además, le explica por qué ve esto y lo
/// lleva al editor a publicarla.
#[component]
fn StoreSoon(name: String, owner: bool) -> impl IntoView {
    view! {
        <div class="grid min-h-screen place-items-center bg-base-200 px-4 py-10">
            <div class="flex max-w-xl flex-col items-center gap-3 text-center">
                <h1 class="text-3xl font-bold">{name}</h1>
                <p class="text-lg text-base-content/75">"Estamos preparando la tienda. Volvé pronto."</p>
                {owner.then(|| view! {
                    <div class="mt-5 flex flex-col items-start gap-2 rounded-box border border-base-300 bg-base-100 p-5 text-left">
                        <p class="font-semibold">"Esto es lo que ven tus clientes: tu tienda todavía no está publicada."</p>
                        <p class="text-sm text-base-content/75">"Cuando la publiques desde el editor, acá van a ver tu diseño y tus productos."</p>
                        <a class="btn btn-primary btn-sm mt-2" href="/editor">"Ir al editor para publicar"</a>
                    </div>
                })}
            </div>
        </div>
    }
}

/// Etiqueta flotante de las tiendas de pruebas. Va abajo a la izquierda para
/// no chocar con el botón de ayuda ni con el carrito (a la derecha).
#[component]
fn TestStoreTag() -> impl IntoView {
    view! {
        <p class="pointer-events-none fixed bottom-[max(1rem,env(safe-area-inset-bottom))] left-4 z-40 rounded-full bg-neutral px-3 py-1.5 text-xs font-semibold text-neutral-content shadow-lg">
            "Tienda de prueba: no se cobra ni se envía nada"
        </p>
    }
}

/// Título de la pestaña y lo que se ve al compartir el enlace (WhatsApp,
/// redes): el nombre de la tienda, una frase y una foto. Con `?ver=` (el
/// enlace de "Compartir" de una ficha), los del producto.
#[component]
fn StoreMeta(view: domain::PageView) -> impl IntoView {
    let name = view.store_name.clone();
    let absolute = |u: String| {
        if u.starts_with('/') {
            format!("{}://{}{u}", site::scheme(), site::current_host())
        } else {
            u
        }
    };
    let shared = leptos_router::hooks::use_query_map()
        .with_untracked(|q| q.get("ver"))
        .and_then(|id| {
            view.products
                .iter()
                .find(|p| p.id == id && p.visible)
                .cloned()
        });
    // La tienda sin publicar no tiene que aparecer en Google.
    let hidden = view.published == 0;
    let icon = view! { <StoreIcon view=view.clone()/> };
    if let Some(p) = shared {
        let title = format!("{} · {name}", p.name);
        let text = p
            .description
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");
        let price = ui::format_price(p.price_cents);
        let description = if text.is_empty() {
            format!("{price} en {name}.")
        } else {
            let short: String = text.chars().take(150).collect();
            let cut = if short.len() < text.len() { "…" } else { "" };
            format!("{price}. {short}{cut}")
        };
        let image = Some(p.image_url).filter(|u| !u.is_empty()).map(absolute);
        return view! {
            {icon}
            <Title text=title.clone()/>
            {hidden.then(|| view! { <leptos_meta::Meta name="robots" content="noindex"/> })}
            <leptos_meta::Meta name="description" content=description.clone()/>
            <leptos_meta::Meta property="og:type" content="product"/>
            <leptos_meta::Meta property="og:title" content=title/>
            <leptos_meta::Meta property="og:description" content=description/>
            {image.map(|i| view! { <leptos_meta::Meta property="og:image" content=i/> })}
        }
        .into_any();
    }
    let banner = view.doc.nodes.iter().find_map(|n| match &n.widget {
        domain::Widget::Banner(b) => Some(b.clone()),
        _ => None,
    });
    let real = |s: &str| {
        let s = s.trim();
        (!s.is_empty() && !domain::STARTER_PLACEHOLDERS.contains(&s)).then(|| s.to_string())
    };
    // La frase de la tienda (Datos de la tienda), la del banner o, si no hay
    // ninguna, qué vende: sus primeras categorías.
    let description = real(&view.store.tagline)
        .or_else(|| banner.as_ref().and_then(|b| real(&b.subtitle)))
        .unwrap_or_else(|| {
            let mut cats: Vec<&str> = Vec::new();
            for p in view.products.iter().filter(|p| p.visible) {
                let c = p.category.trim();
                if !c.is_empty() && !cats.contains(&c) {
                    cats.push(c);
                }
            }
            match cats.len() {
                0 => format!("Comprá online en {name}."),
                1 => format!("{} en {name}. Comprá online.", cats[0]),
                _ => {
                    let list = cats.iter().take(3).copied().collect::<Vec<_>>().join(", ");
                    format!("{list} y más en {name}. Comprá online.")
                }
            }
        });
    let image = banner
        .map(|b| b.image_url)
        .filter(|u| !u.is_empty())
        .or_else(|| {
            view.products
                .iter()
                .map(|p| p.image_url.clone())
                .find(|u| !u.is_empty())
        })
        .map(absolute);
    view! {
        {icon}
        <Title text=name.clone()/>
        {hidden.then(|| view! { <leptos_meta::Meta name="robots" content="noindex"/> })}
        <leptos_meta::Meta name="description" content=description.clone()/>
        <leptos_meta::Meta property="og:type" content="website"/>
        <leptos_meta::Meta property="og:title" content=name/>
        <leptos_meta::Meta property="og:description" content=description/>
        {image.map(|i| view! { <leptos_meta::Meta property="og:image" content=i/> })}
    }
    .into_any()
}

/// Ícono de la pestaña de la tienda: su logo o, sin logo, la inicial sobre
/// el color principal de la tienda (no el de MarketMaker).
#[component]
fn StoreIcon(view: domain::PageView) -> impl IntoView {
    let logo = view.store.logo_url.trim().to_string();
    if !logo.is_empty() {
        return view! { <leptos_meta::Link rel="icon" href=logo/> }.into_any();
    }
    let letter = view
        .store_name
        .trim()
        .chars()
        .find(|c| c.is_alphanumeric())
        .map_or("•".to_string(), |c| c.to_uppercase().collect());
    // Un tema propio trae el color; uno de daisyUI se lee de la página ya
    // pintada (abajo), y mientras tanto va un gris neutro.
    let color = view
        .doc
        .style
        .colors
        .get("primary")
        .filter(|c| c.starts_with('#') && c.len() <= 9)
        .cloned()
        .unwrap_or_else(|| "#52525b".to_string());
    let fixed = view.doc.style.colors.contains_key("primary");
    let href = initial_icon(&letter, &color, "#ffffff");
    if !fixed {
        Effect::new(move |_| painted_icon(&letter));
    }
    view! { <leptos_meta::Link id="mk-icon" rel="icon" href=href/> }.into_any()
}

/// Un cuadrado redondeado de `color` con `letter` en el medio, como SVG.
fn initial_icon(letter: &str, color: &str, ink: &str) -> String {
    let esc = |s: &str| {
        s.replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('"', "&quot;")
    };
    let svg = format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 64 64"><rect width="64" height="64" rx="14" fill="{}"/><text x="32" y="45" font-family="system-ui,sans-serif" font-size="38" font-weight="700" text-anchor="middle" fill="{}">{}</text></svg>"#,
        esc(color),
        esc(ink),
        esc(letter)
    );
    let encoded: String = svg
        .chars()
        .map(|c| match c {
            '"' => "'".to_string(),
            '#' => "%23".to_string(),
            '%' => "%25".to_string(),
            '<' => "%3C".to_string(),
            '>' => "%3E".to_string(),
            c => c.to_string(),
        })
        .collect();
    format!("data:image/svg+xml,{encoded}")
}

/// En el navegador: el color principal del tema de la tienda (y el de su
/// texto), dibujados como PNG con la inicial, en el ícono `#mk-icon`.
fn painted_icon(letter: &str) -> Option<()> {
    #[cfg(feature = "hydrate")]
    {
        use leptos::wasm_bindgen::JsCast;
        let doc = web_sys::window()?.document()?;
        let store = doc.query_selector(".mk-store").ok()??;
        let probe = doc.create_element("span").ok()?;
        _ = probe.set_attribute(
            "style",
            "position:absolute;visibility:hidden;background:var(--color-primary);color:var(--color-primary-content)",
        );
        store.append_child(&probe).ok()?;
        let cs = web_sys::window()?.get_computed_style(&probe).ok()??;
        let bg = cs.get_property_value("background-color").ok()?;
        let ink = cs.get_property_value("color").ok()?;
        probe.remove();
        let canvas = doc
            .create_element("canvas")
            .ok()?
            .dyn_into::<web_sys::HtmlCanvasElement>()
            .ok()?;
        canvas.set_width(64);
        canvas.set_height(64);
        let ctx = canvas
            .get_context("2d")
            .ok()??
            .dyn_into::<web_sys::CanvasRenderingContext2d>()
            .ok()?;
        ctx.set_fill_style_str(&bg);
        ctx.begin_path();
        _ = ctx.round_rect_with_f64(0.0, 0.0, 64.0, 64.0, 14.0);
        ctx.fill();
        ctx.set_fill_style_str(&ink);
        ctx.set_font("700 38px system-ui, sans-serif");
        ctx.set_text_align("center");
        ctx.set_text_baseline("middle");
        _ = ctx.fill_text(letter, 32.0, 35.0);
        let url = canvas.to_data_url().ok()?;
        doc.get_element_by_id("mk-icon")?
            .set_attribute("href", &url)
            .ok()?;
        return Some(());
    }
    #[cfg(not(feature = "hydrate"))]
    {
        let _ = letter;
        None
    }
}

/// Las pantallas que no usa quien compra (entrar, cuenta, fotos, panel y
/// administración) se bajan aparte, como el editor: el programa principal que
/// baja todo comprador queda mucho más chico.
#[derive(Debug)]
struct SignInRoute;

#[leptos_router::lazy_route]
impl leptos_router::LazyRoute for SignInRoute {
    fn data() -> Self {
        SignInRoute
    }

    fn view(_this: Self) -> AnyView {
        view! { <account::SignInPage/> }.into_any()
    }
}

#[derive(Debug)]
struct AccountRoute;

#[leptos_router::lazy_route]
impl leptos_router::LazyRoute for AccountRoute {
    fn data() -> Self {
        AccountRoute
    }

    fn view(_this: Self) -> AnyView {
        view! { <account::AccountPage/> }.into_any()
    }
}

#[derive(Debug)]
struct PhotosRoute;

#[leptos_router::lazy_route]
impl leptos_router::LazyRoute for PhotosRoute {
    fn data() -> Self {
        PhotosRoute
    }

    fn view(_this: Self) -> AnyView {
        view! { <photos::MyPhotosPage/> }.into_any()
    }
}

#[derive(Debug)]
struct PanelLayoutRoute;

#[leptos_router::lazy_route]
impl leptos_router::LazyRoute for PanelLayoutRoute {
    fn data() -> Self {
        PanelLayoutRoute
    }

    fn view(_this: Self) -> AnyView {
        view! { <panel::PanelLayout/> }.into_any()
    }
}

#[derive(Debug)]
struct HomeRoute;

#[leptos_router::lazy_route]
impl leptos_router::LazyRoute for HomeRoute {
    fn data() -> Self {
        HomeRoute
    }

    fn view(_this: Self) -> AnyView {
        view! { <panel::StoreHomePage/> }.into_any()
    }
}

#[derive(Debug)]
struct ProductsRoute;

#[leptos_router::lazy_route]
impl leptos_router::LazyRoute for ProductsRoute {
    fn data() -> Self {
        ProductsRoute
    }

    fn view(_this: Self) -> AnyView {
        view! { <products::ProductsPage/> }.into_any()
    }
}

#[derive(Debug)]
struct ProductEditRoute;

#[leptos_router::lazy_route]
impl leptos_router::LazyRoute for ProductEditRoute {
    fn data() -> Self {
        ProductEditRoute
    }

    fn view(_this: Self) -> AnyView {
        view! { <products::ProductsPage/> }.into_any()
    }
}

#[derive(Debug)]
struct OrdersRoute;

#[leptos_router::lazy_route]
impl leptos_router::LazyRoute for OrdersRoute {
    fn data() -> Self {
        OrdersRoute
    }

    fn view(_this: Self) -> AnyView {
        view! { <orders::OrdersPage/> }.into_any()
    }
}

#[derive(Debug)]
struct PaymentsRoute;

#[leptos_router::lazy_route]
impl leptos_router::LazyRoute for PaymentsRoute {
    fn data() -> Self {
        PaymentsRoute
    }

    fn view(_this: Self) -> AnyView {
        view! { <payments::PaymentsPage/> }.into_any()
    }
}

#[derive(Debug)]
struct CouponsRoute;

#[leptos_router::lazy_route]
impl leptos_router::LazyRoute for CouponsRoute {
    fn data() -> Self {
        CouponsRoute
    }

    fn view(_this: Self) -> AnyView {
        view! { <coupons::CouponsPage/> }.into_any()
    }
}

#[derive(Debug)]
struct SalesRoute;

#[leptos_router::lazy_route]
impl leptos_router::LazyRoute for SalesRoute {
    fn data() -> Self {
        SalesRoute
    }

    fn view(_this: Self) -> AnyView {
        view! { <sales::SalesPage/> }.into_any()
    }
}

#[derive(Debug)]
struct StoreInfoRoute;

#[leptos_router::lazy_route]
impl leptos_router::LazyRoute for StoreInfoRoute {
    fn data() -> Self {
        StoreInfoRoute
    }

    fn view(_this: Self) -> AnyView {
        view! { <store_info::StoreInfoPage/> }.into_any()
    }
}

#[derive(Debug)]
struct UsageRoute;

#[leptos_router::lazy_route]
impl leptos_router::LazyRoute for UsageRoute {
    fn data() -> Self {
        UsageRoute
    }

    fn view(_this: Self) -> AnyView {
        view! { <analytics::UsagePage/> }.into_any()
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

/// La parte del editor que se baja aparte (`split___editor_route_view_*.wasm`):
/// su nombre lleva un hash que cambia en cada compilación, así que se busca
/// en la carpeta `pkg` (una sola vez). `None` si no hay (sin `--split`).
#[cfg(feature = "ssr")]
fn editor_chunk_url(opts: &LeptosOptions) -> Option<String> {
    use std::sync::OnceLock;
    static URL: OnceLock<Option<String>> = OnceLock::new();
    URL.get_or_init(|| {
        let pkg = std::path::Path::new(&*opts.site_root).join(&*opts.site_pkg_dir);
        std::fs::read_dir(pkg).ok()?.flatten().find_map(|e| {
            let name = e.file_name().to_string_lossy().into_owned();
            (name.starts_with("split___editor_route_view") && name.ends_with(".wasm"))
                .then(|| format!("/{}/{name}", opts.site_pkg_dir))
        })
    })
    .clone()
}

/// En `/editor`, pide ya la parte del editor (en paralelo con el programa
/// principal): si no, empieza a bajar recién cuando el principal terminó de
/// arrancar. Va en el `<head>` que arma el servidor (no se hidrata).
fn editor_preload(options: &LeptosOptions) -> Option<impl IntoView> {
    #[cfg(feature = "ssr")]
    {
        let parts = use_context::<http::request::Parts>()?;
        if parts.uri.path() != "/editor" {
            return None;
        }
        let href = editor_chunk_url(options)?;
        Some(
            leptos::html::link()
                .rel("preload")
                .href(href)
                .r#as("fetch")
                .crossorigin("anonymous"),
        )
    }
    #[cfg(not(feature = "ssr"))]
    {
        let _ = options;
        None::<()>
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
                    Ok(initial) => view! {
                        <Editor initial save publish/>
                        <crate::help::HelpButton section=Signal::derive(|| panel::Section::Home) editor=true/>
                    }.into_any(),
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

/// En pantallas de menos de 800 px: el diseño se edita en una computadora;
/// Productos, Pedidos y el resto del panel sí se usan desde el teléfono.
#[component]
fn EditorOnDesktop(panel: String) -> impl IntoView {
    let send = Action::new(|_: &()| send_editor_link());
    view! {
        <div data-theme="light" class="min-h-screen bg-base-200 px-4 py-12 text-base-content min-[800px]:hidden">
            <div class="card mx-auto max-w-md bg-base-100 shadow-sm"><div class="card-body gap-3">
                <h1 class="card-title text-2xl text-balance">"El diseño se edita desde una computadora"</h1>
                <p class="text-pretty text-base-content/75">"Desde el teléfono podés cargar productos, atender pedidos y ver cómo queda tu tienda."</p>
                <a class="btn btn-primary min-h-12" href="/">"Ver mi tienda"</a>
                <button type="button" class="btn min-h-12" disabled=move || send.pending().get()
                    on:click=move |_| { send.dispatch(()); }>
                    {move || if send.pending().get() { "Enviando…" } else { "Enviarme el enlace del editor" }}
                </button>
                <div role="status" aria-live="polite" class="text-sm empty:hidden">
                    {move || send.value().get().map(|r| match r {
                        Ok(()) => view! { <p class="text-success">"Te enviamos el enlace a tu email. Abrilo desde una computadora."</p> }.into_any(),
                        Err(e) => {
                            let msg = e.to_string();
                            let msg = msg.rsplit_once(": ").map_or(msg.clone(), |(_, m)| m.to_string());
                            view! { <p class="text-error">{msg}</p> }.into_any()
                        }
                    })}
                </div>
                <div class="flex flex-wrap gap-2 border-t border-base-200 pt-3">
                    <a class="btn btn-ghost min-h-11" href=format!("{panel}/pedidos")>"Ver pedidos"</a>
                    <a class="btn btn-ghost min-h-11" href=format!("{panel}/productos")>"Ver productos"</a>
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
