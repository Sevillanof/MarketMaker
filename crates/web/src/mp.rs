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
//!   Sin ella, el pago se confirma cuando el comprador vuelve a la tienda.
//! - `MP_TEST=1`: pide credenciales de prueba al vincular.

use axum::extract::{Path, Query, State};
use axum::http::{header, StatusCode};
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
    pub test: bool,
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
    Some(Config {
        client_id: env("MP_CLIENT_ID")?,
        client_secret: env("MP_CLIENT_SECRET")?,
        redirect_url: env("MP_REDIRECT_URL")
            .unwrap_or_else(|| "http://localhost:3000/mp/vuelta".into()),
        public_url: env("MP_PUBLIC_URL")
            .filter(|u| u.starts_with("https://"))
            .map(|u| u.trim_end_matches('/').to_string()),
        test: env("MP_TEST").as_deref() == Some("1"),
        // Se pueden cambiar para probar sin el Mercado Pago real.
        auth_url: env("MP_AUTH_URL").unwrap_or_else(|| "https://auth.mercadopago.com.ar".into()),
        api_url: env("MP_API_URL").unwrap_or_else(|| "https://api.mercadopago.com".into()),
    })
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
        if let Some(c) = config() {
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
        "metadata": { "order_number": order.number },
    });
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
            if db::mp::mark_paid(pool, tenant, order, payment_id, cents)
                .await
                .map_err(|e| e.to_string())?
            {
                leptos::logging::log!("Mercado Pago: pedido {order} pagado (pago {payment_id})");
                crate::email::order_emails(pool, tenant, order, true).await;
            }
        }
    }
    Ok(status)
}

/// `GET /mp/vuelta` (vuelve de vincular) y `POST /mp/aviso/{tienda}` (avisos
/// de pago de Mercado Pago).
pub fn routes<S: Clone + Send + Sync + 'static>(pool: db::Pool) -> Router<S> {
    Router::new()
        .route("/mp/vuelta", get(linked))
        .route("/mp/aviso/{tenant}", post(notice))
        .with_state(pool)
}

/// Vuelve de Mercado Pago después de aceptar (o no) vincular la cuenta.
async fn linked(
    State(pool): State<db::Pool>,
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
    let tenant = match db::mp::take_state(&pool, state).await {
        Ok(Some(t)) => t,
        _ => return fail(None, ""),
    };
    let slug = db::mp::slug_of(&pool, tenant)
        .await
        .ok()
        .flatten()
        .unwrap_or_default();
    let Some(code) = q.get("code") else {
        // Tocó "Cancelar" en Mercado Pago.
        return fail(Some(&slug), "cancelado");
    };
    let Some(c) = config() else {
        return fail(Some(&slug), "configuracion");
    };
    let mut body = json!({
        "client_id": c.client_id,
        "client_secret": c.client_secret,
        "grant_type": "authorization_code",
        "code": code,
        "redirect_uri": c.redirect_url,
    });
    if c.test {
        body["test_token"] = json!(true);
    }
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

/// Aviso de Mercado Pago: "cambió el pago N". Se pregunta por el pago y se
/// actualiza el pedido. Siempre responde 200 para que no reintente de más.
async fn notice(
    State(pool): State<db::Pool>,
    Path(tenant): Path<Uuid>,
    Query(q): Query<HashMap<String, String>>,
    body: axum::body::Bytes,
) -> Response {
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
            if let Err(e) = check_payment(&pool, tenant, &id).await {
                leptos::logging::log!("Mercado Pago: aviso del pago {id}: {e}");
            }
        }
    }
    StatusCode::OK.into_response()
}
