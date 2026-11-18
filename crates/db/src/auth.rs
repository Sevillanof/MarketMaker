//! Cuentas de dueño: códigos para entrar, sesiones y roles por tienda.
//!
//! Nada sensible se guarda en claro: de los códigos y de los tokens de sesión
//! la base guarda solo el hash SHA-256, y de las contraseñas un hash Argon2id
//! con sal propia.

use rand::{Rng, RngCore};
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use uuid::Uuid;

/// Cuánto dura un código.
pub const CODE_MINUTES: i32 = 10;
/// Intentos por código antes de tener que pedir otro.
pub const CODE_ATTEMPTS: i32 = 5;
/// Códigos por email y por hora.
pub const CODES_PER_HOUR: i64 = 5;
/// Códigos por email y por día: frena a quien los pide sin parar para
/// adivinar uno o para llenarle la casilla a otra persona.
pub const CODES_PER_DAY: i64 = 15;
/// Cuánto dura una sesión sin usarse.
pub const SESSION_DAYS: i32 = 30;
/// Cuánto dura una sesión como mucho, aunque se use todos los días: pasado
/// ese tiempo hay que entrar de nuevo con la contraseña.
pub const SESSION_MAX_DAYS: i32 = 180;

fn code_hash(email: &str, code: &str) -> Vec<u8> {
    Sha256::digest(format!("{email}:{code}")).to_vec()
}

fn token_hash(token: &str) -> Vec<u8> {
    Sha256::digest(token.as_bytes()).to_vec()
}

/// Seis dígitos al azar, con ceros a la izquierda.
pub fn new_code() -> String {
    format!("{:06}", rand::rng().random_range(0..1_000_000u32))
}

/// La llave de un enlace de email (elegir contraseña, confirmar la cuenta):
/// 16 bytes al azar en hexadecimal. A diferencia del código de 6 dígitos,
/// que se copia a mano, no se puede adivinar probando.
pub fn new_link_key() -> String {
    let mut bytes = [0u8; 16];
    rand::rng().fill_bytes(&mut bytes);
    hex::encode(bytes)
}

/// 32 bytes al azar en hexadecimal: lo que lleva la cookie.
pub fn new_token() -> String {
    let mut bytes = [0u8; 32];
    rand::rng().fill_bytes(&mut bytes);
    hex::encode(bytes)
}

/// Se pidieron demasiados códigos para este email en la última hora.
#[derive(Debug, PartialEq)]
pub struct TooMany;

/// Crea un código nuevo para `email` (ya normalizado) y lo devuelve en claro
/// para mandarlo por email. Los códigos anteriores sin usar dejan de servir.
pub async fn create_login_code(
    pool: &PgPool,
    email: &str,
) -> sqlx::Result<Result<String, TooMany>> {
    save_login_code(pool, email, new_code()).await
}

/// Como [`create_login_code`], pero con una llave larga para llevar dentro
/// de un enlace (nadie la escribe a mano).
pub async fn create_login_link_key(
    pool: &PgPool,
    email: &str,
) -> sqlx::Result<Result<String, TooMany>> {
    save_login_code(pool, email, new_link_key()).await
}

/// Cuánto dura el enlace de una invitación.
pub const INVITE_DAYS: i32 = 7;

/// La llave del enlace de invitación de `email` (ya normalizado): sirve una
/// vez, por [`INVITE_DAYS`] días, para crear la cuenta. La pide el
/// administrador al invitar, así que no cuenta para los topes por hora, y
/// pedir otro enlace desde Entrar no la anula.
pub async fn create_invite_key(pool: &PgPool, email: &str) -> sqlx::Result<String> {
    let key = new_link_key();
    let mut tx = pool.begin().await?;
    // Una invitación anterior de este email deja de servir.
    sqlx::query(
        "UPDATE login_codes SET used_at = now()
         WHERE email = $1 AND used_at IS NULL AND expires_at - created_at > interval '1 hour'",
    )
    .bind(email)
    .execute(&mut *tx)
    .await?;
    sqlx::query(
        "INSERT INTO login_codes (email, code_hash, expires_at)
         VALUES ($1, $2, now() + make_interval(days => $3))",
    )
    .bind(email)
    .bind(code_hash(email, &key))
    .bind(INVITE_DAYS)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(key)
}

/// Cuántos segundos hace que se le mandó la última invitación a cada email
/// de `emails` (los que nunca recibieron una no aparecen).
pub async fn invitations_sent(
    pool: &PgPool,
    emails: &[String],
) -> sqlx::Result<Vec<(String, i64)>> {
    sqlx::query_as(
        "SELECT email, extract(epoch FROM now() - max(created_at))::bigint
         FROM login_codes
         WHERE email = ANY($1) AND expires_at - created_at > interval '1 hour'
         GROUP BY email",
    )
    .bind(emails)
    .fetch_all(pool)
    .await
}

async fn save_login_code(
    pool: &PgPool,
    email: &str,
    code: String,
) -> sqlx::Result<Result<String, TooMany>> {
    // Las invitaciones (duran días) no cuentan: las manda el administrador.
    let (hour, day): (i64, i64) = sqlx::query_as(
        "SELECT count(*) FILTER (WHERE created_at > now() - interval '1 hour'), count(*)
         FROM login_codes
         WHERE email = $1 AND created_at > now() - interval '1 day'
           AND expires_at - created_at <= interval '1 hour'",
    )
    .bind(email)
    .fetch_one(pool)
    .await?;
    if hour >= CODES_PER_HOUR || day >= CODES_PER_DAY {
        return Ok(Err(TooMany));
    }
    let mut tx = pool.begin().await?;
    // Los códigos anteriores dejan de servir; una invitación sigue valiendo.
    sqlx::query(
        "UPDATE login_codes SET used_at = now()
         WHERE email = $1 AND used_at IS NULL AND expires_at - created_at <= interval '1 hour'",
    )
    .bind(email)
    .execute(&mut *tx)
    .await?;
    sqlx::query(
        "INSERT INTO login_codes (email, code_hash, expires_at)
         VALUES ($1, $2, now() + make_interval(mins => $3))",
    )
    .bind(email)
    .bind(code_hash(email, &code))
    .bind(CODE_MINUTES)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(Ok(code))
}

/// Resultado de revisar un código.
#[derive(Debug, PartialEq)]
pub enum CodeResult {
    /// Era el código: la cuenta existe (se crea la primera vez).
    Ok { user_id: Uuid },
    /// No era. Quedan `left` intentos.
    Wrong { left: u32 },
    /// No hay un código vigente: venció, se usó o se gastaron los intentos.
    Expired,
}

/// Revisa `code` contra los códigos vigentes de `email`: el último que pidió
/// o la llave de su invitación. Si es correcto lo marca como usado y devuelve
/// la cuenta, creándola si es la primera vez.
pub async fn check_login_code(pool: &PgPool, email: &str, code: &str) -> sqlx::Result<CodeResult> {
    let mut tx = pool.begin().await?;
    let hit: Option<Uuid> = sqlx::query_scalar(
        "SELECT id FROM login_codes
         WHERE email = $1 AND code_hash = $2 AND used_at IS NULL AND expires_at > now()
           AND attempts < $3
         ORDER BY created_at DESC LIMIT 1
         FOR UPDATE",
    )
    .bind(email)
    .bind(code_hash(email, code))
    .bind(CODE_ATTEMPTS)
    .fetch_optional(&mut *tx)
    .await?;
    let Some(id) = hit else {
        // No era: el intento se anota en el último código pedido. La llave de
        // una invitación es larga, no se adivina probando y no lleva cuenta
        // (así nadie la gasta a propósito con intentos equivocados).
        let row: Option<(Uuid, i32)> = sqlx::query_as(
            "SELECT id, attempts FROM login_codes
             WHERE email = $1 AND used_at IS NULL AND expires_at > now()
               AND expires_at - created_at <= interval '1 hour'
             ORDER BY created_at DESC LIMIT 1
             FOR UPDATE",
        )
        .bind(email)
        .fetch_optional(&mut *tx)
        .await?;
        let Some((id, attempts)) = row else {
            return Ok(CodeResult::Expired);
        };
        if attempts >= CODE_ATTEMPTS {
            return Ok(CodeResult::Expired);
        }
        let attempts = attempts + 1;
        sqlx::query("UPDATE login_codes SET attempts = $2 WHERE id = $1")
            .bind(id)
            .bind(attempts)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        let left = u32::try_from(CODE_ATTEMPTS - attempts).unwrap_or(0);
        return Ok(if left == 0 {
            CodeResult::Expired
        } else {
            CodeResult::Wrong { left }
        });
    };
    sqlx::query("UPDATE login_codes SET used_at = now() WHERE id = $1")
        .bind(id)
        .execute(&mut *tx)
        .await?;
    let user_id: Uuid = sqlx::query_scalar(
        "INSERT INTO users (email) VALUES ($1)
         ON CONFLICT (email) DO UPDATE SET email = EXCLUDED.email
         RETURNING id",
    )
    .bind(email)
    .fetch_one(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(CodeResult::Ok { user_id })
}

/// Abre una sesión y devuelve el token para la cookie.
pub async fn create_session(pool: &PgPool, user_id: Uuid) -> sqlx::Result<String> {
    let token = new_token();
    sqlx::query(
        "INSERT INTO sessions (token_hash, user_id, expires_at)
         VALUES ($1, $2, now() + make_interval(days => $3))",
    )
    .bind(token_hash(&token))
    .bind(user_id)
    .bind(SESSION_DAYS)
    .execute(pool)
    .await?;
    Ok(token)
}

/// La persona de una sesión vigente.
#[derive(Debug, Clone, PartialEq, sqlx::FromRow)]
pub struct SessionUser {
    pub id: Uuid,
    pub email: String,
    /// Cómo quiere que la llamemos ("" si no lo dijo).
    pub name: String,
    /// La foto de perfil (una de Mis fotos), si eligió una.
    pub avatar_id: Option<Uuid>,
}

/// La cuenta de la sesión `token`, si sigue vigente. Usarla la extiende
/// otros [`SESSION_DAYS`] días.
pub async fn session_user(pool: &PgPool, token: &str) -> sqlx::Result<Option<SessionUser>> {
    sqlx::query_as::<_, SessionUser>(
        "UPDATE sessions s SET expires_at = now() + make_interval(days => $2)
         FROM users u
         WHERE s.token_hash = $1 AND s.expires_at > now() AND u.id = s.user_id
           AND s.created_at > now() - make_interval(days => $3)
         RETURNING u.id, u.email, u.name, u.avatar_id",
    )
    .bind(token_hash(token))
    .bind(SESSION_DAYS)
    .bind(SESSION_MAX_DAYS)
    .fetch_optional(pool)
    .await
}

/// Cierra la sesión `token`.
pub async fn delete_session(pool: &PgPool, token: &str) -> sqlx::Result<()> {
    sqlx::query("DELETE FROM sessions WHERE token_hash = $1")
        .bind(token_hash(token))
        .execute(pool)
        .await?;
    Ok(())
}

/// El rol de la persona en la tienda (`"owner"`), si tiene alguno.
pub async fn role_in(
    pool: &PgPool,
    tenant_id: Uuid,
    user_id: Uuid,
) -> sqlx::Result<Option<String>> {
    sqlx::query_scalar("SELECT role FROM memberships WHERE tenant_id = $1 AND user_id = $2")
        .bind(tenant_id)
        .bind(user_id)
        .fetch_optional(pool)
        .await
}

/// Si la tienda todavía no tiene dueño, la persona pasa a serlo.
pub async fn claim_if_ownerless(pool: &PgPool, tenant_id: Uuid, user_id: Uuid) -> sqlx::Result<()> {
    sqlx::query(
        "INSERT INTO memberships (tenant_id, user_id, role)
         SELECT $1, $2, 'owner'
         WHERE NOT EXISTS (SELECT 1 FROM memberships WHERE tenant_id = $1)
         ON CONFLICT DO NOTHING",
    )
    .bind(tenant_id)
    .bind(user_id)
    .execute(pool)
    .await?;
    Ok(())
}

/// Una tienda de la persona.
#[derive(Debug, Clone, PartialEq, sqlx::FromRow)]
pub struct StoreRow {
    pub slug: String,
    pub name: String,
    pub role: String,
}

/// Las tiendas donde la persona tiene un rol, la más nueva primero.
pub async fn stores_for_user(pool: &PgPool, user_id: Uuid) -> sqlx::Result<Vec<StoreRow>> {
    sqlx::query_as::<_, StoreRow>(
        "SELECT t.slug, t.name, m.role
         FROM memberships m JOIN tenants t ON t.id = m.tenant_id
         WHERE m.user_id = $1
         ORDER BY t.created_at DESC, t.slug",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
}

/// La dirección ya la usa otra tienda.
pub async fn slug_taken(pool: &PgPool, slug: &str) -> sqlx::Result<bool> {
    sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM tenants WHERE slug = $1)")
        .bind(slug)
        .fetch_one(pool)
        .await
}

/// Crea la tienda `slug` con la persona como dueña. Arranca con el diseño
/// `doc` guardado pero **sin publicar**, y con los productos de la tienda
/// `template` copiados como ejemplo (los compradores no los ven), así el
/// diseño no se ve vacío. Devuelve `false` si la dirección ya estaba tomada.
pub async fn create_store(
    pool: &PgPool,
    name: &str,
    slug: &str,
    owner: Uuid,
    template: &str,
    doc: &serde_json::Value,
) -> sqlx::Result<bool> {
    let mut tx = pool.begin().await?;
    let tenant: Option<Uuid> = sqlx::query_scalar(
        "INSERT INTO tenants (slug, name, created_by, pay_coordinate) VALUES ($1, $2, $3, false)
         ON CONFLICT (slug) DO NOTHING
         RETURNING id",
    )
    .bind(slug)
    .bind(name)
    .bind(owner)
    .fetch_optional(&mut *tx)
    .await?;
    let Some(tenant) = tenant else {
        return Ok(false);
    };
    sqlx::query("INSERT INTO memberships (tenant_id, user_id, role) VALUES ($1, $2, 'owner')")
        .bind(tenant)
        .bind(owner)
        .execute(&mut *tx)
        .await?;
    let page: Uuid =
        sqlx::query_scalar("INSERT INTO pages (tenant_id, slug) VALUES ($1, 'home') RETURNING id")
            .bind(tenant)
            .fetch_one(&mut *tx)
            .await?;
    // La versión 1 es el diseño inicial: queda vigente, no publicada.
    let version: Uuid = sqlx::query_scalar(
        "INSERT INTO page_versions (page_id, tenant_id, number, doc, created_by)
         VALUES ($1, $2, 1, $3, $4)
         RETURNING id",
    )
    .bind(page)
    .bind(tenant)
    .bind(doc)
    .bind(owner)
    .fetch_one(&mut *tx)
    .await?;
    sqlx::query("UPDATE pages SET current_version_id = $2 WHERE id = $1")
        .bind(page)
        .bind(version)
        .execute(&mut *tx)
        .await?;
    sqlx::query(
        "INSERT INTO products (tenant_id, name, price_cents, image_url, description, created_at,
                               compare_at_cents, in_stock, visible, photos, tags, photo_fits,
                               example)
         SELECT $1, p.name, p.price_cents, p.image_url, p.description, p.created_at,
                p.compare_at_cents, p.in_stock, true, p.photos, p.tags, p.photo_fits, true
         FROM products p JOIN tenants t ON t.id = p.tenant_id
         WHERE t.slug = $2 AND p.visible",
    )
    .bind(tenant)
    .bind(template)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(true)
}

/// Cuánto dura un pase para pasar de la plataforma a una tienda.
pub const HANDOFF_SECONDS: i32 = 60;

/// Crea un pase de un solo uso para que la persona abra sesión en la tienda.
pub async fn create_handoff(pool: &PgPool, user_id: Uuid, tenant_id: Uuid) -> sqlx::Result<String> {
    let token = new_token();
    sqlx::query(
        "INSERT INTO login_handoffs (token_hash, user_id, tenant_id, expires_at)
         VALUES ($1, $2, $3, now() + make_interval(secs => $4))",
    )
    .bind(token_hash(&token))
    .bind(user_id)
    .bind(tenant_id)
    .bind(f64::from(HANDOFF_SECONDS))
    .execute(pool)
    .await?;
    Ok(token)
}

/// Usa el pase en la tienda `tenant_id`: devuelve la persona si es válido,
/// es para esa tienda, no venció y no se usó.
pub async fn redeem_handoff(
    pool: &PgPool,
    token: &str,
    tenant_id: Uuid,
) -> sqlx::Result<Option<Uuid>> {
    sqlx::query_scalar(
        "UPDATE login_handoffs SET used_at = now()
         WHERE token_hash = $1 AND tenant_id = $2 AND used_at IS NULL AND expires_at > now()
         RETURNING user_id",
    )
    .bind(token_hash(token))
    .bind(tenant_id)
    .fetch_optional(pool)
    .await
}

// ---------------------------------------------------------------------------
// Contraseñas

/// Intentos fallidos seguidos, sumando todas las conexiones, antes de que la
/// cuenta espere. Es alto a propósito: el freno común es por conexión (ver
/// `web::limits`), así nadie puede dejar afuera al dueño desde otra casa;
/// este solo frena un ataque repartido entre muchas conexiones.
pub const PASSWORD_ATTEMPTS: i32 = 40;
/// Cuánto espera la cuenta después de [`PASSWORD_ATTEMPTS`] fallos.
pub const LOCK_MINUTES: i32 = 15;

/// Hash Argon2id de `password` con una sal al azar, en formato PHC
/// (`$argon2id$v=19$...`). Tarda unas decenas de milisegundos a propósito:
/// llamarlo fuera del hilo del servidor (`spawn_blocking`).
pub fn hash_password(password: &str) -> String {
    use argon2::password_hash::{PasswordHasher, SaltString};
    let mut salt = [0u8; 16];
    rand::rng().fill_bytes(&mut salt);
    let salt = SaltString::encode_b64(&salt).expect("16 bytes es una sal válida");
    argon2::Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .expect("Argon2 con parámetros por defecto no falla")
        .to_string()
}

/// `password` corresponde a `hash` (hecho con [`hash_password`]).
pub fn verify_password(password: &str, hash: &str) -> bool {
    use argon2::password_hash::{PasswordHash, PasswordVerifier};
    PasswordHash::new(hash).is_ok_and(|parsed| {
        argon2::Argon2::default()
            .verify_password(password.as_bytes(), &parsed)
            .is_ok()
    })
}

/// Una cuenta buscada por email.
#[derive(Debug, Clone, PartialEq, sqlx::FromRow)]
pub struct Account {
    pub id: Uuid,
    pub email: String,
    pub name: String,
    pub password_hash: Option<String>,
    pub avatar_id: Option<Uuid>,
    /// Minutos que faltan para poder volver a probar la contraseña (0 si puede).
    pub locked_minutes: i32,
}

/// La cuenta de `email` (ya normalizado), si existe.
pub async fn account_by_email(pool: &PgPool, email: &str) -> sqlx::Result<Option<Account>> {
    sqlx::query_as::<_, Account>(
        "SELECT id, email, name, password_hash, avatar_id,
                GREATEST(0, CEIL(EXTRACT(EPOCH FROM (locked_until - now())) / 60))::int AS locked_minutes
         FROM users WHERE email = $1",
    )
    .bind(email)
    .fetch_optional(pool)
    .await
}

/// La cuenta `id`.
pub async fn account_by_id(pool: &PgPool, id: Uuid) -> sqlx::Result<Option<Account>> {
    sqlx::query_as::<_, Account>(
        "SELECT id, email, name, password_hash, avatar_id,
                GREATEST(0, CEIL(EXTRACT(EPOCH FROM (locked_until - now())) / 60))::int AS locked_minutes
         FROM users WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
}

/// Anota un intento fallido. Al llegar a [`PASSWORD_ATTEMPTS`] la cuenta
/// espera [`LOCK_MINUTES`] y el contador vuelve a cero. Devuelve `true` si
/// este intento la dejó esperando.
pub async fn record_failed_login(pool: &PgPool, id: Uuid) -> sqlx::Result<bool> {
    sqlx::query_scalar(
        "UPDATE users SET
           failed_logins = CASE WHEN failed_logins + 1 >= $2 THEN 0 ELSE failed_logins + 1 END,
           locked_until  = CASE WHEN failed_logins + 1 >= $2
                                THEN now() + make_interval(mins => $3) ELSE locked_until END
         WHERE id = $1
         RETURNING locked_until IS NOT NULL AND locked_until > now()",
    )
    .bind(id)
    .bind(PASSWORD_ATTEMPTS)
    .bind(LOCK_MINUTES)
    .fetch_one(pool)
    .await
}

/// Entró bien: se olvidan los intentos fallidos.
pub async fn clear_failed_logins(pool: &PgPool, id: Uuid) -> sqlx::Result<()> {
    sqlx::query("UPDATE users SET failed_logins = 0, locked_until = NULL WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

/// Guarda la contraseña nueva (ya en hash) y destraba la cuenta.
pub async fn set_password_hash(pool: &PgPool, id: Uuid, hash: &str) -> sqlx::Result<()> {
    sqlx::query(
        "UPDATE users SET password_hash = $2, failed_logins = 0, locked_until = NULL WHERE id = $1",
    )
    .bind(id)
    .bind(hash)
    .execute(pool)
    .await?;
    Ok(())
}

/// Crea la cuenta de `email` (ya normalizado) con nombre y contraseña (ya en
/// hash), sin código: es para emails invitados. Devuelve `None` si ese email
/// ya tiene cuenta, para que entre con su contraseña en vez de pisarla.
pub async fn create_account(
    pool: &PgPool,
    email: &str,
    name: &str,
    password_hash: &str,
) -> sqlx::Result<Option<Uuid>> {
    sqlx::query_scalar(
        "INSERT INTO users (email, name, password_hash) VALUES ($1, $2, $3)
         ON CONFLICT (email) DO NOTHING
         RETURNING id",
    )
    .bind(email)
    .bind(name)
    .bind(password_hash)
    .fetch_optional(pool)
    .await
}

/// Cambia el nombre de la cuenta.
pub async fn set_name(pool: &PgPool, id: Uuid, name: &str) -> sqlx::Result<()> {
    sqlx::query("UPDATE users SET name = $2 WHERE id = $1")
        .bind(id)
        .bind(name)
        .execute(pool)
        .await?;
    Ok(())
}

/// Cierra todas las sesiones de la cuenta menos la de `keep` (si hay).
/// Se usa al cambiar la contraseña: quien la tenía robada queda afuera.
pub async fn delete_other_sessions(
    pool: &PgPool,
    user_id: Uuid,
    keep: Option<&str>,
) -> sqlx::Result<()> {
    sqlx::query("DELETE FROM sessions WHERE user_id = $1 AND token_hash IS DISTINCT FROM $2")
        .bind(user_id)
        .bind(keep.map(token_hash))
        .execute(pool)
        .await?;
    Ok(())
}

/// Pone (o quita, con `None`) la foto de perfil. Solo acepta una foto de
/// Mis fotos de la misma cuenta. Devuelve `false` si la foto no es suya.
pub async fn set_avatar(
    pool: &PgPool,
    user_id: Uuid,
    media_id: Option<Uuid>,
) -> sqlx::Result<bool> {
    let done = sqlx::query(
        "UPDATE users SET avatar_id = $2
         WHERE id = $1
           AND ($2::uuid IS NULL OR EXISTS (SELECT 1 FROM media WHERE id = $2 AND owner_id = $1))",
    )
    .bind(user_id)
    .bind(media_id)
    .execute(pool)
    .await?;
    Ok(done.rows_affected() == 1)
}

/// Nombre, logo y contacto de la tienda.
pub async fn store_info(pool: &PgPool, tenant: Uuid) -> sqlx::Result<domain::StoreInfo> {
    let (name, logo, whatsapp, instagram, email): (String, Option<Uuid>, String, String, String) =
        sqlx::query_as(
            "SELECT name, logo_id, whatsapp, instagram, contact_email FROM tenants WHERE id = $1",
        )
        .bind(tenant)
        .fetch_one(pool)
        .await?;
    Ok(domain::StoreInfo {
        name,
        logo_url: logo.map(domain::avatar_url).unwrap_or_default(),
        whatsapp,
        instagram,
        email,
    })
}

/// Guarda los datos de la tienda (ya limpios con `domain::tidy_store_info`).
/// El logo tiene que ser una foto de `user`; `false` si no lo es.
pub async fn save_store_info(
    pool: &PgPool,
    tenant: Uuid,
    user: Uuid,
    info: &domain::StoreInfo,
    logo: Option<Uuid>,
) -> sqlx::Result<bool> {
    let done = sqlx::query(
        "UPDATE tenants SET name = $3, logo_id = $4, whatsapp = $5, instagram = $6,
                contact_email = $7
         WHERE id = $1
           AND ($4::uuid IS NULL OR EXISTS (SELECT 1 FROM media WHERE id = $4 AND owner_id = $2))",
    )
    .bind(tenant)
    .bind(user)
    .bind(&info.name)
    .bind(logo)
    .bind(&info.whatsapp)
    .bind(&info.instagram)
    .bind(&info.email)
    .execute(pool)
    .await?;
    Ok(done.rows_affected() == 1)
}

/// El id del logo de la tienda, si tiene.
pub async fn store_logo_id(pool: &PgPool, tenant: Uuid) -> sqlx::Result<Option<Uuid>> {
    sqlx::query_scalar("SELECT logo_id FROM tenants WHERE id = $1")
        .bind(tenant)
        .fetch_one(pool)
        .await
}

// ---------------------------------------------------------------------------
// Whitelist: quién puede tener cuenta (se suma a ALLOWED_EMAILS).
// ---------------------------------------------------------------------------

/// Los emails agregados y los quitados desde la web.
pub async fn load_allowlist(pool: &PgPool) -> sqlx::Result<(Vec<String>, Vec<String>)> {
    let rows: Vec<(String, String)> = sqlx::query_as("SELECT email, status FROM allowed_emails")
        .fetch_all(pool)
        .await?;
    let mut allowed = Vec::new();
    let mut removed = Vec::new();
    for (email, status) in rows {
        if status == "allowed" {
            allowed.push(email);
        } else {
            removed.push(email);
        }
    }
    Ok((allowed, removed))
}

/// Habilita `email` (ya normalizado). Si estaba quitado, vuelve a entrar.
pub async fn allow_email(pool: &PgPool, email: &str, by: Uuid) -> sqlx::Result<()> {
    sqlx::query(
        "INSERT INTO allowed_emails (email, status, added_by) VALUES ($1, 'allowed', $2)
         ON CONFLICT (email) DO UPDATE
           SET status = 'allowed', added_by = $2, created_at = now(), updated_at = now()",
    )
    .bind(email)
    .bind(by)
    .execute(pool)
    .await?;
    Ok(())
}

/// Quita `email` de la lista. Queda anotado para que tampoco valga si viene
/// de `ALLOWED_EMAILS`.
pub async fn disallow_email(pool: &PgPool, email: &str, by: Uuid) -> sqlx::Result<()> {
    sqlx::query(
        "INSERT INTO allowed_emails (email, status, added_by) VALUES ($1, 'removed', $2)
         ON CONFLICT (email) DO UPDATE
           SET status = 'removed', added_by = $2, updated_at = now()",
    )
    .bind(email)
    .bind(by)
    .execute(pool)
    .await?;
    Ok(())
}

/// Una persona de la lista con lo que sabemos de su cuenta.
#[derive(Debug, Clone, PartialEq)]
pub struct AllowlistMember {
    pub email: String,
    pub name: String,
    pub avatar_id: Option<Uuid>,
    /// Tiene cuenta.
    pub has_account: bool,
    /// Segundos desde que creó la cuenta / desde que entró por última vez.
    pub joined: Option<i64>,
    pub last_seen: Option<i64>,
    /// Cuándo la agregaron a la lista (solo si fue desde la web).
    pub added: Option<i64>,
    /// (slug, nombre)
    pub stores: Vec<(String, String)>,
    pub orders: i64,
}

#[derive(sqlx::FromRow)]
struct MemberRow {
    email: String,
    name: Option<String>,
    avatar_id: Option<Uuid>,
    user_id: Option<Uuid>,
    joined: Option<i64>,
    last_seen: Option<i64>,
    added: Option<i64>,
}

/// Datos de cada email de `emails`, en el mismo orden.
pub async fn allowlist_members(
    pool: &PgPool,
    emails: &[String],
) -> sqlx::Result<Vec<AllowlistMember>> {
    let rows: Vec<MemberRow> = sqlx::query_as(
        "SELECT e.email, u.name, u.avatar_id, u.id AS user_id, extract(epoch FROM now() - u.created_at)::bigint AS joined,
                (SELECT extract(epoch FROM now() - max(s.created_at))::bigint FROM sessions s WHERE s.user_id = u.id) AS last_seen,
                extract(epoch FROM now() - a.created_at)::bigint AS added
         FROM unnest($1::text[]) WITH ORDINALITY AS e(email, n)
         LEFT JOIN users u ON u.email = e.email
         LEFT JOIN allowed_emails a ON a.email = e.email AND a.status = 'allowed'
         ORDER BY e.n",
    )
    .bind(emails)
    .fetch_all(pool)
    .await?;
    let ids: Vec<Uuid> = rows.iter().filter_map(|r| r.user_id).collect();
    let stores: Vec<(Uuid, String, String)> = sqlx::query_as(
        "SELECT m.user_id, t.slug, t.name FROM memberships m
         JOIN tenants t ON t.id = m.tenant_id
         WHERE m.user_id = ANY($1) ORDER BY t.created_at DESC, t.slug",
    )
    .bind(&ids)
    .fetch_all(pool)
    .await?;
    let orders: Vec<(Uuid, i64)> = sqlx::query_as(
        "SELECT m.user_id, count(o.id) FROM memberships m
         JOIN orders o ON o.tenant_id = m.tenant_id
           AND o.status IN ('paid', 'shipped', 'delivered')
         WHERE m.user_id = ANY($1) GROUP BY m.user_id",
    )
    .bind(&ids)
    .fetch_all(pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(|r| AllowlistMember {
            has_account: r.user_id.is_some(),
            name: r.name.unwrap_or_default(),
            avatar_id: r.avatar_id,
            joined: r.joined,
            last_seen: r.last_seen,
            added: r.added,
            stores: stores
                .iter()
                .filter(|(u, ..)| Some(*u) == r.user_id)
                .map(|(_, s, n)| (s.clone(), n.clone()))
                .collect(),
            orders: orders
                .iter()
                .find(|(u, _)| Some(*u) == r.user_id)
                .map_or(0, |(_, n)| *n),
            email: r.email,
        })
        .collect())
}
