//! Diseño de un bloque: uno de [`DESIGNS`] y un color.
//!
//! El color viaja al CSS en tres variables del envoltorio del bloque
//! (`.mk-look`, ver `style/tailwind.css`):
//! - `--mk-c`: el color, para rellenos, bordes e íconos;
//! - `--mk-cc`: el texto que va encima de ese relleno;
//! - `--mk-ct` (calculada en el CSS): el color para títulos sobre el fondo
//!   de la tienda, mezclado con el color del texto del tema para que se lea.
//!
//! Los colores del tema traen su par de texto de daisyUI (`--color-*-content`),
//! pensado para leerse en ese tema. Para un color propio (`#rrggbb`) el texto
//! se elige acá entre negro y blanco, el de más contraste (WCAG): con
//! cualquier color de fondo queda en 4,5:1 o más.

use serde::{Deserialize, Serialize};

/// Los cinco diseños: (número, nombre, descripción corta).
pub const DESIGNS: &[(u8, &str, &str)] = &[
    (1, "Clásico", "Fondo claro y botón de color"),
    (2, "Con borde", "Marco y títulos de color"),
    (3, "Suave", "Fondo con un toque de color"),
    (4, "Intenso", "Todo el bloque del color"),
    (5, "Minimal", "Sin caja, títulos con una línea"),
];

/// Colores del tema: (id, nombre). El vacío es el principal de la tienda.
pub const THEME_COLORS: &[(&str, &str)] = &[
    ("", "Tienda"),
    ("secondary", "Tienda 2"),
    ("accent", "Tienda 3"),
    ("neutral", "Oscuro"),
    ("info", "Celeste"),
    ("success", "Verde"),
    ("warning", "Amarillo"),
    ("error", "Rojo"),
];

/// Diseño y color de un bloque. El valor por defecto es el diseño de siempre.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default)]
pub struct Look {
    /// Número de [`DESIGNS`].
    pub design: u8,
    /// Id de [`THEME_COLORS`] o un color propio `#rrggbb`.
    pub color: String,
}

impl Default for Look {
    fn default() -> Self {
        Self {
            design: 1,
            color: String::new(),
        }
    }
}

impl Look {
    pub fn is_default(&self) -> bool {
        *self == Self::default()
    }

    /// Clases del envoltorio del bloque.
    pub fn classes(&self) -> String {
        format!("mk-look mk-d{}", self.design.clamp(1, 5))
    }

    /// Variables CSS del color. Vacío con el color principal (lo pone el CSS).
    pub fn css_vars(&self) -> String {
        if self.color.is_empty() {
            return String::new();
        }
        if let Some(rgb) = parse_hex(&self.color) {
            let text = if prefers_black_text(rgb) {
                "#000"
            } else {
                "#fff"
            };
            return format!("--mk-c:{}; --mk-cc:{text}; ", self.color.to_lowercase());
        }
        if THEME_COLORS.iter().any(|(id, _)| *id == self.color) {
            return format!(
                "--mk-c:var(--color-{0}); --mk-cc:var(--color-{0}-content); ",
                self.color
            );
        }
        String::new()
    }

    pub fn errors(&self, path: &str, errors: &mut Vec<String>) {
        if !DESIGNS.iter().any(|(n, _, _)| *n == self.design) {
            errors.push(format!("{path}Diseño: Elegí uno de los cinco."));
        }
        let known = THEME_COLORS.iter().any(|(id, _)| *id == self.color);
        if !known && parse_hex(&self.color).is_none() {
            errors.push(format!("{path}Color: Elegí uno de la lista o uno propio."));
        }
    }
}

/// `#rrggbb` → (r, g, b).
pub fn parse_hex(s: &str) -> Option<(u8, u8, u8)> {
    let hex = s.strip_prefix('#')?;
    if hex.len() != 6 || !hex.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    let ch = |i: usize| u8::from_str_radix(&hex[i..i + 2], 16).ok();
    Some((ch(0)?, ch(2)?, ch(4)?))
}

/// Luminancia relativa de WCAG 2.
pub fn luminance((r, g, b): (u8, u8, u8)) -> f64 {
    let lin = |c: u8| {
        let c = f64::from(c) / 255.0;
        if c <= 0.040_45 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * lin(r) + 0.7152 * lin(g) + 0.0722 * lin(b)
}

/// Contraste de WCAG entre dos luminancias (de 1 a 21).
pub fn contrast(a: f64, b: f64) -> f64 {
    let (hi, lo) = if a > b { (a, b) } else { (b, a) };
    (hi + 0.05) / (lo + 0.05)
}

/// Si sobre este fondo se lee mejor el texto negro que el blanco.
pub fn prefers_black_text(rgb: (u8, u8, u8)) -> bool {
    let l = luminance(rgb);
    contrast(l, 0.0) >= contrast(l, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn custom_colors_always_get_readable_text() {
        // Recorre el cubo de colores a saltos: el texto elegido siempre
        // queda en 4,5:1 o más.
        for r in (0..=255u16).step_by(17) {
            for g in (0..=255u16).step_by(17) {
                for b in (0..=255u16).step_by(17) {
                    let rgb = (r as u8, g as u8, b as u8);
                    let l = luminance(rgb);
                    let text = if prefers_black_text(rgb) { 0.0 } else { 1.0 };
                    assert!(contrast(l, text) >= 4.5, "{rgb:?}");
                }
            }
        }
    }

    #[test]
    fn css_vars_and_classes() {
        let l = Look::default();
        assert!(l.is_default());
        assert_eq!(l.css_vars(), "");
        assert_eq!(l.classes(), "mk-look mk-d1");
        let l = Look {
            design: 4,
            color: "secondary".into(),
        };
        assert_eq!(
            l.css_vars(),
            "--mk-c:var(--color-secondary); --mk-cc:var(--color-secondary-content); "
        );
        let yellow = Look {
            design: 2,
            color: "#FFD400".into(),
        };
        assert_eq!(yellow.css_vars(), "--mk-c:#ffd400; --mk-cc:#000; ");
        let navy = Look {
            design: 2,
            color: "#1e2a78".into(),
        };
        assert!(navy.css_vars().ends_with("--mk-cc:#fff; "));
    }

    #[test]
    fn rejects_unknown_values() {
        let mut errors = Vec::new();
        Look {
            design: 9,
            color: "rosa".into(),
        }
        .errors("", &mut errors);
        assert_eq!(errors.len(), 2);
        let mut errors = Vec::new();
        Look {
            design: 5,
            color: "#00aa00".into(),
        }
        .errors("", &mut errors);
        assert!(errors.is_empty());
    }
}
