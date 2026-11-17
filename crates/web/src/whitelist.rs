//! Whitelist: quién puede tener cuenta en MarketMaker, editable desde el menú
//! de la cuenta del administrador (`ADMIN_EMAILS`). Se suma a `ALLOWED_EMAILS`
//! del servidor; los cambios valen al instante.

use domain::{AllowedPerson, WhitelistChange};
use leptos::prelude::*;

use crate::account::FieldNote;
use crate::panel::Avatar;

const CONNECTION: &str =
    "No pudimos comunicarnos con el servidor. Revisá tu conexión y probá de nuevo.";

/// La cuenta administradora que está mirando; error si no lo es.
#[cfg(feature = "ssr")]
async fn require_admin() -> Result<db::auth::SessionUser, ServerFnError> {
    match crate::auth::current_user().await? {
        Some(u) if crate::auth::is_admin(&u.email) => Ok(u),
        _ => Err(ServerFnError::new(
            "Solo el administrador puede ver esta lista.",
        )),
    }
}

/// Todos los emails habilitados con los datos de su cuenta, los más nuevos
/// primero y el administrador al final.
#[server]
pub async fn load_whitelist() -> Result<Vec<AllowedPerson>, ServerFnError> {
    use crate::server::{internal, pool};
    let me = require_admin().await?;
    let emails: Vec<String> = crate::auth::allowed_emails().into_iter().collect();
    let members = db::auth::allowlist_members(&pool()?, &emails)
        .await
        .map_err(internal)?;
    let mut people: Vec<(i64, AllowedPerson)> = members
        .into_iter()
        .map(|m| {
            let is_admin = crate::auth::is_admin(&m.email);
            let age = m.added.or(m.joined);
            let joined = match (m.has_account, m.joined, m.added) {
                (true, Some(s), _) => format!("Se unió {}", domain::ago_es(s)),
                (false, _, Some(s)) => format!("Agregado {}", domain::ago_es(s)),
                _ => String::new(),
            };
            let person = AllowedPerson {
                removable: !is_admin && m.email != me.email,
                is_admin,
                has_account: m.has_account,
                name: m.name,
                avatar: m.avatar_id.map(domain::avatar_url),
                joined,
                last_seen: m
                    .last_seen
                    .map(|s| format!("Último acceso {}", domain::ago_es(s)))
                    .unwrap_or_default(),
                stores: m
                    .stores
                    .into_iter()
                    .map(|(slug, name)| domain::StoreCard {
                        url: crate::stores::store_url(&slug),
                        name,
                        slug,
                    })
                    .collect(),
                orders: m.orders,
                email: m.email,
            };
            let key = if is_admin {
                i64::MAX
            } else {
                age.unwrap_or(i64::MAX - 1)
            };
            (key, person)
        })
        .collect();
    people.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.email.cmp(&b.1.email)));
    Ok(people.into_iter().map(|(_, p)| p).collect())
}

/// Habilita un email para crear su cuenta.
#[server]
pub async fn add_allowed_email(email: String) -> Result<WhitelistChange, ServerFnError> {
    use crate::server::{internal, pool};
    let me = require_admin().await?;
    let Some(email) = domain::normalize_email(&email) else {
        return Ok(WhitelistChange::Invalid);
    };
    if crate::auth::allowed_emails().contains(&email) {
        return Ok(WhitelistChange::Already);
    }
    let pool = pool()?;
    db::auth::allow_email(&pool, &email, me.id)
        .await
        .map_err(internal)?;
    crate::auth::refresh_allowlist(&pool).await?;
    Ok(WhitelistChange::Done)
}

/// Quita un email de la lista: la persona deja de poder entrar.
#[server]
pub async fn remove_allowed_email(email: String) -> Result<WhitelistChange, ServerFnError> {
    use crate::server::{internal, pool};
    let me = require_admin().await?;
    let Some(email) = domain::normalize_email(&email) else {
        return Ok(WhitelistChange::Invalid);
    };
    if crate::auth::is_admin(&email) || email == me.email {
        return Ok(WhitelistChange::Protected);
    }
    let pool = pool()?;
    db::auth::disallow_email(&pool, &email, me.id)
        .await
        .map_err(internal)?;
    crate::auth::refresh_allowlist(&pool).await?;
    Ok(WhitelistChange::Done)
}

/// Abre la ventana y pide la lista de nuevo.
pub fn open_whitelist(dialog: NodeRef<leptos::html::Dialog>, version: RwSignal<u32>) {
    version.update(|v| *v += 1);
    if let Some(d) = dialog.get() {
        _ = d.show_modal();
    }
}

#[component]
pub fn WhitelistDialog(
    dialog: NodeRef<leptos::html::Dialog>,
    version: RwSignal<u32>,
) -> impl IntoView {
    let email = RwSignal::new(String::new());
    let problem = RwSignal::new(None::<String>);
    let confirming = RwSignal::new(None::<String>);
    let list = Resource::new(
        move || version.get(),
        |v| async move {
            if v == 0 {
                Ok(Vec::new())
            } else {
                load_whitelist().await
            }
        },
    );
    let add = Action::new(|e: &String| add_allowed_email(e.clone()));
    let remove = Action::new(|e: &String| remove_allowed_email(e.clone()));
    Effect::new(move |_| match add.value().get() {
        Some(Ok(WhitelistChange::Done)) => {
            email.set(String::new());
            problem.set(None);
            version.update(|v| *v += 1);
        }
        Some(Ok(WhitelistChange::Already)) => {
            problem.set(Some("Ese email ya está en la lista.".into()))
        }
        Some(Ok(_)) => problem.set(Some(
            "Escribí un email completo, por ejemplo nombre@correo.com".into(),
        )),
        Some(Err(_)) => problem.set(Some(CONNECTION.into())),
        None => {}
    });
    Effect::new(move |_| match remove.value().get() {
        Some(Ok(WhitelistChange::Done)) => {
            confirming.set(None);
            version.update(|v| *v += 1);
        }
        Some(_) => problem.set(Some("No pudimos quitar ese email. Probá de nuevo.".into())),
        None => {}
    });
    view! {
        <dialog node_ref=dialog class="modal" aria-labelledby="whitelist-titulo"
            on:close=move |_| { problem.set(None); confirming.set(None); email.set(String::new()); }>
            <div class="modal-box max-w-xl p-0">
                <div class="flex flex-col gap-3 p-6 pb-3">
                    <div class="flex items-start justify-between gap-4">
                        <div>
                            <h2 id="whitelist-titulo" class="text-xl font-extrabold">"Whitelist"</h2>
                            <p class="text-sm text-base-content/70">"Quién puede crear una cuenta. Los cambios se aplican al instante."</p>
                        </div>
                        <button type="button" class="btn btn-ghost btn-sm btn-circle" aria-label="Cerrar"
                            on:click=move |_| { if let Some(d) = dialog.get() { d.close(); } }>"✕"</button>
                    </div>
                    <form class="flex flex-col gap-2" on:submit=move |ev: leptos::ev::SubmitEvent| {
                        ev.prevent_default();
                        problem.set(None);
                        add.dispatch(email.get_untracked());
                    }>
                        <div class="flex gap-2">
                            <input type="email" class="input grow" placeholder="nombre@correo.com"
                                aria-label="Email a agregar" autocomplete="off"
                                prop:value=move || email.get()
                                on:input=move |ev| { problem.set(None); email.set(event_target_value(&ev)); }/>
                            <button type="submit" class="btn btn-primary" disabled=move || add.pending().get()>"Agregar"</button>
                        </div>
                        {move || problem.get().map(|p| view! { <FieldNote ok=false text=p/> })}
                    </form>
                    {move || list.get().and_then(|r| r.ok()).filter(|l| !l.is_empty()).map(|l| {
                        let with_account = l.iter().filter(|p| p.has_account).count();
                        view! {
                            <div class="flex justify-between text-sm text-base-content/70">
                                <span>{format!("{} habilitados · {} con cuenta", l.len(), with_account)}</span>
                                <span>"Más recientes primero"</span>
                            </div>
                        }
                    })}
                </div>
                <div class="max-h-[55vh] overflow-y-auto border-t border-base-300 px-6">
                    <Suspense fallback=|| view! { <p class="py-6 text-sm text-base-content/70">"Cargando…"</p> }>
                        {move || list.get().map(|r| match r {
                            Err(_) => view! { <p class="py-6"><FieldNote ok=false text=CONNECTION/></p> }.into_any(),
                            Ok(people) => view! {
                                <ul>
                                    {people.into_iter().map(|p| view! { <PersonRow p confirming remove/> }).collect_view()}
                                </ul>
                            }.into_any(),
                        })}
                    </Suspense>
                </div>
                <div class="flex justify-end p-4">
                    <button type="button" class="btn" on:click=move |_| { if let Some(d) = dialog.get() { d.close(); } }>"Cerrar"</button>
                </div>
            </div>
            <form method="dialog" class="modal-backdrop"><button>"Cerrar"</button></form>
        </dialog>
    }
}

#[component]
fn PersonRow(
    p: AllowedPerson,
    confirming: RwSignal<Option<String>>,
    remove: Action<String, Result<WhitelistChange, ServerFnError>>,
) -> impl IntoView {
    let initial = p
        .name
        .chars()
        .next()
        .or(p.email.chars().next())
        .unwrap_or('?')
        .to_uppercase()
        .to_string();
    let (tag, tag_class) = if p.is_admin {
        ("Administrador", "bg-primary/10 text-primary")
    } else if p.has_account {
        ("Con cuenta", "bg-success/15 text-success-content")
    } else {
        ("Esperando", "bg-warning/20 text-warning-content")
    };
    let facts = [p.joined.clone(), p.last_seen.clone()]
        .into_iter()
        .filter(|s| !s.is_empty())
        .chain((p.orders > 0).then(|| format!("{} pedidos", p.orders)))
        .collect::<Vec<_>>()
        .join(" · ");
    let email = p.email.clone();
    let email_confirm = p.email.clone();
    let email_remove = p.email.clone();
    let label = format!("Quitar {}", p.email);
    view! {
        <li class="flex gap-3 border-b border-base-300 py-3 last:border-b-0">
            <Avatar letter=initial size="size-10 text-sm" photo=p.avatar.clone()/>
            <div class="min-w-0 grow">
                <div class="flex flex-wrap items-center gap-2">
                    {if p.name.is_empty() {
                        view! { <span class="text-base-content/60">"Sin nombre todavía"</span> }.into_any()
                    } else {
                        view! { <span class="font-semibold">{p.name.clone()}</span> }.into_any()
                    }}
                    <span class=format!("rounded-full px-2 py-0.5 text-xs {tag_class}")>{tag}</span>
                </div>
                <div class="truncate text-sm text-base-content/70">{p.email.clone()}</div>
                {if p.stores.is_empty() {
                    (!p.has_account).then(|| view! { <div class="text-sm text-base-content/60">"Todavía no creó su cuenta"</div> })
                } else { None }}
                {p.stores.iter().map(|s| {
                    let host = s.url.split_once("://").map_or(s.url.clone(), |(_, h)| h.to_string());
                    view! {
                        <div class="text-sm">
                            <a href=s.url.clone() target="_blank" rel="noopener" class="link link-hover inline-flex items-center gap-1.5">
                                <ui::Icon class="size-4 text-base-content/60" name="store"/>
                                {format!("{} · {}", s.name, host)}
                            </a>
                        </div>
                    }
                }).collect_view()}
                {(!facts.is_empty()).then(|| view! { <div class="mt-0.5 text-xs text-base-content/60">{facts}</div> })}
                {move || (confirming.get().as_deref() == Some(email_confirm.as_str())).then(|| {
                    let e = email_remove.clone();
                    view! {
                        <div class="mt-2 flex flex-wrap items-center gap-2 text-sm">
                            <span>"Va a perder el acceso a su cuenta."</span>
                            <button type="button" class="btn btn-ghost btn-xs" on:click=move |_| confirming.set(None)>"Cancelar"</button>
                            <button type="button" class="btn btn-error btn-xs" disabled=move || remove.pending().get()
                                on:click=move |_| { remove.dispatch(e.clone()); }>"Quitar"</button>
                        </div>
                    }
                })}
            </div>
            <div class="self-center text-sm">
                {if p.removable {
                    view! {
                        <button type="button" class="btn btn-ghost btn-sm text-base-content/70 hover:text-error" aria-label=label
                            on:click=move |_| confirming.set(Some(email.clone()))>"Quitar"</button>
                    }.into_any()
                } else {
                    view! { <span class="text-xs text-base-content/60">"No se puede quitar"</span> }.into_any()
                }}
            </div>
        </li>
    }
}
