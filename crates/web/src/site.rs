//! La dirección de la página actual, igual en el servidor y en el navegador,
//! para saber si se está mirando la plataforma o una tienda.

use domain::Site;

/// `Host` del pedido (`demo.localhost:3000`).
pub fn current_host() -> String {
    #[cfg(feature = "ssr")]
    {
        use leptos::prelude::use_context;
        use_context::<http::request::Parts>()
            .and_then(|p| {
                p.headers
                    .get(http::header::HOST)
                    .and_then(|h| h.to_str().ok())
                    .map(str::to_string)
                    .or_else(|| p.uri.authority().map(|a| a.to_string()))
            })
            .unwrap_or_default()
    }
    #[cfg(not(feature = "ssr"))]
    {
        leptos::prelude::window()
            .location()
            .host()
            .unwrap_or_default()
    }
}

/// El sitio de la página actual.
pub fn current_site() -> Site {
    domain::site_from_host(&current_host())
}

/// `http` en desarrollo, `https` en producción (mismo origen que la página).
pub fn scheme() -> String {
    #[cfg(feature = "ssr")]
    {
        if crate::auth::config().secure_cookie {
            "https"
        } else {
            "http"
        }
        .to_string()
    }
    #[cfg(not(feature = "ssr"))]
    {
        leptos::prelude::window()
            .location()
            .protocol()
            .unwrap_or_else(|_| "http:".into())
            .trim_end_matches(':')
            .to_string()
    }
}

/// Dirección completa de la plataforma vista desde aquí.
pub fn platform_url() -> String {
    format!("{}://{}", scheme(), domain::platform_host(&current_host()))
}
