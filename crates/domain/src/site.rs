//! Qué sitio se está mirando según la dirección: la plataforma
//! (`localhost:3000`, donde se entra y se ven "Tus tiendas") o una tienda
//! (`nombre.localhost:3000`, más adelante `nombre.dominio.com`).

use serde::{Deserialize, Serialize};

/// El sitio de una dirección.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum Site {
    /// La plataforma: entrar, tus tiendas, crear una tienda.
    Platform,
    /// Una tienda, por su dirección (`nombre` en `nombre.localhost`).
    Store(String),
}

/// Nombres que no pueden ser la dirección de una tienda.
pub const RESERVED_STORE_SLUGS: &[&str] = &[
    "www",
    "app",
    "api",
    "admin",
    "mail",
    "email",
    "entrar",
    "ayuda",
    "status",
    "soporte",
    "blog",
    "panel",
    "cuenta",
    "tiendas",
    "static",
    "cdn",
    "dev",
    "test",
    "localhost",
    "marketmaker",
];

/// Cuántos nombres del final forman el dominio base de la plataforma:
/// `localhost` (1), `marketmaker.tienda` (2), `marketmaker.com.ar` (3) y,
/// mientras no haya dominio propio, la IP del servidor en `sslip.io` o
/// `nip.io` (`1-2-3-4.sslip.io`, 3).
fn base_labels(labels: &[&str]) -> usize {
    let n = labels.len();
    match labels {
        [.., "localhost"] => 1,
        [.., ip, "sslip" | "nip", "io"] if is_dashed_ip(ip) => 3,
        [.., "com" | "net" | "org" | "gob" | "edu", tld] if n >= 3 && tld.len() == 2 => 3,
        _ => 2,
    }
}

/// `1-2-3-4`, una IP escrita con guiones (como la usa sslip.io).
fn is_dashed_ip(label: &str) -> bool {
    label
        .replace('-', ".")
        .parse::<std::net::Ipv4Addr>()
        .is_ok()
}

/// Lee el sitio de un `Host` (`demo.localhost:3000` → `Store("demo")`).
///
/// El dominio base es `localhost` o los dos últimos nombres
/// (`marketmaker.tienda`): lo que hay antes es la tienda. Las IP, el dominio
/// base solo y `www`/`app` son la plataforma.
pub fn site_from_host(host: &str) -> Site {
    let host = host.trim().to_lowercase();
    // Sin puerto (y sin corchetes de IPv6).
    let name = if host.starts_with('[') {
        return Site::Platform;
    } else {
        host.rsplit_once(':').map_or(host.as_str(), |(h, _)| h)
    };
    if name.parse::<std::net::Ipv4Addr>().is_ok() {
        return Site::Platform;
    }
    let labels: Vec<&str> = name.split('.').filter(|l| !l.is_empty()).collect();
    let base = base_labels(&labels);
    if labels.len() <= base {
        return Site::Platform;
    }
    // Solo el nombre que está justo antes del dominio base.
    let slug = labels[labels.len() - base - 1];
    if slug == "www" || slug == "app" {
        Site::Platform
    } else {
        Site::Store(slug.to_string())
    }
}

/// Dirección de la tienda `slug` vista desde la plataforma en `platform_host`
/// (`localhost:3000` → `demo.localhost:3000`).
pub fn store_host(platform_host: &str, slug: &str) -> String {
    let host = platform_host.trim().to_lowercase();
    let host = host
        .strip_prefix("www.")
        .or_else(|| host.strip_prefix("app."))
        .unwrap_or(&host);
    // Las IP no tienen subdominios: se usa localhost con el mismo puerto.
    let (name, port) = match host.rsplit_once(':') {
        Some((h, p)) => (h, Some(p)),
        None => (host, None),
    };
    let name = if name.parse::<std::net::Ipv4Addr>().is_ok() {
        "localhost"
    } else {
        name
    };
    match port {
        Some(p) => format!("{slug}.{name}:{p}"),
        None => format!("{slug}.{name}"),
    }
}

/// La dirección de una tienda a partir de su nombre ("La Casa de Ana" →
/// `la-casa-de-ana`): sin tildes, solo letras, números y guiones (los
/// espacios y signos se vuelven un guion), hasta 30.
pub fn slug_from_name(name: &str) -> String {
    let mut out = String::new();
    for c in name.trim().to_lowercase().chars() {
        let c = match c {
            'á' | 'à' | 'ä' | 'â' => 'a',
            'é' | 'è' | 'ë' | 'ê' => 'e',
            'í' | 'ì' | 'ï' | 'î' => 'i',
            'ó' | 'ò' | 'ö' | 'ô' => 'o',
            'ú' | 'ù' | 'ü' | 'û' => 'u',
            'ñ' => 'n',
            'ç' => 'c',
            c => c,
        };
        if c.is_ascii_alphanumeric() {
            out.push(c);
        } else if !out.ends_with('-') && !out.is_empty() {
            out.push('-');
        }
    }
    let out: String = out.trim_matches('-').chars().take(30).collect();
    out.trim_end_matches('-').to_string()
}

/// `None` si `slug` sirve como dirección de tienda; si no, por qué no, en
/// palabras simples.
pub fn store_slug_problem(slug: &str) -> Option<&'static str> {
    if slug.len() < 3 {
        return Some("La dirección tiene que tener al menos 3 letras o números.");
    }
    if slug.len() > 30 {
        return Some("La dirección puede tener hasta 30 letras o números.");
    }
    if !slug
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        || slug.starts_with('-')
        || slug.ends_with('-')
        || slug.contains("--")
    {
        return Some("Usá solo letras sin tilde, números y guiones (por ejemplo, la-casa-de-ana).");
    }
    if RESERVED_STORE_SLUGS.contains(&slug) {
        return Some("Esa dirección está reservada. Probá con otra.");
    }
    None
}

/// Una tienda en "Tus tiendas".
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct StoreCard {
    pub name: String,
    pub slug: String,
    /// Dirección de la tienda, con `http://` o `https://`.
    pub url: String,
}

/// Una tienda en el selector del panel.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct NavStore {
    pub name: String,
    pub slug: String,
    /// Dirección de la tienda, con `http://` o `https://`.
    pub url: String,
    /// Pedidos pagados que falta preparar.
    pub to_prepare: i64,
}

/// Lo que muestra el menú del panel: quién entró y sus tiendas.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct PanelNav {
    pub email: String,
    /// Cómo quiere que la llamemos ("" si no lo dijo).
    pub name: String,
    /// La foto de perfil, lista para un `<img>`.
    pub avatar: Option<String>,
    pub stores: Vec<NavStore>,
    /// Administra MarketMaker: ve la Whitelist en su menú.
    pub is_admin: bool,
}

/// Una persona de la Whitelist, con lo que sabemos de su cuenta.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct AllowedPerson {
    pub email: String,
    /// "" si todavía no creó su cuenta o no dijo su nombre.
    pub name: String,
    pub avatar: Option<String>,
    pub has_account: bool,
    pub is_admin: bool,
    /// Se puede quitar de la lista (no la del administrador ni la propia).
    pub removable: bool,
    /// "Se unió hace 3 días" / "Agregada hoy", si se sabe.
    pub joined: String,
    /// "Último acceso ayer", si se sabe.
    pub last_seen: String,
    pub stores: Vec<StoreCard>,
    pub orders: i64,
}

/// Qué pasó al agregar o quitar un email de la Whitelist.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum WhitelistChange {
    Done,
    /// El email no es válido.
    Invalid,
    /// Ya estaba en la lista.
    Already,
    /// No se puede quitar (administrador o la propia cuenta).
    Protected,
}

/// "hoy", "ayer", "hace 3 días", "hace 2 semanas"… para `secs` segundos atrás.
pub fn ago_es(secs: i64) -> String {
    let days = secs.max(0) / 86_400;
    match days {
        0 => "hoy".into(),
        1 => "ayer".into(),
        2..=13 => format!("hace {days} días"),
        14..=59 => format!("hace {} semanas", days / 7),
        _ => format!("hace {} meses", days / 30),
    }
}

/// Si lo que ven los visitantes es lo último que guardó el dueño.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
pub enum PublishState {
    /// Nunca se publicó: los visitantes no ven nada todavía.
    Never,
    /// Hay cambios guardados que todavía no se publicaron.
    Changes,
    /// Los visitantes ven lo último.
    Published,
}

impl PublishState {
    /// `version` es la última guardada y `published` la publicada (0 = nunca).
    pub fn of(version: i32, published: i32) -> Self {
        if published == 0 {
            PublishState::Never
        } else if version > published {
            PublishState::Changes
        } else {
            PublishState::Published
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            PublishState::Never => "Sin publicar",
            PublishState::Changes => "Cambios sin publicar",
            PublishState::Published => "Publicada",
        }
    }
}

/// Inicio de una tienda en el panel: números, últimos pedidos y pendientes.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct StoreOverview {
    pub name: String,
    pub slug: String,
    pub url: String,
    pub publish: PublishState,
    /// Pedidos nuevos o esperando el pago.
    pub awaiting_payment: i64,
    /// Pedidos pagados que falta preparar.
    pub to_prepare: i64,
    /// Pedidos pagados (o ya enviados o entregados) de este mes, hora de Argentina.
    pub month_orders: i64,
    pub month_cents: i64,
    /// Productos propios que se ven en la tienda (sin contar los de ejemplo).
    pub products: i64,
    /// Productos de ejemplo que quedan de la tienda nueva.
    #[serde(default)]
    pub examples: i64,
    pub out_of_stock: i64,
    pub delivery_ready: bool,
    pub mp_linked: bool,
    /// Hay al menos una forma de cobro (Mercado Pago, transferencia o coordinar).
    #[serde(default)]
    pub pay_ready: bool,
    /// Los últimos pedidos, el más nuevo primero.
    pub recent: Vec<crate::Order>,
}

/// Los datos de la tienda que se ven en el diseño: el menú y el pie los usan
/// cuando sus campos quedan vacíos.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
pub struct StoreInfo {
    pub name: String,
    /// `/media/<id>/480.webp`, o vacío.
    pub logo_url: String,
    /// Como lo escribió el dueño, por ejemplo "11 5555-1234".
    pub whatsapp: String,
    /// Usuario sin @, por ejemplo "lahuerta".
    pub instagram: String,
    pub email: String,
}

impl StoreInfo {
    /// Enlace de WhatsApp (con el 54 de Argentina si falta). `None` si no hay.
    pub fn whatsapp_url(&self) -> Option<String> {
        let digits: String = self.whatsapp.chars().filter(char::is_ascii_digit).collect();
        if digits.len() < 8 {
            return None;
        }
        let full = if digits.starts_with("54") {
            digits
        } else {
            format!("549{}", digits.trim_start_matches('0'))
        };
        Some(format!("https://wa.me/{full}"))
    }

    pub fn instagram_url(&self) -> Option<String> {
        (!self.instagram.is_empty()).then(|| format!("https://instagram.com/{}", self.instagram))
    }
}

/// La pantalla "Datos de la tienda": los datos y el id del logo.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct StoreInfoForm {
    pub info: StoreInfo,
    pub logo_id: Option<String>,
}

pub const STORE_NAME_MAX: usize = 60;

/// Limpia los datos de la tienda (sin el logo) y devuelve los problemas, por
/// campo: `name`, `whatsapp`, `instagram` o `email`.
pub fn tidy_store_info(i: &mut StoreInfo) -> Vec<(&'static str, String)> {
    i.name = i.name.trim().to_string();
    i.whatsapp = i.whatsapp.trim().chars().take(30).collect();
    i.instagram = i
        .instagram
        .trim()
        .trim_start_matches("https://")
        .trim_start_matches("www.")
        .trim_start_matches("instagram.com/")
        .trim_start_matches('@')
        .trim_end_matches('/')
        .chars()
        .take(30)
        .collect();
    i.email = i.email.trim().to_lowercase().chars().take(120).collect();
    let mut e = Vec::new();
    if i.name.is_empty() {
        e.push(("name", "Escribí el nombre de tu tienda.".to_string()));
    } else if i.name.chars().count() > STORE_NAME_MAX {
        e.push((
            "name",
            format!("El nombre puede tener hasta {STORE_NAME_MAX} letras."),
        ));
    }
    if !i.whatsapp.is_empty() && i.whatsapp_url().is_none() {
        e.push((
            "whatsapp",
            "Revisá el número, con característica: por ejemplo 11 5555-1234.".to_string(),
        ));
    }
    if !i
        .instagram
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '_')
    {
        e.push((
            "instagram",
            "Escribí solo tu usuario, por ejemplo lahuerta.".to_string(),
        ));
    }
    if !i.email.is_empty() && !crate::looks_like_email(&i.email) {
        e.push((
            "email",
            "El email no es válido. Revisalo (por ejemplo, ana@correo.com).".to_string(),
        ));
    }
    e
}

/// Si una dirección de tienda está libre, mientras se escribe.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum SlugStatus {
    Free,
    /// La usa otra tienda; `suggestion` está libre.
    Taken {
        suggestion: String,
    },
    /// No sirve como dirección: el motivo.
    Problem(String),
}

/// Resultado de crear una tienda.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum NewStoreOutcome {
    /// Creada. `open` lleva a su editor con la sesión ya abierta.
    Created { open: String },
    /// Falta el nombre o la dirección no sirve: el motivo en palabras simples.
    Problem(String),
    /// La dirección ya la usa otra tienda; `suggestion` está libre.
    Taken { suggestion: String },
}

/// La plataforma vista desde una tienda (`demo.localhost:3000` →
/// `localhost:3000`).
pub fn platform_host(store_host: &str) -> String {
    match site_from_host(store_host) {
        Site::Platform => store_host.to_string(),
        Site::Store(slug) => {
            let host = store_host.to_lowercase();
            let at = host.find(&format!("{slug}.")).unwrap_or(0);
            host[at + slug.len() + 1..].to_string()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ages_are_written_in_spanish() {
        assert_eq!(ago_es(3_600), "hoy");
        assert_eq!(ago_es(90_000), "ayer");
        assert_eq!(ago_es(3 * 86_400), "hace 3 días");
        assert_eq!(ago_es(15 * 86_400), "hace 2 semanas");
        assert_eq!(ago_es(90 * 86_400), "hace 3 meses");
    }

    #[test]
    fn store_info_is_tidied_and_links_are_built() {
        let mut i = StoreInfo {
            name: "  La Huerta ".into(),
            whatsapp: "011 5555-1234".into(),
            instagram: "https://www.instagram.com/la.huerta/".into(),
            email: "Hola@LaHuerta.com".into(),
            ..StoreInfo::default()
        };
        assert!(tidy_store_info(&mut i).is_empty());
        assert_eq!(i.name, "La Huerta");
        assert_eq!(i.instagram, "la.huerta");
        assert_eq!(i.email, "hola@lahuerta.com");
        assert_eq!(i.whatsapp_url().unwrap(), "https://wa.me/5491155551234");
        assert_eq!(
            i.instagram_url().unwrap(),
            "https://instagram.com/la.huerta"
        );
        let mut bad = StoreInfo {
            whatsapp: "abc".into(),
            instagram: "mi tienda".into(),
            email: "x@".into(),
            ..StoreInfo::default()
        };
        let fields: Vec<_> = tidy_store_info(&mut bad)
            .into_iter()
            .map(|(f, _)| f)
            .collect();
        assert_eq!(fields, ["name", "whatsapp", "instagram", "email"]);
    }

    #[test]
    fn publish_state_compares_saved_and_published() {
        assert_eq!(PublishState::of(0, 0), PublishState::Never);
        assert_eq!(PublishState::of(3, 0), PublishState::Never);
        assert_eq!(PublishState::of(4, 3), PublishState::Changes);
        assert_eq!(PublishState::of(3, 3), PublishState::Published);
    }

    #[test]
    fn hosts_map_to_sites() {
        let store = |s: &str| Site::Store(s.into());
        assert_eq!(site_from_host("localhost:3000"), Site::Platform);
        assert_eq!(site_from_host("127.0.0.1:3000"), Site::Platform);
        assert_eq!(site_from_host("[::1]:3000"), Site::Platform);
        assert_eq!(site_from_host("demo.localhost:3000"), store("demo"));
        assert_eq!(site_from_host("Demo.LocalHost"), store("demo"));
        assert_eq!(site_from_host("marketmaker.tienda"), Site::Platform);
        assert_eq!(site_from_host("www.marketmaker.tienda"), Site::Platform);
        assert_eq!(site_from_host("ana.marketmaker.tienda"), store("ana"));
        assert_eq!(site_from_host("x.ana.localhost:3000"), store("ana"));
        assert_eq!(site_from_host("185-2-3-4.sslip.io"), Site::Platform);
        assert_eq!(site_from_host("ana.185-2-3-4.sslip.io"), store("ana"));
        assert_eq!(site_from_host("marketmaker.com.ar"), Site::Platform);
        assert_eq!(site_from_host("ana.marketmaker.com.ar"), store("ana"));
    }

    #[test]
    fn store_hosts_from_the_platform() {
        assert_eq!(store_host("localhost:3000", "ana"), "ana.localhost:3000");
        assert_eq!(
            store_host("185-2-3-4.sslip.io", "ana"),
            "ana.185-2-3-4.sslip.io"
        );
        assert_eq!(
            platform_host("ana.185-2-3-4.sslip.io"),
            "185-2-3-4.sslip.io"
        );
        assert_eq!(store_host("127.0.0.1:3000", "ana"), "ana.localhost:3000");
        assert_eq!(
            store_host("www.marketmaker.tienda", "ana"),
            "ana.marketmaker.tienda"
        );
    }

    #[test]
    fn platform_from_a_store() {
        assert_eq!(platform_host("demo.localhost:3000"), "localhost:3000");
        assert_eq!(
            platform_host("ana.marketmaker.tienda"),
            "marketmaker.tienda"
        );
        assert_eq!(platform_host("localhost:3000"), "localhost:3000");
    }

    #[test]
    fn slugs_from_names() {
        assert_eq!(slug_from_name("La Casa de Ana"), "la-casa-de-ana");
        assert_eq!(slug_from_name("  Ñandú & Café "), "nandu-cafe");
        assert_eq!(slug_from_name("mi-tienda--2"), "mi-tienda-2");
        assert_eq!(slug_from_name("¡¡!!"), "");
        assert_eq!(slug_from_name(&"a".repeat(40)).len(), 30);
    }

    #[test]
    fn slug_problems() {
        assert_eq!(store_slug_problem("lacasadeana"), None);
        assert_eq!(store_slug_problem("mi-tienda-2"), None);
        assert!(store_slug_problem("ab").is_some());
        assert!(store_slug_problem("Ana").is_some());
        assert!(store_slug_problem("-ana").is_some());
        assert!(store_slug_problem("www").is_some());
    }
}
