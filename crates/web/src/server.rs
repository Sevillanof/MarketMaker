//! Server functions de la tienda: `load_page` (lo publicado), `load_draft`,
//! `save_page` (guarda el borrador) y `publish_page`.

use domain::{PageDoc, PageView, SaveOutcome};
use leptos::prelude::*;
use leptos::server_fn::codec::Json;

#[cfg(feature = "ssr")]
pub(crate) fn pool() -> Result<db::Pool, ServerFnError> {
    use_context::<db::Pool>().ok_or_else(|| ServerFnError::new("falta el pool de la base"))
}

#[cfg(feature = "ssr")]
/// Una falla nuestra (la base, un servicio): el detalle queda en el registro
/// del servidor y al navegador solo le llega un mensaje general, sin nombres
/// de tablas ni datos internos.
pub(crate) fn internal(e: impl std::fmt::Display) -> ServerFnError {
    leptos::logging::error!("Error interno: {e}");
    ServerFnError::new("Tuvimos un problema de nuestro lado. Probá de nuevo en un momento.")
}

#[cfg(feature = "ssr")]
fn page_view(
    page: db::CurrentPage,
    products: Vec<domain::Product>,
    store: domain::StoreInfo,
) -> Result<PageView, ServerFnError> {
    let doc = match page.doc {
        Some(v) => domain::parse_doc(v).map_err(internal)?,
        None => domain::PageDoc::default(),
    };
    Ok(PageView {
        doc,
        version: u32::try_from(page.version).unwrap_or(0),
        published: u32::try_from(page.published).unwrap_or(0),
        products,
        store_name: store.name.clone(),
        store,
        test_store: false,
        coupons: Vec::new(),
    })
}

/// Los cupones de la tienda que sirven hoy (para el bloque Cupón).
#[cfg(feature = "ssr")]
async fn active_coupons(pool: &db::Pool, tenant: uuid::Uuid) -> Result<Vec<domain::CouponBrief>, ServerFnError> {
    let today = db::coupons::today(pool).await.map_err(internal)?;
    let list = db::coupons::list(pool, tenant).await.map_err(internal)?;
    Ok(list
        .iter()
        .filter(|c| c.status(&today) == domain::CouponStatus::Active)
        .map(domain::Coupon::brief)
        .collect())
}

/// Lo que ven los visitantes: la versión publicada de la página `slug`, con
/// sus productos (sin los de ejemplo). Los cambios guardados sin publicar no
/// aparecen; si nunca se publicó, `published` es 0.
#[server]
pub async fn load_page(slug: String) -> Result<PageView, ServerFnError> {
    let pool = pool()?;
    let tenant = crate::auth::current_tenant().await?;
    let page = db::get_published_page(&pool, tenant.id, &slug)
        .await
        .map_err(internal)?
        .ok_or_else(|| ServerFnError::new(format!("no existe la página {slug}")))?;
    let products = db::products::list_for_buyers(&pool, tenant.id)
        .await
        .map_err(internal)?;
    let store = db::auth::store_info(&pool, tenant.id)
        .await
        .map_err(internal)?;
    let mut view = page_view(page, products, store)?;
    view.test_store = crate::mp::is_test_store(&tenant.slug);
    view.coupons = active_coupons(&pool, tenant.id).await?;
    Ok(view)
}

/// Para el editor: lo último guardado (el borrador), aunque no esté
/// publicado. Solo para quien puede editar la tienda.
#[server]
pub async fn load_draft(slug: String) -> Result<PageView, ServerFnError> {
    let pool = pool()?;
    let tenant = crate::auth::current_tenant().await?;
    crate::auth::require_editor(&tenant).await?;
    let page = db::get_current_page(&pool, tenant.id, &slug)
        .await
        .map_err(internal)?
        .ok_or_else(|| ServerFnError::new(format!("no existe la página {slug}")))?;
    let products = db::list_products(&pool, tenant.id)
        .await
        .map_err(internal)?;
    let store = db::auth::store_info(&pool, tenant.id)
        .await
        .map_err(internal)?;
    let mut view = page_view(page, products, store)?;
    view.coupons = active_coupons(&pool, tenant.id).await?;
    Ok(view)
}

/// Vista de celular del editor: lo último guardado (el borrador) con los
/// productos que ven los compradores. Solo para quien puede editar.
#[server]
pub async fn load_preview(slug: String) -> Result<PageView, ServerFnError> {
    let pool = pool()?;
    let tenant = crate::auth::current_tenant().await?;
    crate::auth::require_editor(&tenant).await?;
    let page = db::get_current_page(&pool, tenant.id, &slug)
        .await
        .map_err(internal)?
        .ok_or_else(|| ServerFnError::new(format!("no existe la página {slug}")))?;
    let products = db::products::list_for_buyers(&pool, tenant.id)
        .await
        .map_err(internal)?;
    let store = db::auth::store_info(&pool, tenant.id)
        .await
        .map_err(internal)?;
    let mut view = page_view(page, products, store)?;
    view.coupons = active_coupons(&pool, tenant.id).await?;
    Ok(view)
}

/// Publica la versión `version` de la página: desde ahora la ven los
/// visitantes. Devuelve la versión publicada.
#[server]
pub async fn publish_page(slug: String, version: u32) -> Result<u32, ServerFnError> {
    let pool = pool()?;
    let tenant = crate::auth::current_tenant().await?;
    crate::auth::require_editor(&tenant).await?;
    if db::publish_page(&pool, tenant.id, &slug, version)
        .await
        .map_err(internal)?
    {
        crate::analytics::record_event("publico", None).await;
        Ok(version)
    } else {
        Err(ServerFnError::new(
            "No encontramos esa versión para publicar. Recargá la página y probá de nuevo.",
        ))
    }
}

/// Guarda `doc` como borrador (nueva versión) de la página `slug` si
/// `base_version` sigue siendo la vigente y el documento es válido. No publica. El documento viaja como JSON.
#[server(input = Json)]
pub async fn save_page(
    slug: String,
    doc: PageDoc,
    base_version: u32,
) -> Result<SaveOutcome, ServerFnError> {
    let pool = pool()?;
    let tenant = crate::auth::current_tenant().await?;
    let user = crate::auth::require_editor(&tenant).await?;
    db::save_page_by(&pool, tenant.id, &slug, &doc, base_version, Some(user.id))
        .await
        .map_err(internal)
}
