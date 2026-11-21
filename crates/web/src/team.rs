//! Equipo de una tienda: el dueño invita a administradores y editores desde
//! "Datos de la tienda". Una invitación sirve como alta: la persona crea su
//! cuenta con el enlace del email aunque no esté en la Whitelist, pero solo
//! trabaja en esa tienda (no crea tiendas propias).

use domain::{TeamChange, TeamInfo, TeamPerson};
use leptos::prelude::*;

use crate::account::FieldNote;
use crate::panel::Avatar;

#[cfg(feature = "ssr")]
mod server {
    use crate::server::{internal, pool};
    use leptos::prelude::ServerFnError;

    /// La tienda `slug`, quien mira y su rol. Pide ser administrador o dueño.
    pub async fn manager(
        slug: &str,
    ) -> Result<Option<(db::Tenant, db::auth::SessionUser, String)>, ServerFnError> {
        let Some(user) = crate::auth::current_user().await? else {
            return Ok(None);
        };
        let pool = pool()?;
        let tenant = db::get_tenant_by_slug(&pool, slug)
            .await
            .map_err(internal)?
            .ok_or_else(|| ServerFnError::new("Esa tienda no existe."))?;
        let role = db::auth::role_in(&pool, tenant.id, user.id)
            .await
            .map_err(internal)?
            .ok_or_else(|| ServerFnError::new("Tu cuenta no puede editar esta tienda."))?;
        if domain::role_rank(&role) < domain::role_rank("admin") {
            return Ok(None);
        }
        Ok(Some((tenant, user, role)))
    }

    /// Un administrador solo maneja editores; el dueño, a todos.
    pub fn may_manage(me: &str, target: &str) -> bool {
        target != "owner" && domain::role_rank(me) > domain::role_rank(target)
    }

    /// Los roles que `me` puede dar al invitar o cambiar.
    pub fn may_give(me: &str, role: &str) -> bool {
        matches!(role, "admin" | "editor") && domain::role_rank(me) > domain::role_rank(role)
    }
}

/// El equipo de la tienda `slug`. `None` si no entró o no administra la tienda.
#[server]
pub async fn load_team(slug: String) -> Result<Option<TeamInfo>, ServerFnError> {
    use crate::server::{internal, pool};
    let Some((tenant, me, role)) = server::manager(&slug).await? else {
        return Ok(None);
    };
    let members = db::auth::team_members(&pool()?, tenant.id)
        .await
        .map_err(internal)?;
    let people = members
        .into_iter()
        .map(|m| TeamPerson {
            manageable: server::may_manage(&role, &m.role),
            avatar: m.avatar_id.map(domain::avatar_url),
            since: if m.joined {
                format!("Se unió {}", domain::ago_es(m.since))
            } else {
                format!("Invitada {}", domain::ago_es(m.since))
            },
            last_seen: m
                .last_seen
                .map(|s| format!("Último acceso {}", domain::ago_es(s)))
                .unwrap_or_default(),
            sent: m
                .sent
                .map(|s| format!("Invitación enviada {}", domain::ago_es(s)))
                .unwrap_or_default(),
            email: m.email,
            name: m.name,
            role: m.role,
            joined: m.joined,
        })
        .collect();
    Ok(Some(TeamInfo {
        store_name: tenant.name,
        me: role,
        me_name: me.name,
        people,
    }))
}

/// Suma a `email` al equipo con el rol `role`. No manda nada: el email sale
/// desde la cartita de la persona, con vista previa.
#[server]
pub async fn add_team_member(
    slug: String,
    email: String,
    role: String,
) -> Result<TeamChange, ServerFnError> {
    use crate::server::{internal, pool};
    let Some((tenant, me, my_role)) = server::manager(&slug).await? else {
        return Ok(TeamChange::NotAllowed);
    };
    if !server::may_give(&my_role, &role) {
        return Ok(TeamChange::NotAllowed);
    }
    let Some(email) = domain::normalize_email(&email) else {
        return Ok(TeamChange::Invalid);
    };
    // Cuántos intentos de invitar por hora: frena una cuenta que invita en serie.
    if !crate::limits::allow(
        &format!("equipo|{}", me.id),
        30,
        std::time::Duration::from_secs(3600),
    ) {
        return Err(ServerFnError::new(
            "Invitaste a muchas personas en poco tiempo. Probá de nuevo en un rato.",
        ));
    }
    let pool = pool()?;
    let result = db::auth::add_team_member(&pool, tenant.id, &email, &role, me.id)
        .await
        .map_err(internal)?;
    crate::auth::refresh_allowlist(&pool).await?;
    Ok(match result {
        None => TeamChange::TooMany,
        Some(db::auth::TeamAdd::Already) => TeamChange::Already,
        Some(db::auth::TeamAdd::Blocked) => TeamChange::Blocked,
        Some(_) => TeamChange::Done,
    })
}

/// Cambia el rol de alguien del equipo.
#[server]
pub async fn set_team_role(
    slug: String,
    email: String,
    role: String,
) -> Result<TeamChange, ServerFnError> {
    use crate::server::{internal, pool};
    let Some((tenant, _, my_role)) = server::manager(&slug).await? else {
        return Ok(TeamChange::NotAllowed);
    };
    let Some(email) = domain::normalize_email(&email) else {
        return Ok(TeamChange::Invalid);
    };
    let pool = pool()?;
    let team = db::auth::team_members(&pool, tenant.id)
        .await
        .map_err(internal)?;
    let Some(target) = team.iter().find(|m| m.email == email) else {
        return Ok(TeamChange::Invalid);
    };
    if !server::may_manage(&my_role, &target.role) || !server::may_give(&my_role, &role) {
        return Ok(TeamChange::NotAllowed);
    }
    db::auth::set_team_role(&pool, tenant.id, &email, &role)
        .await
        .map_err(internal)?;
    Ok(TeamChange::Done)
}

/// Saca a alguien del equipo o anula su invitación.
#[server]
pub async fn remove_team_member(slug: String, email: String) -> Result<TeamChange, ServerFnError> {
    use crate::server::{internal, pool};
    let Some((tenant, _, my_role)) = server::manager(&slug).await? else {
        return Ok(TeamChange::NotAllowed);
    };
    let Some(email) = domain::normalize_email(&email) else {
        return Ok(TeamChange::Invalid);
    };
    let pool = pool()?;
    let team = db::auth::team_members(&pool, tenant.id)
        .await
        .map_err(internal)?;
    let Some(target) = team.iter().find(|m| m.email == email) else {
        return Ok(TeamChange::Invalid);
    };
    if !server::may_manage(&my_role, &target.role) {
        return Ok(TeamChange::NotAllowed);
    }
    db::auth::remove_team_member(&pool, tenant.id, &email)
        .await
        .map_err(internal)?;
    Ok(TeamChange::Done)
}

/// Manda el email de invitación a alguien del equipo, con el asunto, el
/// título y el mensaje que quedaron en la vista previa. `false` si no se pudo.
#[server]
pub async fn send_team_invitation(
    slug: String,
    email: String,
    subject: String,
    title: String,
    message: String,
) -> Result<bool, ServerFnError> {
    use crate::server::{internal, pool};
    let Some((tenant, me, my_role)) = server::manager(&slug).await? else {
        return Ok(false);
    };
    let Some(email) = domain::normalize_email(&email) else {
        return Ok(false);
    };
    if !crate::limits::allow(
        &format!("equipo-mail|{}", me.id),
        30,
        std::time::Duration::from_secs(3600),
    ) {
        return Err(ServerFnError::new(
            "Mandaste muchas invitaciones en poco tiempo. Probá de nuevo en un rato.",
        ));
    }
    let pool = pool()?;
    let team = db::auth::team_members(&pool, tenant.id)
        .await
        .map_err(internal)?;
    let Some(target) = team.iter().find(|m| m.email == email) else {
        return Ok(false);
    };
    if !server::may_manage(&my_role, &target.role) {
        return Ok(false);
    }
    let has_account = target.joined;
    let url = if has_account {
        format!("{}/tienda/{}", crate::email::app_url(), tenant.slug)
    } else {
        let key = db::auth::create_invite_key(&pool, &email)
            .await
            .map_err(internal)?;
        domain::password_link(&crate::email::app_url(), &email, &key, true)
    };
    if !crate::auth::emails_go_out() {
        leptos::logging::log!("\n  Enlace de invitación de equipo para {email}: {url}\n");
    }
    let words = domain::InviteText::clean(&subject, &title, &message);
    match crate::email::deliver(&crate::email::team_invitation(
        &email,
        &url,
        &tenant.name,
        has_account,
        &words,
    ))
    .await
    {
        Ok(()) => {
            db::auth::mark_team_invite_sent(&pool, tenant.id, &email)
                .await
                .map_err(internal)?;
            Ok(true)
        }
        Err(e) => {
            leptos::logging::log!("No pudimos mandar la invitación de equipo a {email}: {e}");
            Ok(false)
        }
    }
}

/// La sección "Equipo" de Datos de la tienda: quién trabaja en la tienda, con
/// el rol de cada persona, y un campo para sumar a otra. Solo la ven el dueño
/// y los administradores.
#[component]
pub fn TeamCard(slug: String) -> impl IntoView {
    let slug = StoredValue::new(slug);
    let version = RwSignal::new(0u32);
    let email = RwSignal::new(String::new());
    let role = RwSignal::new("editor".to_string());
    let problem = RwSignal::new(None::<String>);
    let sent = RwSignal::new(None::<String>);
    let confirming = RwSignal::new(None::<String>);
    let list = Resource::new(move || version.get(), move |_| load_team(slug.get_value()));
    let add = Action::new(move |(e, r): &(String, String)| {
        add_team_member(slug.get_value(), e.clone(), r.clone())
    });
    let change = Action::new(move |(e, r): &(String, String)| {
        set_team_role(slug.get_value(), e.clone(), r.clone())
    });
    let remove = Action::new(move |e: &String| remove_team_member(slug.get_value(), e.clone()));
    Effect::new(move |_| match add.value().get() {
        Some(Ok(TeamChange::Done)) => {
            email.set(String::new());
            problem.set(None);
            version.update(|v| *v += 1);
        }
        Some(Ok(TeamChange::Already)) => {
            problem.set(Some("Esa persona ya está en el equipo.".into()))
        }
        Some(Ok(TeamChange::Blocked)) => problem.set(Some(
            "Ese email no puede sumarse. Escribinos a hola@mkr.com.ar.".into(),
        )),
        Some(Ok(TeamChange::TooMany)) => problem.set(Some(
            "Hay demasiadas invitaciones pendientes. Esperá a que acepten o anulá alguna.".into(),
        )),
        Some(Ok(TeamChange::NotAllowed)) => {
            problem.set(Some("Tu rol no alcanza para sumar a esa persona.".into()))
        }
        Some(Ok(TeamChange::Invalid)) => problem.set(Some(
            "Escribí un email completo, por ejemplo nombre@correo.com".into(),
        )),
        Some(Err(e)) => problem.set(Some(crate::products::error_text(&e))),
        None => {}
    });
    Effect::new(move |_| match change.value().get() {
        Some(Ok(TeamChange::Done)) => version.update(|v| *v += 1),
        Some(_) => {
            problem.set(Some("No pudimos cambiar el rol. Probá de nuevo.".into()));
            version.update(|v| *v += 1);
        }
        None => {}
    });
    Effect::new(move |_| match remove.value().get() {
        Some(Ok(TeamChange::Done)) => {
            confirming.set(None);
            version.update(|v| *v += 1);
        }
        Some(_) => problem.set(Some(
            "No pudimos quitar a esa persona. Probá de nuevo.".into(),
        )),
        None => {}
    });

    // Vista previa del email: se abre desde la cartita de cada persona.
    let preview = NodeRef::<leptos::html::Dialog>::new();
    let invite_to = RwSignal::new(String::new());
    let has_account = RwSignal::new(false);
    let store_name = RwSignal::new(String::new());
    let my_name = RwSignal::new(String::new());
    let subject = RwSignal::new(String::new());
    let title = RwSignal::new(String::new());
    let message = RwSignal::new(String::new());
    let invite_problem = RwSignal::new(None::<String>);
    let original = move || {
        let t = domain::team_invite_text(
            &store_name.get_untracked(),
            &my_name.get_untracked(),
            has_account.get_untracked(),
        );
        subject.set(t.subject);
        title.set(t.title);
        message.set(t.message);
    };
    let changed = move || {
        let t = domain::team_invite_text(&store_name.get(), &my_name.get(), has_account.get());
        subject.get() != t.subject || title.get() != t.title || message.get() != t.message
    };
    let open_preview = Callback::new(move |(to, joined): (String, bool)| {
        invite_to.set(to);
        has_account.set(joined);
        invite_problem.set(None);
        original();
        if let Some(d) = preview.get() {
            _ = d.show_modal();
        }
    });
    let invite = Action::new(move |(e, s, t, m): &(String, String, String, String)| {
        send_team_invitation(slug.get_value(), e.clone(), s.clone(), t.clone(), m.clone())
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
            Err(e) => invite_problem.set(Some(crate::products::error_text(&e))),
        }
    });
    let editable = "w-full rounded-md bg-transparent outline-2 outline-offset-4 outline-dashed outline-primary/40 hover:outline-primary/70 focus:outline-solid focus:outline-primary";
    let pencil = || {
        view! {
            <span class="pointer-events-none absolute -top-3 -right-3 grid size-6 place-items-center rounded-full bg-primary text-primary-content" aria-hidden="true">
                <ui::Icon class="size-3.5" name="pencil"/>
            </span>
        }
    };
    let step = |n: usize, name: String, text: String| {
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
    view! {
        <Suspense fallback=|| view! { <ui::TeamSkeleton/> }>
            {move || Suspend::new(async move {
                let team = list.await.ok().flatten();
                team.map(|team| {
                    store_name.set(team.store_name.clone());
                    my_name.set(team.me_name.clone());
                    let first = team.me_name.split_whitespace().next().unwrap_or("").to_string();
                    my_name.set(first);
                    let me_owner = team.me == "owner";
                    let pending = team.people.iter().filter(|p| !p.joined).count();
                    let summary = if pending > 0 {
                        format!("{} personas · {} pendiente{}", team.people.len(), pending, if pending == 1 { "" } else { "s" })
                    } else {
                        format!("{} personas", team.people.len())
                    };
                    view! {
                        <section class="card border border-base-300 bg-base-100" aria-labelledby="equipo-titulo">
                            <div class="card-body gap-3 p-6">
                                <div>
                                    <h2 id="equipo-titulo" class="text-lg font-bold">"Equipo"</h2>
                                    <p class="text-sm text-base-content/70">"Quién puede trabajar en esta tienda. Los cambios se aplican al instante."</p>
                                </div>
                                <form class="flex flex-col gap-2" novalidate on:submit=move |ev: leptos::ev::SubmitEvent| {
                                    ev.prevent_default();
                                    problem.set(None);
                                    sent.set(None);
                                    add.dispatch((email.get_untracked(), role.get_untracked()));
                                }>
                                    <div class="flex flex-wrap gap-2">
                                        <input type="email" class="input min-w-56 grow" placeholder="nombre@correo.com"
                                            aria-label="Email de la persona a sumar" autocomplete="off"
                                            prop:value=move || email.get()
                                            on:input=move |ev| { problem.set(None); email.set(event_target_value(&ev)); }/>
                                        <select class="select w-auto" aria-label="Rol" prop:value=move || role.get()
                                            on:change=move |ev| role.set(event_target_value(&ev))>
                                            <option value="editor">"Editor"</option>
                                            {me_owner.then(|| view! { <option value="admin">"Administrador"</option> })}
                                        </select>
                                        <button type="submit" class="btn btn-primary" disabled=move || add.pending().get()>"Agregar"</button>
                                    </div>
                                    {move || problem.get().map(|p| view! { <FieldNote ok=false text=p/> })}
                                    {move || sent.get().map(|t| view! { <FieldNote ok=true text=t/> })}
                                </form>
                                <div class="flex flex-wrap justify-between gap-x-4 text-sm text-base-content/70">
                                    <span>{summary}</span>
                                    <span>"Editor: diseño, productos y fotos · Administrador: además pedidos, cobros y equipo"</span>
                                </div>
                                <ul class="border-t border-base-300">
                                    {team.people.into_iter().map(|p| view! { <TeamRow p me_owner confirming remove change on_invite=open_preview/> }).collect_view()}
                                </ul>
                            </div>
                        </section>
                    }
                })
            })}
        </Suspense>
        <dialog node_ref=preview class="modal" aria-labelledby="equipo-invitacion-titulo">
            <form class="modal-box flex max-h-[92vh] max-w-xl flex-col p-0" novalidate
                on:submit=move |ev: leptos::ev::SubmitEvent| {
                    ev.prevent_default();
                    invite_problem.set(None);
                    invite.dispatch((invite_to.get_untracked(), subject.get_untracked(), title.get_untracked(), message.get_untracked()));
                }>
                <div class="flex items-start justify-between gap-4 border-b border-base-300 px-6 py-4">
                    <div class="min-w-0">
                        <h2 id="equipo-invitacion-titulo" class="text-lg font-extrabold">"Enviar invitación"</h2>
                        <p class="truncate text-sm text-base-content/70">"Para " <strong class="text-base-content">{move || invite_to.get()}</strong></p>
                    </div>
                    <button type="button" class="btn btn-ghost btn-sm btn-circle" aria-label="Cerrar"
                        on:click=move |_| { if let Some(d) = preview.get() { d.close(); } }>"✕"</button>
                </div>
                <div class="flex items-center gap-3 border-b border-base-300 px-6 py-3.5 text-sm">
                    <label for="equipo-asunto" class="font-semibold text-base-content/60">"Asunto"</label>
                    <span class="relative grow">
                        <input id="equipo-asunto" type="text" class=format!("{editable} px-1 py-0.5")
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
                            <textarea aria-label="Mensaje del email" rows="3"
                                class=format!("{editable} block h-auto min-h-[2lh] max-h-[10lh] resize-none [field-sizing:content] text-[15px] leading-relaxed text-base-content/70")
                                maxlength=domain::INVITE_MESSAGE_MAX.to_string()
                                prop:value=move || message.get()
                                on:input=move |ev| message.set(event_target_value(&ev))></textarea>
                            {pencil()}
                        </div>
                        <ol class="mt-5 flex flex-col gap-3.5">
                            {move || {
                                let store = store_name.get();
                                let steps: Vec<(&str, &str)> = if has_account.get() {
                                    domain::TEAM_STEPS_OLD.to_vec()
                                } else {
                                    domain::TEAM_STEPS_NEW.to_vec()
                                };
                                steps.into_iter().enumerate()
                                    .map(|(i, (a, b))| step(i + 1, a.replace("{store}", &store), b.replace("{store}", &store)))
                                    .collect_view()
                            }}
                        </ol>
                        <div class="mt-5 rounded-xl bg-primary py-3 text-center text-[15px] font-bold text-primary-content" aria-hidden="true">
                            {move || if has_account.get() { domain::TEAM_BUTTON_OLD.replace("{store}", &store_name.get()) } else { domain::TEAM_BUTTON_NEW.to_string() }}
                        </div>
                        <p class="mt-3 text-center text-xs text-base-content/60">
                            {move || if has_account.get() { "Si no querés participar, ignorá este email.".to_string() } else { format!("El enlace es personal y vence en {} días.", domain::INVITE_DAYS) }}
                        </p>
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
fn TeamRow(
    p: TeamPerson,
    me_owner: bool,
    confirming: RwSignal<Option<String>>,
    remove: Action<String, Result<TeamChange, ServerFnError>>,
    change: Action<(String, String), Result<TeamChange, ServerFnError>>,
    on_invite: Callback<(String, bool)>,
) -> impl IntoView {
    let initial = p
        .name
        .chars()
        .next()
        .or(p.email.chars().next())
        .unwrap_or('?')
        .to_uppercase()
        .to_string();
    let (tag, tag_class) = match p.role.as_str() {
        "owner" => ("Dueño", "bg-primary/10 text-primary"),
        "admin" => ("Administrador", "bg-success/15 text-success-content"),
        _ => ("Editor", "bg-info/15 text-info-content"),
    };
    let facts = [p.since.clone(), p.last_seen.clone(), p.sent.clone()]
        .into_iter()
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join(" · ");
    let email = p.email.clone();
    let email_confirm = p.email.clone();
    let email_remove = p.email.clone();
    let email_role = p.email.clone();
    let email_invite = p.email.clone();
    let joined = p.joined;
    let was_sent = !p.sent.is_empty();
    let label = format!("Quitar a {}", p.email);
    let invite_label = if joined {
        format!("Ver y enviar el aviso a {}", p.email)
    } else if was_sent {
        format!("Ver y reenviar la invitación a {}", p.email)
    } else {
        format!("Ver y enviar la invitación a {}", p.email)
    };
    let role = p.role.clone();
    view! {
        <li class="flex flex-wrap gap-3 border-b border-base-300 py-3 last:border-b-0">
            <Avatar letter=initial size="size-10 text-sm" photo=p.avatar.clone()/>
            <div class="min-w-0 grow basis-48">
                <div class="flex flex-wrap items-center gap-2">
                    {if p.name.is_empty() {
                        view! { <span class="text-base-content/60">"Sin nombre todavía"</span> }.into_any()
                    } else {
                        view! { <span class="font-semibold">{p.name.clone()}</span> }.into_any()
                    }}
                    <span class=format!("rounded-full px-2 py-0.5 text-xs {tag_class}")>{tag}</span>
                    {(!joined).then(|| view! { <span class="rounded-full bg-warning/20 px-2 py-0.5 text-xs text-warning-content">"Pendiente"</span> })}
                </div>
                <div class="truncate text-sm text-base-content/70">{email}</div>
                {(!joined).then(|| view! { <div class="text-sm text-base-content/60">"Todavía no creó su cuenta"</div> })}
                {(!facts.is_empty()).then(|| view! { <div class="mt-0.5 text-xs text-base-content/60">{facts}</div> })}
                {move || (confirming.get().as_deref() == Some(email_confirm.as_str())).then(|| {
                    let e = email_remove.clone();
                    view! {
                        <div class="mt-2 flex flex-wrap items-center gap-2 text-sm">
                            <span>"Va a dejar de ver esta tienda."</span>
                            <button type="button" class="btn btn-ghost btn-xs" on:click=move |_| confirming.set(None)>"Cancelar"</button>
                            <button type="button" class="btn btn-error btn-xs" disabled=move || remove.pending().get()
                                on:click=move |_| { remove.dispatch(e.clone()); }>"Quitar"</button>
                        </div>
                    }
                })}
            </div>
            <div class="flex items-center gap-1 self-center text-sm">
                {p.manageable.then(|| view! {
                    <select class="select select-sm w-auto" aria-label=format!("Rol de {}", p.email)
                        prop:value=role.clone()
                        on:change=move |ev| { change.dispatch((email_role.clone(), event_target_value(&ev))); }>
                        <option value="editor" selected=role == "editor">"Editor"</option>
                        {me_owner.then(|| view! { <option value="admin" selected=role == "admin">"Administrador"</option> })}
                    </select>
                })}
                {p.manageable.then(|| view! {
                    <button type="button" aria-label=invite_label.clone() title=invite_label.clone()
                        class=if was_sent { "btn btn-ghost btn-sm btn-square text-success" } else { "btn btn-ghost btn-sm btn-square text-primary" }
                        on:click=move |_| on_invite.run((email_invite.clone(), joined))>
                        <ui::Icon class="size-5" name=if was_sent { "mail-check" } else { "mail" }/>
                    </button>
                })}
                {if p.manageable {
                    view! {
                        <button type="button" class="btn btn-ghost btn-sm text-base-content/70 hover:text-error" aria-label=label
                            on:click=move |_| confirming.set(Some(p.email.clone()))>"Quitar"</button>
                    }.into_any()
                } else if p.role == "owner" {
                    view! { <span class="text-xs text-base-content/60">"No se puede quitar"</span> }.into_any()
                } else {
                    view! { <span></span> }.into_any()
                }}
            </div>
        </li>
    }
}
