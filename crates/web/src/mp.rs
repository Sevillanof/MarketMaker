//! Mercado Pago (solo servidor): vincular la cuenta de cada tienda (OAuth),
//! crear el pago de un pedido (Checkout Pro) y enterarse cuando se aprueba.
//!
//! Configuración en `.env.local` (ver `.env`):
//! - `MP_CLIENT_ID` y `MP_CLIENT_SECRET`: la aplicación de MarketMaker en
//!   Mercado Pago Developers. Sin ellas, "Vincular mi cuenta" no aparece.
//! - `MP_REDIRECT_URL`: adónde vuelve Mercado Pago al vincular. Tiene que ser
//!   igual a la que está cargada en la aplicación.
//! - `MP_PUBLIC_URL` (opcional): dirección pública `https://…` de este
//!   servidor (por ejemplo un túnel de cloudflared) para los avisos de pago.
//!   Sin ella se usa la de `MP_REDIRECT_URL` si es https (producción); si
//!   tampoco, el pago se confirma cuando el comprador vuelve a la tienda.
//! - `MP_WEBHOOK_SECRET` (opcional): la "clave secreta" de Webhooks de la
//!   aplicación. Con ella se comprueba la firma de cada aviso de pago y se
//!   anota el resultado; el aviso se acepta igual. Con `MP_WEBHOOK_STRICT=1`
//!   además se descarta el que no venga firmado por Mercado Pago (activarlo
//!   recién después de ver avisos reales con firma válida).
//!   `MP_SANDBOX_WEBHOOK_SECRET` es la de la aplicación de prueba.
//! - `MP_SANDBOX_STORES` (opcional, ej. `pruebas`), `MP_SANDBOX_CLIENT_ID` y
//!   `MP_SANDBOX_CLIENT_SECRET`: esas tiendas usan otra aplicación, creada por
//!   una cuenta de prueba "integradora". Mercado Pago no deja pagar si la
//!   aplicación es de una cuenta real y el vendedor es de prueba.
//!
//! Para probar se vinculan cuentas de prueba de Mercado Pago (vendedor y
//! comprador): sus llaves ya son de prueba. No se piden llaves `TEST-`
//! (`test_token`): Checkout Pro con esas llaves falla al pagar.

use axum::extract::{Path, Query, State};
use axum::http::{header, HeaderMap, StatusCode};
use axum::response::{IntoResponse, Redirect, Response};
use axum::routing::{get, post};
use axum::Router;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::OnceLock;
use uuid::Uuid;

/// La aplicación de MarketMaker en Mercado Pago.
pub struct Config {
    pub client_id: String,
    pub client_secret: String,
    pub redirect_url: String,
    pub public_url: Option<String>,
    /// La clave con la que Mercado Pago firma los avisos, si está cargada.
    webhook_secret: Option<String>,
    auth_url: String,
    api_url: String,
}

fn env(name: &str) -> Option<String> {
    std::env::var(name)
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
}

/// La configuración, si está completa.
pub fn config() -> Option<Config> {
    let redirect_url =
        env("MP_REDIRECT_URL").unwrap_or_else(|| "http://localhost:3000/mp/vincular".into());
    // Sin MP_PUBLIC_URL, si la plataforma ya está en https (producción) los
    // avisos van a esa misma dirección. Sin avisos, un pago que se aprueba
    // después (efectivo, revisión) o un comprador que cierra la pestaña antes
    // de volver dejaría el pedido sin entrar nunca.
    let public_url = env("MP_PUBLIC_URL")
        .or_else(|| {
            reqwest::Url::parse(&redirect_url)
                .ok()
                .map(|u| u.origin().ascii_serialization())
        })
        .filter(|u| u.starts_with("https://"))
        .map(|u| u.trim_end_matches('/').to_string());
    Some(Config {
        client_id: env("MP_CLIENT_ID")?,
        client_secret: env("MP_CLIENT_SECRET")?,
        redirect_url,
        public_url,
        webhook_secret: env("MP_WEBHOOK_SECRET"),
        // Se pueden cambiar para probar sin el Mercado Pago real.
        auth_url: env("MP_AUTH_URL").unwrap_or_else(|| "https://auth.mercadopago.com.ar".into()),
        api_url: env("MP_API_URL").unwrap_or_else(|| "https://api.mercadopago.com".into()),
    })
}

/// La aplicación que usa una tienda: la de prueba si está en
/// `MP_SANDBOX_STORES`; si no, la de siempre. Una tienda de prueba sin la
/// aplicación de prueba configurada no cobra (nunca usa la real).
pub fn config_for(slug: &str) -> Option<Config> {
    let mut c = config()?;
    if is_test_store(slug) {
        c.client_id = env("MP_SANDBOX_CLIENT_ID")?;
        c.client_secret = env("MP_SANDBOX_CLIENT_SECRET")?;
        c.webhook_secret = env("MP_SANDBOX_WEBHOOK_SECRET");
    }
    Some(c)
}

/// ¿Es una tienda de pruebas (está en `MP_SANDBOX_STORES`)?
pub fn is_test_store(slug: &str) -> bool {
    env("MP_SANDBOX_STORES").is_some_and(|v| v.split(',').any(|s| s.trim() == slug))
}

/// Como [`config_for`], buscando la dirección de la tienda.
async fn config_of(pool: &db::Pool, tenant: Uuid) -> Option<Config> {
    let slug = db::mp::slug_of(pool, tenant).await.ok().flatten()?;
    config_for(&slug)
}

fn http() -> &'static reqwest::Client {
    static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
    CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(20))
            .build()
            .expect("cliente HTTP")
    })
}

/// Un pedido a la API de Mercado Pago. Devuelve el JSON o el error en palabras.
async fn call(
    method: reqwest::Method,
    url: &str,
    token: Option<&str>,
    body: Option<Value>,
) -> Result<Value, String> {
    let mut req = http().request(method, url);
    if let Some(t) = token {
        req = req.bearer_auth(t);
    }
    if let Some(b) = body {
        req = req
            .header(header::CONTENT_TYPE, "application/json")
            .body(b.to_string());
    }
    let res = req.send().await.map_err(|e| format!("sin conexión: {e}"))?;
    let status = res.status();
    let bytes = res
        .bytes()
        .await
        .map_err(|e| format!("respuesta cortada: {e}"))?;
    let v: Value = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    if status.is_success() {
        Ok(v)
    } else {
        let msg = v["message"].as_str().or(v["error"].as_str()).unwrap_or("");
        Err(format!("{status} {msg}"))
    }
}

/// Adónde mandar al dueño para que acepte vincular su cuenta.
pub fn authorize_url(c: &Config, state: &str) -> String {
    let mut u = reqwest::Url::parse(&format!("{}/authorization", c.auth_url)).expect("MP_AUTH_URL");
    u.query_pairs_mut()
        .append_pair("client_id", &c.client_id)
        .append_pair("response_type", "code")
        .append_pair("platform_id", "mp")
        .append_pair("state", state)
        .append_pair("redirect_uri", &c.redirect_url);
    u.to_string()
}

pub fn new_state() -> String {
    use rand::RngCore;
    let mut b = [0u8; 24];
    rand::rng().fill_bytes(&mut b);
    hex::encode(b)
}

/// Guarda lo que devolvió `/oauth/token`.
async fn save_tokens(pool: &db::Pool, tenant: Uuid, v: &Value) -> Result<(), String> {
    let (Some(access), Some(user_id)) = (v["access_token"].as_str(), v["user_id"].as_i64()) else {
        return Err("Mercado Pago no devolvió la cuenta".into());
    };
    db::mp::save_link(
        pool,
        tenant,
        &db::mp::Tokens {
            user_id,
            access_token: access,
            refresh_token: v["refresh_token"].as_str().unwrap_or(""),
            public_key: v["public_key"].as_str().unwrap_or(""),
            expires_in_secs: v["expires_in"].as_i64().unwrap_or(15_552_000),
            live: v["live_mode"].as_bool().unwrap_or(false),
        },
    )
    .await
    .map_err(|e| e.to_string())
}

/// Una llave vigente de la cuenta de la tienda (la renueva si está por vencer).
pub async fn token(pool: &db::Pool, tenant: Uuid) -> Result<Option<String>, String> {
    let Some(creds) = db::mp::credentials(pool, tenant)
        .await
        .map_err(|e| e.to_string())?
    else {
        return Ok(None);
    };
    if creds.expiring && !creds.refresh_token.is_empty() {
        if let Some(c) = config_of(pool, tenant).await {
            let r = call(
                reqwest::Method::POST,
                &format!("{}/oauth/token", c.api_url),
                None,
                Some(json!({
                    "client_id": c.client_id,
                    "client_secret": c.client_secret,
                    "grant_type": "refresh_token",
                    "refresh_token": creds.refresh_token,
                })),
            )
            .await;
            match r {
                Ok(v) => {
                    save_tokens(pool, tenant, &v).await?;
                    return Ok(v["access_token"].as_str().map(str::to_string));
                }
                // Si no se pudo, se prueba con la que hay.
                Err(e) => leptos::logging::log!("Mercado Pago: no pudimos renovar la llave: {e}"),
            }
        }
    }
    Ok(Some(creds.access_token))
}

fn amount(cents: i64) -> f64 {
    cents as f64 / 100.0
}

/// Crea el pago de un pedido en Mercado Pago y devuelve adónde ir a pagar.
/// `store_base` es la dirección de la tienda (`http://demo.localhost:3000`).
pub async fn create_preference(
    pool: &db::Pool,
    tenant: Uuid,
    store_name: &str,
    store_base: &str,
    order: &domain::Order,
) -> Result<String, String> {
    let Some(token) = token(pool, tenant).await? else {
        return Err("la tienda no vinculó Mercado Pago".into());
    };
    let c = config().ok_or("falta configurar Mercado Pago")?;
    let mut items: Vec<Value> = order
        .items
        .iter()
        .map(|i| {
            json!({
                "id": i.product_id,
                "title": i.name,
                "quantity": i.qty,
                "unit_price": amount(i.unit_cents),
                "currency_id": "ARS",
                "picture_url": absolute(store_base, &i.image_url),
            })
        })
        .collect();
    // Con un cupón, Mercado Pago no acepta renglones negativos: se cobra un
    // solo renglón con lo que cuestan los productos ya con el descuento.
    if order.coupon_cents > 0 {
        let products = order.subtotal_cents - order.coupon_cents - order.discount_cents;
        items = vec![json!({
            "id": "pedido",
            "title": format!("Tu pedido en {store_name} (cupón {})", order.coupon_code),
            "quantity": 1,
            "unit_price": amount(products),
            "currency_id": "ARS",
        })];
    }
    if order.shipping_cents > 0 {
        items.push(json!({
            "id": "envio",
            "title": "Envío",
            "quantity": 1,
            "unit_price": amount(order.shipping_cents),
            "currency_id": "ARS",
        }));
    }
    let back = format!("{store_base}/pedido/{}", order.id);
    let mut body = json!({
        "items": items,
        "payer": { "name": order.customer_name, "email": order.email },
        "external_reference": order.id,
        "statement_descriptor": store_name.chars().take(22).collect::<String>(),
        "back_urls": { "success": back, "pending": back, "failure": back },
    });
    // El intento de compra se descarta a las 48 h: después de 24 h Mercado
    // Pago ya no acepta pagarlo (tampoco en efectivo), así nunca llega un
    // pago de un pedido que ya no existe.
    let until = db::mp::utc_in_hours(pool, 24)
        .await
        .map_err(|e| e.to_string())?;
    body["expires"] = json!(true);
    body["expiration_date_to"] = json!(until);
    body["date_of_expiration"] = json!(until);
    // Mercado Pago solo vuelve solo a direcciones https.
    if back.starts_with("https://") {
        body["auto_return"] = json!("approved");
    }
    if let Some(public) = &c.public_url {
        body["notification_url"] =
            json!(format!("{public}/mp/aviso/{tenant}?source_news=webhooks"));
    }
    let v = call(
        reqwest::Method::POST,
        &format!("{}/checkout/preferences", c.api_url),
        Some(&token),
        Some(body),
    )
    .await?;
    let (Some(id), Some(url)) = (v["id"].as_str(), v["init_point"].as_str()) else {
        return Err("Mercado Pago no devolvió el enlace de pago".into());
    };
    let order_id = Uuid::parse_str(&order.id).map_err(|e| e.to_string())?;
    db::mp::set_preference(pool, tenant, order_id, id, url)
        .await
        .map_err(|e| e.to_string())?;
    Ok(url.to_string())
}

/// Las fotos de la tienda son `/media/…`; Mercado Pago necesita la dirección entera.
fn absolute(base: &str, url: &str) -> String {
    if url.starts_with('/') {
        format!("{base}{url}")
    } else {
        url.to_string()
    }
}

/// Pregunta a Mercado Pago por un pago y, si está aprobado, marca pagado su
/// pedido. Nunca se confía en lo que dice el navegador: el estado sale de la
/// API con la llave de la tienda. Devuelve el estado del pago.
pub async fn check_payment(
    pool: &db::Pool,
    tenant: Uuid,
    payment_id: &str,
) -> Result<String, String> {
    if payment_id.is_empty() || !payment_id.chars().all(|c| c.is_ascii_digit()) {
        return Err("número de pago inválido".into());
    }
    let Some(token) = token(pool, tenant).await? else {
        return Err("la tienda no vinculó Mercado Pago".into());
    };
    let c = config().ok_or("falta configurar Mercado Pago")?;
    let v = call(
        reqwest::Method::GET,
        &format!("{}/v1/payments/{payment_id}", c.api_url),
        Some(&token),
        None,
    )
    .await?;
    let status = v["status"].as_str().unwrap_or("").to_string();
    if status == "approved" && v["currency_id"].as_str() == Some("ARS") {
        let order = v["external_reference"]
            .as_str()
            .and_then(|r| Uuid::parse_str(r).ok());
        let paid = v["transaction_amount"].as_f64().unwrap_or(0.0);
        if let Some(order) = order {
            let cents = (paid * 100.0).round() as i64;
            if let Some(short) =
                db::orders::mark_paid_noting(pool, tenant, order, payment_id, cents)
                    .await
                    .map_err(|e| e.to_string())?
            {
                leptos::logging::log!("Mercado Pago: pedido {order} pagado (pago {payment_id})");
                crate::email::order_emails(pool, tenant, order, true).await;
                // Se cobró algo que ya no había: el dueño tiene que saberlo.
                if !short.is_empty() {
                    crate::email::tell_sold_without_stock(pool, tenant, order, &short).await;
                }
                // Si va por Andreani, se crea el envío (sin frenar la respuesta).
                let pool = pool.clone();
                tokio::spawn(async move {
                    crate::shipments::ship_order(&pool, tenant, order).await;
                });
            }
        }
    }
    // Devuelto desde Mercado Pago o desconocido por el comprador: el pedido
    // ya no está pagado.
    if matches!(status.as_str(), "refunded" | "charged_back") {
        let order = v["external_reference"]
            .as_str()
            .and_then(|r| Uuid::parse_str(r).ok());
        if let Some(order) = order {
            if db::orders::cancel_refunded(pool, tenant, order, payment_id)
                .await
                .map_err(|e| e.to_string())?
            {
                leptos::logging::log!(
                    "Mercado Pago: pedido {order} cancelado, pago {payment_id} {status}"
                );
                crate::email::tell_refunded(pool, tenant, order, status == "charged_back").await;
            }
        }
    }
    Ok(status)
}

/// `GET /mp/vincular` (vuelve de vincular) y `POST /mp/aviso/{tienda}` (avisos
/// de pago de Mercado Pago).
pub fn routes<S: Clone + Send + Sync + 'static>(pool: db::Pool) -> Router<S> {
    Router::new()
        .route("/mp/vincular", get(linked))
        .route("/mp/aviso/{tenant}", post(notice))
        .with_state(pool)
}

/// Vuelve de Mercado Pago después de aceptar (o no) vincular la cuenta.
async fn linked(
    State(pool): State<db::Pool>,
    headers: HeaderMap,
    Query(q): Query<HashMap<String, String>>,
) -> Response {
    let platform = config()
        .and_then(|c| {
            reqwest::Url::parse(&c.redirect_url)
                .ok()
                .map(|u| u.origin().ascii_serialization())
        })
        .unwrap_or_default();
    let fail = |slug: Option<&str>, why: &str| -> Response {
        let to = match slug {
            Some(s) => format!("{platform}/tienda/{s}/cobros?error={why}"),
            None => format!("{platform}/"),
        };
        Redirect::to(&to).into_response()
    };
    let Some(state) = q.get("state") else {
        return fail(None, "");
    };
    // El permiso solo vale en la sesión que lo empezó: si este enlace lo
    // abre otra persona (o nadie con sesión), no se vincula nada.
    let session = crate::auth::session_token_in(&headers);
    let (tenant, same_session) = match db::mp::take_state(&pool, state, session.as_deref()).await {
        Ok(db::mp::StateCheck::Ok(t)) => (t, true),
        Ok(db::mp::StateCheck::OtherSession(t)) => (t, false),
        _ => return fail(None, ""),
    };
    let slug = db::mp::slug_of(&pool, tenant)
        .await
        .ok()
        .flatten()
        .unwrap_or_default();
    if !same_session {
        return fail(Some(&slug), "sesion");
    }
    let Some(code) = q.get("code") else {
        // Tocó "Cancelar" en Mercado Pago.
        return fail(Some(&slug), "cancelado");
    };
    let Some(c) = config_for(&slug) else {
        return fail(Some(&slug), "configuracion");
    };
    let body = json!({
        "client_id": c.client_id,
        "client_secret": c.client_secret,
        "grant_type": "authorization_code",
        "code": code,
        "redirect_uri": c.redirect_url,
    });
    let r = call(
        reqwest::Method::POST,
        &format!("{}/oauth/token", c.api_url),
        None,
        Some(body),
    )
    .await;
    match r {
        Ok(v) => match save_tokens(&pool, tenant, &v).await {
            Ok(()) => Redirect::to(&format!("{platform}/tienda/{slug}/cobros?vinculada=1"))
                .into_response(),
            Err(e) => {
                leptos::logging::log!("Mercado Pago: no pudimos guardar la cuenta: {e}");
                fail(Some(&slug), "guardar")
            }
        },
        Err(e) => {
            leptos::logging::log!("Mercado Pago: no pudimos vincular: {e}");
            fail(Some(&slug), "vincular")
        }
    }
}

/// HMAC-SHA256 (RFC 2104) en hexadecimal.
fn hmac_sha256_hex(key: &[u8], message: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let mut block = [0u8; 64];
    if key.len() > 64 {
        block[..32].copy_from_slice(&Sha256::digest(key));
    } else {
        block[..key.len()].copy_from_slice(key);
    }
    let inner = Sha256::new()
        .chain_update(block.map(|b| b ^ 0x36))
        .chain_update(message)
        .finalize();
    let outer = Sha256::new()
        .chain_update(block.map(|b| b ^ 0x5c))
        .chain_update(inner)
        .finalize();
    hex::encode(outer)
}

/// ¿El aviso viene firmado por Mercado Pago? `signature` es la cabecera
/// `x-signature` (`ts=…,v1=…`); se firma `id:…;request-id:…;ts:…;` con la
/// clave secreta de la aplicación. `data_id` es el `data.id` de la dirección.
fn signed_by_mp(secret: &str, signature: &str, request_id: &str, data_id: &str) -> bool {
    let part = |name: &str| {
        signature
            .split(',')
            .filter_map(|p| p.trim().split_once('='))
            .find(|(k, _)| k.trim() == name)
            .map(|(_, v)| v.trim())
    };
    let (Some(ts), Some(v1)) = (part("ts"), part("v1")) else {
        return false;
    };
    // Las partes que no vienen no entran en lo firmado.
    let mut manifest = String::new();
    if !data_id.is_empty() {
        manifest.push_str(&format!("id:{};", data_id.to_lowercase()));
    }
    if !request_id.is_empty() {
        manifest.push_str(&format!("request-id:{request_id};"));
    }
    manifest.push_str(&format!("ts:{ts};"));
    let expected = hmac_sha256_hex(secret.as_bytes(), manifest.as_bytes());
    // Compara sin cortar antes, para no dar pistas por el tiempo.
    expected.len() == v1.len()
        && expected
            .bytes()
            .zip(v1.bytes())
            .fold(0u8, |acc, (a, b)| acc | (a ^ b.to_ascii_lowercase()))
            == 0
}

/// Aviso de Mercado Pago: "cambió el pago N". Se pregunta por el pago y se
/// actualiza el pedido. Siempre responde 200 para que no reintente de más.
async fn notice(
    State(pool): State<db::Pool>,
    Path(tenant): Path<Uuid>,
    Query(q): Query<HashMap<String, String>>,
    headers: HeaderMap,
    body: axum::body::Bytes,
) -> Response {
    // Cada aviso hace que preguntemos a Mercado Pago con la llave de la
    // tienda: tope por conexión (los avisos de verdad vienen de a pocos).
    let minute = std::time::Duration::from_secs(60);
    let ip = crate::limits::ip_from(&headers);
    if !crate::limits::allow(&format!("mp-notice|{ip}"), 120, minute) {
        return StatusCode::TOO_MANY_REQUESTS.into_response();
    }
    // Con la clave secreta cargada, solo valen los avisos firmados.
    if let Some(secret) = config_of(&pool, tenant)
        .await
        .and_then(|c| c.webhook_secret)
    {
        let header = |name: &str| {
            headers
                .get(name)
                .and_then(|v| v.to_str().ok())
                .unwrap_or("")
        };
        let data_id = q
            .get("data.id")
            .or(q.get("id"))
            .map(String::as_str)
            .unwrap_or("");
        let signed = signed_by_mp(
            &secret,
            header("x-signature"),
            header("x-request-id"),
            data_id,
        );
        // Sin `MP_WEBHOOK_STRICT` solo se anota: el pago igual se confirma
        // preguntándole a Mercado Pago, nunca por lo que diga el aviso.
        let strict = env("MP_WEBHOOK_STRICT").is_some_and(|v| v == "1");
        match (signed, strict) {
            (true, _) => leptos::logging::log!("Mercado Pago: aviso con firma válida."),
            (false, true) => {
                leptos::logging::log!("Mercado Pago: aviso sin firma válida, descartado.");
                return StatusCode::UNAUTHORIZED.into_response();
            }
            (false, false) => leptos::logging::log!(
                "Mercado Pago: aviso sin firma válida (se acepta: modo solo revisar; trae x-signature: {}).",
                !header("x-signature").is_empty()
            ),
        }
    }
    let v: Value = serde_json::from_slice(&body).unwrap_or(Value::Null);
    let kind = v["type"]
        .as_str()
        .or(q.get("type").map(String::as_str))
        .or(q.get("topic").map(String::as_str))
        .unwrap_or("");
    let id = match &v["data"]["id"] {
        Value::String(s) => Some(s.clone()),
        Value::Number(n) => Some(n.to_string()),
        _ => q.get("data.id").or(q.get("id")).cloned(),
    };
    if kind == "payment" {
        if let Some(id) = id {
            // El mismo pago avisado muchas veces seguidas: con unas pocas alcanza.
            if !crate::limits::allow(&format!("mp-pay|{tenant}|{id}"), 6, minute) {
                return StatusCode::OK.into_response();
            }
            if let Err(e) = check_payment(&pool, tenant, &id).await {
                leptos::logging::log!("Mercado Pago: aviso del pago {id}: {e}");
            }
        }
    }
    StatusCode::OK.into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hmac_matches_the_published_examples() {
        // RFC 4231, casos 2 y 6 (clave más larga que un bloque).
        assert_eq!(
            hmac_sha256_hex(b"Jefe", b"what do ya want for nothing?"),
            "5bdcc146bf60754e6a042426089575c75a003f089d2739839dec58b964ec3843"
        );
        assert_eq!(
            hmac_sha256_hex(
                &[0xaa; 131],
                b"Test Using Larger Than Block-Size Key - Hash Key First"
            ),
            "60e431591ee0b67f0d8a26aacbf5b77f8e0bc6213728c5140546040f0ee37f54"
        );
    }

    #[test]
    fn only_notices_signed_with_the_secret_pass() {
        let secret = "clave-secreta";
        let v1 = hmac_sha256_hex(
            secret.as_bytes(),
            b"id:123456;request-id:req-1;ts:1700000000;",
        );
        let signature = format!("ts=1700000000,v1={v1}");
        assert!(signed_by_mp(secret, &signature, "req-1", "123456"));
        // Otro pago, otra clave, o sin firma: no.
        assert!(!signed_by_mp(secret, &signature, "req-1", "999"));
        assert!(!signed_by_mp("otra", &signature, "req-1", "123456"));
        assert!(!signed_by_mp(secret, "", "req-1", "123456"));
        assert!(!signed_by_mp(
            secret,
            "ts=1700000000,v1=",
            "req-1",
            "123456"
        ));
    }
}
