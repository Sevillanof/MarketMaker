//! Mis fotos: la biblioteca de fotos de cada dueño.

use domain::MediaItem;
use sqlx::PgPool;
use uuid::Uuid;

/// Cuántas fotos puede tener un dueño.
pub const MAX_PER_OWNER: i64 = 500;

/// Las fotos del dueño, de la más nueva a la más vieja.
pub async fn list(pool: &PgPool, owner_id: Uuid) -> sqlx::Result<Vec<MediaItem>> {
    let rows = sqlx::query_as::<_, (Uuid, String, i32, i32, i64, i64)>(
        "SELECT id, name, width, height, bytes, web_bytes FROM media
         WHERE owner_id = $1 ORDER BY created_at DESC, id",
    )
    .bind(owner_id)
    .fetch_all(pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(|(id, name, w, h, b, wb)| item(id, name, w, h, b, wb))
        .collect())
}

pub async fn count(pool: &PgPool, owner_id: Uuid) -> sqlx::Result<i64> {
    sqlx::query_scalar("SELECT count(*) FROM media WHERE owner_id = $1")
        .bind(owner_id)
        .fetch_one(pool)
        .await
}

/// Anota una foto ya guardada en el almacén.
pub async fn insert(
    pool: &PgPool,
    id: Uuid,
    owner_id: Uuid,
    name: &str,
    width: u32,
    height: u32,
    bytes: usize,
    web_bytes: usize,
) -> sqlx::Result<MediaItem> {
    let (w, h) = (width as i32, height as i32);
    sqlx::query(
        "INSERT INTO media (id, owner_id, name, width, height, bytes, web_bytes) VALUES ($1, $2, $3, $4, $5, $6, $7)",
    )
    .bind(id)
    .bind(owner_id)
    .bind(name)
    .bind(w)
    .bind(h)
    .bind(bytes as i64)
    .bind(web_bytes as i64)
    .execute(pool)
    .await?;
    Ok(item(
        id,
        name.to_string(),
        w,
        h,
        bytes as i64,
        web_bytes as i64,
    ))
}

/// Borra la foto si es del dueño. Devuelve si existía.
pub async fn delete(pool: &PgPool, owner_id: Uuid, id: Uuid) -> sqlx::Result<bool> {
    let r = sqlx::query("DELETE FROM media WHERE id = $1 AND owner_id = $2")
        .bind(id)
        .bind(owner_id)
        .execute(pool)
        .await?;
    Ok(r.rows_affected() > 0)
}

fn item(id: Uuid, name: String, width: i32, height: i32, bytes: i64, web_bytes: i64) -> MediaItem {
    MediaItem {
        id: id.to_string(),
        url: domain::media_url(&id.to_string()),
        name,
        width: width.max(0) as u32,
        height: height.max(0) as u32,
        original_bytes: bytes.max(0) as u64,
        web_bytes: web_bytes.max(0) as u64,
    }
}

/// Cambia el nombre de la foto si es del dueño. Devuelve si existía.
pub async fn rename(pool: &PgPool, owner_id: Uuid, id: Uuid, name: &str) -> sqlx::Result<bool> {
    let r = sqlx::query("UPDATE media SET name = $3 WHERE id = $1 AND owner_id = $2")
        .bind(id)
        .bind(owner_id)
        .bind(name)
        .execute(pool)
        .await?;
    Ok(r.rows_affected() > 0)
}

/// Dónde usa el dueño cada una de sus fotos: (foto, tienda, página), para
/// las páginas de sus tiendas cuyo borrador o versión publicada la nombra.
pub async fn usage(pool: &PgPool, owner_id: Uuid) -> sqlx::Result<Vec<(Uuid, String, String)>> {
    sqlx::query_as(
        "SELECT DISTINCT m.id, t.name, p.slug
         FROM media m
         JOIN memberships ms ON ms.user_id = m.owner_id
         JOIN tenants t ON t.id = ms.tenant_id
         JOIN pages p ON p.tenant_id = t.id
         JOIN page_versions v ON v.id = p.current_version_id OR v.id = p.published_version_id
         WHERE m.owner_id = $1 AND strpos(v.doc::text, m.id::text) > 0
         ORDER BY t.name, p.slug",
    )
    .bind(owner_id)
    .fetch_all(pool)
    .await
}
