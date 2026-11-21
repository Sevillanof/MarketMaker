//! Cuentas de dueño: server functions para entrar (email y contraseña, o un
//! código por email), crear la cuenta, "Mi cuenta", salir y saber quién está
//! mirando.
//!
//! Quien está invitado (su email está en `ALLOWED_EMAILS`) crea la cuenta con
//! nombre y contraseña, sin código. El código por email queda para cuando la
//! persona se olvidó la contraseña (o prefiere no usarla), para las cuentas de
//! antes que todavía no tienen contraseña y para confirmar el email de quien
//! se registra sin invitación (`REGISTRO_ABIERTO=true`).
//!
//! Quién puede tener cuenta: si `REGISTRO_ABIERTO=true`, cualquiera; si no,
//! solo los emails de `ALLOWED_EMAILS` (una lista separada por comas en
//! `.env.local`). Las dos cosas se releen en cada pedido.

use domain::{
    AccountInfo, CodeCheck, CodeRequest, Me, PasswordChange, PasswordSignIn, SetPassword,
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

    /// Lo que se agregó y se quitó desde la Whitelist de la web (tabla
    /// `allowed_emails`). Se carga al arrancar y cada vez que cambia.
    static WEB_LIST: std::sync::LazyLock<std::sync::RwLock<(HashSet<String>, HashSet<String>)>> =
        std::sync::LazyLock::new(Default::default);

    /// Vuelve a leer la Whitelist de la base de datos.
    pub async fn refresh_allowlist(pool: &db::Pool) -> Result<(), ServerFnError> {
        let (added, removed) = db::auth::load_allowlist(pool).await.map_err(internal)?;
        if let Ok(mut list) = WEB_LIST.write() {
            *list = (added.into_iter().collect(), removed.into_iter().collect());
        }
        Ok(())
    }

    /// Emails que pueden tener cuenta: `ALLOWED_EMAILS` del servidor más los
    /// agregados desde la Whitelist, menos los quitados ahí. Los administradores
    /// siempre están.
    pub fn allowed_emails() -> HashSet<String> {
        let mut all = email_list("ALLOWED_EMAILS");
        if let Ok(list) = WEB_LIST.read() {
            all.extend(list.0.iter().cloned());
            all.retain(|e| !list.1.contains(e));
        }
        all.extend(email_list("ADMIN_EMAILS"));
        all
    }

    /// Los emails de `ALLOWED_EMAILS` / `ADMIN_EMAILS` / agregados, sin los
    /// quitados, para listarlos en la Whitelist.
    pub fn admin_emails() -> HashSet<String> {
        email_list("ADMIN_EMAILS")
    }

    /// `ADMIN_EMAILS` (en `.env.local`): quien administra MarketMaker (ve la
    /// Whitelist y el panel de uso).
    pub fn is_admin(email: &str) -> bool {
        email_list("ADMIN_EMAILS").contains(email)
    }

    fn email_list(key: &str) -> HashSet<String> {
        env_values(key)
            .join(",")
            .split([',', ';', ' '])
            .map(|e| e.trim_matches(|c| c == '"' || c == '\''))
            .filter_map(domain::normalize_email)
            .collect()
    }

    pub fn is_allowed(email: &str) -> bool {
        signup_open() || is_invited(email)
    }

    /// El email está en `ALLOWED_EMAILS`: puede crear su cuenta sin código.
    pub fn is_invited(email: &str) -> bool {
        allowed_emails().contains(email)
    }

    /// Corre `f` (cálculo pesado, como un hash de contraseña) fuera de los
    /// hilos que atienden pedidos.
    pub(crate) async fn blocking<T: Send + 'static>(
        f: impl FnOnce() -> T + Send + 'static,
    ) -> Result<T, ServerFnError> {
        tokio::task::spawn_blocking(f).await.map_err(internal)
    }

    /// Abre la sesión de `user_id` en este navegador.
    pub(crate) async fn open_session(user_id: uuid::Uuid) -> Result<(), ServerFnError> {
        let pool = pool()?;
        // Solo en desarrollo: la tienda demo recién creada no tiene dueño y
        // queda de quien entra primero. En producción una tienda nunca cambia
        // de manos por entrar en su dirección.
        if !config().secure_cookie {
            if let Ok(tenant) = current_tenant().await {
                db::auth::claim_if_ownerless(&pool, tenant.id, user_id)
                    .await
                    .map_err(internal)?;
            }
        }
        let token = db::auth::create_session(&pool, user_id)
            .await
            .map_err(internal)?;
        set_session_cookie(&token);
        Ok(())
    }

    /// Hay un servicio de emails configurado: los códigos y enlaces viajan por
    /// email y no se escriben en el registro del servidor.
    pub(crate) fn emails_go_out() -> bool {
        std::env::var("RESEND_API_KEY").is_ok_and(|k| !k.trim().is_empty())
    }

    /// Frena a quien pide emails en serie desde una misma conexión (para
    /// molestar a otra persona o gastar el cupo de emails): 6 cada 10 minutos.
    pub(crate) fn may_ask_for_email() -> bool {
        let ip = crate::limits::client_ip();
        crate::limits::allow(
            &format!("mail|{ip}"),
            6,
            std::time::Duration::from_secs(10 * 60),
        )
    }

    /// Frena a quien prueba códigos en serie desde una misma conexión: 20
    /// cada 10 minutos (cada código ya tiene sus propios 5 intentos).
    pub(crate) fn may_try_code() -> bool {
        let ip = crate::limits::client_ip();
        crate::limits::allow(
            &format!("code|{ip}"),
            20,
            std::time::Duration::from_secs(10 * 60),
        )
    }

    pub(crate) fn config() -> AuthConfig {
        use_context::<AuthConfig>().unwrap_or_default()
    }

    /// El token de la cookie de sesión del pedido actual.
    pub(crate) fn session_token() -> Option<String> {
        let parts = use_context::<http::request::Parts>()?;
        session_token_in(&parts.headers)
    }

    /// El token de la cookie de sesión que viene en estas cabeceras.
    pub(crate) fn session_token_in(headers: &http::HeaderMap) -> Option<String> {
        headers
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

/// Manda al email de la cuenta el enlace del editor de esta tienda, para abrirlo
/// desde una computadora (el editor no se usa en el teléfono).
#[server]
pub async fn send_editor_link() -> Result<(), ServerFnError> {
    let Some(user) = current_user().await? else {
        return Err(ServerFnError::new(
            "Entrá con tu cuenta para pedir el enlace.",
        ));
    };
    let tenant = current_tenant()
        .await
        .map_err(|_| ServerFnError::new("El enlace se pide desde la tienda que querés editar."))?;
    let owner = db::auth::role_in(&crate::server::pool()?, tenant.id, user.id)
        .await
        .map_err(crate::server::internal)?
        .is_some();
    if !owner {
        return Err(ServerFnError::new(
            "Esta cuenta no puede editar esta tienda.",
        ));
    }
    if !may_ask_for_email() {
        return Err(ServerFnError::new(
            "Pediste varios enlaces en poco tiempo. Probá de nuevo en unos minutos.",
        ));
    }
    let url = format!("{}/editor", crate::email::store_url(&tenant.slug));
    crate::email::deliver(&crate::email::editor_link(&user.email, &tenant.name, &url))
        .await
        .map_err(|e| {
            leptos::logging::log!("Enlace del editor a {}: no se pudo mandar: {e}", user.email);
            ServerFnError::new("No pudimos mandar el email. Probá de nuevo en unos minutos.")
        })
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
    if !may_ask_for_email() {
        return Ok(CodeRequest::TooMany);
    }
    let pool = crate::server::pool()?;
    match db::auth::create_login_code(&pool, &email)
        .await
        .map_err(crate::server::internal)?
    {
        Err(db::auth::TooMany) => Ok(CodeRequest::TooMany),
        Ok(code) => {
            // Sin RESEND_API_KEY (desarrollo), el código va a la terminal. En
            // producción nunca se escribe en el registro.
            if !emails_go_out() {
                leptos::logging::log!("\n  Código para entrar de {email}: {code}\n");
            }
            if let Err(e) = crate::email::deliver(&crate::email::login_code(&email, &code)).await {
                leptos::logging::log!("No pudimos mandar el código a {email}: {e}");
            }
            Ok(CodeRequest::Sent)
        }
    }
}

/// "¿La olvidaste?" o "Creá tu cuenta": manda por email un enlace para elegir
/// la contraseña. Es la única forma de crear una cuenta: el enlace prueba que
/// el email es de quien lo escribe.
#[server]
pub async fn request_password_link(email: String) -> Result<CodeRequest, ServerFnError> {
    let Some(email) = domain::normalize_email(&email) else {
        return Ok(CodeRequest::BadEmail);
    };
    if !may_ask_for_email() {
        return Ok(CodeRequest::TooMany);
    }
    // La respuesta es la misma tenga o no invitación: así nadie puede usar
    // este formulario para averiguar qué emails están en la lista.
    if !is_allowed(&email) {
        return Ok(CodeRequest::Sent);
    }
    let pool = crate::server::pool()?;
    // Si es cuenta nueva o no lo decide quien pregunta: lo dice la base.
    let new_account = db::auth::account_by_email(&pool, &email)
        .await
        .map_err(crate::server::internal)?
        .is_none();
    // El enlace lleva una llave larga, no un código de 6 dígitos: no se
    // puede adivinar probando.
    match db::auth::create_login_link_key(&pool, &email)
        .await
        .map_err(crate::server::internal)?
    {
        Err(db::auth::TooMany) => Ok(CodeRequest::TooMany),
        Ok(code) => {
            let url = domain::password_link(&crate::email::app_url(), &email, &code, new_account);
            // Sin RESEND_API_KEY (desarrollo), el enlace va a la terminal. En
            // producción nunca se escribe en el registro.
            if !emails_go_out() {
                leptos::logging::log!("\n  Enlace para elegir contraseña de {email}: {url}\n");
            }
            let mail = crate::email::password_link(&email, &url, new_account);
            if let Err(e) = crate::email::deliver(&mail).await {
                leptos::logging::log!("No pudimos mandar el enlace a {email}: {e}");
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
    if !is_allowed(&email) || !may_try_code() {
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

/// Sale de la cuenta: cierra todas sus sesiones, la de la plataforma y las
/// de cada tienda (cada dirección tiene su propia cookie). Así, en una
/// computadora compartida, nadie queda con el editor abierto.
#[server]
pub async fn sign_out() -> Result<(), ServerFnError> {
    use crate::server::internal;
    if let Some(token) = session_token() {
        let pool = crate::server::pool()?;
        match db::auth::session_user(&pool, &token)
            .await
            .map_err(internal)?
        {
            Some(user) => db::auth::delete_other_sessions(&pool, user.id, None)
                .await
                .map_err(internal)?,
            None => db::auth::delete_session(&pool, &token)
                .await
                .map_err(internal)?,
        }
    }
    clear_session_cookie();
    Ok(())
}

/// Borra la cuenta de quien entró y las tiendas de las que es la única
/// dueña. Hay que escribir el email de la cuenta tal cual.
#[server]
pub async fn delete_my_account(typed: String) -> Result<Result<(), String>, ServerFnError> {
    use crate::server::internal;
    let Some(user) = current_user().await? else {
        return Err(ServerFnError::new("Tenés que entrar con tu cuenta."));
    };
    if typed.trim().to_lowercase() != user.email.trim().to_lowercase() {
        return Ok(Err("El email no coincide.".into()));
    }
    if is_admin(&user.email) {
        return Ok(Err(
            "La cuenta administradora no se puede borrar desde acá.".into(),
        ));
    }
    let pool = crate::server::pool()?;
    db::auth::delete_user(&pool, user.id)
        .await
        .map_err(internal)?;
    clear_session_cookie();
    Ok(Ok(()))
}

/// Contraseñas equivocadas seguidas, desde una misma conexión y para un
/// mismo email, antes de esperar [`db::auth::LOCK_MINUTES`].
#[cfg(feature = "ssr")]
const IP_PASSWORD_ATTEMPTS: usize = 8;

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
    // Las contraseñas equivocadas se cuentan por conexión y email: quien
    // prueba desde su casa se frena, sin bloquear al dueño en la suya.
    let ip = crate::limits::client_ip();
    let tries = format!("pw|{ip}|{email}");
    let lock = std::time::Duration::from_secs(db::auth::LOCK_MINUTES as u64 * 60);
    if crate::limits::count(&tries, lock) >= IP_PASSWORD_ATTEMPTS
        || !crate::limits::allow(&format!("pw|{ip}"), 30, lock)
    {
        return Ok(PasswordSignIn::Locked {
            minutes: db::auth::LOCK_MINUTES as u32,
        });
    }
    let pool = crate::server::pool()?;
    let Some(account) = db::auth::account_by_email(&pool, &email)
        .await
        .map_err(internal)?
    else {
        crate::limits::hit(&tries);
        return Ok(PasswordSignIn::Wrong);
    };
    if account.locked_minutes > 0 {
        return Ok(PasswordSignIn::Locked {
            minutes: account.locked_minutes as u32,
        });
    }
    let Some(hash) = account.password_hash else {
        // Cuenta de antes, sin contraseña: confirma el email una vez con un
        // enlace y ahí elige la suya. Nadie estrena una contraseña sin
        // demostrar que el email es suyo, tampoco el administrador.
        return Ok(PasswordSignIn::NoPassword);
    };
    let ok = blocking(move || db::auth::verify_password(&password, &hash)).await?;
    if !ok {
        crate::limits::hit(&tries);
        if crate::limits::count(&tries, lock) >= IP_PASSWORD_ATTEMPTS {
            return Ok(PasswordSignIn::Locked {
                minutes: db::auth::LOCK_MINUTES as u32,
            });
        }
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
    crate::limits::clear(&tries);
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
    if !may_try_code() {
        return Ok(SetPassword::Expired);
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
    let pool = crate::server::pool()?;
    let account = db::auth::account_by_id(&pool, user.id)
        .await
        .map_err(crate::server::internal)?;
    let stores = db::auth::stores_for_user(&pool, user.id)
        .await
        .map_err(crate::server::internal)?
        .len() as u32;
    Ok(account.map(|a| AccountInfo {
        email: a.email,
        name: a.name,
        has_password: a.password_hash.is_some(),
        avatar: a.avatar_id.map(domain::avatar_url),
        stores,
    }))
}

/// Cambia el nombre de la cuenta. Devuelve el nombre como quedó.
#[server]
pub async fn save_name(name: String) -> Result<String, ServerFnError> {
    let user = current_user()
        .await?
        .ok_or_else(|| ServerFnError::new("Tenés que entrar con tu cuenta."))?;
    let name = domain::clean_name(&name);
    if name.is_empty() {
        return Err(ServerFnError::new("Escribí tu nombre."));
    }
    db::auth::set_name(&crate::server::pool()?, user.id, &name)
        .await
        .map_err(crate::server::internal)?;
    Ok(name)
}

/// Pone una foto de Mis fotos (`media_id`) como foto de perfil, o la
/// quita con `None`.
#[server]
pub async fn save_avatar(media_id: Option<String>) -> Result<(), ServerFnError> {
    let user = current_user()
        .await?
        .ok_or_else(|| ServerFnError::new("Tenés que entrar con tu cuenta."))?;
    let id = match media_id {
        Some(s) => Some(
            uuid::Uuid::parse_str(s.trim())
                .map_err(|_| ServerFnError::new("No encontramos esa foto."))?,
        ),
        None => None,
    };
    let ok = db::auth::set_avatar(&crate::server::pool()?, user.id, id)
        .await
        .map_err(crate::server::internal)?;
    if ok {
        Ok(())
    } else {
        Err(ServerFnError::new("No encontramos esa foto en tu cuenta."))
    }
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
        let same = (new.clone(), hash.clone());
        if current.chars().count() > domain::PASSWORD_MAX
            || !blocking(move || db::auth::verify_password(&current, &hash)).await?
        {
            return Ok(PasswordChange::WrongCurrent);
        }
        if blocking(move || db::auth::verify_password(&same.0, &same.1)).await? {
            return Ok(PasswordChange::Weak {
                reason: "La contraseña nueva tiene que ser distinta de la actual.".into(),
            });
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
