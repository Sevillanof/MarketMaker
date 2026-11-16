//! Cobros de una tienda, en la plataforma (`/tienda/:slug/cobros`):
//! vincular la cuenta de Mercado Pago y elegir las otras formas de cobro
//! (transferencia, coordinar con el comprador).

use crate::products::{error_text, slug_param, SignInFirst};
use domain::{PaymentOptions, PaymentsInfo};
use leptos::prelude::*;
use leptos_router::hooks::use_query_map;

#[server]
pub async fn store_payments(slug: String) -> Result<Option<PaymentsInfo>, ServerFnError> {
    use crate::server::{internal, pool};
    let Some(tenant) = crate::products::server::owned_store(&slug).await? else {
        return Ok(None);
    };
    let pool = pool()?;
    Ok(Some(PaymentsInfo {
        configured: crate::mp::config().is_some(),
        link: db::mp::link(&pool, tenant.id).await.map_err(internal)?,
        options: db::orders::payment_options(&pool, tenant.id)
            .await
            .map_err(internal)?,
        store_name: tenant.name,
    }))
}

/// Guarda las formas de cobro además de Mercado Pago. Devuelve el problema
/// para mostrar, si hay.
#[server]
pub async fn save_payment_options(
    slug: String,
    options: PaymentOptions,
) -> Result<Option<String>, ServerFnError> {
    use crate::server::{internal, pool};
    let tenant = crate::products::server::require_store(&slug).await?;
    let mut options = options;
    if let Some(problem) = domain::tidy_payment_options(&mut options).into_iter().next() {
        return Ok(Some(problem));
    }
    db::orders::save_payment_options(&pool()?, tenant.id, &options)
        .await
        .map_err(internal)?;
    Ok(None)
}

/// Empieza "Vincular mi cuenta": devuelve la página de Mercado Pago.
#[server]
pub async fn mp_connect(slug: String) -> Result<String, ServerFnError> {
    use crate::server::{internal, pool};
    let tenant = crate::products::server::require_store(&slug).await?;
    let c = crate::mp::config().ok_or_else(|| {
        ServerFnError::new("Los cobros con Mercado Pago todavía no están disponibles.")
    })?;
    let state = crate::mp::new_state();
    db::mp::save_state(&pool()?, tenant.id, &state)
        .await
        .map_err(internal)?;
    Ok(crate::mp::authorize_url(&c, &state))
}

#[server]
pub async fn mp_unlink(slug: String) -> Result<(), ServerFnError> {
    use crate::server::{internal, pool};
    let tenant = crate::products::server::require_store(&slug).await?;
    db::mp::unlink(&pool()?, tenant.id).await.map_err(internal)
}

/// `/tienda/:slug/cobros`.
#[component]
pub fn PaymentsPage() -> impl IntoView {
    if matches!(crate::site::current_site(), domain::Site::Store(_)) {
        return view! { <crate::Home/> }.into_any();
    }
    let slug = slug_param();
    let data = Resource::new(move || slug.get(), store_payments);
    view! {
        <leptos_meta::Title text="Cobros · MarketMaker"/>
        <crate::panel::StoreShell slug=slug.get_untracked() section=crate::panel::Section::Payments>
            <div class="mx-auto flex max-w-3xl flex-col gap-5 px-4 py-8">
                <Suspense fallback=|| view! { <ui::Skeleton class="h-72 w-full"/> }>
                    {move || Suspend::new(async move {
                        match data.await {
                            Ok(Some(info)) => view! { <Payments slug=slug.get_untracked() info/> }.into_any(),
                            Ok(None) => view! { <SignInFirst back=format!("/tienda/{}/cobros", slug.get_untracked())/> }.into_any(),
                            Err(e) => view! { <div role="alert" class="alert alert-error">{error_text(&e)}</div> }.into_any(),
                        }
                    })}
                </Suspense>
            </div>
        </crate::panel::StoreShell>
    }
    .into_any()
}

fn mp_logo() -> impl IntoView {
    view! {
        <span class="inline-flex items-center gap-2 font-bold text-[#009ee3]">
            <span class="grid h-8 w-8 place-items-center rounded-full bg-[#009ee3] text-white" aria-hidden="true">
                <svg class="h-4 w-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.6" stroke-linecap="round" stroke-linejoin="round"><path d="M4 13c2.5 2 5 2 8 0s5.5-2 8 0"/></svg>
            </span>
            "Mercado Pago"
        </span>
    }
}

#[component]
fn Payments(slug: String, info: PaymentsInfo) -> impl IntoView {
    let query = use_query_map();
    let link = RwSignal::new(info.link);
    let problem = RwSignal::new(query.with_untracked(|q| {
        q.get("error").map(|e| match e.as_str() {
            "cancelado" => "No se vinculó: cancelaste en Mercado Pago.".to_string(),
            "configuracion" => {
                "Los cobros con Mercado Pago todavía no están disponibles.".to_string()
            }
            _ => "No pudimos vincular tu cuenta. Probá de nuevo en un rato.".to_string(),
        })
    }));
    let just_linked = query.with_untracked(|q| q.get("vinculada").is_some());
    let slug = StoredValue::new(slug);
    let connect = Action::new(move |_: &()| mp_connect(slug.get_value()));
    Effect::new(move |_| match connect.value().get() {
        Some(Ok(url)) => _ = window().location().set_href(&url),
        Some(Err(e)) => problem.set(Some(error_text(&e))),
        None => {}
    });
    let confirm = RwSignal::new(false);
    let unlink = Action::new(move |_: &()| mp_unlink(slug.get_value()));
    Effect::new(move |_| match unlink.value().get() {
        Some(Ok(())) => {
            link.set(None);
            confirm.set(false);
        }
        Some(Err(e)) => problem.set(Some(error_text(&e))),
        None => {}
    });
    let busy =
        move || connect.pending().get() || connect.value().with(|v| matches!(v, Some(Ok(_))));
    view! {
        <div class="flex flex-wrap items-center gap-3">
            <div class="grow">
                <h1 class="text-3xl font-bold">"Cobros"</h1>
                <p class="text-sm text-base-content/70">{info.store_name}</p>
            </div>
        </div>
        {move || problem.get().map(|e| view! { <div role="alert" class="alert alert-error text-sm">{e}</div> })}
        <div class="card bg-base-100 shadow-sm"><div class="card-body items-start gap-3">
            {mp_logo()}
            {move || match link.get() {
                Some(l) => view! {
                    {just_linked.then(|| view! { <div role="status" class="alert alert-success alert-soft w-full text-sm">"¡Listo! Tu tienda ya cobra con Mercado Pago."</div> })}
                    <h2 class="card-title text-success">"✓ Cuenta vinculada"</h2>
                    <p class="text-sm text-base-content/75">
                        {format!("Cuenta de Mercado Pago n.º {} · desde el {}", l.user_id, domain::order_time_ar(&l.linked_at).split(' ').next().unwrap_or(""))}
                        {(!l.live).then(|| view! { <span class="badge badge-warning badge-sm ml-2">"de prueba"</span> })}
                    </p>
                    <p class="text-sm text-base-content/75">"Tus compradores pagan en Mercado Pago y la plata va directo a tu cuenta. Cuando un pago se aprueba, el pedido pasa solo a \"Para preparar\"."</p>
                    {move || if confirm.get() {
                        view! {
                            <div class="flex flex-wrap items-center gap-2">
                                <span class="text-sm">"¿Desvincular? Tu tienda deja de cobrar online."</span>
                                <button class="btn btn-error btn-sm" disabled=move || unlink.pending().get() on:click=move |_| { unlink.dispatch(()); }>"Sí, desvincular"</button>
                                <button class="btn btn-ghost btn-sm" on:click=move |_| confirm.set(false)>"No"</button>
                            </div>
                        }.into_any()
                    } else {
                        view! { <button class="btn btn-ghost btn-sm" on:click=move |_| confirm.set(true)>"Desvincular"</button> }.into_any()
                    }}
                }.into_any(),
                None if !info.configured => view! {
                    <h2 class="card-title">"Cobrá tus ventas con Mercado Pago"</h2>
                    <p class="text-sm text-base-content/75">"Los cobros con Mercado Pago todavía no están disponibles. Mientras tanto, podés cobrar con transferencia o coordinando con el comprador."</p>
                }.into_any(),
                None => view! {
                    <h2 class="card-title">"Cobrá tus ventas con Mercado Pago"</h2>
                    <p class="text-sm text-base-content/75">"La plata va directo a tu cuenta. Mercado Pago te cobra una comisión por cada venta; MarketMaker no cobra nada."</p>
                    <button class="btn btn-primary mt-1" disabled=busy on:click=move |_| { problem.set(None); connect.dispatch(()); }>
                        {move || if busy() { "Abriendo Mercado Pago…" } else { "Vincular mi cuenta" }}
                    </button>
                    <p class="text-xs text-base-content/60">"Te lleva a Mercado Pago para iniciar sesión y aceptar. Tarda un minuto."</p>
                }.into_any(),
            }}
        </div></div>
        <OtherMethods slug=slug.get_value() options=info.options/>
    }
}

/// Transferencia y coordinar con el comprador: el pedido espera en "Esperando
/// pago" hasta que el dueño toca "Ya me pagó".
#[component]
fn OtherMethods(slug: String, options: PaymentOptions) -> impl IntoView {
    let transfer = RwSignal::new(options.transfer);
    let info = RwSignal::new(options.transfer_info);
    let coordinate = RwSignal::new(options.coordinate);
    let saved = RwSignal::new(false);
    let problem = RwSignal::new(None::<String>);
    let slug = StoredValue::new(slug);
    let save = Action::new(move |o: &PaymentOptions| save_payment_options(slug.get_value(), o.clone()));
    Effect::new(move |_| match save.value().get() {
        Some(Ok(None)) => saved.set(true),
        Some(Ok(Some(p))) => problem.set(Some(p)),
        Some(Err(e)) => problem.set(Some(error_text(&e))),
        None => {}
    });
    let changed = move || {
        saved.set(false);
        problem.set(None);
    };
    view! {
        <form class="card bg-base-100 shadow-sm" on:submit=move |ev| {
            ev.prevent_default();
            save.dispatch(PaymentOptions {
                transfer: transfer.get_untracked(),
                transfer_info: info.get_untracked(),
                coordinate: coordinate.get_untracked(),
            });
        }>
            <div class="card-body items-start gap-3">
                <h2 class="card-title">"Otras formas de cobro"</h2>
                <p class="text-sm text-base-content/75">"El pedido queda en \"Esperando pago\" y guarda las unidades. Cuando cobres, tocá \"Ya me pagó\" y pasa a \"Para preparar\". Si no lo marcás en 48 horas, se cancela solo y el stock vuelve."</p>
                <label class="flex cursor-pointer items-center gap-3">
                    <input type="checkbox" class="toggle toggle-primary" prop:checked=move || transfer.get()
                        on:change=move |ev| { transfer.set(event_target_checked(&ev)); changed(); }/>
                    <span class="font-semibold">"Transferencia"</span>
                </label>
                {move || transfer.get().then(|| view! {
                    <fieldset class="fieldset w-full max-w-md">
                        <label class="fieldset-legend" for="pay-transfer-info">"Adónde te transfieren"</label>
                        <textarea id="pay-transfer-info" class="textarea w-full" rows="3" maxlength=domain::TRANSFER_INFO_MAX
                            placeholder="Alias: mi.tienda.mp\nCBU o CVU: 0000003100000000000000\nTitular: Nombre Apellido"
                            aria-invalid=move || problem.get().is_some().to_string()
                            aria-describedby=move || problem.get().is_some().then_some("pay-transfer-error")
                            prop:value=move || info.get()
                            on:input=move |ev| { info.set(event_target_value(&ev)); changed(); }></textarea>
                        <p class="label">"Lo ve el comprador al confirmar el pedido y en el email."</p>
                        {move || problem.get().map(|p| view! { <p id="pay-transfer-error" class="label text-error">{p}</p> })}
                    </fieldset>
                })}
                <label class="flex cursor-pointer items-center gap-3">
                    <input type="checkbox" class="toggle toggle-primary" prop:checked=move || coordinate.get()
                        on:change=move |ev| { coordinate.set(event_target_checked(&ev)); changed(); }/>
                    <span class="font-semibold">"Coordinar el pago con el comprador"</span>
                </label>
                <p class="text-sm text-base-content/60">"Le escribís por WhatsApp y acuerdan cómo paga (efectivo al retirar, por ejemplo)."</p>
                {move || (!transfer.get() && problem.get().is_some()).then(|| view! {
                    <p class="text-sm text-error">{problem.get()}</p>
                })}
                <div class="flex items-center gap-3">
                    <button type="submit" class="btn btn-primary" disabled=move || save.pending().get()>
                        {move || if save.pending().get() { "Guardando…" } else { "Guardar" }}
                    </button>
                    <span role="status" class="text-sm text-success">{move || saved.get().then_some("Guardado")}</span>
                </div>
            </div>
        </form>
    }
}
