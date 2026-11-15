//! Mis fotos en la plataforma (`/mis-fotos`): todas las fotos del dueño, con
//! cuánto pesan, dónde se usan, y para cambiarles el nombre o borrarlas.

use domain::{MediaItem, MyPhoto};
use leptos::prelude::*;

/// Las fotos de quien entró, con dónde se usa cada una. Vacío si no entró.
#[server]
pub async fn my_photos() -> Result<Option<Vec<MyPhoto>>, ServerFnError> {
    use crate::server::{internal, pool};
    let Some(user) = crate::auth::current_user().await? else {
        return Ok(None);
    };
    let pool = pool()?;
    let items = db::media::list(&pool, user.id).await.map_err(internal)?;
    let usage = db::media::usage(&pool, user.id).await.map_err(internal)?;
    Ok(Some(
        items
            .into_iter()
            .map(|item| {
                let used_in = usage
                    .iter()
                    .filter(|(id, _, _)| id.to_string() == item.id)
                    .map(|(_, store, page)| format!("{store} · {}", domain::page_label(page)))
                    .collect();
                MyPhoto { item, used_in }
            })
            .collect(),
    ))
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

/// Borra una foto de Mis fotos y sus archivos.
#[server]
pub async fn delete_photo(id: String) -> Result<(), ServerFnError> {
    use crate::server::{internal, pool};
    let user = crate::auth::current_user()
        .await?
        .ok_or_else(|| ServerFnError::new("Tenés que entrar con tu cuenta."))?;
    let id = uuid::Uuid::parse_str(&id).map_err(|_| ServerFnError::new("Esa foto no existe."))?;
    if db::media::delete(&pool()?, user.id, id)
        .await
        .map_err(internal)?
    {
        if let Some(store) = use_context::<media::Store>() {
            _ = store.delete_all(&id.to_string()).await;
        }
    }
    Ok(())
}

/// `/mis-fotos`.
#[component]
pub fn MyPhotosPage() -> impl IntoView {
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
        <div data-theme="light" class="min-h-screen bg-base-200 text-base-content">
            <header class="flex items-center gap-3 border-b border-base-300 bg-base-100 px-6 py-3">
                <a href="/" class="text-lg font-bold">"MarketMaker"</a>
            </header>
            <div class="mx-auto flex max-w-5xl flex-col gap-6 px-4 py-10">
                <div class="flex flex-wrap items-center gap-3">
                    <a href="/" class="btn btn-ghost btn-sm">"← Tus tiendas"</a>
                    <h1 class="grow text-3xl font-bold">"Mis fotos"</h1>
                    <label for="mis-fotos-subir" class="btn btn-primary" class:btn-disabled=move || uploads.progress.get().is_some()>
                        "Subir fotos"
                    </label>
                    <input id="mis-fotos-subir" type="file" class="sr-only" multiple
                        accept="image/jpeg,image/png,image/webp,image/gif"
                        on:change=move |ev| uploads.upload(editor::take_files(&ev), Callback::new(|_| ()))/>
                </div>
                <p class="text-base-content/80">
                    "Tus fotos sirven en todas tus tiendas. Las optimizamos al subirlas: tus clientes bajan una versión liviana, sin que se note la diferencia."
                </p>
                <div role="status" aria-live="polite" class="flex flex-col gap-1 text-sm">
                    {move || uploads.progress.get()}
                    {move || uploads.saved.get().into_iter().map(|t| view! { <span>{t}</span> }).collect_view()}
                </div>
                {move || {
                    let errors = uploads.upload_errors.get();
                    (!errors.is_empty()).then(|| view! {
                        <ul role="alert" class="alert alert-error flex flex-col items-start text-sm">
                            {errors.into_iter().map(|e| view! { <li>{e}</li> }).collect_view()}
                        </ul>
                    })
                }}
                <Suspense fallback=|| view! { <ui::Skeleton class="h-64 w-full"/> }>
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
                            Ok(Some(list)) if list.is_empty() => view! {
                                <div class="card bg-base-100 shadow-sm"><div class="card-body">
                                    <p>"Todavía no subiste fotos. Tocá Subir fotos para agregar las de tus productos."</p>
                                </div></div>
                            }.into_any(),
                            Ok(Some(list)) => {
                                let total = list.len();
                                view! {
                                    <p class="text-sm text-base-content/70">{format!("{total} de 500 fotos")}</p>
                                    <ul class="grid gap-4 sm:grid-cols-2 lg:grid-cols-3" aria-label="Tus fotos">
                                        {list.into_iter().map(|p| view! { <PhotoCard photo=p reload/> }).collect_view()}
                                    </ul>
                                }.into_any()
                            }
                        }
                    })}
                </Suspense>
            </div>
        </div>
    }
}

#[component]
fn PhotoCard(photo: MyPhoto, reload: RwSignal<u32>) -> impl IntoView {
    let MyPhoto { item, used_in } = photo;
    let MediaItem {
        id,
        url,
        name,
        width,
        height,
        original_bytes,
        web_bytes,
    } = item;
    let thumb = domain::media_base(&url)
        .map(|b| format!("{b}/480.webp"))
        .unwrap_or(url);
    let saved_name = RwSignal::new(name.clone());
    let draft = RwSignal::new(name);
    let error = RwSignal::new(None::<String>);
    let confirm = RwSignal::new(false);
    let rename =
        Action::new(|(id, name): &(String, String)| rename_photo(id.clone(), name.clone()));
    let remove = Action::new(|id: &String| delete_photo(id.clone()));
    Effect::new(move |_| match rename.value().get() {
        Some(Ok(())) => {
            saved_name.set(draft.get_untracked());
            error.set(None);
        }
        Some(Err(e)) => error.set(Some(clean(e))),
        None => {}
    });
    Effect::new(move |_| match remove.value().get() {
        Some(Ok(())) => reload.update(|n| *n += 1),
        Some(Err(e)) => error.set(Some(clean(e))),
        None => {}
    });
    let id_save = id.clone();
    let save = move || {
        let n = draft.get_untracked().trim().to_string();
        if n.is_empty() {
            draft.set(saved_name.get_untracked());
        } else if n != saved_name.get_untracked() {
            rename.dispatch((id_save.clone(), n));
        }
    };
    let save2 = save.clone();
    let input_id = format!("foto-{id}");
    let uses = used_in.len();
    let sizes = if original_bytes > 0 && web_bytes > 0 {
        format!(
            "{width} × {height} px · subiste {} · tus clientes bajan {}",
            domain::format_bytes(original_bytes),
            domain::format_bytes(web_bytes)
        )
    } else {
        format!("{width} × {height} px")
    };
    view! {
        <li class="card overflow-hidden bg-base-100 shadow-sm">
            <figure class="aspect-[4/3] bg-base-200">
                <img src=thumb alt=move || saved_name.get() loading="lazy" class="h-full w-full object-cover"/>
            </figure>
            <div class="card-body gap-2 p-4">
                <label for=input_id.clone() class="text-xs font-medium text-base-content/70">"Nombre"</label>
                <input id=input_id class="input input-sm w-full" maxlength="80" prop:value=move || draft.get()
                    on:input=move |ev| draft.set(event_target_value(&ev))
                    on:blur=move |_| save()
                    on:keydown=move |ev| if ev.key() == "Enter" { save2() }/>
                <p class="text-xs text-base-content/70">{sizes}</p>
                <p class="text-xs">
                    {if uses == 0 {
                        "No la usás en ninguna tienda.".to_string()
                    } else {
                        format!("Se usa en: {}", used_in.join(", "))
                    }}
                </p>
                {move || error.get().map(|e| view! { <p role="alert" class="text-xs text-error">{e}</p> })}
                <div class="card-actions mt-1 justify-end">
                    {move || {
                        let id = id.clone();
                        if confirm.get() {
                            let warn = if uses == 0 {
                                "¿Borrarla?".to_string()
                            } else {
                                format!("Se usa en {uses} {}. Esos bloques quedan sin imagen. ¿Borrarla igual?", if uses == 1 { "página" } else { "páginas" })
                            };
                            view! {
                                <span class="text-xs grow">{warn}</span>
                                <button class="btn btn-ghost btn-xs" on:click=move |_| confirm.set(false)>"No"</button>
                                <button class="btn btn-error btn-xs" disabled=move || remove.pending().get()
                                    on:click=move |_| { remove.dispatch(id.clone()); }>"Sí, borrar"</button>
                            }.into_any()
                        } else {
                            view! { <button class="btn btn-ghost btn-xs" on:click=move |_| confirm.set(true)>"Borrar"</button> }.into_any()
                        }
                    }}
                </div>
            </div>
        </li>
    }
}

/// El texto de un error de server function, sin el prefijo técnico.
fn clean(e: ServerFnError) -> String {
    let s = e.to_string();
    s.rsplit_once(": ")
        .map_or(s.clone(), |(_, m)| m.to_string())
}
