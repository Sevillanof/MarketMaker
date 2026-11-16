//! Ventas de una tienda, en la plataforma (`/tienda/:slug/ventas`): por
//! ahora, cómo entrega sus pedidos (retiro y envío).

use crate::products::{error_text, slug_param, SignInFirst};
use domain::DeliverySettings;
use leptos::prelude::*;

/// Nombre de la tienda `slug` y cómo entrega. `None` si no entró.
#[server]
pub async fn store_delivery(
    slug: String,
) -> Result<Option<(String, DeliverySettings)>, ServerFnError> {
    use crate::server::{internal, pool};
    let Some(tenant) = crate::products::server::owned_store(&slug).await? else {
        return Ok(None);
    };
    let d = db::orders::delivery(&pool()?, tenant.id)
        .await
        .map_err(internal)?;
    Ok(Some((tenant.name, d)))
}

/// Guarda cómo entrega la tienda. Devuelve los problemas (vacío si salió bien).
#[server(input = leptos::server_fn::codec::Json)]
pub async fn save_store_delivery(
    slug: String,
    delivery: DeliverySettings,
) -> Result<Vec<String>, ServerFnError> {
    use crate::server::{internal, pool};
    let tenant = crate::products::server::require_store(&slug).await?;
    let mut d = delivery;
    let errors = domain::tidy_delivery(&mut d);
    if errors.is_empty() {
        db::orders::save_delivery(&pool()?, tenant.id, &d)
            .await
            .map_err(internal)?;
    }
    Ok(errors)
}

/// `/tienda/:slug/ventas`.
#[component]
pub fn SalesPage() -> impl IntoView {
    if matches!(crate::site::current_site(), domain::Site::Store(_)) {
        return view! { <crate::Home/> }.into_any();
    }
    let slug = slug_param();
    let data = Resource::new(move || slug.get(), store_delivery);
    view! {
        <leptos_meta::Title text="Entregas · MarketMaker"/>
        <crate::panel::StoreShell slug=slug.get_untracked() section=crate::panel::Section::Delivery>
            <div class="mx-auto flex max-w-3xl flex-col gap-5 px-4 py-8">
                <Suspense fallback=|| view! { <ui::Skeleton class="h-96 w-full"/> }>
                    {move || Suspend::new(async move {
                        match data.await {
                            Ok(Some((_, d))) => view! { <DeliveryForm slug=slug.get_untracked() d/> }.into_any(),
                            Ok(None) => view! { <SignInFirst back=format!("/tienda/{}/ventas", slug.get_untracked())/> }.into_any(),
                            Err(e) => view! { <div role="alert" class="alert alert-error">{error_text(&e)}</div> }.into_any(),
                        }
                    })}
                </Suspense>
            </div>
        </crate::panel::StoreShell>
    }
    .into_any()
}

#[component]
fn DeliveryForm(slug: String, d: DeliverySettings) -> impl IntoView {
    let pickup = RwSignal::new(d.pickup_enabled);
    let address = RwSignal::new(d.pickup_address);
    let hours = RwSignal::new(d.pickup_hours);
    let shipping = RwSignal::new(d.shipping_enabled);
    // Vacío: a coordinar. "0": gratis.
    let cost = RwSignal::new(
        d.shipping_cents
            .map(domain::price_input)
            .unwrap_or_default(),
    );
    let note = RwSignal::new(d.shipping_note);
    let problems = RwSignal::new(Vec::<String>::new());
    let saved = RwSignal::new(false);
    let slug = StoredValue::new(slug);

    let save =
        Action::new(move |d: &DeliverySettings| save_store_delivery(slug.get_value(), d.clone()));
    Effect::new(move |_| match save.value().get() {
        Some(Ok(e)) => {
            saved.set(e.is_empty());
            problems.set(e);
        }
        Some(Err(e)) => problems.set(vec![error_text(&e)]),
        None => {}
    });
    let submit = move || {
        saved.set(false);
        let text = cost.get_untracked();
        let shipping_cents = if text.trim().is_empty() {
            None
        } else {
            match domain::parse_price(&text) {
                Some(c) => Some(c),
                None => {
                    problems.set(vec!["Escribí el costo del envío como 3500 o 3.500,50, o dejalo vacío para coordinarlo.".into()]);
                    return;
                }
            }
        };
        save.dispatch(DeliverySettings {
            pickup_enabled: pickup.get_untracked(),
            pickup_address: address.get_untracked(),
            pickup_hours: hours.get_untracked(),
            shipping_enabled: shipping.get_untracked(),
            shipping_cents,
            shipping_note: note.get_untracked(),
        });
    };
    let cost_hint = move || {
        let t = cost.get();
        if t.trim().is_empty() {
            "Vacío: el costo se coordina con cada comprador.".to_string()
        } else {
            match domain::parse_price(&t) {
                Some(0) => "El comprador ve envío gratis.".to_string(),
                Some(c) => format!("El comprador ve envío {}.", ui::format_price(c)),
                None => "Escribilo como 3500 o 3.500,50.".to_string(),
            }
        }
    };
    let dirty = move || {
        saved.set(false);
    };

    view! {
        <div class="flex flex-wrap items-center gap-3">
            <div class="grow">
                <h1 class="text-3xl font-bold">"Entregas"</h1>
                <p class="text-sm text-base-content/70">"Lo que elijas acá aparece en \"Finalizar compra\" de tu tienda."</p>
            </div>
        </div>
        <form class="card bg-base-100 shadow-sm" novalidate on:submit=move |ev| { ev.prevent_default(); submit(); }>
            <div class="card-body gap-4">
                {move || (!problems.get().is_empty()).then(|| view! {
                    <div role="alert" class="alert alert-error text-sm">
                        <ul>{problems.get().into_iter().map(|p| view! { <li>{p}</li> }).collect_view()}</ul>
                    </div>
                })}

                <div class="rounded-box border border-base-300 p-4">
                    <label class="flex cursor-pointer items-center gap-3">
                        <input type="checkbox" class="toggle toggle-primary" id="d-pickup"
                            prop:checked=move || pickup.get()
                            on:change=move |ev| { pickup.set(event_target_checked(&ev)); dirty(); }/>
                        <span><span class="block font-semibold">"Retiro en el local"</span>
                        <span class="block text-sm text-base-content/70">"El comprador pasa a buscar su pedido, sin costo."</span></span>
                    </label>
                    {move || pickup.get().then(|| view! {
                        <div class="mt-3 grid gap-x-4 sm:grid-cols-2">
                            <fieldset class="fieldset">
                                <label class="fieldset-legend" for="d-address">"Dirección"</label>
                                <input id="d-address" class="input w-full" maxlength=domain::PICKUP_ADDRESS_MAX placeholder="Av. Corrientes 1234, CABA"
                                    prop:value=move || address.get() on:input=move |ev| { address.set(event_target_value(&ev)); dirty(); }/>
                            </fieldset>
                            <fieldset class="fieldset">
                                <label class="fieldset-legend" for="d-hours">"Horarios " <span class="font-normal text-base-content/60">"(opcional)"</span></label>
                                <input id="d-hours" class="input w-full" maxlength=domain::PICKUP_HOURS_MAX placeholder="Lun a Vie de 10 a 18 h"
                                    prop:value=move || hours.get() on:input=move |ev| { hours.set(event_target_value(&ev)); dirty(); }/>
                            </fieldset>
                        </div>
                    })}
                </div>

                <div class="rounded-box border border-base-300 p-4">
                    <label class="flex cursor-pointer items-center gap-3">
                        <input type="checkbox" class="toggle toggle-primary" id="d-shipping"
                            prop:checked=move || shipping.get()
                            on:change=move |ev| { shipping.set(event_target_checked(&ev)); dirty(); }/>
                        <span><span class="block font-semibold">"Envío a domicilio"</span>
                        <span class="block text-sm text-base-content/70">"El comprador escribe su dirección al comprar."</span></span>
                    </label>
                    {move || shipping.get().then(|| view! {
                        <div class="mt-3 grid gap-x-4 sm:grid-cols-[200px_1fr]">
                            <fieldset class="fieldset">
                                <label class="fieldset-legend" for="d-cost">"Costo " <span class="font-normal text-base-content/60">"(opcional)"</span></label>
                                <label class="input w-full"><span class="text-base-content/60">"$"</span>
                                    <input id="d-cost" inputmode="decimal" placeholder="A coordinar" aria-describedby="d-cost-hint"
                                        prop:value=move || cost.get() on:input=move |ev| { cost.set(event_target_value(&ev)); dirty(); }/>
                                </label>
                                <p id="d-cost-hint" class="label whitespace-normal">{cost_hint}</p>
                            </fieldset>
                            <fieldset class="fieldset">
                                <label class="fieldset-legend" for="d-note">"Aviso para el comprador"</label>
                                <input id="d-note" class="input w-full" maxlength=domain::SHIPPING_NOTE_MAX placeholder="Lo coordinamos por WhatsApp"
                                    prop:value=move || note.get() on:input=move |ev| { note.set(event_target_value(&ev)); dirty(); }/>
                            </fieldset>
                        </div>
                    })}
                </div>

                <div class="flex items-center gap-3">
                    <button type="submit" class="btn btn-primary" disabled=move || save.pending().get()>
                        {move || if save.pending().get() { "Guardando…" } else { "Guardar" }}
                    </button>
                    <span role="status" class="text-sm text-success">{move || saved.get().then_some("✓ Guardado")}</span>
                </div>
            </div>
        </form>
    }
}
