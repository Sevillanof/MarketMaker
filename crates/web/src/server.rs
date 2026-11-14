//! Server functions: `load_page` y `save_page`.

use domain::PageView;
use leptos::prelude::*;

/// Tenant fijo de esta etapa (todavía no hay subdominios ni login).
#[cfg(feature = "ssr")]
pub(crate) const TENANT_SLUG: &str = "demo";

#[cfg(feature = "ssr")]
pub(crate) fn pool() -> Result<db::Pool, ServerFnError> {
    use_context::<db::Pool>().ok_or_else(|| ServerFnError::new("falta el pool de la base"))
}

#[cfg(feature = "ssr")]
fn internal(e: impl std::fmt::Display) -> ServerFnError {
    ServerFnError::new(e.to_string())
}

/// Última versión guardada de la página `slug` del tenant `demo`, con sus productos.
#[server]
pub async fn load_page(slug: String) -> Result<PageView, ServerFnError> {
    let pool = pool()?;
    let tenant = db::get_tenant_by_slug(&pool, TENANT_SLUG)
        .await
        .map_err(internal)?
        .ok_or_else(|| ServerFnError::new("no existe el tenant demo"))?;
    let page = db::get_current_page(&pool, tenant.id, &slug)
        .await
        .map_err(internal)?
        .ok_or_else(|| ServerFnError::new(format!("no existe la página {slug}")))?;
    let doc = match page.doc {
        Some(v) => serde_json::from_value(v).map_err(internal)?,
        None => domain::PageDoc::default(),
    };
    let products = db::list_products(&pool, tenant.id)
        .await
        .map_err(internal)?;
    Ok(PageView {
        doc,
        version: u32::try_from(page.version).unwrap_or(0),
        products,
    })
}
