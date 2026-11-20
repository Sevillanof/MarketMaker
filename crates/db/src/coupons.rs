//! Cupones de descuento de cada tienda.

use domain::{Coupon, CouponKind};
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

#[derive(FromRow)]
struct Row {
    id: Uuid,
    code: String,
    kind: String,
    value: i64,
    min_cents: i64,
    expires: String,
    max_uses: i32,
    uses: i32,
    active: bool,
}

impl From<Row> for Coupon {
    fn from(r: Row) -> Self {
        Coupon {
            id: r.id.to_string(),
            code: r.code,
            kind: CouponKind::from_str(&r.kind),
            value: r.value,
            min_cents: r.min_cents,
            expires: r.expires,
            max_uses: r.max_uses.max(0) as u32,
            uses: r.uses.max(0) as u32,
            active: r.active,
        }
    }
}

/// `SELECT … FROM coupons` con todas las columnas, seguido de `rest`.
macro_rules! select {
    ($rest:literal) => {
        concat!(
            "SELECT id, code, kind, value, min_cents, COALESCE(to_char(expires, 'YYYY-MM-DD'), '') AS expires, ",
            "COALESCE(max_uses, 0) AS max_uses, uses, active FROM coupons ",
            $rest
        )
    };
}

/// Los cupones de la tienda, el más nuevo primero.
pub async fn list(pool: &PgPool, tenant: Uuid) -> sqlx::Result<Vec<Coupon>> {
    let rows = sqlx::query_as::<_, Row>(select!(
        "WHERE tenant_id = $1 ORDER BY created_at DESC, code"
    ))
    .bind(tenant)
    .fetch_all(pool)
    .await?;
    Ok(rows.into_iter().map(Into::into).collect())
}

/// El cupón con ese código (ya normalizado).
pub async fn by_code(pool: &PgPool, tenant: Uuid, code: &str) -> sqlx::Result<Option<Coupon>> {
    let row = sqlx::query_as::<_, Row>(select!("WHERE tenant_id = $1 AND code = $2"))
        .bind(tenant)
        .bind(code)
        .fetch_optional(pool)
        .await?;
    Ok(row.map(Into::into))
}

/// La fecha de hoy en Argentina, `AAAA-MM-DD`.
pub async fn today(pool: &PgPool) -> sqlx::Result<String> {
    sqlx::query_scalar(
        "SELECT to_char(now() AT TIME ZONE 'America/Argentina/Buenos_Aires', 'YYYY-MM-DD')",
    )
    .fetch_one(pool)
    .await
}

/// Cómo salió guardar un cupón.
#[derive(Debug, PartialEq, Eq)]
pub enum Saved {
    Saved,
    /// Ya hay otro cupón con ese código.
    CodeTaken,
    /// El cupón a editar no existe.
    Missing,
}

/// Crea el cupón (`c.id` vacío) o cambia el que tiene ese `id` (conserva
/// sus usos y si está activo).
pub async fn save(pool: &PgPool, tenant: Uuid, c: &Coupon) -> sqlx::Result<Saved> {
    let max_uses = (c.max_uses > 0).then_some(c.max_uses as i32);
    let expires = (!c.expires.is_empty()).then_some(c.expires.as_str());
    let result = if c.id.is_empty() {
        sqlx::query(
            "INSERT INTO coupons (tenant_id, code, kind, value, min_cents, expires, max_uses)
             VALUES ($1, $2, $3, $4, $5, $6::date, $7)",
        )
        .bind(tenant)
        .bind(&c.code)
        .bind(c.kind.as_str())
        .bind(c.value)
        .bind(c.min_cents)
        .bind(expires)
        .bind(max_uses)
        .execute(pool)
        .await
    } else {
        let Ok(id) = Uuid::parse_str(&c.id) else {
            return Ok(Saved::Missing);
        };
        sqlx::query(
            "UPDATE coupons SET code = $3, kind = $4, value = $5, min_cents = $6,
                    expires = $7::date, max_uses = $8
             WHERE tenant_id = $1 AND id = $2",
        )
        .bind(tenant)
        .bind(id)
        .bind(&c.code)
        .bind(c.kind.as_str())
        .bind(c.value)
        .bind(c.min_cents)
        .bind(expires)
        .bind(max_uses)
        .execute(pool)
        .await
    };
    match result {
        Ok(r) if r.rows_affected() == 0 => Ok(Saved::Missing),
        Ok(_) => Ok(Saved::Saved),
        Err(sqlx::Error::Database(e)) if e.is_unique_violation() => Ok(Saved::CodeTaken),
        Err(e) => Err(e),
    }
}

pub async fn set_active(pool: &PgPool, tenant: Uuid, id: Uuid, active: bool) -> sqlx::Result<bool> {
    let r = sqlx::query("UPDATE coupons SET active = $3 WHERE tenant_id = $1 AND id = $2")
        .bind(tenant)
        .bind(id)
        .bind(active)
        .execute(pool)
        .await?;
    Ok(r.rows_affected() == 1)
}

pub async fn delete(pool: &PgPool, tenant: Uuid, id: Uuid) -> sqlx::Result<bool> {
    let r = sqlx::query("DELETE FROM coupons WHERE tenant_id = $1 AND id = $2")
        .bind(tenant)
        .bind(id)
        .execute(pool)
        .await?;
    Ok(r.rows_affected() == 1)
}
