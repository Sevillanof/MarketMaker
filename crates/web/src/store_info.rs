//! Datos de una tienda, en la plataforma (`/tienda/:slug/datos`): el nombre,
//! el logo y cómo contactarla. El menú y el pie del diseño los usan cuando
//! sus campos quedan vacíos.

use crate::account::FieldNote;
use crate::products::{error_text, slug_param, SignInFirst};
use domain::{StoreInfo, StoreInfoForm};
use leptos::prelude::*;

/// Los datos de la tienda `slug`. `None` si no entró con su cuenta.
#[server]
pub async fn load_store_info(slug: String) -> Result<Option<StoreInfoForm>, ServerFnError> {
    use crate::server::{internal, pool};
    let Some(tenant) = crate::products::server::owned_store(&slug).await? else {
        return Ok(None);
    };
    let pool = pool()?;
    let info = db::auth::store_info(&pool, tenant.id)
        .await
        .map_err(internal)?;
    let logo_id = db::auth::store_logo_id(&pool, tenant.id)
        .await
        .map_err(internal)?
        .map(|id| id.to_string());
    Ok(Some(StoreInfoForm { info, logo_id }))
}

/// Guarda los datos. Devuelve los problemas por campo (vacío si salió bien).
#[server(input = leptos::server_fn::codec::Json)]
pub async fn save_store_info(
    slug: String,
    form: StoreInfoForm,
) -> Result<Vec<(String, String)>, ServerFnError> {
    use crate::server::{internal, pool};
    let tenant = crate::products::server::require_store(&slug).await?;
    let user = crate::auth::current_user()
        .await?
        .ok_or_else(|| ServerFnError::new("Tenés que entrar con tu cuenta."))?;
    let mut info = form.info;
    let errors = domain::tidy_store_info(&mut info);
    if !errors.is_empty() {
        return Ok(errors
            .into_iter()
            .map(|(f, e)| (f.to_string(), e))
            .collect());
    }
    let logo = match form.logo_id.as_deref() {
        None | Some("") => None,
        Some(id) => {
            Some(uuid::Uuid::parse_str(id).map_err(|_| ServerFnError::new("Esa foto no existe."))?)
        }
    };
    let saved = db::auth::save_store_info(&pool()?, tenant.id, user.id, &info, logo)
        .await
        .map_err(internal)?;
    if !saved {
        return Ok(vec![(
            "logo".into(),
            "No encontramos esa foto en Mis fotos. Subila de nuevo.".into(),
        )]);
    }
    Ok(Vec::new())
}

/// `/tienda/:slug/datos`.
#[component]
pub fn StoreInfoPage() -> impl IntoView {
    if matches!(crate::site::current_site(), domain::Site::Store(_)) {
        return view! { <crate::Home/> }.into_any();
    }
    let slug = slug_param();
    let data = Resource::new(move || slug.get(), load_store_info);
    // Las dos se piden desde la página (no desde adentro del Suspend): un
    // recurso creado durante la hidratación no coincide con el del servidor.
    let admin = Resource::new(move || slug.get(), store_admin);
    let reminder = Resource::new(move || slug.get(), cart_reminder);
    view! {
        <leptos_meta::Title text="Datos de la tienda · MarketMaker"/>
            <div class="flex w-full max-w-[1440px] flex-col gap-5 px-8 max-sm:px-4 py-8 max-sm:py-5 max-sm:pb-28">
                <Suspense fallback=|| view! { <ui::PanelSkeleton kind=ui::PanelSkeletonKind::StoreInfo/> }>
                    {move || Suspend::new(async move {
                        match data.await {
                            Ok(Some(form)) => {
                                let name = form.info.name.clone();
                                let admin = admin.await.ok().flatten();
                                let reminder = reminder.await.ok().flatten();
                                view! {
                                    <StoreInfoEditor slug=slug.get_untracked() form/>
                                    {reminder.map(|on| view! { <BuyerNoticesCard slug=slug.get_untracked() on/> })}
                                    <crate::team::TeamCard slug=slug.get_untracked()/>
                                    {admin.map(|(rest, paused)| view! {
                                        <StoreAdminCard slug=slug.get_untracked() name rest paused/>
                                    })}
                                }.into_any()
                            }
                            Ok(None) => view! { <SignInFirst back=format!("/tienda/{}/datos", slug.get_untracked())/> }.into_any(),
                            Err(e) => view! { <p role="alert" class="text-sm text-red-700">{error_text(&e)}</p> }.into_any(),
                        }
                    })}
                </Suspense>
            </div>
    }
    .into_any()
}

const FIELD_IDS: &[(&str, &str)] = &[
    ("name", "s-name"),
    ("logo", "s-logo"),
    ("whatsapp", "s-whatsapp"),
    ("instagram", "s-instagram"),
    ("email", "s-email"),
    ("tagline", "s-tagline"),
    ("hours", "s-hours"),
];

/// Dónde aparece un dato en la tienda, al lado de su nombre.
fn seen_in(text: &'static str) -> impl IntoView {
    view! { <span class="mk-seen-in">"Se ve en: " {text}</span> }
}

#[component]
fn StoreInfoEditor(slug: String, form: StoreInfoForm) -> impl IntoView {
    let StoreInfoForm { info, logo_id } = form;
    let name = RwSignal::new(info.name);
    let whatsapp = RwSignal::new(info.whatsapp);
    let instagram = RwSignal::new(info.instagram);
    let email = RwSignal::new(info.email);
    let tagline = RwSignal::new(info.tagline);
    let hours = RwSignal::new(info.hours);
    // (id, dirección) del logo elegido.
    let logo = RwSignal::new(logo_id.map(|id| (id, info.logo_url)));
    let errors = RwSignal::new(Vec::<(String, String)>::new());
    let saved = RwSignal::new(false);
    let slug = StoredValue::new(slug);
    let photos = editor::MyPhotos::new();

    let error_for = move |field: &'static str| {
        move || {
            errors
                .get()
                .into_iter()
                .find(|(f, _)| f == field)
                .map(|(_, text)| view! { <FieldNote ok=false text/> })
        }
    };
    let invalid = move |field: &'static str| move || errors.get().iter().any(|(f, _)| f == field);

    let save =
        Action::new(move |form: &StoreInfoForm| save_store_info(slug.get_value(), form.clone()));
    Effect::new(move |_| match save.value().get() {
        Some(Ok(e)) => {
            saved.set(e.is_empty());
            if let Some(id) = e
                .first()
                .and_then(|(f, _)| FIELD_IDS.iter().find(|(k, _)| k == f))
                .map(|(_, id)| *id)
            {
                crate::platform::focus_id(id);
            }
            errors.set(e);
        }
        Some(Err(e)) => errors.set(vec![("form".into(), error_text(&e))]),
        None => {}
    });
    let submit = move |tidy_fields: bool| {
        saved.set(false);
        errors.set(Vec::new());
        let mut info = StoreInfo {
            name: name.get_untracked(),
            logo_url: String::new(),
            whatsapp: whatsapp.get_untracked(),
            instagram: instagram.get_untracked(),
            email: email.get_untracked(),
            tagline: tagline.get_untracked(),
            hours: hours.get_untracked(),
            ..StoreInfo::default()
        };
        // Se ve como queda guardado (por ejemplo, Instagram sin la dirección).
        let _ = domain::tidy_store_info(&mut info);
        // Al guardar solo mientras se escribe no se tocan los campos: el
        // cursor saltaría.
        if tidy_fields {
            name.set(info.name.clone());
            whatsapp.set(info.whatsapp.clone());
            instagram.set(info.instagram.clone());
            email.set(info.email.clone());
            tagline.set(info.tagline.clone());
            hours.set(info.hours.clone());
        }
        save.dispatch(StoreInfoForm {
            info,
            logo_id: logo.get_untracked().map(|(id, _)| id),
        });
    };
    let busy = move || photos.progress.get().is_some();
    // Cada cambio se guarda solo cuando se deja de tipear un momento (igual
    // que Cobros y Entregas).
    let tick = RwSignal::new(0u32);
    let dirty = move || {
        saved.set(false);
        tick.update(|t| *t = t.wrapping_add(1));
        let n = tick.get_untracked();
        set_timeout(
            move || {
                if tick.get_untracked() == n && photos.progress.get_untracked().is_none() {
                    submit(false);
                }
            },
            std::time::Duration::from_millis(800),
        );
    };

    view! {
        <div>
            <h1 class="text-3xl font-bold max-sm:text-2xl">"Datos de la tienda"</h1>
            <p class="text-sm text-base-content/70">"El menú y el pie de tu tienda usan estos datos. Si los cambiás acá, se actualizan en todas tus páginas."</p>
        </div>
        <form class="card bg-base-100 shadow-sm" novalidate on:submit=move |ev| { ev.prevent_default(); submit(true); }>
            <div class="card-body gap-5">
                <fieldset class="fieldset">
                    <label class="fieldset-legend" for="s-name">"Nombre de la tienda" {seen_in("menú, pie y pestaña")}</label>
                    <input id="s-name" class="input w-full" autocomplete="organization"
                        maxlength=domain::STORE_NAME_MAX.to_string()
                        aria-invalid=move || invalid("name")().to_string()
                        prop:value=move || name.get()
                        on:input=move |ev| { name.set(event_target_value(&ev)); dirty(); }/>
                    {error_for("name")}
                </fieldset>

                <div class="flex flex-col gap-2">
                    <span class="flex items-center text-sm font-semibold">"Logo" {seen_in("menú y pie")}</span>
                    <div class="flex flex-wrap items-center gap-4">
                        <label for="s-logo" class="group relative grid size-20 shrink-0 cursor-pointer place-items-center overflow-hidden rounded-box border border-base-300 bg-base-200 has-[:focus-visible]:outline-2 has-[:focus-visible]:outline-offset-2 has-[:focus-visible]:outline-primary">
                            {move || match logo.get() {
                                Some((_, url)) => view! { <img class="size-full object-contain p-1" src=url alt="Logo de la tienda"/> }.into_any(),
                                None => view! { <ui::Icon class="size-7 text-base-content/50" name="photo"/> }.into_any(),
                            }}
                            {move || busy().then(|| view! {
                                <span class="absolute inset-0 grid place-items-center bg-black/45 text-white">
                                    <span class="loading loading-spinner loading-sm"></span>
                                </span>
                            })}
                            <input id="s-logo" type="file" class="sr-only"
                                aria-label=move || if logo.get().is_some() { "Cambiar el logo" } else { "Subir un logo" }
                                accept="image/svg+xml,image/png,image/webp,image/jpeg"
                                disabled=busy
                                on:change=move |ev| {
                                    errors.update(|e| e.retain(|(f, _)| f != "logo"));
                                    photos.upload(
                                        editor::take_files(&ev).into_iter().take(1).collect(),
                                        Callback::new(move |item: domain::MediaItem| {
                                            logo.set(Some((item.id.clone(), domain::avatar_url(&item.id))));
                                            dirty();
                                        }),
                                    )
                                }/>
                        </label>
                        <div class="flex flex-col items-start gap-1 text-sm">
                            <label for="s-logo" class="btn btn-sm">
                                {move || if logo.get().is_some() { "Cambiar logo" } else { "Subir logo" }}
                            </label>
                            {move || logo.get().is_some().then(|| view! {
                                <button type="button" class="btn btn-ghost btn-sm text-base-content/70"
                                    on:click=move |_| { logo.set(None); dirty(); }>"Quitar logo"</button>
                            })}
                            <span class="text-base-content/60">"SVG o PNG con fondo transparente quedan mejor."</span>
                        </div>
                    </div>
                    {move || {
                        let upload = photos.upload_errors.get();
                        upload.into_iter().next().map(|text| view! { <FieldNote ok=false text/> })
                    }}
                    {error_for("logo")}
                </div>

                <div class="grid gap-x-4 sm:grid-cols-2">
                    <fieldset class="fieldset">
                        <label class="fieldset-legend" for="s-whatsapp">"WhatsApp " <span class="font-normal text-base-content/60">"(opcional)"</span> {seen_in("pie y Contacto")}</label>
                        <input id="s-whatsapp" class="input w-full" type="tel" inputmode="tel" autocomplete="tel-national"
                            placeholder="11 5555-1234"
                            aria-invalid=move || invalid("whatsapp")().to_string()
                            prop:value=move || whatsapp.get()
                            on:input=move |ev| { whatsapp.set(event_target_value(&ev)); dirty(); }/>
                        {error_for("whatsapp")}
                    </fieldset>
                    <fieldset class="fieldset">
                        <label class="fieldset-legend" for="s-instagram">"Instagram " <span class="font-normal text-base-content/60">"(opcional)"</span> {seen_in("pie y Contacto")}</label>
                        <label class="input w-full"><span class="text-base-content/60">"@"</span>
                            <input id="s-instagram" autocapitalize="none" spellcheck="false" placeholder="lahuerta"
                                aria-invalid=move || invalid("instagram")().to_string()
                                prop:value=move || instagram.get()
                                on:input=move |ev| { instagram.set(event_target_value(&ev)); dirty(); }/>
                        </label>
                        {error_for("instagram")}
                    </fieldset>
                    <fieldset class="fieldset sm:col-span-2">
                        <label class="fieldset-legend" for="s-email">"Email de contacto " <span class="font-normal text-base-content/60">"(opcional)"</span> {seen_in("pie y Contacto")}</label>
                        <input id="s-email" class="input w-full" type="email" autocomplete="email" placeholder="hola@lahuerta.com"
                            aria-invalid=move || invalid("email")().to_string()
                            prop:value=move || email.get()
                            on:input=move |ev| { email.set(event_target_value(&ev)); dirty(); }/>
                        {error_for("email")}
                    </fieldset>
                </div>
                <div class="-mt-2 grid gap-x-4 sm:grid-cols-2">
                    <fieldset class="fieldset">
                        <label class="fieldset-legend" for="s-tagline">"Descripción corta " <span class="font-normal text-base-content/60">"(opcional)"</span> {seen_in("pie")}</label>
                        <textarea id="s-tagline" class="textarea w-full" rows="2"
                            maxlength=domain::STORE_TAGLINE_MAX.to_string()
                            placeholder="Ej.: Macetas de barro pintadas a mano en Rosario."
                            aria-invalid=move || invalid("tagline")().to_string()
                            prop:value=move || tagline.get()
                            on:input=move |ev| { tagline.set(event_target_value(&ev)); dirty(); }></textarea>
                        {error_for("tagline")}
                    </fieldset>
                    <fieldset class="fieldset content-start">
                        <label class="fieldset-legend" for="s-hours">"Horarios de atención " <span class="font-normal text-base-content/60">"(opcional)"</span> {seen_in("pie y Contacto")}</label>
                        <input id="s-hours" class="input w-full"
                            maxlength=domain::STORE_HOURS_MAX.to_string()
                            placeholder="Ej.: Lun a Vie de 10 a 18 h"
                            aria-invalid=move || invalid("hours")().to_string()
                            prop:value=move || hours.get()
                            on:input=move |ev| { hours.set(event_target_value(&ev)); dirty(); }/>
                        {error_for("hours")}
                    </fieldset>
                </div>

                // Vista previa del pie con lo que se está escribiendo.
                <section aria-labelledby="s-preview" class="flex flex-col gap-2">
                    <h2 id="s-preview" class="text-xs font-semibold tracking-wide text-base-content/60 uppercase">"Vista previa del pie"</h2>
                    <div class="mk-foot-preview grid items-center gap-x-6 gap-y-3 rounded-box border border-base-300 px-6 py-5 sm:grid-cols-[minmax(0,1fr)_auto]">
                        <div class="flex min-w-0 items-center gap-3">
                            {move || logo.get().map(|(_, url)| view! {
                                <img src=url alt="" class="h-8 w-auto max-w-32 min-w-8 shrink-0 object-contain object-left"/>
                            })}
                            <p class="truncate text-lg font-bold">{move || {
                                let n = name.get();
                                if n.trim().is_empty() { "Nombre de tu tienda".to_string() } else { n }
                            }}</p>
                        </div>
                        <p class="flex flex-wrap gap-x-4 text-sm text-base-content/80">
                            {move || (!whatsapp.get().trim().is_empty()).then_some(view! { <span>"WhatsApp"</span> })}
                            {move || (!instagram.get().trim().is_empty()).then_some(view! { <span>"Instagram"</span> })}
                            {move || (!email.get().trim().is_empty()).then_some(view! { <span>"Email"</span> })}
                        </p>
                        <p class="text-sm text-base-content/70 sm:col-span-2">{move || {
                            let n = name.get();
                            [format!("© {}", n.trim()), tagline.get().trim().to_string(), hours.get().trim().to_string()]
                                .into_iter()
                                .filter(|t| !t.is_empty() && t != "© ")
                                .collect::<Vec<_>>()
                                .join(" · ")
                        }}</p>
                    </div>
                </section>

                <div class="mk-savebar flex flex-wrap items-center gap-3">
                    <p class="text-sm text-base-content/70" role="status">
                        {move || if save.pending().get() { "Guardando…" } else if saved.get() { "Guardado" } else { "Los cambios se guardan solos." }}
                    </p>
                    {error_for("form")}
                </div>
            </div>
        </form>
    }
}

/// Dirección y pausa de la tienda `slug`, para la sección "Tu tienda".
/// ¿La tienda recuerda las compras sin terminar? `None` si quien mira no es
/// dueño ni administrador.
#[server]
pub async fn cart_reminder(slug: String) -> Result<Option<bool>, ServerFnError> {
    use crate::server::{internal, pool};
    let Some(tenant) = crate::products::server::owned_store_min(&slug, "admin").await? else {
        return Ok(None);
    };
    Ok(Some(
        db::orders::cart_reminder(&pool()?, tenant.id)
            .await
            .map_err(internal)?,
    ))
}

#[server]
pub async fn set_cart_reminder(slug: String, on: bool) -> Result<(), ServerFnError> {
    let tenant = crate::products::server::require_store_min(&slug, "admin").await?;
    db::orders::set_cart_reminder(&crate::server::pool()?, tenant.id, on)
        .await
        .map_err(crate::server::internal)
}

/// «Avisos a compradores»: el recordatorio de compras sin terminar.
#[component]
fn BuyerNoticesCard(slug: String, on: bool) -> impl IntoView {
    let slug = StoredValue::new(slug);
    let on_now = RwSignal::new(on);
    let problem = RwSignal::new(None::<String>);
    let toggle = move |value: bool| {
        on_now.set(value);
        problem.set(None);
        leptos::task::spawn_local(async move {
            if let Err(e) = set_cart_reminder(slug.get_value(), value).await {
                on_now.set(!value);
                problem.set(Some(error_text(&e)));
            }
        });
    };
    view! {
        <section class="card border border-base-300 bg-base-100" aria-labelledby="avisos-compradores">
            <div class="card-body gap-3 p-5">
                <h2 id="avisos-compradores" class="text-lg font-semibold">"Avisos a compradores"</h2>
                <label class="flex cursor-pointer items-start gap-3">
                    <input type="checkbox" class="toggle toggle-primary mt-0.5" id="recordar-carritos"
                        aria-describedby="recordar-carritos-ayuda"
                        prop:checked=move || on_now.get()
                        on:change=move |ev| toggle(event_target_checked(&ev))/>
                    <span>
                        <span class="block font-medium">"Recordar compras sin terminar"</span>
                        <span id="recordar-carritos-ayuda" class="block text-sm text-base-content/70">
                            "Enviamos un único email a quien dejó su correo y no completó el pago. Incluye un enlace para retomar su carrito."
                        </span>
                    </span>
                </label>
                {move || problem.get().map(|e| view! { <p role="alert" class="text-sm text-red-700">{e}</p> })}
            </div>
        </section>
    }
}

#[server]
pub async fn store_admin(slug: String) -> Result<Option<(String, bool)>, ServerFnError> {
    use crate::server::{internal, pool};
    let Some(tenant) = crate::products::server::owned_store(&slug).await? else {
        return Ok(None);
    };
    // Pausar, cambiar la dirección y borrar son solo del dueño.
    let me = crate::auth::current_user().await?;
    let is_owner = match me {
        Some(u) => db::auth::role_in(&pool()?, tenant.id, u.id)
            .await
            .map_err(internal)?
            .as_deref()
            == Some("owner"),
        None => false,
    };
    if !is_owner {
        return Ok(None);
    }
    let paused = db::auth::store_paused(&pool()?, tenant.id)
        .await
        .map_err(internal)?;
    let url = crate::email::store_url(&tenant.slug);
    let host = url.split_once("://").map_or(url.as_str(), |(_, h)| h);
    let rest = host.split_once('.').map_or("", |(_, r)| r).to_string();
    Ok(Some((rest, paused)))
}

#[server]
pub async fn pause_store(slug: String, paused: bool) -> Result<(), ServerFnError> {
    let tenant = crate::products::server::require_store_min(&slug, "owner").await?;
    db::auth::set_store_paused(&crate::server::pool()?, tenant.id, paused)
        .await
        .map_err(crate::server::internal)
}

/// Cambia la dirección. `Ok(Err(problema))` si no se puede usar.
#[server]
pub async fn change_store_slug(
    slug: String,
    new_slug: String,
) -> Result<Result<String, String>, ServerFnError> {
    let tenant = crate::products::server::require_store_min(&slug, "owner").await?;
    let new_slug = new_slug.trim().to_lowercase();
    if new_slug == tenant.slug {
        return Ok(Ok(new_slug));
    }
    if let Some(problem) = domain::store_slug_problem(&new_slug) {
        return Ok(Err(problem.to_string()));
    }
    let pool = crate::server::pool()?;
    if !db::auth::rename_store(&pool, tenant.id, &new_slug)
        .await
        .map_err(crate::server::internal)?
    {
        return Ok(Err("Esa dirección ya la usa otra tienda.".into()));
    }
    Ok(Ok(new_slug))
}

/// Borra la tienda. Hay que escribir su nombre tal cual.
#[server]
pub async fn delete_store(
    slug: String,
    typed: String,
) -> Result<Result<(), String>, ServerFnError> {
    let tenant = crate::products::server::require_store_min(&slug, "owner").await?;
    let norm = |s: &str| {
        s.split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
            .to_lowercase()
    };
    if norm(&typed) != norm(&tenant.name) {
        return Ok(Err("El nombre no coincide.".into()));
    }
    db::auth::delete_store(&crate::server::pool()?, tenant.id)
        .await
        .map_err(crate::server::internal)?;
    Ok(Ok(()))
}

/// "Tu tienda", al final de Datos de la tienda: cambiar la dirección,
/// pausarla y borrarla.
#[component]
fn StoreAdminCard(slug: String, name: String, rest: String, paused: bool) -> impl IntoView {
    let slug = StoredValue::new(slug);
    let name = StoredValue::new(name);
    let rest = StoredValue::new(rest);
    let paused = RwSignal::new(paused);
    let editing = RwSignal::new(false);
    let new_slug = RwSignal::new(slug.get_value());
    let slug_error = RwSignal::new(None::<String>);
    let pause_error = RwSignal::new(None::<String>);
    let rename = Action::new(move |s: &String| {
        let s = s.clone();
        async move { change_store_slug(slug.get_value(), s).await }
    });
    Effect::new(move |_| match rename.value().get() {
        Some(Ok(Ok(s))) => {
            if let Some(w) = leptos::web_sys::window() {
                _ = w.location().set_href(&format!("/tienda/{s}/datos"));
            }
        }
        Some(Ok(Err(p))) => slug_error.set(Some(p)),
        Some(Err(e)) => slug_error.set(Some(error_text(&e))),
        None => {}
    });
    let toggle = move |on: bool| {
        paused.set(on);
        pause_error.set(None);
        leptos::task::spawn_local(async move {
            if let Err(e) = pause_store(slug.get_value(), on).await {
                paused.set(!on);
                pause_error.set(Some(error_text(&e)));
            }
        });
    };
    let dialog = NodeRef::<leptos::html::Dialog>::new();
    let typed = RwSignal::new(String::new());
    let delete_error = RwSignal::new(None::<String>);
    let delete = Action::new(move |t: &String| {
        let t = t.clone();
        async move { delete_store(slug.get_value(), t).await }
    });
    Effect::new(move |_| match delete.value().get() {
        Some(Ok(Ok(()))) => {
            if let Some(w) = leptos::web_sys::window() {
                _ = w.location().set_href("/");
            }
        }
        Some(Ok(Err(p))) => delete_error.set(Some(p)),
        Some(Err(e)) => delete_error.set(Some(error_text(&e))),
        None => {}
    });
    view! {
        <section class="card border border-base-300 bg-base-100" aria-labelledby="tu-tienda">
            <div class="card-body gap-0 p-5">
                <h2 id="tu-tienda" class="mb-2 text-lg font-semibold">"Tu tienda"</h2>
                <div class="flex flex-wrap items-center gap-3 border-b border-base-200 py-3">
                    <div class="min-w-0 grow">
                        <p class="font-medium">"Dirección"</p>
                        <Show when=move || !editing.get() fallback=move || view! {
                            <form class="mt-2 flex flex-wrap items-center gap-2"
                                on:submit=move |ev| { ev.prevent_default(); slug_error.set(None); rename.dispatch(new_slug.get_untracked()); }>
                                <label class="input w-auto">
                                    <input aria-label="Nueva dirección" class="w-40" autocomplete="off" spellcheck="false"
                                        prop:value=move || new_slug.get()
                                        on:input=move |ev| new_slug.set(event_target_value(&ev))/>
                                    <span class="text-base-content/60">{format!(".{}", rest.get_value())}</span>
                                </label>
                                <button class="btn btn-primary btn-sm" disabled=move || rename.pending().get()>"Guardar"</button>
                                <button type="button" class="btn btn-ghost btn-sm" on:click=move |_| { editing.set(false); slug_error.set(None); new_slug.set(slug.get_value()); }>"Cancelar"</button>
                                <p class="basis-full text-xs text-base-content/60">"El enlace anterior deja de funcionar."</p>
                                {move || slug_error.get().map(|e| view! { <p class="basis-full text-sm text-red-700" role="alert">{e}</p> })}
                            </form>
                        }>
                            <p class="truncate text-sm text-base-content/70"><b class="text-base-content">{slug.get_value()}</b>{format!(".{}", rest.get_value())}</p>
                        </Show>
                    </div>
                    <Show when=move || !editing.get()>
                        <button type="button" class="btn btn-sm" on:click=move |_| editing.set(true)>"Cambiar"</button>
                    </Show>
                </div>
                <label class="flex cursor-pointer items-center gap-3 border-b border-base-200 py-3">
                    <span class="min-w-0 grow">
                        <span class="block font-medium">"Pausar la tienda"</span>
                        <span class="block text-sm text-base-content/70">"Quien entra ve «Volvemos pronto» y no puede comprar. Vos la seguís viendo."</span>
                        {move || pause_error.get().map(|e| view! { <span class="block text-sm text-red-700" role="alert">{e}</span> })}
                    </span>
                    <input type="checkbox" class="toggle toggle-primary"
                        prop:checked=move || paused.get()
                        on:change=move |ev| toggle(event_target_checked(&ev))/>
                </label>
                <div class="flex flex-wrap items-center gap-3 pt-3">
                    <div class="min-w-0 grow">
                        <p class="font-medium">"Borrar la tienda"</p>
                        <p class="text-sm text-base-content/70">"Se borran el diseño, los productos y los pedidos. Tus fotos quedan en Mis fotos."</p>
                    </div>
                    <button type="button" class="btn btn-sm btn-outline btn-error"
                        on:click=move |_| { typed.set(String::new()); delete_error.set(None); if let Some(d) = dialog.get() { _ = d.show_modal(); } }>"Borrar…"</button>
                </div>
            </div>
        </section>
        <dialog node_ref=dialog class="modal" aria-labelledby="borrar-tienda">
            <div class="modal-box max-w-md">
                <h2 id="borrar-tienda" class="text-lg font-bold">"¿Borrar la tienda?"</h2>
                <p class="mt-1 text-sm text-base-content/70">"No se puede deshacer. Escribí "<b>{name.get_value()}</b>" para confirmar."</p>
                <form class="mt-4 flex flex-col gap-3"
                    on:submit=move |ev| { ev.prevent_default(); delete_error.set(None); delete.dispatch(typed.get_untracked()); }>
                    <input class="input w-full" aria-label="Nombre de la tienda" autocomplete="off"
                        prop:value=move || typed.get()
                        on:input=move |ev| typed.set(event_target_value(&ev))/>
                    {move || delete_error.get().map(|e| view! { <p class="text-sm text-red-700" role="alert">{e}</p> })}
                    <div class="flex justify-end gap-2">
                        <button type="button" class="btn btn-ghost" autofocus
                            on:click=move |_| if let Some(d) = dialog.get() { d.close(); }>"Cancelar"</button>
                        <button class="btn btn-error" disabled=move || delete.pending().get() || typed.with(|t| t.trim().is_empty())>"Borrar la tienda"</button>
                    </div>
                </form>
            </div>
            <form method="dialog" class="modal-backdrop"><button aria-label="Cerrar">"cerrar"</button></form>
        </dialog>
    }
}
