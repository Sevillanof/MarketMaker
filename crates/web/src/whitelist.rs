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
    let invited = db::auth::invitations_sent(&pool()?, &emails)
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
                invited: invited
                    .iter()
                    .find(|(e, _)| !m.has_account && *e == m.email)
                    .map(|(_, s)| format!("Invitación enviada {}", domain::ago_es(*s)))
                    .unwrap_or_default(),
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

/// Habilita un email para crear su cuenta. No le manda nada: la invitación
/// se envía aparte, cuando el administrador quiere.
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

/// Le manda la invitación (con su enlace personal) a un email habilitado que
/// todavía no tiene cuenta, con el asunto, el título y el mensaje que el
/// administrador dejó en la vista previa. `false` si no se pudo mandar.
#[server]
pub async fn send_invitation(
    email: String,
    subject: String,
    title: String,
    message: String,
) -> Result<bool, ServerFnError> {
    use crate::server::{internal, pool};
    require_admin().await?;
    let Some(email) = domain::normalize_email(&email) else {
        return Ok(false);
    };
    if !crate::auth::allowed_emails().contains(&email) {
        return Ok(false);
    }
    let pool = pool()?;
    let has_account = db::auth::account_by_email(&pool, &email)
        .await
        .map_err(internal)?
        .is_some();
    if has_account {
        return Ok(false);
    }
    let key = db::auth::create_invite_key(&pool, &email)
        .await
        .map_err(internal)?;
    let url = domain::password_link(&crate::email::app_url(), &email, &key, true);
    if !crate::auth::emails_go_out() {
        leptos::logging::log!("\n  Enlace de invitación de {email}: {url}\n");
    }
    let words = domain::InviteText::clean(&subject, &title, &message);
    match crate::email::deliver(&crate::email::invitation(&email, &url, &words)).await {
        Ok(()) => Ok(true),
        Err(e) => {
            leptos::logging::log!("No pudimos mandar la invitación a {email}: {e}");
            Ok(false)
        }
    }
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
    let sent = RwSignal::new(None::<String>);
    // Solo en el navegador: la lista se pide al abrir la ventana, no al armar la página.
    let list = LocalResource::new(move || {
        let v = version.get();
        async move {
            if v == 0 {
                Ok(Vec::new())
            } else {
                load_whitelist().await
            }
        }
    });
    let add = Action::new(|e: &String| add_allowed_email(e.clone()));
    let remove = Action::new(|e: &String| remove_allowed_email(e.clone()));
    // La vista previa de la invitación: se abre con la carta de cada persona
    // y recién ahí se envía, con los textos que queden escritos.
    let preview = NodeRef::<leptos::html::Dialog>::new();
    let invite_to = RwSignal::new(String::new());
    let subject = RwSignal::new(String::new());
    let title = RwSignal::new(String::new());
    let message = RwSignal::new(String::new());
    let invite_problem = RwSignal::new(None::<String>);
    let original = move || {
        subject.set(domain::INVITE_SUBJECT.into());
        title.set(domain::INVITE_TITLE.into());
        message.set(domain::INVITE_MESSAGE.into());
    };
    let changed = move || {
        subject.get() != domain::INVITE_SUBJECT
            || title.get() != domain::INVITE_TITLE
            || message.get() != domain::INVITE_MESSAGE
    };
    let open_preview = Callback::new(move |to: String| {
        invite_to.set(to);
        invite_problem.set(None);
        original();
        if let Some(d) = preview.get() {
            _ = d.show_modal();
        }
    });
    let invite = Action::new(|(e, s, t, m): &(String, String, String, String)| {
        send_invitation(e.clone(), s.clone(), t.clone(), m.clone())
    });
    Effect::new(move |_| {
        let Some(result) = invite.value().get() else {
            return;
        };
        let to = invite_to.get_untracked();
        match result {
            Ok(true) => {
                problem.set(None);
                sent.set(Some(format!("Le enviamos la invitación a {to}.")));
                version.update(|v| *v += 1);
                if let Some(d) = preview.get() {
                    d.close();
                }
            }
            Ok(false) => invite_problem.set(Some(
                "No pudimos enviar la invitación. Probá de nuevo en un momento.".into(),
            )),
            Err(_) => invite_problem.set(Some(CONNECTION.into())),
        }
    });
    // Un texto del email que se cambia ahí mismo: borde punteado y lápiz.
    let editable = "w-full rounded-md bg-transparent outline-2 outline-offset-4 outline-dashed outline-primary/40 hover:outline-primary/70 focus:outline-solid focus:outline-primary";
    let pencil = || {
        view! {
            <span class="pointer-events-none absolute -top-3 -right-3 grid size-6 place-items-center rounded-full bg-primary text-primary-content" aria-hidden="true">
                <ui::Icon class="size-3.5" name="pencil"/>
            </span>
        }
    };
    let step = |n: u8, name: &'static str, text: &'static str| {
        view! {
            <li class="flex gap-3.5">
                <span class="grid size-7 shrink-0 place-items-center rounded-full bg-primary/10 text-[13px] font-bold text-primary" aria-hidden="true">{n}</span>
                <span>
                    <span class="block text-sm font-bold">{name}</span>
                    <span class="block text-[13px] leading-snug text-base-content/70">{text}</span>
                </span>
            </li>
        }
    };
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
            on:close=move |_| { problem.set(None); sent.set(None); confirming.set(None); email.set(String::new()); }>
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
                    <form class="flex flex-col gap-2" novalidate on:submit=move |ev: leptos::ev::SubmitEvent| {
                        ev.prevent_default();
                        problem.set(None);
                        sent.set(None);
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
                        {move || sent.get().map(|t| view! { <FieldNote ok=true text=t/> })}
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
                                    {people.into_iter().map(|p| view! { <PersonRow p confirming remove on_invite=open_preview/> }).collect_view()}
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
        <dialog node_ref=preview class="modal" aria-labelledby="invitacion-titulo">
            <form class="modal-box flex max-h-[92vh] max-w-xl flex-col p-0" novalidate
                on:submit=move |ev: leptos::ev::SubmitEvent| {
                    ev.prevent_default();
                    invite_problem.set(None);
                    invite.dispatch((invite_to.get_untracked(), subject.get_untracked(), title.get_untracked(), message.get_untracked()));
                }>
                <div class="flex items-start justify-between gap-4 border-b border-base-300 px-6 py-4">
                    <div class="min-w-0">
                        <h2 id="invitacion-titulo" class="text-lg font-extrabold">"Enviar invitación"</h2>
                        <p class="truncate text-sm text-base-content/70">"Para " <strong class="text-base-content">{move || invite_to.get()}</strong></p>
                    </div>
                    <button type="button" class="btn btn-ghost btn-sm btn-circle" aria-label="Cerrar"
                        on:click=move |_| { if let Some(d) = preview.get() { d.close(); } }>"✕"</button>
                </div>
                <div class="flex items-center gap-3 border-b border-base-300 px-6 py-3.5 text-sm">
                    <label for="invitacion-asunto" class="font-semibold text-base-content/60">"Asunto"</label>
                    <span class="relative grow">
                        <input id="invitacion-asunto" type="text" class=format!("{editable} px-1 py-0.5")
                            maxlength=domain::INVITE_SUBJECT_MAX.to_string() autocomplete="off"
                            prop:value=move || subject.get()
                            on:input=move |ev| subject.set(event_target_value(&ev))/>
                        {pencil()}
                    </span>
                </div>
                <div class="overflow-y-auto overscroll-contain bg-base-200 px-5 py-5">
                    <div class="mx-auto max-w-md rounded-2xl border border-base-300 bg-base-100 p-7">
                        <div class="flex items-center gap-2.5" aria-hidden="true">
                            <span class="grid size-8 place-items-center rounded-[9px] bg-gradient-to-br from-[#4330d6] to-[#6a55f0] text-lg font-extrabold text-white">"m"</span>
                            <span class="font-extrabold">"MarketMaker"</span>
                        </div>
                        <div class="relative mt-6">
                            <input type="text" aria-label="Título del email"
                                class=format!("{editable} text-2xl leading-tight font-extrabold tracking-tight")
                                maxlength=domain::INVITE_TITLE_MAX.to_string() autocomplete="off"
                                prop:value=move || title.get()
                                on:input=move |ev| title.set(event_target_value(&ev))/>
                            {pencil()}
                        </div>
                        <div class="relative mt-5">
                            <textarea aria-label="Mensaje del email" rows="3" autofocus
                                class=format!("{editable} block h-auto min-h-[2lh] max-h-[10lh] resize-none [field-sizing:content] text-[15px] leading-relaxed text-base-content/70")
                                maxlength=domain::INVITE_MESSAGE_MAX.to_string()
                                prop:value=move || message.get()
                                on:input=move |ev| message.set(event_target_value(&ev))></textarea>
                            {pencil()}
                        </div>
                        <ol class="mt-5 flex flex-col gap-3.5">
                            {step(1, "Creá tu cuenta", "Elegís tu nombre y una contraseña.")}
                            {step(2, "Armá tu tienda", "Diseño, productos y formas de cobro.")}
                            {step(3, "Publicala", "Tu dirección propia: tutienda.mkr.com.ar")}
                        </ol>
                        <div class="mt-5 rounded-xl bg-primary py-3 text-center text-[15px] font-bold text-primary-content" aria-hidden="true">"Crear mi cuenta"</div>
                        <p class="mt-3 text-center text-xs text-base-content/60">"El enlace es personal y vence en 7 días."</p>
                    </div>
                </div>
                <p class="border-t border-base-300 bg-base-200/50 px-6 py-2.5 text-[13px] text-base-content/60">
                    "Tocá cualquier texto marcado para cambiarlo. Los pasos y el botón van siempre."
                </p>
                <div class="flex flex-wrap items-center justify-between gap-x-4 gap-y-2 border-t border-base-300 px-6 py-4">
                    <div class="flex min-w-0 flex-col gap-1">
                        <button type="button" class="self-start text-sm font-semibold text-primary hover:underline disabled:text-base-content/40 disabled:no-underline"
                            disabled=move || !changed() on:click=move |_| original()>"Volver al texto original"</button>
                        {move || invite_problem.get().map(|p| view! { <FieldNote ok=false text=p/> })}
                    </div>
                    <div class="flex gap-2">
                        <button type="button" class="btn btn-ghost"
                            on:click=move |_| { if let Some(d) = preview.get() { d.close(); } }>"Cancelar"</button>
                        <button type="submit" class="btn btn-primary" disabled=move || invite.pending().get()>
                            {move || if invite.pending().get() { "Enviando…" } else { "Enviar invitación" }}
                        </button>
                    </div>
                </div>
            </form>
            <form method="dialog" class="modal-backdrop"><button>"Cerrar"</button></form>
        </dialog>
    }
}

#[component]
fn PersonRow(
    p: AllowedPerson,
    confirming: RwSignal<Option<String>>,
    remove: Action<String, Result<WhitelistChange, ServerFnError>>,
    on_invite: Callback<String>,
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
    } else if !p.invited.is_empty() {
        ("Invitado", "bg-primary/10 text-primary")
    } else {
        ("Esperando", "bg-warning/20 text-warning-content")
    };
    let facts = [p.joined.clone(), p.last_seen.clone(), p.invited.clone()]
        .into_iter()
        .filter(|s| !s.is_empty())
        .chain((p.orders > 0).then(|| format!("{} pedidos", p.orders)))
        .collect::<Vec<_>>()
        .join(" · ");
    let email = p.email.clone();
    let email_confirm = p.email.clone();
    let email_remove = p.email.clone();
    let label = format!("Quitar {}", p.email);
    // La carta: para enviar la invitación; con tilde si ya se le envió.
    let was_invited = !p.invited.is_empty();
    let can_invite = !p.has_account && !p.is_admin;
    let email_invite = p.email.clone();
    let invite_label = if was_invited {
        format!("Ver y reenviar la invitación a {}", p.email)
    } else {
        format!("Ver y enviar la invitación a {}", p.email)
    };
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
            <div class="flex items-center gap-1 self-center text-sm">
                {can_invite.then(|| view! {
                    <button type="button" aria-label=invite_label.clone() title=invite_label.clone()
                        class=if was_invited { "btn btn-ghost btn-sm btn-square text-success" } else { "btn btn-ghost btn-sm btn-square text-primary" }
                        on:click=move |_| on_invite.run(email_invite.clone())>
                        <ui::Icon class="size-5" name=if was_invited { "mail-check" } else { "mail" }/>
                    </button>
                })}
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
