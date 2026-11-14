use leptos::prelude::*;
use leptos_meta::{provide_meta_context, MetaTags, Stylesheet, Title};
use leptos_router::{
    components::{Route, Router, Routes},
    path, SsrMode, StaticSegment,
};
use storefront::StorefrontPage;

mod dev;
pub mod server;

pub use server::*;

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
                    <Route path=StaticSegment("") view=HomePage ssr=SsrMode::Async/>
                    <Route path=path!("/dev/widgets") view=dev::DevWidgetsPage/>
                </Routes>
            </main>
        </Router>
    }
}

/// Marketplace en `/`: la última versión guardada de la página `home`.
#[component]
fn HomePage() -> impl IntoView {
    let page = Resource::new(|| (), |_| load_page("home".to_string()));
    view! {
        <Suspense fallback=|| view! { <div class="p-8">"Cargando…"</div> }>
            {move || Suspend::new(async move {
                match page.await {
                    Ok(view) => view! { <StorefrontPage view/> }.into_any(),
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
}
