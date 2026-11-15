//! Ajustes de toda la tienda que no son colores ni letras:
//!
//! - [`StoreLayout`]: el margen de cada lado (entre el borde de la pantalla y
//!   los bloques) y hasta qué ancho se estira el contenido.
//! - [`StoreStyle`]: la forma de los botones, las esquinas de las tarjetas y
//!   el fondo de la página.
//!
//! Valen para todas las páginas. Los documentos anteriores no los traen y se
//! ven como siempre: 16 px de margen, contenido de 1120 px y lo que traiga
//! el tema.

use serde::{Deserialize, Serialize};

/// Anchos máximos sugeridos del contenido, en px. `0` es "Ancho completo".
pub const WIDTH_PRESETS: &[(u16, &str)] = &[
    (0, "Completo"),
    (1440, "1440"),
    (1120, "1120"),
    (960, "960"),
];

/// Ancho del contenido que viene de fábrica.
pub const DEFAULT_WIDTH: u16 = 1120;
/// Ancho mínimo del contenido, al arrastrar los bordes.
pub const MIN_WIDTH: u16 = 640;
/// Ancho máximo del contenido (sin contar "Ancho completo").
pub const MAX_WIDTH: u16 = 1600;
/// Al arrastrar los bordes, el ancho salta de a estos píxeles.
pub const WIDTH_STEP: u16 = 40;

/// Margen que viene de fábrica en cada lado, en px.
pub const DEFAULT_PAD: u16 = 16;
/// Margen máximo de un lado, en px.
pub const MAX_PAD: u16 = 160;
/// Los botones − y + del margen suman o restan de a estos píxeles.
pub const PAD_STEP: u16 = 8;

/// Un lado del contenido.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Side {
    Top,
    Right,
    Bottom,
    Left,
}

impl Side {
    pub const ALL: [Side; 4] = [Side::Top, Side::Right, Side::Bottom, Side::Left];

    pub fn label(self) -> &'static str {
        match self {
            Side::Top => "Arriba",
            Side::Right => "Derecha",
            Side::Bottom => "Abajo",
            Side::Left => "Izquierda",
        }
    }

    /// Nombre corto, para el campo de la medida.
    pub fn short(self) -> &'static str {
        match self {
            Side::Top => "Arr.",
            Side::Right => "Der.",
            Side::Bottom => "Abajo",
            Side::Left => "Izq.",
        }
    }
}

fn default_pad() -> u16 {
    DEFAULT_PAD
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default)]
pub struct StoreLayout {
    /// Margen de cada lado, en px.
    #[serde(default = "default_pad")]
    pub pad_top: u16,
    #[serde(default = "default_pad")]
    pub pad_right: u16,
    #[serde(default = "default_pad")]
    pub pad_bottom: u16,
    #[serde(default = "default_pad")]
    pub pad_left: u16,
    /// Ancho máximo del contenido en px; `0` = ancho completo.
    pub max_width: u16,
}

impl Default for StoreLayout {
    fn default() -> Self {
        Self {
            pad_top: DEFAULT_PAD,
            pad_right: DEFAULT_PAD,
            pad_bottom: DEFAULT_PAD,
            pad_left: DEFAULT_PAD,
            max_width: DEFAULT_WIDTH,
        }
    }
}

/// Lleva un ancho cualquiera al paso de [`WIDTH_STEP`], entre [`MIN_WIDTH`]
/// y [`MAX_WIDTH`].
pub fn snap_width(px: f64) -> u16 {
    let step = f64::from(WIDTH_STEP);
    let w = (px / step).round() * step;
    w.clamp(f64::from(MIN_WIDTH), f64::from(MAX_WIDTH)) as u16
}

impl StoreLayout {
    pub fn is_default(&self) -> bool {
        *self == Self::default()
    }

    pub fn is_full(&self) -> bool {
        self.max_width == 0
    }

    /// Margen de un lado, en px.
    pub fn pad(&self, side: Side) -> u16 {
        match side {
            Side::Top => self.pad_top,
            Side::Right => self.pad_right,
            Side::Bottom => self.pad_bottom,
            Side::Left => self.pad_left,
        }
    }

    /// Cambia el margen de un lado (limitado a `0..=MAX_PAD`).
    pub fn set_pad(&mut self, side: Side, px: i32) {
        let px = px.clamp(0, i32::from(MAX_PAD)) as u16;
        match side {
            Side::Top => self.pad_top = px,
            Side::Right => self.pad_right = px,
            Side::Bottom => self.pad_bottom = px,
            Side::Left => self.pad_left = px,
        }
    }

    /// Pone el mismo margen en los cuatro lados.
    pub fn set_all_pads(&mut self, px: i32) {
        for s in Side::ALL {
            self.set_pad(s, px);
        }
    }

    /// Los cuatro lados tienen el mismo margen.
    pub fn pads_equal(&self) -> bool {
        Side::ALL.iter().all(|s| self.pad(*s) == self.pad_top)
    }

    /// Estilo del contenedor de la tienda publicada (`.mk-wrap`).
    pub fn css_vars(&self) -> String {
        let max = if self.is_full() {
            "100%".to_string()
        } else {
            format!("{}px", self.max_width)
        };
        format!(
            "--mk-pt:{}px; --mk-pr:{}px; --mk-pb:{}px; --mk-pl:{}px; --mk-maxw:{max}",
            self.pad_top, self.pad_right, self.pad_bottom, self.pad_left
        )
    }

    pub fn errors(&self, errors: &mut Vec<String>) {
        if Side::ALL.iter().any(|s| self.pad(*s) > MAX_PAD) {
            errors.push(format!(
                "Márgenes: cada lado puede tener hasta {MAX_PAD} px."
            ));
        }
        if !self.is_full() && !(MIN_WIDTH..=MAX_WIDTH).contains(&self.max_width) {
            errors.push(format!(
                "Ancho de la tienda: tiene que estar entre {MIN_WIDTH} y {MAX_WIDTH} px."
            ));
        }
    }
}

/// Forma de los botones: `(id, nombre, radio CSS)`. La vacía es la del tema.
pub const BUTTON_SHAPES: &[(&str, &str, &str)] = &[
    ("", "Del tema", ""),
    ("square", "Recto", "0.125rem"),
    ("soft", "Suave", "0.5rem"),
    ("pill", "Píldora", "999px"),
];

/// Esquinas de tarjetas, imágenes y cuadros: `(id, nombre, radio CSS)`.
pub const CORNERS: &[(&str, &str, &str)] = &[
    ("", "Del tema", ""),
    ("square", "Rectas", "0"),
    ("soft", "Suaves", "0.75rem"),
    ("round", "Redondas", "1.5rem"),
];

/// Fondo de la página: `(id, nombre, color CSS)`. Todos salen del tema, así
/// el texto siempre se lee bien. El vacío es el de siempre (base-200).
pub const BACKGROUNDS: &[(&str, &str, &str)] = &[
    ("", "Gris suave", "var(--color-base-200)"),
    ("base", "Liso", "var(--color-base-100)"),
    ("deep", "Gris", "var(--color-base-300)"),
    (
        "primary",
        "Tono principal",
        "color-mix(in oklab, var(--color-primary) 8%, var(--color-base-100))",
    ),
    (
        "secondary",
        "Tono secundario",
        "color-mix(in oklab, var(--color-secondary) 8%, var(--color-base-100))",
    ),
    (
        "accent",
        "Tono de acento",
        "color-mix(in oklab, var(--color-accent) 10%, var(--color-base-100))",
    ),
];

fn css_of(
    table: &'static [(&'static str, &'static str, &'static str)],
    id: &str,
) -> Option<&'static str> {
    table
        .iter()
        .find(|(k, ..)| *k == id)
        .map(|(_, _, c)| *c)
        .filter(|c| !c.is_empty())
}

/// Forma de los botones, esquinas y fondo de toda la tienda. Lo vacío es lo
/// que trae el tema.
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
#[serde(default)]
pub struct StoreStyle {
    /// Id de [`BUTTON_SHAPES`].
    pub buttons: String,
    /// Id de [`CORNERS`].
    pub corners: String,
    /// Id de [`BACKGROUNDS`].
    pub background: String,
}

impl StoreStyle {
    pub fn is_default(&self) -> bool {
        *self == Self::default()
    }

    /// Variables CSS para la raíz de la tienda (`.mk-store`): cambian los
    /// radios del tema (daisyUI) y el fondo.
    pub fn css_vars(&self) -> String {
        let mut s = String::new();
        if let Some(r) = css_of(BUTTON_SHAPES, &self.buttons) {
            s.push_str(&format!("--radius-field:{r}; --radius-selector:{r}; "));
        }
        if let Some(r) = css_of(CORNERS, &self.corners) {
            s.push_str(&format!("--radius-box:{r}; "));
        }
        let bg = css_of(BACKGROUNDS, &self.background).unwrap_or("var(--color-base-200)");
        s.push_str(&format!("--mk-bg:{bg}"));
        s
    }

    pub fn errors(&self, errors: &mut Vec<String>) {
        if !BUTTON_SHAPES.iter().any(|(k, ..)| *k == self.buttons) {
            errors.push("Botones: Elegí una de las formas.".into());
        }
        if !CORNERS.iter().any(|(k, ..)| *k == self.corners) {
            errors.push("Esquinas: Elegí una de las opciones.".into());
        }
        if !BACKGROUNDS.iter().any(|(k, ..)| *k == self.background) {
            errors.push("Fondo: Elegí uno de los colores.".into());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_keeps_the_old_look_and_validates() {
        let l = StoreLayout::default();
        assert!(Side::ALL.iter().all(|s| l.pad(*s) == 16));
        assert_eq!(l.max_width, 1120);
        assert_eq!(
            l.css_vars(),
            "--mk-pt:16px; --mk-pr:16px; --mk-pb:16px; --mk-pl:16px; --mk-maxw:1120px"
        );
        let mut e = Vec::new();
        l.errors(&mut e);
        StoreStyle::default().errors(&mut e);
        assert!(e.is_empty());
        assert_eq!(
            StoreStyle::default().css_vars(),
            "--mk-bg:var(--color-base-200)"
        );
        // Un documento viejo (sin márgenes por lado) toma 16 px.
        let old: StoreLayout = serde_json::from_str(r#"{"max_width":960}"#).unwrap();
        assert_eq!(old.pad_left, 16);
        assert_eq!(old.max_width, 960);
    }

    #[test]
    fn sides_are_set_one_by_one_or_together_within_limits() {
        let mut l = StoreLayout::default();
        l.set_pad(Side::Left, 40);
        l.set_pad(Side::Top, -5);
        assert_eq!((l.pad_left, l.pad_top, l.pad_right), (40, 0, 16));
        assert!(!l.pads_equal());
        l.set_all_pads(999);
        assert!(l.pads_equal());
        assert_eq!(l.pad_bottom, MAX_PAD);
        l.max_width = 0;
        assert!(l.css_vars().ends_with("--mk-maxw:100%"));
        let mut e = Vec::new();
        StoreLayout {
            pad_top: 500,
            max_width: 200,
            ..StoreLayout::default()
        }
        .errors(&mut e);
        assert_eq!(e.len(), 2);
    }

    #[test]
    fn style_changes_theme_radii_and_background() {
        let s = StoreStyle {
            buttons: "pill".into(),
            corners: "square".into(),
            background: "base".into(),
        };
        let css = s.css_vars();
        assert!(css.contains("--radius-field:999px"));
        assert!(css.contains("--radius-box:0;"));
        assert!(css.contains("--mk-bg:var(--color-base-100)"));
        let mut e = Vec::new();
        StoreStyle {
            buttons: "x".into(),
            ..StoreStyle::default()
        }
        .errors(&mut e);
        assert_eq!(e.len(), 1);
    }

    #[test]
    fn snap_goes_to_steps_within_limits() {
        assert_eq!(snap_width(1133.0), 1120);
        assert_eq!(snap_width(1141.0), 1160);
        assert_eq!(snap_width(100.0), MIN_WIDTH);
        assert_eq!(snap_width(5000.0), MAX_WIDTH);
        for (w, _) in WIDTH_PRESETS.iter().filter(|(w, _)| *w > 0) {
            assert_eq!(snap_width(f64::from(*w)), *w);
        }
    }
}
