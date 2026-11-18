//! Las llaves de terceros (Mercado Pago, Andreani) se guardan cifradas: una
//! copia de la base sola no alcanza para usarlas. La clave vive fuera de la
//! base, en el servidor.
//!
//! Lo cifrado empieza con `enc1:`; lo que no, se lee tal cual (así se
//! guardaba antes, y así queda si el servidor arranca sin clave).

use chacha20poly1305::aead::{Aead, KeyInit};
use chacha20poly1305::{Key, XChaCha20Poly1305, XNonce};
use rand::RngCore;
use sqlx::PgPool;
use std::sync::OnceLock;

const PREFIX: &str = "enc1:";
const NONCE: usize = 24;

static KEY: OnceLock<[u8; 32]> = OnceLock::new();

/// Deja la clave lista para todo el programa. Solo vale la primera.
pub fn init(key: [u8; 32]) {
    _ = KEY.set(key);
}

/// Lee una clave escrita en hexadecimal (64 caracteres).
pub fn parse_key(text: &str) -> Option<[u8; 32]> {
    hex::decode(text.trim()).ok()?.try_into().ok()
}

/// La clave del archivo `path`; si no existe, crea una al azar y la guarda
/// para que solo la lea el usuario del servidor.
pub fn load_or_create_key(path: &std::path::Path) -> std::io::Result<[u8; 32]> {
    use std::io::{Error, ErrorKind, Write};
    match std::fs::read_to_string(path) {
        Ok(text) => parse_key(&text)
            .ok_or_else(|| Error::new(ErrorKind::InvalidData, "la clave guardada no es válida")),
        Err(e) if e.kind() == ErrorKind::NotFound => {
            let mut key = [0u8; 32];
            rand::rng().fill_bytes(&mut key);
            let mut options = std::fs::OpenOptions::new();
            options.write(true).create_new(true);
            #[cfg(unix)]
            {
                use std::os::unix::fs::OpenOptionsExt;
                options.mode(0o600);
            }
            let mut file = options.open(path)?;
            file.write_all(hex::encode(key).as_bytes())?;
            file.sync_all()?;
            Ok(key)
        }
        Err(e) => Err(e),
    }
}

fn seal_with(key: &[u8; 32], plain: &str) -> String {
    let mut nonce = [0u8; NONCE];
    rand::rng().fill_bytes(&mut nonce);
    let cipher = XChaCha20Poly1305::new(Key::from_slice(key));
    match cipher.encrypt(XNonce::from_slice(&nonce), plain.as_bytes()) {
        Ok(sealed) => {
            let mut out = nonce.to_vec();
            out.extend(sealed);
            format!("{PREFIX}{}", hex::encode(out))
        }
        Err(_) => plain.to_string(),
    }
}

fn open_with(key: &[u8; 32], stored: &str) -> Option<String> {
    let bytes = hex::decode(stored.strip_prefix(PREFIX)?).ok()?;
    if bytes.len() < NONCE {
        return None;
    }
    let (nonce, sealed) = bytes.split_at(NONCE);
    let cipher = XChaCha20Poly1305::new(Key::from_slice(key));
    let plain = cipher.decrypt(XNonce::from_slice(nonce), sealed).ok()?;
    String::from_utf8(plain).ok()
}

/// Cómo se guarda `plain`: cifrado si hay clave; si no (o si está vacío), igual.
pub fn seal(plain: &str) -> String {
    match KEY.get() {
        Some(key) if !plain.is_empty() && !plain.starts_with(PREFIX) => seal_with(key, plain),
        _ => plain.to_string(),
    }
}

/// Lo guardado, en claro. Si estaba cifrado y no se puede abrir (falta la
/// clave o es otra) queda vacío: esa cuenta hay que vincularla de nuevo.
pub fn open(stored: &str) -> String {
    if !stored.starts_with(PREFIX) {
        return stored.to_string();
    }
    KEY.get()
        .and_then(|key| open_with(key, stored))
        .unwrap_or_default()
}

/// Cifra lo que quedó guardado en claro de antes. Devuelve cuántas cuentas
/// tocó. Sin clave no hace nada.
pub async fn seal_stored(pool: &PgPool) -> sqlx::Result<u64> {
    if KEY.get().is_none() {
        return Ok(0);
    }
    let like = format!("{PREFIX}%");
    let mut done = 0;
    let rows: Vec<(uuid::Uuid, String, String)> = sqlx::query_as(
        "SELECT id, mp_access_token, COALESCE(mp_refresh_token, '') FROM tenants
         WHERE mp_access_token IS NOT NULL
           AND (mp_access_token NOT LIKE $1 OR COALESCE(mp_refresh_token, '') NOT LIKE $1)",
    )
    .bind(&like)
    .fetch_all(pool)
    .await?;
    for (id, access, refresh) in rows {
        // Solo si nadie la cambió mientras tanto.
        done += sqlx::query(
            "UPDATE tenants SET mp_access_token = $4, mp_refresh_token = $5
             WHERE id = $1 AND mp_access_token = $2 AND COALESCE(mp_refresh_token, '') = $3",
        )
        .bind(id)
        .bind(&access)
        .bind(&refresh)
        .bind(seal(&access))
        .bind(seal(&refresh))
        .execute(pool)
        .await?
        .rows_affected();
    }
    let rows: Vec<(uuid::Uuid, String, String)> = sqlx::query_as(
        "SELECT tenant_id, credential, access_token FROM andreani_accounts
         WHERE (credential <> '' AND credential NOT LIKE $1)
            OR (access_token <> '' AND access_token NOT LIKE $1)",
    )
    .bind(&like)
    .fetch_all(pool)
    .await?;
    for (id, credential, token) in rows {
        done += sqlx::query(
            "UPDATE andreani_accounts SET credential = $4, access_token = $5
             WHERE tenant_id = $1 AND credential = $2 AND access_token = $3",
        )
        .bind(id)
        .bind(&credential)
        .bind(&token)
        .bind(seal(&credential))
        .bind(seal(&token))
        .execute(pool)
        .await?
        .rows_affected();
    }
    Ok(done)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sealed_values_open_only_with_their_key() {
        let key = [7u8; 32];
        let sealed = seal_with(&key, "APP_USR-123");
        assert!(sealed.starts_with(PREFIX));
        assert!(!sealed.contains("APP_USR"));
        assert_ne!(sealed, seal_with(&key, "APP_USR-123"));
        assert_eq!(open_with(&key, &sealed).as_deref(), Some("APP_USR-123"));
        assert_eq!(open_with(&[8u8; 32], &sealed), None);
        // Tocado por el camino: no abre.
        let mut broken = sealed.clone();
        broken.pop();
        broken.push(if sealed.ends_with('0') { '1' } else { '0' });
        assert_eq!(open_with(&key, &broken), None);
    }

    #[test]
    fn plain_values_from_before_read_as_they_are() {
        assert_eq!(open("APP_USR-123"), "APP_USR-123");
        assert_eq!(open(""), "");
    }

    #[test]
    fn the_key_file_is_created_once_and_kept_private() {
        let path = std::env::temp_dir().join(format!("mm-key-{}", uuid::Uuid::new_v4()));
        let first = load_or_create_key(&path).unwrap();
        assert_eq!(load_or_create_key(&path).unwrap(), first);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(&path).unwrap().permissions().mode();
            assert_eq!(mode & 0o777, 0o600);
        }
        std::fs::remove_file(&path).unwrap();
        assert!(parse_key("corta").is_none());
    }
}
