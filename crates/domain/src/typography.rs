//! Letras de la tienda: fuentes, tamaño y estilo del texto.
//!
//! Hay dos niveles. [`Typography`] vale para toda la tienda (fuente de los
//! títulos, fuente del texto y tamaño general). [`TextStyle`] cambia un
//! bloque en particular; lo que queda vacío se toma de la tienda.
//!
//! La lista de fuentes es corta y elegida a mano: todas se leen bien, tienen
//! acentos y eñes, y se sirven desde `/fonts` (sin depender de terceros).

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

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
    Font {
        id: "dmsans",
        label: "DM Sans",
        hint: "Natural y cercana",
        css: "'DM Sans Variable', ui-sans-serif, system-ui, sans-serif",
    },
    Font {
        id: "manrope",
        label: "Manrope",
        hint: "Contemporánea y limpia",
        css: "'Manrope Variable', ui-sans-serif, system-ui, sans-serif",
    },
    Font {
        id: "cormorant",
        label: "Cormorant Garamond",
        hint: "Refinada, para títulos",
        css: "'Cormorant Garamond Variable', ui-serif, Georgia, serif",
    },
    Font {
        id: "spacegrotesk",
        label: "Space Grotesk",
        hint: "Urbana, con carácter",
        css: "'Space Grotesk Variable', ui-sans-serif, system-ui, sans-serif",
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

/// Combinaciones sugeridas, para elegir de un clic (en el orden del selector
/// de Letras). Los ids de fuente son los que se guardan: renombrar una
/// combinación no cambia ninguna tienda.
pub const FONT_PAIRS: &[FontPair] = &[
    FontPair {
        label: "Editorial",
        heading: "lora",
        body: "inter",
    },
    FontPair {
        label: "Simple",
        heading: "inter",
        body: "inter",
    },
    FontPair {
        label: "Impacto",
        heading: "oswald",
        body: "inter",
    },
    FontPair {
        label: "Legible",
        heading: "atkinson",
        body: "atkinson",
    },
    FontPair {
        label: "Del dispositivo",
        heading: "system",
        body: "system",
    },
    FontPair {
        label: "Geométrica",
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
        label: "Natural",
        heading: "dmsans",
        body: "dmsans",
    },
    FontPair {
        label: "Contemporánea",
        heading: "manrope",
        body: "manrope",
    },
    FontPair {
        label: "Refinada",
        heading: "cormorant",
        body: "dmsans",
    },
    FontPair {
        label: "Urbana",
        heading: "spacegrotesk",
        body: "dmsans",
    },
];

/// Tamaño general del texto de la tienda: `(id, nombre, factor)`.
pub const STORE_SIZES: &[(&str, &str, f32)] = &[
    ("normal", "Normal", 1.0),
    ("large", "Grande", 1.125),
    ("xlarge", "Muy grande", 1.25),
];

/// Tamaño general en porcentaje: mínimo, máximo y paso del deslizador.
pub const SCALE_MIN: u16 = 80;
pub const SCALE_MAX: u16 = 150;
pub const SCALE_STEP: u16 = 5;
/// Tamaño base mínimo y máximo de un tipo de texto, en px de referencia.
pub const ROLE_MIN: u16 = 8;
pub const ROLE_MAX: u16 = 72;

/// Tipos de texto con tamaño propio: `(id, nombre, px de referencia)`. El
/// valor de referencia es el tamaño que tiene hoy ese texto en los widgets:
/// al elegir otro, todos los textos de ese tipo se agrandan o achican en la
/// misma proporción (así se conserva la jerarquía entre ellos).
pub const TEXT_ROLES: &[(&str, &str, u16)] = &[
    ("title", "Título principal", 42),
    ("section", "Títulos de sección", 30),
    ("subtitle", "Subtítulos y detalles", 12),
    ("product", "Nombres de productos", 13),
    ("body", "Texto y descripciones", 13),
    ("button", "Botones", 12),
    ("price", "Precios", 13),
    ("nav", "Menú y pie", 12),
    ("brand", "Nombre de tienda", 26),
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

/// Animaciones del texto de un bloque: id, nombre y qué hace. La vacía
/// (ninguna) deja el texto quieto. Los ids son los de las clases
/// `mk-anim-<id>` de `style/tailwind.css`.
pub const TEXT_ANIMATIONS: &[(&str, &str, &str)] = &[
    (
        "typewriter",
        "Máquina de escribir",
        "Escribe letra por letra, con cursor.",
    ),
    (
        "mask",
        "Subida con máscara",
        "Las palabras suben desde una línea invisible.",
    ),
    (
        "scramble",
        "Decodificar",
        "Letras al azar que se ordenan hasta formar el texto.",
    ),
    (
        "stagger",
        "Palabra por palabra",
        "Entran escalonadas, una detrás de otra.",
    ),
    (
        "blur",
        "Del desenfoque a nítido",
        "Aparece desenfocado y se enfoca de golpe.",
    ),
    (
        "bounce",
        "Rebote por letra",
        "Cada letra cae y rebota como un resorte.",
    ),
    (
        "tracking",
        "Espaciado que se expande",
        "Las letras se separan despacio.",
    ),
    (
        "flip",
        "Giro de letras",
        "Cada letra gira en 3D sobre su eje.",
    ),
    (
        "zoom",
        "Impacto con onda",
        "Cae con fuerza y lanza una onda expansiva.",
    ),
    (
        "split",
        "Fractura",
        "El texto se parte al medio y revela una palabra oculta.",
    ),
    (
        "gradient",
        "Degradé en movimiento",
        "Los colores fluyen dentro de las letras.",
    ),
    ("wave", "Ola", "Las letras suben y bajan como una ola."),
    ("glitch", "Glitch RGB", "Cortes breves en cian y rojo."),
    (
        "rotating",
        "Palabra que gira",
        "Una palabra cambia por otra con un giro 3D.",
    ),
    (
        "marker",
        "Resaltador",
        "Un trazo fluorescente pinta detrás del texto.",
    ),
    (
        "squiggle",
        "Subrayado ondulado",
        "Una línea ondulada se dibuja debajo.",
    ),
    (
        "outline",
        "Contorno a relleno",
        "Pasa de solo borde a letra rellena.",
    ),
    (
        "extrude",
        "Extrusión 3D",
        "Las letras ganan volumen en capas profundas.",
    ),
    (
        "strike",
        "Tachar y reemplazar",
        "Tacha el texto anterior y muestra el nuevo.",
    ),
    (
        "count",
        "Contador",
        "Cuenta hasta el número del texto, como un odómetro.",
    ),
];

/// El dato extra que pide una animación: (etiqueta del campo, ejemplo).
pub fn animation_extra(id: &str) -> Option<(&'static str, &'static str)> {
    match id {
        "rotating" => Some((
            "Palabras que rotan, separadas por coma",
            "rápida, simple, tuya",
        )),
        "strike" => Some(("Texto anterior (se muestra tachado)", "$49.900")),
        "split" => Some(("Palabra oculta", "GRATIS")),
        _ => None,
    }
}

/// Qué animación es (nombre y descripción), si el id existe.
pub fn text_animation(id: &str) -> Option<&'static (&'static str, &'static str, &'static str)> {
    TEXT_ANIMATIONS.iter().find(|(k, _, _)| *k == id)
}

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
    /// Tamaño general (id de [`STORE_SIZES`]). Sigue valiendo mientras no
    /// haya un porcentaje propio (`scale`).
    pub size: String,
    /// Tamaño general en porcentaje (`80..=150`, de a 5); `0` = el de `size`.
    pub scale: u16,
    /// Tamaño base de cada tipo de texto, en px (ver [`TEXT_ROLES`]). Los
    /// que faltan quedan como siempre.
    pub sizes: BTreeMap<String, u16>,
}

impl Default for Typography {
    fn default() -> Self {
        Self {
            heading: "system".into(),
            body: "system".into(),
            size: "normal".into(),
            scale: 0,
            sizes: BTreeMap::new(),
        }
    }
}

impl Typography {
    /// Factor del tamaño general: el porcentaje si lo hay, si no el de `size`.
    pub fn scale_factor(&self) -> f32 {
        if self.scale > 0 {
            f32::from(self.scale) / 100.0
        } else {
            factor(STORE_SIZES, &self.size)
        }
    }

    /// Porcentaje del tamaño general (112,5 para "Grande").
    pub fn scale_percent(&self) -> f32 {
        self.scale_factor() * 100.0
    }

    /// Tamaño base de un tipo de texto, en px (el de referencia si no se tocó).
    pub fn role_px(&self, id: &str) -> u16 {
        self.sizes
            .get(id)
            .copied()
            .or_else(|| TEXT_ROLES.iter().find(|(k, _, _)| *k == id).map(|r| r.2))
            .unwrap_or(16)
    }

    pub fn is_default(&self) -> bool {
        *self == Self::default()
    }

    /// Variables CSS para la raíz de la tienda (`.mk-store`).
    pub fn css_vars(&self) -> String {
        let scale = self.scale_factor();
        // "Simple" no pisa nada: las letras las pone el tema de la tienda.
        let mut s = String::new();
        if self.heading != "system" {
            s.push_str(&format!("--mk-heading:{}; ", font_css(&self.heading)));
        }
        if self.body != "system" {
            s.push_str(&format!("--mk-body:{}; ", font_css(&self.body)));
        }
        s.push_str(&format!("--mk-store-scale:{scale}"));
        // Un tipo de texto tocado: cuánto se agranda respecto de su tamaño
        // de referencia (ver `style/tailwind.css`, "Tamaños por tipo").
        for (id, _, reference) in TEXT_ROLES {
            if let Some(px) = self.sizes.get(*id) {
                let r = f32::from(*px) / f32::from(*reference);
                s.push_str(&format!("; --mk-r-{id}:{r:.4}"));
            }
        }
        s
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
        if self.scale != 0
            && (!(SCALE_MIN..=SCALE_MAX).contains(&self.scale) || self.scale % SCALE_STEP != 0)
        {
            errors.push(format!(
                "Letras › Tamaño general: entre {SCALE_MIN} % y {SCALE_MAX} %."
            ));
        }
        for (id, px) in &self.sizes {
            if !TEXT_ROLES.iter().any(|(k, _, _)| k == id) {
                errors.push(format!("Letras › Tamaños: «{id}» no es un tipo de texto."));
            } else if !(ROLE_MIN..=ROLE_MAX).contains(px) {
                errors.push(format!(
                    "Letras › Tamaños: entre {ROLE_MIN} y {ROLE_MAX} px."
                ));
            }
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
    /// Id de [`TEXT_ANIMATIONS`]; vacía = el texto queda quieto.
    pub animation: String,
    /// Dato extra de la animación (ver [`animation_extra`]).
    pub anim_text: String,
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
        if text_animation(&self.animation).is_some() {
            c.push_str(&format!(" mk-anim mk-anim-{}", self.animation));
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
        if !self.animation.is_empty() && text_animation(&self.animation).is_none() {
            errors.push(format!("{path}Animación: Elegí una de las opciones."));
        }
        if self.anim_text.chars().count() > 80 {
            errors.push(format!(
                "{path}Animación: El texto extra admite hasta 80 letras."
            ));
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
        assert_eq!(FONT_PAIRS.len(), 13);
        assert!(FONT_PAIRS
            .iter()
            .any(|p| p.heading == "system" && p.body == "system"));
    }

    #[test]
    fn css_vars() {
        let t = Typography {
            heading: "playfair".into(),
            body: "inter".into(),
            size: "large".into(),
            ..Typography::default()
        };
        let v = t.css_vars();
        assert!(v.contains("--mk-heading:'Playfair Display Variable'"));
        assert!(v.contains("--mk-store-scale:1.125"));
        assert!(!v.contains("--mk-r-"));
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
        // Con las letras por defecto, la tienda usa las del tema.
        assert_eq!(Typography::default().css_vars(), "--mk-store-scale:1");
    }

    #[test]
    fn animation_adds_class_and_validates() {
        let s = TextStyle {
            animation: "wave".into(),
            ..TextStyle::default()
        };
        assert_eq!(s.classes(), "mk-type mk-anim mk-anim-wave");
        assert!(!s.is_default());
        let mut errors = Vec::new();
        s.errors("", &mut errors);
        assert!(errors.is_empty(), "{errors:?}");
        let bad = TextStyle {
            animation: "disco".into(),
            ..TextStyle::default()
        };
        assert_eq!(bad.classes(), "mk-type");
        bad.errors("", &mut errors);
        assert_eq!(errors.len(), 1);
        for (id, _, _) in TEXT_ANIMATIONS {
            assert!(text_animation(id).is_some());
        }
        assert_eq!(TEXT_ANIMATIONS.len(), 20);
    }

    #[test]
    fn scale_and_role_sizes() {
        let mut t = Typography::default();
        // Sin porcentaje propio vale el tamaño de siempre, sin redondear.
        t.size = "large".into();
        assert_eq!(t.scale_percent(), 112.5);
        t.scale = 125;
        assert!(t.css_vars().contains("--mk-store-scale:1.25"));
        // Un tipo de texto tocado: proporción respecto de su referencia.
        t.sizes.insert("price".into(), 26);
        assert_eq!(t.role_px("price"), 26);
        assert_eq!(t.role_px("nav"), 12);
        assert!(t.css_vars().contains("--mk-r-price:2.0000"));
        let mut e = Vec::new();
        t.errors(&mut e);
        assert!(e.is_empty(), "{e:?}");
        t.scale = 83;
        t.sizes.insert("price".into(), 5);
        t.sizes.insert("zzz".into(), 20);
        let mut e = Vec::new();
        t.errors(&mut e);
        assert_eq!(e.len(), 3);
        // Un documento anterior (sin los campos nuevos) carga igual.
        let old: Typography =
            serde_json::from_str(r#"{"heading":"lora","size":"xlarge"}"#).unwrap();
        assert_eq!(old.scale_percent(), 125.0);
        assert!(old.sizes.is_empty());
    }
}
