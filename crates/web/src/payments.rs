//! Cobros de una tienda, en la plataforma (`/tienda/:slug/cobros`):
//! vincular la cuenta de Mercado Pago para cobrar las ventas.

use crate::products::{error_text, slug_param, Shell, SignInFirst};
use domain::PaymentsInfo;
use leptos::prelude::*;
use leptos_router::hooks::use_query_map;

#[server]
pub async fn store_payments(slug: String) -> Result<Option<PaymentsInfo>, ServerFnError> {
    use crate::server::{internal, pool};
    let Some(tenant) = crate::products::server::owned_store(&slug).await? else {
        return Ok(None);
    };
    Ok(Some(PaymentsInfo {
        configured: crate::mp::config().is_some(),
        link: db::mp::link(&pool()?, tenant.id).await.map_err(internal)?,
        store_name: tenant.name,
    }))
}

/// Empieza "Vincular mi cuenta": devuelve la página de Mercado Pago.
#[server]
pub async fn mp_connect(slug: String) -> Result<String, ServerFnError> {
    use crate::server::{internal, pool};
    let tenant = crate::products::server::require_store(&slug).await?;
    let c = crate::mp::config()
        .ok_or_else(|| ServerFnError::new("Falta configurar Mercado Pago en MarketMaker."))?;
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
        <Shell>
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
        </Shell>
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
            "configuracion" => "Falta configurar Mercado Pago en MarketMaker.".to_string(),
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
    let busy = move || connect.pending().get() || connect.value().with(|v| matches!(v, Some(Ok(_))));
    view! {
        <div class="flex flex-wrap items-center gap-3">
            <a href="/" class="btn btn-ghost btn-sm">"← Tus tiendas"</a>
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
                    <p class="text-sm text-base-content/75">"Falta configurar la aplicación de Mercado Pago de MarketMaker (MP_CLIENT_ID y MP_CLIENT_SECRET en .env.local). Mientras tanto, los pedidos se pagan coordinando con el comprador."</p>
                }.into_any(),
                None => view! {
                    <h2 class="card-title">"Cobrá tus ventas con Mercado Pago"</h2>
                    <p class="text-sm text-base-content/75">"La plata va directo a tu cuenta. Mercado Pago te cobra una comisión por cada venta; MarketMaker no cobra nada."</p>
                    <button class="btn btn-primary mt-1" disabled=busy on:click=move |_| { problem.set(None); connect.dispatch(()); }>
                        {move || if busy() { "Abriendo Mercado Pago…" } else { "Vincular mi cuenta" }}
                    </button>
                    <p class="text-xs text-base-content/60">"Te lleva a Mercado Pago para iniciar sesión y aceptar. Tarda un minuto."</p>
                    <p class="text-sm text-base-content/75">"Mientras no la vincules, los pedidos se pagan coordinando con el comprador."</p>
                }.into_any(),
            }}
        </div></div>
    }
}
