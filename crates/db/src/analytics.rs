//! Uso y abandono: eventos anónimos y los números del panel del administrador.
//! No se guarda nombre, email, IP ni lo que la persona escribe.

use sha2::{Digest, Sha256};
use sqlx::{FromRow, PgPool};
use std::sync::OnceLock;

/// Cuántos días se conserva el detalle de cada visita.
pub const KEEP_DAYS: i32 = 90;

/// El código de una visita: un hash de IP + navegador con una sal que cambia
/// cada día y solo vive en la memoria del servidor. No se puede volver a la
/// persona y mañana el mismo visitante tiene otro código.
pub fn visit_code(ip: &str, user_agent: &str) -> String {
    static SECRET: OnceLock<[u8; 16]> = OnceLock::new();
    let secret = SECRET.get_or_init(|| {
        use rand::RngCore;
        let mut b = [0u8; 16];
        rand::rng().fill_bytes(&mut b);
        b
    });
    let day = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs() / 86_400);
    let mut h = Sha256::new();
    h.update(secret);
    h.update(day.to_be_bytes());
    h.update(ip.as_bytes());
    h.update([0]);
    h.update(user_agent.as_bytes());
    hex::encode(&h.finalize()[..4])
}

pub async fn record(
    pool: &PgPool,
    visit: &str,
    who: &str,
    step: &str,
    store: Option<&str>,
    device: &str,
    detail: Option<&str>,
) -> sqlx::Result<()> {
    sqlx::query(
        "INSERT INTO eventos (visita, quien, paso, tienda, dispositivo, detalle)
         VALUES ($1, $2, $3, $4, $5, $6)",
    )
    .bind(visit)
    .bind(who)
    .bind(step)
    .bind(store)
    .bind(device)
    .bind(detail)
    .execute(pool)
    .await?;
    Ok(())
}

/// Borra los eventos más viejos que [`KEEP_DAYS`].
pub async fn purge(pool: &PgPool) -> sqlx::Result<u64> {
    Ok(
        sqlx::query("DELETE FROM eventos WHERE creado < now() - make_interval(days => $1)")
            .bind(KEEP_DAYS)
            .execute(pool)
            .await?
            .rows_affected(),
    )
}

/// Dueños: cuántas cuentas nuevas (de los últimos `days` días) llegaron a cada
/// paso, en orden: cuenta, tienda, editó, producto, publicó, cobros, vendió.
/// Cada paso cuenta solo a quienes ya pasaron por los anteriores.
pub async fn owner_funnel(pool: &PgPool, days: i32) -> sqlx::Result<Vec<i64>> {
    let r: (i64, i64, i64, i64, i64, i64, i64) = sqlx::query_as(
        "WITH f AS (
           SELECT u.id,
             EXISTS (SELECT 1 FROM memberships m WHERE m.user_id = u.id) AS tienda,
             EXISTS (SELECT 1 FROM memberships m JOIN page_versions v ON v.tenant_id = m.tenant_id
                     WHERE m.user_id = u.id AND v.number > 1) AS edito,
             EXISTS (SELECT 1 FROM memberships m JOIN products p ON p.tenant_id = m.tenant_id
                     WHERE m.user_id = u.id AND NOT p.example) AS producto,
             EXISTS (SELECT 1 FROM memberships m JOIN pages g ON g.tenant_id = m.tenant_id
                     WHERE m.user_id = u.id AND g.published_version_id IS NOT NULL) AS publico,
             EXISTS (SELECT 1 FROM memberships m JOIN tenants t ON t.id = m.tenant_id
                     WHERE m.user_id = u.id
                       AND (t.mp_user_id IS NOT NULL OR (t.pay_transfer AND t.transfer_alias <> ''))) AS cobros,
             EXISTS (SELECT 1 FROM memberships m JOIN orders o ON o.tenant_id = m.tenant_id
                     WHERE m.user_id = u.id AND o.status IN ('paid', 'shipped', 'delivered')) AS vendio
           FROM users u WHERE u.created_at >= now() - make_interval(days => $1)
         )
         SELECT count(*),
           count(*) FILTER (WHERE tienda),
           count(*) FILTER (WHERE tienda AND edito),
           count(*) FILTER (WHERE tienda AND edito AND producto),
           count(*) FILTER (WHERE tienda AND edito AND producto AND publico),
           count(*) FILTER (WHERE tienda AND edito AND producto AND publico AND cobros),
           count(*) FILTER (WHERE tienda AND edito AND producto AND publico AND cobros AND vendio)
         FROM f",
    )
    .bind(days)
    .fetch_one(pool)
    .await?;
    Ok(vec![r.0, r.1, r.2, r.3, r.4, r.5, r.6])
}

/// Compradores: visitas (de los últimos `days` días) que llegaron a cada paso:
/// tienda, producto, carrito, comprar, entrega, pago; y al final, pedidos pagados.
pub async fn buyer_funnel(pool: &PgPool, days: i32) -> sqlx::Result<Vec<i64>> {
    let r: (i64, i64, i64, i64, i64, i64, i64) = sqlx::query_as(
        "WITH v AS (
           SELECT visita, array_agg(DISTINCT paso) AS pasos FROM eventos
           WHERE quien = 'buyer' AND creado >= now() - make_interval(days => $1)
           GROUP BY visita
         )
         SELECT count(*),
           count(*) FILTER (WHERE 'producto' = ANY(pasos)),
           count(*) FILTER (WHERE 'carrito' = ANY(pasos)),
           count(*) FILTER (WHERE 'comprar' = ANY(pasos)),
           count(*) FILTER (WHERE 'entrega' = ANY(pasos)),
           count(*) FILTER (WHERE 'pago' = ANY(pasos)),
           (SELECT count(*) FROM orders WHERE status IN ('paid', 'shipped', 'delivered')
              AND created_at >= now() - make_interval(days => $1))
         FROM v",
    )
    .bind(days)
    .fetch_one(pool)
    .await?;
    Ok(vec![r.0, r.1, r.2, r.3, r.4, r.5, r.6])
}

/// Una visita de la lista: hasta dónde llegó y cuánto estuvo.
#[derive(Debug, Clone, FromRow)]
pub struct VisitRow {
    pub visita: String,
    pub ultimo: String,
    pub segundos: i64,
    pub dispositivo: String,
    pub termino: bool,
    /// Segundos desde que empezó la visita.
    pub hace: i64,
}

/// Las visitas más recientes. `who` es 'owner' o 'buyer'; con `only_dropped`,
/// solo las que no terminaron (dueño: publicar; comprador: pedido hecho).
pub async fn recent_visits(
    pool: &PgPool,
    who: &str,
    days: i32,
    only_dropped: bool,
    limit: i64,
) -> sqlx::Result<Vec<VisitRow>> {
    sqlx::query_as(
        "SELECT * FROM (
           SELECT visita,
             (array_agg(paso ORDER BY creado DESC, id DESC))[1] AS ultimo,
             EXTRACT(EPOCH FROM max(creado) - min(creado))::bigint AS segundos,
             (array_agg(dispositivo ORDER BY id))[1] AS dispositivo,
             bool_or(paso = CASE WHEN quien = 'owner' THEN 'publico' ELSE 'pedido' END) AS termino,
             EXTRACT(EPOCH FROM now() - min(creado))::bigint AS hace,
             min(creado) AS inicio
           FROM eventos
           WHERE quien = $1 AND creado >= now() - make_interval(days => $2)
           GROUP BY visita, quien
         ) x
         WHERE NOT ($3 AND termino)
         ORDER BY inicio DESC LIMIT $4",
    )
    .bind(who)
    .bind(days)
    .bind(only_dropped)
    .bind(limit)
    .fetch_all(pool)
    .await
}

/// Los pasos de una visita, en orden: segundos desde el primero, paso y detalle.
pub async fn visit_steps(
    pool: &PgPool,
    visit: &str,
    who: &str,
) -> sqlx::Result<Vec<(i64, String, Option<String>)>> {
    sqlx::query_as(
        "SELECT EXTRACT(EPOCH FROM creado - min(creado) OVER ())::bigint, paso, detalle
         FROM eventos WHERE visita = $1 AND quien = $2
           AND creado >= now() - make_interval(days => $3)
         ORDER BY creado, id",
    )
    .bind(visit)
    .bind(who)
    .bind(KEEP_DAYS)
    .fetch_all(pool)
    .await
}
