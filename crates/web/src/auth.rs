//! Cuentas de dueño: server functions para entrar con un código por email,
//! salir y saber quién está mirando.
//!
//! Por ahora solo pueden entrar los emails de `ALLOWED_EMAILS` (una lista
//! separada por comas en `.env`, que se relee en cada pedido). El código se escribe en la terminal del
//! servidor: todavía no hay servicio de envío de emails.

use domain::{CodeCheck, CodeRequest, Me};
use leptos::prelude::*;

#[cfg(feature = "ssr")]
pub use ssr::*;

#[cfg(feature = "ssr")]
mod ssr {
    use super::*;
    use crate::server::{internal, pool, TENANT_SLUG};
    use http::{header, HeaderValue};
    use std::collections::HashSet;

    /// Nombre de la cookie de sesión.
    pub const COOKIE: &str = "mm_session";

    /// Configuración de las cuentas.
    #[derive(Clone, Debug, Default)]
    pub struct AuthConfig {
        /// Cookie `Secure` (solo con https, en producción).
        pub secure_cookie: bool,
    }

    impl AuthConfig {
        pub fn new(secure_cookie: bool) -> Self {
            Self { secure_cookie }
        }
    }

    /// Emails habilitados para entrar, normalizados: los de `ALLOWED_EMAILS`
    /// del entorno más los que tenga hoy el archivo `.env`. El archivo se lee
    /// en cada pedido, así que agregar un email no obliga a reiniciar el
    /// servidor (y una variable vieja en la terminal no lo tapa).
    pub fn allowed_emails() -> HashSet<String> {
        let from_env = std::env::var("ALLOWED_EMAILS").unwrap_or_default();
        let from_file = dotenvy::from_filename_iter(".env")
            .into_iter()
            .flatten()
            .flatten()
            .filter(|(k, _)| k == "ALLOWED_EMAILS")
            .map(|(_, v)| v)
            .collect::<Vec<_>>()
            .join(",");
        format!("{from_env},{from_file}")
            .split([',', ';', ' '])
            .map(|e| e.trim_matches(|c| c == '"' || c == '\''))
            .filter_map(domain::normalize_email)
            .collect()
    }

    pub fn is_allowed(email: &str) -> bool {
        allowed_emails().contains(email)
    }

    pub(crate) fn config() -> AuthConfig {
        use_context::<AuthConfig>().unwrap_or_default()
    }

    /// El token de la cookie de sesión del pedido actual.
    pub(crate) fn session_token() -> Option<String> {
        let parts = use_context::<http::request::Parts>()?;
        parts
            .headers
            .get_all(header::COOKIE)
            .iter()
            .filter_map(|v| v.to_str().ok())
            .flat_map(|v| v.split(';'))
            .filter_map(|c| c.trim().split_once('='))
            .find(|(name, _)| *name == COOKIE)
            .map(|(_, value)| value.to_string())
            .filter(|v| !v.is_empty() && v.len() <= 128)
    }

    fn set_cookie(value: &str, max_age_secs: i64) {
        let Some(res) = use_context::<leptos_axum::ResponseOptions>() else {
            return;
        };
        let secure = if config().secure_cookie {
            "; Secure"
        } else {
            ""
        };
        let cookie = format!(
            "{COOKIE}={value}; Path=/; HttpOnly; SameSite=Lax; Max-Age={max_age_secs}{secure}"
        );
        if let Ok(v) = HeaderValue::from_str(&cookie) {
            res.append_header(header::SET_COOKIE, v);
        }
    }

    /// La persona que entró, si su sesión sigue vigente y su email sigue
    /// habilitado.
    pub async fn current_user() -> Result<Option<db::auth::SessionUser>, ServerFnError> {
        let Some(token) = session_token() else {
            return Ok(None);
        };
        let user = db::auth::session_user(&pool()?, &token)
            .await
            .map_err(internal)?;
        Ok(user.filter(|u| is_allowed(&u.email)))
    }

    /// La tienda de este pedido. Por ahora siempre `demo`.
    pub(crate) async fn current_tenant() -> Result<db::Tenant, ServerFnError> {
        db::get_tenant_by_slug(&pool()?, TENANT_SLUG)
            .await
            .map_err(internal)?
            .ok_or_else(|| ServerFnError::new("no existe el tenant demo"))
    }

    /// La persona que entró, si puede editar `tenant`; si no, un error en
    /// palabras simples. Toda server function que cambia la tienda lo llama
    /// primero: esconder un botón no alcanza.
    pub async fn require_editor(
        tenant: &db::Tenant,
    ) -> Result<db::auth::SessionUser, ServerFnError> {
        let user = current_user()
            .await?
            .ok_or_else(|| ServerFnError::new("Tenés que entrar con tu cuenta para guardar. Abrí /entrar en otra pestaña y volvé a intentar."))?;
        let role = db::auth::role_in(&pool()?, tenant.id, user.id)
            .await
            .map_err(internal)?;
        match role {
            Some(_) => Ok(user),
            None => Err(ServerFnError::new("Tu cuenta no puede editar esta tienda.")),
        }
    }

    pub(crate) fn set_session_cookie(token: &str) {
        set_cookie(token, i64::from(db::auth::SESSION_DAYS) * 24 * 60 * 60);
    }

    pub(crate) fn clear_session_cookie() {
        set_cookie("", 0);
    }
}

/// Quién está mirando y si puede editar esta tienda. `None` si no entró.
#[server]
pub async fn who_am_i() -> Result<Option<Me>, ServerFnError> {
    let Some(user) = current_user().await? else {
        return Ok(None);
    };
    let tenant = current_tenant().await?;
    let role = db::auth::role_in(&crate::server::pool()?, tenant.id, user.id)
        .await
        .map_err(crate::server::internal)?;
    Ok(Some(Me {
        email: user.email,
        can_edit: role.is_some(),
    }))
}

/// Paso 1: manda un código de 6 dígitos al email, si está habilitado.
#[server]
pub async fn request_code(email: String) -> Result<CodeRequest, ServerFnError> {
    let Some(email) = domain::normalize_email(&email) else {
        return Ok(CodeRequest::BadEmail);
    };
    if !is_allowed(&email) {
        leptos::logging::log!(
            "\n  {email} pidió un código pero no está en ALLOWED_EMAILS (hay {} habilitados). Agregalo en .env.\n",
            allowed_emails().len()
        );
        return Ok(CodeRequest::NotAllowed);
    }
    let pool = crate::server::pool()?;
    match db::auth::create_login_code(&pool, &email)
        .await
        .map_err(crate::server::internal)?
    {
        Err(db::auth::TooMany) => Ok(CodeRequest::TooMany),
        Ok(code) => {
            // Sin servicio de emails todavía: el código va a la terminal.
            leptos::logging::log!("\n  Código para entrar de {email}: {code}\n");
            Ok(CodeRequest::Sent)
        }
    }
}

/// Paso 2: revisa el código; si es correcto abre la sesión (cookie) y, si la
/// tienda todavía no tiene dueño, esta persona pasa a serlo.
#[server]
pub async fn check_code(
    email: String,
    code: String,
    next: String,
) -> Result<CodeCheck, ServerFnError> {
    let (Some(email), Some(code)) = (
        domain::normalize_email(&email),
        domain::normalize_code(&code),
    ) else {
        return Ok(CodeCheck::Wrong {
            left: db::auth::CODE_ATTEMPTS as u32,
        });
    };
    if !is_allowed(&email) {
        return Ok(CodeCheck::Expired);
    }
    let pool = crate::server::pool()?;
    let result = db::auth::check_login_code(&pool, &email, &code)
        .await
        .map_err(crate::server::internal)?;
    match result {
        db::auth::CodeResult::Wrong { left } => Ok(CodeCheck::Wrong { left }),
        db::auth::CodeResult::Expired => Ok(CodeCheck::Expired),
        db::auth::CodeResult::Ok { user_id } => {
            let tenant = current_tenant().await?;
            db::auth::claim_if_ownerless(&pool, tenant.id, user_id)
                .await
                .map_err(crate::server::internal)?;
            let token = db::auth::create_session(&pool, user_id)
                .await
                .map_err(crate::server::internal)?;
            set_session_cookie(&token);
            Ok(CodeCheck::In {
                next: domain::safe_next(&next),
            })
        }
    }
}

/// Cierra la sesión en este navegador.
#[server]
pub async fn sign_out() -> Result<(), ServerFnError> {
    if let Some(token) = session_token() {
        db::auth::delete_session(&crate::server::pool()?, &token)
            .await
            .map_err(crate::server::internal)?;
    }
    clear_session_cookie();
    Ok(())
}
