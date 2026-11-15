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
    })
}

pub async fn save_delivery(pool: &PgPool, tenant: Uuid, d: &DeliverySettings) -> sqlx::Result<()> {
    sqlx::query(
        "UPDATE tenants SET pickup_enabled = $2, pickup_address = $3, pickup_hours = $4,
                shipping_enabled = $5, shipping_cents = $6, shipping_note = $7
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
}

/// Guarda el pedido con el próximo número de la tienda. Devuelve id y número.
pub async fn create(pool: &PgPool, tenant: Uuid, o: &NewOrder<'_>) -> sqlx::Result<(Uuid, i32)> {
    let subtotal: i64 = o.items.iter().map(OrderItem::total_cents).sum();
    let items = serde_json::to_value(o.items).map_err(|e| sqlx::Error::Encode(Box::new(e)))?;
    let mut tx = pool.begin().await?;
    // El UPDATE bloquea la fila de la tienda: dos compras a la vez no
    // pueden llevarse el mismo número.
    let (number,): (i32,) = sqlx::query_as(
        "UPDATE tenants SET next_order_number = next_order_number + 1
         WHERE id = $1 RETURNING next_order_number - 1",
    )
    .bind(tenant)
    .fetch_one(&mut *tx)
    .await?;
    let (id,): (Uuid,) = sqlx::query_as(
        "INSERT INTO orders (tenant_id, number, status, customer_name, email, phone, delivery,
                             address, postal_code, city, notes, items,
                             subtotal_cents, shipping_cents, total_cents)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)
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
    .fetch_one(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok((id, number))
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
        }
    }
}

/// `SELECT … FROM orders` con todas las columnas de un pedido, seguido de `rest`.
macro_rules! select {
    ($rest:literal) => {
        concat!(
            "SELECT id, number, status, customer_name, email, phone, delivery, address, ",
            "postal_code, city, notes, items, subtotal_cents, shipping_cents, total_cents, ",
            "to_char(created_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"') AS created_at ",
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
        "WHERE tenant_id = $1 ORDER BY created_at DESC, number DESC LIMIT 500"
    ))
    .bind(tenant)
    .fetch_all(pool)
    .await?;
    Ok(rows.into_iter().map(Into::into).collect())
}

/// Cambia el estado de un pedido de la tienda. `false` si no existe.
pub async fn set_status(
    pool: &PgPool,
    tenant: Uuid,
    id: Uuid,
    status: OrderStatus,
) -> sqlx::Result<bool> {
    let r = sqlx::query(
        "UPDATE orders SET status = $3, updated_at = now() WHERE tenant_id = $1 AND id = $2",
    )
    .bind(tenant)
    .bind(id)
    .bind(status.as_str())
    .execute(pool)
    .await?;
    Ok(r.rows_affected() == 1)
}
