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
    /// Ya tiene cuenta: pide la contraseña. No se dice el nombre ni si tiene
    /// contraseña, para no contarle nada a quien prueba emails ajenos. Las
    /// cuentas de antes, sin contraseña, eligen una con un código.
    Existing,
    /// Demasiadas consultas seguidas desde la misma conexión.
    TooMany,
    /// Es nuevo: crea su cuenta.
    New,
}

/// Resultado de entrar con email y contraseña.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum PasswordSignIn {
    /// Entró: ya tiene la sesión abierta. `next` es adónde ir.
    In { next: String },
    /// La contraseña no es correcta (o no hay cuenta con ese email).
    Wrong,
    /// Muchos intentos fallidos: tiene que esperar `minutes` minutos o
    /// entrar con un código.
    Locked { minutes: u32 },
    /// La cuenta todavía no tiene contraseña: la elige una sola vez con el
    /// enlace que le mandamos por email (así confirma que el email es suyo).
    NoPassword,
    /// La cuenta del administrador no tenía contraseña y la que escribió no sirve
    /// para estrenarla; el texto dice por qué.
    Weak { reason: String },
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

/// Resultado de crear la cuenta con nombre y contraseña.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum AccountCreation {
    /// Cuenta creada y adentro. `next` es adónde ir.
    In { next: String },
    /// Hace falta confirmar el email con un código (no está invitado: entró
    /// por el registro abierto).
    NeedsCode,
    /// Ese email ya tiene cuenta: que entre con su contraseña.
    Exists,
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
    /// La foto de perfil, lista para un `<img>`.
    pub avatar: Option<String>,
    /// En cuántas tiendas es dueña.
    pub stores: u32,
}

/// La dirección de la foto de perfil `id` (la de 480 px alcanza para un círculo).
pub fn avatar_url(id: impl std::fmt::Display) -> String {
    format!("/media/{id}/480.webp")
}

/// Resultado de cambiar la contraseña desde "Mi cuenta".
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum PasswordChange {
    Done,
    /// La contraseña actual no es correcta.
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
    } else if is_common_password(password) {
        Some("Esa contraseña es muy común y se adivina fácil. Elegí otra.".into())
    } else {
        None
    }
}

/// Las contraseñas que más se prueban al intentar entrar en cuentas ajenas.
const COMMON_PASSWORDS: &[&str] = &[
    "password",
    "password1",
    "password12",
    "password123",
    "passw0rd",
    "contraseña",
    "contrasena",
    "contraseña1",
    "contrasena1",
    "micontraseña",
    "micontrasena",
    "qwerty12",
    "qwerty123",
    "qwertyui",
    "qwertyuiop",
    "asdfghjk",
    "asdfghjkl",
    "zxcvbnm1",
    "abcd1234",
    "abc12345",
    "a1b2c3d4",
    "iloveyou",
    "teamo123",
    "tequiero",
    "hola1234",
    "holahola",
    "bienvenido",
    "argentina",
    "argentina1",
    "argentina10",
    "bocajuniors",
    "riverplate",
    "messi1010",
    "maradona10",
    "futbol10",
    "dragon12",
    "superman",
    "batman123",
    "football",
    "baseball",
    "sunshine",
    "princess",
    "starwars",
    "welcome1",
    "letmein1",
    "trustno1",
    "admin123",
    "administrador",
    "marketmaker",
    "mkr12345",
    "12341234",
    "11223344",
    "12121212",
    "12344321",
    "1q2w3e4r",
    "1qaz2wsx",
];

/// Una contraseña común, una sola letra repetida o números seguidos.
pub fn is_common_password(password: &str) -> bool {
    let p = password.trim().to_lowercase();
    let chars: Vec<char> = p.chars().collect();
    let repeated = chars.windows(2).all(|w| w[0] == w[1]);
    let digits: Vec<u32> = chars.iter().filter_map(|c| c.to_digit(10)).collect();
    let sequence = digits.len() == chars.len()
        && (digits.windows(2).all(|w| (w[0] + 1) % 10 == w[1])
            || digits.windows(2).all(|w| (w[1] + 1) % 10 == w[0]));
    repeated || sequence || COMMON_PASSWORDS.contains(&p.as_str())
}

/// Qué tan buena es una contraseña, de 0 a 4, para la barrita de colores.
/// Solo orienta: la regla que manda es [`password_problem`].
pub fn password_score(password: &str) -> u8 {
    let n = password.chars().count();
    if n == 0 {
        return 0;
    }
    if n < PASSWORD_MIN || is_common_password(password) {
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
    let code: String = raw
        .chars()
        .filter(|c| !c.is_whitespace() && *c != '-')
        .collect();
    let typed = code.len() == 6 && code.chars().all(|c| c.is_ascii_digit());
    // La llave de un enlace: 32 letras y números en hexadecimal.
    let link = code.len() == 32 && code.chars().all(|c| matches!(c, '0'..='9' | 'a'..='f'));
    (typed || link).then_some(code)
}

/// El enlace del email para elegir contraseña: `/entrar?recuperar=…&codigo=…`
/// (y `&nuevo` si es para confirmar una cuenta nueva). `base` sin `/` final.
pub fn password_link(base: &str, email: &str, code: &str, new_account: bool) -> String {
    let enc = |v: &str| {
        v.bytes()
            .map(|b| match b {
                b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'.' | b'-' | b'_' | b'@' => {
                    (b as char).to_string()
                }
                _ => format!("%{b:02X}"),
            })
            .collect::<String>()
    };
    let base = base.trim_end_matches('/');
    let extra = if new_account { "&nuevo" } else { "" };
    format!(
        "{base}/entrar?recuperar={}&codigo={}{extra}",
        enc(email),
        enc(code)
    )
}

#[cfg(test)]
mod password_link_tests {
    #[test]
    fn encodes_email_and_code() {
        assert_eq!(
            super::password_link("https://mkr.com.ar/", "ana+x@correo.com", "123456", false),
            "https://mkr.com.ar/entrar?recuperar=ana%2Bx@correo.com&codigo=123456"
        );
        assert!(super::password_link("http://a", "a@b.c", "1", true).ends_with("&nuevo"));
    }
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

    #[test]
    fn common_passwords_are_rejected() {
        for p in [
            "password",
            "12345678",
            "aaaaaaaa",
            "87654321",
            "Argentina",
            "1234567890",
        ] {
            assert!(password_problem(p).is_some(), "{p}");
            assert_eq!(password_score(p), 1, "{p}");
        }
        assert!(password_problem("Huerta-2026-lindo").is_none());
    }

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
        assert_eq!(password_score("mate dulce"), 2);
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
        // La llave de un enlace: 32 en hexadecimal, tal cual.
        let key = "0123456789abcdef0123456789abcdef";
        assert_eq!(normalize_code(key), Some(key.into()));
        assert_eq!(normalize_code(&key[..31]), None);
        assert_eq!(normalize_code("0123456789abcdef0123456789abcdeg"), None);
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
