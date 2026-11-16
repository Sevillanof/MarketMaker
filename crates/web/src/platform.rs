//! La plataforma (`localhost:3000`): entrar, "Tus tiendas" y crear una
//! tienda. Y las pantallas de paso entre la plataforma y cada tienda.

use crate::auth::{sign_out, who_am_i};
use crate::stores::{create_store, my_stores, use_pass};
use domain::NewStoreOutcome;
use leptos::prelude::*;
use leptos_router::{components::Redirect, hooks::use_query_map};

/// Va a `url` recargando la página.
pub(crate) fn go(url: &str) {
    let _ = window().location().set_href(url);
}

#[component]
fn Shell(children: Children) -> impl IntoView {
    view! {
        <div data-theme="light" class="min-h-screen bg-base-200 text-base-content">
            <header class="flex items-center gap-3 border-b border-base-300 bg-base-100 px-6 py-3">
                <a href="/" class="text-lg"><crate::account::Logo/></a>
            </header>
            <div class="mx-auto flex max-w-3xl flex-col gap-6 px-4 py-10">{children()}</div>
        </div>
    }
}

/// `localhost:3000/`: sin cuenta, la página de inicio de MarketMaker; con
/// cuenta, el panel de su tienda más nueva (o crear la primera).
#[component]
pub fn PlatformHome() -> impl IntoView {
    let me = Resource::new(|| (), |_| who_am_i());
    let stores = Resource::new(|| (), |_| my_stores());
    view! {
        <Suspense fallback=|| view! { <ui::PageSkeleton/> }>
            {move || Suspend::new(async move {
                match me.await.ok().flatten() {
                    None => view! {
                        <leptos_meta::Title text="MarketMaker · Armá tu tienda online"/>
                        <crate::landing::Landing/>
                    }.into_any(),
                    Some(me) => {
                        let stores = stores.await.unwrap_or_default();
                        match stores.first() {
                            // Con tiendas, el panel arranca en la más nueva.
                            Some(s) => view! { <Redirect path=format!("/tienda/{}", s.slug)/> }.into_any(),
                            None => view! {
                                <leptos_meta::Title text="Tu primera tienda · MarketMaker"/>
                                <Shell><FirstStore email=me.email/></Shell>
                            }.into_any(),
                        }
                    }
                }
            })}
        </Suspense>
    }
}

/// Con cuenta y sin tiendas: crear la primera.
#[component]
fn FirstStore(email: String) -> impl IntoView {
    let out = Action::new(|_: &()| sign_out());
    Effect::new(move |_| {
        if let Some(Ok(())) = out.value().get() {
            go("/");
        }
    });
    view! {
        <div class="flex flex-wrap items-center gap-3">
            <h1 class="grow text-3xl font-bold">"Creá tu primera tienda"</h1>
            <a class="text-sm text-base-content/70 hover:text-base-content" href="/cuenta" title="Mi cuenta">{email}</a>
            <button class="btn btn-ghost btn-sm" on:click=move |_| { out.dispatch(()); }>"Salir"</button>
        </div>
        <div class="card bg-base-100 shadow-sm"><div class="card-body">
            <NewStore first=true/>
        </div></div>
    }
}

/// Formulario para crear una tienda: nombre y dirección (sugerida a partir
/// del nombre hasta que la persona la cambia).
#[component]
pub(crate) fn NewStore(
    first: bool,
    /// En una ventana: muestra "Cancelar" y la cierra.
    #[prop(optional)]
    on_cancel: Option<Callback<()>>,
) -> impl IntoView {
    let name = RwSignal::new(String::new());
    let slug = RwSignal::new(String::new());
    let slug_touched = RwSignal::new(false);
    // Errores chicos debajo de cada campo; `problem` es para lo que no es de
    // un campo (la conexión).
    let name_error = RwSignal::new(None::<String>);
    let slug_error = RwSignal::new(None::<String>);
    let problem = RwSignal::new(None::<String>);
    let suffix = format!(".{}", domain::platform_host(&crate::site::current_host()));

    // Mientras se escribe la dirección: libre o tomada, un rato después de
    // la última tecla.
    let status = RwSignal::new(None::<domain::SlugStatus>);
    let check = Action::new(|s: &String| crate::stores::slug_status(s.clone()));
    Effect::new(move |_| {
        let s = slug.get();
        status.set(None);
        if s.len() < 3 {
            return;
        }
        set_timeout(
            move || {
                if slug.get_untracked() == s {
                    check.dispatch(s);
                }
            },
            std::time::Duration::from_millis(400),
        );
    });
    Effect::new(move |_| {
        if let Some(Ok(st)) = check.value().get() {
            if check.input().get_untracked().is_none() {
                status.set(Some(st));
            }
        }
    });

    let create = Action::new(|(n, s): &(String, String)| create_store(n.clone(), s.clone()));
    Effect::new(move |_| match create.value().get() {
        Some(Ok(NewStoreOutcome::Created { open })) => go(&open),
        Some(Ok(NewStoreOutcome::Problem(p))) => {
            if p.contains("nombre") {
                name_error.set(Some(p));
                focus_id("nueva-tienda-nombre");
            } else {
                slug_error.set(Some(p));
                focus_id("nueva-tienda-direccion");
            }
        }
        Some(Ok(NewStoreOutcome::Taken { suggestion })) => {
            slug_error.set(Some(format!(
                "Esa dirección ya la usa otra tienda. Te sugerimos «{suggestion}», que está libre."
            )));
            slug.set(suggestion);
            slug_touched.set(true);
            focus_id("nueva-tienda-direccion");
        }
        Some(Err(_)) => problem.set(Some(
            "No pudimos crear la tienda. Revisá tu conexión y probá de nuevo.".into(),
        )),
        None => {}
    });
    let on_submit = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        problem.set(None);
        let n = name.get_untracked().trim().to_string();
        let s = slug.get_untracked().trim().to_lowercase();
        let n_err = if n.is_empty() {
            Some("Escribí el nombre de tu tienda.".to_string())
        } else if n.chars().count() > 60 {
            Some("El nombre puede tener hasta 60 letras.".to_string())
        } else {
            None
        };
        let s_err = domain::store_slug_problem(&s).map(str::to_string);
        name_error.set(n_err.clone());
        slug_error.set(s_err.clone());
        if n_err.is_some() {
            focus_id("nueva-tienda-nombre");
        } else if s_err.is_some() {
            focus_id("nueva-tienda-direccion");
        } else {
            create.dispatch((n, s));
        }
    };
    let slug_note = move || {
        if let Some(e) = slug_error.get() {
            return Some(view! { <p id="nueva-tienda-direccion-error" class="text-sm text-error">{e}</p> }.into_any());
        }
        match status.get()? {
            domain::SlugStatus::Free => Some(view! { <p class="text-sm text-success">"✓ Está libre"</p> }.into_any()),
            domain::SlugStatus::Taken { suggestion } => Some(view! {
                <p class="text-sm text-error">
                    "Ya la usa otra tienda. "
                    <button type="button" class="link link-primary" on:click=move |_| {
                        slug.set(suggestion.clone());
                        slug_touched.set(true);
                    }>{format!("Usar «{suggestion}»")}</button>
                </p>
            }.into_any()),
            domain::SlugStatus::Problem(p) => Some(view! { <p class="text-sm text-error">{p}</p> }.into_any()),
        }
    };
    view! {
        <form class="flex flex-col gap-4" novalidate on:submit=on_submit>
                <div>
                    <h2 id="crear-tienda-titulo" class="text-xl font-bold">{if first { "Tu tienda" } else { "Crear tienda" }}</h2>
                    <p class="text-sm text-base-content/70">"Arrancás con un diseño de ejemplo y lo cambiás cuando quieras."</p>
                </div>
                <div class="flex flex-col gap-1">
                    <label class="font-medium" for="nueva-tienda-nombre">"Nombre de la tienda"</label>
                    <input id="nueva-tienda-nombre" class="input w-full" class:input-error=move || name_error.get().is_some()
                        name="name" maxlength="60" placeholder="La Casa de Ana"
                        aria-invalid=move || name_error.get().is_some().to_string()
                        aria-describedby=move || name_error.get().is_some().then_some("nueva-tienda-nombre-error")
                        prop:value=move || name.get()
                        on:input=move |ev| {
                            let v = event_target_value(&ev);
                            if !slug_touched.get_untracked() {
                                slug.set(domain::slug_from_name(&v));
                                slug_error.set(None);
                            }
                            name.set(v);
                            name_error.set(None);
                        }/>
                    {move || name_error.get().map(|e| view! { <p id="nueva-tienda-nombre-error" class="text-sm text-error">{e}</p> })}
                </div>
                <div class="flex flex-col gap-1">
                    <label class="font-medium" for="nueva-tienda-direccion">"Dirección"</label>
                    <span id="nueva-tienda-direccion-ayuda" class="text-sm text-base-content/70">"Así la van a encontrar tus clientes. Letras sin tilde, números y guiones."</span>
                    <div class="join w-full">
                        <input id="nueva-tienda-direccion" class="input join-item w-full font-mono" class:input-error=move || slug_error.get().is_some()
                            name="slug" maxlength="30" placeholder="la-casa-de-ana"
                            autocapitalize="off" spellcheck="false"
                            aria-invalid=move || slug_error.get().is_some().to_string()
                            aria-describedby=move || if slug_error.get().is_some() {
                                "nueva-tienda-direccion-ayuda nueva-tienda-direccion-error"
                            } else {
                                "nueva-tienda-direccion-ayuda"
                            }
                            prop:value=move || slug.get()
                            on:input=move |ev| {
                                slug_touched.set(true);
                                slug.set(event_target_value(&ev).to_lowercase().replace(' ', "-"));
                                slug_error.set(None);
                            }/>
                        <span class="join-item flex items-center border border-base-300 bg-base-200 px-3 font-mono text-sm">{suffix}</span>
                    </div>
                    <div aria-live="polite">{slug_note}</div>
                </div>
                {move || problem.get().map(|p| view! { <p role="alert" class="text-sm text-error">{p}</p> })}
                <div class="flex flex-wrap justify-end gap-2">
                    {on_cancel.map(|cancel| view! {
                        <button class="btn btn-ghost" type="button" on:click=move |_| cancel.run(())>"Cancelar"</button>
                    })}
                    <button class="btn btn-primary" type="submit" disabled=move || create.pending().get()>
                        {move || if create.pending().get() { "Creando tu tienda…" } else { "Crear y empezar a editar" }}
                    </button>
                </div>
        </form>
    }
}

/// Lleva el foco a un campo (después de que se dibuje su error).
pub(crate) fn focus_id(id: &'static str) {
    request_animation_frame(move || {
        use leptos::wasm_bindgen::JsCast;
        if let Some(el) = document()
            .get_element_by_id(id)
            .and_then(|e| e.dyn_into::<leptos::web_sys::HtmlElement>().ok())
        {
            _ = el.focus();
        }
    });
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
