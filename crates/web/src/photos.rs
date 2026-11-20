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

/// Cuánto usa de Mis fotos (se pide de nuevo al subir o borrar fotos).
#[server]
pub async fn photo_quota() -> Result<Option<domain::PhotoQuota>, ServerFnError> {
    use crate::server::{internal, pool};
    let Some(user) = crate::auth::current_user().await? else {
        return Ok(None);
    };
    Ok(Some(
        db::media::quota(&pool()?, user.id).await.map_err(internal)?,
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
    let photos = Resource::new(|| (), |_| my_photos());
    let uploads = editor::MyPhotos::new();
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
                    "Tus fotos sirven en todas tus tiendas. Las optimizamos al subirlas: tus clientes bajan una versión liviana, sin que se note la diferencia."
                </p>
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
                                view! {
                                    <QuotaBar quota shared=uploads/>
                                    <PhotoGrid initial=photos shared=uploads/>
                                }.into_any()
                            }
                        }
                    })}
                </Suspense>
            </div>
        </crate::panel::StoreShell>
    }
}

/// La grilla de fotos. Las fotos nuevas que llegan de la bandeja de subidas
/// se suman al principio (con su `view-transition-name`, para que la miniatura
/// de la bandeja vuele hasta acá).
#[component]
fn PhotoGrid(initial: Vec<MyPhoto>, shared: editor::MyPhotos) -> impl IntoView {
    // Esta página ya trae la lista completa: la comparte con el resto de la app.
    shared
        .items
        .set(Some(initial.iter().map(|p| p.item.clone()).collect()));
    let list = RwSignal::new(
        initial
            .into_iter()
            .map(|p| (p, false))
            .collect::<Vec<_>>(),
    );
    Effect::new(move |_| {
        let Some(items) = shared.items.get() else {
            return;
        };
        list.update(|l| {
            let fresh: Vec<MediaItem> = items
                .into_iter()
                .filter(|i| !l.iter().any(|(p, _)| p.item.id == i.id))
                .collect();
            // `items` viene con la más nueva primero.
            for item in fresh.into_iter().rev() {
                l.insert(0, (MyPhoto { item, used_in: Vec::new() }, true));
            }
        });
    });
    let removed = Callback::new(move |id: String| {
        list.update(|l| l.retain(|(p, _)| p.item.id != id));
        shared.items.update(|l| {
            if let Some(l) = l {
                l.retain(|p| p.id != id);
            }
        });
    });
    view! {
        <div class="card bg-base-100 shadow-sm" class:hidden=move || !list.with(Vec::is_empty)>
            <div class="card-body">
                <p>"Todavía no subiste fotos. Tocá Subir fotos para agregar las de tus productos."</p>
            </div>
        </div>
        <ul class="grid gap-4 max-sm:grid-cols-2 max-sm:gap-2.5 sm:grid-cols-2 lg:grid-cols-3" aria-label="Tus fotos"
            class:hidden=move || list.with(Vec::is_empty)>
            <For each=move || list.get() key=|(p, _)| p.item.id.clone()
                children=move |(p, fresh)| view! { <PhotoCard photo=p fresh removed shared/> }/>
        </ul>
    }
}

#[component]
fn PhotoCard(
    photo: MyPhoto,
    fresh: bool,
    removed: Callback<String>,
    shared: editor::MyPhotos,
) -> impl IntoView {
    let MyPhoto { item, used_in } = photo;
    let MediaItem {
        id,
        url,
        name,
        width,
        height,
        original_bytes,
        web_bytes,
        alt,
        ..
    } = item;
    let thumb = domain::media_base(&url)
        .map(|b| format!("{b}/480.webp"))
        .unwrap_or(url);
    let saved_name = RwSignal::new(name.clone());
    let draft = RwSignal::new(name);
    let error = RwSignal::new(None::<String>);
    let confirm = RwSignal::new(false);
    // Teléfono: cada foto es una miniatura; al tocarla se abren sus datos.
    let open = RwSignal::new(false);
    let rename =
        Action::new(|(id, name): &(String, String)| rename_photo(id.clone(), name.clone()));
    let remove = Action::new(|id: &String| delete_photo(id.clone()));
    let id_removed = id.clone();
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
    let id_renamed = id.clone();
    Effect::new(move |_| match rename.value().get() {
        Some(Ok(())) => {
            let new_name = draft.get_untracked();
            shared.items.update(|l| {
                if let Some(p) = l.iter_mut().flatten().find(|p| p.id == id_renamed) {
                    p.name = new_name.clone();
                }
            });
            saved_name.set(new_name);
            error.set(None);
        }
        Some(Err(e)) => error.set(Some(clean(e))),
        None => {}
    });
    Effect::new(move |_| match remove.value().get() {
        Some(Ok(())) => removed.run(id_removed.clone()),
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
    let alt_id = format!("foto-{id}-alt");
    let alt_help = format!("foto-{id}-alt-ayuda");
    let uses = used_in.len();
    let (id_card, id_fig) = (id.clone(), id.clone());
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
        <li class="card overflow-hidden bg-base-100 shadow-sm" class=("max-sm:col-span-2", move || open.get())
            data-photo-card=id_card>
            <figure class="aspect-[4/3] bg-base-200" class=("max-sm:aspect-square", move || !open.get())
                style=fresh.then(|| format!("view-transition-name: foto-{id_fig}"))>
                <img src=thumb alt=move || saved_name.get() loading=if fresh { "eager" } else { "lazy" } class="h-full w-full object-cover"/>
            </figure>
            <button type="button" class="flex min-h-12 w-full items-center gap-2 px-3 text-left text-sm sm:hidden"
                aria-expanded=move || open.get().to_string()
                on:click=move |_| open.update(|v| *v = !*v)>
                <span class="min-w-0 grow">
                    <span class="block truncate font-medium">{move || saved_name.get()}</span>
                    <span class="block truncate text-xs text-base-content/60">{if uses == 0 { "Sin usar".to_string() } else { format!("En {uses} {}", if uses == 1 { "lugar" } else { "lugares" }) }}</span>
                </span>
                <span aria-hidden="true" class="text-base-content/60">{move || if open.get() { "▴" } else { "▾" }}</span>
            </button>
            <div class="card-body gap-2 p-4" class=("max-sm:hidden", move || !open.get())>
                <label for=input_id.clone() class="text-xs font-medium text-base-content/70">"Nombre"</label>
                <input id=input_id class="input input-sm w-full" maxlength="80" prop:value=move || draft.get()
                    on:input=move |ev| draft.set(event_target_value(&ev))
                    on:blur=move |_| save()
                    on:keydown=move |ev| if ev.key() == "Enter" { save2() }/>
                <label for=alt_id.clone() class="text-xs font-medium text-base-content/70">"Qué se ve en la foto"</label>
                <textarea id=alt_id class="textarea textarea-sm w-full" rows="2" maxlength=domain::PHOTO_ALT_MAX
                    aria-describedby=alt_help.clone()
                    placeholder="Ej.: Remera azul de algodón, vista de frente"
                    prop:value=move || alt_draft.get()
                    on:input=move |ev| alt_draft.set(event_target_value(&ev))
                    on:blur=move |_| save_alt()></textarea>
                <p id=alt_help class="text-xs text-base-content/60">
                    {move || if saved_alt.with(String::is_empty) {
                        "Ayuda a que Google la encuentre y a quien no ve la pantalla. Se usa en los bloques donde la pongas."
                    } else {
                        "Guardado. Se usa en los bloques nuevos donde la pongas."
                    }}
                </p>
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

/// Cuánto usa de Mis fotos y cuánto le queda.
#[component]
fn QuotaBar(quota: domain::PhotoQuota, shared: editor::MyPhotos) -> impl IntoView {
    let quota = RwSignal::new(quota);
    // Al subir o borrar fotos, se vuelve a pedir cuánto se usa.
    Effect::new(move |prev: Option<Option<usize>>| {
        let n = shared.items.with(|l| l.as_ref().map(Vec::len));
        if prev.is_some_and(|p| p != n) {
            leptos::task::spawn_local(async move {
                if let Ok(Some(q)) = photo_quota().await {
                    quota.set(q);
                }
            });
        }
        n
    });
    let pct = move || quota.get().percent();
    let class = move || {
        let pct = pct();
        if pct >= 90 {
            "progress progress-error w-full"
        } else if pct >= 75 {
            "progress progress-warning w-full"
        } else {
            "progress progress-primary w-full"
        }
    };
    view! {
        <div class="card bg-base-100 shadow-sm">
            <div class="card-body gap-2 p-4">
                <div class="flex flex-wrap items-baseline justify-between gap-2">
                    <span class="font-medium">{move || quota.get().text()}</span>
                    <span class="text-sm text-base-content/70">{move || quota.get().left_text()}</span>
                </div>
                <progress class=class value=move || pct().to_string() max="100" aria-label="Espacio usado de Mis fotos"></progress>
                {move || (pct() >= 90).then(|| view! {
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
