//! Server functions: `load_page` y `save_page`.

use domain::{PageDoc, PageView, SaveOutcome};
use leptos::prelude::*;
use leptos::server_fn::codec::Json;

/// Tenant fijo de esta etapa (todavía no hay subdominios ni login).
#[cfg(feature = "ssr")]
pub(crate) const TENANT_SLUG: &str = "demo";

#[cfg(feature = "ssr")]
pub(crate) fn pool() -> Result<db::Pool, ServerFnError> {
    use_context::<db::Pool>().ok_or_else(|| ServerFnError::new("falta el pool de la base"))
}

#[cfg(feature = "ssr")]
pub(crate) fn internal(e: impl std::fmt::Display) -> ServerFnError {
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
        Some(v) => domain::parse_doc(v).map_err(internal)?,
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

/// Guarda `doc` como nueva versión de la página `slug` si `base_version` sigue
/// siendo la vigente y el documento es válido. El documento viaja como JSON.
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
