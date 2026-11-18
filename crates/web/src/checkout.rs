//! Comprar en la tienda: la página "Finalizar compra" (`/comprar`) y la del
//! pedido ya hecho (`/pedido/:id`).
//!
//! El comprador paga como lo ofrezca la tienda: Mercado Pago (el pedido entra
//! recién con el pago aprobado), transferencia o coordinando con la tienda
//! (el dueño marca cuando cobró).

use domain::{
    AndreaniBranch, AndreaniQuote, AndreaniService, CheckoutForm, CheckoutInfo, Delivery,
    DeliverySettings, FieldError, OrderReceipt, PageView, PlaceOrder,
};
use leptos::prelude::*;
use leptos::server_fn::codec::Json;
use leptos_router::hooks::use_params_map;
use ui::{format_price, Image};
use widgets::Cart;

/// Lo que necesita "Finalizar compra": nombre de la tienda y cómo entrega.
#[server]
pub async fn load_checkout() -> Result<CheckoutInfo, ServerFnError> {
    use crate::server::pool;
    let tenant = crate::auth::current_tenant().await?;
    let pool = pool()?;
    Ok(CheckoutInfo {
        delivery: delivery_with_andreani(&pool, tenant.id).await?,
        pay_methods: methods(&pool, tenant.id).await?,
        store_name: tenant.name,
    })
}

/// Cómo entrega la tienda, con los servicios de Andreani que ofrece.
#[cfg(feature = "ssr")]
async fn delivery_with_andreani(
    pool: &db::Pool,
    tenant: uuid::Uuid,
) -> Result<DeliverySettings, ServerFnError> {
    let mut d = db::orders::delivery(pool, tenant)
        .await
        .map_err(crate::server::internal)?;
    if let Some((acc, s)) = andreani_ready(pool, tenant).await? {
        d.andreani = s.services.clone();
        // Los paquetes grandes van como Bigger, si la cuenta lo tiene.
        if domain::contract_services(&acc.contracts.0).contains(&AndreaniService::Bigger) {
            d.andreani.push(AndreaniService::Bigger);
        }
    }
    Ok(d)
}

/// La cuenta de Andreani de la tienda, si está lista para ofrecerse.
#[cfg(feature = "ssr")]
async fn andreani_ready(
    pool: &db::Pool,
    tenant: uuid::Uuid,
) -> Result<Option<(db::andreani::Account, domain::AndreaniSettings)>, ServerFnError> {
    let Some(acc) = db::andreani::account(pool, tenant)
        .await
        .map_err(crate::server::internal)?
    else {
        return Ok(None);
    };
    let Some(s) = acc.settings.clone().map(|j| j.0) else {
        return Ok(None);
    };
    Ok((s.enabled && !s.services.is_empty()).then_some((acc, s)))
}

/// Cotizaciones recientes: el mismo carrito al mismo código postal no se
/// vuelve a preguntar por 15 minutos (y el pedido se cobra con lo que vio
/// el comprador).
#[cfg(feature = "ssr")]
fn quote_cache(
) -> &'static std::sync::Mutex<std::collections::HashMap<String, (std::time::Instant, AndreaniQuote)>>
{
    static CACHE: std::sync::OnceLock<
        std::sync::Mutex<std::collections::HashMap<String, (std::time::Instant, AndreaniQuote)>>,
    > = std::sync::OnceLock::new();
    CACHE.get_or_init(Default::default)
}

/// Lo que cuesta mandar estos productos a `postal_code` con Andreani.
#[cfg(feature = "ssr")]
async fn andreani_quote(
    pool: &db::Pool,
    tenant: uuid::Uuid,
    postal_code: &str,
    lines: &[(String, u32)],
) -> Result<AndreaniQuote, ServerFnError> {
    use crate::server::internal;
    const TTL: std::time::Duration = std::time::Duration::from_secs(15 * 60);
    let problem = |postal_code: String, m: &str| AndreaniQuote {
        postal_code,
        rates: Vec::new(),
        problem: Some(m.to_string()),
    };
    let Some(cp) = domain::normalize_postal_code(postal_code) else {
        return Ok(problem(
            String::new(),
            "Revisá el código postal: son 4 números, por ejemplo 1425.",
        ));
    };
    let Some((acc, settings)) = andreani_ready(pool, tenant).await? else {
        return Ok(problem(cp, "Esta tienda no envía con Andreani."));
    };
    let mut sorted: Vec<_> = lines
        .iter()
        .map(|(id, q)| (id.clone(), (*q).min(widgets::cart::MAX_QTY)))
        .collect();
    sorted.sort();
    let ids: Vec<uuid::Uuid> = sorted
        .iter()
        .filter_map(|(id, _)| uuid::Uuid::parse_str(id).ok())
        .collect();
    let products = db::products::by_ids(pool, tenant, &ids)
        .await
        .map_err(internal)?;
    let mut items = Vec::new();
    for (id, qty) in &sorted {
        if let Some(p) = products
            .iter()
            .find(|p| p.id == *id && p.visible && !p.example)
        {
            items.push(crate::andreani::Line {
                parcel: p.parcel.unwrap_or(settings.parcel),
                unit_cents: p.price_cents,
                qty: *qty,
            });
        }
    }
    if items.is_empty() {
        return Ok(problem(cp, "Tu carrito está vacío."));
    }
    // La clave incluye lo que cambia el precio del envío: productos (precio y
    // paquete), origen y servicios. Si el dueño cambia algo, se cotiza de nuevo.
    let key = format!(
        "{tenant}|{cp}|{:?}|{}|{:?}|{:?}",
        items
            .iter()
            .map(|l| (l.parcel, l.unit_cents, l.qty))
            .collect::<Vec<_>>(),
        settings.origin.postal_code,
        settings.services,
        acc.contracts
            .0
            .iter()
            .map(|c| c.service)
            .collect::<Vec<_>>(),
    );
    if let Some((at, q)) = quote_cache().lock().unwrap().get(&key) {
        if at.elapsed() < TTL {
            return Ok(q.clone());
        }
    }
    let available = domain::contract_services(&acc.contracts.0);
    let bigger = items.iter().any(|l| l.parcel.is_bigger());
    if bigger && !available.contains(&AndreaniService::Bigger) {
        return Ok(problem(
            cp,
            "Este pedido es muy grande para enviarlo con Andreani. Elegí otra forma de entrega.",
        ));
    }
    let origin = settings.origin.postal_code.clone();
    let rates =
        match crate::andreani::rates(pool, tenant, &acc.client_type, &origin, &cp, &items, bigger)
            .await
        {
            Ok(r) => r,
            Err(e) => {
                leptos::logging::warn!("Andreani: no se pudo cotizar a {cp}: {e}");
                // No se guarda: el próximo intento vuelve a preguntar.
                return Ok(problem(
                    cp,
                    "No pudimos calcular el envío con Andreani. Probá de nuevo en un momento.",
                ));
            }
        };
    let rates: Vec<_> = rates
        .into_iter()
        .filter(|r| {
            if bigger {
                r.service == AndreaniService::Bigger
            } else {
                settings.services.contains(&r.service)
            }
        })
        .collect();
    let q = AndreaniQuote {
        problem: rates
            .is_empty()
            .then(|| "Andreani no llega a ese código postal.".to_string()),
        postal_code: cp,
        rates,
    };
    let mut cache = quote_cache().lock().unwrap();
    cache.retain(|_, (at, _)| at.elapsed() < TTL);
    cache.insert(key, (std::time::Instant::now(), q.clone()));
    Ok(q)
}

#[cfg(feature = "ssr")]
const TOO_MANY_LOOKUPS: &str =
    "Recibimos muchas consultas seguidas desde tu conexión. Esperá unos minutos y probá de nuevo.";

/// Cotizar y buscar sucursales gastan el cupo de Andreani de la tienda: 40
/// consultas de cada una cada 10 minutos por conexión y tienda alcanzan de
/// sobra para comprar.
#[cfg(feature = "ssr")]
fn may_ask_andreani(what: &str, tenant: uuid::Uuid) -> bool {
    crate::limits::allow(
        &format!("{what}|{tenant}|{}", crate::limits::client_ip()),
        40,
        std::time::Duration::from_secs(10 * 60),
    )
}

/// "Finalizar compra": lo que cuesta el envío con Andreani a `postal_code`.
#[server(input = Json)]
pub async fn quote_shipping(
    postal_code: String,
    lines: Vec<(String, u32)>,
) -> Result<AndreaniQuote, ServerFnError> {
    let tenant = crate::auth::current_tenant().await?;
    if !may_ask_andreani("quote", tenant.id) {
        return Err(ServerFnError::new(TOO_MANY_LOOKUPS));
    }
    let lines: Vec<_> = lines.into_iter().take(domain::MAX_ORDER_LINES).collect();
    andreani_quote(&crate::server::pool()?, tenant.id, &postal_code, &lines).await
}

/// Sucursales cerca de un código postal, guardadas 15 minutos: el comprador
/// las ve al elegir y se vuelven a mirar al confirmar el pedido.
#[cfg(feature = "ssr")]
async fn andreani_branches(
    pool: &db::Pool,
    tenant: uuid::Uuid,
    cp: &str,
) -> Result<Vec<AndreaniBranch>, crate::andreani::Error> {
    type Cache = std::sync::Mutex<
        std::collections::HashMap<String, (std::time::Instant, Vec<AndreaniBranch>)>,
    >;
    static CACHE: std::sync::OnceLock<Cache> = std::sync::OnceLock::new();
    const TTL: std::time::Duration = std::time::Duration::from_secs(15 * 60);
    let cache = CACHE.get_or_init(Default::default);
    let key = format!("{tenant}|{cp}");
    if let Some((at, b)) = cache.lock().unwrap().get(&key) {
        if at.elapsed() < TTL {
            return Ok(b.clone());
        }
    }
    let b = crate::andreani::branches(pool, tenant, cp).await?;
    if !b.is_empty() {
        let mut c = cache.lock().unwrap();
        c.retain(|_, (at, _)| at.elapsed() < TTL);
        c.insert(key, (std::time::Instant::now(), b.clone()));
    }
    Ok(b)
}

/// "Finalizar compra": las sucursales de Andreani cerca de `postal_code`.
#[server]
pub async fn find_branches(postal_code: String) -> Result<Vec<AndreaniBranch>, ServerFnError> {
    let tenant = crate::auth::current_tenant().await?;
    let Some(cp) = domain::normalize_postal_code(&postal_code) else {
        return Ok(Vec::new());
    };
    if andreani_ready(&crate::server::pool()?, tenant.id)
        .await?
        .is_none()
    {
        return Ok(Vec::new());
    }
    if !may_ask_andreani("branches", tenant.id) {
        return Err(ServerFnError::new(TOO_MANY_LOOKUPS));
    }
    andreani_branches(&crate::server::pool()?, tenant.id, &cp)
        .await
        .map_err(|e| {
            leptos::logging::warn!("Andreani: sucursales de {cp}: {e}");
            ServerFnError::new(
                "No pudimos traer las sucursales de Andreani. Probá de nuevo en un momento.",
            )
        })
}

/// Las formas de pago que ofrece la tienda, en orden.
#[cfg(feature = "ssr")]
async fn methods(
    pool: &db::Pool,
    tenant: uuid::Uuid,
) -> Result<Vec<domain::PayMethod>, ServerFnError> {
    let options = db::orders::payment_options(pool, tenant)
        .await
        .map_err(crate::server::internal)?;
    Ok(domain::pay_methods(online(pool, tenant).await?, &options))
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
    let delivery = delivery_with_andreani(&pool, tenant.id).await?;
    let mut form = form;
    let errors = domain::tidy_checkout(&mut form, &delivery);
    if !errors.is_empty() {
        return Ok(PlaceOrder::Invalid { errors });
    }
    let open = db::get_published_page(&pool, tenant.id, "home")
        .await
        .map_err(internal)?
        .is_some_and(|p| p.published > 0);
    let offered = methods(&pool, tenant.id).await?;
    if !open || offered.is_empty() {
        return Ok(PlaceOrder::CartChanged {
            message: "Esta tienda todavía no está recibiendo compras.".into(),
        });
    }
    if !offered.contains(&form.pay) {
        return Ok(PlaceOrder::Invalid {
            errors: vec![FieldError {
                field: "pay".into(),
                message: "Elegí cómo querés pagar.".into(),
            }],
        });
    }
    // Frena los pedidos falsos en serie: los de transferencia guardan stock
    // y mandan emails. 5 cada 10 minutos por conexión y tienda alcanza.
    let ip = crate::limits::client_ip();
    if !crate::limits::allow(
        &format!("order|{}|{ip}", tenant.id),
        5,
        std::time::Duration::from_secs(10 * 60),
    ) {
        return Ok(PlaceOrder::CartChanged {
            message: "Recibimos varios pedidos seguidos desde tu conexión. Esperá unos minutos y probá de nuevo.".into(),
        });
    }
    // Un pedido por transferencia o a coordinar manda la confirmación al
    // email que escribió el comprador, sin saber si es suyo: tope por
    // dirección y por tienda para que no sirva para molestar a terceros.
    if form.pay != domain::PayMethod::MercadoPago {
        let day = std::time::Duration::from_secs(24 * 60 * 60);
        let to = form.email.trim().to_lowercase();
        if !crate::limits::allow(&format!("order-mail|{to}"), 6, day) {
            return Ok(PlaceOrder::CartChanged {
                message: "Ya recibimos varios pedidos con ese email hoy. Si necesitás hacer otro, escribile a la tienda.".into(),
            });
        }
        if !crate::limits::allow(
            &format!("order-store|{}", tenant.id),
            120,
            std::time::Duration::from_secs(60 * 60),
        ) {
            return Ok(PlaceOrder::CartChanged {
                message: "La tienda está recibiendo muchos pedidos en este momento. Probá de nuevo en unos minutos.".into(),
            });
        }
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
        let Some(p) = products
            .iter()
            .find(|p| p.id == *id && p.visible && !p.example)
        else {
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
    // Con Andreani: el envío se cobra con la cotización de recién (la misma
    // que vio el comprador) y la sucursal tiene que existir.
    let invalid = |field: &str, message: &str| {
        Ok(PlaceOrder::Invalid {
            errors: vec![FieldError {
                field: field.into(),
                message: message.into(),
            }],
        })
    };
    let mut shipment = None;
    let mut shipping_cents = domain::shipping_for(form.delivery, &delivery);
    if let Some(svc) = form.delivery.andreani() {
        let q = andreani_quote(&pool, tenant.id, &form.postal_code, &form.lines).await?;
        let Some(cents) = q.cents(svc) else {
            return invalid(
                "delivery",
                q.problem
                    .as_deref()
                    .unwrap_or("Ese envío ya no está disponible. Elegí otra forma de entrega."),
            );
        };
        let mut s = domain::Shipment {
            service: svc,
            quote_cents: cents,
            dni: form.dni.clone(),
            branch_code: String::new(),
            branch_name: String::new(),
            branch_address: String::new(),
            state: domain::ShipmentState::Waiting,
            pedido_id: String::new(),
            tracking: String::new(),
            status: String::new(),
            error: String::new(),
        };
        if svc.needs_branch() {
            let branches = match andreani_branches(&pool, tenant.id, &q.postal_code).await {
                Ok(b) => b,
                Err(e) => {
                    leptos::logging::warn!("Andreani: sucursales al confirmar: {e}");
                    return invalid(
                        "branch",
                        "No pudimos confirmar la sucursal con Andreani. Probá de nuevo en un momento.",
                    );
                }
            };
            let Some(b) = branches.into_iter().find(|b| b.code == form.branch) else {
                return invalid("branch", "Elegí la sucursal donde lo retirás.");
            };
            s.branch_code = b.code;
            s.branch_name = b.name;
            s.branch_address = b.address;
        }
        shipping_cents = cents;
        shipment = Some(s);
    }
    let pay_online = form.pay == domain::PayMethod::MercadoPago;
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
        shipping_cents,
        pay: form.pay,
        shipment: shipment.as_ref(),
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
    if !pay_online {
        // Transferencia o a coordinar: el pedido ya está, espera que el dueño
        // marque el pago. Con Mercado Pago se avisa cuando se aprueba.
        crate::email::order_emails(&pool, tenant.id, id, false).await;
    }
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
            // El intento ya está guardado: desde su página se puede reintentar.
            Err(e) => {
                leptos::logging::log!("Mercado Pago: no pudimos crear el pago del pedido {id}: {e}")
            }
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
    let can_pay = order.status == domain::OrderStatus::PendingPayment
        && order.pay == domain::PayMethod::MercadoPago
        && online(&pool, tenant.id).await?;
    let transfer_info = if order.pay == domain::PayMethod::Transfer {
        db::orders::payment_options(&pool, tenant.id)
            .await
            .map_err(internal)?
            .transfer_text()
    } else {
        String::new()
    };
    Ok(Some(OrderReceipt {
        transfer_info,
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
            o.status == domain::OrderStatus::PendingPayment
                && o.pay == domain::PayMethod::MercadoPago
        })
        .ok_or_else(|| ServerFnError::new("Este pedido ya no se puede pagar."))?;
    crate::mp::create_preference(&pool, tenant.id, &tenant.name, &store_base(), &order)
        .await
        .map_err(|e| {
            leptos::logging::log!("Mercado Pago: pedido {}: {e}", order.id);
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
pub(crate) fn StoreFrame(view: PageView, children: Children) -> impl IntoView {
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
pub(crate) fn StoreHeader(name: String) -> impl IntoView {
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
    city: RwSignal<String>,
    notes: RwSignal<String>,
    pay: RwSignal<domain::PayMethod>,
    /// Con Andreani: código postal, sucursal y DNI.
    postal: RwSignal<String>,
    branch: RwSignal<String>,
    dni: RwSignal<String>,
    /// La última cotización de Andreani para `postal`.
    quote: RwSignal<Option<AndreaniQuote>>,
}

/// Lo que cuesta el envío elegido (con Andreani, según la cotización).
fn ship_cents(f: Fields, d: &DeliverySettings) -> i64 {
    match f.delivery.get() {
        Delivery::Andreani(s) => f
            .quote
            .with(|q| q.as_ref().and_then(|q| q.cents(s)))
            .unwrap_or(0),
        x => domain::shipping_for(x, d),
    }
}

/// Cómo se muestra el costo del envío elegido.
fn ship_text(f: Fields, d: &DeliverySettings) -> String {
    match f.delivery.get() {
        Delivery::Pickup => "Gratis".to_string(),
        Delivery::Shipping => domain::shipping_label(d, format_price),
        Delivery::Andreani(s) => f
            .quote
            .with(|q| q.as_ref().and_then(|q| q.cents(s)))
            .map(format_price)
            .unwrap_or_else(|| "A calcular".to_string()),
    }
}

/// En qué paso está cada campo: 1 tus datos, 2 entrega, 3 pago.
fn step_of(field: &str) -> u8 {
    match field {
        "name" | "email" | "phone" => 1,
        "delivery" | "address" | "postal_code" | "city" | "notes" | "dni" | "branch" => 2,
        _ => 3,
    }
}

const BUYER_KEY: &str = "mk-comprador";

fn buyer_storage() -> Option<leptos::web_sys::Storage> {
    window().local_storage().ok().flatten()
}

/// Guarda en este navegador los datos de contacto y de envío que escribe el
/// comprador, así no los vuelve a escribir si el pago falla o si vuelve a la
/// tienda. No guarda la forma de pago ni la nota.
fn remember_buyer(f: &Fields) {
    let (name, email, phone, address, city, postal, dni) =
        (f.name, f.email, f.phone, f.address, f.city, f.postal, f.dni);
    // Después de hidratar: recupera lo guardado en los campos vacíos.
    Effect::new(move |_| {
        let Some(raw) = buyer_storage().and_then(|s| s.get_item(BUYER_KEY).ok().flatten()) else {
            return;
        };
        let Ok(v) = serde_json::from_str::<std::collections::HashMap<String, String>>(&raw) else {
            return;
        };
        for (sig, key) in [
            (name, "name"),
            (email, "email"),
            (phone, "phone"),
            (address, "address"),
            (city, "city"),
            (postal, "postal"),
            (dni, "dni"),
        ] {
            if sig.get_untracked().is_empty() {
                if let Some(x) = v.get(key) {
                    sig.set(x.clone());
                }
            }
        }
    });
    Effect::new(move |prev: Option<()>| {
        let v = serde_json::json!({
            "name": name.get(), "email": email.get(), "phone": phone.get(),
            "address": address.get(), "city": city.get(), "postal": postal.get(), "dni": dni.get(),
        });
        // La primera vez corre antes de recuperar: no pisa lo guardado con vacío.
        if prev.is_some() {
            if let Some(s) = buyer_storage() {
                _ = s.set_item(BUYER_KEY, &v.to_string());
            }
        }
    });
}

fn forget_buyer() {
    if let Some(s) = buyer_storage() {
        _ = s.remove_item(BUYER_KEY);
    }
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
        city: RwSignal::new(String::new()),
        notes: RwSignal::new(String::new()),
        pay: RwSignal::new(info.pay_methods.first().copied().unwrap_or_default()),
        postal: RwSignal::new(String::new()),
        branch: RwSignal::new(String::new()),
        dni: RwSignal::new(String::new()),
        quote: RwSignal::new(None),
    };
    remember_buyer(&f);
    let has_andreani = d.with_value(|d| !d.andreani.is_empty());
    let lines_now = move || -> Vec<(String, u32)> {
        cart.items()
            .into_iter()
            .filter(|i| i.product.in_stock)
            .map(|i| (i.product.id, i.qty))
            .collect()
    };
    // Cotiza con Andreani cuando el código postal tiene forma de uno.
    let quoting = Action::new(|(cp, lines): &(String, Vec<(String, u32)>)| {
        quote_shipping(cp.clone(), lines.clone())
    });
    Effect::new(move |_| {
        if !has_andreani || !cart.ready.get() {
            return;
        }
        let Some(cp) = domain::normalize_postal_code(&f.postal.get()) else {
            f.quote.set(None);
            return;
        };
        if f.quote
            .with_untracked(|q| q.as_ref().is_some_and(|q| q.postal_code == cp))
        {
            return;
        }
        quoting.dispatch((cp, lines_now()));
    });
    Effect::new(move |_| {
        let Some(r) = quoting.value().get() else {
            return;
        };
        let q = r.unwrap_or_else(|e| AndreaniQuote {
            postal_code: String::new(),
            rates: Vec::new(),
            problem: Some(error_text(&e)),
        });
        // Llegó tarde: el comprador ya cambió el código postal.
        let now = domain::normalize_postal_code(&f.postal.get_untracked()).unwrap_or_default();
        if !q.postal_code.is_empty() && q.postal_code != now {
            return;
        }
        // Si lo elegido ya no está, pasa a lo primero que haya.
        let offered = |x: Delivery| match x {
            Delivery::Andreani(s) => q.cents(s).is_some(),
            Delivery::Shipping => d.with_value(|d| d.shipping_enabled),
            Delivery::Pickup => d.with_value(|d| d.pickup_enabled),
        };
        if !offered(f.delivery.get_untracked()) {
            if let Some(r) = q.rates.first() {
                f.delivery.set(Delivery::Andreani(r.service));
            }
        }
        f.quote.set(Some(q));
    });
    let methods = StoredValue::new(info.pay_methods.clone());
    let step = RwSignal::new(1u8);
    let errors = RwSignal::new(Vec::<FieldError>::new());
    // Uso y abandono: hasta qué paso llegó (solo hacia adelante).
    Effect::new(move |reached: Option<u8>| {
        let now = step.get();
        match (now, reached.unwrap_or(1)) {
            (2, r) if r < 2 => crate::analytics::track_client("entrega", None),
            (3, r) if r < 3 => crate::analytics::track_client("pago", None),
            _ => {}
        }
        now.max(reached.unwrap_or(1))
    });
    let problem = RwSignal::new(None::<String>);
    // Muestra los errores: abre el paso del primero y lleva el foco ahí.
    let show_errors = move |e: Vec<FieldError>| {
        if let Some(first) = e.first() {
            crate::analytics::track_client("error_compra", Some(first.field.clone()));
            step.set(step_of(&first.field));
        }
        focus_first(&e);
        errors.set(e);
    };
    let place = Action::new(|form: &CheckoutForm| send_order(form.clone()));
    let navigate = leptos_router::hooks::use_navigate();
    Effect::new(move |_| match place.value().get() {
        Some(Ok(PlaceOrder::Placed { id, .. })) => {
            cart.clear();
            forget_buyer();
            navigate(&format!("/pedido/{id}"), Default::default());
        }
        // El carrito queda como está hasta que Mercado Pago apruebe el pago:
        // si la persona vuelve sin pagar, lo encuentra igual.
        Some(Ok(PlaceOrder::Pay { url, .. })) => {
            _ = window().location().set_href(&url);
        }
        Some(Ok(PlaceOrder::Invalid { errors: e })) => show_errors(e),
        Some(Ok(PlaceOrder::CartChanged { message })) => problem.set(Some(message)),
        Some(Err(e)) => problem.set(Some(error_text(&e))),
        None => {}
    });
    let form_now = move || CheckoutForm {
        name: f.name.get_untracked(),
        email: f.email.get_untracked(),
        phone: f.phone.get_untracked(),
        delivery: f.delivery.get_untracked(),
        address: f.address.get_untracked(),
        postal_code: f.postal.get_untracked(),
        city: f.city.get_untracked(),
        notes: f.notes.get_untracked(),
        pay: f.pay.get_untracked(),
        lines: lines_now(),
        branch: f.branch.get_untracked(),
        dni: f.dni.get_untracked(),
    };
    // "Seguir": revisa solo los campos de este paso y abre el siguiente.
    let next = move |from: u8| {
        let mut form = form_now();
        let e: Vec<_> = d
            .with_value(|d| domain::tidy_checkout(&mut form, d))
            .into_iter()
            .filter(|e| step_of(&e.field) == from)
            .collect();
        if e.is_empty() {
            errors.set(Vec::new());
            step.set(from + 1);
            focus_id(format!("step-{}", from + 1));
        } else {
            show_errors(e);
        }
    };
    let submit = move || {
        problem.set(None);
        let mut form = form_now();
        let e = d.with_value(|d| domain::tidy_checkout(&mut form, d));
        if e.is_empty() {
            errors.set(Vec::new());
            place.dispatch(form);
        } else {
            show_errors(e);
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
    let data_line = move || {
        [f.name.get(), f.phone.get(), f.email.get()]
            .into_iter()
            .filter(|s| !s.trim().is_empty())
            .collect::<Vec<_>>()
            .join(" · ")
    };
    let branches = RwSignal::new(Vec::<AndreaniBranch>::new());
    let delivery_line = move || match f.delivery.get() {
        Delivery::Andreani(s) if s.needs_branch() => {
            let b = f.branch.get();
            let name = branches.with(|v| {
                v.iter()
                    .find(|x| x.code == b)
                    .map(|x| x.name.clone())
                    .unwrap_or_default()
            });
            format!("{} · {name}", s.label())
        }
        Delivery::Andreani(s) => format!(
            "{} · {}",
            s.label(),
            domain::ship_address(f.address.get().trim(), f.city.get().trim(), "")
        ),
        Delivery::Shipping => format!(
            "Envío · {}",
            domain::ship_address(f.address.get().trim(), f.city.get().trim(), "")
        ),
        Delivery::Pickup => d.with_value(|d| {
            if d.pickup_address.is_empty() {
                "Retiro en el local".to_string()
            } else {
                format!("Retiro · {}", d.pickup_address)
            }
        }),
    };

    view! {
        <StoreFrame view>
            <StoreHeader name=info.store_name/>
            {move || if !cart.ready.get() {
                view! { <ui::Skeleton class="h-96 w-full"/> }.into_any()
            } else if methods.with_value(|m| m.is_empty()) {
                view! {
                    <div class="card bg-base-100 shadow-sm"><div class="card-body items-center gap-3 py-16 text-center">
                        <h1 class="mk-h text-2xl font-bold">"Esta tienda todavía no está recibiendo compras"</h1>
                        <p class="text-base-content/70">"Tu carrito queda guardado. Volvé a intentarlo en unos días."</p>
                        <a class="btn btn-primary" href="/">"Volver a la tienda"</a>
                    </div></div>
                }.into_any()
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
                    <form class="grid items-start gap-5 lg:grid-cols-[1fr_360px]" novalidate
                        on:submit=move |ev| { ev.prevent_default(); submit(); }>
                        <div class="flex min-w-0 flex-col gap-3">
                            <h1 class="mk-h px-1 text-2xl font-bold">"Finalizar compra"</h1>
                            <MobileSummary cart f d=d.get_value()/>
                            <Step n=1 title="Tus datos" step done_line=Signal::derive(data_line)>
                                <TextField id="name" label="Nombre y apellido" value=f.name errors autocomplete="name" max=domain::NAME_MAX/>
                                <div class="grid gap-x-4 sm:grid-cols-2">
                                    <TextField id="phone" label="WhatsApp" value=f.phone errors autocomplete="tel" kind="tel" placeholder="11 5555-1234" max=30/>
                                    <TextField id="email" label="Email" value=f.email errors autocomplete="email" kind="email" placeholder="nombre@correo.com" max=domain::EMAIL_MAX/>
                                </div>
                                <button type="button" class="btn btn-primary mt-3 self-start" on:click=move |_| next(1)>"Seguir"</button>
                            </Step>
                            <Step n=2 title="Entrega" step done_line=Signal::derive(delivery_line)>
                                {has_andreani.then(|| view! {
                                    <div class="mb-2 sm:max-w-xs">
                                        <TextField id="postal_code" label="Código postal" value=f.postal errors autocomplete="postal-code" inputmode="numeric" placeholder="1425" max=10/>
                                        <p role="status" class="-mt-1 text-sm"
                                            class:opacity-70=move || f.quote.with(|q| q.as_ref().is_none_or(|q| q.problem.is_none()))>
                                            {move || if quoting.pending().get() {
                                                "Calculando el envío…".to_string()
                                            } else if let Some(p) = f.quote.with(|q| q.as_ref().and_then(|q| q.problem.clone())) {
                                                p
                                            } else if f.quote.with(Option::is_none) {
                                                "Escribilo para ver el costo del envío con Andreani.".to_string()
                                            } else {
                                                String::new()
                                            }}
                                        </p>
                                    </div>
                                })}
                                <DeliveryChoice f d=d.get_value() errors/>
                                {move || (f.delivery.get() == Delivery::Andreani(AndreaniService::Branch)).then(|| view! {
                                    <BranchPicker f errors branches/>
                                })}
                                {move || f.delivery.get().needs_address().then(|| view! {
                                    <div class="mt-1 grid gap-x-4 sm:grid-cols-[2fr_1fr]">
                                        <TextField id="address" label="Dirección" value=f.address errors autocomplete="street-address" placeholder="Calle, número, piso y depto." max=domain::ADDRESS_MAX/>
                                        <TextField id="city" label="Localidad" value=f.city errors autocomplete="address-level2" placeholder="CABA" max=domain::CITY_MAX/>
                                    </div>
                                })}
                                {move || f.delivery.get().andreani().is_some().then(|| view! {
                                    <div class="sm:max-w-xs">
                                        <TextField id="dni" label="DNI de quien recibe" value=f.dni errors inputmode="numeric" placeholder="30123456" max=12/>
                                        <p class="-mt-1 text-sm text-base-content/70">"Andreani lo pide para entregar el paquete."</p>
                                    </div>
                                })}
                                <Note f/>
                                <button type="button" class="btn btn-primary mt-4 self-start" on:click=move |_| next(2)>"Seguir al pago"</button>
                            </Step>
                            <Step n=3 title="Pago" step done_line=Signal::derive(String::new)>
                                <PayChoice f methods=methods.get_value() errors/>
                                {move || problem.get().map(|m| view! { <p role="alert" class="mt-3 text-sm text-error">{m}</p> })}
                                <PayButton f busy=Signal::derive(busy)/>
                            </Step>
                        </div>
                        <Summary cart f d=d.get_value()/>
                    </form>
                }.into_any()
            }}
        </StoreFrame>
    }
}

/// Un paso de la compra: abierto, hecho (una línea y "Cambiar") o todavía no.
#[component]
fn Step(
    n: u8,
    title: &'static str,
    step: RwSignal<u8>,
    done_line: Signal<String>,
    children: ChildrenFn,
) -> impl IntoView {
    let children = StoredValue::new(children);
    let open = move || step.get() == n;
    let done = move || step.get() > n;
    let later = move || step.get() < n;
    view! {
        <section class="rounded-box border bg-base-100 shadow-sm"
            class:border-base-200=move || !open()
            class:border-primary=open
            aria-labelledby=format!("step-{n}")>
            <div class="flex items-center gap-3 px-5 py-4">
                {move || if done() {
                    view! {
                        <span class="grid h-7 w-7 shrink-0 place-items-center rounded-full bg-success text-success-content" aria-hidden="true">
                            <svg class="h-4 w-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3" stroke-linecap="round" stroke-linejoin="round"><path d="M5 12l5 5L20 7"/></svg>
                        </span>
                    }.into_any()
                } else {
                    view! {
                        <span class="grid h-7 w-7 shrink-0 place-items-center rounded-full text-sm font-bold" aria-hidden="true"
                            class:bg-primary=open class:text-primary-content=open
                            class:bg-base-200=move || !open() class:text-base-content=move || !open()>{n}</span>
                    }.into_any()
                }}
                <div class="min-w-0 flex-1">
                    <h2 id=format!("step-{n}") tabindex="-1" class="mk-h text-lg font-bold outline-none"
                        class:opacity-60=later>
                        {title}
                    </h2>
                    {move || done().then(|| view! { <p class="line-clamp-2 text-sm break-words text-base-content/70">{done_line.get()}</p> })}
                </div>
                {move || done().then(|| view! {
                    <button type="button" class="btn btn-ghost btn-sm text-primary" aria-label=format!("Cambiar: {title}")
                        on:click=move |_| { step.set(n); focus_id(format!("step-{n}")); }>"Cambiar"</button>
                })}
            </div>
            {move || open().then(|| view! { <div class="flex flex-col px-5 pb-5 sm:pl-15">{children.with_value(|c| c())}</div> })}
        </section>
    }
}

/// "+ Agregar una nota para la tienda": la nota es opcional, queda escondida.
#[component]
fn Note(f: Fields) -> impl IntoView {
    let shown = RwSignal::new(!f.notes.get_untracked().is_empty());
    view! {
        {move || if shown.get() {
            view! {
                <fieldset class="fieldset">
                    <label class="fieldset-legend" for="f-notes">"Nota para la tienda "<span class="font-normal text-base-content/60">"(opcional)"</span></label>
                    <textarea id="f-notes" class="textarea w-full" rows="2" maxlength=domain::NOTES_MAX
                        placeholder="Por ejemplo: timbre roto, llamar al llegar"
                        prop:value=move || f.notes.get()
                        on:input=move |ev| f.notes.set(event_target_value(&ev))></textarea>
                </fieldset>
            }.into_any()
        } else {
            view! {
                <button type="button" class="link link-primary link-hover mt-2 self-start text-sm font-semibold no-underline"
                    on:click=move |_| { shown.set(true); focus_id("f-notes".into()); }>
                    "+ Agregar una nota para la tienda"
                </button>
            }.into_any()
        }}
    }
}

/// Lleva el foco (y la vista) a un elemento por su id.
pub(crate) fn focus_id(id: String) {
    use leptos::wasm_bindgen::JsCast;
    request_animation_frame(move || {
        let Some(el) = document()
            .get_element_by_id(&id)
            .and_then(|e| e.dyn_into::<leptos::web_sys::HtmlElement>().ok())
        else {
            return;
        };
        let opts = leptos::web_sys::ScrollIntoViewOptions::new();
        opts.set_block(leptos::web_sys::ScrollLogicalPosition::Center);
        el.scroll_into_view_with_scroll_into_view_options(&opts);
        let focus = leptos::web_sys::FocusOptions::new();
        focus.set_prevent_scroll(true);
        _ = el.focus_with_options(&focus);
    });
}

/// Lleva el foco al primer campo con problema.
fn focus_first(errors: &[FieldError]) {
    let Some(first) = errors.first() else {
        return;
    };
    let id = match first.field.as_str() {
        "delivery" => "f-delivery-0".to_string(),
        "pay" => "f-pay-0".to_string(),
        "cart" => return,
        f => format!("f-{f}"),
    };
    focus_id(id);
}

pub(crate) fn error_for(errors: RwSignal<Vec<FieldError>>, field: &'static str) -> Option<String> {
    errors.with(|e| {
        e.iter()
            .find(|e| e.field == field)
            .map(|e| e.message.clone())
    })
}

/// Un campo de texto con su etiqueta y su error debajo.
#[component]
pub(crate) fn TextField(
    id: &'static str,
    label: &'static str,
    value: RwSignal<String>,
    errors: RwSignal<Vec<FieldError>>,
    #[prop(optional)] autocomplete: &'static str,
    #[prop(optional)] kind: &'static str,
    #[prop(optional)] placeholder: &'static str,
    #[prop(optional)] inputmode: &'static str,
    max: usize,
) -> impl IntoView {
    let error = move || error_for(errors, id);
    let inputmode = (!inputmode.is_empty()).then_some(inputmode);
    let input_id = format!("f-{id}");
    let error_id = format!("f-{id}-error");
    let kind = if kind.is_empty() { "text" } else { kind };
    view! {
        <fieldset class="fieldset">
            <label class="fieldset-legend" for=input_id.clone()>{label}</label>
            <input id=input_id class="input w-full" class:input-error=move || error().is_some()
                type=kind autocomplete=autocomplete placeholder=placeholder maxlength=max required inputmode=inputmode
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

/// Cómo lo recibe: Andreani (cuando ya hay cotización), envío propio o
/// retiro, lado a lado (solo los que ofrece la tienda).
#[component]
fn DeliveryChoice(
    f: Fields,
    d: DeliverySettings,
    errors: RwSignal<Vec<FieldError>>,
) -> impl IntoView {
    let mut own = Vec::new();
    if d.shipping_enabled {
        let price = domain::shipping_label(&d, format_price);
        own.push((Delivery::Shipping, price));
    }
    if d.pickup_enabled {
        let detail = if d.pickup_address.is_empty() {
            "Gratis".to_string()
        } else {
            format!("Gratis · {}", d.pickup_address)
        };
        own.push((Delivery::Pickup, detail));
    }
    let options = move || {
        let mut v: Vec<(Delivery, String)> = f.quote.with(|q| {
            q.iter()
                .flat_map(|q| q.rates.iter())
                .map(|r| {
                    let price = format_price(r.cents);
                    let detail = if r.service.needs_branch() {
                        format!("{price} · retirás en una sucursal")
                    } else {
                        price
                    };
                    (Delivery::Andreani(r.service), detail)
                })
                .collect()
        });
        v.extend(own.iter().cloned());
        v
    };
    let error = move || error_for(errors, "delivery");
    view! {
        <div role="radiogroup" aria-label="Cómo lo recibís" class="grid gap-2 sm:grid-cols-2">
            {move || options().into_iter().enumerate().map(|(i, (kind, detail))| {
                let free = detail.starts_with("Gratis");
                view! {
                    <label class="flex cursor-pointer items-center gap-3 rounded-box border border-base-300 px-4 py-3 has-checked:border-primary has-checked:bg-primary/8 has-checked:outline has-checked:outline-primary">
                        <input type="radio" name="delivery" class="radio radio-primary radio-sm" id=format!("f-delivery-{i}")
                            prop:checked=move || f.delivery.get() == kind
                            on:change=move |_| {
                                f.delivery.set(kind);
                                errors.update(|e| e.retain(|e| e.field != "delivery"));
                            }/>
                        <span class="min-w-0 flex-1">
                            <span class="block font-semibold">{kind.label()}</span>
                            <span class="block truncate text-sm" class:text-success=free class:text-base-content=move || !free class:opacity-70=!free>{detail}</span>
                        </span>
                    </label>
                }
            }).collect_view()}
            {move || error().map(|m| view! { <p class="text-sm text-error sm:col-span-2">{m}</p> })}
        </div>
    }
}

/// La sucursal de Andreani donde retira, entre las cercanas a su código postal.
#[component]
fn BranchPicker(
    f: Fields,
    errors: RwSignal<Vec<FieldError>>,
    branches: RwSignal<Vec<AndreaniBranch>>,
) -> impl IntoView {
    let load = Action::new(|cp: &String| find_branches(cp.clone()));
    let loaded = RwSignal::new(String::new());
    let problem = RwSignal::new(None::<String>);
    Effect::new(move |_| {
        let cp = f
            .quote
            .with(|q| q.as_ref().map(|q| q.postal_code.clone()))
            .unwrap_or_default();
        if cp.is_empty() || loaded.get_untracked() == cp {
            return;
        }
        loaded.set(cp.clone());
        load.dispatch(cp);
    });
    Effect::new(move |_| match load.value().get() {
        Some(Ok(list)) => {
            problem.set(list.is_empty().then(|| {
                "No hay sucursales de Andreani cerca de ese código postal. Elegí envío a domicilio.".to_string()
            }));
            let code = f.branch.get_untracked();
            if !list.iter().any(|b| b.code == code) {
                f.branch.set(if list.len() == 1 {
                    list[0].code.clone()
                } else {
                    String::new()
                });
            }
            branches.set(list);
        }
        Some(Err(e)) => problem.set(Some(error_text(&e))),
        None => {}
    });
    let error = move || error_for(errors, "branch");
    let chosen = move || {
        let code = f.branch.get();
        branches.with(|v| v.iter().find(|b| b.code == code).cloned())
    };
    view! {
        <fieldset class="fieldset mt-1">
            <label class="fieldset-legend" for="f-branch">"Sucursal donde lo retirás"</label>
            <select id="f-branch" class="select w-full" class:select-error=move || error().is_some() required
                aria-invalid=move || error().is_some().to_string()
                aria-describedby=move || error().is_some().then_some("f-branch-error")
                disabled=move || load.pending().get()
                on:change=move |ev| {
                    f.branch.set(event_target_value(&ev));
                    errors.update(|e| e.retain(|e| e.field != "branch"));
                }>
                <option value="" disabled selected=move || f.branch.get().is_empty()>
                    {move || if load.pending().get() { "Buscando sucursales…" } else { "Elegí una sucursal" }}
                </option>
                {move || branches.get().into_iter().map(|b| {
                    let code = b.code.clone();
                    view! {
                        <option value=b.code.clone() selected=move || f.branch.get() == code>
                            {format!("{} · {}", b.name, b.address)}
                        </option>
                    }
                }).collect_view()}
            </select>
            {move || chosen().filter(|b| !b.hours.is_empty()).map(|b| view! {
                <p class="label whitespace-normal">{format!("Horario: {}", b.hours)}</p>
            })}
            {move || problem.get().map(|m| view! { <p role="status" class="label whitespace-normal text-base-content">{m}</p> })}
            {move || error().map(|m| view! { <p id="f-branch-error" class="label text-error">{m}</p> })}
        </fieldset>
    }
}

/// Ícono de cada forma de pago.
fn pay_icon(kind: domain::PayMethod) -> impl IntoView {
    use domain::PayMethod as P;
    let (bg, path, width) = match kind {
        P::MercadoPago => (
            "rounded-full bg-[#009ee3] text-white",
            "M4 13c2.5 2 5 2 8 0s5.5-2 8 0",
            "2.6",
        ),
        P::Transfer => (
            "rounded-field bg-base-200",
            "M3 10h18M5 10v8m4-8v8m6-8v8m4-8v8M3 21h18M12 3l9 5H3z",
            "2",
        ),
        P::Coordinate => (
            "rounded-field bg-base-200",
            "M21 12a8 8 0 0 1-11.6 7.1L4 20l1-4.6A8 8 0 1 1 21 12z",
            "2",
        ),
    };
    view! {
        <span class=format!("grid h-9 w-9 shrink-0 place-items-center {bg}") aria-hidden="true">
            <svg class="h-[18px] w-[18px]" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width=width stroke-linecap="round" stroke-linejoin="round"><path d=path/></svg>
        </span>
    }
}

/// Cómo paga: las formas que ofrece la tienda, una fila cada una.
#[component]
fn PayChoice(
    f: Fields,
    methods: Vec<domain::PayMethod>,
    errors: RwSignal<Vec<FieldError>>,
) -> impl IntoView {
    use domain::PayMethod as P;
    let error = move || error_for(errors, "pay");
    view! {
        <div role="radiogroup" aria-label="Cómo pagás" class="divide-y divide-base-200 overflow-hidden rounded-box border border-base-300">
            {methods.into_iter().enumerate().map(|(i, kind)| {
                let (title, detail) = match kind {
                    P::MercadoPago => ("Mercado Pago", "Tarjeta, débito o dinero en cuenta"),
                    P::Transfer => ("Transferencia", "Te pasamos el alias al confirmar"),
                    P::Coordinate => ("Acordar con la tienda", "Por ejemplo, efectivo al retirar"),
                };
                view! {
                    <label class="flex cursor-pointer items-center gap-3 px-4 py-3 has-checked:bg-primary/8">
                        <input type="radio" name="pay" class="radio radio-primary radio-sm" id=format!("f-pay-{i}")
                            prop:checked=move || f.pay.get() == kind
                            on:change=move |_| {
                                f.pay.set(kind);
                                errors.update(|e| e.retain(|e| e.field != "pay"));
                            }/>
                        {pay_icon(kind)}
                        <span class="min-w-0 flex-1">
                            <span class="block font-semibold">{title}</span>
                            <span class="block text-sm text-base-content/70">{detail}</span>
                        </span>
                    </label>
                }
            }).collect_view()}
        </div>
        {move || error().map(|m| view! { <p class="mt-1 text-sm text-error">{m}</p> })}
    }
}

/// El botón final, con lo que pasa después según cómo paga.
#[component]
fn PayButton(f: Fields, busy: Signal<bool>) -> impl IntoView {
    use domain::PayMethod as P;
    let mp = move || f.pay.get() == P::MercadoPago;
    view! {
        <button type="submit" disabled=busy
            class=move || if mp() {
                "btn btn-lg btn-block mt-4 border-0 bg-[#009ee3] text-white hover:bg-[#008ac6]"
            } else {
                "btn btn-lg btn-block mt-4 btn-primary"
            }>
            {move || match (busy.get(), f.pay.get()) {
                (true, P::MercadoPago) => "Abriendo Mercado Pago…".to_string(),
                (true, _) => "Enviando…".to_string(),
                (false, P::MercadoPago) => "Pagar con Mercado Pago".to_string(),
                (false, _) => "Confirmar pedido".to_string(),
            }}
        </button>
        <p class="mt-2 text-center text-xs text-base-content/70">
            {move || match f.pay.get() {
                P::MercadoPago => "Te llevamos a Mercado Pago para pagar.",
                P::Transfer => "Al confirmar te mostramos adónde transferir.",
                P::Coordinate => "La tienda te escribe por WhatsApp para acordar el pago.",
            }}
        </p>
    }
}

/// Los productos del carrito, uno por fila.
#[component]
fn OrderLines(cart: Cart) -> impl IntoView {
    view! {
        <ul>
            {move || cart.items().into_iter().filter(|i| i.product.in_stock).map(|i| {
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
    }
}

/// Subtotal, envío y total.
#[component]
fn Totals(cart: Cart, f: Fields, d: DeliverySettings) -> impl IntoView {
    let d = StoredValue::new(d);
    let subtotal = move || cart.total_cents();
    let shipping = move || d.with_value(|d| ship_cents(f, d));
    let shipping_text = move || d.with_value(|d| ship_text(f, d));
    view! {
        <div class="mt-3 flex justify-between text-sm text-base-content/75"><span>"Subtotal"</span><span class="tabular-nums">{move || format_price(subtotal())}</span></div>
        <div class="mt-1 flex justify-between text-sm text-base-content/75"><span>"Envío"</span><span>{shipping_text}</span></div>
        <div class="mt-3 flex justify-between text-lg font-bold"><span>"Total"</span><span class="tabular-nums">{move || format_price(subtotal() + shipping())}</span></div>
    }
}

/// En el celular, el pedido va arriba y plegado.
#[component]
fn MobileSummary(cart: Cart, f: Fields, d: DeliverySettings) -> impl IntoView {
    let d2 = d.clone();
    let total = move || cart.total_cents() + ship_cents(f, &d2);
    view! {
        <details class="group rounded-box bg-base-100 shadow-sm lg:hidden">
            <summary class="flex cursor-pointer list-none items-center gap-2 px-5 py-3 text-sm [&::-webkit-details-marker]:hidden">
                <span>{move || { let n = cart.count(); format!("Tu pedido · {n} {}", if n == 1 { "producto" } else { "productos" }) }}</span>
                <span class="ml-auto font-bold tabular-nums">{move || format_price(total())}</span>
                <svg class="h-4 w-4 transition-transform group-open:rotate-180" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" aria-hidden="true"><path d="M6 9l6 6 6-6"/></svg>
            </summary>
            <div class="px-5 pb-4">
                <OrderLines cart/>
                <a href="/" class="link link-hover mt-2 inline-block text-sm text-base-content/70">"Cambiar productos"</a>
                <Totals cart f d/>
            </div>
        </details>
    }
}

/// "Tu pedido" a la derecha (en compu): productos, envío y total.
#[component]
fn Summary(cart: Cart, f: Fields, d: DeliverySettings) -> impl IntoView {
    view! {
        <aside class="card hidden bg-base-100 shadow-sm lg:sticky lg:top-4 lg:block lg:mt-11"><div class="card-body gap-0 p-6">
            <h2 class="mk-h mb-1 text-lg font-bold">"Tu pedido"</h2>
            <OrderLines cart/>
            <a href="/" class="link link-hover mt-2 self-start text-sm text-base-content/70">"Cambiar productos"</a>
            <Totals cart f d/>
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
    let cart = Cart::provide(view.products.clone());
    let OrderReceipt {
        store_name,
        order,
        delivery,
        can_pay,
        transfer_info,
    } = r;
    // El pedido ya entró (pagado, o por transferencia o a coordinar): el
    // carrito se vacía. Si todavía falta pagar en Mercado Pago, queda igual.
    if order.status != domain::OrderStatus::PendingPayment {
        Effect::new(move |_| {
            if cart.ready.get() && cart.count() > 0 {
                cart.clear();
            }
        });
    }
    let title = if order.number > 0 {
        format!("Pedido #{}", order.number)
    } else {
        "Tu compra".to_string()
    };
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
        _ => order.destination().unwrap_or_default(),
    };
    let shipping_text = match order.delivery {
        Delivery::Pickup => "Gratis".to_string(),
        Delivery::Shipping if delivery.shipping_cents.is_none() && order.shipping_cents == 0 => {
            "A coordinar".to_string()
        }
        Delivery::Shipping if order.shipping_cents == 0 => "Gratis".to_string(),
        Delivery::Shipping | Delivery::Andreani(_) => format_price(order.shipping_cents),
    };
    view! {
        <StoreFrame view>
            <StoreHeader name=store_name/>
            <div class="mx-auto flex w-full max-w-2xl flex-col gap-5">
                <ReceiptHead first_name number=order.number phone=order.phone.clone() status=order.status can_pay id=order.id.clone()
                    method=order.pay transfer_info total=order.total_cents/>
                <div class="card bg-base-100 shadow-sm"><div class="card-body gap-0 p-6">
                    <h2 class="mk-h mb-1 text-lg font-bold">{title}</h2>
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
                    {order.shipment.as_ref().filter(|s| !s.tracking.is_empty()).map(|s| view! {
                        <p class="mt-1 text-sm">"Seguimiento de Andreani: "
                            <a class="link link-primary" href=domain::tracking_url(&s.tracking) target="_blank" rel="noopener noreferrer">{s.tracking.clone()}</a>
                        </p>
                    })}
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
    method: domain::PayMethod,
    transfer_info: String,
    total: i64,
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
                    format!("{first_name}, falta pagar tu compra"),
                    "Tu compra se confirma cuando Mercado Pago aprueba el pago. Tu carrito sigue guardado.".to_string(),
                ),
            };
            let id = id.clone();
            view! {
                <h1 class="mk-h text-2xl font-bold">{title}</h1>
                <p class="text-base-content/75">{text}</p>
                {move || problem.get().map(|m| view! { <div role="alert" class="alert alert-warning text-sm">{m}</div> })}
                <button type="button" class="btn btn-primary mt-3" disabled=busy on:click=move |_| { problem.set(None); pay.dispatch(id.clone()); }>
                    {move || if busy() { "Abriendo Mercado Pago…" } else { "Pagar con Mercado Pago" }}
                </button>
                <a class="link link-hover text-sm text-base-content/70" href="/">"Volver a la tienda"</a>
            }.into_any()
        }
        S::PendingPayment => view! {
            <h1 class="mk-h text-2xl font-bold">"Tu compra no se completó"</h1>
            <p class="text-base-content/75">"No se cobró nada. Tu carrito sigue guardado en la tienda."</p>
            <a class="btn btn-primary mt-3" href="/">"Volver a la tienda"</a>
        }.into_any(),
        S::New if method == domain::PayMethod::Transfer => view! {
            <h1 class="mk-h text-2xl font-bold">{format!("{first_name}, ya casi está: falta la transferencia")}</h1>
            <p class="text-base-content/75">{format!("Pedido #{number} · Transferí {} a:", format_price(total))}</p>
            <p class="w-full max-w-md whitespace-pre-line rounded-box bg-base-200 px-4 py-3 text-left font-mono text-sm select-all">{transfer_info}</p>
            <p class="text-sm text-base-content/75">{format!("Cuando la tienda reciba la transferencia, empieza a preparar tu pedido. Si no la recibe en 48 horas, el pedido se cancela. Si hace falta algo, te escribe al {phone}.")}</p>
        }.into_any(),
        _ => view! {
            {check}
            <h1 class="mk-h text-2xl font-bold">{format!("¡Gracias, {first_name}! Recibimos tu pedido")}</h1>
            <p class="text-base-content/75">{format!("Pedido #{number} · la tienda te va a escribir al {phone} para coordinar el pago. Cuando lo reciba, empieza a prepararlo.")}</p>
            <a class="btn btn-primary mt-3" href="/">"Seguir comprando"</a>
        }.into_any(),
    };
    view! {
        <div class="card bg-base-100 shadow-sm"><div class="card-body items-center gap-2 py-10 text-center">
            {body}
        </div></div>
    }
}
