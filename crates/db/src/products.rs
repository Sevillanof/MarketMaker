//! Productos de una tienda: los que carga el dueño en la pantalla Productos
//! y los que se muestran en la tienda.

use domain::Product;
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

/// Más que esto por tienda es seguro un error (o un abuso).
pub const MAX_PER_TENANT: i64 = 2000;

#[derive(FromRow)]
struct Row {
    id: Uuid,
    name: String,
    price_cents: i64,
    image_url: String,
    description: String,
    compare_at_cents: i64,
    in_stock: bool,
    stock: Option<i32>,
    visible: bool,
    photos: Vec<String>,
    tags: Vec<String>,
}

impl From<Row> for Product {
    fn from(r: Row) -> Self {
        Product {
            id: r.id.to_string(),
            name: r.name,
            price_cents: r.price_cents,
            image_url: r.image_url,
            description: r.description,
            compare_at_cents: r.compare_at_cents,
            in_stock: r.in_stock && r.stock != Some(0),
            stock: r.stock.map(|s| s.max(0) as u32),
            visible: r.visible,
            photos: r.photos,
            tags: r.tags,
        }
    }
}

/// `SELECT … ` con todas las columnas de un producto, seguido de `rest`.
macro_rules! select {
    ($rest:literal) => {
        concat!(
            "SELECT id, name, price_cents, image_url, description, compare_at_cents, ",
            "in_stock, stock, visible, photos, tags ",
            $rest
        )
    };
}

/// Los productos visibles, el más nuevo primero (lo que ve la tienda): así
/// un producto recién cargado aparece en las grillas.
pub async fn list_visible(pool: &PgPool, tenant: Uuid) -> sqlx::Result<Vec<Product>> {
    let rows = sqlx::query_as::<_, Row>(select!(
        "FROM products WHERE tenant_id = $1 AND visible ORDER BY created_at DESC, id"
    ))
    .bind(tenant)
    .fetch_all(pool)
    .await?;
    Ok(rows.into_iter().map(Into::into).collect())
}

/// Todos los productos, el último que se tocó primero (lo que ve el dueño).
pub async fn list_all(pool: &PgPool, tenant: Uuid) -> sqlx::Result<Vec<Product>> {
    let rows = sqlx::query_as::<_, Row>(select!(
        "FROM products WHERE tenant_id = $1 ORDER BY updated_at DESC, id"
    ))
    .bind(tenant)
    .fetch_all(pool)
    .await?;
    Ok(rows.into_iter().map(Into::into).collect())
}

/// Los productos que llevan la etiqueta `tag` (visibles o no).
pub async fn with_tag(pool: &PgPool, tenant: Uuid, tag: &str) -> sqlx::Result<Vec<Product>> {
    let rows = sqlx::query_as::<_, Row>(select!(
        "FROM products WHERE tenant_id = $1 AND tags @> ARRAY[$2]
         ORDER BY created_at, id"
    ))
    .bind(tenant)
    .bind(tag)
    .fetch_all(pool)
    .await?;
    Ok(rows.into_iter().map(Into::into).collect())
}

pub async fn get(pool: &PgPool, tenant: Uuid, id: Uuid) -> sqlx::Result<Option<Product>> {
    let row = sqlx::query_as::<_, Row>(select!("FROM products WHERE tenant_id = $1 AND id = $2"))
        .bind(tenant)
        .bind(id)
        .fetch_optional(pool)
        .await?;
    Ok(row.map(Into::into))
}

pub async fn count(pool: &PgPool, tenant: Uuid) -> sqlx::Result<i64> {
    sqlx::query_scalar("SELECT count(*) FROM products WHERE tenant_id = $1")
        .bind(tenant)
        .fetch_one(pool)
        .await
}

/// Crea un producto (ya validado con [`domain::tidy_product`]) y devuelve su id.
pub async fn create(pool: &PgPool, tenant: Uuid, p: &Product) -> sqlx::Result<Uuid> {
    sqlx::query_scalar(
        "INSERT INTO products (tenant_id, name, price_cents, image_url, description,
                               compare_at_cents, in_stock, visible, photos, tags, stock)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
         RETURNING id",
    )
    .bind(tenant)
    .bind(&p.name)
    .bind(p.price_cents)
    .bind(&p.image_url)
    .bind(&p.description)
    .bind(p.compare_at_cents)
    .bind(p.in_stock)
    .bind(p.visible)
    .bind(&p.photos)
    .bind(&p.tags)
    .bind(p.stock.map(|s| s as i32))
    .fetch_one(pool)
    .await
}

/// Guarda los cambios de un producto. `false` si no existe en esa tienda.
pub async fn save(pool: &PgPool, tenant: Uuid, id: Uuid, p: &Product) -> sqlx::Result<bool> {
    let done = sqlx::query(
        "UPDATE products SET name = $3, price_cents = $4, image_url = $5, description = $6,
                compare_at_cents = $7, in_stock = $8, visible = $9, photos = $10, tags = $11,
                stock = $12, updated_at = now()
         WHERE tenant_id = $1 AND id = $2",
    )
    .bind(tenant)
    .bind(id)
    .bind(&p.name)
    .bind(p.price_cents)
    .bind(&p.image_url)
    .bind(&p.description)
    .bind(p.compare_at_cents)
    .bind(p.in_stock)
    .bind(p.visible)
    .bind(&p.photos)
    .bind(&p.tags)
    .bind(p.stock.map(|s| s as i32))
    .execute(pool)
    .await?;
    Ok(done.rows_affected() == 1)
}

pub async fn set_visible(
    pool: &PgPool,
    tenant: Uuid,
    id: Uuid,
    visible: bool,
) -> sqlx::Result<bool> {
    let done = sqlx::query(
        "UPDATE products SET visible = $3, updated_at = now() WHERE tenant_id = $1 AND id = $2",
    )
    .bind(tenant)
    .bind(id)
    .bind(visible)
    .execute(pool)
    .await?;
    Ok(done.rows_affected() == 1)
}

pub async fn delete(pool: &PgPool, tenant: Uuid, id: Uuid) -> sqlx::Result<bool> {
    let done = sqlx::query("DELETE FROM products WHERE tenant_id = $1 AND id = $2")
        .bind(tenant)
        .bind(id)
        .execute(pool)
        .await?;
    Ok(done.rows_affected() == 1)
}

/// Todas las etiquetas de la tienda, de la más usada a la menos usada.
pub async fn tags(pool: &PgPool, tenant: Uuid) -> sqlx::Result<Vec<String>> {
    sqlx::query_scalar(
        "SELECT t FROM products, unnest(tags) AS t
         WHERE tenant_id = $1
         GROUP BY t ORDER BY count(*) DESC, t",
    )
    .bind(tenant)
    .fetch_all(pool)
    .await
}

/// Las etiquetas de la tienda con en cuántos productos está cada una, de la
/// más usada a la menos usada.
pub async fn tag_counts(pool: &PgPool, tenant: Uuid) -> sqlx::Result<Vec<domain::TagCount>> {
    let rows: Vec<(String, i64)> = sqlx::query_as(
        "SELECT t, count(*) FROM products, unnest(tags) AS t
         WHERE tenant_id = $1
         GROUP BY t ORDER BY count(*) DESC, t",
    )
    .bind(tenant)
    .fetch_all(pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(|(tag, products)| domain::TagCount { tag, products })
        .collect())
}

/// Los productos `ids` de la tienda (los que existan), para armar un pedido
/// con los precios de la base y no con los que mande el navegador.
pub async fn by_ids(pool: &PgPool, tenant: Uuid, ids: &[Uuid]) -> sqlx::Result<Vec<Product>> {
    let rows = sqlx::query_as::<_, Row>(select!(
        "FROM products WHERE tenant_id = $1 AND id = ANY($2)"
    ))
    .bind(tenant)
    .bind(ids)
    .fetch_all(pool)
    .await?;
    Ok(rows.into_iter().map(Into::into).collect())
}

/// Productos sin stock: marcados "Sin stock" o con 0 unidades.
pub async fn out_of_stock(pool: &PgPool, tenant: Uuid) -> sqlx::Result<i64> {
    sqlx::query_scalar(
        "SELECT count(*) FROM products WHERE tenant_id = $1 AND (NOT in_stock OR stock = 0)",
    )
    .bind(tenant)
    .fetch_one(pool)
    .await
}
