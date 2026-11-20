//! Mis fotos en la plataforma (`/mis-fotos`): todas las fotos del dueño, con
//! cuánto pesan, dónde se usan, y para cambiarles el nombre o borrarlas.

use domain::{MediaItem, MyLibrary, MyPhoto};
use leptos::prelude::*;

/// Las fotos de quien entró, con dónde se usa cada una. Vacío si no entró.
#[server]
pub async fn my_photos() -> Result<Option<MyLibrary>, ServerFnError> {
    use crate::server::{internal, pool};
    let Some(user) = crate::auth::current_user().await? else {
        return Ok(None);
    };
    let pool = pool()?;
    let items = db::media::list(&pool, user.id).await.map_err(internal)?;
    let usage = db::media::usage(&pool, user.id).await.map_err(internal)?;
    let quota = db::media::quota(&pool, user.id).await.map_err(internal)?;
    let photos = items
        .into_iter()
        .map(|item| {
            let used_in = usage
                .iter()
                .filter(|(id, _, _)| id.to_string() == item.id)
                .map(|(_, store, page)| format!("{store} · {}", domain::page_label(page)))
                .collect();
            MyPhoto { item, used_in }
        })
        .collect();
    Ok(Some(MyLibrary { photos, quota }))
}

/// Cambia el nombre de una foto.
#[server]
pub async fn rename_photo(id: String, name: String) -> Result<(), ServerFnError> {
    use crate::server::{internal, pool};
    let user = crate::auth::current_user()
        .await?
        .ok_or_else(|| ServerFnError::new("Tenés que entrar con tu cuenta."))?;
    let name = domain::media_name(&format!("{}.x", name.trim()));
    let id = uuid::Uuid::parse_str(&id).map_err(|_| ServerFnError::new("Esa foto no existe."))?;
    if !db::media::rename(&pool()?, user.id, id, &name)
        .await
        .map_err(internal)?
    {
        return Err(ServerFnError::new("Esa foto ya no está en Mis fotos."));
    }
    Ok(())
}

/// Cambia la descripción de una foto (qué se ve en ella).
#[server]
pub async fn set_photo_alt(id: String, alt: String) -> Result<(), ServerFnError> {
    use crate::server::{internal, pool};
    let user = crate::auth::current_user()
        .await?
        .ok_or_else(|| ServerFnError::new("Tenés que entrar con tu cuenta."))?;
    let alt: String = alt
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(domain::PHOTO_ALT_MAX)
        .collect();
    let id = uuid::Uuid::parse_str(&id).map_err(|_| ServerFnError::new("Esa foto no existe."))?;
    if !db::media::set_alt(&pool()?, user.id, id, &alt)
        .await
        .map_err(internal)?
    {
        return Err(ServerFnError::new("Esa foto ya no está en Mis fotos."));
    }
    Ok(())
}

/// Borra una foto de Mis fotos y sus archivos.
#[server]
pub async fn delete_photo(id: String) -> Result<(), ServerFnError> {
    use crate::server::{internal, pool};
    let user = crate::auth::current_user()
        .await?
        .ok_or_else(|| ServerFnError::new("Tenés que entrar con tu cuenta."))?;
    let id = uuid::Uuid::parse_str(&id).map_err(|_| ServerFnError::new("Esa foto no existe."))?;
    if let Some(folder) = db::media::delete(&pool()?, user.id, id)
        .await
        .map_err(internal)?
    {
        if let Some(store) = use_context::<media::Store>() {
            _ = store.delete_all(&folder).await;
        }
    }
    Ok(())
}

/// `/mis-fotos`.
#[component]
pub fn MyPhotosPage() -> impl IntoView {
    // La tienda que queda elegida en el menú (`?tienda=`); sin ella, la más nueva.
    let store = leptos_router::hooks::use_query_map()
        .read_untracked()
        .get("tienda")
        .unwrap_or_default();
    let reload = RwSignal::new(0u32);
    let photos = Resource::new(move || reload.get(), |_| my_photos());
    let uploads = editor::MyPhotos::new();
    // Al terminar de subir, se vuelve a pedir la lista (con dónde se usan).
    Effect::new(move |prev: Option<bool>| {
        let busy = uploads.progress.get().is_some();
        if prev == Some(true) && !busy {
            reload.update(|n| *n += 1);
        }
        busy
    });
    view! {
        <leptos_meta::Title text="Mis fotos · MarketMaker"/>
        <crate::panel::StoreShell slug=store section=crate::panel::Section::Photos>
            <div class="mx-auto flex max-w-5xl flex-col gap-6 px-4 py-10 max-sm:gap-4 max-sm:py-5 max-sm:pb-24">
                <div class="flex flex-wrap items-center gap-3">
                    <h1 class="grow text-3xl font-bold max-sm:text-2xl">"Mis fotos"</h1>
                    // En el teléfono es el botón flotante de abajo (ver `.mk-fab`).
                    <label for="mis-fotos-subir" class="btn btn-primary mk-fab" class:btn-disabled=move || uploads.progress.get().is_some()>
                        "Subir fotos"
                    </label>
                    <input id="mis-fotos-subir" type="file" class="sr-only" multiple
                        accept="image/jpeg,image/png,image/webp,image/gif,image/svg+xml"
                        on:change=move |ev| uploads.upload(editor::take_files(&ev), Callback::new(|_| ()))/>
                    <editor::LinkImport photos=uploads done=Callback::new(|_| ()) uid="mis-fotos"/>
                </div>
                <p class="text-base-content/80">
                    "Tus fotos sirven en todas tus tiendas. Tocá una para cambiarle el nombre o la descripción."
                </p>
                <div role="status" aria-live="polite" class="flex flex-col gap-1 text-sm">
                    {move || uploads.progress.get()}
                    {move || uploads.saved.get().into_iter().map(|t| view! { <span>{t}</span> }).collect_view()}
                </div>
                {move || {
                    let errors = uploads.upload_errors.get();
                    (!errors.is_empty()).then(|| view! {
                        <ul role="alert" class="flex flex-col gap-0.5 text-sm text-error">
                            {errors.into_iter().map(|e| view! { <li>{e}</li> }).collect_view()}
                        </ul>
                    })
                }}
                <Suspense fallback=|| view! { <ui::PanelSkeleton kind=ui::PanelSkeletonKind::Photos/> }>
                    {move || Suspend::new(async move {
                        match photos.await {
                            Ok(None) => view! {
                                <div class="card bg-base-100 shadow-sm"><div class="card-body">
                                    <p>"Entrá con tu cuenta para ver tus fotos."</p>
                                    <a class="btn btn-primary self-start" href="/entrar?volver=/mis-fotos">"Entrar"</a>
                                </div></div>
                            }.into_any(),
                            Err(_) => view! {
                                <div role="alert" class="alert alert-error">"No pudimos traer tus fotos. Recargá la página."</div>
                            }.into_any(),
                            Ok(Some(MyLibrary { photos, quota })) => {
                                let empty = photos.is_empty();
                                view! {
                                    <QuotaBar quota/>
                                    {if empty {
                                        view! {
                                            <div class="card bg-base-100 shadow-sm"><div class="card-body">
                                                <p>"Todavía no subiste fotos. Tocá Subir fotos para agregar las de tus productos."</p>
                                            </div></div>
                                        }.into_any()
                                    } else {
                                        view! { <PhotoGrid photos reload/> }.into_any()
                                    }}
                                }.into_any()
                            }
                        }
                    })}
                </Suspense>
            </div>
        </crate::panel::StoreShell>
    }
}

/// Las fotos en una grilla compacta (foto, nombre y si se usa). Al tocar una,
/// sus datos se editan en el panel de al lado (abajo, en el teléfono).
#[component]
fn PhotoGrid(photos: Vec<MyPhoto>, reload: RwSignal<u32>) -> impl IntoView {
    // Cada foto con su nombre, que se ve en la grilla y se cambia en el panel.
    let photos: Vec<(MyPhoto, RwSignal<String>)> = photos
        .into_iter()
        .map(|p| {
            let name = RwSignal::new(p.item.name.clone());
            (p, name)
        })
        .collect();
    let all = StoredValue::new(photos.clone());
    let selected = RwSignal::new(None::<String>);
    view! {
        <div class="grid items-start gap-5" class=("lg:grid-cols-[1fr_20rem]", move || selected.get().is_some())>
            <ul class="grid grid-cols-2 gap-3.5 sm:grid-cols-3 lg:grid-cols-4" aria-label="Tus fotos">
                {photos.into_iter().map(|(p, name)| {
                    let id = p.item.id.clone();
                    let id2 = id.clone();
                    let on = Memo::new(move |_| selected.get().as_deref() == Some(id.as_str()));
                    let thumb = domain::media_base(&p.item.url)
                        .map(|b| format!("{b}/480.webp"))
                        .unwrap_or(p.item.url.clone());
                    let svg = p.item.url.ends_with(".svg");
                    let used = !p.used_in.is_empty();
                    view! {
                        <li>
                            <button type="button" class="mk-photo-tile" aria-pressed=move || on.get().to_string()
                                on:click=move |_| selected.set((!on.get_untracked()).then(|| id2.clone()))>
                                <span class="mk-photo-tile-img">
                                    // Un logo o una foto muy apaisada se ve entera, no recortada.
                                    <img src=thumb alt="" loading="lazy" class=if svg { "object-contain p-3" } else { "object-cover" }
                                        onload="if(this.naturalWidth>this.naturalHeight*1.6)this.className='object-contain p-2'"/>
                                </span>
                                <span class="block px-2.5 py-2">
                                    <span class="block truncate text-sm font-semibold">{move || name.get()}</span>
                                    <span class=if used { "block text-xs text-emerald-700" } else { "block text-xs text-base-content/60" }>
                                        {if used { "En uso" } else { "Sin usar" }}
                                    </span>
                                </span>
                            </button>
                        </li>
                    }
                }).collect_view()}
            </ul>
            {move || selected.get().and_then(|id| all.with_value(|a| a.iter().find(|(p, _)| p.item.id == id).cloned()))
                .map(|(photo, name)| view! { <PhotoDetail photo name reload selected/> })}
        </div>
    }
}

/// Los datos de la foto elegida: nombre, descripción, dónde se usa y borrar.
#[component]
fn PhotoDetail(
    photo: MyPhoto,
    name: RwSignal<String>,
    reload: RwSignal<u32>,
    selected: RwSignal<Option<String>>,
) -> impl IntoView {
    let MyPhoto { item, used_in } = photo;
    let MediaItem { id, url, alt, .. } = item;
    let preview = domain::media_base(&url)
        .map(|b| format!("{b}/480.webp"))
        .unwrap_or(url.clone());
    let svg = url.ends_with(".svg");
    let draft = RwSignal::new(name.get_untracked());
    let error = RwSignal::new(None::<String>);
    let confirm = RwSignal::new(false);
    let rename =
        Action::new(|(id, name): &(String, String)| rename_photo(id.clone(), name.clone()));
    let remove = Action::new(|id: &String| delete_photo(id.clone()));
    let saved_alt = RwSignal::new(alt.clone());
    let alt_draft = RwSignal::new(alt);
    let alt_action =
        Action::new(|(id, alt): &(String, String)| set_photo_alt(id.clone(), alt.clone()));
    Effect::new(move |_| match alt_action.value().get() {
        Some(Ok(())) => {
            saved_alt.set(alt_draft.get_untracked().trim().to_string());
            error.set(None);
        }
        Some(Err(e)) => error.set(Some(clean(e))),
        None => {}
    });
    let id_alt = id.clone();
    let save_alt = move || {
        let a = alt_draft.get_untracked().trim().to_string();
        if a != saved_alt.get_untracked() {
            alt_action.dispatch((id_alt.clone(), a));
        }
    };
    Effect::new(move |_| match rename.value().get() {
        Some(Ok(())) => {
            name.set(draft.get_untracked().trim().to_string());
            error.set(None);
        }
        Some(Err(e)) => error.set(Some(clean(e))),
        None => {}
    });
    Effect::new(move |_| match remove.value().get() {
        Some(Ok(())) => {
            selected.set(None);
            reload.update(|n| *n += 1);
        }
        Some(Err(e)) => error.set(Some(clean(e))),
        None => {}
    });
    let id_save = id.clone();
    let save = move || {
        let n = draft.get_untracked().trim().to_string();
        if n.is_empty() {
            draft.set(name.get_untracked());
        } else if n != name.get_untracked() {
            rename.dispatch((id_save.clone(), n));
        }
    };
    let save2 = save.clone();
    let save3 = save.clone();
    let save_alt2 = save_alt.clone();
    let input_id = format!("foto-{id}");
    let alt_id = format!("foto-{id}-alt");
    let alt_help = format!("foto-{id}-alt-ayuda");
    let uses = used_in.len();
    view! {
        // Computadora: al costado de la grilla. Teléfono: una hoja abajo.
        <aside aria-label="Datos de la foto" class="mk-photo-detail flex flex-col gap-2 rounded-box border border-base-300 bg-base-100 p-4">
            <img src=preview alt="" class=if svg { "mk-photo-preview p-4" } else { "mk-photo-preview" }/>
            <label for=input_id.clone() class="mt-1 text-sm font-semibold">"Nombre"</label>
            <input id=input_id class="input input-sm w-full" maxlength="80" prop:value=move || draft.get()
                on:input=move |ev| draft.set(event_target_value(&ev))
                on:blur=move |_| save()
                on:keydown=move |ev| if ev.key() == "Enter" { save2() }/>
            <label for=alt_id.clone() class="mt-1 text-sm font-semibold">"Qué se ve en la foto"</label>
            <textarea id=alt_id class="textarea textarea-sm w-full" rows="2" maxlength=domain::PHOTO_ALT_MAX
                aria-describedby=alt_help.clone()
                placeholder="Ej.: Maceta de barro con flores, vista de frente"
                prop:value=move || alt_draft.get()
                on:input=move |ev| alt_draft.set(event_target_value(&ev))
                on:blur=move |_| save_alt()></textarea>
            <p id=alt_help class="text-xs text-base-content/60">
                {move || if saved_alt.with(String::is_empty) {
                    "Ayuda a que Google la encuentre y a quien no ve la pantalla."
                } else {
                    "Guardado. Se usa en los bloques nuevos donde la pongas."
                }}
            </p>
            <p class="mt-1 text-sm font-semibold">"Dónde se usa"</p>
            {if uses == 0 {
                view! { <p class="text-sm text-base-content/70">"Todavía no la usás en ninguna tienda."</p> }.into_any()
            } else {
                view! { <ul class="text-sm">{used_in.into_iter().map(|u| view! { <li>{u}</li> }).collect_view()}</ul> }.into_any()
            }}
            {move || error.get().map(|e| view! { <p role="alert" class="text-xs text-error">{e}</p> })}
            <div class="mt-2 flex flex-wrap items-center gap-2">
                {move || {
                    let id = id.clone();
                    if confirm.get() {
                        let warn = if uses == 0 {
                            "¿Borrarla?".to_string()
                        } else {
                            format!("Se usa en {uses} {}. Esos bloques quedan sin imagen. ¿Borrarla igual?", if uses == 1 { "lugar" } else { "lugares" })
                        };
                        view! {
                            <span class="w-full text-sm">{warn}</span>
                            <button type="button" class="btn btn-ghost btn-sm" on:click=move |_| confirm.set(false)>"No, dejarla"</button>
                            <button type="button" class="btn btn-error btn-sm" disabled=move || remove.pending().get()
                                on:click=move |_| { remove.dispatch(id.clone()); }>"Sí, borrar"</button>
                        }.into_any()
                    } else {
                        view! {
                            <button type="button" class="btn btn-ghost btn-sm grow-0 text-red-700" on:click=move |_| confirm.set(true)>"Borrar foto"</button>
                        }.into_any()
                    }
                }}
                <button type="button" class="btn btn-primary btn-sm ml-auto" on:click=move |_| {
                    save3();
                    save_alt2();
                    selected.set(None);
                }>"Listo"</button>
            </div>
        </aside>
    }
}

/// Cuánto usa de Mis fotos y cuánto le queda.
#[component]
fn QuotaBar(quota: domain::PhotoQuota) -> impl IntoView {
    let pct = quota.percent();
    let class = if pct >= 90 {
        "progress progress-error w-full"
    } else if pct >= 75 {
        "progress progress-warning w-full"
    } else {
        "progress progress-primary w-full"
    };
    view! {
        <div class="card bg-base-100 shadow-sm">
            <div class="card-body gap-2 p-4">
                <div class="flex flex-wrap items-baseline justify-between gap-2">
                    <span class="font-medium">{quota.text()}</span>
                    <span class="text-sm text-base-content/70">{quota.left_text()}</span>
                </div>
                <progress class=class value=pct.to_string() max="100" aria-label="Espacio usado de Mis fotos"></progress>
                {(pct >= 90).then(|| view! {
                    <p class="text-sm text-error">"Te queda poco lugar. Borrá las fotos que no uses para poder subir otras."</p>
                })}
            </div>
        </div>
    }
}

/// El texto de un error de server function, sin el prefijo técnico.
fn clean(e: ServerFnError) -> String {
    let s = e.to_string();
    s.rsplit_once(": ")
        .map_or(s.clone(), |(_, m)| m.to_string())
}
