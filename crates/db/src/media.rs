//! Mis fotos: la biblioteca de fotos de cada dueño.

use domain::MediaItem;
use sqlx::PgPool;
use uuid::Uuid;

/// Cuántas fotos puede tener un dueño.
pub const MAX_PER_OWNER: i64 = domain::PHOTOS_MAX_PER_OWNER as i64;

type Row = (Uuid, String, i32, i32, i64, i64, String);

fn from_row((id, name, w, h, b, wb, alt): Row) -> MediaItem {
    MediaItem {
        alt,
        ..item(id, name, w, h, b, wb)
    }
}

/// Las fotos del dueño, de la más nueva a la más vieja.
pub async fn list(pool: &PgPool, owner_id: Uuid) -> sqlx::Result<Vec<MediaItem>> {
    let rows = sqlx::query_as::<_, Row>(
        "SELECT id, name, width, height, bytes, web_bytes, alt FROM media
         WHERE owner_id = $1 ORDER BY created_at DESC, id",
    )
    .bind(owner_id)
    .fetch_all(pool)
    .await?;
    Ok(rows.into_iter().map(from_row).collect())
}

pub async fn count(pool: &PgPool, owner_id: Uuid) -> sqlx::Result<i64> {
    sqlx::query_scalar("SELECT count(*) FROM media WHERE owner_id = $1")
        .bind(owner_id)
        .fetch_one(pool)
        .await
}

/// Cuántas fotos tiene el dueño y cuánto ocupan.
pub async fn quota(pool: &PgPool, owner_id: Uuid) -> sqlx::Result<domain::PhotoQuota> {
    let (photos, bytes): (i64, i64) = sqlx::query_as(
        "SELECT count(*), coalesce(sum(stored_bytes), 0)::bigint FROM media WHERE owner_id = $1",
    )
    .bind(owner_id)
    .fetch_one(pool)
    .await?;
    Ok(domain::PhotoQuota {
        photos: photos.max(0) as u64,
        bytes: bytes.max(0) as u64,
    })
}

/// La foto del dueño con este archivo original (por su sha256), si ya la
/// subió antes.
pub async fn find_by_hash(
    pool: &PgPool,
    owner_id: Uuid,
    sha256: &str,
) -> sqlx::Result<Option<MediaItem>> {
    let row = sqlx::query_as::<_, Row>(
        "SELECT id, name, width, height, bytes, web_bytes, alt FROM media
         WHERE owner_id = $1 AND sha256 = $2",
    )
    .bind(owner_id)
    .bind(sha256)
    .fetch_optional(pool)
    .await?;
    Ok(row.map(from_row))
}

/// La huella del archivo original (sha256 en hexadecimal), para reconocer
/// una foto que ya se subió.
pub fn fingerprint(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// Una foto recién guardada en el almacén, para anotarla.
pub struct NewMedia<'a> {
    pub id: Uuid,
    pub owner_id: Uuid,
    pub name: &'a str,
    pub width: u32,
    pub height: u32,
    /// Lo que pesaba el archivo original.
    pub bytes: usize,
    /// Lo que baja un visitante (960 px en AVIF).
    pub web_bytes: usize,
    /// Lo que ocupan todos sus tamaños en el almacén.
    pub stored_bytes: usize,
    pub sha256: &'a str,
}

/// Anota una foto ya guardada en el almacén.
pub async fn insert(pool: &PgPool, m: &NewMedia<'_>) -> sqlx::Result<MediaItem> {
    let (w, h) = (m.width as i32, m.height as i32);
    sqlx::query(
        "INSERT INTO media (id, owner_id, name, width, height, bytes, web_bytes, stored_bytes, sha256)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
    )
    .bind(m.id)
    .bind(m.owner_id)
    .bind(m.name)
    .bind(w)
    .bind(h)
    .bind(m.bytes as i64)
    .bind(m.web_bytes as i64)
    .bind(m.stored_bytes as i64)
    .bind(m.sha256)
    .execute(pool)
    .await?;
    Ok(item(
        m.id,
        m.name.to_string(),
        w,
        h,
        m.bytes as i64,
        m.web_bytes as i64,
    ))
}

/// Cambia la descripción de una foto del dueño. Devuelve si existía.
pub async fn set_alt(pool: &PgPool, owner_id: Uuid, id: Uuid, alt: &str) -> sqlx::Result<bool> {
    let r = sqlx::query("UPDATE media SET alt = $3 WHERE id = $1 AND owner_id = $2")
        .bind(id)
        .bind(owner_id)
        .bind(alt)
        .execute(pool)
        .await?;
    Ok(r.rows_affected() > 0)
}

/// Dónde deberían estar los archivos de cada foto: id → carpeta.
pub async fn all_folders(pool: &PgPool) -> sqlx::Result<std::collections::HashMap<String, String>> {
    let rows: Vec<(Uuid, Uuid, bool)> =
        sqlx::query_as("SELECT id, owner_id, in_user_folder FROM media")
            .fetch_all(pool)
            .await?;
    Ok(rows
        .into_iter()
        .map(|(id, owner, moved)| (id.to_string(), folder(owner, id, moved)))
        .collect())
}

/// Borra la foto si es del dueño. Devuelve si existía.
/// Borra la foto de la lista del dueño. Devuelve dónde estaban sus archivos
/// para borrarlos, o `None` si no era suya o ya no estaba.
pub async fn delete(pool: &PgPool, owner_id: Uuid, id: Uuid) -> sqlx::Result<Option<String>> {
    let in_user_folder: Option<bool> = sqlx::query_scalar(
        "DELETE FROM media WHERE id = $1 AND owner_id = $2 RETURNING in_user_folder",
    )
    .bind(id)
    .bind(owner_id)
    .fetch_optional(pool)
    .await?;
    Ok(in_user_folder.map(|moved| folder(owner_id, id, moved)))
}

/// La carpeta de la foto en el almacén: la del usuario, o la vieja (solo
/// el id) si todavía no se movió.
fn folder(owner_id: Uuid, id: Uuid, in_user_folder: bool) -> String {
    if in_user_folder {
        media_folder(owner_id, id)
    } else {
        id.to_string()
    }
}

/// `usuarios/<dueño>/fotos/<foto>`; igual que `media::photo_folder`.
pub fn media_folder(owner_id: Uuid, id: Uuid) -> String {
    format!("usuarios/{owner_id}/fotos/{id}")
}

/// Dónde están los archivos de la foto `id` y dónde podrían estar si se
/// está moviendo justo ahora: (carpeta actual, la otra). `None` si la foto
/// no existe.
pub async fn locate(pool: &PgPool, id: Uuid) -> sqlx::Result<Option<(String, String)>> {
    let row: Option<(Uuid, bool)> =
        sqlx::query_as("SELECT owner_id, in_user_folder FROM media WHERE id = $1")
            .bind(id)
            .fetch_optional(pool)
            .await?;
    Ok(row.map(|(owner, moved)| (folder(owner, id, moved), folder(owner, id, !moved))))
}

/// Fotos que todavía están en la carpeta vieja: (foto, dueño).
pub async fn not_in_user_folder(pool: &PgPool) -> sqlx::Result<Vec<(Uuid, Uuid)>> {
    sqlx::query_as("SELECT id, owner_id FROM media WHERE NOT in_user_folder ORDER BY created_at")
        .fetch_all(pool)
        .await
}

/// Anota que la foto ya está en la carpeta del usuario y cuánto ocupa
/// (`None` si no se pudo medir).
pub async fn mark_in_user_folder(
    pool: &PgPool,
    id: Uuid,
    stored_bytes: Option<u64>,
) -> sqlx::Result<()> {
    sqlx::query(
        "UPDATE media SET in_user_folder = true, stored_bytes = coalesce($2, stored_bytes) WHERE id = $1",
    )
        .bind(id)
        .bind(stored_bytes.map(|b| b as i64))
        .execute(pool)
        .await?;
    Ok(())
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
        alt: String::new(),
        duplicate: false,
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
/// las páginas de sus tiendas cuyo borrador o versión publicada la nombra, y
/// los productos que la tienen entre sus fotos (página `producto:<nombre>`).
pub async fn usage(pool: &PgPool, owner_id: Uuid) -> sqlx::Result<Vec<(Uuid, String, String)>> {
    sqlx::query_as(
        "SELECT DISTINCT m.id, t.name, p.slug
         FROM media m
         JOIN memberships ms ON ms.user_id = m.owner_id
         JOIN tenants t ON t.id = ms.tenant_id
         JOIN pages p ON p.tenant_id = t.id
         JOIN page_versions v ON v.id = p.current_version_id OR v.id = p.published_version_id
         WHERE m.owner_id = $1 AND strpos(v.doc::text, m.id::text) > 0
         UNION
         SELECT DISTINCT m.id, t.name, 'producto:' || pr.name
         FROM media m
         JOIN memberships ms ON ms.user_id = m.owner_id
         JOIN tenants t ON t.id = ms.tenant_id
         JOIN products pr ON pr.tenant_id = t.id
         WHERE m.owner_id = $1 AND strpos(array_to_string(pr.photos, ' '), m.id::text) > 0
         UNION
         SELECT DISTINCT m.id, t.name, 'logo:'
         FROM media m
         JOIN tenants t ON t.logo_id = m.id
         WHERE m.owner_id = $1
         ORDER BY 2, 3",
    )
    .bind(owner_id)
    .fetch_all(pool)
    .await
}
