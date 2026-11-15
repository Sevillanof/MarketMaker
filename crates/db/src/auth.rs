//! Cuentas de dueño: códigos para entrar, sesiones y roles por tienda.
//!
//! Nada sensible se guarda en claro: de los códigos y de los tokens de sesión
//! la base guarda solo el hash SHA-256.

use rand::{Rng, RngCore};
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use uuid::Uuid;

/// Cuánto dura un código.
pub const CODE_MINUTES: i32 = 10;
/// Intentos por código antes de tener que pedir otro.
pub const CODE_ATTEMPTS: i32 = 5;
/// Códigos por email y por hora.
pub const CODES_PER_HOUR: i64 = 5;
/// Cuánto dura una sesión sin usarse.
pub const SESSION_DAYS: i32 = 30;

fn code_hash(email: &str, code: &str) -> Vec<u8> {
    Sha256::digest(format!("{email}:{code}")).to_vec()
}

fn token_hash(token: &str) -> Vec<u8> {
    Sha256::digest(token.as_bytes()).to_vec()
}

/// Seis dígitos al azar, con ceros a la izquierda.
pub fn new_code() -> String {
    format!("{:06}", rand::rng().random_range(0..1_000_000u32))
}

/// 32 bytes al azar en hexadecimal: lo que lleva la cookie.
pub fn new_token() -> String {
    let mut bytes = [0u8; 32];
    rand::rng().fill_bytes(&mut bytes);
    hex::encode(bytes)
}

/// Se pidieron demasiados códigos para este email en la última hora.
#[derive(Debug, PartialEq)]
pub struct TooMany;

/// Crea un código nuevo para `email` (ya normalizado) y lo devuelve en claro
/// para mandarlo por email. Los códigos anteriores sin usar dejan de servir.
pub async fn create_login_code(
    pool: &PgPool,
    email: &str,
) -> sqlx::Result<Result<String, TooMany>> {
    let recent: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM login_codes
         WHERE email = $1 AND created_at > now() - interval '1 hour'",
    )
    .bind(email)
    .fetch_one(pool)
    .await?;
    if recent >= CODES_PER_HOUR {
        return Ok(Err(TooMany));
    }
    let code = new_code();
    let mut tx = pool.begin().await?;
    sqlx::query("UPDATE login_codes SET used_at = now() WHERE email = $1 AND used_at IS NULL")
        .bind(email)
        .execute(&mut *tx)
        .await?;
    sqlx::query(
        "INSERT INTO login_codes (email, code_hash, expires_at)
         VALUES ($1, $2, now() + make_interval(mins => $3))",
    )
    .bind(email)
    .bind(code_hash(email, &code))
    .bind(CODE_MINUTES)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(Ok(code))
}

/// Resultado de revisar un código.
#[derive(Debug, PartialEq)]
pub enum CodeResult {
    /// Era el código: la cuenta existe (se crea la primera vez).
    Ok { user_id: Uuid },
    /// No era. Quedan `left` intentos.
    Wrong { left: u32 },
    /// No hay un código vigente: venció, se usó o se gastaron los intentos.
    Expired,
}

/// Revisa `code` contra el último código vigente de `email`. Si es correcto lo
/// marca como usado y devuelve la cuenta, creándola si es la primera vez.
pub async fn check_login_code(pool: &PgPool, email: &str, code: &str) -> sqlx::Result<CodeResult> {
    let mut tx = pool.begin().await?;
    let row: Option<(Uuid, Vec<u8>, i32)> = sqlx::query_as(
        "SELECT id, code_hash, attempts FROM login_codes
         WHERE email = $1 AND used_at IS NULL AND expires_at > now()
         ORDER BY created_at DESC LIMIT 1
         FOR UPDATE",
    )
    .bind(email)
    .fetch_optional(&mut *tx)
    .await?;
    let Some((id, hash, attempts)) = row else {
        return Ok(CodeResult::Expired);
    };
    if attempts >= CODE_ATTEMPTS {
        return Ok(CodeResult::Expired);
    }
    if hash != code_hash(email, code) {
        let attempts = attempts + 1;
        sqlx::query("UPDATE login_codes SET attempts = $2 WHERE id = $1")
            .bind(id)
            .bind(attempts)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        let left = u32::try_from(CODE_ATTEMPTS - attempts).unwrap_or(0);
        return Ok(if left == 0 {
            CodeResult::Expired
        } else {
            CodeResult::Wrong { left }
        });
    }
    sqlx::query("UPDATE login_codes SET used_at = now() WHERE id = $1")
        .bind(id)
        .execute(&mut *tx)
        .await?;
    let user_id: Uuid = sqlx::query_scalar(
        "INSERT INTO users (email) VALUES ($1)
         ON CONFLICT (email) DO UPDATE SET email = EXCLUDED.email
         RETURNING id",
    )
    .bind(email)
    .fetch_one(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(CodeResult::Ok { user_id })
}

/// Abre una sesión y devuelve el token para la cookie.
pub async fn create_session(pool: &PgPool, user_id: Uuid) -> sqlx::Result<String> {
    let token = new_token();
    sqlx::query(
        "INSERT INTO sessions (token_hash, user_id, expires_at)
         VALUES ($1, $2, now() + make_interval(days => $3))",
    )
    .bind(token_hash(&token))
    .bind(user_id)
    .bind(SESSION_DAYS)
    .execute(pool)
    .await?;
    Ok(token)
}

/// La persona de una sesión vigente.
#[derive(Debug, Clone, PartialEq, sqlx::FromRow)]
pub struct SessionUser {
    pub id: Uuid,
    pub email: String,
}

/// La cuenta de la sesión `token`, si sigue vigente. Usarla la extiende
/// otros [`SESSION_DAYS`] días.
pub async fn session_user(pool: &PgPool, token: &str) -> sqlx::Result<Option<SessionUser>> {
    sqlx::query_as::<_, SessionUser>(
        "UPDATE sessions s SET expires_at = now() + make_interval(days => $2)
         FROM users u
         WHERE s.token_hash = $1 AND s.expires_at > now() AND u.id = s.user_id
         RETURNING u.id, u.email",
    )
    .bind(token_hash(token))
    .bind(SESSION_DAYS)
    .fetch_optional(pool)
    .await
}

/// Cierra la sesión `token`.
pub async fn delete_session(pool: &PgPool, token: &str) -> sqlx::Result<()> {
    sqlx::query("DELETE FROM sessions WHERE token_hash = $1")
        .bind(token_hash(token))
        .execute(pool)
        .await?;
    Ok(())
}

/// El rol de la persona en la tienda (`"owner"`), si tiene alguno.
pub async fn role_in(
    pool: &PgPool,
    tenant_id: Uuid,
    user_id: Uuid,
) -> sqlx::Result<Option<String>> {
    sqlx::query_scalar("SELECT role FROM memberships WHERE tenant_id = $1 AND user_id = $2")
        .bind(tenant_id)
        .bind(user_id)
        .fetch_optional(pool)
        .await
}

/// Si la tienda todavía no tiene dueño, la persona pasa a serlo.
pub async fn claim_if_ownerless(pool: &PgPool, tenant_id: Uuid, user_id: Uuid) -> sqlx::Result<()> {
    sqlx::query(
        "INSERT INTO memberships (tenant_id, user_id, role)
         SELECT $1, $2, 'owner'
         WHERE NOT EXISTS (SELECT 1 FROM memberships WHERE tenant_id = $1)
         ON CONFLICT DO NOTHING",
    )
    .bind(tenant_id)
    .bind(user_id)
    .execute(pool)
    .await?;
    Ok(())
}
