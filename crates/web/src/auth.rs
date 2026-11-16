//! Cuentas de dueño: server functions para entrar (email y contraseña, o un
//! código por email), crear la cuenta, "Mi cuenta", salir y saber quién está
//! mirando.
//!
//! El código por email se usa para confirmar el email al crear la cuenta y
//! cuando la persona se olvidó la contraseña (o prefiere no usarla).
//!
//! Quién puede tener cuenta: si `REGISTRO_ABIERTO=true`, cualquiera; si no,
//! solo los emails de `ALLOWED_EMAILS` (una lista separada por comas en
//! `.env.local`). Las dos cosas se releen en cada pedido.

use domain::{
    AccountInfo, CodeCheck, CodeRequest, EmailLookup, Me, PasswordChange, PasswordSignIn,
    SetPassword,
};
use leptos::prelude::*;

#[cfg(feature = "ssr")]
pub use ssr::*;

#[cfg(feature = "ssr")]
mod ssr {
    use super::*;
    use crate::server::{internal, pool};
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
    /// en `.env.local` (no va a git, es el lugar recomendado), en `.env` y en
    /// la terminal. Los archivos se leen en cada pedido, así que agregar o
    /// quitar un email no obliga a reiniciar el servidor.
    /// Los valores de `key` en la terminal, `.env.local` y `.env`.
    fn env_values(key: &str) -> Vec<String> {
        let from_files = [".env.local", ".env"].into_iter().flat_map(|f| {
            dotenvy::from_filename_iter(f)
                .into_iter()
                .flatten()
                .flatten()
        });
        std::env::var(key)
            .ok()
            .into_iter()
            .chain(from_files.filter(|(k, _)| k == key).map(|(_, v)| v))
            .collect()
    }

    /// Cualquiera puede crear su cuenta (`REGISTRO_ABIERTO=true`). Si no, solo
    /// los emails de `ALLOWED_EMAILS`.
    pub fn signup_open() -> bool {
        env_values("REGISTRO_ABIERTO")
            .first()
            .is_some_and(|v| matches!(v.trim().to_lowercase().as_str(), "true" | "1" | "si" | "sí"))
    }

    pub fn allowed_emails() -> HashSet<String> {
        env_values("ALLOWED_EMAILS")
            .join(",")
            .split([',', ';', ' '])
            .map(|e| e.trim_matches(|c| c == '"' || c == '\''))
            .filter_map(domain::normalize_email)
            .collect()
    }

    pub fn is_allowed(email: &str) -> bool {
        signup_open() || allowed_emails().contains(email)
    }

    /// Corre `f` (cálculo pesado, como un hash de contraseña) fuera de los
    /// hilos que atienden pedidos.
    pub(crate) async fn blocking<T: Send + 'static>(
        f: impl FnOnce() -> T + Send + 'static,
    ) -> Result<T, ServerFnError> {
        tokio::task::spawn_blocking(f).await.map_err(internal)
    }

    /// Abre la sesión de `user_id` en este navegador y, si la tienda de esta
    /// dirección todavía no tiene dueño, esta persona pasa a serlo.
    pub(crate) async fn open_session(user_id: uuid::Uuid) -> Result<(), ServerFnError> {
        let pool = pool()?;
        if let Ok(tenant) = current_tenant().await {
            db::auth::claim_if_ownerless(&pool, tenant.id, user_id)
                .await
                .map_err(internal)?;
        }
        let token = db::auth::create_session(&pool, user_id)
            .await
            .map_err(internal)?;
        set_session_cookie(&token);
        Ok(())
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

    /// La tienda de la dirección de este pedido (`demo.localhost` → `demo`).
    /// La tienda sale siempre de la dirección, nunca de lo que manda el
    /// navegador.
    pub(crate) async fn current_tenant() -> Result<db::Tenant, ServerFnError> {
        match crate::site::current_site() {
            domain::Site::Platform => Err(ServerFnError::new("Esta dirección no es una tienda.")),
            domain::Site::Store(slug) => db::get_tenant_by_slug(&pool()?, &slug)
                .await
                .map_err(internal)?
                .ok_or_else(|| ServerFnError::new("No existe una tienda en esta dirección.")),
        }
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
    // En la plataforma no hay tienda que editar.
    let can_edit = match current_tenant().await {
        Ok(tenant) => db::auth::role_in(&crate::server::pool()?, tenant.id, user.id)
            .await
            .map_err(crate::server::internal)?
            .is_some(),
        Err(_) => false,
    };
    Ok(Some(Me {
        email: user.email,
        name: user.name,
        can_edit,
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
            "\n  {email} pidió un código pero no está en ALLOWED_EMAILS (hay {} habilitados). Agregalo en .env.local.\n",
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
            // Sin RESEND_API_KEY, el email (con el código) va a la terminal.
            leptos::logging::log!("\n  Código para entrar de {email}: {code}\n");
            if let Err(e) = crate::email::deliver(&crate::email::login_code(&email, &code)).await {
                leptos::logging::log!("No pudimos mandar el código a {email}: {e}");
            }
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
            // Una tienda sin dueño (la demo) queda de quien entra primero.
            open_session(user_id).await?;
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

/// Paso 1 de Entrar: ¿este email ya tiene cuenta?
#[server]
pub async fn lookup_email(email: String) -> Result<EmailLookup, ServerFnError> {
    let Some(email) = domain::normalize_email(&email) else {
        return Ok(EmailLookup::BadEmail);
    };
    let account = db::auth::account_by_email(&crate::server::pool()?, &email)
        .await
        .map_err(crate::server::internal)?;
    if !is_allowed(&email) {
        if account.is_none() {
            leptos::logging::log!(
                "\n  {email} quiso crear una cuenta pero no está en ALLOWED_EMAILS (hay {} habilitados). Agregalo en .env.local.\n",
                allowed_emails().len()
            );
        }
        return Ok(EmailLookup::NotAllowed);
    }
    Ok(match account {
        Some(a) => EmailLookup::Existing {
            name: a.name,
            has_password: a.password_hash.is_some(),
        },
        None => EmailLookup::New,
    })
}

/// Entrar con email y contraseña.
#[server]
pub async fn sign_in(
    email: String,
    password: String,
    next: String,
) -> Result<PasswordSignIn, ServerFnError> {
    use crate::server::internal;
    let Some(email) = domain::normalize_email(&email) else {
        return Ok(PasswordSignIn::Wrong);
    };
    if !is_allowed(&email) || password.chars().count() > domain::PASSWORD_MAX {
        return Ok(PasswordSignIn::Wrong);
    }
    let pool = crate::server::pool()?;
    let Some(account) = db::auth::account_by_email(&pool, &email)
        .await
        .map_err(internal)?
    else {
        return Ok(PasswordSignIn::Wrong);
    };
    if account.locked_minutes > 0 {
        return Ok(PasswordSignIn::Locked {
            minutes: account.locked_minutes as u32,
        });
    }
    let Some(hash) = account.password_hash else {
        return Ok(PasswordSignIn::NoPassword);
    };
    let ok = blocking(move || db::auth::verify_password(&password, &hash)).await?;
    if !ok {
        let locked = db::auth::record_failed_login(&pool, account.id)
            .await
            .map_err(internal)?;
        return Ok(if locked {
            PasswordSignIn::Locked {
                minutes: db::auth::LOCK_MINUTES as u32,
            }
        } else {
            PasswordSignIn::Wrong
        });
    }
    db::auth::clear_failed_logins(&pool, account.id)
        .await
        .map_err(internal)?;
    open_session(account.id).await?;
    Ok(PasswordSignIn::In {
        next: domain::safe_next(&next),
    })
}

/// Con el código que le llegó por email: crea la cuenta (con nombre) o le pone
/// una contraseña nueva, y entra. Cierra las otras sesiones de la cuenta.
#[server]
pub async fn set_password_with_code(
    email: String,
    code: String,
    name: String,
    password: String,
    next: String,
) -> Result<SetPassword, ServerFnError> {
    use crate::server::internal;
    if let Some(reason) = domain::password_problem(&password) {
        return Ok(SetPassword::Weak { reason });
    }
    let (Some(email), Some(code)) = (
        domain::normalize_email(&email),
        domain::normalize_code(&code),
    ) else {
        return Ok(SetPassword::Wrong {
            left: db::auth::CODE_ATTEMPTS as u32,
        });
    };
    if !is_allowed(&email) {
        return Ok(SetPassword::NotAllowed);
    }
    let pool = crate::server::pool()?;
    let user_id = match db::auth::check_login_code(&pool, &email, &code)
        .await
        .map_err(internal)?
    {
        db::auth::CodeResult::Wrong { left } => return Ok(SetPassword::Wrong { left }),
        db::auth::CodeResult::Expired => return Ok(SetPassword::Expired),
        db::auth::CodeResult::Ok { user_id } => user_id,
    };
    let name = domain::clean_name(&name);
    if !name.is_empty() {
        db::auth::set_name(&pool, user_id, &name)
            .await
            .map_err(internal)?;
    }
    let hash = blocking(move || db::auth::hash_password(&password)).await?;
    db::auth::set_password_hash(&pool, user_id, &hash)
        .await
        .map_err(internal)?;
    db::auth::delete_other_sessions(&pool, user_id, None)
        .await
        .map_err(internal)?;
    open_session(user_id).await?;
    Ok(SetPassword::In {
        next: domain::safe_next(&next),
    })
}

/// Lo que muestra "Mi cuenta". `None` si no entró.
#[server]
pub async fn load_account() -> Result<Option<AccountInfo>, ServerFnError> {
    let Some(user) = current_user().await? else {
        return Ok(None);
    };
    let account = db::auth::account_by_id(&crate::server::pool()?, user.id)
        .await
        .map_err(crate::server::internal)?;
    Ok(account.map(|a| AccountInfo {
        email: a.email,
        name: a.name,
        has_password: a.password_hash.is_some(),
    }))
}

/// Cambia el nombre de la cuenta. Devuelve el nombre como quedó.
#[server]
pub async fn save_name(name: String) -> Result<String, ServerFnError> {
    let user = current_user()
        .await?
        .ok_or_else(|| ServerFnError::new("Tenés que entrar con tu cuenta."))?;
    let name = domain::clean_name(&name);
    db::auth::set_name(&crate::server::pool()?, user.id, &name)
        .await
        .map_err(crate::server::internal)?;
    Ok(name)
}

/// Cambia la contraseña desde "Mi cuenta". Si la cuenta todavía no tenía,
/// `current` no se mira. Cierra las otras sesiones de la cuenta.
#[server]
pub async fn change_password(
    current: String,
    new: String,
) -> Result<PasswordChange, ServerFnError> {
    use crate::server::internal;
    let user = current_user()
        .await?
        .ok_or_else(|| ServerFnError::new("Tenés que entrar con tu cuenta."))?;
    if let Some(reason) = domain::password_problem(&new) {
        return Ok(PasswordChange::Weak { reason });
    }
    let pool = crate::server::pool()?;
    let account = db::auth::account_by_id(&pool, user.id)
        .await
        .map_err(internal)?
        .ok_or_else(|| ServerFnError::new("No encontramos tu cuenta."))?;
    if let Some(hash) = account.password_hash {
        if current.chars().count() > domain::PASSWORD_MAX
            || !blocking(move || db::auth::verify_password(&current, &hash)).await?
        {
            return Ok(PasswordChange::WrongCurrent);
        }
    }
    let hash = blocking(move || db::auth::hash_password(&new)).await?;
    db::auth::set_password_hash(&pool, user.id, &hash)
        .await
        .map_err(internal)?;
    db::auth::delete_other_sessions(&pool, user.id, session_token().as_deref())
        .await
        .map_err(internal)?;
    Ok(PasswordChange::Done)
}
