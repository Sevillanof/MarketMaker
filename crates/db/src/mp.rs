//! Mercado Pago: la cuenta vinculada de cada tienda y el pago de cada pedido.

use domain::MpLink;
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

/// Guarda el `state` de un "Vincular mi cuenta" que empieza.
pub async fn save_state(pool: &PgPool, tenant: Uuid, state: &str) -> sqlx::Result<()> {
    // De paso, los que nunca volvieron.
    sqlx::query("DELETE FROM mp_oauth_states WHERE created_at < now() - interval '1 day'")
        .execute(pool)
        .await?;
    sqlx::query("INSERT INTO mp_oauth_states (state, tenant_id) VALUES ($1, $2)")
        .bind(state)
        .bind(tenant)
        .execute(pool)
        .await?;
    Ok(())
}

/// La tienda de un `state` de hace menos de 30 minutos. Se usa una sola vez.
pub async fn take_state(pool: &PgPool, state: &str) -> sqlx::Result<Option<Uuid>> {
    sqlx::query_scalar(
        "DELETE FROM mp_oauth_states WHERE state = $1
         RETURNING CASE WHEN created_at > now() - interval '30 minutes' THEN tenant_id END",
    )
    .bind(state)
    .fetch_optional(pool)
    .await
    .map(Option::flatten)
}

/// Lo que devuelve Mercado Pago al vincular o renovar.
pub struct Tokens<'a> {
    pub user_id: i64,
    pub access_token: &'a str,
    pub refresh_token: &'a str,
    pub public_key: &'a str,
    pub expires_in_secs: i64,
    pub live: bool,
}

pub async fn save_link(pool: &PgPool, tenant: Uuid, t: &Tokens<'_>) -> sqlx::Result<()> {
    sqlx::query(
        "UPDATE tenants SET mp_user_id = $2, mp_access_token = $3, mp_refresh_token = $4,
                mp_public_key = $5, mp_token_expires_at = now() + make_interval(secs => $6),
                mp_linked_at = COALESCE(CASE WHEN mp_user_id = $2 THEN mp_linked_at END, now()),
                mp_live = $7
         WHERE id = $1",
    )
    .bind(tenant)
    .bind(t.user_id)
    .bind(t.access_token)
    .bind(t.refresh_token)
    .bind(t.public_key)
    .bind(t.expires_in_secs as f64)
    .bind(t.live)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn unlink(pool: &PgPool, tenant: Uuid) -> sqlx::Result<()> {
    sqlx::query(
        "UPDATE tenants SET mp_user_id = NULL, mp_access_token = NULL, mp_refresh_token = NULL,
                mp_public_key = NULL, mp_token_expires_at = NULL, mp_linked_at = NULL,
                mp_live = false
         WHERE id = $1",
    )
    .bind(tenant)
    .execute(pool)
    .await?;
    Ok(())
}

#[derive(FromRow)]
struct LinkRow {
    mp_user_id: Option<i64>,
    linked_at: Option<String>,
    mp_live: bool,
}

/// La cuenta vinculada, para mostrarla.
pub async fn link(pool: &PgPool, tenant: Uuid) -> sqlx::Result<Option<MpLink>> {
    let r = sqlx::query_as::<_, LinkRow>(
        "SELECT mp_user_id, mp_live,
                to_char(mp_linked_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"') AS linked_at
         FROM tenants WHERE id = $1",
    )
    .bind(tenant)
    .fetch_one(pool)
    .await?;
    Ok(r.mp_user_id.map(|user_id| MpLink {
        user_id,
        linked_at: r.linked_at.unwrap_or_default(),
        live: r.mp_live,
    }))
}

/// Las llaves de la cuenta vinculada, para hablar con Mercado Pago.
#[derive(FromRow, Clone)]
pub struct Credentials {
    pub access_token: String,
    pub refresh_token: String,
    /// Vence en menos de una semana: hay que renovarlo.
    pub expiring: bool,
}

pub async fn credentials(pool: &PgPool, tenant: Uuid) -> sqlx::Result<Option<Credentials>> {
    sqlx::query_as::<_, Credentials>(
        "SELECT mp_access_token AS access_token, mp_refresh_token AS refresh_token,
                COALESCE(mp_token_expires_at < now() + interval '7 days', true) AS expiring
         FROM tenants WHERE id = $1 AND mp_access_token IS NOT NULL",
    )
    .bind(tenant)
    .fetch_optional(pool)
    .await
}

/// Guarda la preferencia de pago de un pedido (adónde se lo manda a pagar).
pub async fn set_preference(
    pool: &PgPool,
    tenant: Uuid,
    order: Uuid,
    preference_id: &str,
    init_point: &str,
) -> sqlx::Result<()> {
    sqlx::query(
        "UPDATE orders SET mp_preference_id = $3, mp_init_point = $4, updated_at = now()
         WHERE tenant_id = $1 AND id = $2",
    )
    .bind(tenant)
    .bind(order)
    .bind(preference_id)
    .bind(init_point)
    .execute(pool)
    .await?;
    Ok(())
}

/// Adónde mandar a pagar un pedido que todavía no se pagó.
pub async fn init_point(pool: &PgPool, tenant: Uuid, order: Uuid) -> sqlx::Result<Option<String>> {
    sqlx::query_scalar(
        "SELECT mp_init_point FROM orders
         WHERE tenant_id = $1 AND id = $2 AND status IN ('new', 'pending_payment')",
    )
    .bind(tenant)
    .bind(order)
    .fetch_optional(pool)
    .await
    .map(Option::flatten)
}

/// Marca pagado un pedido que esperaba el pago, si el monto alcanza. `true`
/// si cambió.
pub async fn mark_paid(
    pool: &PgPool,
    tenant: Uuid,
    order: Uuid,
    payment_id: &str,
    paid_cents: i64,
) -> sqlx::Result<bool> {
    let r = sqlx::query(
        "UPDATE orders SET status = 'paid', mp_payment_id = $3, paid_at = now(), updated_at = now()
         WHERE tenant_id = $1 AND id = $2 AND status IN ('new', 'pending_payment')
           AND total_cents <= $4",
    )
    .bind(tenant)
    .bind(order)
    .bind(payment_id)
    .bind(paid_cents)
    .execute(pool)
    .await?;
    Ok(r.rows_affected() == 1)
}

/// La dirección corta (`slug`) de una tienda.
pub async fn slug_of(pool: &PgPool, tenant: Uuid) -> sqlx::Result<Option<String>> {
    sqlx::query_scalar("SELECT slug FROM tenants WHERE id = $1")
        .bind(tenant)
        .fetch_optional(pool)
        .await
}

/// Nombre y dirección corta de una tienda, para los emails.
pub async fn name_and_slug(pool: &PgPool, tenant: Uuid) -> sqlx::Result<Option<(String, String)>> {
    sqlx::query_as("SELECT name, slug FROM tenants WHERE id = $1")
        .bind(tenant)
        .fetch_optional(pool)
        .await
}

/// Los emails de los dueños de una tienda (para avisar las ventas).
pub async fn owner_emails(pool: &PgPool, tenant: Uuid) -> sqlx::Result<Vec<String>> {
    sqlx::query_scalar(
        "SELECT u.email FROM memberships m JOIN users u ON u.id = m.user_id
         WHERE m.tenant_id = $1 AND m.role = 'owner' ORDER BY m.created_at",
    )
    .bind(tenant)
    .fetch_all(pool)
    .await
}
