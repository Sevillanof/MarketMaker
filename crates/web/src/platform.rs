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
fn Shell(
    /// Más ancho, para elegir la plantilla viéndola en grande.
    #[prop(optional)]
    wide: bool,
    children: Children,
) -> impl IntoView {
    view! {
        <div data-theme="light" class="min-h-screen bg-base-200 text-base-content">
            <header class="flex items-center gap-3 border-b border-base-300 bg-base-100 px-6 py-3">
                <a href="/" class="text-lg"><crate::account::Logo/></a>
            </header>
            <div class=if wide { "mx-auto flex max-w-5xl flex-col gap-6 px-4 py-10" } else { "mx-auto flex max-w-3xl flex-col gap-6 px-4 py-10" }>{children()}</div>
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
                                <Shell wide=true><FirstStore email=if me.name.trim().is_empty() { me.email } else { me.name }/></Shell>
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

    let template = RwSignal::new(domain::STORE_TEMPLATES[0].id.to_string());
    let create = Action::new(|(n, s, t): &(String, String, String)| {
        create_store(n.clone(), s.clone(), t.clone())
    });
    let opening = RwSignal::new(false);
    let busy = move || create.pending().get() || opening.get();
    Effect::new(move |_| match create.value().get() {
        Some(Ok(NewStoreOutcome::Created { open })) => {
            // Sigue diciendo "Creando tu tienda…" hasta que abre su Inicio.
            opening.set(true);
            go(&open)
        }
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
            create.dispatch((n, s, template.get_untracked()));
        }
    };
    let slug_note = move || {
        if let Some(e) = slug_error.get() {
            return Some(
                view! { <p id="nueva-tienda-direccion-error" class="text-sm text-error">{e}</p> }
                    .into_any(),
            );
        }
        match status.get()? {
            domain::SlugStatus::Free => {
                Some(view! { <p class="text-sm text-success">"✓ Está libre"</p> }.into_any())
            }
            domain::SlugStatus::Taken { suggestion } => Some(
                view! {
                    <p class="text-sm text-error">
                        "Ya la usa otra tienda. "
                        <button type="button" class="link link-primary" on:click=move |_| {
                            slug.set(suggestion.clone());
                            slug_touched.set(true);
                        }>{format!("Usar «{suggestion}»")}</button>
                    </p>
                }
                .into_any(),
            ),
            domain::SlugStatus::Problem(p) => {
                Some(view! { <p class="text-sm text-error">{p}</p> }.into_any())
            }
        }
    };
    view! {
        <form class="flex flex-col gap-4" novalidate on:submit=on_submit>
                <div>
                    <h2 id="crear-tienda-titulo" class="text-xl font-bold">{if first { "Tu tienda" } else { "Crear tienda" }}</h2>
                    <p class="text-sm text-base-content/70">"Arrancás con un diseño de ejemplo y lo cambiás cuando quieras."</p>
                </div>
                <div class="grid items-start gap-4 sm:grid-cols-2">
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
                    <label class="font-medium" for="nueva-tienda-direccion">"Dirección web"</label>
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
                            }
                            // Al salir del campo la deja lista: sin tildes,
                            // signos ni guiones de más ("Mi Tienda!!" → "mi-tienda").
                            on:blur=move |_| {
                                let now = slug.get_untracked();
                                let clean = domain::slug_from_name(&now);
                                if !now.is_empty() && clean != now {
                                    slug.set(clean);
                                }
                            }/>
                        <span class="join-item flex items-center border border-base-300 bg-base-200 px-3 font-mono text-sm">{suffix}</span>
                    </div>
                    <div aria-live="polite">{slug_note}</div>
                </div>
                </div>
                <fieldset class="flex flex-col gap-2">
                    <legend class="font-medium">"Diseño de partida"</legend>
                    <span id="nueva-tienda-diseno-ayuda" class="text-sm text-base-content/70">"Elegí el que más se parezca a lo que querés. Después cambiás todo."</span>
                    // A la izquierda las plantillas; a la derecha, la elegida en grande.
                    <div class="grid items-start gap-4 sm:grid-cols-[17rem_1fr]">
                        <div class="flex flex-col gap-2" aria-describedby="nueva-tienda-diseno-ayuda">
                            {domain::STORE_TEMPLATES.iter().map(|t| {
                                let id = t.id;
                                view! {
                                    <label class="mk-template-card">
                                        <input type="radio" name="template" value=id class="mk-template-input"
                                            prop:checked=move || template.get() == id
                                            on:change=move |_| template.set(id.to_string())/>
                                        <picture>
                                            <source srcset=format!("{}.avif", t.preview) type="image/avif"/>
                                            <img src=format!("{}.webp", t.preview) alt="" width="96" height="60" loading="lazy" class="mk-template-img"/>
                                        </picture>
                                        <span class="flex min-w-0 flex-col">
                                            <span class="font-semibold">{t.name}</span>
                                            <span class="text-xs text-base-content/70">{t.hint}</span>
                                        </span>
                                    </label>
                                }
                            }).collect_view()}
                        </div>
                        {move || {
                            let id = template.get();
                            let t = domain::STORE_TEMPLATES.iter().find(|t| t.id == id).unwrap_or(&domain::STORE_TEMPLATES[0]);
                            view! {
                                <figure class="overflow-hidden rounded-box border border-base-300 bg-base-200">
                                    <picture>
                                        <source srcset=format!("{}.avif", t.preview) type="image/avif"/>
                                        <img src=format!("{}.webp", t.preview) alt=format!("Vista previa de la plantilla {}", t.name)
                                            width="1280" height="800" class="block aspect-[8/5] w-full object-cover object-top"/>
                                    </picture>
                                    <figcaption class="border-t border-base-300 bg-base-100 px-3 py-2 text-sm">
                                        <b>{t.name}</b>" · Así arranca tu tienda. Los textos y las fotos son de ejemplo."
                                    </figcaption>
                                </figure>
                            }
                        }}
                    </div>
                </fieldset>
                {move || problem.get().map(|p| view! { <p role="alert" class="text-sm text-error">{p}</p> })}
                <div class="flex flex-wrap justify-end gap-2">
                    {on_cancel.map(|cancel| view! {
                        <button class="btn btn-ghost" type="button" on:click=move |_| cancel.run(())>"Cancelar"</button>
                    })}
                    <button class="btn btn-primary" type="submit" disabled=busy>
                        {move || if busy() {
                            view! { <span class="loading loading-spinner loading-sm"></span>"Creando tu tienda…" }.into_any()
                        } else {
                            view! { "Crear mi tienda" }.into_any()
                        }}
                    </button>
                </div>
                {move || busy().then(|| view! {
                    <p role="status" class="text-sm text-base-content/70">"Estamos preparando tu tienda y su dirección segura. Puede tardar unos segundos."</p>
                })}
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
    crate::not_found_status();
    view! {
        <leptos_meta::Title text="Página no encontrada · MarketMaker"/>
        <Shell>
            <div class="card bg-base-100 shadow-sm"><div class="card-body gap-3">
                <h1 class="card-title text-2xl">"Esta página no existe"</h1>
                <p>"Puede que la dirección esté mal escrita o que la página ya no exista."</p>
                <a class="btn btn-primary self-start" href="/">"Ir al inicio"</a>
            </div></div>
        </Shell>
    }
}

/// La dirección no es de ninguna tienda (`nadie.localhost:3000`): página
/// oscura con estantes vacíos y la dirección que se buscó.
#[component]
pub fn StoreNotFound() -> impl IntoView {
    crate::not_found_status();
    let host = crate::site::current_host();
    let (name, rest) = match host.split_once('.') {
        Some((n, r)) => (n.to_string(), format!(".{r}")),
        None => (host.clone(), String::new()),
    };
    view! {
        <leptos_meta::Title text="Tienda no encontrada · MarketMaker"/>
        <div data-theme="dark" class="flex min-h-screen flex-col bg-[#1f2330] text-white">
            <header class="px-6 py-4"><a href=crate::site::platform_url() class="text-lg"><crate::account::Logo/></a></header>
            <main class="flex flex-1 items-center justify-center px-4 py-10 text-center">
                <div class="flex max-w-md flex-col items-center gap-4">
                    <div class="flex gap-3" aria-hidden="true">
                        <span class="size-14 rounded-xl border-2 border-dashed border-[#565d78]"></span>
                        <span class="size-14 rounded-xl border-2 border-dashed border-[#565d78]"></span>
                        <span class="size-14 rounded-xl border-2 border-dashed border-[#565d78]"></span>
                    </div>
                    <p class="max-w-full break-all rounded-full border border-[#454c68] bg-[#2c3146] px-4 py-1 text-sm">
                        <strong class="font-semibold text-[#b4a9ff]">{name}</strong>{rest}
                    </p>
                    <h1 class="text-3xl font-extrabold tracking-tight sm:text-4xl">"Acá todavía no hay nada"</h1>
                    <p class="text-[#c3c8da]">"Esta tienda no existe o dejó de estar disponible. Revisá que la dirección esté bien escrita."</p>
                    <a class="btn btn-primary mt-2" href=crate::site::platform_url()>"Ir a MarketMaker"</a>
                </div>
            </main>
        </div>
    }
}

/// `/entrar/pase?t=…` en una tienda: usa el pase de un solo uso y lleva al
/// editor con la sesión abierta.
#[component]
pub fn PassPage() -> impl IntoView {
    // El pase se usa en el servidor, al armar la página: guarda la sesión y
    // redirige a /editor de una vez, sin esperar a que el navegador baje y
    // arranque todo el programa de la página (WASM) solo para canjearlo.
    let query = use_query_map();
    let token = query.read_untracked().get("t").unwrap_or_default();
    let pass = Resource::new(move || token.clone(), |t| use_pass(t));
    view! {
        <leptos_meta::Title text="Abriendo tu tienda · MarketMaker"/>
        <Shell>
            <div class="card bg-base-100 shadow-sm"><div class="card-body gap-3" role="status">
                <Suspense fallback=|| view! { <p>"Abriendo el editor de tu tienda…"</p> }>
                    {move || Suspend::new(async move {
                        match pass.await {
                            Ok(true) => view! {
                                <p class="flex items-center gap-2"><span class="loading loading-spinner loading-sm" aria-hidden="true"></span>"Abriendo el editor de tu tienda…"</p>
                                <leptos_router::components::Redirect path="/editor"/>
                            }.into_any(),
                            _ => view! {
                                <h1 class="card-title text-2xl">"Este enlace ya no sirve"</h1>
                                <p>"Los enlaces para abrir el editor duran un minuto y se usan una sola vez. Volvé a tus tiendas y tocá Editar de nuevo, o entrá con tu email."</p>
                                <div class="flex flex-wrap gap-2">
                                    <a class="btn btn-primary" href=crate::site::platform_url()>"Ir a tus tiendas"</a>
                                    <a class="btn btn-ghost" href="/entrar?volver=/editor">"Entrar con mi email"</a>
                                </div>
                            }.into_any(),
                        }
                    })}
                </Suspense>
            </div></div>
        </Shell>
    }
}
