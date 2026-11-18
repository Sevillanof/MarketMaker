//! Cuentas de compradores (una por tienda): lo que el navegador sabe de quien
//! entró. Todo lo demás (sesiones, enlaces) vive en el servidor.

use serde::{Deserialize, Serialize};

/// El comprador que entró a su cuenta de la tienda.
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub struct BuyerMe {
    pub email: String,
    pub name: String,
    pub phone: String,
    pub address: String,
    pub postal_code: String,
    pub city: String,
    /// Foto de perfil (data URL chica), si puso una.
    #[serde(default)]
    pub avatar: Option<String>,
}

impl BuyerMe {
    /// La letra del avatar cuando no hay foto: la inicial del nombre, o del
    /// email si todavía no lo dijo.
    pub fn initial(&self) -> String {
        let source = if self.name.trim().is_empty() {
            &self.email
        } else {
            &self.name
        };
        source
            .trim()
            .chars()
            .next()
            .map(|c| c.to_uppercase().collect())
            .unwrap_or_else(|| "?".to_string())
    }

    /// Cómo mostrarle su nombre: el que puso, o la parte local del email.
    pub fn display_name(&self) -> String {
        if self.name.trim().is_empty() {
            self.email
                .split('@')
                .next()
                .unwrap_or(&self.email)
                .to_string()
        } else {
            self.name.trim().to_string()
        }
    }
}

/// Lo que el comprador puede cambiar de sus datos.
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub struct BuyerProfile {
    pub name: String,
    pub phone: String,
    pub address: String,
    pub postal_code: String,
    pub city: String,
}

/// Una foto de perfil válida: un data URL de imagen JPEG, PNG o WebP, en
/// base64 y de hasta `max` caracteres. Nunca SVG ni otro tipo.
pub fn valid_avatar(data_url: &str, max: usize) -> bool {
    let Some(rest) = data_url
        .strip_prefix("data:image/jpeg;base64,")
        .or_else(|| data_url.strip_prefix("data:image/png;base64,"))
        .or_else(|| data_url.strip_prefix("data:image/webp;base64,"))
    else {
        return false;
    };
    data_url.len() <= max
        && !rest.is_empty()
        && rest
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'+' | b'/' | b'='))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn avatar_only_accepts_small_raster_data_urls() {
        assert!(valid_avatar("data:image/webp;base64,AAAA", 100));
        assert!(valid_avatar("data:image/jpeg;base64,/9j/4AAQ", 100));
        assert!(!valid_avatar("data:image/svg+xml;base64,AAAA", 100));
        assert!(!valid_avatar("data:image/png;base64,AA\"onerror=x", 100));
        assert!(!valid_avatar("https://x.com/a.png", 100));
        assert!(!valid_avatar("data:image/png;base64,AAAA", 10));
        assert!(!valid_avatar("data:image/png;base64,", 100));
    }

    #[test]
    fn initial_and_name_fall_back_to_email() {
        let mut b = BuyerMe {
            email: "lucia@email.com".into(),
            ..Default::default()
        };
        assert_eq!(b.initial(), "L");
        assert_eq!(b.display_name(), "lucia");
        b.name = " Ángela Gómez ".into();
        assert_eq!(b.initial(), "Á");
        assert_eq!(b.display_name(), "Ángela Gómez");
    }
}
