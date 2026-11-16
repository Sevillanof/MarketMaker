//! Ventas de una tienda, en la plataforma (`/tienda/:slug/ventas`): por
//! ahora, cómo entrega sus pedidos (retiro y envío).

use crate::products::{error_text, slug_param, SignInFirst};
use crate::checkout::{error_for, focus_id, TextField};
use domain::{
    AndreaniAccount, AndreaniConnect, AndreaniPanel, AndreaniSaved, AndreaniService, AndreaniSettings,
    DeliverySettings, FieldError,
};
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

/// La cuenta de Andreani como la ve el dueño (sin el código ni el token).
#[cfg(feature = "ssr")]
fn account_view(
    acc: db::andreani::Account,
    store_name: &str,
    pickup_address: &str,
) -> AndreaniAccount {
    let available = domain::contract_services(&acc.contracts.0);
    let configured = acc.settings.is_some();
    let settings = match acc.settings {
        Some(s) => s.0,
        None => {
            // Primera vez: el origen se precarga con la dirección de retiro.
            let (street, number) = domain::split_street(pickup_address)
                .map(|(s, n, _)| (s, n))
                .unwrap_or_default();
            AndreaniSettings {
                enabled: true,
                origin: domain::AndreaniOrigin {
                    street,
                    number,
                    sender: store_name.to_string(),
                    ..Default::default()
                },
                services: available
                    .iter()
                    .copied()
                    .filter(|s| *s != AndreaniService::Bigger)
                    .collect(),
                parcel: domain::Parcel::default(),
            }
        }
    };
    AndreaniAccount {
        holder: acc.holder,
        cuit: acc.cuit,
        email: acc.email,
        client_type: acc.client_type,
        configured,
        available,
        settings,
    }
}

/// Envíos con Andreani de la tienda `slug`. `None` si no entró.
#[server]
pub async fn load_andreani(slug: String) -> Result<Option<AndreaniPanel>, ServerFnError> {
    use crate::server::{internal, pool};
    let Some(tenant) = crate::products::server::owned_store(&slug).await? else {
        return Ok(None);
    };
    let pool = pool()?;
    let d = db::orders::delivery(&pool, tenant.id).await.map_err(internal)?;
    let account = db::andreani::account(&pool, tenant.id)
        .await
        .map_err(internal)?
        .map(|a| account_view(a, &tenant.name, &d.pickup_address));
    Ok(Some(AndreaniPanel {
        account,
        store_name: tenant.name,
        pickup_address: d.pickup_address,
    }))
}

/// Conecta la cuenta de Andreani con el código que pegó el dueño.
#[server]
pub async fn connect_andreani(slug: String, code: String) -> Result<AndreaniConnect, ServerFnError> {
    use crate::andreani::Error;
    use crate::server::{internal, pool};
    let tenant = crate::products::server::require_store(&slug).await?;
    let code: String = code.chars().filter(|c| !c.is_whitespace()).collect();
    if code.is_empty() {
        return Ok(AndreaniConnect::Problem(
            "Pegá el código que te da Andreani.".into(),
        ));
    }
    if code.len() > 2000 {
        return Ok(AndreaniConnect::Problem(
            "Ese código no es válido. Copialo de nuevo desde Andreani y pegalo completo.".into(),
        ));
    }
    let key = format!("andreani-connect:{}", tenant.id);
    if !crate::limits::allow(&key, 10, std::time::Duration::from_secs(600)) {
        return Ok(AndreaniConnect::Problem(
            "Hiciste muchos intentos seguidos. Esperá unos minutos y probá de nuevo.".into(),
        ));
    }
    let info = match crate::andreani::login(&code).await {
        Ok(i) => i,
        Err(Error::Unauthorized) => {
            return Ok(AndreaniConnect::Problem(
                "Ese código no es válido. Copialo de nuevo desde Andreani (opción WooCommerce) y pegalo completo.".into(),
            ))
        }
        Err(e) => {
            leptos::logging::warn!("Andreani: no se pudo conectar: {e}");
            return Ok(AndreaniConnect::Problem(
                "Andreani no responde en este momento. Probá de nuevo en unos minutos.".into(),
            ));
        }
    };
    if info.contracts.is_empty() {
        return Ok(AndreaniConnect::Problem(
            "Tu cuenta de Andreani todavía no tiene servicios de envío habilitados. Completá el alta en Andreani y probá de nuevo.".into(),
        ));
    }
    let pool = pool()?;
    db::andreani::connect(
        &pool,
        tenant.id,
        &db::andreani::Login {
            credential: &code,
            access_token: &info.access_token,
            client_type: &info.client_type,
            holder: &info.holder,
            cuit: &info.cuit,
            email: &info.email,
            contracts: &info.contracts,
        },
    )
    .await
    .map_err(internal)?;
    let d = db::orders::delivery(&pool, tenant.id).await.map_err(internal)?;
    let acc = db::andreani::account(&pool, tenant.id)
        .await
        .map_err(internal)?
        .ok_or_else(|| ServerFnError::new("cuenta"))?;
    Ok(AndreaniConnect::Connected(account_view(
        acc,
        &tenant.name,
        &d.pickup_address,
    )))
}

/// Guarda lo que eligió el dueño para Andreani y le manda el origen.
#[server(input = leptos::server_fn::codec::Json)]
pub async fn save_andreani(
    slug: String,
    settings: AndreaniSettings,
) -> Result<AndreaniSaved, ServerFnError> {
    use crate::server::{internal, pool};
    let tenant = crate::products::server::require_store(&slug).await?;
    let pool = pool()?;
    let Some(acc) = db::andreani::account(&pool, tenant.id).await.map_err(internal)? else {
        return Ok(AndreaniSaved {
            errors: vec![FieldError {
                field: "account".into(),
                message: "Conectá tu cuenta de Andreani primero.".into(),
            }],
            warning: None,
        });
    };
    let available = domain::contract_services(&acc.contracts.0);
    let mut s = settings;
    let errors = domain::tidy_andreani(&mut s, &available);
    if !errors.is_empty() {
        return Ok(AndreaniSaved { errors, warning: None });
    }
    db::andreani::save_settings(&pool, tenant.id, &s)
        .await
        .map_err(internal)?;
    let warning = match crate::andreani::save_origin(&pool, tenant.id, &s.origin).await {
        Ok(()) => None,
        Err(e) => {
            leptos::logging::warn!("Andreani: no se pudo guardar el origen: {e}");
            Some("Guardamos los cambios, pero Andreani no tomó la dirección de origen. Volvé a guardar en unos minutos.".into())
        }
    };
    Ok(AndreaniSaved { errors: Vec::new(), warning })
}

/// Desconecta la cuenta de Andreani (los envíos ya creados siguen en Andreani).
#[server]
pub async fn disconnect_andreani(slug: String) -> Result<(), ServerFnError> {
    use crate::server::{internal, pool};
    let tenant = crate::products::server::require_store(&slug).await?;
    db::andreani::disconnect(&pool()?, tenant.id)
        .await
        .map_err(internal)
}

/// `/tienda/:slug/ventas`.
#[component]
pub fn SalesPage() -> impl IntoView {
    if matches!(crate::site::current_site(), domain::Site::Store(_)) {
        return view! { <crate::Home/> }.into_any();
    }
    let slug = slug_param();
    let data = Resource::new(move || slug.get(), store_delivery);
    let andreani = Resource::new(move || slug.get(), load_andreani);
    view! {
        <leptos_meta::Title text="Entregas · MarketMaker"/>
        <crate::panel::StoreShell slug=slug.get_untracked() section=crate::panel::Section::Delivery>
            <div class="mx-auto flex max-w-3xl flex-col gap-5 px-4 py-8">
                <Suspense fallback=|| view! { <ui::Skeleton class="h-96 w-full"/> }>
                    {move || Suspend::new(async move {
                        match (data.await, andreani.await) {
                            (Ok(Some((_, d))), Ok(Some(panel))) => view! {
                                <DeliveryForm slug=slug.get_untracked() d/>
                                <AndreaniCard slug=slug.get_untracked() panel/>
                            }.into_any(),
                            (Ok(None), _) | (_, Ok(None)) => view! { <SignInFirst back=format!("/tienda/{}/ventas", slug.get_untracked())/> }.into_any(),
                            (Err(e), _) | (_, Err(e)) => view! { <div role="alert" class="alert alert-error">{error_text(&e)}</div> }.into_any(),
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
            andreani: Vec::new(),
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

/// Envíos con Andreani: conectar la cuenta y elegir qué ofrecer.
#[component]
fn AndreaniCard(slug: String, panel: AndreaniPanel) -> impl IntoView {
    let slug = StoredValue::new(slug);
    // Lo que se ve ahora: se cambia al conectar o desconectar sin recargar.
    let account = RwSignal::new(panel.account);
    view! {
        <section class="card bg-base-100 shadow-sm" aria-labelledby="a-title">
            <div class="card-body gap-4">
                <div>
                    <h2 id="a-title" class="text-xl font-bold">"Envíos con Andreani"</h2>
                    <p class="text-sm text-base-content/70">
                        "Tus compradores ven el costo real del envío y eligen domicilio o sucursal. Vos imprimís la etiqueta y despachás."
                    </p>
                </div>
                {move || match account.get() {
                    Some(acc) => view! {
                        <AndreaniConnected slug=slug.get_value() acc
                            on_disconnect=Callback::new(move |_| account.set(None))/>
                    }.into_any(),
                    None => view! {
                        <AndreaniConnectSteps slug=slug.get_value()
                            on_connect=Callback::new(move |a| account.set(Some(a)))/>
                    }.into_any(),
                }}
            </div>
        </section>
    }
}

/// Los tres pasos para conectar la cuenta.
#[component]
fn AndreaniConnectSteps(slug: String, on_connect: Callback<AndreaniAccount>) -> impl IntoView {
    let code = RwSignal::new(String::new());
    let problem = RwSignal::new(None::<String>);
    let slug = StoredValue::new(slug);
    let connect = Action::new(move |code: &String| connect_andreani(slug.get_value(), code.clone()));
    Effect::new(move |_| match connect.value().get() {
        Some(Ok(AndreaniConnect::Connected(a))) => on_connect.run(a),
        Some(Ok(AndreaniConnect::Problem(m))) => {
            problem.set(Some(m));
            focus_id("a-code".into());
        }
        Some(Err(e)) => problem.set(Some(error_text(&e))),
        None => {}
    });
    let submit = move || {
        if code.get_untracked().trim().is_empty() {
            problem.set(Some("Pegá el código que te da Andreani.".into()));
            focus_id("a-code".into());
            return;
        }
        problem.set(None);
        connect.dispatch(code.get_untracked());
    };
    let step = "flex size-7 shrink-0 items-center justify-center rounded-full bg-primary text-sm font-bold text-primary-content";
    view! {
        <ol class="flex flex-col gap-4">
            <li class="flex gap-3">
                <span class=step aria-hidden="true">"1"</span>
                <div>
                    <p class="font-semibold">"Creá tu cuenta PyME en Andreani"</p>
                    <p class="text-sm text-base-content/70">"Es gratis. Te pide tu CUIL o CUIT. Si ya tenés cuenta, seguí al paso 2."</p>
                    <a class="link link-primary text-sm" href=domain::ANDREANI_SIGNUP_URL target="_blank" rel="noopener noreferrer">
                        "Ir a Andreani PyMEs ↗"
                    </a>
                </div>
            </li>
            <li class="flex gap-3">
                <span class=step aria-hidden="true">"2"</span>
                <div>
                    <p class="font-semibold">"Copiá tu código de integración"</p>
                    <p class="text-sm text-base-content/70">"En Andreani, entrá a Integraciones, elegí WooCommerce y copiá el código que aparece."</p>
                    <a class="link link-primary text-sm" href=domain::ANDREANI_CODE_URL target="_blank" rel="noopener noreferrer">
                        "Abrir Integraciones ↗"
                    </a>
                </div>
            </li>
            <li class="flex gap-3">
                <span class=step aria-hidden="true">"3"</span>
                <form class="grow" novalidate on:submit=move |ev| { ev.prevent_default(); submit(); }>
                    <label class="font-semibold" for="a-code">"Pegalo acá"</label>
                    <div class="mt-1 flex flex-wrap gap-2">
                        <input id="a-code" class="input grow font-mono" class:input-error=move || problem.get().is_some()
                            autocomplete="off" autocapitalize="none" spellcheck="false"
                            placeholder="Código de Andreani"
                            aria-invalid=move || problem.get().is_some().to_string()
                            aria-describedby=move || problem.get().is_some().then_some("a-code-error")
                            prop:value=move || code.get()
                            on:input=move |ev| { code.set(event_target_value(&ev)); problem.set(None); }/>
                        <button type="submit" class="btn btn-primary" disabled=move || connect.pending().get()>
                            {move || if connect.pending().get() { "Conectando…" } else { "Conectar" }}
                        </button>
                    </div>
                    <p id="a-code-error" role="status" class="mt-1 text-sm text-error">{move || problem.get()}</p>
                </form>
            </li>
        </ol>
    }
}

/// La cuenta conectada: qué se ofrece, desde dónde se despacha y el paquete
/// de la tienda.
#[component]
fn AndreaniConnected(
    slug: String,
    acc: AndreaniAccount,
    on_disconnect: Callback<()>,
) -> impl IntoView {
    let slug = StoredValue::new(slug);
    let s = acc.settings.clone();
    let enabled = RwSignal::new(s.enabled);
    let services = RwSignal::new(s.services.clone());
    let sender = RwSignal::new(s.origin.sender.clone());
    let street = RwSignal::new(s.origin.street.clone());
    let number = RwSignal::new(s.origin.number.clone());
    let floor = RwSignal::new(s.origin.floor.clone());
    let city = RwSignal::new(s.origin.city.clone());
    let province = RwSignal::new(s.origin.province.clone());
    let postal = RwSignal::new(s.origin.postal_code.clone());
    let weight = RwSignal::new(domain::kg_input(s.parcel.weight_g));
    let length = RwSignal::new(domain::cm_input(s.parcel.length_mm));
    let width = RwSignal::new(domain::cm_input(s.parcel.width_mm));
    let height = RwSignal::new(domain::cm_input(s.parcel.height_mm));
    let errors = RwSignal::new(Vec::<FieldError>::new());
    let saved = RwSignal::new(false);
    let warning = RwSignal::new(None::<String>);
    let configured = RwSignal::new(acc.configured);
    let confirm_off = RwSignal::new(false);

    let save = Action::new(move |s: &AndreaniSettings| save_andreani(slug.get_value(), s.clone()));
    Effect::new(move |_| match save.value().get() {
        Some(Ok(r)) => {
            saved.set(r.errors.is_empty());
            if r.errors.is_empty() {
                configured.set(true);
            }
            focus_first(&r.errors);
            errors.set(r.errors);
            warning.set(r.warning);
        }
        Some(Err(e)) => warning.set(Some(error_text(&e))),
        None => {}
    });
    let disconnect = Action::new(move |_: &()| disconnect_andreani(slug.get_value()));
    Effect::new(move |_| {
        if let Some(Ok(())) = disconnect.value().get() {
            on_disconnect.run(());
        }
    });

    let submit = move || {
        saved.set(false);
        warning.set(None);
        let parcel = match domain::parse_parcel(
            &weight.get_untracked(),
            &length.get_untracked(),
            &width.get_untracked(),
            &height.get_untracked(),
        ) {
            Ok(Some(p)) => p,
            Ok(None) => {
                let e = vec![FieldError {
                    field: "parcel".into(),
                    message: "Escribí el peso y las medidas del paquete de tu tienda.".into(),
                }];
                focus_first(&e);
                errors.set(e);
                return;
            }
            Err(m) => {
                let e = vec![FieldError { field: "parcel".into(), message: m }];
                focus_first(&e);
                errors.set(e);
                return;
            }
        };
        save.dispatch(AndreaniSettings {
            enabled: enabled.get_untracked(),
            origin: domain::AndreaniOrigin {
                street: street.get_untracked(),
                number: number.get_untracked(),
                floor: floor.get_untracked(),
                city: city.get_untracked(),
                province: province.get_untracked(),
                postal_code: postal.get_untracked(),
                sender: sender.get_untracked(),
            },
            services: services.get_untracked(),
            parcel,
        });
    };
    let dirty = move || saved.set(false);
    let clear = move |field: &'static str| {
        dirty();
        errors.update(|e| e.retain(|e| e.field != field));
    };
    // Cada cambio de un campo borra "✓ Guardado".
    Effect::new(move |first: Option<()>| {
        (
            sender.track(),
            street.track(),
            number.track(),
            floor.track(),
            city.track(),
            enabled.track(),
            services.track(),
        );
        if first.is_some() {
            dirty();
        }
    });

    let holder = if acc.holder.is_empty() { acc.email.clone() } else { acc.holder.clone() };
    let cuit = (!acc.cuit.is_empty()).then(|| format!("CUIT {}", acc.cuit));
    let offer: Vec<AndreaniService> = acc
        .available
        .iter()
        .copied()
        .filter(|s| *s != AndreaniService::Bigger)
        .collect();
    let has_bigger = acc.available.contains(&AndreaniService::Bigger);
    let parcel_error = move || error_for(errors, "parcel");
    let services_error = move || error_for(errors, "services");
    let province_error = move || error_for(errors, "province");

    view! {
        <div class="flex flex-wrap items-center gap-3 rounded-box bg-success/10 px-4 py-3">
            <span class="grow text-sm">
                <span class="font-semibold text-success">"✓ Conectado"</span>
                " como " <span class="font-semibold">{holder}</span>
                {cuit.map(|c| view! { <span class="text-base-content/70">" · " {c}</span> })}
            </span>
            {move || if confirm_off.get() {
                view! {
                    <span class="flex items-center gap-2 text-sm">
                        "¿Desconectar Andreani?"
                        <button type="button" class="btn btn-sm btn-error" disabled=move || disconnect.pending().get()
                            on:click=move |_| { disconnect.dispatch(()); }>"Sí, desconectar"</button>
                        <button type="button" class="btn btn-sm btn-ghost" on:click=move |_| confirm_off.set(false)>"No"</button>
                    </span>
                }.into_any()
            } else {
                view! {
                    <button type="button" class="btn btn-sm btn-ghost" on:click=move |_| confirm_off.set(true)>"Desconectar"</button>
                }.into_any()
            }}
        </div>
        {move || (!configured.get()).then(|| view! {
            <p class="text-sm">"Revisá desde dónde despachás y tocá " <b>"Guardar"</b> " para empezar a ofrecer Andreani."</p>
        })}

        <form class="flex flex-col gap-4" novalidate on:submit=move |ev| { ev.prevent_default(); submit(); }>
            <label class="flex cursor-pointer items-center gap-3">
                <input type="checkbox" class="toggle toggle-primary" id="a-enabled"
                    prop:checked=move || enabled.get()
                    on:change=move |ev| enabled.set(event_target_checked(&ev))/>
                <span class="font-semibold">"Ofrecer Andreani en \"Finalizar compra\""</span>
            </label>

            <fieldset class="fieldset" aria-describedby=move || services_error().is_some().then_some("f-services-error")>
                <legend class="fieldset-legend">"Qué ofrecés"</legend>
                {offer.into_iter().enumerate().map(|(i, svc)| {
                    let id = if i == 0 { "f-services".to_string() } else { format!("a-svc-{i}") };
                    view! {
                        <label class="flex cursor-pointer items-start gap-3 py-1">
                            <input type="checkbox" class="checkbox checkbox-primary mt-0.5" id=id
                                prop:checked=move || services.with(|v| v.contains(&svc))
                                on:change=move |ev| {
                                    let on = event_target_checked(&ev);
                                    services.update(|v| { v.retain(|x| *x != svc); if on { v.push(svc); } });
                                    clear("services");
                                }/>
                            <span><span class="block font-medium">{svc.label()}</span>
                            <span class="block text-sm text-base-content/70">{svc.owner_hint()}</span></span>
                        </label>
                    }
                }).collect_view()}
                {has_bigger.then(|| view! {
                    <p class="label whitespace-normal">"Los paquetes de más de 50 kg o muy grandes van como Andreani Bigger, solo cuando hace falta."</p>
                })}
                {move || services_error().map(|m| view! { <p id="f-services-error" class="label text-error">{m}</p> })}
            </fieldset>

            <div>
                <h3 class="font-semibold">"Desde dónde despachás"</h3>
                <p class="text-sm text-base-content/70">"Sale en la etiqueta como remitente."</p>
                <div class="mt-1 grid gap-x-4 sm:grid-cols-6">
                    <div class="sm:col-span-6"><TextField id="sender" label="Nombre del remitente" value=sender errors autocomplete="organization" max=60/></div>
                    <div class="sm:col-span-3"><TextField id="street" label="Calle" value=street errors autocomplete="address-line1" placeholder="Av. Corrientes" max=80/></div>
                    <div class="sm:col-span-1"><TextField id="number" label="Número" value=number errors placeholder="1234" max=10/></div>
                    <fieldset class="fieldset sm:col-span-2">
                        <label class="fieldset-legend" for="a-floor">"Piso y depto " <span class="font-normal text-base-content/60">"(opcional)"</span></label>
                        <input id="a-floor" class="input w-full" maxlength=30 autocomplete="address-line2" placeholder="3° B"
                            prop:value=move || floor.get() on:input=move |ev| floor.set(event_target_value(&ev))/>
                    </fieldset>
                    <div class="sm:col-span-2"><TextField id="city" label="Localidad" value=city errors autocomplete="address-level2" max=60/></div>
                    <fieldset class="fieldset sm:col-span-2">
                        <label class="fieldset-legend" for="f-province">"Provincia"</label>
                        <select id="f-province" class="select w-full" class:select-error=move || province_error().is_some()
                            autocomplete="address-level1" required
                            aria-invalid=move || province_error().is_some().to_string()
                            aria-describedby=move || province_error().is_some().then_some("f-province-error")
                            on:change=move |ev| { province.set(event_target_value(&ev)); clear("province"); }>
                            <option value="" disabled selected=move || province.get().is_empty()>"Elegí"</option>
                            {domain::PROVINCES.iter().map(|p| view! {
                                <option value=*p selected=move || province.get() == *p>{*p}</option>
                            }).collect_view()}
                        </select>
                        {move || province_error().map(|m| view! { <p id="f-province-error" class="label text-error">{m}</p> })}
                    </fieldset>
                    <div class="sm:col-span-2"><TextField id="postal_code" label="Código postal" value=postal errors autocomplete="postal-code" placeholder="1425" max=10/></div>
                </div>
            </div>

            <fieldset class="fieldset" aria-describedby="a-parcel-hint">
                <legend class="fieldset-legend">"Paquete de tu tienda"</legend>
                <p id="a-parcel-hint" class="label whitespace-normal">
                    "Se usa para cotizar los productos que no tienen peso y medidas propios. Lo podés cambiar en cada producto."
                </p>
                <div class="grid grid-cols-2 gap-2 sm:grid-cols-4">
                    <ParcelInput id="f-parcel" label="Peso" unit="kg" value=weight on_input=Callback::new(move |_| clear("parcel")) invalid=Signal::derive(move || parcel_error().is_some())/>
                    <ParcelInput id="a-length" label="Largo" unit="cm" value=length on_input=Callback::new(move |_| clear("parcel")) invalid=Signal::derive(move || parcel_error().is_some())/>
                    <ParcelInput id="a-width" label="Ancho" unit="cm" value=width on_input=Callback::new(move |_| clear("parcel")) invalid=Signal::derive(move || parcel_error().is_some())/>
                    <ParcelInput id="a-height" label="Alto" unit="cm" value=height on_input=Callback::new(move |_| clear("parcel")) invalid=Signal::derive(move || parcel_error().is_some())/>
                </div>
                {move || parcel_error().map(|m| view! { <p class="label whitespace-normal text-error">{m}</p> })}
            </fieldset>

            <div class="flex flex-wrap items-center gap-3">
                <button type="submit" class="btn btn-primary" disabled=move || save.pending().get()>
                    {move || if save.pending().get() { "Guardando…" } else { "Guardar" }}
                </button>
                <span role="status" class="text-sm">
                    {move || saved.get().then(|| view! { <span class="text-success">"✓ Guardado"</span> })}
                    {move || warning.get().map(|w| view! { <span class="text-warning">{w}</span> })}
                </span>
            </div>
        </form>
    }
}

/// Un número con su unidad (peso o medida del paquete).
#[component]
pub(crate) fn ParcelInput(
    id: &'static str,
    label: &'static str,
    unit: &'static str,
    value: RwSignal<String>,
    #[prop(optional, into)] on_input: Option<Callback<()>>,
    #[prop(optional, into)] invalid: Signal<bool>,
) -> impl IntoView {
    view! {
        <div>
            <label class="text-sm" for=id>{label}</label>
            <label class="input w-full" class:input-error=move || invalid.get()>
                <input id=id inputmode="decimal" autocomplete="off" placeholder="0"
                    aria-invalid=move || invalid.get().to_string()
                    prop:value=move || value.get()
                    on:input=move |ev| {
                        value.set(event_target_value(&ev));
                        if let Some(f) = on_input { f.run(()); }
                    }/>
                <span class="text-base-content/60">{unit}</span>
            </label>
        </div>
    }
}

/// Lleva el foco al primer campo con problema.
fn focus_first(errors: &[FieldError]) {
    if let Some(first) = errors.first() {
        focus_id(format!("f-{}", first.field));
    }
}
