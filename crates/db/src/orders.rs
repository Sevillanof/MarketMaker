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

/// Cómo salió guardar un pedido.
pub enum Created {
    Saved { id: Uuid, number: i32 },
    /// No alcanzan las unidades de un producto (quedan `left`).
    Short { name: String, left: u32 },
}

/// Guarda el pedido con el próximo número de la tienda y descuenta el stock
/// de los productos que lo cuentan, todo junto: si no alcanza, no se guarda.
pub async fn create(pool: &PgPool, tenant: Uuid, o: &NewOrder<'_>) -> sqlx::Result<Created> {
    let subtotal: i64 = o.items.iter().map(OrderItem::total_cents).sum();
    let items = serde_json::to_value(o.items).map_err(|e| sqlx::Error::Encode(Box::new(e)))?;
    let mut tx = pool.begin().await?;
    for item in o.items {
        let Ok(product) = Uuid::parse_str(&item.product_id) else {
            continue;
        };
        // Solo baja si alcanza; la fila queda bloqueada hasta el final, así
        // dos compras a la vez no se llevan la misma unidad.
        let done = sqlx::query(
            "UPDATE products SET stock = stock - $3
             WHERE tenant_id = $1 AND id = $2 AND stock IS NOT NULL AND stock >= $3",
        )
        .bind(tenant)
        .bind(product)
        .bind(item.qty as i32)
        .execute(&mut *tx)
        .await?;
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
    Ok(Created::Saved { id, number })
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
    sqlx::query("UPDATE orders SET status = $3, updated_at = now() WHERE tenant_id = $1 AND id = $2")
        .bind(tenant)
        .bind(id)
        .bind(status.as_str())
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(true)
}
