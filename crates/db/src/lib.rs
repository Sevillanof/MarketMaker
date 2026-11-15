//! Consultas SQLx. Solo servidor: se usa desde `web` (feature `ssr`) y `server`.
//!
//! Se usa `sqlx::query_as` sin las macros verificadas en compilación, para
//! que compilar no dependa de tener la base levantada.

pub mod auth;

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

/// La versión vigente de una página. `version` es 0 si la página todavía no
/// tiene ninguna versión guardada.
#[derive(Debug, Clone, FromRow)]
pub struct CurrentPage {
    pub page_id: Uuid,
    pub version: i32,
    pub doc: Option<serde_json::Value>,
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
        "SELECT p.id AS page_id, COALESCE(v.number, 0) AS version, v.doc
         FROM pages p
         LEFT JOIN page_versions v ON v.id = p.current_version_id
         WHERE p.tenant_id = $1 AND p.slug = $2",
    )
    .bind(tenant_id)
    .bind(slug)
    .fetch_optional(pool)
    .await
}

/// Productos del tenant por orden de creación.
pub async fn list_products(pool: &PgPool, tenant_id: Uuid) -> sqlx::Result<Vec<Product>> {
    let rows = sqlx::query_as::<_, (Uuid, String, i64, String, String)>(
        "SELECT id, name, price_cents, image_url, description
         FROM products
         WHERE tenant_id = $1
         ORDER BY created_at, id",
    )
    .bind(tenant_id)
    .fetch_all(pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(|(id, name, price_cents, image_url, description)| Product {
            id: id.to_string(),
            name,
            price_cents,
            image_url,
            description,
        })
        .collect())
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
/// versión `number + 1`, la marca como vigente y borra las que quedan fuera de
/// las últimas [`KEEP_VERSIONS`].
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

    sqlx::query("DELETE FROM page_versions WHERE page_id = $1 AND number <= $2")
        .bind(page_id)
        .bind(number - KEEP_VERSIONS)
        .execute(&mut *tx)
        .await?;

    tx.commit().await?;
    Ok(SaveOutcome::Saved {
        version: u32::try_from(number).unwrap_or(0),
    })
}
