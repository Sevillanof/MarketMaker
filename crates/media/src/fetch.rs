//! Traer una foto desde un enlace (Dropbox, Google Drive, una imagen suelta
//! o una página que muestra una foto) para copiarla a Mis fotos.
//!
//! Es el servidor el que pide la dirección, así que se cuida de no entrar a
//! la red interna: solo `http`/`https` a los puertos 80 y 443, y las
//! direcciones se resuelven acá y se descartan las privadas (también en cada
//! redirección). Corta a los 10 MB y a los 15 segundos.

use std::net::{IpAddr, SocketAddr};
use std::time::Duration;

use crate::MAX_BYTES;

/// Cuánto puede tardar todo (incluida la página y la foto).
const TIMEOUT: Duration = Duration::from_secs(15);
/// Redirecciones que se siguen como mucho.
const MAX_REDIRECTS: usize = 5;
/// Lo que se lee de una página para buscar su foto.
const MAX_HTML: usize = 1024 * 1024;

/// Una foto traída de un enlace.
#[derive(Debug)]
pub struct Fetched {
    /// Nombre para Mis fotos (del archivo o del sitio).
    pub name: String,
    pub bytes: Vec<u8>,
}

/// Qué hacer con un enlace antes de pedirlo.
#[derive(Debug, PartialEq)]
pub enum Plan {
    /// Pedir esta dirección (quizás cambiada para bajar el archivo).
    Get(url::Url),
    /// Un sitio que no deja sacar sus fotos: el texto explica qué hacer.
    Blocked(&'static str),
}

const DOWNLOAD_IT: &str =
    "no deja que otros sitios saquen sus fotos. Bajala a tu compu y subila con Subir fotos.";

/// Revisa el enlace y lo cambia para bajar el archivo cuando hace falta
/// (Dropbox `dl=1`, Drive `uc?export=download`).
pub fn plan(link: &str) -> Result<Plan, String> {
    let link = link.trim();
    let mut url = url::Url::parse(link)
        .or_else(|_| url::Url::parse(&format!("https://{link}")))
        .map_err(|_| {
            "Ese enlace no parece una dirección web. Copialo de nuevo desde la barra del navegador."
                .to_string()
        })?;
    if !matches!(url.scheme(), "http" | "https") {
        return Err("El enlace tiene que empezar con https://".into());
    }
    if !url.username().is_empty() || url.password().is_some() {
        return Err("Ese enlace trae usuario y contraseña: no lo podemos usar.".into());
    }
    let host = url.host_str().unwrap_or_default().to_lowercase();
    let host = host.trim_start_matches("www.").to_string();
    let is = |h: &str| host == h || host.ends_with(&format!(".{h}"));

    if is("photos.google.com")
        || is("photos.app.goo.gl")
        || is("lh3.googleusercontent.com") && link.contains("/pw/")
    {
        return Ok(Plan::Blocked("Google Fotos"));
    }
    if is("instagram.com") {
        return Ok(Plan::Blocked("Instagram"));
    }
    if is("facebook.com") || is("fb.com") || is("fbcdn.net") {
        return Ok(Plan::Blocked("Facebook"));
    }
    if is("pinterest.com") || is("pin.it") || host.starts_with("pinterest.") {
        return Ok(Plan::Blocked("Pinterest"));
    }

    if is("dropbox.com") {
        // `dl=0` muestra una página; `dl=1` baja el archivo.
        let pairs: Vec<(String, String)> = url
            .query_pairs()
            .filter(|(k, _)| k != "dl" && k != "raw")
            .map(|(k, v)| (k.into_owned(), v.into_owned()))
            .collect();
        url.query_pairs_mut()
            .clear()
            .extend_pairs(pairs)
            .append_pair("dl", "1");
    } else if host == "drive.google.com" {
        // /file/d/<id>/view u open?id=<id> → descarga directa.
        let id = url
            .path_segments()
            .and_then(|mut s| {
                let parts: Vec<&str> = s.by_ref().collect();
                parts
                    .iter()
                    .position(|p| *p == "d")
                    .and_then(|i| parts.get(i + 1))
                    .map(|s| s.to_string())
            })
            .or_else(|| {
                url.query_pairs()
                    .find(|(k, _)| k == "id")
                    .map(|(_, v)| v.into_owned())
            });
        if let Some(id) = id.filter(|i| {
            i.chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        }) {
            url = url::Url::parse(&format!(
                "https://drive.google.com/uc?export=download&id={id}"
            ))
            .map_err(|e| e.to_string())?;
        }
    }
    Ok(Plan::Get(url))
}

/// Busca la foto principal de una página: `og:image`, `twitter:image` o
/// `<link rel="image_src">`. Devuelve la dirección completa.
pub fn page_image(html: &str, base: &url::Url) -> Option<url::Url> {
    let mut best: Option<(u8, String)> = None;
    for tag in html.split('<').skip(1) {
        let tag = tag.split('>').next().unwrap_or_default();
        let lower = tag.to_ascii_lowercase();
        let rank = if lower.starts_with("meta") {
            match attr(tag, "property")
                .or_else(|| attr(tag, "name"))
                .map(|s| s.to_ascii_lowercase())
                .as_deref()
            {
                Some("og:image:secure_url") => 0,
                Some("og:image") | Some("og:image:url") => 1,
                Some("twitter:image") | Some("twitter:image:src") => 2,
                _ => continue,
            }
        } else if lower.starts_with("link")
            && attr(tag, "rel").is_some_and(|r| r.eq_ignore_ascii_case("image_src"))
        {
            3
        } else {
            continue;
        };
        let value = if rank == 3 {
            attr(tag, "href")
        } else {
            attr(tag, "content")
        };
        if let Some(v) = value.filter(|v| !v.trim().is_empty()) {
            if best.as_ref().is_none_or(|(r, _)| rank < *r) {
                best = Some((rank, v));
            }
        }
    }
    let (_, href) = best?;
    let href = href.replace("&amp;", "&");
    base.join(href.trim())
        .ok()
        .filter(|u| matches!(u.scheme(), "http" | "https"))
}

/// El valor de un atributo de una etiqueta HTML (`content="…"`).
fn attr(tag: &str, name: &str) -> Option<String> {
    let lower = tag.to_ascii_lowercase();
    let mut from = 0;
    while let Some(i) = lower[from..].find(name) {
        let start = from + i;
        from = start + name.len();
        let before_ok = start == 0 || lower.as_bytes()[start - 1].is_ascii_whitespace();
        let rest = lower[from..].trim_start();
        if !before_ok || !rest.starts_with('=') {
            continue;
        }
        let offset = tag.len() - rest.len() + 1;
        let value = tag[offset..].trim_start();
        let (quote, body) = match value.chars().next()? {
            q @ ('"' | '\'') => (Some(q), &value[1..]),
            _ => (None, value),
        };
        let end = match quote {
            Some(q) => body.find(q)?,
            None => body
                .find(|c: char| c.is_whitespace() || c == '/')
                .unwrap_or(body.len()),
        };
        return Some(body[..end].to_string());
    }
    None
}

/// Nombre para Mis fotos a partir del enlace: el archivo (`remera-azul`) o
/// el sitio (`Foto de dropbox.com`).
pub fn name_from(url: &url::Url) -> String {
    let file = url
        .path_segments()
        .and_then(|mut s| s.next_back())
        .filter(|f| f.contains('.') && f.len() > 4)
        .map(|f| {
            let f = percent_decode(f);
            let stem = f.rsplit_once('.').map_or(f.as_str(), |(s, _)| s);
            stem.replace(['_', '-'], " ")
                .trim()
                .chars()
                .take(80)
                .collect::<String>()
        });
    match file {
        Some(f) if !f.is_empty() => f,
        _ => format!(
            "Foto de {}",
            url.host_str()
                .unwrap_or("internet")
                .trim_start_matches("www.")
        ),
    }
}

fn percent_decode(s: &str) -> String {
    url::form_urlencoded::parse(format!("x={s}").as_bytes())
        .next()
        .map(|(_, v)| v.into_owned())
        .unwrap_or_else(|| s.to_string())
}

/// ¿Es una dirección de la red interna (o de esta máquina)?
pub fn is_private(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => {
            v4.is_private()
                || v4.is_loopback()
                || v4.is_link_local()
                || v4.is_unspecified()
                || v4.is_broadcast()
                || v4.is_multicast()
                || v4.is_documentation()
                // 100.64.0.0/10 (CGNAT, Tailscale) y 0.0.0.0/8
                || (v4.octets()[0] == 100 && (v4.octets()[1] & 0xc0) == 64)
                || v4.octets()[0] == 0
        }
        IpAddr::V6(v6) => {
            if let Some(v4) = v6.to_ipv4_mapped() {
                return is_private(IpAddr::V4(v4));
            }
            let seg = v6.segments()[0];
            v6.is_loopback()
                || v6.is_unspecified()
                || v6.is_multicast()
                || (seg & 0xfe00) == 0xfc00 // fc00::/7
                || (seg & 0xffc0) == 0xfe80 // fe80::/10
        }
    }
}

/// Resuelve nombres descartando las direcciones privadas, así una
/// redirección o un DNS tramposo no llevan a la red interna.
#[derive(Clone, Copy)]
struct PublicOnly {
    allow_private: bool,
}

impl reqwest::dns::Resolve for PublicOnly {
    fn resolve(&self, name: reqwest::dns::Name) -> reqwest::dns::Resolving {
        let allow = self.allow_private;
        let host = name.as_str().to_string();
        Box::pin(async move {
            let addrs: Vec<SocketAddr> = tokio::net::lookup_host((host.as_str(), 0))
                .await?
                .filter(|a| allow || !is_private(a.ip()))
                .collect();
            if addrs.is_empty() {
                return Err("esa dirección es de una red privada".into());
            }
            Ok(Box::new(addrs.into_iter()) as reqwest::dns::Addrs)
        })
    }
}

/// Trae fotos de enlaces.
#[derive(Clone)]
pub struct Fetcher {
    client: reqwest::Client,
    allow_private: bool,
}

impl Fetcher {
    /// `allow_private` deja traer de la red local: solo para pruebas.
    pub fn new(allow_private: bool) -> Result<Self, String> {
        let client = reqwest::Client::builder()
            .no_proxy()
            .redirect(reqwest::redirect::Policy::none())
            .dns_resolver(PublicOnly { allow_private })
            .timeout(TIMEOUT)
            .connect_timeout(Duration::from_secs(8))
            .user_agent("Mozilla/5.0 (compatible; MarketMaker/1.0; fotos de productos)")
            .build()
            .map_err(|e| e.to_string())?;
        Ok(Self {
            client,
            allow_private,
        })
    }

    /// Trae la foto del enlace, o un problema en palabras simples.
    pub async fn fetch(&self, link: &str) -> Result<Fetched, String> {
        let url = match plan(link)? {
            Plan::Get(u) => u,
            Plan::Blocked(site) => return Err(format!("{site} {DOWNLOAD_IT}")),
        };
        let work = async {
            let (final_url, content_type, body) =
                self.get(url.clone(), MAX_HTML.max(MAX_BYTES)).await?;
            if looks_like_image(&content_type, &body) {
                return Ok(Fetched {
                    name: name_from(&url),
                    bytes: body,
                });
            }
            if content_type.contains("html") || body.starts_with(b"<") || body.starts_with(b"\n") {
                let html = String::from_utf8_lossy(&body[..body.len().min(MAX_HTML)]).into_owned();
                let Some(img) = page_image(&html, &final_url) else {
                    return Err("En esa página no encontramos una foto. Abrí la foto sola (clic derecho → Abrir imagen en una pestaña nueva) y pegá ese enlace.".to_string());
                };
                let (_, ct, bytes) = self.get(img.clone(), MAX_BYTES).await?;
                if looks_like_image(&ct, &bytes) {
                    return Ok(Fetched {
                        name: name_from(&img),
                        bytes,
                    });
                }
            }
            Err("Ese enlace no lleva a una foto. Fijate que sea la foto y no una carpeta o una página para entrar.".to_string())
        };
        tokio::time::timeout(TIMEOUT, work)
            .await
            .unwrap_or_else(|_| Err("El otro sitio tardó demasiado en responder. Probá de nuevo o bajá la foto y subila.".into()))
    }

    /// Pide `url` siguiendo redirecciones (revisando cada una) y lee hasta
    /// `limit` bytes. Devuelve la dirección final, el tipo y el cuerpo.
    async fn get(
        &self,
        mut url: url::Url,
        limit: usize,
    ) -> Result<(url::Url, String, Vec<u8>), String> {
        for _ in 0..=MAX_REDIRECTS {
            self.check_host(&url).await?;
            let res = self
                .client
                .get(url.as_str())
                .header(
                    "accept",
                    "image/avif,image/webp,image/*,text/html;q=0.8,*/*;q=0.5",
                )
                .send()
                .await
                .map_err(|e| {
                    if e.is_timeout() {
                        "El otro sitio tardó demasiado en responder.".to_string()
                    } else {
                        "No pudimos conectarnos con ese sitio. Revisá el enlace.".to_string()
                    }
                })?;
            let status = res.status();
            if status.is_redirection() {
                let next = res
                    .headers()
                    .get("location")
                    .and_then(|v| v.to_str().ok())
                    .and_then(|l| url.join(l).ok())
                    .ok_or("El sitio respondió con una redirección rota.")?;
                if !matches!(next.scheme(), "http" | "https") {
                    return Err("El sitio redirige a una dirección que no podemos usar.".into());
                }
                url = next;
                continue;
            }
            if status == reqwest::StatusCode::NOT_FOUND {
                return Err("Ese enlace no existe más (el sitio dice que no está).".into());
            }
            if status == reqwest::StatusCode::UNAUTHORIZED
                || status == reqwest::StatusCode::FORBIDDEN
            {
                return Err("Esa foto es privada. Compartila como \"cualquiera con el enlace\" y probá de nuevo.".into());
            }
            if !status.is_success() {
                return Err(format!(
                    "El sitio no nos dio la foto (respondió {}).",
                    status.as_u16()
                ));
            }
            if res.content_length().is_some_and(|n| n as usize > limit) {
                return Err("Esa foto pesa más de 10 MB, que es el máximo.".into());
            }
            let content_type = res
                .headers()
                .get("content-type")
                .and_then(|v| v.to_str().ok())
                .unwrap_or_default()
                .to_ascii_lowercase();
            let mut res = res;
            let mut body = Vec::new();
            while let Some(chunk) = res
                .chunk()
                .await
                .map_err(|_| "Se cortó la descarga de la foto. Probá de nuevo.".to_string())?
            {
                body.extend_from_slice(&chunk);
                if body.len() > limit {
                    return Err("Esa foto pesa más de 10 MB, que es el máximo.".into());
                }
            }
            return Ok((url, content_type, body));
        }
        Err("El sitio redirige demasiadas veces.".into())
    }

    /// Una dirección escrita como IP no pasa por el resolvedor: se revisa acá.
    async fn check_host(&self, url: &url::Url) -> Result<(), String> {
        if url.port().is_some_and(|p| p != 80 && p != 443) && !self.allow_private {
            return Err("Ese enlace usa un puerto raro: no lo podemos usar.".into());
        }
        let private = match url.host() {
            Some(url::Host::Ipv4(ip)) => is_private(IpAddr::V4(ip)),
            Some(url::Host::Ipv6(ip)) => is_private(IpAddr::V6(ip)),
            Some(url::Host::Domain(d)) => {
                d.eq_ignore_ascii_case("localhost")
                    || d.ends_with(".localhost")
                    || d.ends_with(".local")
                    || d.ends_with(".internal")
            }
            None => true,
        };
        if private && !self.allow_private {
            return Err("Ese enlace lleva a una red privada: no lo podemos usar.".into());
        }
        Ok(())
    }
}

/// ¿Es una imagen? Por el tipo que dice el sitio o por cómo empieza el
/// archivo (algunos sitios dicen `application/octet-stream`).
fn looks_like_image(content_type: &str, body: &[u8]) -> bool {
    if content_type.starts_with("image/") {
        return true;
    }
    body.starts_with(&[0xff, 0xd8, 0xff]) // JPEG
        || body.starts_with(b"\x89PNG")
        || body.starts_with(b"GIF8")
        || (body.len() > 12 && &body[0..4] == b"RIFF" && &body[8..12] == b"WEBP")
        || (body.len() > 12 && &body[4..8] == b"ftyp")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn get(link: &str) -> String {
        match plan(link).unwrap() {
            Plan::Get(u) => u.to_string(),
            Plan::Blocked(s) => format!("bloqueado {s}"),
        }
    }

    #[test]
    fn rewrites_dropbox_and_drive_links() {
        assert_eq!(
            get("https://www.dropbox.com/scl/fi/abc/remera.jpg?rlkey=xyz&dl=0"),
            "https://www.dropbox.com/scl/fi/abc/remera.jpg?rlkey=xyz&dl=1"
        );
        assert_eq!(
            get("https://drive.google.com/file/d/1AbC-d_E/view?usp=sharing"),
            "https://drive.google.com/uc?export=download&id=1AbC-d_E"
        );
        assert_eq!(
            get("https://drive.google.com/open?id=1AbC"),
            "https://drive.google.com/uc?export=download&id=1AbC"
        );
        assert_eq!(get("i.imgur.com/abc.png"), "https://i.imgur.com/abc.png");
    }

    #[test]
    fn says_which_sites_do_not_let_photos_out() {
        assert_eq!(
            get("https://www.instagram.com/p/xyz/"),
            "bloqueado Instagram"
        );
        assert_eq!(
            get("https://photos.app.goo.gl/abc"),
            "bloqueado Google Fotos"
        );
        assert_eq!(
            get("https://ar.pinterest.com/pin/1/"),
            "bloqueado Pinterest"
        );
        assert!(plan("ftp://x.com/a.jpg").is_err());
        assert!(plan("https://user:pw@x.com/a.jpg").is_err());
    }

    #[test]
    fn finds_the_photo_of_a_page() {
        let base = url::Url::parse("https://tienda.com/productos/remera").unwrap();
        let html = r#"<html><head>
            <meta name="twitter:image" content="https://cdn.tienda.com/tw.jpg">
            <meta property="og:image" content="/img/remera.jpg?w=1200&amp;q=80" />
            </head></html>"#;
        assert_eq!(
            page_image(html, &base).unwrap().as_str(),
            "https://tienda.com/img/remera.jpg?w=1200&q=80"
        );
        let html = r#"<link rel="image_src" href="https://x.com/a.png">"#;
        assert_eq!(
            page_image(html, &base).unwrap().as_str(),
            "https://x.com/a.png"
        );
        assert!(page_image("<p>hola</p>", &base).is_none());
        assert!(page_image(
            r#"<meta property="og:image" content="javascript:alert(1)">"#,
            &base
        )
        .is_none());
    }

    #[test]
    fn names_and_private_addresses() {
        let u = url::Url::parse("https://x.com/fotos/Remera%20azul.JPG?x=1").unwrap();
        assert_eq!(name_from(&u), "Remera azul");
        let u = url::Url::parse("https://www.dropbox.com/s/abc").unwrap();
        assert_eq!(name_from(&u), "Foto de dropbox.com");
        for ip in [
            "127.0.0.1",
            "10.1.2.3",
            "192.168.0.10",
            "169.254.169.254",
            "100.100.1.1",
            "::1",
            "fd00::1",
            "::ffff:10.0.0.1",
        ] {
            assert!(is_private(ip.parse().unwrap()), "{ip}");
        }
        for ip in ["8.8.8.8", "151.101.1.140", "2606:4700::1111"] {
            assert!(!is_private(ip.parse().unwrap()), "{ip}");
        }
        assert!(looks_like_image(
            "application/octet-stream",
            &[0xff, 0xd8, 0xff, 0xe0]
        ));
        assert!(!looks_like_image("text/html", b"<html>"));
    }
}
