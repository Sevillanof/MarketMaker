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
    /// Qué se ve en la foto, para buscadores y quien no ve la pantalla.
    #[serde(default)]
    pub alt: String,
    /// Al subirla: ya estaba en Mis fotos y se usó esa.
    #[serde(default)]
    pub duplicate: bool,
}

/// Cuántas fotos puede tener cada usuario en Mis fotos.
pub const PHOTOS_MAX_PER_OWNER: u64 = 1000;
/// Cuánto pueden ocupar (todos sus tamaños) las fotos de cada usuario.
pub const PHOTO_BYTES_MAX_PER_OWNER: u64 = 500 * 1024 * 1024;
/// Largo máximo de la descripción de una foto.
pub const PHOTO_ALT_MAX: usize = 150;

/// Cuánto usa el usuario de Mis fotos y cuánto le queda.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
pub struct PhotoQuota {
    pub photos: u64,
    pub bytes: u64,
}

impl PhotoQuota {
    /// Qué tan lleno está, de 0 a 100 (lo que esté más cerca del límite).
    pub fn percent(&self) -> u64 {
        let by_photos = self.photos * 100 / PHOTOS_MAX_PER_OWNER;
        let by_bytes = self.bytes * 100 / PHOTO_BYTES_MAX_PER_OWNER;
        by_photos.max(by_bytes).min(100)
    }

    /// "Usás 23,4 MB de 500 MB · 34 de 1000 fotos".
    pub fn text(&self) -> String {
        format!(
            "Usás {} de {} · {} de {} fotos",
            format_bytes(self.bytes),
            format_bytes(PHOTO_BYTES_MAX_PER_OWNER),
            self.photos,
            PHOTOS_MAX_PER_OWNER
        )
    }

    /// "Te quedan 476,6 MB y 966 fotos".
    pub fn left_text(&self) -> String {
        let bytes = PHOTO_BYTES_MAX_PER_OWNER.saturating_sub(self.bytes);
        let photos = PHOTOS_MAX_PER_OWNER.saturating_sub(self.photos);
        format!("Te quedan {} y {photos} fotos", format_bytes(bytes))
    }

    /// Si se puede sumar una foto que ocupa `bytes`; si no, por qué.
    pub fn room_for(&self, bytes: u64) -> Result<(), String> {
        if self.photos >= PHOTOS_MAX_PER_OWNER {
            return Err(format!(
                "Ya tenés {} fotos, que es el máximo. Borrá alguna que no uses para subir otra.",
                self.photos
            ));
        }
        if self.bytes + bytes > PHOTO_BYTES_MAX_PER_OWNER {
            return Err(format!(
                "Llegaste a los {} de Mis fotos. Borrá alguna que no uses para subir otra.",
                format_bytes(PHOTO_BYTES_MAX_PER_OWNER)
            ));
        }
        Ok(())
    }
}

/// La pantalla Mis fotos: las fotos y cuánto se usa.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct MyLibrary {
    pub photos: Vec<MyPhoto>,
    pub quota: PhotoQuota,
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
    if slug == "logo:" {
        return "Logo de la tienda".into();
    }
    if let Some(name) = slug.strip_prefix("producto:") {
        return format!("Producto «{name}»");
    }
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
    #[test]
    fn quota_texts_and_limits() {
        let q = super::PhotoQuota {
            photos: 34,
            bytes: 24 * 1024 * 1024,
        };
        assert_eq!(q.text(), "Usás 24,0 MB de 500,0 MB · 34 de 1000 fotos");
        assert_eq!(q.left_text(), "Te quedan 476,0 MB y 966 fotos");
        assert_eq!(q.percent(), 4);
        assert!(q.room_for(1024).is_ok());
        assert!(q.room_for(480 * 1024 * 1024).is_err());
        let full = super::PhotoQuota {
            photos: 1000,
            bytes: 0,
        };
        assert_eq!(full.percent(), 100);
        assert!(full.room_for(1).is_err());
    }

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
            alt: String::new(),
            duplicate: false,
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
