//! Compras: los ajustes de entrega de cada tienda y sus pedidos.

use domain::{Delivery, DeliverySettings, Order, OrderItem, OrderStatus};
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

#[derive(FromRow)]
struct DeliveryRow {
    pickup_enabled: bool,
    pickup_address: String,
    pickup_hours: String,
    shipping_enabled: bool,
    shipping_cents: Option<i64>,
    shipping_note: String,
}

/// Cómo entrega la tienda.
pub async fn delivery(pool: &PgPool, tenant: Uuid) -> sqlx::Result<DeliverySettings> {
    let r = sqlx::query_as::<_, DeliveryRow>(
        "SELECT pickup_enabled, pickup_address, pickup_hours, shipping_enabled,
                shipping_cents, shipping_note
         FROM tenants WHERE id = $1",
    )
    .bind(tenant)
    .fetch_one(pool)
    .await?;
    Ok(DeliverySettings {
        pickup_enabled: r.pickup_enabled,
        pickup_address: r.pickup_address,
        pickup_hours: r.pickup_hours,
        shipping_enabled: r.shipping_enabled,
        shipping_cents: r.shipping_cents,
        shipping_note: r.shipping_note,
        andreani: Vec::new(),
    })
}

/// Formas de cobro de la tienda además de Mercado Pago.
pub async fn payment_options(pool: &PgPool, tenant: Uuid) -> sqlx::Result<domain::PaymentOptions> {
    let (transfer, transfer_alias, transfer_holder, coordinate): (bool, String, String, bool) =
        sqlx::query_as(
            "SELECT pay_transfer, transfer_alias, transfer_holder, pay_coordinate
             FROM tenants WHERE id = $1",
        )
        .bind(tenant)
        .fetch_one(pool)
        .await?;
    Ok(domain::PaymentOptions {
        transfer,
        transfer_alias,
        transfer_holder,
        coordinate,
    })
}

pub async fn save_payment_options(
    pool: &PgPool,
    tenant: Uuid,
    p: &domain::PaymentOptions,
) -> sqlx::Result<()> {
    sqlx::query(
        "UPDATE tenants SET pay_transfer = $2, transfer_alias = $3, transfer_holder = $4,
                            pay_coordinate = $5
         WHERE id = $1",
    )
    .bind(tenant)
    .bind(p.transfer)
    .bind(&p.transfer_alias)
    .bind(&p.transfer_holder)
    .bind(p.coordinate)
    .execute(pool)
    .await?;
    Ok(())
}

/// Si el dueño ya guardó Entregas alguna vez (la tienda nueva no cuenta).
pub async fn delivery_set(pool: &PgPool, tenant: Uuid) -> sqlx::Result<bool> {
    sqlx::query_scalar("SELECT delivery_set FROM tenants WHERE id = $1")
        .bind(tenant)
        .fetch_one(pool)
        .await
}

pub async fn save_delivery(pool: &PgPool, tenant: Uuid, d: &DeliverySettings) -> sqlx::Result<()> {
    sqlx::query(
        "UPDATE tenants SET pickup_enabled = $2, pickup_address = $3, pickup_hours = $4,
                shipping_enabled = $5, shipping_cents = $6, shipping_note = $7,
                delivery_set = true
         WHERE id = $1",
    )
    .bind(tenant)
    .bind(d.pickup_enabled)
    .bind(&d.pickup_address)
    .bind(&d.pickup_hours)
    .bind(d.shipping_enabled)
    .bind(d.shipping_cents)
    .bind(&d.shipping_note)
    .execute(pool)
    .await?;
    Ok(())
}

/// Un pedido listo para guardar (ya validado y con los precios de la base).
pub struct NewOrder<'a> {
    pub status: OrderStatus,
    pub customer_name: &'a str,
    pub email: &'a str,
    pub phone: &'a str,
    pub delivery: Delivery,
    pub address: &'a str,
    pub postal_code: &'a str,
    pub city: &'a str,
    pub notes: &'a str,
    pub items: &'a [OrderItem],
    pub shipping_cents: i64,
    pub pay: domain::PayMethod,
    /// Con Andreani: sucursal, DNI y cotización.
    pub shipment: Option<&'a domain::Shipment>,
}

/// Cómo salió guardar un pedido.
pub enum Created {
    Saved {
        id: Uuid,
        number: i32,
    },
    /// No alcanzan las unidades de un producto (quedan `left`).
    Short {
        name: String,
        left: u32,
    },
}

/// Guarda el pedido con el próximo número de la tienda y descuenta el stock
/// de los productos que lo cuentan, todo junto: si no alcanza, no se guarda.
pub async fn create(pool: &PgPool, tenant: Uuid, o: &NewOrder<'_>) -> sqlx::Result<Created> {
    let subtotal: i64 = o.items.iter().map(OrderItem::total_cents).sum();
    let items = serde_json::to_value(o.items).map_err(|e| sqlx::Error::Encode(Box::new(e)))?;
    let mut tx = pool.begin().await?;
    // Esperando el pago online es solo un intento: se revisa que alcance,
    // pero el stock se descuenta recién cuando se aprueba el pago.
    let attempt = o.status == OrderStatus::PendingPayment;
    for item in o.items {
        let Ok(product) = Uuid::parse_str(&item.product_id) else {
            continue;
        };
        // Solo baja si alcanza; la fila queda bloqueada hasta el final, así
        // dos compras a la vez no se llevan la misma unidad.
        let done = if attempt {
            sqlx::query(
                "SELECT 1 FROM products
                 WHERE tenant_id = $1 AND id = $2 AND (stock IS NULL OR stock >= $3)",
            )
            .bind(tenant)
            .bind(product)
            .bind(item.qty as i32)
            .execute(&mut *tx)
            .await?
        } else {
            sqlx::query(
                "UPDATE products SET stock = stock - $3
                 WHERE tenant_id = $1 AND id = $2 AND stock IS NOT NULL AND stock >= $3",
            )
            .bind(tenant)
            .bind(product)
            .bind(item.qty as i32)
            .execute(&mut *tx)
            .await?
        };
        if done.rows_affected() == 0 {
            let left: Option<Option<i32>> =
                sqlx::query_scalar("SELECT stock FROM products WHERE tenant_id = $1 AND id = $2")
                    .bind(tenant)
                    .bind(product)
                    .fetch_optional(&mut *tx)
                    .await?;
            if let Some(Some(left)) = left {
                tx.rollback().await?;
                return Ok(Created::Short {
                    name: item.name.clone(),
                    left: left.max(0) as u32,
                });
            }
        }
    }
    // El UPDATE bloquea la fila de la tienda: dos compras a la vez no
    // pueden llevarse el mismo número. Un intento todavía no lleva número.
    let number: Option<i32> = if attempt {
        None
    } else {
        Some(next_number(&mut tx, tenant).await?)
    };
    let (id,): (Uuid,) = sqlx::query_as(
        "INSERT INTO orders (tenant_id, number, status, customer_name, email, phone, delivery,
                             address, postal_code, city, notes, items,
                             subtotal_cents, shipping_cents, total_cents, payment, shipment)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17)
         RETURNING id",
    )
    .bind(tenant)
    .bind(number)
    .bind(o.status.as_str())
    .bind(o.customer_name)
    .bind(o.email)
    .bind(o.phone)
    .bind(o.delivery.as_str())
    .bind(o.address)
    .bind(o.postal_code)
    .bind(o.city)
    .bind(o.notes)
    .bind(items)
    .bind(subtotal)
    .bind(o.shipping_cents)
    .bind(subtotal + o.shipping_cents)
    .bind(o.pay.as_str())
    .bind(o.shipment.map(sqlx::types::Json))
    .fetch_one(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(Created::Saved {
        id,
        number: number.unwrap_or(0),
    })
}

/// El próximo número de pedido de la tienda (bloquea su fila hasta el final).
async fn next_number(tx: &mut sqlx::PgConnection, tenant: Uuid) -> sqlx::Result<i32> {
    sqlx::query_scalar(
        "UPDATE tenants SET next_order_number = next_order_number + 1
         WHERE id = $1 RETURNING next_order_number - 1",
    )
    .bind(tenant)
    .fetch_one(tx)
    .await
}

/// Se aprobó el pago online del pedido: entra al proceso. Recibe su número,
/// descuenta el stock (sin bajar de 0: el pago ya está hecho) y queda
/// pagado. `false` si ya estaba pagado, no existe o se pagó menos.
pub async fn mark_paid(
    pool: &PgPool,
    tenant: Uuid,
    order: Uuid,
    payment_id: &str,
    paid_cents: i64,
) -> sqlx::Result<bool> {
    let mut tx = pool.begin().await?;
    let row: Option<(Option<i32>, serde_json::Value)> = sqlx::query_as(
        "SELECT number, items FROM orders
         WHERE tenant_id = $1 AND id = $2 AND status IN ('new', 'pending_payment')
           AND total_cents <= $3
         FOR UPDATE",
    )
    .bind(tenant)
    .bind(order)
    .bind(paid_cents)
    .fetch_optional(&mut *tx)
    .await?;
    let Some((number, items)) = row else {
        return Ok(false);
    };
    // Los pedidos con número ya descontaron su stock al crearse.
    if number.is_none() {
        let items: Vec<OrderItem> = serde_json::from_value(items).unwrap_or_default();
        for item in items {
            let Ok(product) = Uuid::parse_str(&item.product_id) else {
                continue;
            };
            sqlx::query(
                "UPDATE products SET stock = GREATEST(stock - $3, 0)
                 WHERE tenant_id = $1 AND id = $2 AND stock IS NOT NULL",
            )
            .bind(tenant)
            .bind(product)
            .bind(item.qty as i32)
            .execute(&mut *tx)
            .await?;
        }
        let n = next_number(&mut tx, tenant).await?;
        sqlx::query("UPDATE orders SET number = $3 WHERE tenant_id = $1 AND id = $2")
            .bind(tenant)
            .bind(order)
            .bind(n)
            .execute(&mut *tx)
            .await?;
    }
    sqlx::query(
        "UPDATE orders SET status = 'paid', mp_payment_id = $3, paid_at = now(), updated_at = now()
         WHERE tenant_id = $1 AND id = $2",
    )
    .bind(tenant)
    .bind(order)
    .bind(payment_id)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(true)
}

#[derive(FromRow)]
struct OrderRow {
    id: Uuid,
    number: i32,
    status: String,
    customer_name: String,
    email: String,
    phone: String,
    delivery: String,
    address: String,
    postal_code: String,
    city: String,
    notes: String,
    items: serde_json::Value,
    subtotal_cents: i64,
    shipping_cents: i64,
    total_cents: i64,
    created_at: String,
    payment: String,
    regret_code: String,
    shipment: Option<sqlx::types::Json<domain::Shipment>>,
}

impl From<OrderRow> for Order {
    fn from(r: OrderRow) -> Self {
        Order {
            id: r.id.to_string(),
            number: r.number,
            status: OrderStatus::from_str(&r.status),
            customer_name: r.customer_name,
            email: r.email,
            phone: r.phone,
            delivery: Delivery::from_str(&r.delivery),
            address: r.address,
            postal_code: r.postal_code,
            city: r.city,
            notes: r.notes,
            items: serde_json::from_value(r.items).unwrap_or_default(),
            subtotal_cents: r.subtotal_cents,
            shipping_cents: r.shipping_cents,
            total_cents: r.total_cents,
            created_at: r.created_at,
            pay: domain::PayMethod::from_str(&r.payment),
            regret_code: r.regret_code,
            shipment: r.shipment.map(|j| j.0),
        }
    }
}

/// `SELECT … FROM orders` con todas las columnas de un pedido, seguido de `rest`.
macro_rules! select {
    ($rest:literal) => {
        concat!(
            "SELECT id, COALESCE(number, 0) AS number, status, customer_name, email, phone, delivery, address, ",
            "postal_code, city, notes, items, subtotal_cents, shipping_cents, total_cents, ",
            "to_char(created_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"') AS created_at, payment, regret_code, shipment ",
            "FROM orders ",
            $rest
        )
    };
}

/// Un pedido de la tienda.
pub async fn get(pool: &PgPool, tenant: Uuid, id: Uuid) -> sqlx::Result<Option<Order>> {
    let row = sqlx::query_as::<_, OrderRow>(select!("WHERE tenant_id = $1 AND id = $2"))
        .bind(tenant)
        .bind(id)
        .fetch_optional(pool)
        .await?;
    Ok(row.map(Into::into))
}

/// Los pedidos de la tienda, el más nuevo primero.
pub async fn list(pool: &PgPool, tenant: Uuid) -> sqlx::Result<Vec<Order>> {
    let rows = sqlx::query_as::<_, OrderRow>(select!(
        "WHERE tenant_id = $1 AND number IS NOT NULL
         ORDER BY created_at DESC, number DESC LIMIT 500"
    ))
    .bind(tenant)
    .fetch_all(pool)
    .await?;
    Ok(rows.into_iter().map(Into::into).collect())
}

/// Cambia el estado de un pedido de la tienda. `false` si no existe. Al
/// cancelar, las unidades vuelven al stock; al recuperar un cancelado, se
/// descuentan otra vez (sin bajar de 0).
pub async fn set_status(
    pool: &PgPool,
    tenant: Uuid,
    id: Uuid,
    status: OrderStatus,
) -> sqlx::Result<bool> {
    let mut tx = pool.begin().await?;
    let before: Option<(String, serde_json::Value)> = sqlx::query_as(
        "SELECT status, items FROM orders WHERE tenant_id = $1 AND id = $2 FOR UPDATE",
    )
    .bind(tenant)
    .bind(id)
    .fetch_optional(&mut *tx)
    .await?;
    let Some((before, items)) = before else {
        return Ok(false);
    };
    let was_cancelled = OrderStatus::from_str(&before) == OrderStatus::Cancelled;
    let cancel = status == OrderStatus::Cancelled;
    if was_cancelled != cancel {
        let items: Vec<OrderItem> = serde_json::from_value(items).unwrap_or_default();
        let sign: i32 = if cancel { 1 } else { -1 };
        for item in items {
            let Ok(product) = Uuid::parse_str(&item.product_id) else {
                continue;
            };
            sqlx::query(
                "UPDATE products SET stock = GREATEST(stock + $3, 0)
                 WHERE tenant_id = $1 AND id = $2 AND stock IS NOT NULL",
            )
            .bind(tenant)
            .bind(product)
            .bind(sign * item.qty as i32)
            .execute(&mut *tx)
            .await?;
        }
    }
    sqlx::query(
        "UPDATE orders SET status = $3, updated_at = now() WHERE tenant_id = $1 AND id = $2",
    )
    .bind(tenant)
    .bind(id)
    .bind(status.as_str())
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(true)
}

/// Descarta los intentos de compra que no se pagaron en `hours` horas (no
/// tienen número ni descontaron stock: el dueño nunca los vio). Los pedidos
/// por transferencia o a coordinar que el dueño no marcó como pagados en ese
/// tiempo se cancelan, y sus unidades vuelven al stock. Devuelve cuántos sacó.
pub async fn expire_unpaid(pool: &PgPool, hours: i32) -> sqlx::Result<usize> {
    let dropped = sqlx::query(
        "DELETE FROM orders
         WHERE status = 'pending_payment' AND number IS NULL
           AND updated_at < now() - make_interval(hours => $1)",
    )
    .bind(hours)
    .execute(pool)
    .await?
    .rows_affected() as usize;
    let stale: Vec<(Uuid, Uuid)> = sqlx::query_as(
        "SELECT tenant_id, id FROM orders
         WHERE status IN ('new', 'pending_payment') AND number IS NOT NULL
           AND updated_at < now() - make_interval(hours => $1)",
    )
    .bind(hours)
    .fetch_all(pool)
    .await?;
    let mut done = dropped;
    for (tenant, id) in stale {
        if set_status(pool, tenant, id, OrderStatus::Cancelled).await? {
            done += 1;
        }
    }
    Ok(done)
}

/// Números de la tienda para su Inicio en el panel.
#[derive(Debug, Clone, Default, PartialEq, FromRow)]
pub struct Summary {
    /// Nuevos o esperando el pago.
    pub awaiting_payment: i64,
    /// Pagados, falta prepararlos.
    pub to_prepare: i64,
    /// Pagados, enviados o entregados este mes (hora de Argentina).
    pub month_orders: i64,
    pub month_cents: i64,
}

pub async fn summary(pool: &PgPool, tenant: Uuid) -> sqlx::Result<Summary> {
    sqlx::query_as::<_, Summary>(
        "WITH month AS (
            SELECT date_trunc('month', now() AT TIME ZONE 'America/Argentina/Buenos_Aires')
                   AT TIME ZONE 'America/Argentina/Buenos_Aires' AS start
         )
         SELECT
            count(*) FILTER (WHERE status IN ('new', 'pending_payment')
                             AND number IS NOT NULL) AS awaiting_payment,
            count(*) FILTER (WHERE status = 'paid') AS to_prepare,
            count(*) FILTER (WHERE status IN ('paid', 'shipped', 'delivered')
                             AND created_at >= (SELECT start FROM month)) AS month_orders,
            COALESCE(sum(total_cents) FILTER (WHERE status IN ('paid', 'shipped', 'delivered')
                             AND created_at >= (SELECT start FROM month)), 0)::bigint AS month_cents
         FROM orders WHERE tenant_id = $1",
    )
    .bind(tenant)
    .fetch_one(pool)
    .await
}

/// Los últimos `n` pedidos de la tienda, el más nuevo primero.
pub async fn recent(pool: &PgPool, tenant: Uuid, n: i64) -> sqlx::Result<Vec<Order>> {
    let rows = sqlx::query_as::<_, OrderRow>(select!(
        "WHERE tenant_id = $1 AND number IS NOT NULL
         ORDER BY created_at DESC, number DESC LIMIT $2"
    ))
    .bind(tenant)
    .bind(n)
    .fetch_all(pool)
    .await?;
    Ok(rows.into_iter().map(Into::into).collect())
}

/// Cuántos pedidos falta preparar en cada tienda de la persona, por dirección.
pub async fn to_prepare_by_store(pool: &PgPool, user: Uuid) -> sqlx::Result<Vec<(String, i64)>> {
    sqlx::query_as(
        "SELECT t.slug, count(o.id)
         FROM memberships m
         JOIN tenants t ON t.id = m.tenant_id
         LEFT JOIN orders o ON o.tenant_id = t.id AND o.status = 'paid'
         WHERE m.user_id = $1
         GROUP BY t.slug",
    )
    .bind(user)
    .fetch_all(pool)
    .await
}

/// Botón de arrepentimiento: el comprador del pedido `number` (con ese
/// email) pide revocar la compra. Devuelve el id del pedido, el código del
/// pedido de arrepentimiento y si es la primera vez. `None` si no hay un
/// pedido con ese número y email.
pub async fn request_regret(
    pool: &PgPool,
    tenant: Uuid,
    number: i32,
    email: &str,
) -> sqlx::Result<Option<(Uuid, String, bool)>> {
    use rand::Rng;
    const LETTERS: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789";
    let code: String = {
        let mut rng = rand::rng();
        (0..6)
            .map(|_| LETTERS[rng.random_range(0..LETTERS.len())] as char)
            .collect()
    };
    let code = format!("ARR-{code}");
    sqlx::query_as(
        "UPDATE orders
         SET regret_code = CASE WHEN regret_code = '' THEN $4 ELSE regret_code END,
             regret_at = COALESCE(regret_at, now()),
             updated_at = now()
         WHERE tenant_id = $1 AND number = $2 AND lower(email) = lower($3)
         RETURNING id, regret_code, regret_code = $4",
    )
    .bind(tenant)
    .bind(number)
    .bind(email.trim())
    .bind(&code)
    .fetch_optional(pool)
    .await
}

/// Toma el alta del envío con Andreani de un pedido pagado: lo pasa a
/// "creando" si estaba esperando o había fallado. `false` si no corresponde
/// (no está pagado, no va por Andreani o ya se está creando o está creado).
/// Así dos intentos a la vez no crean dos envíos.
pub async fn claim_shipment(pool: &PgPool, tenant: Uuid, order: Uuid) -> sqlx::Result<bool> {
    let done = sqlx::query(
        "UPDATE orders SET shipment = jsonb_set(shipment, '{state}', '\"Creating\"')
         WHERE tenant_id = $1 AND id = $2 AND shipment IS NOT NULL
           AND status IN ('paid', 'shipped', 'delivered')
           AND shipment->>'state' IN ('Waiting', 'Failed')",
    )
    .bind(tenant)
    .bind(order)
    .execute(pool)
    .await?;
    Ok(done.rows_affected() == 1)
}

/// Guarda cómo quedó el envío con Andreani de un pedido.
pub async fn save_shipment(
    pool: &PgPool,
    tenant: Uuid,
    order: Uuid,
    shipment: &domain::Shipment,
) -> sqlx::Result<()> {
    sqlx::query("UPDATE orders SET shipment = $3 WHERE tenant_id = $1 AND id = $2")
        .bind(tenant)
        .bind(order)
        .bind(sqlx::types::Json(shipment))
        .execute(pool)
        .await?;
    Ok(())
}

/// La tienda de un pedido.
pub async fn tenant_of(pool: &PgPool, order: Uuid) -> sqlx::Result<Option<Uuid>> {
    sqlx::query_scalar("SELECT tenant_id FROM orders WHERE id = $1")
        .bind(order)
        .fetch_optional(pool)
        .await
}

/// Pedidos con envío de Andreani creado que todavía no se entregó, de los
/// últimos 30 días (más viejos se dejan de seguir).
pub async fn shipments_to_track(pool: &PgPool) -> sqlx::Result<Vec<(Uuid, Uuid)>> {
    sqlx::query_as(
        "SELECT tenant_id, id FROM orders
         WHERE shipment->>'state' = 'Created'
           AND COALESCE(shipment->>'status', '') <> 'entregado'
           AND status IN ('paid', 'shipped')
           AND created_at > now() - interval '30 days'
         ORDER BY tenant_id, created_at",
    )
    .fetch_all(pool)
    .await
}
