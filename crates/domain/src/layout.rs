//! Ajustes de toda la tienda que no son colores ni letras:
//!
//! - [`StoreLayout`]: el margen de cada lado (entre el borde de la pantalla y
//!   los bloques) y el espacio entre bloques. El contenido ocupa todo el
//!   ancho de la pantalla menos los márgenes; `max_width` es un dato
//!   heredado que se conserva al guardar pero ya no limita nada.
//! - [`StoreStyle`]: la forma de los botones, las esquinas de las tarjetas y
//!   el fondo de la página.
//!
//! Valen para todas las páginas. Los documentos anteriores no los traen y se
//! ven como siempre: 16 px de margen, sin espacio entre bloques y lo que
//! traiga el tema.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::theme::{color_index, normalize_hex, readable_on, theme_base, COLOR_KEYS};

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
/// Márgenes de una tienda nueva: arriba y abajo, en px.
pub const FRESH_PAD_Y: u16 = 8;
/// Márgenes de una tienda nueva: a los costados, en px.
pub const FRESH_PAD_X: u16 = 24;
/// Margen máximo de un lado, en px.
pub const MAX_PAD: u16 = 160;
/// Espacio entre bloques de una tienda nueva, en px.
pub const FRESH_GAP: u16 = 10;
/// Espacio máximo entre bloques, en px.
pub const MAX_GAP: u16 = 40;
/// El espacio entre bloques sube o baja de a estos píxeles.
pub const GAP_STEP: u16 = 2;
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
    /// Dato heredado: el ancho máximo que tenían las tiendas. Se conserva al
    /// guardar (por si hay que volver atrás) pero ya no limita el ancho.
    pub max_width: u16,
    /// Espacio entre un bloque y el de al lado, en px (`0..=MAX_GAP`, de a
    /// [`GAP_STEP`]). Los documentos anteriores no lo traen: 0, como se ven.
    pub gap: u16,
}

impl Default for StoreLayout {
    fn default() -> Self {
        Self {
            pad_top: DEFAULT_PAD,
            pad_right: DEFAULT_PAD,
            pad_bottom: DEFAULT_PAD,
            pad_left: DEFAULT_PAD,
            max_width: DEFAULT_WIDTH,
            gap: 0,
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

    /// Los valores de una tienda nueva (y de "Valores iniciales"): ancho
    /// completo, 8 px arriba y abajo, 24 px a los costados. `default()` sigue
    /// siendo el de siempre, para que las tiendas ya hechas no cambien.
    pub fn fresh() -> Self {
        Self {
            pad_top: FRESH_PAD_Y,
            pad_right: FRESH_PAD_X,
            pad_bottom: FRESH_PAD_Y,
            pad_left: FRESH_PAD_X,
            max_width: 0,
            gap: FRESH_GAP,
        }
    }

    /// Cambia el espacio entre bloques (de a [`GAP_STEP`], hasta [`MAX_GAP`]).
    pub fn set_gap(&mut self, px: i32) {
        let step = i32::from(GAP_STEP);
        let px = (f64::from(px) / f64::from(step)).round() as i32 * step;
        self.gap = px.clamp(0, i32::from(MAX_GAP)) as u16;
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
        format!(
            "--mk-pt:{}px; --mk-pr:{}px; --mk-pb:{}px; --mk-pl:{}px; --mk-gap:{}px",
            self.pad_top, self.pad_right, self.pad_bottom, self.pad_left, self.gap
        )
    }

    pub fn errors(&self, errors: &mut Vec<String>) {
        if Side::ALL.iter().any(|s| self.pad(*s) > MAX_PAD) {
            errors.push(format!(
                "Márgenes: cada lado puede tener hasta {MAX_PAD} px."
            ));
        }
        if self.gap > MAX_GAP {
            errors.push(format!("Espacio entre bloques: hasta {MAX_GAP} px."));
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

/// Tema propio de la tienda, además de la forma de los botones, las esquinas
/// y el fondo. Lo vacío es lo que trae el tema elegido.
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
#[serde(default)]
pub struct StoreStyle {
    /// Id de [`BUTTON_SHAPES`] (tiendas anteriores y plantillas).
    pub buttons: String,
    /// Id de [`CORNERS`] (tiendas anteriores y plantillas).
    pub corners: String,
    /// Id de [`BACKGROUNDS`].
    pub background: String,
    /// Color propio del fondo, `#rrggbb`. Si está, manda sobre [`BACKGROUNDS`].
    #[serde(skip_serializing_if = "String::is_empty")]
    pub bg_color: String,
    /// Imagen del fondo (ruta o `https://`). Va debajo de los bloques.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub bg_image: String,
    /// Cuánto se oscurece la imagen, en % (`0..=`[`MAX_BG_DIM`]).
    #[serde(skip_serializing_if = "is_zero")]
    pub bg_dim: u8,
    /// La imagen queda quieta cuando se desplaza la página.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub bg_fixed: bool,
    /// Nombre que se le puso al tema (solo para mostrar).
    #[serde(skip_serializing_if = "String::is_empty")]
    pub name: String,
    /// Colores cambiados: clave de [`COLOR_KEYS`] → `#rrggbb`.
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub colors: BTreeMap<String, String>,
    /// Esquema de color oscuro (barras, formularios del navegador).
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub dark: bool,
    /// Esquinas, en px (`0..=`[`MAX_RADIUS`]).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub radius_box: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub radius_field: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub radius_selector: Option<u8>,
    /// Tamaño base de campos y selectores, en px.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size_field: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size_selector: Option<f32>,
    /// Grosor de los bordes, en px.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub border: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub depth: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub noise: Option<bool>,
}

fn is_zero(v: &u8) -> bool {
    *v == 0
}

/// Lo más que se puede oscurecer la imagen del fondo, en %.
pub const MAX_BG_DIM: u8 = 80;

/// ¿Sirve como dirección de imagen dentro de un `url()` de CSS? Solo rutas
/// propias o `https://`, sin comillas, paréntesis, espacios ni llaves.
pub fn safe_bg_image(url: &str) -> bool {
    (url.starts_with('/') && !url.starts_with("//") || url.starts_with("https://"))
        && url.len() <= 500
        && !url
            .chars()
            .any(|c| c.is_control() || c.is_whitespace() || "\"'()\\;{}<>".contains(c))
}

/// Esquina más redonda que se puede elegir, en px.
pub const MAX_RADIUS: u8 = 48;
/// Tamaño base de campos y selectores: mínimo, máximo y paso, en px.
pub const SIZE_RANGE: (f32, f32, f32) = (3.0, 5.0, 0.5);
/// Grosores de borde que se pueden elegir, en px.
pub const BORDER_STEPS: [f32; 5] = [0.5, 1.0, 1.5, 2.0, 3.0];
/// Opciones de esquina, en px.
pub const RADIUS_STEPS: [u8; 5] = [0, 4, 8, 16, 32];

fn rem(px: f32) -> String {
    format!("{}rem", px / 16.0)
}

impl StoreStyle {
    /// Sin ningún ajuste de tema propio (ni colores, ni esquinas, ni tamaños).
    pub fn theme_is_default(&self) -> bool {
        self.colors.is_empty()
            && !self.dark
            && self.radius_box.is_none()
            && self.radius_field.is_none()
            && self.radius_selector.is_none()
            && self.size_field.is_none()
            && self.size_selector.is_none()
            && self.border.is_none()
            && self.depth.is_none()
            && self.noise.is_none()
    }

    pub fn is_default(&self) -> bool {
        *self == Self::default()
    }

    /// Vuelve al tema tal cual viene (conserva fondo, botones y esquinas
    /// viejos de las plantillas).
    pub fn reset_theme(&mut self) {
        *self = Self {
            buttons: std::mem::take(&mut self.buttons),
            corners: std::mem::take(&mut self.corners),
            background: std::mem::take(&mut self.background),
            bg_color: std::mem::take(&mut self.bg_color),
            bg_image: std::mem::take(&mut self.bg_image),
            bg_dim: self.bg_dim,
            bg_fixed: self.bg_fixed,
            ..Self::default()
        };
    }

    /// Color que se ve de `key` con el tema `theme`: el cambiado o el del tema.
    pub fn color(&self, theme: &str, key: &str) -> String {
        if let Some(c) = self.colors.get(key) {
            return c.clone();
        }
        let i = color_index(key).unwrap_or(0);
        let base = theme_base(theme);
        // Un color de texto que no se cambió, sobre un color que sí: se calcula.
        if let Some(bg) = key.strip_suffix("-content") {
            if let Some(c) = self.colors.get(bg) {
                return readable_on(c).to_string();
            }
        }
        base.colors[i].to_string()
    }

    /// Cambia un color. `None` lo deja como lo trae el tema. Los colores de
    /// texto (`…-content`) que no se eligieron a mano se calculan solos.
    pub fn set_color(&mut self, key: &str, hex: Option<&str>) {
        if color_index(key).is_none() {
            return;
        }
        match hex.and_then(normalize_hex) {
            Some(h) => {
                self.colors.insert(key.to_string(), h);
            }
            None => {
                self.colors.remove(key);
            }
        }
    }

    pub fn radius(&self, theme: &str, which: Corner) -> u8 {
        let b = theme_base(theme);
        match which {
            Corner::Box => self.radius_box.unwrap_or(b.radius_box),
            Corner::Field => self.radius_field.unwrap_or(b.radius_field),
            Corner::Selector => self.radius_selector.unwrap_or(b.radius_selector),
        }
    }

    pub fn set_radius(&mut self, which: Corner, px: u8) {
        let px = Some(px.min(MAX_RADIUS));
        match which {
            Corner::Box => self.radius_box = px,
            Corner::Field => self.radius_field = px,
            Corner::Selector => self.radius_selector = px,
        }
        // Las formas viejas ya no mandan.
        self.buttons.clear();
        self.corners.clear();
    }

    /// Variables CSS para la raíz de la tienda (`.mk-store`): cambian los
    /// radios, los colores y las medidas del tema (daisyUI) y el fondo.
    pub fn css_vars(&self) -> String {
        let mut s = String::new();
        if let Some(r) = css_of(BUTTON_SHAPES, &self.buttons) {
            s.push_str(&format!("--radius-field:{r}; --radius-selector:{r}; "));
        }
        if let Some(r) = css_of(CORNERS, &self.corners) {
            s.push_str(&format!("--radius-box:{r}; "));
        }
        for (key, hex) in &self.colors {
            if color_index(key).is_some() && normalize_hex(hex).is_some() {
                s.push_str(&format!("--color-{key}:{hex}; "));
                if !key.ends_with("-content") && !key.starts_with("base-") {
                    let ck = format!("{key}-content");
                    if !self.colors.contains_key(&ck) {
                        s.push_str(&format!("--color-{ck}:{}; ", readable_on(hex)));
                    }
                }
            }
        }
        if self.dark {
            s.push_str("color-scheme:dark; ");
        }
        for (name, v) in [
            ("--radius-box", self.radius_box),
            ("--radius-field", self.radius_field),
            ("--radius-selector", self.radius_selector),
        ] {
            if let Some(px) = v {
                s.push_str(&format!("{name}:{}; ", rem(f32::from(px))));
            }
        }
        if let Some(v) = self.size_field {
            s.push_str(&format!("--size-field:{}; ", rem(v)));
        }
        if let Some(v) = self.size_selector {
            s.push_str(&format!("--size-selector:{}; ", rem(v)));
        }
        if let Some(v) = self.border {
            s.push_str(&format!("--border:{v}px; "));
        }
        if let Some(v) = self.depth {
            s.push_str(&format!("--depth:{}; ", u8::from(v)));
        }
        if let Some(v) = self.noise {
            s.push_str(&format!("--noise:{}; ", u8::from(v)));
        }
        let custom = normalize_hex(&self.bg_color).filter(|c| *c == self.bg_color);
        let bg = custom
            .as_deref()
            .or_else(|| css_of(BACKGROUNDS, &self.background))
            .unwrap_or("var(--color-base-200)");
        if safe_bg_image(&self.bg_image) {
            let dim = f32::from(self.bg_dim.min(MAX_BG_DIM)) / 100.0;
            let url = &self.bg_image;
            if dim > 0.0 {
                s.push_str(&format!(
                    "--mk-bg-img:linear-gradient(rgb(0 0 0 / {dim}),rgb(0 0 0 / {dim})),url({url}); "
                ));
            } else {
                s.push_str(&format!("--mk-bg-img:url({url}); "));
            }
            if self.bg_fixed {
                s.push_str("--mk-bg-att:fixed; ");
            }
        }
        s.push_str(&format!("--mk-bg:{bg}"));
        s
    }

    /// El tema en CSS, para copiarlo (botón "CSS" del editor).
    pub fn theme_css(&self, theme: &str) -> String {
        let b = theme_base(theme);
        let mut s = format!(
            "[data-theme=\"{}\"] {{\n  color-scheme: {};\n",
            if self.name.trim().is_empty() {
                "mi-tema"
            } else {
                self.name.trim()
            },
            if self.dark || b.dark { "dark" } else { "light" }
        );
        for (k, _) in COLOR_KEYS {
            s.push_str(&format!("  --color-{k}: {};\n", self.color(theme, k)));
        }
        s.push_str(&format!(
            "  --radius-box: {};\n  --radius-field: {};\n  --radius-selector: {};\n  --size-field: {};\n  --size-selector: {};\n  --border: {}px;\n  --depth: {};\n  --noise: {};\n}}",
            rem(f32::from(self.radius(theme, Corner::Box))),
            rem(f32::from(self.radius(theme, Corner::Field))),
            rem(f32::from(self.radius(theme, Corner::Selector))),
            rem(self.size_field.unwrap_or(b.size_field)),
            rem(self.size_selector.unwrap_or(b.size_selector)),
            self.border.unwrap_or(b.border),
            u8::from(self.depth.unwrap_or(b.depth)),
            u8::from(self.noise.unwrap_or(b.noise)),
        ));
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
        if !self.bg_color.is_empty()
            && normalize_hex(&self.bg_color).as_deref() != Some(&self.bg_color)
        {
            errors.push("Fondo: el color no es válido.".into());
        }
        if !self.bg_image.is_empty() && !safe_bg_image(&self.bg_image) {
            errors.push("Fondo: la imagen no es válida.".into());
        }
        if self.bg_dim > MAX_BG_DIM {
            errors.push(format!("Fondo: oscurecer hasta {MAX_BG_DIM} %."));
        }
        if self
            .colors
            .iter()
            .any(|(k, v)| color_index(k).is_none() || normalize_hex(v).as_deref() != Some(v))
        {
            errors.push("Colores: alguno no es un color válido.".into());
        }
        let radii = [self.radius_box, self.radius_field, self.radius_selector];
        if radii.iter().flatten().any(|r| *r > MAX_RADIUS) {
            errors.push(format!("Esquinas: hasta {MAX_RADIUS} px."));
        }
        let (lo, hi, _) = SIZE_RANGE;
        if [self.size_field, self.size_selector]
            .iter()
            .flatten()
            .any(|v| !(lo..=hi).contains(v))
        {
            errors.push(format!("Tamaños: entre {lo} y {hi} px."));
        }
        if self.border.is_some_and(|b| !(0.0..=4.0).contains(&b)) {
            errors.push("Bordes: entre 0 y 4 px.".into());
        }
        if self.name.chars().count() > 40 {
            errors.push("Nombre del tema: hasta 40 letras.".into());
        }
    }
}

/// Las tres familias de esquinas del tema.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Corner {
    /// Tarjetas, ventanas, avisos.
    Box,
    /// Botones, campos, pestañas.
    Field,
    /// Casillas, interruptores, etiquetas.
    Selector,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fresh_layout_is_full_width_and_old_docs_keep_their_look() {
        let f = StoreLayout::fresh();
        assert_eq!(f.gap, FRESH_GAP);
        assert_eq!(
            (f.pad_top, f.pad_bottom, f.pad_left, f.pad_right),
            (8, 8, 24, 24)
        );
        assert!(!f.is_default());
        let mut e = Vec::new();
        f.errors(&mut e);
        assert!(e.is_empty());
        // Un documento nuevo guarda sus márgenes; uno viejo, sin "layout",
        // sigue con los de siempre.
        let json = serde_json::to_string(&crate::PageDoc::default()).unwrap();
        assert!(json.contains("\"layout\""));
        let mut v: serde_json::Value = serde_json::from_str(&json).unwrap();
        v.as_object_mut().unwrap().remove("layout");
        let old: crate::PageDoc = serde_json::from_value(v).unwrap();
        assert_eq!(old.layout, StoreLayout::default());
    }

    #[test]
    fn default_keeps_the_old_look_and_validates() {
        let l = StoreLayout::default();
        assert!(Side::ALL.iter().all(|s| l.pad(*s) == 16));
        assert_eq!(l.max_width, 1120);
        assert_eq!(
            l.css_vars(),
            "--mk-pt:16px; --mk-pr:16px; --mk-pb:16px; --mk-pl:16px; --mk-gap:0px"
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
        // El ancho heredado ya no se valida ni llega al CSS.
        l.max_width = 0;
        assert!(!l.css_vars().contains("maxw"));
        let mut e = Vec::new();
        StoreLayout {
            pad_top: 500,
            max_width: 200,
            gap: 60,
            ..StoreLayout::default()
        }
        .errors(&mut e);
        assert_eq!(e.len(), 2);
    }

    #[test]
    fn gap_snaps_to_steps_and_old_documents_keep_zero() {
        let mut l = StoreLayout::default();
        l.set_gap(7);
        assert_eq!(l.gap, 8);
        l.set_gap(-3);
        assert_eq!(l.gap, 0);
        l.set_gap(999);
        assert_eq!(l.gap, MAX_GAP);
        assert_eq!(StoreLayout::fresh().gap, FRESH_GAP);
        // Sin el campo: 0 (como se ven hoy); el ancho heredado se conserva.
        let old: StoreLayout = serde_json::from_str(r#"{"max_width":960}"#).unwrap();
        assert_eq!((old.gap, old.max_width), (0, 960));
        let json = serde_json::to_string(&old).unwrap();
        assert!(json.contains("\"max_width\":960"));
    }

    #[test]
    fn style_changes_theme_radii_and_background() {
        let s = StoreStyle {
            buttons: "pill".into(),
            corners: "square".into(),
            background: "base".into(),
            ..StoreStyle::default()
        };
        let css = s.css_vars();
        assert!(css.contains("--radius-field:999px"));
        assert!(css.contains("--radius-box:0;"));
        assert!(css.contains("--mk-bg:var(--color-base-100)"));
        let mut img = StoreStyle::default();
        img.bg_image = "/img/banner.webp".into();
        img.bg_dim = 30;
        img.bg_fixed = true;
        let css = img.css_vars();
        assert!(css.contains("url(/img/banner.webp)") && css.contains("0.3"));
        assert!(css.contains("--mk-bg-att:fixed"));
        img.bg_image = "https://x.com/a.jpg);color:red".into();
        assert!(!img.css_vars().contains("--mk-bg-img"));
        img.bg_color = "#12ab34".into();
        assert!(img.css_vars().ends_with("--mk-bg:#12ab34"));
        let mut e = Vec::new();
        StoreStyle {
            buttons: "x".into(),
            ..StoreStyle::default()
        }
        .errors(&mut e);
        assert_eq!(e.len(), 1);
    }

    #[test]
    fn own_theme_changes_colors_with_readable_text_and_sizes() {
        let mut s = StoreStyle::default();
        assert!(s.theme_is_default());
        s.set_color("primary", Some("#FFD400"));
        s.set_radius(Corner::Field, 32);
        s.size_field = Some(5.0);
        s.border = Some(1.5);
        s.depth = Some(false);
        let css = s.css_vars();
        assert!(css.contains("--color-primary:#ffd400;"));
        // Amarillo claro: el texto sale oscuro.
        assert!(css.contains("--color-primary-content:#111111;"));
        assert!(css.contains("--radius-field:2rem;"));
        assert!(css.contains("--size-field:0.3125rem;"));
        assert!(css.contains("--border:1.5px;"));
        assert!(css.contains("--depth:0;"));
        assert_eq!(s.color("light", "primary-content"), "#111111");
        assert_eq!(s.color("light", "secondary"), "#f43098");
        // Color de texto elegido a mano.
        s.set_color("primary-content", Some("#ff0000"));
        assert!(s.css_vars().contains("--color-primary-content:#ff0000;"));
        let mut e = Vec::new();
        s.errors(&mut e);
        assert!(e.is_empty(), "{e:?}");
        // Un documento con colores sin sentido no se acepta.
        s.colors.insert("primary".into(), "rojo".into());
        s.errors(&mut e);
        assert_eq!(e.len(), 1);
        s.reset_theme();
        assert!(s.theme_is_default());
        assert!(StoreStyle::default()
            .theme_css("dark")
            .contains("color-scheme: dark"));
        // Los documentos de antes no cambian al guardarse.
        let json = serde_json::to_string(&StoreStyle::default()).unwrap();
        assert!(!json.contains("colors") && !json.contains("depth"));
    }

    #[test]
    fn readable_text_follows_wcag_threshold() {
        assert_eq!(readable_on("#ffffff"), "#111111");
        assert_eq!(readable_on("#000000"), "#ffffff");
        assert_eq!(readable_on("#422ad5"), "#ffffff");
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
