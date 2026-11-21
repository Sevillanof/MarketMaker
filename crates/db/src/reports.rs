//! Reportes de errores que mandan las personas desde cualquier página.

use sqlx::{FromRow, PgPool};
use uuid::Uuid;

/// Días que se conserva un reporte ya resuelto.
pub const KEEP_RESOLVED_DAYS: i32 = 90;

/// Un reporte sin la captura (esa se pide aparte).
#[derive(Debug, Clone, FromRow)]
pub struct Report {
    pub id: Uuid,
    pub num: i64,
    pub estado: String,
    pub descripcion: String,
    pub pagina: String,
    pub quien: String,
    pub pantalla: String,
    pub navegador: String,
    pub tiene_captura: bool,
    /// Segundos desde que llegó.
    pub hace: i64,
}

pub struct NewReport<'a> {
    pub descripcion: &'a str,
    pub pagina: &'a str,
    pub quien: &'a str,
    pub pantalla: &'a str,
    pub navegador: &'a str,
    pub captura: Option<&'a [u8]>,
}

/// Guarda un reporte nuevo y devuelve su número.
pub async fn insert(pool: &PgPool, r: NewReport<'_>) -> sqlx::Result<i64> {
    sqlx::query_scalar(
        "INSERT INTO reportes (descripcion, pagina, quien, pantalla, navegador, captura)
         VALUES ($1, $2, $3, $4, $5, $6) RETURNING num",
    )
    .bind(r.descripcion)
    .bind(r.pagina)
    .bind(r.quien)
    .bind(r.pantalla)
    .bind(r.navegador)
    .bind(r.captura)
    .fetch_one(pool)
    .await
}

/// Las columnas de un reporte, sin la captura (literal para `concat!`).
macro_rules! cols {
    () => {
        "id, num, estado, descripcion, pagina, quien, pantalla, navegador,
         (captura IS NOT NULL) AS tiene_captura,
         EXTRACT(EPOCH FROM now() - creado)::bigint AS hace"
    };
}

/// Los últimos reportes (los más nuevos primero); `estado` vacío = todos.
pub async fn list(pool: &PgPool, estado: &str, limit: i64) -> sqlx::Result<Vec<Report>> {
    sqlx::query_as::<_, Report>(concat!(
        "SELECT ",
        cols!(),
        " FROM reportes WHERE ($1 = '' OR estado = $1)
         ORDER BY creado DESC LIMIT $2"
    ))
    .bind(estado)
    .bind(limit)
    .fetch_all(pool)
    .await
}

pub async fn get(pool: &PgPool, num: i64) -> sqlx::Result<Option<Report>> {
    sqlx::query_as::<_, Report>(concat!("SELECT ", cols!(), " FROM reportes WHERE num = $1"))
        .bind(num)
        .fetch_optional(pool)
        .await
}

/// La captura de un reporte.
pub async fn screenshot(pool: &PgPool, num: i64) -> sqlx::Result<Option<Vec<u8>>> {
    sqlx::query_scalar("SELECT captura FROM reportes WHERE num = $1")
        .bind(num)
        .fetch_optional(pool)
        .await
        .map(Option::flatten)
}

pub async fn set_status(pool: &PgPool, num: i64, estado: &str) -> sqlx::Result<bool> {
    Ok(sqlx::query("UPDATE reportes SET estado = $2 WHERE num = $1")
        .bind(num)
        .bind(estado)
        .execute(pool)
        .await?
        .rows_affected()
        > 0)
}

/// Cuántos hay de cada estado: (nuevos, en curso, resueltos).
pub async fn counts(pool: &PgPool) -> sqlx::Result<(i64, i64, i64)> {
    sqlx::query_as(
        "SELECT count(*) FILTER (WHERE estado = 'nuevo'),
                count(*) FILTER (WHERE estado = 'en_curso'),
                count(*) FILTER (WHERE estado = 'resuelto')
         FROM reportes",
    )
    .fetch_one(pool)
    .await
}

/// Cuántos reportes siguen sin atender.
pub async fn count_new(pool: &PgPool) -> sqlx::Result<i64> {
    sqlx::query_scalar("SELECT count(*) FROM reportes WHERE estado = 'nuevo'")
        .fetch_one(pool)
        .await
}

/// Borra los reportes resueltos hace más de [`KEEP_RESOLVED_DAYS`] días.
pub async fn purge(pool: &PgPool) -> sqlx::Result<u64> {
    Ok(sqlx::query(
        "DELETE FROM reportes WHERE estado = 'resuelto'
         AND creado < now() - make_interval(days => $1)",
    )
    .bind(KEEP_RESOLVED_DAYS)
    .execute(pool)
    .await?
    .rows_affected())
}
