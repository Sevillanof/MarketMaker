//! Márgenes y ancho de toda la tienda: cuánto espacio queda entre el borde
//! de la pantalla y los bloques, y hasta qué ancho se estira el contenido.
//!
//! Vale para todas las páginas. Los documentos anteriores no lo traen y se
//! ven como siempre: margen Normal (16 px) y contenido de 1120 px.

use serde::{Deserialize, Serialize};

/// Margen alrededor del contenido: `(id, nombre, píxeles, para qué sirve)`.
pub const MARGINS: &[(&str, &str, u16, &str)] = &[
    ("narrow", "Angosto", 8, "Casi de borde a borde"),
    ("normal", "Normal", 16, "Como viene"),
    ("wide", "Amplio", 48, "Con aire a los costados"),
];

/// Anchos máximos sugeridos del contenido, en px. `0` es "Ancho completo".
pub const WIDTH_PRESETS: &[(u16, &str)] = &[
    (0, "Ancho completo"),
    (1440, "1440 px"),
    (1120, "1120 px"),
    (960, "960 px"),
];

/// Ancho del contenido que viene de fábrica.
pub const DEFAULT_WIDTH: u16 = 1120;
/// Ancho mínimo del contenido, al arrastrar los bordes.
pub const MIN_WIDTH: u16 = 640;
/// Ancho máximo del contenido (sin contar "Ancho completo").
pub const MAX_WIDTH: u16 = 1600;
/// Al arrastrar los bordes, el ancho salta de a estos píxeles.
pub const WIDTH_STEP: u16 = 40;
/// Con "Ancho completo", el editor dibuja la página como una pantalla de
/// este ancho como mínimo, el de una pantalla grande (si el lienzo es más
/// ancho, usa todo el lienzo).
pub const FULL_MIN_PX: u16 = 1600;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default)]
pub struct StoreLayout {
    /// Id de [`MARGINS`].
    pub margin: String,
    /// Ancho máximo del contenido en px; `0` = ancho completo.
    pub max_width: u16,
}

impl Default for StoreLayout {
    fn default() -> Self {
        Self {
            margin: "normal".into(),
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

    /// Margen en px.
    pub fn margin_px(&self) -> u16 {
        MARGINS
            .iter()
            .find(|(id, ..)| *id == self.margin)
            .map_or(16, |(_, _, px, _)| *px)
    }

    /// Nombre del margen para mostrar.
    pub fn margin_label(&self) -> &'static str {
        MARGINS
            .iter()
            .find(|(id, ..)| *id == self.margin)
            .map_or("Normal", |(_, l, ..)| *l)
    }

    /// Nombre del ancho para mostrar.
    pub fn width_label(&self) -> String {
        if self.is_full() {
            "Ancho completo".into()
        } else {
            format!("{} px", self.max_width)
        }
    }

    /// Estilo del contenedor de la tienda publicada (`.mk-wrap`).
    pub fn css_vars(&self) -> String {
        let max = if self.is_full() {
            "100%".to_string()
        } else {
            format!("{}px", self.max_width)
        };
        format!("--mk-pad:{}px; --mk-maxw:{max}", self.margin_px())
    }

    pub fn errors(&self, errors: &mut Vec<String>) {
        if !MARGINS.iter().any(|(id, ..)| *id == self.margin) {
            errors.push("Márgenes: Elegí Angosto, Normal o Amplio.".into());
        }
        if !self.is_full() && !(MIN_WIDTH..=MAX_WIDTH).contains(&self.max_width) {
            errors.push(format!(
                "Ancho de la tienda: tiene que estar entre {MIN_WIDTH} y {MAX_WIDTH} px."
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_keeps_the_old_look_and_validates() {
        let l = StoreLayout::default();
        assert_eq!(l.margin_px(), 16);
        assert_eq!(l.max_width, 1120);
        assert_eq!(l.css_vars(), "--mk-pad:16px; --mk-maxw:1120px");
        let mut e = Vec::new();
        l.errors(&mut e);
        assert!(e.is_empty());
    }

    #[test]
    fn full_width_and_bad_values() {
        let l = StoreLayout {
            margin: "wide".into(),
            max_width: 0,
        };
        assert_eq!(l.css_vars(), "--mk-pad:48px; --mk-maxw:100%");
        let mut e = Vec::new();
        StoreLayout {
            margin: "huge".into(),
            max_width: 200,
        }
        .errors(&mut e);
        assert_eq!(e.len(), 2);
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
