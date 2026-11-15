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
    fn names_from_files() {
        assert_eq!(media_name("C:\\fotos\\Remera azul.JPG"), "Remera azul");
        assert_eq!(media_name(".png"), "Foto");
        assert_eq!(media_name("sin-extension"), "sin-extension");
    }
}
