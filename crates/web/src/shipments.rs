//! Envíos con Andreani de los pedidos (solo servidor): crearlos cuando el
//! pedido se paga, seguirlos y bajar la etiqueta.

use axum::extract::{Path, State};
use axum::http::{header, HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::Router;
use domain::{Order, Shipment, ShipmentState};
use uuid::Uuid;

/// Crea en Andreani el envío de un pedido pagado, si va por Andreani y
/// todavía no se creó (o falló). Devuelve el pedido como quedó.
pub async fn ship_order(pool: &db::Pool, tenant: Uuid, order: Uuid) -> Option<Order> {
    match db::orders::claim_shipment(pool, tenant, order).await {
        Ok(true) => {}
        Ok(false) => return db::orders::get(pool, tenant, order).await.ok().flatten(),
        Err(e) => {
            leptos::logging::warn!("Andreani: no se pudo tomar el envío del pedido {order}: {e}");
            return None;
        }
    }
    let o = db::orders::get(pool, tenant, order).await.ok().flatten()?;
    let mut sh = o.shipment.clone()?;
    match create(pool, tenant, &o, &sh).await {
        Ok(id) => {
            leptos::logging::log!("Andreani: pedido #{} creado (envío {id})", o.number);
            sh.state = ShipmentState::Created;
            sh.pedido_id = id;
            sh.error.clear();
        }
        Err(m) => {
            leptos::logging::warn!("Andreani: pedido #{}: {m}", o.number);
            sh.state = ShipmentState::Failed;
            sh.error = m;
            // El dueño se entera aunque no abra el pedido.
            if let (Ok(Some((name, slug))), Ok(owners)) = (
                db::mp::name_and_slug(pool, tenant).await,
                db::mp::owner_emails(pool, tenant).await,
            ) {
                crate::email::send(crate::email::shipment_failed_to_owner(
                    owners, &name, &slug, &o, &sh.error,
                ));
            }
        }
    }
    if let Err(e) = db::orders::save_shipment(pool, tenant, order, &sh).await {
        leptos::logging::warn!("Andreani: no se pudo guardar el envío del pedido {order}: {e}");
    }
    db::orders::get(pool, tenant, order).await.ok().flatten()
}

/// Da de alta el envío. El error, en palabras para el dueño.
async fn create(
    pool: &db::Pool,
    tenant: Uuid,
    o: &Order,
    sh: &Shipment,
) -> Result<String, String> {
    use crate::andreani::Error;
    let acc = db::andreani::account(pool, tenant)
        .await
        .map_err(|e| e.to_string())?
        .ok_or("La cuenta de Andreani no está conectada. Conectala en Entregas y reintentá.")?;
    let settings = acc
        .settings
        .as_ref()
        .map(|s| s.0.clone())
        .ok_or("Falta guardar desde dónde despachás, en Entregas.")?;
    let contract = acc
        .contracts
        .0
        .iter()
        .find(|c| c.service == sh.service)
        .ok_or_else(|| format!("Tu cuenta de Andreani no tiene el servicio \"{}\".", sh.service.label()))?;
    let ids: Vec<Uuid> = o
        .items
        .iter()
        .filter_map(|i| Uuid::parse_str(&i.product_id).ok())
        .collect();
    let products = db::products::by_ids(pool, tenant, &ids)
        .await
        .map_err(|e| e.to_string())?;
    let lines: Vec<_> = o
        .items
        .iter()
        .map(|i| crate::andreani::Line {
            parcel: products
                .iter()
                .find(|p| p.id == i.product_id)
                .and_then(|p| p.parcel)
                .unwrap_or(settings.parcel),
            unit_cents: i.unit_cents,
            qty: i.qty,
        })
        .collect();
    let bigger = lines.iter().any(|l| l.parcel.is_bigger());
    let (street, number, floor) = if sh.service.needs_branch() {
        Default::default()
    } else {
        domain::split_street(&o.address).unwrap_or((o.address.clone(), "S/N".into(), String::new()))
    };
    let (name, last_name) = domain::split_name(&o.customer_name);
    let remito = o.number.to_string();
    crate::andreani::create_shipment(
        pool,
        tenant,
        &crate::andreani::NewShipment {
            client_type: &acc.client_type,
            contract_id: &contract.id,
            quote_cents: sh.quote_cents,
            origin_postal_code: &settings.origin.postal_code,
            street: &street,
            number: &number,
            floor: &floor,
            postal_code: &o.postal_code,
            city: &o.city,
            branch_code: &sh.branch_code,
            name: &name,
            last_name: &last_name,
            phone: &domain::phone_digits(&o.phone),
            dni: &sh.dni,
            email: &o.email,
            lines: &lines,
            bigger,
            merchant_email: &acc.email,
            remito: &remito,
        },
    )
    .await
    .map_err(|e| match e {
        Error::Unauthorized => {
            "Andreani no aceptó el código de integración. Volvé a conectar la cuenta en Entregas y reintentá.".to_string()
        }
        Error::Api(_, m) if !m.is_empty() => format!("Andreani respondió: {m}"),
        Error::Api(..) => "Andreani no pudo crear el envío. Reintentá en unos minutos.".to_string(),
        Error::Network(_) => {
            "Andreani no respondió. Antes de reintentar, fijate en Andreani si el envío se creó igual.".to_string()
        }
    })
}

/// Pregunta a Andreani por los envíos creados de `orders` (hasta 100) y
/// guarda número de seguimiento y estado. Cuando Andreani lo tiene en
/// camino, el pedido pasa a Enviados y el comprador recibe el seguimiento;
/// cuando lo entrega, pasa a Entregados. Devuelve cuántos cambiaron.
pub async fn refresh(pool: &db::Pool, tenant: Uuid, orders: &[Order]) -> usize {
    use domain::OrderStatus;
    let ids: Vec<String> = orders
        .iter()
        .filter_map(|o| o.shipment.as_ref())
        .filter(|s| s.state == ShipmentState::Created && !s.pedido_id.is_empty())
        .map(|s| s.pedido_id.clone())
        .take(100)
        .collect();
    if ids.is_empty() {
        return 0;
    }
    let found = match crate::andreani::track(pool, tenant, &ids).await {
        Ok(f) => f,
        Err(e) => {
            leptos::logging::warn!("Andreani: seguimiento: {e}");
            return 0;
        }
    };
    let mut changed = 0;
    for o in orders {
        let Some(sh) = &o.shipment else { continue };
        let Some(t) = found.iter().find(|t| t.pedido_id == sh.pedido_id) else {
            continue;
        };
        if t.tracking == sh.tracking && t.status == sh.status {
            continue;
        }
        let mut new = sh.clone();
        if !t.tracking.is_empty() {
            new.tracking = t.tracking.clone();
        }
        new.status = t.status.clone();
        let Ok(id) = Uuid::parse_str(&o.id) else { continue };
        if db::orders::save_shipment(pool, tenant, id, &new).await.is_err() {
            continue;
        }
        changed += 1;
        let moving = !new.tracking.is_empty()
            && !new.status.is_empty()
            && new.status != "pendiente de ingreso";
        let delivered = domain::tracking_is_delivered(&new.status);
        // Solo hacia adelante: un pedido que el dueño movió a mano no vuelve atrás.
        if delivered && matches!(o.status, OrderStatus::Paid | OrderStatus::Shipped) {
            let _ = db::orders::set_status(pool, tenant, id, OrderStatus::Delivered).await;
        } else if moving && o.status == OrderStatus::Paid {
            let _ = db::orders::set_status(pool, tenant, id, OrderStatus::Shipped).await;
        }
        // El seguimiento le llega al comprador una vez: cuando aparece.
        if sh.tracking.is_empty() && !new.tracking.is_empty() {
            if let Ok(Some((name, slug))) = db::mp::name_and_slug(pool, tenant).await {
                let mut o = o.clone();
                o.shipment = Some(new.clone());
                crate::email::send(crate::email::shipped_to_buyer(&name, &slug, &o, &new.tracking));
            }
        }
    }
    changed
}

/// Cada 15 minutos sigue los envíos de Andreani de todas las tiendas (los
/// creados en los últimos 30 días que todavía no se entregaron).
pub async fn track_loop(pool: db::Pool) {
    const EVERY: std::time::Duration = std::time::Duration::from_secs(15 * 60);
    match db::orders::recover_stuck_shipments(&pool).await {
        Ok(0) => {}
        Ok(n) => leptos::logging::warn!("Andreani: {n} envío(s) quedaron a medias y pasaron a \"falló\""),
        Err(e) => leptos::logging::warn!("Andreani: no se pudieron revisar los envíos a medias: {e}"),
    }
    loop {
        // Pagados que nunca llegaron a crearse (reinicio justo después del pago).
        if let Ok(waiting) = db::orders::shipments_waiting(&pool).await {
            for (tenant, order) in waiting {
                ship_order(&pool, tenant, order).await;
            }
        }
        tokio::time::sleep(EVERY).await;
        let pending = match db::orders::shipments_to_track(&pool).await {
            Ok(p) => p,
            Err(e) => {
                leptos::logging::warn!("Andreani: no pudimos buscar envíos para seguir: {e}");
                continue;
            }
        };
        let mut by_tenant: std::collections::BTreeMap<Uuid, Vec<Order>> = Default::default();
        for (tenant, order) in pending {
            if let Ok(Some(o)) = db::orders::get(&pool, tenant, order).await {
                by_tenant.entry(tenant).or_default().push(o);
            }
        }
        for (tenant, orders) in by_tenant {
            for chunk in orders.chunks(100) {
                refresh(&pool, tenant, chunk).await;
            }
        }
    }
}

/// `GET /andreani/etiqueta/{pedido}`: la etiqueta en PDF, para el dueño.
pub fn routes<S: Clone + Send + Sync + 'static>(pool: db::Pool) -> Router<S> {
    Router::new()
        .route("/andreani/etiqueta/{order}", get(label))
        .with_state(pool)
}

async fn label(
    State(pool): State<db::Pool>,
    Path(order): Path<String>,
    headers: HeaderMap,
) -> Response {
    let fail = |status: StatusCode, msg: &str| (status, msg.to_string()).into_response();
    let Ok(order) = Uuid::parse_str(&order) else {
        return fail(StatusCode::NOT_FOUND, "Ese pedido no existe.");
    };
    let token = headers
        .get_all(header::COOKIE)
        .iter()
        .filter_map(|v| v.to_str().ok())
        .flat_map(|v| v.split(';'))
        .filter_map(|c| c.trim().split_once('='))
        .find(|(name, _)| *name == crate::auth::COOKIE)
        .map(|(_, v)| v.to_string());
    let user = match token {
        Some(t) => db::auth::session_user(&pool, &t).await.ok().flatten(),
        None => None,
    };
    let Some(user) = user.filter(|u| crate::auth::is_allowed(&u.email)) else {
        return fail(StatusCode::UNAUTHORIZED, "Entrá con tu cuenta para bajar la etiqueta.");
    };
    let Ok(Some(tenant)) = db::orders::tenant_of(&pool, order).await else {
        return fail(StatusCode::NOT_FOUND, "Ese pedido no existe.");
    };
    if !matches!(db::auth::role_in(&pool, tenant, user.id).await, Ok(Some(_))) {
        return fail(StatusCode::NOT_FOUND, "Ese pedido no existe.");
    }
    let Ok(Some(o)) = db::orders::get(&pool, tenant, order).await else {
        return fail(StatusCode::NOT_FOUND, "Ese pedido no existe.");
    };
    let Some(tracking) = o.shipment.as_ref().map(|s| s.tracking.clone()).filter(|t| !t.is_empty()) else {
        return fail(
            StatusCode::CONFLICT,
            "La etiqueta aparece cuando pagás el envío en Andreani.",
        );
    };
    let client_type = match db::andreani::account(&pool, tenant).await {
        Ok(Some(a)) => a.client_type,
        _ => return fail(StatusCode::CONFLICT, "Conectá tu cuenta de Andreani en Entregas."),
    };
    match crate::andreani::label(&pool, tenant, &client_type, &[tracking]).await {
        Ok(pdf) => (
            [
                (header::CONTENT_TYPE, "application/pdf".to_string()),
                (
                    header::CONTENT_DISPOSITION,
                    format!("attachment; filename=\"etiqueta-pedido-{}.pdf\"", o.number),
                ),
                (header::CACHE_CONTROL, "private, no-store".to_string()),
            ],
            pdf,
        )
            .into_response(),
        Err(e) => {
            leptos::logging::warn!("Andreani: etiqueta del pedido {order}: {e}");
            fail(
                StatusCode::BAD_GATEWAY,
                "Andreani no devolvió la etiqueta. Probá de nuevo en unos minutos.",
            )
        }
    }
}
