//! Cuentas de compradores (una por tienda): enlaces para entrar, sesiones,
//! datos y pedidos. Ver `migrations/*_buyers.sql`.
//!
//! Nada sensible se guarda en claro: de las llaves de los enlaces y de los
//! tokens de sesión la base guarda solo el hash SHA-256. Toda consulta lleva
//! el `tenant`: una cuenta de una tienda nunca sirve en otra.

use rand::RngCore;
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use uuid::Uuid;

/// Cuánto dura un enlace para entrar.
pub const LINK_MINUTES: i32 = 15;
/// Enlaces por email y tienda, por hora.
pub const LINKS_PER_HOUR: i64 = 5;
/// Enlaces por email y tienda, por día.
pub const LINKS_PER_DAY: i64 = 12;
/// Cuánto dura una sesión sin usarse.
pub const SESSION_DAYS: i32 = 30;
/// Cuánto dura una sesión como mucho, aunque se use todos los días.
pub const SESSION_MAX_DAYS: i32 = 90;
/// Tamaño máximo de la foto de perfil (texto data URL).
pub const AVATAR_MAX_BYTES: usize = 80 * 1024;

fn hash(value: &str) -> Vec<u8> {
    Sha256::digest(value.as_bytes()).to_vec()
}

/// 32 bytes al azar en hexadecimal: la llave del enlace y el token de la cookie.
pub fn new_secret() -> String {
    let mut bytes = [0u8; 32];
    rand::rng().fill_bytes(&mut bytes);
    hex::encode(bytes)
}

/// Se pidieron demasiados enlaces para este email.
#[derive(Debug, PartialEq)]
pub struct TooMany;

/// Crea un enlace de ingreso para `email` (ya normalizado) en `tenant` y
/// devuelve la llave en claro para mandarla por email. Los enlaces anteriores
/// sin usar dejan de servir.
pub async fn create_link(
    pool: &PgPool,
    tenant: Uuid,
    email: &str,
) -> sqlx::Result<Result<String, TooMany>> {
    create_link_for(pool, tenant, email, None).await
}

/// Como [`create_link`], pero anota un producto para dejarlo en favoritos
/// cuando la persona entre con este enlace.
pub async fn create_link_for(
    pool: &PgPool,
    tenant: Uuid,
    email: &str,
    favorite: Option<Uuid>,
) -> sqlx::Result<Result<String, TooMany>> {
    let (hour, day): (i64, i64) = sqlx::query_as(
        "SELECT count(*) FILTER (WHERE created_at > now() - interval '1 hour'), count(*)
         FROM buyer_links
         WHERE tenant_id = $1 AND email = $2 AND created_at > now() - interval '1 day'",
    )
    .bind(tenant)
    .bind(email)
    .fetch_one(pool)
    .await?;
    if hour >= LINKS_PER_HOUR || day >= LINKS_PER_DAY {
        return Ok(Err(TooMany));
    }
    let key = new_secret();
    let mut tx = pool.begin().await?;
    sqlx::query(
        "UPDATE buyer_links SET used_at = now()
         WHERE tenant_id = $1 AND email = $2 AND used_at IS NULL",
    )
    .bind(tenant)
    .bind(email)
    .execute(&mut *tx)
    .await?;
    sqlx::query(
        "INSERT INTO buyer_links (tenant_id, email, key_hash, expires_at, pending_favorite)
         VALUES ($1, $2, $3, now() + make_interval(mins => $4), $5)",
    )
    .bind(tenant)
    .bind(email)
    .bind(hash(&key))
    .bind(LINK_MINUTES)
    .bind(favorite)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(Ok(key))
}

/// Canjea la llave de un enlace: si sirve (de esta tienda, vigente, sin usar)
/// la marca como usada y devuelve la cuenta del comprador, creándola la
/// primera vez con el nombre y el teléfono de su último pedido.
pub async fn redeem_link(pool: &PgPool, tenant: Uuid, key: &str) -> sqlx::Result<Option<Uuid>> {
    let mut tx = pool.begin().await?;
    let email: Option<(String, Option<Uuid>)> = sqlx::query_as(
        "UPDATE buyer_links SET used_at = now()
         WHERE tenant_id = $1 AND key_hash = $2 AND used_at IS NULL AND expires_at > now()
         RETURNING email, pending_favorite",
    )
    .bind(tenant)
    .bind(hash(key))
    .fetch_optional(&mut *tx)
    .await?;
    let Some((email, favorite)) = email else {
        return Ok(None);
    };
    // Los datos de su último pedido con este email, para no pedírselos de nuevo.
    let last: Option<(String, String, String, String, String)> = sqlx::query_as(
        "SELECT customer_name, phone, address, postal_code, city FROM orders
         WHERE tenant_id = $1 AND lower(email) = $2 AND number IS NOT NULL
         ORDER BY created_at DESC LIMIT 1",
    )
    .bind(tenant)
    .bind(&email)
    .fetch_optional(&mut *tx)
    .await?;
    let (name, phone, address, postal, city) = last.unwrap_or_default();
    let (id,): (Uuid,) = sqlx::query_as(
        "INSERT INTO buyers (tenant_id, email, name, phone, address, postal_code, city)
         VALUES ($1, $2, $3, $4, $5, $6, $7)
         ON CONFLICT (tenant_id, email) DO UPDATE SET email = EXCLUDED.email
         RETURNING id",
    )
    .bind(tenant)
    .bind(&email)
    .bind(name)
    .bind(phone)
    .bind(address)
    .bind(postal)
    .bind(city)
    .fetch_one(&mut *tx)
    .await?;
    if let Some(product) = favorite {
        add_favorite(&mut *tx, tenant, id, product).await?;
    }
    tx.commit().await?;
    Ok(Some(id))
}

/// Cuántos favoritos puede tener un comprador, para que nadie llene la base.
pub const FAVORITES_MAX: i64 = 200;

/// Guarda `product` en los favoritos de `buyer`. Solo si el producto es de la
/// misma tienda que el comprador (y no se pasó del máximo). Devuelve si quedó
/// guardado.
async fn add_favorite(
    conn: &mut sqlx::PgConnection,
    tenant: Uuid,
    buyer: Uuid,
    product: Uuid,
) -> sqlx::Result<bool> {
    let done = sqlx::query(
        "INSERT INTO buyer_favorites (buyer_id, product_id)
         SELECT $2, p.id FROM products p
         WHERE p.id = $3 AND p.tenant_id = $1
           AND (SELECT count(*) FROM buyer_favorites WHERE buyer_id = $2) < $4
         ON CONFLICT DO NOTHING",
    )
    .bind(tenant)
    .bind(buyer)
    .bind(product)
    .bind(FAVORITES_MAX)
    .execute(conn)
    .await?;
    Ok(done.rows_affected() > 0)
}

/// Marca (`on`) o desmarca un favorito. Con `on`, devuelve si quedó
/// guardado (`false` si el producto no es de esta tienda o se llegó al
/// máximo); al quitar, `true`.
pub async fn set_favorite(
    pool: &PgPool,
    tenant: Uuid,
    buyer: Uuid,
    product: Uuid,
    on: bool,
) -> sqlx::Result<bool> {
    if on {
        let mut conn = pool.acquire().await?;
        let saved = add_favorite(&mut conn, tenant, buyer, product).await?;
        if saved {
            return Ok(true);
        }
        // Ya estaba: también es "guardado".
        let (exists,): (bool,) = sqlx::query_as(
            "SELECT EXISTS (SELECT 1 FROM buyer_favorites f JOIN products p ON p.id = f.product_id
             WHERE f.buyer_id = $1 AND f.product_id = $2 AND p.tenant_id = $3)",
        )
        .bind(buyer)
        .bind(product)
        .bind(tenant)
        .fetch_one(&mut *conn)
        .await?;
        Ok(exists)
    } else {
        sqlx::query("DELETE FROM buyer_favorites WHERE buyer_id = $1 AND product_id = $2")
            .bind(buyer)
            .bind(product)
            .execute(pool)
            .await?;
        Ok(true)
    }
}

/// Los productos favoritos de `buyer` que se pueden ver en la tienda.
pub async fn favorite_ids(pool: &PgPool, tenant: Uuid, buyer: Uuid) -> sqlx::Result<Vec<Uuid>> {
    sqlx::query_scalar(
        "SELECT f.product_id FROM buyer_favorites f JOIN products p ON p.id = f.product_id
         WHERE f.buyer_id = $1 AND p.tenant_id = $2 AND p.visible AND NOT p.example
         ORDER BY f.created_at DESC",
    )
    .bind(buyer)
    .bind(tenant)
    .fetch_all(pool)
    .await
}

/// Abre una sesión y devuelve el token para la cookie.
pub async fn create_session(pool: &PgPool, buyer: Uuid) -> sqlx::Result<String> {
    let token = new_secret();
    sqlx::query(
        "INSERT INTO buyer_sessions (token_hash, buyer_id, expires_at)
         VALUES ($1, $2, now() + make_interval(days => $3))",
    )
    .bind(hash(&token))
    .bind(buyer)
    .bind(SESSION_DAYS)
    .execute(pool)
    .await?;
    Ok(token)
}

/// Un comprador, con lo que se guarda de él.
#[derive(Debug, Clone, PartialEq, sqlx::FromRow)]
pub struct Buyer {
    pub id: Uuid,
    pub email: String,
    pub name: String,
    pub phone: String,
    pub address: String,
    pub postal_code: String,
    pub city: String,
    pub avatar: Option<String>,
}

/// El comprador de la sesión `token` en `tenant`, si sigue vigente. Usarla la
/// extiende otros [`SESSION_DAYS`] días.
pub async fn session_buyer(
    pool: &PgPool,
    tenant: Uuid,
    token: &str,
) -> sqlx::Result<Option<Buyer>> {
    sqlx::query_as::<_, Buyer>(
        "UPDATE buyer_sessions s SET expires_at = now() + make_interval(days => $3)
         FROM buyers b
         WHERE s.token_hash = $1 AND s.expires_at > now() AND b.id = s.buyer_id
           AND b.tenant_id = $2
           AND s.created_at > now() - make_interval(days => $4)
         RETURNING b.id, b.email, b.name, b.phone, b.address, b.postal_code, b.city, b.avatar",
    )
    .bind(hash(token))
    .bind(tenant)
    .bind(SESSION_DAYS)
    .bind(SESSION_MAX_DAYS)
    .fetch_optional(pool)
    .await
}

/// Cierra la sesión `token`.
pub async fn delete_session(pool: &PgPool, token: &str) -> sqlx::Result<()> {
    sqlx::query("DELETE FROM buyer_sessions WHERE token_hash = $1")
        .bind(hash(token))
        .execute(pool)
        .await?;
    Ok(())
}

/// Guarda nombre, teléfono y dirección.
pub async fn save_profile(
    pool: &PgPool,
    buyer: Uuid,
    name: &str,
    phone: &str,
    address: &str,
    postal_code: &str,
    city: &str,
) -> sqlx::Result<()> {
    sqlx::query(
        "UPDATE buyers SET name = $2, phone = $3, address = $4, postal_code = $5, city = $6
         WHERE id = $1",
    )
    .bind(buyer)
    .bind(name)
    .bind(phone)
    .bind(address)
    .bind(postal_code)
    .bind(city)
    .execute(pool)
    .await?;
    Ok(())
}

/// Pone (o quita, con `None`) la foto de perfil.
pub async fn set_avatar(pool: &PgPool, buyer: Uuid, avatar: Option<&str>) -> sqlx::Result<()> {
    sqlx::query("UPDATE buyers SET avatar = $2 WHERE id = $1")
        .bind(buyer)
        .bind(avatar)
        .execute(pool)
        .await?;
    Ok(())
}

/// Borra la cuenta del comprador (sus sesiones también). Los pedidos quedan
/// en la tienda, porque son parte de sus ventas.
pub async fn delete_account(pool: &PgPool, buyer: Uuid) -> sqlx::Result<()> {
    sqlx::query("DELETE FROM buyers WHERE id = $1")
        .bind(buyer)
        .execute(pool)
        .await?;
    Ok(())
}

/// Limpieza: enlaces y sesiones vencidos.
pub async fn purge_expired(pool: &PgPool) -> sqlx::Result<()> {
    sqlx::query("DELETE FROM buyer_links WHERE expires_at < now() - interval '2 days'")
        .execute(pool)
        .await?;
    sqlx::query("DELETE FROM buyer_sessions WHERE expires_at < now()")
        .execute(pool)
        .await?;
    Ok(())
}
