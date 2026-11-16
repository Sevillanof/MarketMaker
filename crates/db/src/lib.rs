//! Consultas SQLx. Solo servidor: se usa desde `web` (feature `ssr`) y `server`.
//!
//! Se usa `sqlx::query_as` sin las macros verificadas en compilación, para
//! que compilar no dependa de tener la base levantada.

pub mod andreani;
pub mod auth;
pub mod media;
pub mod mp;
pub mod orders;
pub mod products;

use domain::{PageDoc, Product, SaveOutcome};
use sqlx::{FromRow, PgExecutor, PgPool};
use uuid::Uuid;

pub use sqlx::PgPool as Pool;

#[derive(Debug, Clone, FromRow)]
pub struct Tenant {
    pub id: Uuid,
    pub slug: String,
    pub name: String,
}

/// Una versión de una página (la del borrador o la publicada). `version` es 0 si la página todavía no
/// tiene ninguna versión guardada.
#[derive(Debug, Clone, FromRow)]
pub struct CurrentPage {
    pub page_id: Uuid,
    pub version: i32,
    pub doc: Option<serde_json::Value>,
    /// Número de la versión publicada (0 si nunca se publicó).
    pub published: i32,
}

pub async fn get_tenant_by_slug(pool: &PgPool, slug: &str) -> sqlx::Result<Option<Tenant>> {
    sqlx::query_as::<_, Tenant>("SELECT id, slug, name FROM tenants WHERE slug = $1")
        .bind(slug)
        .fetch_optional(pool)
        .await
}

pub async fn get_current_page(
    pool: &PgPool,
    tenant_id: Uuid,
    slug: &str,
) -> sqlx::Result<Option<CurrentPage>> {
    sqlx::query_as::<_, CurrentPage>(
        "SELECT p.id AS page_id, COALESCE(v.number, 0) AS version, v.doc,
                COALESCE(pv.number, 0) AS published
         FROM pages p
         LEFT JOIN page_versions v ON v.id = p.current_version_id
         LEFT JOIN page_versions pv ON pv.id = p.published_version_id
         WHERE p.tenant_id = $1 AND p.slug = $2",
    )
    .bind(tenant_id)
    .bind(slug)
    .fetch_optional(pool)
    .await
}

/// La versión publicada de una página: lo que ven los visitantes. `doc` es
/// `None` si nunca se publicó.
pub async fn get_published_page(
    pool: &PgPool,
    tenant_id: Uuid,
    slug: &str,
) -> sqlx::Result<Option<CurrentPage>> {
    sqlx::query_as::<_, CurrentPage>(
        "SELECT p.id AS page_id, COALESCE(pv.number, 0) AS version, pv.doc,
                COALESCE(pv.number, 0) AS published
         FROM pages p
         LEFT JOIN page_versions pv ON pv.id = p.published_version_id
         WHERE p.tenant_id = $1 AND p.slug = $2",
    )
    .bind(tenant_id)
    .bind(slug)
    .fetch_optional(pool)
    .await
}

/// Publica la versión `number` de la página: desde ahora es la que ven los
/// visitantes. Devuelve `false` si esa versión no existe.
pub async fn publish_page(
    pool: &PgPool,
    tenant_id: Uuid,
    slug: &str,
    number: u32,
) -> sqlx::Result<bool> {
    let number = i32::try_from(number).unwrap_or(i32::MAX);
    let done = sqlx::query(
        "UPDATE pages p SET published_version_id = v.id
         FROM page_versions v
         WHERE p.tenant_id = $1 AND p.slug = $2 AND v.page_id = p.id AND v.number = $3",
    )
    .bind(tenant_id)
    .bind(slug)
    .bind(number)
    .execute(pool)
    .await?;
    Ok(done.rows_affected() == 1)
}

/// Productos que se ven en la tienda (los visibles), el más nuevo primero.
pub async fn list_products(pool: &PgPool, tenant_id: Uuid) -> sqlx::Result<Vec<Product>> {
    products::list_visible(pool, tenant_id).await
}

/// Inserta la versión `number` de la página y la marca como vigente.
pub async fn save_page_version<'e, E>(
    exec: E,
    tenant_id: Uuid,
    page_id: Uuid,
    number: i32,
    doc: &serde_json::Value,
    created_by: Option<Uuid>,
) -> sqlx::Result<()>
where
    E: PgExecutor<'e>,
{
    sqlx::query(
        "WITH v AS (
           INSERT INTO page_versions (page_id, tenant_id, number, doc, created_by)
           VALUES ($1, $2, $3, $4, $5)
           RETURNING id
         )
         UPDATE pages SET current_version_id = (SELECT id FROM v) WHERE id = $1",
    )
    .bind(page_id)
    .bind(tenant_id)
    .bind(number)
    .bind(doc)
    .bind(created_by)
    .execute(exec)
    .await?;
    Ok(())
}

/// Cuántas versiones se conservan por página.
pub const KEEP_VERSIONS: i32 = 20;

/// Guarda `doc` como nueva versión de la página, todo dentro de una transacción:
/// bloquea la página, compara `base_version` con la vigente, valida, inserta la
/// versión `number + 1`, la marca como vigente (el borrador) y borra las que
/// quedan fuera de las últimas [`KEEP_VERSIONS`], salvo la publicada.
/// Guardar no publica: eso lo hace [`publish_page`].
///
/// Devuelve `RowNotFound` si la página no existe.
pub async fn save_page(
    pool: &PgPool,
    tenant_id: Uuid,
    slug: &str,
    doc: &PageDoc,
    base_version: u32,
) -> sqlx::Result<SaveOutcome> {
    save_page_by(pool, tenant_id, slug, doc, base_version, None).await
}

/// Como [`save_page`], anotando quién guardó la versión.
pub async fn save_page_by(
    pool: &PgPool,
    tenant_id: Uuid,
    slug: &str,
    doc: &PageDoc,
    base_version: u32,
    created_by: Option<Uuid>,
) -> sqlx::Result<SaveOutcome> {
    let mut tx = pool.begin().await?;

    let (page_id, current): (Uuid, i32) = sqlx::query_as(
        "SELECT p.id, COALESCE(v.number, 0) AS number
         FROM pages p
         LEFT JOIN page_versions v ON v.id = p.current_version_id
         WHERE p.tenant_id = $1 AND p.slug = $2
         FOR UPDATE OF p",
    )
    .bind(tenant_id)
    .bind(slug)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or(sqlx::Error::RowNotFound)?;

    let current_u32 = u32::try_from(current).unwrap_or(0);
    if current_u32 != base_version {
        return Ok(SaveOutcome::Conflict {
            current: current_u32,
        });
    }
    if let Err(errors) = domain::validate(doc) {
        return Ok(SaveOutcome::Invalid { errors });
    }

    let number = current + 1;
    let value = serde_json::to_value(doc).map_err(|e| sqlx::Error::Encode(Box::new(e)))?;
    save_page_version(&mut *tx, tenant_id, page_id, number, &value, created_by).await?;

    // La versión publicada se conserva aunque sea vieja: la tienda la muestra.
    sqlx::query(
        "DELETE FROM page_versions
         WHERE page_id = $1 AND number <= $2
           AND id IS DISTINCT FROM (SELECT published_version_id FROM pages WHERE id = $1)",
    )
    .bind(page_id)
    .bind(number - KEEP_VERSIONS)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;
    Ok(SaveOutcome::Saved {
        version: u32::try_from(number).unwrap_or(0),
    })
}
