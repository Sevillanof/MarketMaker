//! Mis fotos: lo que el editor sabe de cada foto del dueño.

use serde::{Deserialize, Serialize};

/// Una foto de la biblioteca del dueño.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct MediaItem {
    pub id: String,
    /// Lo que se guarda en el bloque (`/media/<id>/960.webp`).
    pub url: String,
    /// El nombre del archivo que subió, sin la extensión.
    pub name: String,
    pub width: u32,
    pub height: u32,
    /// Lo que pesaba el archivo que subió el dueño.
    #[serde(default)]
    pub original_bytes: u64,
    /// Lo que baja un visitante: la foto de 960 px en AVIF.
    #[serde(default)]
    pub web_bytes: u64,
}

/// Peso legible: `3,4 MB`, `21 KB`.
pub fn format_bytes(bytes: u64) -> String {
    if bytes >= 1024 * 1024 {
        let tenths = (bytes * 10 + 512 * 1024) / (1024 * 1024);
        format!("{},{} MB", tenths / 10, tenths % 10)
    } else {
        format!("{} KB", bytes.div_ceil(1024).max(1))
    }
}

/// "Se achicó de 3,4 MB a 21 KB (99 % menos)", o `None` si no hay datos.
pub fn savings_text(item: &MediaItem) -> Option<String> {
    if item.original_bytes == 0 || item.web_bytes == 0 {
        return None;
    }
    let pct = item.original_bytes.saturating_sub(item.web_bytes) * 100 / item.original_bytes;
    let mut text = format!(
        "{}: de {} a {}",
        item.name,
        format_bytes(item.original_bytes),
        format_bytes(item.web_bytes)
    );
    if item.web_bytes < item.original_bytes {
        text.push_str(&format!(" ({pct} % menos)"));
    }
    Some(text)
}

/// Una foto con los lugares donde se usa, para la pantalla Mis fotos.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct MyPhoto {
    pub item: MediaItem,
    /// "Mi tienda · Inicio", uno por página que la usa (borrador o publicada).
    pub used_in: Vec<String>,
}

/// Cómo se llama una página para el dueño: `home` es "Inicio"; las demás,
/// su dirección con mayúscula (`ofertas` → "Ofertas").
pub fn page_label(slug: &str) -> String {
    if slug == "home" {
        return "Inicio".into();
    }
    let mut c = slug.chars();
    match c.next() {
        Some(f) => f
            .to_uppercase()
            .chain(c)
            .collect::<String>()
            .replace('-', " "),
        None => "Inicio".into(),
    }
}

/// Anchos en los que se guarda cada foto (ver el crate `media`).
pub const MEDIA_WIDTHS: [u32; 3] = [480, 960, 1600];

/// La dirección que usan los bloques para la foto `id`.
pub fn media_url(id: &str) -> String {
    format!("/media/{id}/960.webp")
}

/// Si `src` es una foto de la biblioteca, su carpeta (`/media/<id>`).
pub fn media_base(src: &str) -> Option<&str> {
    let rest = src.strip_prefix("/media/")?;
    let (id, file) = rest.split_once('/')?;
    let valid_id = !id.is_empty() && id.chars().all(|c| c.is_ascii_hexdigit() || c == '-');
    (valid_id && file.ends_with(".webp")).then(|| &src[..7 + id.len()])
}

/// Nombre para mostrar a partir del archivo subido: sin extensión, sin
/// carpetas, corto.
pub fn media_name(file_name: &str) -> String {
    let base = file_name.rsplit(['/', '\\']).next().unwrap_or(file_name);
    let stem = base.rsplit_once('.').map_or(base, |(s, _)| s).trim();
    let stem: String = stem.chars().filter(|c| !c.is_control()).take(80).collect();
    if stem.is_empty() {
        "Foto".to_string()
    } else {
        stem
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base_only_for_library_photos() {
        let id = "0b6f6c1e-8d3a-4f0e-9a51-3f1a2b3c4d5e";
        assert_eq!(
            media_base(&media_url(id)),
            Some(format!("/media/{id}").as_str())
        );
        assert_eq!(media_base("/img/hero.webp"), None);
        assert_eq!(media_base("/media/../etc/960.webp"), None);
        assert_eq!(media_base("https://x.com/media/a/960.webp"), None);
    }

    #[test]
    fn readable_sizes_and_savings() {
        assert_eq!(format_bytes(3_565_158), "3,4 MB");
        assert_eq!(format_bytes(21_000), "21 KB");
        assert_eq!(format_bytes(10), "1 KB");
        let item = MediaItem {
            id: "a".into(),
            url: "/media/a/960.webp".into(),
            name: "Remera".into(),
            width: 1,
            height: 1,
            original_bytes: 3_565_158,
            web_bytes: 21_000,
        };
        assert_eq!(
            savings_text(&item).unwrap(),
            "Remera: de 3,4 MB a 21 KB (99 % menos)"
        );
        assert_eq!(
            savings_text(&MediaItem {
                web_bytes: 0,
                ..item
            }),
            None
        );
    }

    #[test]
    fn names_from_files() {
        assert_eq!(media_name("C:\\fotos\\Remera azul.JPG"), "Remera azul");
        assert_eq!(media_name(".png"), "Foto");
        assert_eq!(media_name("sin-extension"), "sin-extension");
    }
}
