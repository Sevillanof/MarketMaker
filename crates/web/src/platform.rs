//! La plataforma (`localhost:3000`): entrar, "Tus tiendas" y crear una
//! tienda. Y las pantallas de paso entre la plataforma y cada tienda.

use crate::auth::{sign_out, who_am_i};
use crate::stores::{create_store, my_stores, open_store, use_pass};
use domain::{NewStoreOutcome, StoreCard};
use leptos::prelude::*;
use leptos_router::hooks::use_query_map;

/// Va a `url` recargando la página.
fn go(url: &str) {
    let _ = window().location().set_href(url);
}

#[component]
fn Shell(children: Children) -> impl IntoView {
    view! {
        <div data-theme="light" class="min-h-screen bg-base-200 text-base-content">
            <header class="flex items-center gap-3 border-b border-base-300 bg-base-100 px-6 py-3">
                <a href="/" class="text-lg font-bold">"MarketMaker"</a>
            </header>
            <div class="mx-auto flex max-w-3xl flex-col gap-6 px-4 py-10">{children()}</div>
        </div>
    }
}

/// `localhost:3000/`: sin cuenta, una invitación a entrar; con cuenta, tus
/// tiendas y el formulario para crear una.
#[component]
pub fn PlatformHome() -> impl IntoView {
    let me = Resource::new(|| (), |_| who_am_i());
    let stores = Resource::new(|| (), |_| my_stores());
    view! {
        <leptos_meta::Title text="Tus tiendas · MarketMaker"/>
        <Shell>
            <Suspense fallback=|| view! { <ui::Skeleton class="h-48 w-full"/> }>
                {move || Suspend::new(async move {
                    match me.await.ok().flatten() {
                        None => view! { <Welcome/> }.into_any(),
                        Some(me) => {
                            let stores = stores.await.unwrap_or_default();
                            view! { <Dashboard email=me.email stores/> }.into_any()
                        }
                    }
                })}
            </Suspense>
        </Shell>
    }
}

#[component]
fn Welcome() -> impl IntoView {
    let demo = format!(
        "{}://{}",
        crate::site::scheme(),
        domain::store_host(&crate::site::current_host(), "demo")
    );
    view! {
        <div class="card bg-base-100 shadow-sm">
            <div class="card-body gap-4">
                <h1 class="card-title text-3xl">"Armá tu tienda online"</h1>
                <p class="text-base-content/80">"Entrá con tu email para ver tus tiendas o crear una nueva. Por ahora MarketMaker es solo por invitación."</p>
                <div class="flex flex-wrap gap-2">
                    <a class="btn btn-primary" href="/entrar?volver=/">"Entrar"</a>
                    <a class="btn btn-ghost" href=demo>"Ver una tienda de ejemplo"</a>
                </div>
            </div>
        </div>
    }
}

#[component]
fn Dashboard(email: String, stores: Vec<StoreCard>) -> impl IntoView {
    let out = Action::new(|_: &()| sign_out());
    Effect::new(move |_| {
        if let Some(Ok(())) = out.value().get() {
            go("/");
        }
    });
    let open = Action::new(|slug: &String| open_store(slug.clone()));
    let open_error = RwSignal::new(None::<String>);
    Effect::new(move |_| match open.value().get() {
        Some(Ok(url)) => go(&url),
        Some(Err(_)) => open_error.set(Some(
            "No pudimos abrir el editor. Recargá la página y probá de nuevo.".into(),
        )),
        None => {}
    });
    let empty = stores.is_empty();
    view! {
        <div class="flex flex-wrap items-center gap-3">
            <h1 class="grow text-3xl font-bold">{if empty { "Creá tu primera tienda" } else { "Tus tiendas" }}</h1>
            <a class="btn btn-outline btn-sm" href="/mis-fotos">"Mis fotos"</a>
            <span class="text-sm text-base-content/70">{email}</span>
            <button class="btn btn-ghost btn-sm" on:click=move |_| { out.dispatch(()); }>"Salir"</button>
        </div>
        {move || open_error.get().map(|e| view! { <div role="alert" class="alert alert-error text-sm">{e}</div> })}
        <ul class="grid gap-4 sm:grid-cols-2" aria-label="Tus tiendas">
            {stores.into_iter().map(|s| {
                let slug = s.slug.clone();
                let address = s.url.split_once("://").map_or(s.url.clone(), |(_, a)| a.to_string());
                view! {
                    <li class="card bg-base-100 shadow-sm">
                        <div class="card-body gap-2">
                            <h2 class="card-title">{s.name.clone()}</h2>
                            <p class="font-mono text-sm text-base-content/70">{address}</p>
                            <div class="card-actions mt-2">
                                <button class="btn btn-primary btn-sm" disabled=move || open.pending().get()
                                    on:click=move |_| { open.dispatch(slug.clone()); }>"Editar"</button>
                                <a class="btn btn-outline btn-sm" href=format!("/tienda/{}/productos", s.slug)>"Productos"</a>
                                <a class="btn btn-ghost btn-sm" href=s.url.clone() target="_blank" rel="noopener">"Ver tienda"</a>
                            </div>
                        </div>
                    </li>
                }
            }).collect_view()}
        </ul>
        <NewStore first=empty/>
    }
}

/// Formulario para crear una tienda: nombre y dirección (sugerida a partir
/// del nombre hasta que la persona la cambia).
#[component]
fn NewStore(first: bool) -> impl IntoView {
    let name = RwSignal::new(String::new());
    let slug = RwSignal::new(String::new());
    let slug_touched = RwSignal::new(false);
    let problem = RwSignal::new(None::<String>);
    let suffix = format!(".{}", domain::platform_host(&crate::site::current_host()));
    let create = Action::new(|(n, s): &(String, String)| create_store(n.clone(), s.clone()));
    Effect::new(move |_| match create.value().get() {
        Some(Ok(NewStoreOutcome::Created { open })) => go(&open),
        Some(Ok(NewStoreOutcome::Problem(p))) => problem.set(Some(p)),
        Some(Ok(NewStoreOutcome::Taken { suggestion })) => {
            problem.set(Some(format!(
                "Esa dirección ya la usa otra tienda. Te sugerimos «{suggestion}», que está libre."
            )));
            slug.set(suggestion);
            slug_touched.set(true);
        }
        Some(Err(_)) => problem.set(Some(
            "No pudimos crear la tienda. Revisá tu conexión y probá de nuevo.".into(),
        )),
        None => {}
    });
    let on_submit = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        problem.set(None);
        create.dispatch((name.get_untracked(), slug.get_untracked()));
    };
    view! {
        <form class="card bg-base-100 shadow-sm" on:submit=on_submit>
            <div class="card-body gap-4">
                <h2 class="card-title">{if first { "Tu tienda" } else { "Crear otra tienda" }}</h2>
                <label class="flex flex-col gap-1">
                    <span class="font-medium">"Nombre de la tienda"</span>
                    <input class="input w-full" name="name" required maxlength="60" placeholder="La Casa de Ana"
                        prop:value=move || name.get()
                        on:input=move |ev| {
                            let v = event_target_value(&ev);
                            if !slug_touched.get_untracked() {
                                slug.set(domain::slug_from_name(&v));
                            }
                            name.set(v);
                        }/>
                </label>
                <label class="flex flex-col gap-1">
                    <span class="font-medium">"Dirección"</span>
                    <span class="text-sm text-base-content/70">"Así la van a encontrar tus clientes. Letras sin tilde, números y guiones."</span>
                    <div class="join w-full">
                        <input class="input join-item w-full font-mono" name="slug" required maxlength="30" placeholder="lacasadeana"
                            autocapitalize="off" spellcheck="false"
                            prop:value=move || slug.get()
                            on:input=move |ev| {
                                slug_touched.set(true);
                                slug.set(event_target_value(&ev).to_lowercase());
                            }/>
                        <span class="join-item flex items-center border border-base-300 bg-base-200 px-3 font-mono text-sm">{suffix}</span>
                    </div>
                </label>
                {move || problem.get().map(|p| view! { <div role="alert" class="alert alert-error text-sm">{p}</div> })}
                <button class="btn btn-primary self-start" type="submit" disabled=move || create.pending().get()>
                    {move || if create.pending().get() { "Creando…" } else { "Crear tienda" }}
                </button>
            </div>
        </form>
    }
}

/// Una dirección de tienda en la plataforma (`localhost:3000/ofertas`).
#[component]
pub fn NotAStore() -> impl IntoView {
    view! {
        <Shell>
            <div class="card bg-base-100 shadow-sm"><div class="card-body gap-3">
                <h1 class="card-title text-2xl">"Esta página no existe"</h1>
                <p>"Las tiendas tienen su propia dirección, por ejemplo demo.localhost:3000."</p>
                <a class="btn btn-primary self-start" href="/">"Ir a tus tiendas"</a>
            </div></div>
        </Shell>
    }
}

/// La dirección no es de ninguna tienda (`nadie.localhost:3000`).
#[component]
pub fn StoreNotFound() -> impl IntoView {
    view! {
        <Shell>
            <div class="card bg-base-100 shadow-sm"><div class="card-body gap-3">
                <h1 class="card-title text-2xl">"No encontramos esta tienda"</h1>
                <p>"No hay ninguna tienda en esta dirección. Revisá que esté bien escrita."</p>
                <a class="btn btn-primary self-start" href=crate::site::platform_url()>"Ir a MarketMaker"</a>
            </div></div>
        </Shell>
    }
}

/// `/entrar/pase?t=…` en una tienda: usa el pase de un solo uso y lleva al
/// editor con la sesión abierta.
#[component]
pub fn PassPage() -> impl IntoView {
    let query = use_query_map();
    let pass = Action::new(|t: &String| use_pass(t.clone()));
    Effect::new(move |_| {
        // Solo en el navegador, una vez.
        let token = query.read_untracked().get("t").unwrap_or_default();
        pass.dispatch(token);
    });
    Effect::new(move |_| {
        if let Some(Ok(true)) = pass.value().get() {
            go("/editor");
        }
    });
    view! {
        <leptos_meta::Title text="Abriendo tu tienda · MarketMaker"/>
        <Shell>
            <div class="card bg-base-100 shadow-sm"><div class="card-body gap-3" role="status">
                {move || match pass.value().get() {
                    None | Some(Ok(true)) => view! {
                        <p class="flex items-center gap-2"><span class="loading loading-spinner loading-sm" aria-hidden="true"></span>"Abriendo el editor de tu tienda…"</p>
                    }.into_any(),
                    _ => view! {
                        <h1 class="card-title text-2xl">"Este enlace ya no sirve"</h1>
                        <p>"Los enlaces para abrir el editor duran un minuto y se usan una sola vez. Volvé a tus tiendas y tocá Editar de nuevo, o entrá con tu email."</p>
                        <div class="flex flex-wrap gap-2">
                            <a class="btn btn-primary" href=crate::site::platform_url()>"Ir a tus tiendas"</a>
                            <a class="btn btn-ghost" href="/entrar?volver=/editor">"Entrar con mi email"</a>
                        </div>
                    }.into_any(),
                }}
            </div></div>
        </Shell>
    }
}
