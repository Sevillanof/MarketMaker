//! Límites por dirección IP para frenar a quien prueba cosas en serie:
//! emails en Entrar, contraseñas, pedidos falsos. Viven en la memoria del
//! servidor (se vacían al reiniciar), que alcanza para una sola máquina.

use std::collections::{HashMap, VecDeque};
use std::sync::{LazyLock, Mutex};
use std::time::{Duration, Instant};

static HITS: LazyLock<Mutex<HashMap<String, VecDeque<Instant>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// Ventana más larga que se usa; lo más viejo se puede olvidar.
const LONGEST: Duration = Duration::from_secs(60 * 60);

fn with_hits<T>(key: &str, window: Duration, f: impl FnOnce(&mut VecDeque<Instant>) -> T) -> T {
    let now = Instant::now();
    let mut map = HITS.lock().unwrap_or_else(|e| e.into_inner());
    if map.len() > 20_000 {
        map.retain(|_, v| v.back().is_some_and(|t| now.duration_since(*t) < LONGEST));
    }
    let hits = map.entry(key.to_string()).or_default();
    while hits
        .front()
        .is_some_and(|t| now.duration_since(*t) >= window)
    {
        hits.pop_front();
    }
    f(hits)
}

/// Anota un intento y dice si todavía está dentro del límite (`max` intentos
/// cada `window`). Si ya se pasó, no lo anota.
pub fn allow(key: &str, max: usize, window: Duration) -> bool {
    with_hits(key, window, |hits| {
        if hits.len() >= max {
            false
        } else {
            hits.push_back(Instant::now());
            true
        }
    })
}

/// Cuántos intentos hubo en la última `window`, sin anotar uno nuevo.
pub fn count(key: &str, window: Duration) -> usize {
    with_hits(key, window, |hits| hits.len())
}

/// Anota un intento (por ejemplo, una contraseña equivocada).
pub fn hit(key: &str) {
    with_hits(key, LONGEST, |hits| hits.push_back(Instant::now()));
}

/// Olvida los intentos de `key` (por ejemplo, al entrar bien).
pub fn clear(key: &str) {
    HITS.lock().unwrap_or_else(|e| e.into_inner()).remove(key);
}

/// La IP de quien hace el pedido. En producción la app solo escucha en
/// 127.0.0.1 detrás de Caddy, que pone la IP real al final de
/// `X-Forwarded-For` (y descarta lo que mande el navegador).
pub fn client_ip() -> String {
    use leptos::prelude::use_context;
    let Some(parts) = use_context::<http::request::Parts>() else {
        return "local".into();
    };
    parts
        .headers
        .get_all("x-forwarded-for")
        .iter()
        .filter_map(|v| v.to_str().ok())
        .flat_map(|v| v.split(','))
        .map(str::trim)
        .filter(|ip| !ip.is_empty())
        .last()
        .map(str::to_string)
        .unwrap_or_else(|| "local".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allows_up_to_max() {
        let key = "test|allow";
        let w = Duration::from_secs(60);
        assert!(allow(key, 2, w));
        assert!(allow(key, 2, w));
        assert!(!allow(key, 2, w));
        assert_eq!(count(key, w), 2);
        clear(key);
        assert_eq!(count(key, w), 0);
    }
}
