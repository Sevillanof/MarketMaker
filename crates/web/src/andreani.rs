//! Andreani (solo servidor): la API que usa el plugin oficial de WooCommerce.
//! Cada tienda la conecta con el "Credential ID" de su cuenta PyME.
//!
//! Configuración (opcional) en `.env.local`:
//! - `ANDREANI_API_URL`: para probar con un Andreani de mentira
//!   (`scripts/fake-andreani.js`). Sin ella, el real.

use domain::{AndreaniContract, AndreaniOrigin, AndreaniService};
use serde_json::{json, Value};
use std::sync::OnceLock;
use uuid::Uuid;

fn api_url() -> String {
    std::env::var("ANDREANI_API_URL")
        .ok()
        .map(|v| v.trim().trim_end_matches('/').to_string())
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| "https://woocommerce-api-acom.andreani.com".into())
}

fn http() -> &'static reqwest::Client {
    static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
    CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(15))
            .build()
            .expect("cliente HTTP")
    })
}

/// Por qué falló un pedido a Andreani.
#[derive(Debug)]
pub enum Error {
    /// El código (o el token) no sirve.
    Unauthorized,
    /// Andreani respondió con un error: el mensaje que dio.
    Api(u16, String),
    /// No se pudo hablar con Andreani.
    Network(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Unauthorized => write!(f, "credencial no válida"),
            Error::Api(s, m) => write!(f, "{s} {m}"),
            Error::Network(m) => write!(f, "sin conexión: {m}"),
        }
    }
}

enum Auth<'a> {
    Credential(&'a str),
    Token(&'a str),
}

/// Un pedido a la API. Devuelve `response` del JSON (o el JSON entero si no
/// viene envuelto).
async fn call(
    method: reqwest::Method,
    path: &str,
    auth: Auth<'_>,
    body: Option<Value>,
) -> Result<Value, Error> {
    let mut req = http().request(method, format!("{}{path}", api_url()));
    req = match auth {
        Auth::Credential(c) => req.header("Authorization", c),
        Auth::Token(t) => req.header("X-Auth-Token", t),
    };
    if let Some(b) = body {
        req = req
            .header("Content-Type", "application/json")
            .body(b.to_string());
    }
    let res = req.send().await.map_err(|e| Error::Network(e.to_string()))?;
    let status = res.status();
    let bytes = res
        .bytes()
        .await
        .map_err(|e| Error::Network(e.to_string()))?;
    let v: Value = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    if status == 401 || status == 403 {
        return Err(Error::Unauthorized);
    }
    if !status.is_success() {
        let msg = v["message"]
            .as_str()
            .or(v["error"].as_str())
            .unwrap_or("")
            .to_string();
        return Err(Error::Api(status.as_u16(), msg));
    }
    Ok(match v.get("response") {
        Some(r) => r.clone(),
        None => v,
    })
}

/// Lo que devuelve Andreani al entrar.
#[derive(Debug, Clone)]
pub struct LoginInfo {
    pub access_token: String,
    pub client_type: String,
    pub holder: String,
    pub cuit: String,
    pub email: String,
    pub contracts: Vec<AndreaniContract>,
}

fn text(v: &Value, keys: &[&str]) -> String {
    for k in keys {
        match &v[*k] {
            Value::String(s) if !s.trim().is_empty() => return s.trim().to_string(),
            Value::Number(n) => return n.to_string(),
            _ => {}
        }
    }
    String::new()
}

/// El tipo de cliente que trae el código: es base64 de "pyme|…" o "corpo|…".
fn client_type_of(credential: &str) -> &'static str {
    use base64_lite::decode;
    match decode(credential.trim()) {
        Some(d) if d.starts_with(b"corpo") => "corporativo",
        _ => "pyme",
    }
}

/// El servicio de un contrato: "bigger" si el tipo de envío lo dice; si no,
/// según el modo de entrega.
fn contract_of(c: &Value) -> Option<AndreaniContract> {
    let id = text(c, &["id"]);
    if id.is_empty() {
        return None;
    }
    let kind = text(c, &["tipoDeEnvioNombre"]);
    let service = if kind.eq_ignore_ascii_case("bigger") {
        AndreaniService::Bigger
    } else {
        AndreaniService::from_api(&text(c, &["modoDeEntregaNombre"]))?
    };
    Some(AndreaniContract {
        id,
        number: text(c, &["numeroDeContrato"]),
        service,
    })
}

/// Entra con el código del dueño.
pub async fn login(credential: &str) -> Result<LoginInfo, Error> {
    let r = call(
        reqwest::Method::POST,
        "/api/v1/Login",
        Auth::Credential(credential.trim()),
        Some(json!({})),
    )
    .await?;
    let access_token = text(&r, &["accessToken"]);
    if access_token.is_empty() {
        return Err(Error::Unauthorized);
    }
    let mut client_type = text(&r, &["clientType"]).to_lowercase();
    if client_type.is_empty() {
        client_type = client_type_of(credential).to_string();
    }
    let contracts = r["contratos"]
        .as_array()
        .map(|a| a.iter().filter_map(contract_of).collect())
        .unwrap_or_default();
    Ok(LoginInfo {
        access_token,
        client_type,
        holder: text(&r, &["razonSocial", "name", "nombre"]),
        cuit: text(&r, &["cuit", "cuil", "documento"]),
        email: text(&r, &["email"]),
        contracts,
    })
}

/// Un pedido con el token de la tienda. Si venció, vuelve a entrar con el
/// código guardado una vez. Solo para pedidos que se pueden repetir.
async fn with_account(
    tenant: Uuid,
    method: reqwest::Method,
    path: &str,
    body: Option<Value>,
) -> Result<Value, Error> {
    let pool = crate::server::pool().map_err(|e| Error::Network(e.to_string()))?;
    let acc = db::andreani::account(&pool, tenant)
        .await
        .map_err(|e| Error::Network(e.to_string()))?
        .ok_or(Error::Unauthorized)?;
    if !acc.access_token.is_empty() {
        match call(method.clone(), path, Auth::Token(&acc.access_token), body.clone()).await {
            Err(Error::Unauthorized) => {}
            other => return other,
        }
    }
    let fresh = login(&acc.credential).await?;
    let _ = db::andreani::set_token(&pool, tenant, &fresh.access_token).await;
    call(method, path, Auth::Token(&fresh.access_token), body).await
}

/// Guarda en la cuenta de Andreani desde dónde despacha la tienda.
pub async fn save_origin(tenant: Uuid, o: &AndreaniOrigin) -> Result<(), Error> {
    with_account(
        tenant,
        reqwest::Method::PATCH,
        "/api/v1/Settings/origin",
        Some(json!({
            "calle": o.street,
            "numero": o.number,
            "piso": o.floor,
            "localidad": o.city,
            "provincia": o.province,
            "codigoPostal": o.postal_code,
            "remitente": o.sender,
            "sucursalAsignada": "",
        })),
    )
    .await
    .map(|_| ())
}

/// Decodificador base64 mínimo (solo para leer el tipo de cliente del código).
mod base64_lite {
    pub fn decode(s: &str) -> Option<Vec<u8>> {
        let val = |c: u8| -> Option<u32> {
            Some(match c {
                b'A'..=b'Z' => c - b'A',
                b'a'..=b'z' => c - b'a' + 26,
                b'0'..=b'9' => c - b'0' + 52,
                b'+' | b'-' => 62,
                b'/' | b'_' => 63,
                _ => return None,
            } as u32)
        };
        let bytes: Vec<u8> = s.bytes().filter(|c| *c != b'=').collect();
        let mut out = Vec::with_capacity(bytes.len() * 3 / 4);
        for chunk in bytes.chunks(4) {
            let mut n = 0u32;
            for (i, c) in chunk.iter().enumerate() {
                n |= val(*c)? << (18 - 6 * i);
            }
            let take = chunk.len().saturating_sub(1);
            for i in 0..take {
                out.push((n >> (16 - 8 * i)) as u8);
            }
        }
        Some(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tipo_de_cliente() {
        // "pyme|abc" y "corpo|abc".
        assert_eq!(client_type_of("cHltZXxhYmM="), "pyme");
        assert_eq!(client_type_of("Y29ycG98YWJj"), "corporativo");
        assert_eq!(client_type_of("???"), "pyme");
    }

    #[test]
    fn contratos() {
        let c = json!({"id": 7, "numeroDeContrato": "400006709",
                       "modoDeEntregaNombre": "A sucursal", "tipoDeEnvioNombre": "paqueteria"});
        let k = contract_of(&c).unwrap();
        assert_eq!(k.id, "7");
        assert_eq!(k.service, AndreaniService::Branch);
        let c = json!({"id": "9", "modoDeEntregaNombre": "A domicilio", "tipoDeEnvioNombre": "Bigger"});
        assert_eq!(contract_of(&c).unwrap().service, AndreaniService::Bigger);
        assert!(contract_of(&json!({"id": "1", "modoDeEntregaNombre": "raro"})).is_none());
    }
}
