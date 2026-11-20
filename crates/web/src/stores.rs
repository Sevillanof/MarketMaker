//! Varias tiendas: "Tus tiendas" en la plataforma, crear una tienda y pasar
//! al editor de una tienda sin volver a pedir un código.

use domain::{NewStoreOutcome, StoreCard};
use leptos::prelude::*;

#[cfg(feature = "ssr")]
pub(crate) fn store_url(slug: &str) -> String {
    let platform = domain::platform_host(&crate::site::current_host());
    format!(
        "{}://{}",
        crate::site::scheme(),
        domain::store_host(&platform, slug)
    )
}

/// Enlace de un solo uso que abre sesión en la tienda y lleva a su editor.
#[cfg(feature = "ssr")]
async fn pass_url(pool: &db::Pool, user: uuid::Uuid, slug: &str) -> Result<String, ServerFnError> {
    let tenant = db::get_tenant_by_slug(pool, slug)
        .await
        .map_err(crate::server::internal)?
        .ok_or_else(|| ServerFnError::new("No existe esa tienda."))?;
    let token = db::auth::create_handoff(pool, user, tenant.id)
        .await
        .map_err(crate::server::internal)?;
    Ok(format!("{}/entrar/pase?t={token}", store_url(slug)))
}

/// Las tiendas de quien entró. Vacío si no entró.
#[server]
pub async fn my_stores() -> Result<Vec<StoreCard>, ServerFnError> {
    let Some(user) = crate::auth::current_user().await? else {
        return Ok(vec![]);
    };
    let rows = db::auth::stores_for_user(&crate::server::pool()?, user.id)
        .await
        .map_err(crate::server::internal)?;
    Ok(rows
        .into_iter()
        .map(|r| StoreCard {
            url: store_url(&r.slug),
            name: r.name,
            slug: r.slug,
        })
        .collect())
}

/// Una dirección libre parecida a `slug` (`ana`, `ana-2`, `ana-3`…).
#[cfg(feature = "ssr")]
async fn free_slug(pool: &db::Pool, slug: &str) -> Result<String, ServerFnError> {
    let base: String = if slug.len() < 3 {
        format!("{slug}tienda")
    } else {
        slug.chars().take(26).collect::<String>().trim_end_matches('-').to_string()
    };
    for n in 1..1000 {
        let candidate = if n == 1 {
            base.clone()
        } else {
            format!("{base}-{n}")
        };
        if domain::store_slug_problem(&candidate).is_none()
            && !db::auth::slug_taken(pool, &candidate)
                .await
                .map_err(crate::server::internal)?
        {
            return Ok(candidate);
        }
    }
    Err(ServerFnError::new("No encontramos una dirección libre."))
}

/// Crea una tienda con quien entró como dueño. Arranca con el diseño inicial
/// sin publicar y los productos de `demo` como ejemplo.
#[server]
pub async fn create_store(
    name: String,
    slug: String,
    template: String,
) -> Result<NewStoreOutcome, ServerFnError> {
    let user = crate::auth::current_user().await?.ok_or_else(|| {
        ServerFnError::new("Tenés que entrar con tu cuenta para crear una tienda.")
    })?;
    let name = name.trim().to_string();
    if name.is_empty() {
        return Ok(NewStoreOutcome::Problem(
            "Escribí el nombre de tu tienda.".into(),
        ));
    }
    if name.chars().count() > 60 {
        return Ok(NewStoreOutcome::Problem(
            "El nombre puede tener hasta 60 letras.".into(),
        ));
    }
    let slug = slug.trim().to_lowercase();
    if let Some(problem) = domain::store_slug_problem(&slug) {
        return Ok(NewStoreOutcome::Problem(problem.into()));
    }
    let pool = crate::server::pool()?;
    let doc = serde_json::to_value(domain::template_doc(&template, &name)).map_err(crate::server::internal)?;
    let created = db::auth::create_store(&pool, &name, &slug, user.id, "demo", &doc)
        .await
        .map_err(crate::server::internal)?;
    if !created {
        return Ok(NewStoreOutcome::Taken {
            suggestion: free_slug(&pool, &slug).await?,
        });
    }
    crate::analytics::record_event("tienda_creada", None).await;
    warm_certificate(&slug).await;
    // Primero Inicio, con los primeros pasos: el editor es la pantalla más
    // difícil para arrancar.
    Ok(NewStoreOutcome::Created {
        open: format!("/tienda/{slug}"),
    })
}

/// En producción cada tienda tiene su propio certificado https, que Caddy
/// pide la primera vez que alguien entra a la dirección. Eso tardaba unos
/// 10 segundos con la pantalla quieta justo al crear la tienda: lo pedimos
/// acá, mientras la persona ve "Creando tu tienda…". Si falla, no pasa nada:
/// Caddy lo pide igual cuando el navegador llega.
#[cfg(feature = "ssr")]
async fn warm_certificate(slug: &str) {
    let url = crate::email::store_url(slug);
    if !url.starts_with("https://") {
        return;
    }
    let Ok(client) = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(25))
        .build()
    else {
        return;
    };
    let started = std::time::Instant::now();
    match client.head(format!("{url}/favicon.ico")).send().await {
        Ok(_) => leptos::logging::log!(
            "Certificado de {url} listo en {} ms",
            started.elapsed().as_millis()
        ),
        Err(e) => leptos::logging::log!("No se pudo preparar el certificado de {url}: {e}"),
    }
}

/// ¿Está libre la dirección `slug`? Para avisar mientras se escribe.
#[server]
pub async fn slug_status(slug: String) -> Result<domain::SlugStatus, ServerFnError> {
    use domain::SlugStatus;
    if crate::auth::current_user().await?.is_none() {
        return Err(ServerFnError::new("Tenés que entrar con tu cuenta."));
    }
    // Cada tecla pregunta: 60 por minuto por conexión alcanza de sobra.
    if !crate::limits::allow(
        &format!("slug|{}", crate::limits::client_ip()),
        60,
        std::time::Duration::from_secs(60),
    ) {
        return Err(ServerFnError::new("Esperá un momento."));
    }
    let slug = slug.trim().to_lowercase();
    if let Some(problem) = domain::store_slug_problem(&slug) {
        return Ok(SlugStatus::Problem(problem.into()));
    }
    let pool = crate::server::pool()?;
    if db::auth::slug_taken(&pool, &slug)
        .await
        .map_err(crate::server::internal)?
    {
        return Ok(SlugStatus::Taken {
            suggestion: free_slug(&pool, &slug).await?,
        });
    }
    Ok(SlugStatus::Free)
}

/// Enlace para abrir el editor de una tienda propia desde la plataforma.
#[server]
pub async fn open_store(slug: String) -> Result<String, ServerFnError> {
    let user = crate::auth::current_user()
        .await?
        .ok_or_else(|| ServerFnError::new("Tenés que entrar con tu cuenta."))?;
    let pool = crate::server::pool()?;
    let mine = db::auth::stores_for_user(&pool, user.id)
        .await
        .map_err(crate::server::internal)?
        .iter()
        .any(|s| s.slug == slug);
    if !mine {
        return Err(ServerFnError::new("Tu cuenta no puede editar esa tienda."));
    }
    pass_url(&pool, user.id, &slug).await
}

/// En la tienda: usa el pase y abre la sesión. `false` si venció o ya se usó.
#[server]
pub async fn use_pass(token: String) -> Result<bool, ServerFnError> {
    let tenant = crate::auth::current_tenant().await?;
    let pool = crate::server::pool()?;
    let Some(user_id) = db::auth::redeem_handoff(&pool, token.trim(), tenant.id)
        .await
        .map_err(crate::server::internal)?
    else {
        return Ok(false);
    };
    let session = db::auth::create_session(&pool, user_id)
        .await
        .map_err(crate::server::internal)?;
    crate::auth::set_session_cookie(&session);
    Ok(true)
}
