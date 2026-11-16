//! Comprar en la tienda: la página "Finalizar compra" (`/comprar`) y la del
//! pedido ya hecho (`/pedido/:id`).
//!
//! Por ahora el pago se coordina con la tienda; cuando esté Mercado Pago, el
//! botón va a llevar a pagar.

use domain::{
    CheckoutForm, CheckoutInfo, Delivery, DeliverySettings, FieldError, OrderReceipt, PageView,
    PlaceOrder,
};
use leptos::prelude::*;
use leptos::server_fn::codec::Json;
use leptos_router::hooks::use_params_map;
use ui::{format_price, Image};
use widgets::Cart;

/// Lo que necesita "Finalizar compra": nombre de la tienda y cómo entrega.
#[server]
pub async fn load_checkout() -> Result<CheckoutInfo, ServerFnError> {
    use crate::server::{internal, pool};
    let tenant = crate::auth::current_tenant().await?;
    let pool = pool()?;
    Ok(CheckoutInfo {
        delivery: db::orders::delivery(&pool, tenant.id)
            .await
            .map_err(internal)?,
        online_payments: online(&pool, tenant.id).await?,
        store_name: tenant.name,
    })
}

/// La tienda cobra con Mercado Pago (está configurado y la vinculó).
#[cfg(feature = "ssr")]
async fn online(pool: &db::Pool, tenant: uuid::Uuid) -> Result<bool, ServerFnError> {
    Ok(crate::mp::config().is_some()
        && db::mp::credentials(pool, tenant)
            .await
            .map_err(crate::server::internal)?
            .is_some())
}

/// Dirección de esta tienda, para volver de Mercado Pago.
#[cfg(feature = "ssr")]
fn store_base() -> String {
    format!(
        "{}://{}",
        crate::site::scheme(),
        crate::site::current_host()
    )
}

/// Guarda el pedido. Los precios salen de la base, no del navegador.
#[server(input = Json)]
pub async fn send_order(form: CheckoutForm) -> Result<PlaceOrder, ServerFnError> {
    use crate::server::{internal, pool};
    let tenant = crate::auth::current_tenant().await?;
    let pool = pool()?;
    let delivery = db::orders::delivery(&pool, tenant.id)
        .await
        .map_err(internal)?;
    let mut form = form;
    let errors = domain::tidy_checkout(&mut form, &delivery);
    if !errors.is_empty() {
        return Ok(PlaceOrder::Invalid { errors });
    }
    let changed = |message: String| Ok(PlaceOrder::CartChanged { message });
    let mut ids = Vec::new();
    for (id, _) in &form.lines {
        match uuid::Uuid::parse_str(id) {
            Ok(id) => ids.push(id),
            Err(_) => return changed("Un producto del carrito ya no está en la tienda.".into()),
        }
    }
    let products = db::products::by_ids(&pool, tenant.id, &ids)
        .await
        .map_err(internal)?;
    let mut items = Vec::new();
    for (id, qty) in &form.lines {
        let Some(p) = products.iter().find(|p| p.id == *id && p.visible) else {
            return changed(
                "Un producto del carrito ya no está en la tienda. Revisá el carrito.".into(),
            );
        };
        if !p.in_stock {
            return changed(format!(
                "\"{}\" se agotó. Sacalo del carrito para seguir.",
                p.name
            ));
        }
        items.push(domain::OrderItem {
            product_id: p.id.clone(),
            name: p.name.clone(),
            image_url: p.image_url.clone(),
            unit_cents: p.price_cents,
            qty: (*qty).min(widgets::cart::MAX_QTY),
        });
    }
    let pay_online = online(&pool, tenant.id).await?;
    let new = db::orders::NewOrder {
        status: if pay_online {
            domain::OrderStatus::PendingPayment
        } else {
            domain::OrderStatus::New
        },
        customer_name: &form.name,
        email: &form.email,
        phone: &form.phone,
        delivery: form.delivery,
        address: &form.address,
        postal_code: &form.postal_code,
        city: &form.city,
        notes: &form.notes,
        items: &items,
        shipping_cents: domain::shipping_for(form.delivery, &delivery),
    };
    let (id, number) = match db::orders::create(&pool, tenant.id, &new)
        .await
        .map_err(internal)?
    {
        db::orders::Created::Saved { id, number } => (id, number),
        db::orders::Created::Short { name, left: 0 } => {
            return changed(format!(
                "\"{name}\" se agotó. Sacalo del carrito para seguir."
            ))
        }
        db::orders::Created::Short { name, left } => {
            return changed(format!(
                "De \"{name}\" {} {left}. Bajá la cantidad en el carrito para seguir.",
                if left == 1 { "queda" } else { "quedan" }
            ))
        }
    };
    if pay_online {
        let order = db::orders::get(&pool, tenant.id, id)
            .await
            .map_err(internal)?
            .ok_or_else(|| ServerFnError::new("No encontramos el pedido."))?;
        match crate::mp::create_preference(&pool, tenant.id, &tenant.name, &store_base(), &order)
            .await
        {
            Ok(url) => {
                return Ok(PlaceOrder::Pay {
                    id: id.to_string(),
                    url,
                })
            }
            // El pedido ya está guardado: desde su página se puede reintentar.
            Err(e) => leptos::logging::log!(
                "Mercado Pago: no pudimos crear el pago del pedido #{number}: {e}"
            ),
        }
    }
    Ok(PlaceOrder::Placed {
        id: id.to_string(),
        number,
    })
}

/// Un pedido de esta tienda, para su página. El id largo hace de llave.
#[server]
pub async fn load_order(
    id: String,
    payment_id: Option<String>,
) -> Result<Option<OrderReceipt>, ServerFnError> {
    use crate::server::{internal, pool};
    let tenant = crate::auth::current_tenant().await?;
    let Ok(id) = uuid::Uuid::parse_str(&id) else {
        return Ok(None);
    };
    let pool = pool()?;
    // Volvió de Mercado Pago: se pregunta por el pago (no se cree lo que dice
    // la dirección). Así anda aunque Mercado Pago no pueda avisarnos.
    if let Some(p) = payment_id.filter(|p| !p.is_empty() && p != "null") {
        if let Err(e) = crate::mp::check_payment(&pool, tenant.id, &p).await {
            leptos::logging::log!("Mercado Pago: pago {p}: {e}");
        }
    }
    let Some(order) = db::orders::get(&pool, tenant.id, id)
        .await
        .map_err(internal)?
    else {
        return Ok(None);
    };
    let can_pay = matches!(
        order.status,
        domain::OrderStatus::New | domain::OrderStatus::PendingPayment
    ) && online(&pool, tenant.id).await?;
    Ok(Some(OrderReceipt {
        delivery: db::orders::delivery(&pool, tenant.id)
            .await
            .map_err(internal)?,
        store_name: tenant.name,
        order,
        can_pay,
    }))
}

/// "Pagar con Mercado Pago" desde la página del pedido: el mismo enlace de
/// antes, o uno nuevo si no había.
#[server]
pub async fn pay_order(id: String) -> Result<String, ServerFnError> {
    use crate::server::{internal, pool};
    let tenant = crate::auth::current_tenant().await?;
    let order_id =
        uuid::Uuid::parse_str(&id).map_err(|_| ServerFnError::new("No encontramos el pedido."))?;
    let pool = pool()?;
    if let Some(url) = db::mp::init_point(&pool, tenant.id, order_id)
        .await
        .map_err(internal)?
    {
        return Ok(url);
    }
    let order = db::orders::get(&pool, tenant.id, order_id)
        .await
        .map_err(internal)?
        .filter(|o| {
            matches!(
                o.status,
                domain::OrderStatus::New | domain::OrderStatus::PendingPayment
            )
        })
        .ok_or_else(|| ServerFnError::new("Este pedido ya no se puede pagar."))?;
    crate::mp::create_preference(&pool, tenant.id, &tenant.name, &store_base(), &order)
        .await
        .map_err(|e| {
            leptos::logging::log!("Mercado Pago: pedido #{}: {e}", order.number);
            ServerFnError::new("No pudimos conectar con Mercado Pago. Probá de nuevo en un rato.")
        })
}

fn error_text(e: &ServerFnError) -> String {
    match e {
        ServerFnError::ServerError(m) => m.clone(),
        _ => "No pudimos conectarnos. Revisá tu conexión y probá de nuevo.".into(),
    }
}

/// Fondo y letras de la tienda, para que estas páginas se vean como ella.
#[component]
fn StoreFrame(view: PageView, children: Children) -> impl IntoView {
    let doc = view.doc;
    view! {
        <div data-theme=doc.theme.clone() class="mk-store min-h-screen bg-base-200 text-base-content"
            style=format!("{}; {}", doc.typography.css_vars(), doc.style.css_vars())>
            <div class="mx-auto flex max-w-[1120px] flex-col gap-5 px-4 py-4">
                {children()}
            </div>
        </div>
    }
}

#[component]
fn StoreHeader(name: String) -> impl IntoView {
    view! {
        <header class="flex h-18 items-center rounded-box bg-base-100 px-6 shadow-sm">
            <a href="/" class="mk-h text-lg font-bold">{name}</a>
            <a href="/" class="link link-hover ml-auto text-sm text-base-content/70">"← Volver a la tienda"</a>
        </header>
    }
}

/// `/comprar` en una tienda.
#[component]
pub fn CheckoutPage() -> impl IntoView {
    if crate::site::current_site() == domain::Site::Platform {
        return view! { <crate::platform::NotAStore/> }.into_any();
    }
    let page = Resource::new(|| (), |_| crate::load_page("home".to_string()));
    let info = Resource::new(|| (), |_| load_checkout());
    view! {
        <leptos_meta::Title text="Finalizar compra"/>
        <Suspense fallback=|| view! { <ui::PageSkeleton/> }>
            {move || Suspend::new(async move {
                match (page.await, info.await) {
                    (Ok(view), Ok(info)) => view! { <Checkout view info/> }.into_any(),
                    _ => view! { <crate::platform::StoreNotFound/> }.into_any(),
                }
            })}
        </Suspense>
    }
    .into_any()
}

/// Lo que se está escribiendo en el formulario.
#[derive(Clone, Copy)]
struct Fields {
    name: RwSignal<String>,
    email: RwSignal<String>,
    phone: RwSignal<String>,
    delivery: RwSignal<Delivery>,
    address: RwSignal<String>,
    postal_code: RwSignal<String>,
    city: RwSignal<String>,
    notes: RwSignal<String>,
}

#[component]
fn Checkout(view: PageView, info: CheckoutInfo) -> impl IntoView {
    let cart = Cart::provide(view.products.clone());
    let d = StoredValue::new(info.delivery);
    let first = if d.with_value(|d| d.shipping_enabled) {
        Delivery::Shipping
    } else {
        Delivery::Pickup
    };
    let f = Fields {
        name: RwSignal::new(String::new()),
        email: RwSignal::new(String::new()),
        phone: RwSignal::new(String::new()),
        delivery: RwSignal::new(first),
        address: RwSignal::new(String::new()),
        postal_code: RwSignal::new(String::new()),
        city: RwSignal::new(String::new()),
        notes: RwSignal::new(String::new()),
    };
    let errors = RwSignal::new(Vec::<FieldError>::new());
    let problem = RwSignal::new(None::<String>);
    let place = Action::new(|form: &CheckoutForm| send_order(form.clone()));
    Effect::new(move |_| match place.value().get() {
        Some(Ok(PlaceOrder::Placed { id, .. })) => {
            cart.clear();
            _ = window().location().set_href(&format!("/pedido/{id}"));
        }
        Some(Ok(PlaceOrder::Pay { url, .. })) => {
            cart.clear();
            _ = window().location().set_href(&url);
        }
        Some(Ok(PlaceOrder::Invalid { errors: e })) => {
            focus_first(&e);
            errors.set(e);
        }
        Some(Ok(PlaceOrder::CartChanged { message })) => problem.set(Some(message)),
        Some(Err(e)) => problem.set(Some(error_text(&e))),
        None => {}
    });
    let submit = move || {
        problem.set(None);
        let mut form = CheckoutForm {
            name: f.name.get_untracked(),
            email: f.email.get_untracked(),
            phone: f.phone.get_untracked(),
            delivery: f.delivery.get_untracked(),
            address: f.address.get_untracked(),
            postal_code: f.postal_code.get_untracked(),
            city: f.city.get_untracked(),
            notes: f.notes.get_untracked(),
            lines: cart
                .items()
                .into_iter()
                .filter(|i| i.product.in_stock)
                .map(|i| (i.product.id, i.qty))
                .collect(),
        };
        let e = d.with_value(|d| domain::tidy_checkout(&mut form, d));
        if e.is_empty() {
            errors.set(Vec::new());
            place.dispatch(form);
        } else {
            focus_first(&e);
            errors.set(e);
        }
    };
    let busy = move || {
        place.pending().get()
            || place.value().with(|v| {
                matches!(
                    v,
                    Some(Ok(PlaceOrder::Placed { .. } | PlaceOrder::Pay { .. }))
                )
            })
    };
    let empty = Memo::new(move |_| cart.count() == 0);

    view! {
        <StoreFrame view>
            <StoreHeader name=info.store_name/>
            {move || if !cart.ready.get() {
                view! { <ui::Skeleton class="h-96 w-full"/> }.into_any()
            } else if empty.get() {
                view! {
                    <div class="card bg-base-100 shadow-sm"><div class="card-body items-center gap-3 py-16 text-center">
                        <widgets::cart::BagIcon class="h-12 w-12 text-base-content/40"/>
                        <h1 class="mk-h text-2xl font-bold">"Tu carrito está vacío"</h1>
                        <p class="text-base-content/70">"Sumá productos desde la tienda y volvé acá para terminar la compra."</p>
                        <a class="btn btn-primary" href="/">"Ir a la tienda"</a>
                    </div></div>
                }.into_any()
            } else {
                view! {
                    <form class="grid items-start gap-6 lg:grid-cols-[1fr_380px]" novalidate
                        on:submit=move |ev| { ev.prevent_default(); submit(); }>
                        <div class="card bg-base-100 shadow-sm"><div class="card-body gap-1 p-7">
                            <h1 class="mk-h text-2xl font-bold">"Finalizar compra"</h1>
                            <p class="text-base-content/70">"Completá tus datos y cómo querés recibir tu pedido."</p>
                            {move || {
                                let n = errors.with(|e| e.len());
                                (n > 0).then(|| view! {
                                    <div role="alert" class="alert alert-error alert-soft mt-3 text-sm">
                                        {if n == 1 { "Revisá el campo marcado.".to_string() } else { format!("Revisá los {n} campos marcados.") }}
                                    </div>
                                })
                            }}
                            <h2 class="mk-h mt-5 text-lg font-bold">"Tus datos"</h2>
                            <div class="grid gap-x-4 sm:grid-cols-2">
                                <TextField id="name" label="Nombre y apellido" value=f.name errors autocomplete="name" max=domain::NAME_MAX/>
                                <TextField id="phone" label="Teléfono (WhatsApp)" value=f.phone errors autocomplete="tel" kind="tel" placeholder="11 5555-1234" max=30/>
                            </div>
                            <TextField id="email" label="Email" hint="te mandamos el comprobante" value=f.email errors autocomplete="email" kind="email" max=domain::EMAIL_MAX/>
                            <h2 class="mk-h mt-5 text-lg font-bold" id="f-delivery-label">"¿Cómo lo recibís?"</h2>
                            <DeliveryChoice f d=d.get_value() errors/>
                            {move || (f.delivery.get() == Delivery::Shipping).then(|| view! {
                                <div class="grid gap-x-4 sm:grid-cols-[2fr_1fr_1fr]">
                                    <TextField id="address" label="Calle y número" value=f.address errors autocomplete="street-address" placeholder="Av. Santa Fe 2050, 3° B" max=domain::ADDRESS_MAX/>
                                    <TextField id="postal_code" label="Código postal" value=f.postal_code errors autocomplete="postal-code" placeholder="1123" max=10/>
                                    <TextField id="city" label="Ciudad" value=f.city errors autocomplete="address-level2" max=domain::CITY_MAX/>
                                </div>
                            })}
                            <fieldset class="fieldset mt-1">
                                <legend class="fieldset-legend">"Algo más para la tienda " <span class="font-normal text-base-content/60">"(opcional)"</span></legend>
                                <textarea id="f-notes" class="textarea w-full" rows="2" maxlength=domain::NOTES_MAX
                                    placeholder="Por ejemplo: timbre roto, llamar al llegar"
                                    prop:value=move || f.notes.get()
                                    on:input=move |ev| f.notes.set(event_target_value(&ev))></textarea>
                            </fieldset>
                        </div></div>
                        <Summary cart f d=d.get_value() problem busy=Signal::derive(busy) online=info.online_payments/>
                    </form>
                }.into_any()
            }}
        </StoreFrame>
    }
}

/// Lleva el foco al primer campo con problema.
fn focus_first(errors: &[FieldError]) {
    use leptos::wasm_bindgen::JsCast;
    let Some(first) = errors.first() else {
        return;
    };
    let id = match first.field.as_str() {
        "delivery" => "f-delivery-0".to_string(),
        "cart" => return,
        f => format!("f-{f}"),
    };
    request_animation_frame(move || {
        if let Some(el) = document()
            .get_element_by_id(&id)
            .and_then(|e| e.dyn_into::<leptos::web_sys::HtmlElement>().ok())
        {
            _ = el.focus();
        }
    });
}

fn error_for(errors: RwSignal<Vec<FieldError>>, field: &'static str) -> Option<String> {
    errors.with(|e| {
        e.iter()
            .find(|e| e.field == field)
            .map(|e| e.message.clone())
    })
}

/// Un campo de texto con su etiqueta y su error debajo.
#[component]
fn TextField(
    id: &'static str,
    label: &'static str,
    value: RwSignal<String>,
    errors: RwSignal<Vec<FieldError>>,
    #[prop(optional)] hint: &'static str,
    #[prop(optional)] autocomplete: &'static str,
    #[prop(optional)] kind: &'static str,
    #[prop(optional)] placeholder: &'static str,
    max: usize,
) -> impl IntoView {
    let error = move || error_for(errors, id);
    let input_id = format!("f-{id}");
    let error_id = format!("f-{id}-error");
    let kind = if kind.is_empty() { "text" } else { kind };
    view! {
        <fieldset class="fieldset">
            <label class="fieldset-legend" for=input_id.clone()>
                {label}
                {(!hint.is_empty()).then(|| view! { <span class="font-normal text-base-content/60">{format!(" · {hint}")}</span> })}
            </label>
            <input id=input_id class="input w-full" class:input-error=move || error().is_some()
                type=kind autocomplete=autocomplete placeholder=placeholder maxlength=max
                aria-invalid=move || error().is_some().to_string()
                aria-describedby=move || error().is_some().then(|| error_id.clone())
                prop:value=move || value.get()
                on:input=move |ev| {
                    value.set(event_target_value(&ev));
                    // Al corregir, el aviso de ese campo se va.
                    if error().is_some() {
                        errors.update(|e| e.retain(|e| e.field != id));
                    }
                }/>
            {move || error().map(|m| view! { <p id=format!("f-{id}-error") class="label text-error">{m}</p> })}
        </fieldset>
    }
}

/// Retiro o envío, como tarjetas para elegir (solo las que ofrece la tienda).
#[component]
fn DeliveryChoice(
    f: Fields,
    d: DeliverySettings,
    errors: RwSignal<Vec<FieldError>>,
) -> impl IntoView {
    let mut options = Vec::new();
    if d.pickup_enabled {
        let detail = [d.pickup_address.clone(), d.pickup_hours.clone()]
            .into_iter()
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join(" · ");
        options.push((Delivery::Pickup, detail, "Gratis".to_string()));
    }
    if d.shipping_enabled {
        options.push((
            Delivery::Shipping,
            d.shipping_note.clone(),
            domain::shipping_label(&d, format_price),
        ));
    }
    let error = move || error_for(errors, "delivery");
    view! {
        <div role="radiogroup" aria-labelledby="f-delivery-label" class="flex flex-col gap-2">
            {options.into_iter().enumerate().map(|(i, (kind, detail, price))| {
                let free = price == "Gratis";
                view! {
                    <label class="flex cursor-pointer items-start gap-3 rounded-box border border-base-300 px-4 py-3 has-checked:border-primary has-checked:bg-primary/8 has-checked:outline has-checked:outline-primary">
                        <input type="radio" name="delivery" class="radio radio-primary radio-sm mt-0.5" id=format!("f-delivery-{i}")
                            prop:checked=move || f.delivery.get() == kind
                            on:change=move |_| {
                                f.delivery.set(kind);
                                errors.update(|e| e.retain(|e| e.field != "delivery"));
                            }/>
                        <span class="min-w-0 flex-1">
                            <span class="block font-semibold">{kind.label()}</span>
                            {(!detail.is_empty()).then(|| view! { <span class="block text-sm text-base-content/70">{detail}</span> })}
                        </span>
                        <span class="font-semibold" class:text-success=free>{price}</span>
                    </label>
                }
            }).collect_view()}
            {move || error().map(|m| view! { <p class="text-sm text-error">{m}</p> })}
        </div>
    }
}

/// "Tu pedido" a la derecha: productos, envío, total y el botón.
#[component]
fn Summary(
    cart: Cart,
    f: Fields,
    d: DeliverySettings,
    problem: RwSignal<Option<String>>,
    busy: Signal<bool>,
    online: bool,
) -> impl IntoView {
    let d = StoredValue::new(d);
    let items = move || {
        cart.items()
            .into_iter()
            .filter(|i| i.product.in_stock)
            .collect::<Vec<_>>()
    };
    let subtotal = move || cart.total_cents();
    let shipping = move || d.with_value(|d| domain::shipping_for(f.delivery.get(), d));
    let shipping_text = move || match f.delivery.get() {
        Delivery::Pickup => "Gratis".to_string(),
        Delivery::Shipping => d.with_value(|d| domain::shipping_label(d, format_price)),
    };
    view! {
        <aside class="card bg-base-100 shadow-sm lg:sticky lg:top-4"><div class="card-body gap-0 p-6">
            <h2 class="mk-h mb-2 text-lg font-bold">"Tu pedido"</h2>
            <ul>
                {move || items().into_iter().map(|i| {
                    let line = i.product.price_cents * i64::from(i.qty);
                    view! {
                        <li class="flex items-center gap-3 border-b border-base-200 py-3">
                            <div class="h-12 w-12 shrink-0 overflow-hidden rounded-field"><Image src=i.product.image_url.clone() alt=""/></div>
                            <div class="min-w-0 flex-1">
                                <p class="truncate text-sm font-semibold">{i.product.name.clone()}</p>
                                <p class="text-xs text-base-content/70">{format!("{} × {}", i.qty, format_price(i.product.price_cents))}</p>
                            </div>
                            <span class="text-sm font-semibold tabular-nums">{format_price(line)}</span>
                        </li>
                    }
                }).collect_view()}
            </ul>
            <a href="/" class="link link-hover mt-2 self-start text-sm text-base-content/70">"Cambiar productos"</a>
            <div class="mt-3 flex justify-between text-sm text-base-content/75"><span>"Subtotal"</span><span>{move || format_price(subtotal())}</span></div>
            <div class="mt-1 flex justify-between text-sm text-base-content/75"><span>"Envío"</span><span>{shipping_text}</span></div>
            <div class="mt-3 mb-4 flex justify-between text-lg font-bold"><span>"Total"</span><span>{move || format_price(subtotal() + shipping())}</span></div>
            {move || problem.get().map(|m| view! { <div role="alert" class="alert alert-warning mb-3 text-sm">{m}</div> })}
            <button type="submit" class="btn btn-primary btn-lg btn-block" disabled=busy>
                {move || match (busy.get(), online) {
                    (true, true) => "Abriendo Mercado Pago…",
                    (true, false) => "Enviando…",
                    (false, true) => "Ir a pagar con Mercado Pago",
                    (false, false) => "Confirmar pedido",
                }}
            </button>
            <p class="mt-2 text-center text-xs text-base-content/70">
                {if online {
                    "Pagás en Mercado Pago: tarjeta de crédito o débito, dinero en cuenta o efectivo."
                } else {
                    "Esta tienda todavía no recibe pagos online: te escribe para coordinar el pago."
                }}
            </p>
        </div></aside>
    }
}

/// `/pedido/:id` en una tienda: el pedido recién hecho.
#[component]
pub fn OrderPage() -> impl IntoView {
    if crate::site::current_site() == domain::Site::Platform {
        return view! { <crate::platform::NotAStore/> }.into_any();
    }
    let params = use_params_map();
    let id = params.read_untracked().get("id").unwrap_or_default();
    let page = Resource::new(|| (), |_| crate::load_page("home".to_string()));
    // Al volver de Mercado Pago, la dirección trae `payment_id` y `status`.
    let query = leptos_router::hooks::use_query_map();
    let payment_id = query.read_untracked().get("payment_id");
    let receipt = Resource::new(
        move || (id.clone(), payment_id.clone()),
        |(id, p)| load_order(id, p),
    );
    view! {
        <leptos_meta::Title text="Tu pedido"/>
        <Suspense fallback=|| view! { <ui::PageSkeleton/> }>
            {move || Suspend::new(async move {
                match (page.await, receipt.await) {
                    (Ok(view), Ok(Some(r))) => view! { <Receipt view r/> }.into_any(),
                    (Ok(view), _) => view! {
                        <StoreFrame view>
                            <div class="card bg-base-100 shadow-sm"><div class="card-body items-center gap-3 py-16 text-center">
                                <h1 class="mk-h text-2xl font-bold">"No encontramos este pedido"</h1>
                                <p class="text-base-content/70">"Revisá el enlace del email que te mandamos."</p>
                                <a class="btn btn-primary" href="/">"Ir a la tienda"</a>
                            </div></div>
                        </StoreFrame>
                    }.into_any(),
                    _ => view! { <crate::platform::StoreNotFound/> }.into_any(),
                }
            })}
        </Suspense>
    }
    .into_any()
}

#[component]
fn Receipt(view: PageView, r: OrderReceipt) -> impl IntoView {
    // Para que el contador del menú quede en 0 si se vuelve a la tienda.
    Cart::provide(view.products.clone());
    let OrderReceipt {
        store_name,
        order,
        delivery,
        can_pay,
    } = r;
    let first_name = order
        .customer_name
        .split_whitespace()
        .next()
        .unwrap_or("")
        .to_string();
    let where_ = match order.delivery {
        Delivery::Pickup => [
            delivery.pickup_address.clone(),
            delivery.pickup_hours.clone(),
        ]
        .into_iter()
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join(" · "),
        Delivery::Shipping => format!("{}, {} ({})", order.address, order.city, order.postal_code),
    };
    let shipping_text = match order.delivery {
        Delivery::Pickup => "Gratis".to_string(),
        Delivery::Shipping if delivery.shipping_cents.is_none() && order.shipping_cents == 0 => {
            "A coordinar".to_string()
        }
        Delivery::Shipping if order.shipping_cents == 0 => "Gratis".to_string(),
        Delivery::Shipping => format_price(order.shipping_cents),
    };
    view! {
        <StoreFrame view>
            <StoreHeader name=store_name/>
            <div class="mx-auto flex w-full max-w-2xl flex-col gap-5">
                <ReceiptHead first_name number=order.number phone=order.phone.clone() status=order.status can_pay id=order.id.clone()/>
                <div class="card bg-base-100 shadow-sm"><div class="card-body gap-0 p-6">
                    <h2 class="mk-h mb-1 text-lg font-bold">{format!("Pedido #{}", order.number)}</h2>
                    <ul>
                        {order.items.iter().map(|i| view! {
                            <li class="flex items-center gap-3 border-b border-base-200 py-3">
                                <div class="h-12 w-12 shrink-0 overflow-hidden rounded-field"><Image src=i.image_url.clone() alt=""/></div>
                                <div class="min-w-0 flex-1">
                                    <p class="truncate text-sm font-semibold">{i.name.clone()}</p>
                                    <p class="text-xs text-base-content/70">{format!("{} × {}", i.qty, format_price(i.unit_cents))}</p>
                                </div>
                                <span class="text-sm font-semibold tabular-nums">{format_price(i.total_cents())}</span>
                            </li>
                        }).collect_view()}
                    </ul>
                    <div class="mt-3 flex justify-between text-sm text-base-content/75"><span>"Subtotal"</span><span>{format_price(order.subtotal_cents)}</span></div>
                    <div class="mt-1 flex justify-between text-sm text-base-content/75"><span>"Envío"</span><span>{shipping_text}</span></div>
                    <div class="mt-3 flex justify-between text-lg font-bold"><span>"Total"</span><span>{format_price(order.total_cents)}</span></div>
                    <h3 class="mt-5 font-semibold">{order.delivery.label()}</h3>
                    <p class="text-sm text-base-content/75">{where_}</p>
                    <h3 class="mt-4 font-semibold">"Tus datos"</h3>
                    <p class="text-sm text-base-content/75">{format!("{} · {} · {}", order.customer_name, order.email, order.phone)}</p>
                    <p class="mt-4 text-xs text-base-content/60">"Guardá esta página: es la de tu pedido."</p>
                </div></div>
            </div>
        </StoreFrame>
    }
}

/// Arriba de la página del pedido: gracias, o lo que falta (pagar).
#[component]
fn ReceiptHead(
    first_name: String,
    number: i32,
    phone: String,
    status: domain::OrderStatus,
    can_pay: bool,
    id: String,
) -> impl IntoView {
    use domain::OrderStatus as S;
    let query = leptos_router::hooks::use_query_map();
    // Lo que dice Mercado Pago al volver; solo para el texto, el estado real
    // es el del pedido.
    let back = query.with_untracked(|q| q.get("status").or_else(|| q.get("collection_status")));
    let pay = Action::new(move |id: &String| pay_order(id.clone()));
    let problem = RwSignal::new(None::<String>);
    Effect::new(move |_| match pay.value().get() {
        Some(Ok(url)) => _ = window().location().set_href(&url),
        Some(Err(e)) => problem.set(Some(error_text(&e))),
        None => {}
    });
    let busy = move || pay.pending().get() || pay.value().with(|v| matches!(v, Some(Ok(_))));
    let check = view! {
        <div class="grid h-14 w-14 place-items-center rounded-full bg-success/15 text-success" aria-hidden="true">
            <svg class="h-7 w-7" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.4" stroke-linecap="round" stroke-linejoin="round"><path d="M5 12l5 5L20 7"/></svg>
        </div>
    };
    let body = match status {
        S::Paid | S::Shipped | S::Delivered => view! {
            {check}
            <h1 class="mk-h text-2xl font-bold">{format!("¡Gracias, {first_name}! Tu pago está aprobado")}</h1>
            <p class="text-base-content/75">{format!("Pedido #{number} · la tienda ya lo está preparando. Si hace falta algo, te escribe al {phone}.")}</p>
            <a class="btn btn-primary mt-3" href="/">"Seguir comprando"</a>
        }.into_any(),
        S::Cancelled => view! {
            <h1 class="mk-h text-2xl font-bold">{format!("Pedido #{number} cancelado")}</h1>
            <p class="text-base-content/75">"Si tenés dudas, escribile a la tienda."</p>
            <a class="btn btn-primary mt-3" href="/">"Ir a la tienda"</a>
        }.into_any(),
        _ if can_pay => {
            let (title, text) = match back.as_deref() {
                Some("pending" | "in_process") => (
                    "Tu pago está pendiente".to_string(),
                    "Mercado Pago lo está procesando (por ejemplo, si pagás en efectivo). Cuando se apruebe, la tienda empieza a preparar tu pedido.".to_string(),
                ),
                Some("rejected" | "failure") => (
                    "El pago no se completó".to_string(),
                    "No se cobró nada. Podés intentar de nuevo, con otra tarjeta o medio de pago.".to_string(),
                ),
                _ => (
                    format!("{first_name}, falta pagar tu pedido"),
                    "Tu pedido está guardado. Pagalo en Mercado Pago para que la tienda lo prepare.".to_string(),
                ),
            };
            let id = id.clone();
            view! {
                <h1 class="mk-h text-2xl font-bold">{title}</h1>
                <p class="text-base-content/75">{format!("Pedido #{number} · {text}")}</p>
                {move || problem.get().map(|m| view! { <div role="alert" class="alert alert-warning text-sm">{m}</div> })}
                <button type="button" class="btn btn-primary mt-3" disabled=busy on:click=move |_| { problem.set(None); pay.dispatch(id.clone()); }>
                    {move || if busy() { "Abriendo Mercado Pago…" } else { "Pagar con Mercado Pago" }}
                </button>
            }.into_any()
        }
        _ => view! {
            {check}
            <h1 class="mk-h text-2xl font-bold">{format!("¡Gracias, {first_name}! Recibimos tu pedido")}</h1>
            <p class="text-base-content/75">{format!("Pedido #{number} · te vamos a escribir al {phone} para coordinar el pago y la entrega.")}</p>
            <a class="btn btn-primary mt-3" href="/">"Seguir comprando"</a>
        }.into_any(),
    };
    view! {
        <div class="card bg-base-100 shadow-sm"><div class="card-body items-center gap-2 py-10 text-center">
            {body}
        </div></div>
    }
}
