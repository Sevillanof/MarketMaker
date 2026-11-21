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
        configured: crate::mp::config_for(&tenant.slug).is_some(),
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
    if let Some(problem) = domain::tidy_payment_options(&mut options) {
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
    let c = crate::mp::config_for(&tenant.slug).ok_or_else(|| {
        ServerFnError::new("Los cobros con Mercado Pago todavía no están disponibles.")
    })?;
    // El permiso queda atado a esta sesión: al volver de Mercado Pago solo
    // vale en el mismo navegador que lo empezó.
    let session = crate::auth::session_token()
        .ok_or_else(|| ServerFnError::new("Tu sesión venció. Entrá de nuevo."))?;
    let state = crate::mp::new_state();
    db::mp::save_state(&pool()?, tenant.id, &state, &session)
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
            <div class="mx-auto flex max-w-3xl flex-col gap-5 px-4 py-10 max-sm:py-5">
                <Suspense fallback=|| view! { <ui::PanelSkeleton kind=ui::PanelSkeletonKind::Form/> }>
                    {move || Suspend::new(async move {
                        match data.await {
                            Ok(Some(info)) => view! { <Payments slug=slug.get_untracked() info/> }.into_any(),
                            Ok(None) => view! { <SignInFirst back=format!("/tienda/{}/cobros", slug.get_untracked())/> }.into_any(),
                            Err(e) => view! { <div role="alert" class="alert alert-error">{error_text(&e)}</div> }.into_any(),
                        }
                    })}
                </Suspense>
            </div>
    }
    .into_any()
}

/// Ícono redondo de una forma de cobro.
pub(crate) fn method_icon(kind: &'static str) -> impl IntoView {
    let (bg, path) = match kind {
        "mp" => (
            "bg-[#009ee3] text-white rounded-full",
            "M4 13c2.5 2 5 2 8 0s5.5-2 8 0",
        ),
        "transfer" => (
            "bg-base-200 rounded-field",
            "M3 10h18M5 10v8m4-8v8m6-8v8m4-8v8M3 21h18M12 3l9 5H3z",
        ),
        _ => (
            "bg-base-200 rounded-field",
            "M21 12a8 8 0 0 1-11.6 7.1L4 20l1-4.6A8 8 0 1 1 21 12z",
        ),
    };
    let width = if kind == "mp" { "2.6" } else { "2" };
    view! {
        <span class=format!("grid h-10 w-10 shrink-0 place-items-center {bg}") aria-hidden="true">
            <svg class="h-5 w-5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width=width stroke-linecap="round" stroke-linejoin="round"><path d=path/></svg>
        </span>
    }
}

/// Cómo se guardan las otras formas de cobro (se guardan solas al tocar).
#[derive(Clone, Copy, PartialEq)]
enum Saving {
    Idle,
    Busy,
    Done,
}

#[component]
fn Payments(slug: String, info: PaymentsInfo) -> impl IntoView {
    let query = use_query_map();
    let link = RwSignal::new(info.link);
    let mp_problem = RwSignal::new(query.with_untracked(|q| {
        q.get("error").map(|e| match e.as_str() {
            "cancelado" => "No se vinculó: cancelaste en Mercado Pago.".to_string(),
            "sesion" => "No se vinculó: el enlace no corresponde a esta sesión. Tocá \"Vincular mi cuenta\" de nuevo desde acá.".to_string(),
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
        Some(Err(e)) => mp_problem.set(Some(error_text(&e))),
        None => {}
    });
    let confirm = RwSignal::new(false);
    let unlink = Action::new(move |_: &()| mp_unlink(slug.get_value()));
    Effect::new(move |_| match unlink.value().get() {
        Some(Ok(())) => {
            link.set(None);
            confirm.set(false);
        }
        Some(Err(e)) => mp_problem.set(Some(error_text(&e))),
        None => {}
    });
    let busy =
        move || connect.pending().get() || connect.value().with(|v| matches!(v, Some(Ok(_))));

    // Transferencia y coordinar: se guardan solas al tocar el interruptor o
    // al salir de un campo.
    let o = info.options;
    let transfer = RwSignal::new(o.transfer);
    let alias = RwSignal::new(o.transfer_alias);
    let holder = RwSignal::new(o.transfer_holder);
    let coordinate = RwSignal::new(o.coordinate);
    let installments = RwSignal::new(o.installments);
    let discount = RwSignal::new(o.transfer_discount);
    let saving = RwSignal::new(Saving::Idle);
    let alias_problem = RwSignal::new(None::<String>);
    let discount_note = RwSignal::new(None::<String>);
    let save_problem = RwSignal::new(None::<String>);
    let save =
        Action::new(move |o: &PaymentOptions| save_payment_options(slug.get_value(), o.clone()));
    Effect::new(move |_| match save.value().get() {
        Some(Ok(None)) => saving.set(Saving::Done),
        Some(Ok(Some(p))) => {
            saving.set(Saving::Idle);
            alias_problem.set(Some(p));
        }
        Some(Err(e)) => {
            saving.set(Saving::Idle);
            save_problem.set(Some(error_text(&e)));
        }
        None => {}
    });
    // `from_transfer_toggle`: recién prendió Transferencia y todavía no pudo
    // escribir el alias, así que no se le marca el campo en rojo.
    let save_with = move |from_transfer_toggle: bool| {
        save_problem.set(None);
        let mut options = PaymentOptions {
            transfer: transfer.get_untracked(),
            transfer_alias: alias.get_untracked(),
            transfer_holder: holder.get_untracked(),
            coordinate: coordinate.get_untracked(),
            installments: installments.get_untracked(),
            transfer_discount: discount.get_untracked(),
        };
        // Transferencia sin alias (o con uno que no sirve): todavía no se
        // activa, pero lo demás se guarda igual. Antes no se guardaba nada y
        // un cambio en otra forma de cobro se perdía sin aviso.
        let mut complete = true;
        if options.transfer {
            if let Some(p) = domain::transfer_account_problem(&options.transfer_alias) {
                complete = false;
                options.transfer = false;
                let empty = options.transfer_alias.trim().is_empty();
                if !empty {
                    alias_problem.set(Some(p));
                } else if !from_transfer_toggle {
                    alias_problem.set(Some(
                        "Escribí tu alias, CBU o CVU para activar la transferencia.".into(),
                    ));
                }
            }
        }
        if complete {
            alias_problem.set(None);
        }
        saving.set(Saving::Busy);
        save.dispatch(options);
        complete
    };
    let save_now = move || save_with(false);
    // La transferencia cuenta recién con un alias que sirve.
    let transfer_ready =
        move || transfer.get() && alias.with(|a| domain::transfer_account_problem(a).is_none());
    let none_active = move || link.with(|l| l.is_none()) && !transfer_ready() && !coordinate.get();

    view! {
        <div class="flex flex-wrap items-end gap-3">
            <div class="grow">
                <h1 class="text-3xl font-bold max-sm:text-2xl">"Cobros"</h1>
                <p class="text-base-content/70">"Elegí cómo te pueden pagar. Podés tener más de una."</p>
            </div>
            <span role="status" class="text-sm font-semibold text-success">
                {move || match saving.get() {
                    Saving::Busy => view! { <span class="text-base-content/60">"Guardando…"</span> }.into_any(),
                    Saving::Done => view! { <span>"✓ Guardado"</span> }.into_any(),
                    Saving::Idle => ().into_any(),
                }}
            </span>
        </div>
        {move || none_active().then(|| view! {
            <p class="-mt-2 text-sm text-warning-content/90"><span class="font-semibold text-warning">"●"</span>" Activá al menos una forma de cobro para que tu tienda pueda vender."</p>
        })}
        {move || save_problem.get().map(|p| view! { <p class="-mt-2 text-sm text-error">{p}</p> })}
        <div class="card divide-y divide-base-200 bg-base-100 shadow-sm">
            // Mercado Pago
            <div class="flex items-start gap-4 px-6 py-5">
                {method_icon("mp")}
                <div class="min-w-0 flex-1">
                    <h2 class="font-bold" id="pay-mp">"Mercado Pago"</h2>
                    {move || match link.get() {
                        Some(l) => view! {
                            <p class="text-sm text-base-content/70">
                                {format!("Cuenta n.º {} · vinculada el {}", l.user_id, domain::order_time_ar(&l.linked_at).split(' ').next().unwrap_or(""))}
                                {(!l.live).then(|| view! { <span class="badge badge-warning badge-sm ml-2">"de prueba"</span> })}
                            </p>
                            {just_linked.then(|| view! { <p class="mt-1 text-sm text-success">"Tu tienda ya cobra con Mercado Pago."</p> })}
                        }.into_any(),
                        None => view! {
                            <p class="text-sm text-base-content/70">"Tarjeta, débito o dinero en cuenta. La plata va directo a tu cuenta."</p>
                        }.into_any(),
                    }}
                    {move || (link.with(|l| l.is_some()) && confirm.get()).then(|| view! {
                        <div class="mt-2 flex flex-wrap items-center gap-2">
                            <span class="text-sm">"¿Desvincular? Tu tienda deja de cobrar con Mercado Pago."</span>
                            <button class="btn btn-error btn-sm" disabled=move || unlink.pending().get() on:click=move |_| { unlink.dispatch(()); }>"Sí, desvincular"</button>
                            <button class="btn btn-ghost btn-sm" on:click=move |_| confirm.set(false)>"No"</button>
                        </div>
                    })}
                    {move || mp_problem.get().map(|e| view! { <p class="mt-1 text-sm text-error">{e}</p> })}
                    {move || link.with(|l| l.is_some()).then(|| view! {
                        <fieldset class="fieldset mt-2 max-w-xs py-0">
                            <label class="fieldset-legend" for="pay-installments">"Cuotas sin interés para mostrar"</label>
                            <select id="pay-installments" class="select w-full"
                                on:change=move |ev| {
                                    installments.set(event_target_value(&ev).parse().unwrap_or(0));
                                    save_now();
                                }>
                                {domain::INSTALLMENT_CHOICES.into_iter().map(|n| view! {
                                    <option value=n.to_string() selected=move || installments.get() == n>
                                        {if n == 0 { "No mostrar".to_string() } else { format!("{n} cuotas sin interés") }}
                                    </option>
                                }).collect_view()}
                            </select>
                            <p class="label whitespace-normal">"Se muestran en los precios. Activalas también en tu cuenta de Mercado Pago, que es la que las cobra."</p>
                        </fieldset>
                    })}
                </div>
                {move || match link.get() {
                    Some(_) => view! {
                        <div class="flex shrink-0 items-center gap-2">
                            <span class="badge badge-success badge-soft">"Vinculada"</span>
                            {move || (!confirm.get()).then(|| view! {
                                <button class="btn btn-ghost btn-sm" on:click=move |_| confirm.set(true)>"Desvincular"</button>
                            })}
                        </div>
                    }.into_any(),
                    None if !info.configured => view! {
                        <span class="badge badge-ghost shrink-0">"Próximamente"</span>
                    }.into_any(),
                    None => view! {
                        <button class="btn btn-sm shrink-0 border-0 bg-[#009ee3] text-white hover:bg-[#008ac6]" disabled=busy
                            on:click=move |_| { mp_problem.set(None); connect.dispatch(()); }>
                            {move || if busy() { "Abriendo…" } else { "Vincular cuenta" }}
                        </button>
                    }.into_any(),
                }}
            </div>
            // Transferencia
            <div class="flex items-start gap-4 px-6 py-5">
                {method_icon("transfer")}
                <div class="min-w-0 flex-1">
                    <h2 class="font-bold" id="pay-transfer">"Transferencia"</h2>
                    <p class="text-sm text-base-content/70" id="pay-transfer-desc">"Te transfieren a tu cuenta bancaria o virtual."</p>
                    {move || transfer.get().then(|| view! {
                        <div class="mt-3 grid max-w-xl gap-x-3 sm:grid-cols-2">
                            <fieldset class="fieldset py-0">
                                <label class="fieldset-legend" for="pay-alias">"Alias, CBU o CVU"</label>
                                <input id="pay-alias" class="input w-full" class:input-error=move || alias_problem.get().is_some()
                                    autocomplete="off" spellcheck="false" maxlength=domain::TRANSFER_ALIAS_MAX
                                    placeholder="mi.tienda.mp"
                                    aria-invalid=move || alias_problem.get().is_some().to_string()
                                    aria-describedby=move || alias_problem.get().is_some().then_some("pay-alias-error")
                                    prop:value=move || alias.get()
                                    on:input=move |ev| { alias.set(event_target_value(&ev)); alias_problem.set(None); }
                                    on:change=move |_| {
                                        if !save_now() && alias_problem.get_untracked().is_none() {
                                            alias_problem.set(Some("Escribí tu alias, CBU o CVU para activar la transferencia.".into()));
                                        }
                                    }/>
                                {move || alias_problem.get().map(|p| view! { <p id="pay-alias-error" class="label text-error">{p}</p> })}
                            </fieldset>
                            <fieldset class="fieldset py-0">
                                <label class="fieldset-legend" for="pay-holder">"Titular de la cuenta "<span class="font-normal text-base-content/60">"(opcional)"</span></label>
                                <input id="pay-holder" class="input w-full" autocomplete="off" maxlength=domain::TRANSFER_HOLDER_MAX
                                    placeholder="Nombre y apellido"
                                    prop:value=move || holder.get()
                                    on:input=move |ev| holder.set(event_target_value(&ev))
                                    on:change=move |_| { save_now(); }/>
                            </fieldset>
                        </div>
                        <fieldset class="fieldset mt-1 max-w-xs py-0">
                            <label class="fieldset-legend" for="pay-discount">"Descuento por transferencia "<span class="font-normal text-base-content/60">"(opcional)"</span></label>
                            <label class="input w-32">
                                <input id="pay-discount" type="number" inputmode="numeric" min="0" max=domain::TRANSFER_DISCOUNT_MAX
                                    prop:value=move || discount.get().to_string()
                                    on:change=move |ev| {
                                        let typed = event_target_value(&ev).trim().parse::<i64>().unwrap_or(0);
                                        let v = typed.clamp(0, domain::TRANSFER_DISCOUNT_MAX as i64) as u8;
                                        // Si se corrigió, se dice por qué (antes cambiaba sin aviso).
                                        discount_note.set((i64::from(v) != typed).then(|| if typed < 0 {
                                            "El descuento no puede ser negativo: quedó en 0 %.".to_string()
                                        } else {
                                            format!("El máximo es {} %: quedó en {} %.", domain::TRANSFER_DISCOUNT_MAX, domain::TRANSFER_DISCOUNT_MAX)
                                        }));
                                        discount.set(v);
                                        save_now();
                                    }/>
                                <span class="text-base-content/60">"%"</span>
                            </label>
                            {move || discount_note.get().map(|m| view! { <p class="label whitespace-normal text-error">{m}</p> })}
                            <p class="label whitespace-normal">"Se descuenta del total al pagar con transferencia y se muestra en los precios."</p>
                        </fieldset>
                        <p class="mt-1 text-xs text-base-content/60">
                            {move || if alias.with(|a| a.trim().is_empty()) && alias_problem.get().is_none() {
                                "Escribí tu alias, CBU o CVU para activar la transferencia."
                            } else {
                                "El comprador ve el alias y el titular después de confirmar la compra, en la página de su pedido y en el email."
                            }}
                        </p>
                    })}
                </div>
                <input type="checkbox" role="switch" class="toggle toggle-primary mt-1 shrink-0"
                    aria-labelledby="pay-transfer" aria-describedby="pay-transfer-desc"
                    prop:checked=move || transfer.get()
                    on:change=move |ev| {
                        let on = event_target_checked(&ev);
                        transfer.set(on);
                        if !save_with(true) && on {
                            // Falta el alias: se guarda cuando lo escriba.
                            request_animation_frame(|| {
                                use leptos::wasm_bindgen::JsCast;
                                if let Some(el) = document().get_element_by_id("pay-alias")
                                    .and_then(|e| e.dyn_into::<leptos::web_sys::HtmlElement>().ok()) {
                                    _ = el.focus();
                                }
                            });
                        }
                    }/>
            </div>
            // Coordinar
            <div class="flex items-start gap-4 px-6 py-5">
                {method_icon("coordinate")}
                <div class="min-w-0 flex-1">
                    <h2 class="font-bold" id="pay-coordinate">"Acordar con el comprador"</h2>
                    <p class="text-sm text-base-content/70" id="pay-coordinate-desc">"Le escribís por WhatsApp y arreglan cómo paga, por ejemplo efectivo al retirar."</p>
                </div>
                <input type="checkbox" role="switch" class="toggle toggle-primary mt-1 shrink-0"
                    aria-labelledby="pay-coordinate" aria-describedby="pay-coordinate-desc"
                    prop:checked=move || coordinate.get()
                    on:change=move |ev| { coordinate.set(event_target_checked(&ev)); save_now(); }/>
            </div>
        </div>
        <p class="flex gap-2 rounded-box border border-base-200 bg-base-100/60 px-4 py-3 text-sm text-base-content/70">
            <span aria-hidden="true">"ⓘ"</span>
            <span>"Con transferencia o acordado, el pedido espera en "<b>"Pedidos"</b>" hasta que toques "<b>"\"Ya me pagó\""</b>". Si pasan 48 horas sin pago, se cancela solo."</span>
        </p>
    }
}
