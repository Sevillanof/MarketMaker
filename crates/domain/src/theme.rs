//! Tema propio de la tienda (editor "Mi tienda"): colores, esquinas, tamaños,
//! bordes y efectos que se apoyan en las variables de daisyUI.
//!
//! Lo que no se toca queda como lo trae el tema elegido. Los valores de
//! fábrica de cada tema están en [`THEME_BASES`] para poder mostrarlos en el
//! editor sin leer el CSS.

/// Cada color editable de la tienda: `(variable de daisyUI sin `--color-`, nombre)`.
/// El orden es el de [`ThemeBase::colors`].
pub const COLOR_KEYS: [(&str, &str); 20] = [
    ("base-100", "Fondo"),
    ("base-200", "Fondo 2"),
    ("base-300", "Fondo 3"),
    ("base-content", "Texto"),
    ("primary", "Principal"),
    ("primary-content", "Texto sobre principal"),
    ("secondary", "Secundario"),
    ("secondary-content", "Texto sobre secundario"),
    ("accent", "Acento"),
    ("accent-content", "Texto sobre acento"),
    ("neutral", "Neutro"),
    ("neutral-content", "Texto sobre neutro"),
    ("info", "Información"),
    ("info-content", "Texto sobre información"),
    ("success", "Éxito"),
    ("success-content", "Texto sobre éxito"),
    ("warning", "Aviso"),
    ("warning-content", "Texto sobre aviso"),
    ("error", "Error"),
    ("error-content", "Texto sobre error"),
];

/// Valores de fábrica de un tema de daisyUI.
#[derive(Clone, Copy, Debug)]
pub struct ThemeBase {
    pub dark: bool,
    /// En el orden de [`COLOR_KEYS`], como `#rrggbb`.
    pub colors: [&'static str; 20],
    /// Esquinas, en px.
    pub radius_box: u8,
    pub radius_field: u8,
    pub radius_selector: u8,
    /// Tamaño base de campos y selectores, en px (4 = el de siempre).
    pub size_field: f32,
    pub size_selector: f32,
    /// Grosor de los bordes, en px.
    pub border: f32,
    pub depth: bool,
    pub noise: bool,
}

/// Valores de fábrica del tema `theme` (el primero si no se conoce).
pub fn theme_base(theme: &str) -> &'static ThemeBase {
    let i = crate::THEMES.iter().position(|t| *t == theme).unwrap_or(0);
    &THEME_BASES[i]
}

/// Posición de un color en [`COLOR_KEYS`].
pub fn color_index(key: &str) -> Option<usize> {
    COLOR_KEYS.iter().position(|(k, _)| *k == key)
}

/// `#rrggbb` en minúsculas (acepta `#RGB`); `None` si no es un color.
pub fn normalize_hex(s: &str) -> Option<String> {
    let h = s.trim().strip_prefix('#')?;
    if !h.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    match h.len() {
        6 => Some(format!("#{}", h.to_ascii_lowercase())),
        3 => {
            let mut o = String::from("#");
            for c in h.chars() {
                o.push(c.to_ascii_lowercase());
                o.push(c.to_ascii_lowercase());
            }
            Some(o)
        }
        _ => None,
    }
}

/// Luminancia relativa (WCAG) de un `#rrggbb`.
fn luminance(hex: &str) -> f32 {
    let ch = |i: usize| {
        let v = f32::from(u8::from_str_radix(&hex[i..i + 2], 16).unwrap_or(0)) / 255.0;
        if v <= 0.04045 {
            v / 12.92
        } else {
            ((v + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * ch(1) + 0.7152 * ch(3) + 0.0722 * ch(5)
}

/// Texto que se lee bien sobre `bg` (oscuro o blanco): mismo umbral que
/// `contrast-color()`, para llegar a 4,5:1.
pub fn readable_on(bg: &str) -> &'static str {
    if luminance(bg) > 0.17913 {
        "#111111"
    } else {
        "#ffffff"
    }
}

/// Mezcla dos colores `#rrggbb` canal por canal: `a × (1 − t) + b × t`.
pub fn mix(a: &str, b: &str, t: f32) -> String {
    let ch = |h: &str, i: usize| f32::from(u8::from_str_radix(&h[i..i + 2], 16).unwrap_or(0));
    let c = |i: usize| {
        let v = (ch(a, i) * (1.0 - t) + ch(b, i) * t)
            .round()
            .clamp(0.0, 255.0);
        format!("{:02x}", v as u8)
    };
    format!("#{}{}{}", c(1), c(3), c(5))
}

/// Los 20 colores de la variante opuesta de un tema (la oscura de uno claro
/// y al revés), a partir de los cuatro de la original: fondo, texto,
/// principal y acento. Reglas de la maqueta del panel de estilo.
pub fn opposite_colors(
    bg: &str,
    _ink: &str,
    primary: &str,
    line: &str,
    to_dark: bool,
) -> [String; 20] {
    let (bg2, ink2, pri2, line2) = if to_dark {
        (
            mix(bg, "#10131a", 0.91),
            "#f6f5f2".to_string(),
            mix(primary, "#ffffff", 0.65),
            mix(line, "#ffffff", 0.34),
        )
    } else {
        (
            mix(bg, "#ffffff", 0.94),
            "#242528".to_string(),
            mix(primary, "#111111", 0.66),
            mix(line, "#333333", 0.22),
        )
    };
    // Estados (información, éxito, aviso, error): los del tema de daisyUI
    // de ese modo, con su texto calculado.
    let state_base = theme_base(if to_dark { "dark" } else { "light" });
    let mut out: [String; 20] = std::array::from_fn(|_| String::new());
    for (i, (key, _)) in COLOR_KEYS.iter().enumerate() {
        out[i] = match *key {
            "base-100" => bg2.clone(),
            "base-200" => mix(&bg2, &ink2, 0.08),
            "base-300" => mix(&bg2, &ink2, 0.16),
            "base-content" => ink2.clone(),
            "primary" | "secondary" => pri2.clone(),
            "accent" => line2.clone(),
            "neutral" => ink2.clone(),
            "info" | "success" | "warning" | "error" => state_base.colors[i].to_string(),
            _ => String::new(),
        };
    }
    for (i, (key, _)) in COLOR_KEYS.iter().enumerate() {
        if let Some(base) = key.strip_suffix("-content") {
            if base != "base" {
                let j = color_index(base).unwrap_or(0);
                out[i] = readable_on(&out[j]).to_string();
            }
        }
    }
    out
}

/// Valores de fábrica de cada tema (sacados de daisyUI), en el orden de [`THEMES`].
pub const THEME_BASES: &[ThemeBase] = &[
    // Línea
    ThemeBase {
        dark: false,
        colors: [
            "#faf8f4", "#f0ece4", "#d9d3c7", "#292821", "#35382e", "#fffefa", "#625e54", "#faf8f4",
            "#35382e", "#fffefa", "#35382e", "#fffefa", "#0369a1", "#ffffff", "#15803d", "#ffffff",
            "#b45309", "#ffffff", "#b91c1c", "#ffffff",
        ],
        radius_box: 2,
        radius_field: 2,
        radius_selector: 2,
        size_field: 4.0,
        size_selector: 4.0,
        border: 1.0,
        depth: false,
        noise: false,
    },
    // Pulso
    ThemeBase {
        dark: false,
        colors: [
            "#f6f7ff", "#e9edff", "#cbd4f5", "#10193b", "#2947c9", "#ffffff", "#485574", "#f6f7ff",
            "#2947c9", "#ffffff", "#111d4c", "#ffffff", "#0369a1", "#ffffff", "#15803d", "#ffffff",
            "#b45309", "#ffffff", "#b91c1c", "#ffffff",
        ],
        radius_box: 16,
        radius_field: 32,
        radius_selector: 16,
        size_field: 4.0,
        size_selector: 4.0,
        border: 1.0,
        depth: false,
        noise: false,
    },
    // Noche
    ThemeBase {
        dark: true,
        colors: [
            "#18221e", "#26342c", "#485b4d", "#f4f3e8", "#c5dcae", "#17241a", "#c6d1c4", "#18221e",
            "#c5dcae", "#17241a", "#101a15", "#f4f3e8", "#7dd3fc", "#082f49", "#86efac", "#052e16",
            "#fcd34d", "#451a03", "#fca5a5", "#450a0a",
        ],
        radius_box: 10,
        radius_field: 4,
        radius_selector: 10,
        size_field: 4.0,
        size_selector: 4.0,
        border: 1.0,
        depth: false,
        noise: false,
    },
    // Brisa
    ThemeBase {
        dark: false,
        colors: [
            "#f5fafb", "#dceff0", "#567f89", "#16323b", "#17627b", "#ffffff", "#405c64", "#f5fafb",
            "#17627b", "#ffffff", "#153f4e", "#ffffff", "#0369a1", "#ffffff", "#15803d", "#ffffff",
            "#b45309", "#ffffff", "#b91c1c", "#ffffff",
        ],
        radius_box: 16,
        radius_field: 32,
        radius_selector: 16,
        size_field: 4.0,
        size_selector: 4.0,
        border: 1.0,
        depth: false,
        noise: false,
    },
    // Tinta
    ThemeBase {
        dark: false,
        colors: [
            "#fbfaf7", "#e8e7e2", "#817e78", "#181818", "#171717", "#ffffff", "#52514e", "#fbfaf7",
            "#171717", "#ffffff", "#171717", "#ffffff", "#0369a1", "#ffffff", "#15803d", "#ffffff",
            "#b45309", "#ffffff", "#b91c1c", "#ffffff",
        ],
        radius_box: 0,
        radius_field: 0,
        radius_selector: 0,
        size_field: 4.0,
        size_selector: 4.0,
        border: 1.0,
        depth: false,
        noise: false,
    },
    // Cítrico
    ThemeBase {
        dark: false,
        colors: [
            "#fffaf0", "#fff0d6", "#96704f", "#352415", "#a33b15", "#ffffff", "#705039", "#fffaf0",
            "#a33b15", "#ffffff", "#733013", "#fffaf0", "#0369a1", "#ffffff", "#15803d", "#ffffff",
            "#b45309", "#ffffff", "#b91c1c", "#ffffff",
        ],
        radius_box: 10,
        radius_field: 8,
        radius_selector: 10,
        size_field: 4.0,
        size_selector: 4.0,
        border: 1.0,
        depth: false,
        noise: false,
    },
    // Aura
    ThemeBase {
        dark: false,
        colors: [
            "#faf8fc", "#eee8f5", "#88719a", "#33273f", "#654778", "#ffffff", "#62526d", "#faf8fc",
            "#654778", "#ffffff", "#49364f", "#ffffff", "#0369a1", "#ffffff", "#15803d", "#ffffff",
            "#b45309", "#ffffff", "#b91c1c", "#ffffff",
        ],
        radius_box: 14,
        radius_field: 32,
        radius_selector: 14,
        size_field: 4.0,
        size_selector: 4.0,
        border: 1.0,
        depth: false,
        noise: false,
    },
    // Brote
    ThemeBase {
        dark: false,
        colors: [
            "#f7faf4", "#e2edda", "#6a8a6e", "#1c3323", "#315f3b", "#ffffff", "#4c634e", "#f7faf4",
            "#315f3b", "#ffffff", "#244732", "#ffffff", "#0369a1", "#ffffff", "#15803d", "#ffffff",
            "#b45309", "#ffffff", "#b91c1c", "#ffffff",
        ],
        radius_box: 14,
        radius_field: 4,
        radius_selector: 14,
        size_field: 4.0,
        size_selector: 4.0,
        border: 1.0,
        depth: false,
        noise: false,
    },
    // Órbita
    ThemeBase {
        dark: true,
        colors: [
            "#111f33", "#223854", "#7894aa", "#f5f8fc", "#9ee4e9", "#102b38", "#c6d4e4", "#111f33",
            "#9ee4e9", "#102b38", "#091626", "#f5f8fc", "#7dd3fc", "#082f49", "#86efac", "#052e16",
            "#fcd34d", "#451a03", "#fca5a5", "#450a0a",
        ],
        radius_box: 6,
        radius_field: 4,
        radius_selector: 6,
        size_field: 4.0,
        size_selector: 4.0,
        border: 1.0,
        depth: false,
        noise: false,
    },
    // Dalia
    ThemeBase {
        dark: false,
        colors: [
            "#fff9f8", "#f7e6e5", "#9c7278", "#45262d", "#943d51", "#ffffff", "#705057", "#fff9f8",
            "#943d51", "#ffffff", "#5b3039", "#ffffff", "#0369a1", "#ffffff", "#15803d", "#ffffff",
            "#b45309", "#ffffff", "#b91c1c", "#ffffff",
        ],
        radius_box: 5,
        radius_field: 2,
        radius_selector: 5,
        size_field: 4.0,
        size_selector: 4.0,
        border: 1.0,
        depth: false,
        noise: false,
    },
    // Cacao
    ThemeBase {
        dark: true,
        colors: [
            "#261b17", "#47332c", "#aa8d77", "#fff7eb", "#e6c49c", "#2a1b15", "#e2cec0", "#261b17",
            "#e6c49c", "#2a1b15", "#1c120f", "#fff7eb", "#7dd3fc", "#082f49", "#86efac", "#052e16",
            "#fcd34d", "#451a03", "#fca5a5", "#450a0a",
        ],
        radius_box: 3,
        radius_field: 2,
        radius_selector: 3,
        size_field: 4.0,
        size_selector: 4.0,
        border: 1.0,
        depth: false,
        noise: false,
    },
    // Sol
    ThemeBase {
        dark: false,
        colors: [
            "#fffdf6", "#fff0bd", "#95752b", "#30270e", "#544000", "#ffffff", "#65552b", "#fffdf6",
            "#544000", "#ffffff", "#3c3010", "#ffffff", "#0369a1", "#ffffff", "#15803d", "#ffffff",
            "#b45309", "#ffffff", "#b91c1c", "#ffffff",
        ],
        radius_box: 10,
        radius_field: 32,
        radius_selector: 10,
        size_field: 4.0,
        size_selector: 4.0,
        border: 1.0,
        depth: false,
        noise: false,
    },
    // Los seis temas anteriores (siguen funcionando en tiendas que ya los usan).
    ThemeBase {
        dark: false,
        colors: [
            "#ffffff", "#f8f8f8", "#eeeeee", "#18181b", "#422ad5", "#e0e7ff", "#f43098", "#f9e4f0",
            "#00d3bb", "#084d49", "#09090b", "#e4e4e7", "#00bafe", "#042e49", "#00d390", "#004c39",
            "#fcb700", "#793205", "#ff627d", "#4d0218",
        ],
        radius_box: 8,
        radius_field: 4,
        radius_selector: 8,
        size_field: 4.0,
        size_selector: 4.0,
        border: 1.0,
        depth: true,
        noise: false,
    },
    ThemeBase {
        dark: true,
        colors: [
            "#1d232a", "#191e24", "#15191e", "#ecf9ff", "#605dff", "#edf1fe", "#f43098", "#f9e4f0",
            "#00d3bb", "#084d49", "#09090b", "#e4e4e7", "#00bafe", "#042e49", "#00d390", "#004c39",
            "#fcb700", "#793205", "#ff627d", "#4d0218",
        ],
        radius_box: 8,
        radius_field: 4,
        radius_selector: 8,
        size_field: 4.0,
        size_selector: 4.0,
        border: 1.0,
        depth: true,
        noise: false,
    },
    ThemeBase {
        dark: false,
        colors: [
            "#faf7f5", "#efeae6", "#e7e2df", "#291334", "#44ebd3", "#005d58", "#f9cbe5", "#a0004a",
            "#ffd6a7", "#9f2d00", "#262629", "#e4e4e7", "#00a4f2", "#042e49", "#00ba7b", "#002c21",
            "#eeaf00", "#411e03", "#fe1c55", "#4d0218",
        ],
        radius_box: 16,
        radius_field: 32,
        radius_selector: 16,
        size_field: 4.0,
        size_selector: 4.0,
        border: 2.0,
        depth: true,
        noise: false,
    },
    ThemeBase {
        dark: false,
        colors: [
            "#ffffff", "#e8e8e8", "#d1d1d1", "#333c4d", "#66cc8a", "#223d30", "#377cfb", "#ffffff",
            "#f68067", "#000000", "#333c4d", "#f9fafb", "#00b5ff", "#000000", "#00a96e", "#000000",
            "#ffbe00", "#000000", "#ff5861", "#000000",
        ],
        radius_box: 16,
        radius_field: 8,
        radius_selector: 16,
        size_field: 4.0,
        size_selector: 4.0,
        border: 1.0,
        depth: false,
        noise: false,
    },
    ThemeBase {
        dark: false,
        colors: [
            "#ffffff", "#e8e8e8", "#d1d1d1", "#181a2a", "#0082ce", "#ffffff", "#61738d", "#ffffff",
            "#009689", "#ffffff", "#000000", "#ffffff", "#0090b5", "#ffffff", "#00a43b", "#ffffff",
            "#fdc700", "#000000", "#ff6266", "#000000",
        ],
        radius_box: 4,
        radius_field: 4,
        radius_selector: 4,
        size_field: 4.0,
        size_selector: 4.0,
        border: 1.0,
        depth: false,
        noise: false,
    },
    ThemeBase {
        dark: false,
        colors: [
            "#ece3ca", "#e4d8b4", "#dbca9b", "#793205", "#ff9fa0", "#801518", "#b7f6cd", "#00642e",
            "#d08700", "#793205", "#56524c", "#d4d0ce", "#0082ce", "#fef2c6", "#00776f", "#fef2c6",
            "#f34700", "#fef2c6", "#ff6266", "#7c2808",
        ],
        radius_box: 8,
        radius_field: 4,
        radius_selector: 4,
        size_field: 4.0,
        size_selector: 4.0,
        border: 1.0,
        depth: false,
        noise: false,
    },
];

#[cfg(test)]
mod tests_temas {
    use super::*;

    #[test]
    fn cada_tema_tiene_nombre_y_valores_de_fabrica() {
        assert_eq!(crate::THEMES.len(), crate::THEME_LABELS.len());
        assert_eq!(crate::THEMES.len(), THEME_BASES.len());
        assert!(crate::OFFERED_THEMES <= crate::THEMES.len());
        assert_eq!(crate::THEMES[0], "linea");
    }
}
