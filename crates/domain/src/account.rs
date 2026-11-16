//! Cuentas de dueño: tipos que viajan entre el navegador y el servidor y
//! ayudantes puros (sin base ni red) para validar lo que escribe la persona.

use serde::{Deserialize, Serialize};

/// Quién está mirando la página, si entró con su cuenta.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Me {
    pub email: String,
    /// Cómo quiere que la llamemos ("" si no lo dijo).
    pub name: String,
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

/// Qué pasa con el email que escribió en el primer paso de Entrar.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum EmailLookup {
    /// No parece un email.
    BadEmail,
    /// No está en la lista de personas habilitadas (registro por invitación).
    NotAllowed,
    /// Ya tiene cuenta. Sin contraseña (cuentas de antes) entra con un código
    /// y elige una.
    Existing { name: String, has_password: bool },
    /// Es nuevo: crea su cuenta.
    New,
}

/// Resultado de entrar con email y contraseña.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum PasswordSignIn {
    /// Entró: ya tiene la sesión abierta. `next` es adónde ir.
    In { next: String },
    /// La contraseña no es esa (o no hay cuenta con ese email).
    Wrong,
    /// Muchos intentos fallidos: tiene que esperar `minutes` minutos o
    /// entrar con un código.
    Locked { minutes: u32 },
    /// La cuenta todavía no tiene contraseña: tiene que elegir una con un código.
    NoPassword,
}

/// Resultado de escribir el código y la contraseña nueva (crear la cuenta o
/// elegir otra contraseña).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum SetPassword {
    /// Listo y adentro. `next` es adónde ir.
    In { next: String },
    /// El código no es ese. Quedan `left` intentos.
    Wrong { left: u32 },
    /// El código venció o se usó: hay que pedir otro.
    Expired,
    /// La contraseña no sirve; el texto dice por qué.
    Weak { reason: String },
    /// El email no está habilitado.
    NotAllowed,
}

/// Lo que muestra "Mi cuenta".
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct AccountInfo {
    pub email: String,
    pub name: String,
    pub has_password: bool,
    /// En cuántas tiendas es dueña.
    pub stores: u32,
}

/// Resultado de cambiar la contraseña desde "Mi cuenta".
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum PasswordChange {
    Done,
    /// La contraseña actual no es esa.
    WrongCurrent,
    Weak {
        reason: String,
    },
}

/// Mínimo de caracteres de una contraseña.
pub const PASSWORD_MIN: usize = 8;
/// Máximo, para que nadie mande megabytes a la función de hash.
pub const PASSWORD_MAX: usize = 128;
/// Máximo de caracteres del nombre.
pub const ACCOUNT_NAME_MAX: usize = 60;

/// Si la contraseña sirve; si no, por qué, en palabras simples.
pub fn password_problem(password: &str) -> Option<String> {
    let n = password.chars().count();
    if n < PASSWORD_MIN {
        Some(format!(
            "La contraseña tiene que tener {PASSWORD_MIN} caracteres o más."
        ))
    } else if n > PASSWORD_MAX {
        Some(format!(
            "La contraseña puede tener hasta {PASSWORD_MAX} caracteres."
        ))
    } else if password.trim().is_empty() {
        Some("La contraseña no puede ser solo espacios.".into())
    } else {
        None
    }
}

/// Qué tan buena es una contraseña, de 0 a 4, para la barrita de colores.
/// Solo orienta: la regla que manda es [`password_problem`].
pub fn password_score(password: &str) -> u8 {
    let n = password.chars().count();
    if n == 0 {
        return 0;
    }
    if n < PASSWORD_MIN {
        return 1;
    }
    let kinds = [
        password.chars().any(|c| c.is_lowercase()),
        password.chars().any(|c| c.is_uppercase()),
        password.chars().any(|c| c.is_ascii_digit()),
        password.chars().any(|c| !c.is_alphanumeric()),
    ]
    .iter()
    .filter(|k| **k)
    .count();
    match (n, kinds) {
        (12.., 3..) | (16.., _) => 4,
        (_, 3..) | (12.., _) => 3,
        _ => 2,
    }
}

/// El nombre como lo escribe la persona: sin espacios de más y cortado al
/// máximo.
pub fn clean_name(raw: &str) -> String {
    raw.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(ACCOUNT_NAME_MAX)
        .collect()
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
    fn passwords_need_eight_characters() {
        assert!(password_problem("corta").is_some());
        assert!(password_problem("        ").is_some());
        assert!(password_problem("mate con facturas").is_none());
        assert!(password_problem(&"x".repeat(129)).is_some());
        assert_eq!(password_score(""), 0);
        assert_eq!(password_score("abc"), 1);
        assert_eq!(password_score("abcdefgh"), 2);
        assert_eq!(password_score("Abcdefg1"), 3);
        assert_eq!(password_score("Abcdefgh1234!"), 4);
    }

    #[test]
    fn names_are_tidied() {
        assert_eq!(clean_name("  Fran   Sevillano "), "Fran Sevillano");
        assert_eq!(clean_name(&"a".repeat(80)).len(), ACCOUNT_NAME_MAX);
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
