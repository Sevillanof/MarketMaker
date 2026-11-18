//! Andreani (solo servidor): la API que usa el plugin oficial de WooCommerce.
//! Cada tienda la conecta con el "Credential ID" de su cuenta PyME.
//!
//! Configuración (opcional) en `.env.local`:
//! - `ANDREANI_API_URL`: para probar con un Andreani de mentira
//!   (`scripts/fake-andreani.js`). Sin ella, el real.

use domain::{
    AndreaniBranch, AndreaniContract, AndreaniOrigin, AndreaniRate, AndreaniService, Parcel,
};
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

/// Un pedido a la API. Devuelve el cuerpo tal cual (con 401/403 como
/// [`Error::Unauthorized`] y otros errores con el mensaje de Andreani).
async fn send(
    method: reqwest::Method,
    path: &str,
    auth: Auth<'_>,
    body: Option<Value>,
) -> Result<Vec<u8>, Error> {
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
    let res = req
        .send()
        .await
        .map_err(|e| Error::Network(e.to_string()))?;
    let status = res.status();
    let bytes = res
        .bytes()
        .await
        .map_err(|e| Error::Network(e.to_string()))?;
    if status == 401 || status == 403 {
        return Err(Error::Unauthorized);
    }
    if !status.is_success() {
        let v: Value = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
        let msg = v["message"]
            .as_str()
            .or(v["error"].as_str())
            .or(v["response"]["message"].as_str())
            .unwrap_or("")
            .to_string();
        return Err(Error::Api(status.as_u16(), msg));
    }
    Ok(bytes.to_vec())
}

/// `response` del JSON (o el JSON entero si no viene envuelto).
fn unwrap_json(bytes: &[u8]) -> Value {
    let v: Value = serde_json::from_slice(bytes).unwrap_or(Value::Null);
    match v.get("response") {
        Some(r) => r.clone(),
        None => v,
    }
}

async fn call(
    method: reqwest::Method,
    path: &str,
    auth: Auth<'_>,
    body: Option<Value>,
) -> Result<Value, Error> {
    send(method, path, auth, body)
        .await
        .map(|b| unwrap_json(&b))
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

/// Un pedido con el token de la tienda. Si venció (401), vuelve a entrar
/// con el código guardado y lo repite una vez: con 401 Andreani no hizo nada,
/// así que repetirlo es seguro también para crear envíos.
async fn with_account_raw(
    pool: &db::Pool,
    tenant: Uuid,
    method: reqwest::Method,
    path: &str,
    body: Option<Value>,
) -> Result<Vec<u8>, Error> {
    let acc = db::andreani::account(pool, tenant)
        .await
        .map_err(|e| Error::Network(e.to_string()))?
        .ok_or(Error::Unauthorized)?;
    if !acc.access_token.is_empty() {
        match send(
            method.clone(),
            path,
            Auth::Token(&acc.access_token),
            body.clone(),
        )
        .await
        {
            Err(Error::Unauthorized) => {}
            other => return other,
        }
    }
    let fresh = login(&acc.credential).await?;
    let _ = db::andreani::set_token(pool, tenant, &fresh.access_token).await;
    send(method, path, Auth::Token(&fresh.access_token), body).await
}

async fn with_account(
    pool: &db::Pool,
    tenant: Uuid,
    method: reqwest::Method,
    path: &str,
    body: Option<Value>,
) -> Result<Value, Error> {
    with_account_raw(pool, tenant, method, path, body)
        .await
        .map(|b| unwrap_json(&b))
}

/// Guarda en la cuenta de Andreani desde dónde despacha la tienda.
pub async fn save_origin(pool: &db::Pool, tenant: Uuid, o: &AndreaniOrigin) -> Result<(), Error> {
    with_account(
        pool,
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

/// La parte de la dirección según el tipo de cliente ("Pyme" o "Corporative").
fn kind_path(client_type: &str) -> &'static str {
    if client_type.starts_with("corpo") {
        "Corporative"
    } else {
        "Pyme"
    }
}

/// Un renglón del carrito para cotizar: el paquete, el precio unitario y la cantidad.
pub struct Line {
    pub parcel: Parcel,
    pub unit_cents: i64,
    pub qty: u32,
}

/// Lo que cuesta mandar `lines` de `from` a `to` con cada servicio.
pub async fn rates(
    pool: &db::Pool,
    tenant: Uuid,
    client_type: &str,
    from: &str,
    to: &str,
    lines: &[Line],
    bigger: bool,
) -> Result<Vec<AndreaniRate>, Error> {
    let cm = |mm: u32| f64::from(mm) / 10.0;
    let products: Vec<Value> = lines
        .iter()
        .map(|l| {
            json!({
                "quantity": l.qty,
                "price": (l.unit_cents / 100).max(1),
                "dimensions": {
                    "width": cm(l.parcel.width_mm),
                    "height": cm(l.parcel.height_mm),
                    "depth": cm(l.parcel.length_mm),
                    "grams": l.parcel.weight_g,
                },
            })
        })
        .collect();
    let r = with_account(
        pool,
        tenant,
        reqwest::Method::POST,
        &format!("/api/v1/{}/rates", kind_path(client_type)),
        Some(json!({
            "postal_code_origin": from,
            "postal_code_destination": to,
            "products": products,
            "force_bigger": bigger,
        })),
    )
    .await?;
    let list = r["rates"]
        .as_array()
        .or(r.as_array())
        .cloned()
        .unwrap_or_default();
    Ok(list
        .iter()
        .filter_map(|x| {
            let service = AndreaniService::from_api(&text(x, &["code"]))?;
            let total = match &x["total"] {
                Value::Number(n) => n.as_f64()?,
                Value::String(s) => s.replace(',', ".").parse().ok()?,
                _ => return None,
            };
            (total > 0.0).then(|| AndreaniRate {
                service,
                cents: (total * 100.0).round() as i64,
            })
        })
        .collect())
}

/// Las sucursales de Andreani cerca de un código postal.
pub async fn branches(
    pool: &db::Pool,
    tenant: Uuid,
    postal_code: &str,
) -> Result<Vec<AndreaniBranch>, Error> {
    let r = with_account(
        pool,
        tenant,
        reqwest::Method::GET,
        &format!("/api/v1/Branch?postalCode={postal_code}"),
        None,
    )
    .await?;
    let list = r["data"]
        .as_array()
        .or(r.as_array())
        .cloned()
        .unwrap_or_default();
    Ok(list.iter().filter_map(branch_of).collect())
}

fn branch_of(b: &Value) -> Option<AndreaniBranch> {
    let code = text(b, &["Codigo", "codigo"]);
    if code.is_empty() {
        return None;
    }
    let d = &b["Direccion"];
    let street = [text(d, &["Calle"]), text(d, &["Numero"])]
        .into_iter()
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join(" ");
    let address = [street, text(d, &["Localidad"])]
        .into_iter()
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join(", ");
    Some(AndreaniBranch {
        code,
        name: text(b, &["Descripcion", "descripcion"]),
        address,
        hours: text(b, &["horarioDeAtencion"]),
    })
}

/// Lo que hace falta para dar de alta un envío.
pub struct NewShipment<'a> {
    pub client_type: &'a str,
    pub contract_id: &'a str,
    pub quote_cents: i64,
    pub origin_postal_code: &'a str,
    pub street: &'a str,
    pub number: &'a str,
    pub floor: &'a str,
    pub postal_code: &'a str,
    pub city: &'a str,
    pub branch_code: &'a str,
    pub name: &'a str,
    pub last_name: &'a str,
    pub phone: &'a str,
    pub dni: &'a str,
    pub email: &'a str,
    pub lines: &'a [Line],
    pub bigger: bool,
    pub merchant_email: &'a str,
    /// El número de pedido de la tienda.
    pub remito: &'a str,
}

/// Da de alta el envío y devuelve el número de pedido de Andreani. No es
/// idempotente: si falla por la red, no se reintenta solo.
pub async fn create_shipment(
    pool: &db::Pool,
    tenant: Uuid,
    n: &NewShipment<'_>,
) -> Result<String, Error> {
    let cm = |mm: u32| f64::from(mm) / 10.0;
    let products: Vec<Value> = n
        .lines
        .iter()
        .map(|l| {
            json!({
                "price": (l.unit_cents / 100).max(1),
                "quantity": l.qty,
                "kgrams": f64::from(l.parcel.weight_g) / 1000.0,
                "width": cm(l.parcel.width_mm),
                "depth": cm(l.parcel.length_mm),
                "height": cm(l.parcel.height_mm),
            })
        })
        .collect();
    let r = with_account(
        pool,
        tenant,
        reqwest::Method::POST,
        &format!("/api/v1/{}/ShippingRegistration", kind_path(n.client_type)),
        Some(json!({
            "contract": { "id_contract": n.contract_id },
            "price_shipment": n.quote_cents as f64 / 100.0,
            "origin": { "postal_code": n.origin_postal_code },
            "destination": {
                "street": n.street,
                "number": n.number,
                "floor": n.floor,
                "postal_code": n.postal_code,
                "locality": n.city,
                "code_branch": n.branch_code,
            },
            "recipient": {
                "name": n.name,
                "last_name": n.last_name,
                "phone_number": n.phone,
                "dni": n.dni,
                "email": n.email,
            },
            "products": products,
            "force_bigger": n.bigger,
            "email_merchant": n.merchant_email,
            "remito": n.remito,
        })),
    )
    .await?;
    let id = text(&r, &["pedidoId", "pedido_id", "numeroDeEnvio"]);
    if id.is_empty() {
        return Err(Error::Api(
            200,
            "Andreani no devolvió el número de envío".into(),
        ));
    }
    Ok(id)
}

/// La etiqueta en PDF de envíos ya pagados (por número de seguimiento).
pub async fn label(
    pool: &db::Pool,
    tenant: Uuid,
    client_type: &str,
    trackings: &[String],
) -> Result<Vec<u8>, Error> {
    let pdf = with_account_raw(
        pool,
        tenant,
        reqwest::Method::POST,
        &format!("/api/v1/{}/ticket", kind_path(client_type)),
        Some(json!({ "trackingNumbers": trackings })),
    )
    .await?;
    if !pdf.starts_with(b"%PDF") {
        return Err(Error::Api(200, "Andreani no devolvió un PDF".into()));
    }
    Ok(pdf)
}

/// Seguimiento de un envío: número y estado según Andreani.
#[derive(Debug, Clone, PartialEq)]
pub struct Tracking {
    pub pedido_id: String,
    pub tracking: String,
    pub status: String,
}

/// El seguimiento de hasta 100 envíos (por número de pedido de Andreani).
pub async fn track(
    pool: &db::Pool,
    tenant: Uuid,
    pedido_ids: &[String],
) -> Result<Vec<Tracking>, Error> {
    let r = with_account(
        pool,
        tenant,
        reqwest::Method::POST,
        "/api/v1/Shipments/ByOrderNumbers",
        Some(json!({ "salesOrderNumbers": pedido_ids })),
    )
    .await?;
    let list = r
        .as_array()
        .or(r["data"].as_array())
        .cloned()
        .unwrap_or_default();
    Ok(list
        .iter()
        .map(|x| Tracking {
            pedido_id: text(x, &["salesOrderNumber"]),
            tracking: text(x, &["trackingNumber"]),
            status: text(x, &["trackingStatus"]).to_lowercase(),
        })
        .filter(|t| !t.pedido_id.is_empty())
        .collect())
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
        let c =
            json!({"id": "9", "modoDeEntregaNombre": "A domicilio", "tipoDeEnvioNombre": "Bigger"});
        assert_eq!(contract_of(&c).unwrap().service, AndreaniService::Bigger);
        assert!(contract_of(&json!({"id": "1", "modoDeEntregaNombre": "raro"})).is_none());
    }

    #[test]
    fn sucursales() {
        let b = json!({"Codigo": "S01", "Descripcion": "Andreani Palermo",
            "Direccion": {"Calle": "Av. Santa Fe", "Numero": "3200", "Localidad": "CABA"},
            "horarioDeAtencion": "Lun a Vie 9 a 18 h"});
        let got = branch_of(&b).unwrap();
        assert_eq!(got.address, "Av. Santa Fe 3200, CABA");
        assert_eq!(got.hours, "Lun a Vie 9 a 18 h");
        assert!(branch_of(&json!({"Descripcion": "sin código"})).is_none());
    }
}
