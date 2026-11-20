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
    view! {
        <leptos_meta::Title text="Datos de la tienda · MarketMaker"/>
            <div class="mx-auto flex max-w-3xl flex-col gap-5 px-4 py-8 max-sm:py-5 max-sm:pb-28">
                <Suspense fallback=|| view! { <ui::PanelSkeleton kind=ui::PanelSkeletonKind::StoreInfo/> }>
                    {move || Suspend::new(async move {
                        match data.await {
                            Ok(Some(form)) => view! { <StoreInfoEditor slug=slug.get_untracked() form/> }.into_any(),
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
    let dirty = move || saved.set(false);

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
    let submit = move || {
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
        name.set(info.name.clone());
        whatsapp.set(info.whatsapp.clone());
        instagram.set(info.instagram.clone());
        email.set(info.email.clone());
        tagline.set(info.tagline.clone());
        hours.set(info.hours.clone());
        save.dispatch(StoreInfoForm {
            info,
            logo_id: logo.get_untracked().map(|(id, _)| id),
        });
    };
    let busy = move || photos.progress.get().is_some();

    view! {
        <div>
            <h1 class="text-3xl font-bold max-sm:text-2xl">"Datos de la tienda"</h1>
            <p class="text-sm text-base-content/70">"El menú y el pie de tu tienda usan estos datos. Si los cambiás acá, se actualizan en todas tus páginas."</p>
        </div>
        <form class="card bg-base-100 shadow-sm" novalidate on:submit=move |ev| { ev.prevent_default(); submit(); }>
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
                    <button type="submit" class="btn btn-primary" disabled=move || save.pending().get() || busy()>
                        {move || if save.pending().get() { "Guardando…" } else { "Guardar" }}
                    </button>
                    {move || saved.get().then(|| view! { <FieldNote ok=true text="Guardamos los datos de tu tienda."/> })}
                    {error_for("form")}
                </div>
            </div>
        </form>
    }
}
