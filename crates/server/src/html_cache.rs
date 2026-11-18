//! Caché corto de las páginas públicas.
//!
//! La app arma cada página de la tienda de nuevo en cada visita, aunque
//! quien la mira sea un visitante sin sesión que siempre ve lo mismo. Acá se
//! guarda el HTML de esos pedidos unos segundos (30 por defecto, o
//! `HTML_CACHE_SECS`; con 0 se apaga) y se reparte a los siguientes.
//!
//! Solo se guarda lo que es seguro compartir:
//! - pedidos GET sin ninguna cookie (nadie con sesión, ni el dueño);
//! - respuestas 200 de tipo HTML que no ponen cookies;
//! - nunca el panel, el editor, la cuenta, el pedido ni la compra.
//!
//! Costo: lo que el dueño publica o el stock que cambia puede tardar hasta
//! ese tiempo en verse para los visitantes.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use axum::body::{to_bytes, Body, Bytes};
use axum::extract::{Request, State};
use axum::http::{header, HeaderMap, HeaderValue, Method, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};

/// Páginas más grandes que esto no se guardan.
const MAX_ENTRY: usize = 512 * 1024;
/// Páginas más grandes que esto no se arman en memoria (se corta con error).
const MAX_BUFFER: usize = 8 * 1024 * 1024;
/// Cantidad máxima de páginas guardadas a la vez.
const MAX_ENTRIES: usize = 500;

/// Rutas que dependen de quién mira o que cambian algo: nunca se guardan.
const NEVER: [&str; 12] = [
    "/api", "/media", "/mp", "/tls-ok", "/admin", "/editor", "/tienda", "/cuenta", "/comprar",
    "/pedido", "/entrar", "/envios",
];

struct Entry {
    at: Instant,
    status: StatusCode,
    headers: HeaderMap,
    body: Bytes,
}

#[derive(Clone)]
pub struct HtmlCache {
    ttl: Duration,
    map: Arc<Mutex<HashMap<String, Entry>>>,
}

impl HtmlCache {
    pub fn from_env() -> Self {
        let secs = std::env::var("HTML_CACHE_SECS")
            .ok()
            .and_then(|v| v.trim().parse::<u64>().ok())
            .map_or(30, |n| n.min(600));
        Self {
            ttl: Duration::from_secs(secs),
            map: Arc::default(),
        }
    }

    fn get(&self, key: &str) -> Option<Response> {
        let map = self.map.lock().unwrap_or_else(|e| e.into_inner());
        let e = map.get(key).filter(|e| e.at.elapsed() < self.ttl)?;
        let mut res = Response::new(Body::from(e.body.clone()));
        *res.status_mut() = e.status;
        *res.headers_mut() = e.headers.clone();
        res.headers_mut()
            .insert("x-cache", HeaderValue::from_static("HIT"));
        Some(res)
    }

    fn put(&self, key: String, status: StatusCode, headers: HeaderMap, body: Bytes) {
        let mut map = self.map.lock().unwrap_or_else(|e| e.into_inner());
        if map.len() >= MAX_ENTRIES {
            let ttl = self.ttl;
            map.retain(|_, e| e.at.elapsed() < ttl);
            if map.len() >= MAX_ENTRIES {
                map.clear();
            }
        }
        map.insert(
            key,
            Entry {
                at: Instant::now(),
                status,
                headers,
                body,
            },
        );
    }
}

/// ¿Este pedido se puede atender con una página guardada?
fn cacheable_request(req: &Request) -> bool {
    if req.method() != Method::GET {
        return false;
    }
    let h = req.headers();
    if h.contains_key(header::COOKIE) || h.contains_key(header::AUTHORIZATION) {
        return false;
    }
    let path = req.uri().path();
    !NEVER.iter().any(|p| {
        path == *p || path.strip_prefix(p).is_some_and(|rest| rest.starts_with('/'))
    })
}

/// ¿Esta respuesta es segura de compartir con otros visitantes?
fn cacheable_response(res: &Response) -> bool {
    let h = res.headers();
    res.status() == StatusCode::OK
        && !h.contains_key(header::SET_COOKIE)
        && h.get(header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .is_some_and(|v| v.starts_with("text/html"))
        && !h
            .get(header::CACHE_CONTROL)
            .and_then(|v| v.to_str().ok())
            .is_some_and(|v| v.contains("no-store") || v.contains("private"))
}

pub async fn middleware(State(cache): State<HtmlCache>, req: Request, next: Next) -> Response {
    if cache.ttl.is_zero() || !cacheable_request(&req) {
        return next.run(req).await;
    }
    let host = req
        .headers()
        .get(header::HOST)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_lowercase();
    let key = format!(
        "{host}{}",
        req.uri().path_and_query().map_or("/", |p| p.as_str())
    );
    if let Some(hit) = cache.get(&key) {
        return hit;
    }
    let res = next.run(req).await;
    if !cacheable_response(&res) {
        return res;
    }
    let (mut parts, body) = res.into_parts();
    let Ok(bytes) = to_bytes(body, MAX_BUFFER).await else {
        return (StatusCode::INTERNAL_SERVER_ERROR, "No pudimos armar la página.").into_response();
    };
    // El largo lo recalcula el servidor con el cuerpo completo.
    parts.headers.remove(header::CONTENT_LENGTH);
    if bytes.len() <= MAX_ENTRY {
        cache.put(key, parts.status, parts.headers.clone(), bytes.clone());
    }
    parts
        .headers
        .insert("x-cache", HeaderValue::from_static("MISS"));
    Response::from_parts(parts, Body::from(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn get(path: &str, cookie: bool) -> Request {
        let mut b = Request::builder().method("GET").uri(path);
        if cookie {
            b = b.header(header::COOKIE, "mk_session=x");
        }
        b.body(Body::empty()).unwrap()
    }

    #[test]
    fn only_anonymous_public_pages() {
        assert!(cacheable_request(&get("/", false)));
        assert!(cacheable_request(&get("/ofertas", false)));
        assert!(cacheable_request(&get("/terminos", false)));
        assert!(!cacheable_request(&get("/", true)));
        for p in ["/editor", "/tienda/ana/pedidos", "/api/x", "/pedido/abc", "/comprar", "/entrar", "/cuenta", "/tls-ok"] {
            assert!(!cacheable_request(&get(p, false)), "{p}");
        }
        let post = Request::builder().method("POST").uri("/").body(Body::empty()).unwrap();
        assert!(!cacheable_request(&post));
    }

    #[test]
    fn responses_with_cookies_or_errors_are_not_shared() {
        let html = |status: StatusCode, set_cookie: bool| {
            let mut r = Response::new(Body::empty());
            *r.status_mut() = status;
            r.headers_mut()
                .insert(header::CONTENT_TYPE, HeaderValue::from_static("text/html; charset=utf-8"));
            if set_cookie {
                r.headers_mut()
                    .insert(header::SET_COOKIE, HeaderValue::from_static("a=b"));
            }
            r
        };
        assert!(cacheable_response(&html(StatusCode::OK, false)));
        assert!(!cacheable_response(&html(StatusCode::OK, true)));
        assert!(!cacheable_response(&html(StatusCode::NOT_FOUND, false)));
    }
}
