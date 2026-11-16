//! Andreani: la cuenta conectada de cada tienda y lo que el dueño eligió
//! ofrecer (origen, servicios, paquete de la tienda).

use domain::{AndreaniContract, AndreaniSettings};
use sqlx::types::Json;
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

/// La cuenta de Andreani de una tienda, como está guardada.
#[derive(Debug, Clone, FromRow)]
pub struct Account {
    pub credential: String,
    pub access_token: String,
    pub client_type: String,
    pub holder: String,
    pub cuit: String,
    pub email: String,
    pub contracts: Json<Vec<AndreaniContract>>,
    /// `None`: la conectó pero todavía no guardó el origen.
    pub settings: Option<Json<AndreaniSettings>>,
}

/// Lo que devuelve Andreani al entrar con el código.
pub struct Login<'a> {
    pub credential: &'a str,
    pub access_token: &'a str,
    pub client_type: &'a str,
    pub holder: &'a str,
    pub cuit: &'a str,
    pub email: &'a str,
    pub contracts: &'a [AndreaniContract],
}

pub async fn account(pool: &PgPool, tenant: Uuid) -> sqlx::Result<Option<Account>> {
    sqlx::query_as::<_, Account>(
        "SELECT credential, access_token, client_type, holder, cuit, email, contracts, settings
         FROM andreani_accounts WHERE tenant_id = $1",
    )
    .bind(tenant)
    .fetch_optional(pool)
    .await
}

/// Conecta (o vuelve a conectar) la cuenta. Si ya había una, conserva lo que
/// eligió el dueño.
pub async fn connect(pool: &PgPool, tenant: Uuid, l: &Login<'_>) -> sqlx::Result<()> {
    sqlx::query(
        "INSERT INTO andreani_accounts
             (tenant_id, credential, access_token, client_type, holder, cuit, email, contracts)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
         ON CONFLICT (tenant_id) DO UPDATE SET
             credential = EXCLUDED.credential, access_token = EXCLUDED.access_token,
             client_type = EXCLUDED.client_type, holder = EXCLUDED.holder,
             cuit = EXCLUDED.cuit, email = EXCLUDED.email, contracts = EXCLUDED.contracts,
             updated_at = now()",
    )
    .bind(tenant)
    .bind(l.credential)
    .bind(l.access_token)
    .bind(l.client_type)
    .bind(l.holder)
    .bind(l.cuit)
    .bind(l.email)
    .bind(Json(l.contracts))
    .execute(pool)
    .await?;
    Ok(())
}

/// El token nuevo, después de volver a entrar.
pub async fn set_token(pool: &PgPool, tenant: Uuid, token: &str) -> sqlx::Result<()> {
    sqlx::query(
        "UPDATE andreani_accounts SET access_token = $2, updated_at = now() WHERE tenant_id = $1",
    )
    .bind(tenant)
    .bind(token)
    .execute(pool)
    .await?;
    Ok(())
}

/// Guarda lo que eligió el dueño. `false` si la cuenta no está conectada.
pub async fn save_settings(
    pool: &PgPool,
    tenant: Uuid,
    settings: &AndreaniSettings,
) -> sqlx::Result<bool> {
    let done = sqlx::query(
        "UPDATE andreani_accounts SET settings = $2, updated_at = now() WHERE tenant_id = $1",
    )
    .bind(tenant)
    .bind(Json(settings))
    .execute(pool)
    .await?;
    Ok(done.rows_affected() == 1)
}

pub async fn disconnect(pool: &PgPool, tenant: Uuid) -> sqlx::Result<()> {
    sqlx::query("DELETE FROM andreani_accounts WHERE tenant_id = $1")
        .bind(tenant)
        .execute(pool)
        .await?;
    Ok(())
}
