//! Cuentas de dueño: tipos que viajan entre el navegador y el servidor y
//! ayudantes puros (sin base ni red) para validar lo que escribe la persona.

use serde::{Deserialize, Serialize};

/// Quién está mirando la página, si entró con su cuenta.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Me {
    pub email: String,
    /// Puede editar y publicar la tienda que está mirando.
    pub can_edit: bool,
}

/// Resultado de pedir un código para entrar.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum CodeRequest {
    /// Se mandó un código a ese email.
    Sent,
    /// No parece un email.
    BadEmail,
    /// El email no está en la lista de personas habilitadas.
    NotAllowed,
    /// Pidió demasiados códigos en la última hora.
    TooMany,
}

/// Resultado de escribir el código.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum CodeCheck {
    /// Entró: ya tiene la sesión abierta. `next` es adónde ir.
    In { next: String },
    /// El código no es ese. Quedan `left` intentos.
    Wrong { left: u32 },
    /// Venció, se usó o se gastaron los intentos: hay que pedir otro.
    Expired,
}

/// Emails: sin espacios alrededor y en minúsculas. `None` si no parece un email.
pub fn normalize_email(raw: &str) -> Option<String> {
    let email = raw.trim().to_lowercase();
    let (user, host) = email.split_once('@')?;
    let ok = !user.is_empty()
        && host.contains('.')
        && !host.starts_with('.')
        && !host.ends_with('.')
        && !host.contains('@')
        && email.len() <= 254
        && !email.chars().any(char::is_whitespace);
    ok.then_some(email)
}

/// El código tal como lo escribe la persona ("482 913", "482-913") → "482913".
/// `None` si no son 6 dígitos.
pub fn normalize_code(raw: &str) -> Option<String> {
    let digits: String = raw
        .chars()
        .filter(|c| !c.is_whitespace() && *c != '-')
        .collect();
    (digits.len() == 6 && digits.chars().all(|c| c.is_ascii_digit())).then_some(digits)
}

/// Adónde volver después de entrar. Solo direcciones de este mismo sitio
/// (`/editor`, `/ofertas`); cualquier otra cosa lleva al editor.
pub fn safe_next(raw: &str) -> String {
    let ok = raw.starts_with('/')
        && !raw.starts_with("//")
        && !raw.contains('\\')
        && !raw.chars().any(char::is_control);
    if ok {
        raw.to_string()
    } else {
        "/editor".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn emails_are_trimmed_and_lowercased() {
        assert_eq!(
            normalize_email("  Ana@Correo.COM "),
            Some("ana@correo.com".into())
        );
        for bad in [
            "",
            "ana",
            "ana@",
            "@correo.com",
            "ana@correo",
            "a b@c.com",
            "a@b@c.com",
            "a@.com",
        ] {
            assert_eq!(normalize_email(bad), None, "{bad}");
        }
    }

    #[test]
    fn codes_accept_spaces_and_dashes() {
        assert_eq!(normalize_code("482 913"), Some("482913".into()));
        assert_eq!(normalize_code("482-913"), Some("482913".into()));
        assert_eq!(normalize_code("48291"), None);
        assert_eq!(normalize_code("48291a"), None);
    }

    #[test]
    fn next_only_goes_to_this_site() {
        assert_eq!(safe_next("/ofertas"), "/ofertas");
        for bad in [
            "",
            "https://malo.com",
            "//malo.com",
            "/\\malo.com",
            "editor",
        ] {
            assert_eq!(safe_next(bad), "/editor", "{bad}");
        }
    }
}
