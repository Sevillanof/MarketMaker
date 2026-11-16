//! Botón de arrepentimiento (`/arrepentimiento` en cada tienda): el
//! comprador pide revocar una compra con su nombre, email y número de
//! pedido. Recibe un código en pantalla y por email, y los dueños un email
//! para coordinar la devolución (Ley 24.240, art. 34, y Res. 424/2020).

use crate::account::FieldNote;
use domain::{FieldError, PageView, RegretForm, RegretOutcome};
use leptos::prelude::*;

/// Registra el pedido de arrepentimiento y avisa por email.
#[server(input = leptos::server_fn::codec::Json)]
pub async fn send_regret(form: RegretForm) -> Result<RegretOutcome, ServerFnError> {
    use crate::server::{internal, pool};
    let tenant = crate::auth::current_tenant().await?;
    let mut form = form;
    let number = match domain::tidy_regret(&mut form) {
        Ok(n) => n,
        Err(errors) => return Ok(RegretOutcome::Invalid { errors }),
    };
    // Frena a quien prueba números de pedido en serie.
    let ip = crate::limits::client_ip();
    if !crate::limits::allow(
        &format!("regret|{}|{ip}", tenant.id),
        6,
        std::time::Duration::from_secs(30 * 60),
    ) {
        return Ok(RegretOutcome::Problem(
            "Recibimos varios intentos seguidos desde tu conexión. Esperá unos minutos y probá de nuevo.".into(),
        ));
    }
    let pool = pool()?;
    let Some((id, code, first)) = db::orders::request_regret(&pool, tenant.id, number, &form.email)
        .await
        .map_err(internal)?
    else {
        return Ok(RegretOutcome::Invalid {
            errors: vec![FieldError {
                field: "number".into(),
                message: "No encontramos un pedido con ese número y ese email. Revisá el email de confirmación de tu compra.".into(),
            }],
        });
    };
    if first {
        let order = db::orders::get(&pool, tenant.id, id).await.map_err(internal)?;
        let owners = db::mp::owner_emails(&pool, tenant.id).await.map_err(internal)?;
        if let Some(o) = order {
            crate::email::send(crate::email::regret_to_buyer(&o.email, &tenant.name, number, &code));
            crate::email::send(crate::email::regret_to_owner(
                owners,
                &tenant.name,
                &tenant.slug,
                &o,
                &code,
                &form.reason,
            ));
        }
    }
    Ok(RegretOutcome::Sent { code, number })
}

/// `/arrepentimiento` en una tienda.
#[component]
pub fn RegretPage() -> impl IntoView {
    if crate::site::current_site() == domain::Site::Platform {
        return view! { <crate::platform::NotAStore/> }.into_any();
    }
    let page = Resource::new(|| (), |_| crate::load_page("home".to_string()));
    view! {
        <leptos_meta::Title text="Botón de arrepentimiento"/>
        <Suspense fallback=|| view! { <ui::PageSkeleton/> }>
            {move || Suspend::new(async move {
                match page.await {
                    Ok(view) => view! { <Regret view/> }.into_any(),
                    Err(_) => view! { <crate::platform::StoreNotFound/> }.into_any(),
                }
            })}
        </Suspense>
    }
    .into_any()
}

const FIELD_IDS: &[(&str, &str)] = &[("name", "r-name"), ("email", "r-email"), ("number", "r-number")];

#[component]
fn Regret(view: PageView) -> impl IntoView {
    let store_name = view.store_name.clone();
    let name = RwSignal::new(String::new());
    let email = RwSignal::new(String::new());
    let number = RwSignal::new(String::new());
    let reason = RwSignal::new(String::new());
    let errors = RwSignal::new(Vec::<FieldError>::new());
    let problem = RwSignal::new(None::<String>);
    let done = RwSignal::new(None::<(String, i32)>);

    let send = Action::new(|f: &RegretForm| send_regret(f.clone()));
    Effect::new(move |_| match send.value().get() {
        Some(Ok(RegretOutcome::Sent { code, number })) => done.set(Some((code, number))),
        Some(Ok(RegretOutcome::Invalid { errors: e })) => {
            if let Some(id) = e
                .first()
                .and_then(|f| FIELD_IDS.iter().find(|(k, _)| *k == f.field))
                .map(|(_, id)| *id)
            {
                crate::platform::focus_id(id);
            }
            errors.set(e);
        }
        Some(Ok(RegretOutcome::Problem(p))) => problem.set(Some(p)),
        Some(Err(_)) => problem.set(Some("No pudimos enviarlo. Revisá tu conexión y probá de nuevo.".into())),
        None => {}
    });
    let error_for = move |field: &'static str| {
        move || {
            errors
                .get()
                .into_iter()
                .find(|e| e.field == field)
                .map(|e| view! { <FieldNote ok=false text=e.message/> })
        }
    };
    let invalid = move |field: &'static str| move || errors.get().iter().any(|e| e.field == field).to_string();
    let submit = move || {
        errors.set(Vec::new());
        problem.set(None);
        send.dispatch(RegretForm {
            name: name.get_untracked(),
            email: email.get_untracked(),
            number: number.get_untracked(),
            reason: reason.get_untracked(),
        });
    };

    view! {
        <crate::checkout::StoreFrame view>
            <crate::checkout::StoreHeader name=store_name.clone()/>
            <div class="mx-auto w-full max-w-xl">
                {move || match done.get() {
                    Some((code, n)) => view! {
                        <div class="card bg-base-100 shadow-sm"><div class="card-body gap-3" role="status">
                            <h1 class="mk-h text-2xl font-bold">"Recibimos tu pedido"</h1>
                            <p>{format!("Registramos que querés arrepentirte de la compra #{n}. Tu código es:")}</p>
                            <p class="text-2xl font-bold tracking-wider">{code}</p>
                            <p class="text-base-content/75">"Te lo mandamos también por email. La tienda se va a comunicar con vos para coordinar la devolución y el reintegro."</p>
                            <a class="btn btn-primary self-start" href="/">"Volver a la tienda"</a>
                        </div></div>
                    }.into_any(),
                    None => view! {
                        <form class="card bg-base-100 shadow-sm" novalidate on:submit=move |ev| { ev.prevent_default(); submit(); }>
                            <div class="card-body gap-4">
                                <div>
                                    <h1 class="mk-h text-2xl font-bold">"Botón de arrepentimiento"</h1>
                                    <p class="mt-1 text-base-content/75">"Tenés 10 días corridos desde que recibiste tu compra para arrepentirte, sin dar motivos y sin costo. Completá estos datos y te mandamos un código."</p>
                                </div>
                                <fieldset class="fieldset">
                                    <label class="fieldset-legend" for="r-name">"Tu nombre"</label>
                                    <input id="r-name" class="input w-full" autocomplete="name" maxlength="80"
                                        aria-invalid=invalid("name") prop:value=move || name.get()
                                        on:input=move |ev| name.set(event_target_value(&ev))/>
                                    {error_for("name")}
                                </fieldset>
                                <fieldset class="fieldset">
                                    <label class="fieldset-legend" for="r-email">"Email de la compra"</label>
                                    <input id="r-email" class="input w-full" type="email" autocomplete="email" maxlength="120"
                                        aria-invalid=invalid("email") prop:value=move || email.get()
                                        on:input=move |ev| email.set(event_target_value(&ev))/>
                                    {error_for("email")}
                                </fieldset>
                                <fieldset class="fieldset">
                                    <label class="fieldset-legend" for="r-number">"Número de pedido"</label>
                                    <input id="r-number" class="input w-full" inputmode="numeric" placeholder="1001" maxlength="12"
                                        aria-invalid=invalid("number") prop:value=move || number.get()
                                        on:input=move |ev| number.set(event_target_value(&ev))/>
                                    {error_for("number")}
                                </fieldset>
                                <fieldset class="fieldset">
                                    <label class="fieldset-legend" for="r-reason">"Comentario " <span class="font-normal text-base-content/60">"(opcional)"</span></label>
                                    <textarea id="r-reason" class="textarea w-full" rows="3" maxlength="500"
                                        prop:value=move || reason.get()
                                        on:input=move |ev| reason.set(event_target_value(&ev))></textarea>
                                </fieldset>
                                <div class="flex flex-col items-start gap-2">
                                    <button type="submit" class="btn btn-primary" disabled=move || send.pending().get()>
                                        {move || if send.pending().get() { "Enviando…" } else { "Enviar pedido de arrepentimiento" }}
                                    </button>
                                    {move || problem.get().map(|text| view! { <FieldNote ok=false text/> })}
                                </div>
                            </div>
                        </form>
                    }.into_any(),
                }}
            </div>
        </crate::checkout::StoreFrame>
    }
}
