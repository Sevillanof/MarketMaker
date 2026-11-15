//! Letras de la tienda: fuentes, tamaño y estilo del texto.
//!
//! Hay dos niveles. [`Typography`] vale para toda la tienda (fuente de los
//! títulos, fuente del texto y tamaño general). [`TextStyle`] cambia un
//! bloque en particular; lo que queda vacío se toma de la tienda.
//!
//! La lista de fuentes es corta y elegida a mano: todas se leen bien, tienen
//! acentos y eñes, y se sirven desde `/fonts` (sin depender de terceros).

use serde::{Deserialize, Serialize};

/// Una fuente de la lista.
#[derive(Debug, PartialEq)]
pub struct Font {
    pub id: &'static str,
    /// Nombre para mostrar en el editor.
    pub label: &'static str,
    /// Para qué sirve, en pocas palabras.
    pub hint: &'static str,
    /// Valor de `font-family`, con respaldos.
    pub css: &'static str,
}

/// Fuentes disponibles. La primera es la de siempre (la del sistema).
pub const FONTS: &[Font] = &[
    Font {
        id: "system",
        label: "Simple",
        hint: "La del dispositivo de cada visitante",
        css: "ui-sans-serif, system-ui, sans-serif",
    },
    Font {
        id: "inter",
        label: "Inter",
        hint: "Moderna y neutra",
        css: "'Inter Variable', ui-sans-serif, system-ui, sans-serif",
    },
    Font {
        id: "montserrat",
        label: "Montserrat",
        hint: "Geométrica, con presencia",
        css: "'Montserrat Variable', ui-sans-serif, system-ui, sans-serif",
    },
    Font {
        id: "nunito",
        label: "Nunito",
        hint: "Redondeada y amigable",
        css: "'Nunito Variable', ui-sans-serif, system-ui, sans-serif",
    },
    Font {
        id: "atkinson",
        label: "Atkinson",
        hint: "Pensada para leer fácil",
        css: "'Atkinson Hyperlegible Next Variable', ui-sans-serif, system-ui, sans-serif",
    },
    Font {
        id: "lora",
        label: "Lora",
        hint: "Clásica, con serifa",
        css: "'Lora Variable', ui-serif, Georgia, serif",
    },
    Font {
        id: "playfair",
        label: "Playfair",
        hint: "Elegante, para títulos",
        css: "'Playfair Display Variable', ui-serif, Georgia, serif",
    },
    Font {
        id: "oswald",
        label: "Oswald",
        hint: "Angosta y fuerte, para títulos",
        css: "'Oswald Variable', ui-sans-serif, system-ui, sans-serif",
    },
];

pub fn font(id: &str) -> Option<&'static Font> {
    FONTS.iter().find(|f| f.id == id)
}

/// `font-family` de una fuente; si no se conoce, la del sistema.
pub fn font_css(id: &str) -> &'static str {
    font(id).unwrap_or(&FONTS[0]).css
}

/// Combinación lista para usar: fuente de títulos y fuente del texto.
#[derive(Debug, PartialEq)]
pub struct FontPair {
    pub label: &'static str,
    pub heading: &'static str,
    pub body: &'static str,
}

/// Combinaciones sugeridas, para elegir de un clic.
pub const FONT_PAIRS: &[FontPair] = &[
    FontPair {
        label: "Simple",
        heading: "system",
        body: "system",
    },
    FontPair {
        label: "Moderna",
        heading: "inter",
        body: "inter",
    },
    FontPair {
        label: "Con presencia",
        heading: "montserrat",
        body: "inter",
    },
    FontPair {
        label: "Amigable",
        heading: "nunito",
        body: "nunito",
    },
    FontPair {
        label: "Clásica",
        heading: "lora",
        body: "lora",
    },
    FontPair {
        label: "Elegante",
        heading: "playfair",
        body: "inter",
    },
    FontPair {
        label: "Impacto",
        heading: "oswald",
        body: "inter",
    },
    FontPair {
        label: "Fácil de leer",
        heading: "atkinson",
        body: "atkinson",
    },
];

/// Tamaño general del texto de la tienda: `(id, nombre, factor)`.
pub const STORE_SIZES: &[(&str, &str, f32)] = &[
    ("normal", "Normal", 1.0),
    ("large", "Grande", 1.125),
    ("xlarge", "Muy grande", 1.25),
];

/// Tamaño de un bloque respecto del de la tienda: `(id, nombre, factor)`.
/// El vacío es "igual que la tienda".
pub const BLOCK_SIZES: &[(&str, &str, f32)] = &[
    ("", "Igual", 1.0),
    ("smaller", "Más chico", 0.875),
    ("larger", "Más grande", 1.125),
    ("largest", "Muy grande", 1.25),
];

/// Alineación de un bloque. La vacía deja la que trae el bloque.
pub const ALIGN_OPTIONS: &[(&str, &str)] = &[
    ("", "Como viene"),
    ("left", "Izquierda"),
    ("center", "Centro"),
    ("right", "Derecha"),
];

fn factor(table: &[(&str, &str, f32)], id: &str) -> f32 {
    table
        .iter()
        .find(|(k, _, _)| *k == id)
        .map_or(1.0, |(_, _, f)| *f)
}

/// Letras de toda la tienda.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default)]
pub struct Typography {
    /// Fuente de los títulos (id de [`FONTS`]).
    pub heading: String,
    /// Fuente del resto del texto.
    pub body: String,
    /// Tamaño general (id de [`STORE_SIZES`]).
    pub size: String,
}

impl Default for Typography {
    fn default() -> Self {
        Self {
            heading: "system".into(),
            body: "system".into(),
            size: "normal".into(),
        }
    }
}

impl Typography {
    pub fn is_default(&self) -> bool {
        *self == Self::default()
    }

    /// Variables CSS para la raíz de la tienda (`.mk-store`).
    pub fn css_vars(&self) -> String {
        let scale = factor(STORE_SIZES, &self.size);
        format!(
            "--mk-heading:{}; --mk-body:{}; --mk-store-scale:{scale}",
            font_css(&self.heading),
            font_css(&self.body)
        )
    }

    pub fn errors(&self, errors: &mut Vec<String>) {
        if font(&self.heading).is_none() {
            errors.push("Letras › Fuente de los títulos: Elegí una de las opciones.".into());
        }
        if font(&self.body).is_none() {
            errors.push("Letras › Fuente del texto: Elegí una de las opciones.".into());
        }
        if !STORE_SIZES.iter().any(|(k, _, _)| *k == self.size) {
            errors.push("Letras › Tamaño: Elegí una de las opciones.".into());
        }
    }
}

/// Letras de un bloque. Lo vacío (o falso) se toma de la tienda o del bloque.
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
#[serde(default)]
pub struct TextStyle {
    /// Fuente para todo el bloque (títulos y texto); vacía = la de la tienda.
    pub font: String,
    /// Id de [`BLOCK_SIZES`].
    pub size: String,
    pub bold: bool,
    pub italic: bool,
    /// Id de [`ALIGN_OPTIONS`].
    pub align: String,
}

impl TextStyle {
    pub fn is_default(&self) -> bool {
        *self == Self::default()
    }

    /// Variables CSS del envoltorio del bloque (`.mk-type`).
    pub fn css_vars(&self) -> String {
        let mut s = String::new();
        if !self.font.is_empty() {
            let css = font_css(&self.font);
            s.push_str(&format!("--mk-heading:{css}; --mk-body:{css}; "));
        }
        let k = factor(BLOCK_SIZES, &self.size);
        s.push_str(&format!("--mk-block-scale:{k}"));
        s
    }

    /// Clases del envoltorio del bloque.
    pub fn classes(&self) -> String {
        let mut c = String::from("mk-type");
        if self.bold {
            c.push_str(" mk-bold");
        }
        if self.italic {
            c.push_str(" mk-italic");
        }
        match self.align.as_str() {
            "left" => c.push_str(" mk-align-left"),
            "center" => c.push_str(" mk-align-center"),
            "right" => c.push_str(" mk-align-right"),
            _ => {}
        }
        c
    }

    pub fn errors(&self, path: &str, errors: &mut Vec<String>) {
        if !self.font.is_empty() && font(&self.font).is_none() {
            errors.push(format!("{path}Fuente: Elegí una de las opciones."));
        }
        if !BLOCK_SIZES.iter().any(|(k, _, _)| *k == self.size) {
            errors.push(format!("{path}Tamaño de letra: Elegí una de las opciones."));
        }
        if !ALIGN_OPTIONS.iter().any(|(k, _)| *k == self.align) {
            errors.push(format!("{path}Alineación: Elegí una de las opciones."));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pairs_use_known_fonts() {
        for p in FONT_PAIRS {
            assert!(font(p.heading).is_some(), "{}", p.label);
            assert!(font(p.body).is_some(), "{}", p.label);
        }
        assert_eq!(FONT_PAIRS[0].heading, Typography::default().heading);
    }

    #[test]
    fn css_vars() {
        let t = Typography {
            heading: "playfair".into(),
            body: "inter".into(),
            size: "large".into(),
        };
        let v = t.css_vars();
        assert!(v.contains("--mk-heading:'Playfair Display Variable'"));
        assert!(v.contains("--mk-store-scale:1.125"));
        let s = TextStyle {
            font: "lora".into(),
            size: "smaller".into(),
            bold: true,
            align: "center".into(),
            ..TextStyle::default()
        };
        assert!(s.css_vars().contains("--mk-body:'Lora Variable'"));
        assert!(s.css_vars().ends_with("--mk-block-scale:0.875"));
        assert_eq!(s.classes(), "mk-type mk-bold mk-align-center");
        assert_eq!(TextStyle::default().css_vars(), "--mk-block-scale:1");
    }
}
