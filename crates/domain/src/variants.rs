//! Variantes de un producto: colores y talles, con stock por combinación.
//!
//! Son opcionales: un producto sin colores ni talles se vende como siempre
//! (un precio y un stock). Con variantes, cada combinación color × talle
//! tiene sus unidades, y el stock del producto es la suma.
//!
//! En el carrito cada combinación es un renglón propio, con el id
//! [`line_id`] (`producto~color~talle`).

use serde::{Deserialize, Serialize};

use crate::Product;

/// Más colores o talles que esto por producto es seguro un error.
pub const VARIANT_COLORS_MAX: usize = 20;
pub const VARIANT_SIZES_MAX: usize = 20;
/// Largo máximo del nombre de un color o un talle.
pub const VARIANT_NAME_MAX: usize = 24;
/// Separa el producto, el color y el talle en el id de un renglón.
const SEP: char = '~';

/// Un color del producto: su nombre, el punto de color y su foto.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
pub struct VariantColor {
    pub name: String,
    /// `#rrggbb` para el punto de color (vacío: sin punto).
    #[serde(default)]
    pub hex: String,
    /// Una de las fotos del producto (vacía: la principal).
    #[serde(default)]
    pub photo: String,
}

/// Unidades de una combinación (color o talle vacíos si el producto no
/// tiene esa opción).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
pub struct VariantStock {
    #[serde(default)]
    pub color: String,
    #[serde(default)]
    pub size: String,
    pub qty: u32,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
pub struct Variants {
    #[serde(default)]
    pub colors: Vec<VariantColor>,
    #[serde(default)]
    pub sizes: Vec<String>,
    #[serde(default)]
    pub stock: Vec<VariantStock>,
}

impl Variants {
    /// Sin colores ni talles: el producto se vende como siempre.
    pub fn is_empty(&self) -> bool {
        self.colors.is_empty() && self.sizes.is_empty()
    }

    /// Todas las combinaciones (color, talle), en el orden en que se cargaron.
    pub fn combos(&self) -> Vec<(String, String)> {
        if self.is_empty() {
            return Vec::new();
        }
        let colors: Vec<String> = if self.colors.is_empty() {
            vec![String::new()]
        } else {
            self.colors.iter().map(|c| c.name.clone()).collect()
        };
        let sizes: Vec<String> = if self.sizes.is_empty() {
            vec![String::new()]
        } else {
            self.sizes.clone()
        };
        colors
            .iter()
            .flat_map(|c| sizes.iter().map(move |s| (c.clone(), s.clone())))
            .collect()
    }

    /// Unidades de una combinación (0 si no existe).
    pub fn qty(&self, color: &str, size: &str) -> u32 {
        self.stock
            .iter()
            .find(|v| v.color == color && v.size == size)
            .map_or(0, |v| v.qty)
    }

    /// Deja `qty` unidades en una combinación.
    pub fn set_qty(&mut self, color: &str, size: &str, qty: u32) {
        match self.stock.iter_mut().find(|v| v.color == color && v.size == size) {
            Some(v) => v.qty = qty,
            None => self.stock.push(VariantStock {
                color: color.to_string(),
                size: size.to_string(),
                qty,
            }),
        }
    }

    /// Unidades de todas las combinaciones.
    pub fn total(&self) -> u32 {
        self.stock.iter().map(|v| v.qty).sum()
    }

    /// Unidades de un color sumando sus talles (o de un talle sumando sus
    /// colores, con `color` vacío y `size` puesto).
    pub fn qty_of_color(&self, color: &str) -> u32 {
        self.stock.iter().filter(|v| v.color == color).map(|v| v.qty).sum()
    }

    pub fn color(&self, name: &str) -> Option<&VariantColor> {
        self.colors.iter().find(|c| c.name == name)
    }

    /// Está la combinación (con las opciones que tiene el producto).
    pub fn has(&self, color: &str, size: &str) -> bool {
        self.combos().iter().any(|(c, s)| c == color && s == size)
    }

    /// Ordena lo cargado: nombres sin espacios de más ni repetidos, y una
    /// fila de stock por combinación (las que sobran se van, las que faltan
    /// quedan en 0). Devuelve los errores para mostrar.
    pub fn tidy(&mut self, photos: &[String]) -> Vec<String> {
        let mut errors = Vec::new();
        let clean = |s: &str| s.replace(SEP, "-").split_whitespace().collect::<Vec<_>>().join(" ");
        let mut seen = Vec::new();
        self.colors.retain_mut(|c| {
            c.name = clean(&c.name);
            c.hex = c.hex.trim().to_lowercase();
            if !is_hex(&c.hex) {
                c.hex.clear();
            }
            if !c.photo.is_empty() && !photos.contains(&c.photo) {
                c.photo.clear();
            }
            let key = c.name.to_lowercase();
            if c.name.is_empty() || seen.contains(&key) {
                return false;
            }
            seen.push(key);
            true
        });
        let mut seen = Vec::new();
        self.sizes = self
            .sizes
            .iter()
            .map(|s| clean(s))
            .filter(|s| {
                let key = s.to_lowercase();
                if s.is_empty() || seen.contains(&key) {
                    return false;
                }
                seen.push(key);
                true
            })
            .collect();
        if self.colors.len() > VARIANT_COLORS_MAX {
            errors.push(format!("Podés poner hasta {VARIANT_COLORS_MAX} colores."));
        }
        if self.sizes.len() > VARIANT_SIZES_MAX {
            errors.push(format!("Podés poner hasta {VARIANT_SIZES_MAX} talles."));
        }
        let long = self
            .colors
            .iter()
            .map(|c| c.name.as_str())
            .chain(self.sizes.iter().map(String::as_str))
            .any(|n| n.chars().count() > VARIANT_NAME_MAX);
        if long {
            errors.push(format!(
                "Cada color o talle puede tener hasta {VARIANT_NAME_MAX} letras."
            ));
        }
        let old = std::mem::take(&mut self.stock);
        self.stock = self
            .combos()
            .into_iter()
            .map(|(color, size)| {
                let qty = old
                    .iter()
                    .find(|v| v.color == color && v.size == size)
                    .map_or(0, |v| v.qty);
                VariantStock { color, size, qty }
            })
            .collect();
        if self.stock.iter().any(|v| v.qty > crate::STOCK_MAX) {
            errors.push(format!("La cantidad puede ser hasta {}.", crate::STOCK_MAX));
        }
        errors
    }
}

fn is_hex(s: &str) -> bool {
    s.len() == 7 && s.starts_with('#') && s[1..].chars().all(|c| c.is_ascii_hexdigit())
}

/// "Azul · 40", "Azul" o "40".
pub fn variant_label(color: &str, size: &str) -> String {
    match (color.is_empty(), size.is_empty()) {
        (false, false) => format!("{color} · {size}"),
        (false, true) => color.to_string(),
        (true, false) => size.to_string(),
        (true, true) => String::new(),
    }
}

/// El nombre del producto sin la combinación que le agrega
/// [`Product::for_variant`]: "Zapatillas · Azul · 40" → "Zapatillas".
pub fn base_name<'a>(name: &'a str, color: &str, size: &str) -> &'a str {
    let label = variant_label(color, size);
    if label.is_empty() {
        return name;
    }
    name.strip_suffix(&format!(" · {label}")).unwrap_or(name)
}

/// Id del renglón del carrito para una combinación (el del producto si no
/// tiene variantes).
pub fn line_id(product_id: &str, color: &str, size: &str) -> String {
    if color.is_empty() && size.is_empty() {
        product_id.to_string()
    } else {
        format!("{product_id}{SEP}{color}{SEP}{size}")
    }
}

/// Lo contrario de [`line_id`]: (producto, color, talle).
pub fn parse_line_id(line: &str) -> (&str, &str, &str) {
    let mut parts = line.splitn(3, SEP);
    let id = parts.next().unwrap_or_default();
    (id, parts.next().unwrap_or_default(), parts.next().unwrap_or_default())
}

impl Product {
    pub fn has_variants(&self) -> bool {
        !self.variants.is_empty()
    }

    /// El producto tal como se vende en una combinación: el nombre con la
    /// variante ("Zapatillas · Azul · 40"), la foto del color y sus unidades.
    /// `None` si el producto no tiene esa combinación.
    pub fn for_variant(&self, color: &str, size: &str) -> Option<Product> {
        if !self.variants.has(color, size) {
            return None;
        }
        let qty = self.variants.qty(color, size);
        let mut p = self.clone();
        p.id = line_id(&self.id, color, size);
        p.name = format!("{} · {}", self.name, variant_label(color, size));
        if let Some(photo) = self.variants.color(color).map(|c| c.photo.clone()) {
            if !photo.is_empty() {
                p.image_url = photo;
            }
        }
        p.stock = Some(qty);
        p.in_stock = self.in_stock && qty > 0;
        p.variants = Variants::default();
        Some(p)
    }

    /// Lo que puede ir al carrito: el producto, o cada combinación si tiene
    /// variantes.
    pub fn sellable(&self) -> Vec<Product> {
        if !self.has_variants() {
            return vec![self.clone()];
        }
        self.variants
            .combos()
            .iter()
            .filter_map(|(c, s)| self.for_variant(c, s))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base_name_drops_the_combination() {
        assert_eq!(base_name("Zapatillas · Azul · 40", "Azul", "40"), "Zapatillas");
        assert_eq!(base_name("Zapatillas · Azul", "Azul", ""), "Zapatillas");
        assert_eq!(base_name("Taza", "", ""), "Taza");
        // Si el nombre no termina en la combinación, queda como está.
        assert_eq!(base_name("Taza · roja", "Azul", ""), "Taza · roja");
    }

    fn shoes() -> Product {
        let mut p = Product {
            id: "p1".into(),
            name: "Zapatillas".into(),
            photos: vec!["/a.webp".into(), "/b.webp".into()],
            ..Product::default()
        };
        p.variants = Variants {
            colors: vec![
                VariantColor { name: " Azul ".into(), hex: "#2563EB".into(), photo: "/b.webp".into() },
                VariantColor { name: "rojo".into(), hex: "rojo".into(), photo: "/x.webp".into() },
                VariantColor { name: "Rojo".into(), ..Default::default() },
            ],
            sizes: vec!["40".into(), " 41 ".into(), "".into()],
            stock: vec![VariantStock { color: "Azul".into(), size: "41".into(), qty: 3 }],
        };
        p
    }

    #[test]
    fn tidy_cleans_names_and_makes_one_row_per_combo() {
        let mut p = shoes();
        let errors = p.variants.tidy(&p.photos.clone());
        assert!(errors.is_empty(), "{errors:?}");
        let v = &p.variants;
        assert_eq!(v.colors.len(), 2, "el repetido se va");
        assert_eq!((v.colors[0].name.as_str(), v.colors[0].hex.as_str()), ("Azul", "#2563eb"));
        assert_eq!((v.colors[1].hex.as_str(), v.colors[1].photo.as_str()), ("", ""));
        assert_eq!(v.sizes, vec!["40", "41"]);
        assert_eq!(v.stock.len(), 4);
        assert_eq!((v.qty("Azul", "41"), v.qty("rojo", "40"), v.total()), (3, 0, 3));
    }

    #[test]
    fn each_combo_sells_with_its_name_photo_and_stock() {
        let mut p = shoes();
        p.variants.tidy(&p.photos.clone());
        let line = p.for_variant("Azul", "41").unwrap();
        assert_eq!(line.id, "p1~Azul~41");
        assert_eq!(line.name, "Zapatillas · Azul · 41");
        assert_eq!(line.image_url, "/b.webp");
        assert_eq!((line.stock, line.in_stock), (Some(3), true));
        assert!(!p.for_variant("Azul", "40").unwrap().in_stock);
        assert!(p.for_variant("Verde", "40").is_none());
        assert_eq!(p.sellable().len(), 4);
        assert_eq!(parse_line_id("p1~Azul~41"), ("p1", "Azul", "41"));
        assert_eq!(parse_line_id("p1"), ("p1", "", ""));
        assert_eq!(line_id("p1", "", ""), "p1");
    }

    #[test]
    fn sizes_only_and_no_variants() {
        let mut p = Product { id: "p2".into(), name: "Remera".into(), ..Product::default() };
        assert_eq!(p.sellable().len(), 1);
        p.variants.sizes = vec!["S".into(), "M".into()];
        p.variants.tidy(&[]);
        p.variants.set_qty("", "M", 2);
        assert_eq!(p.variants.combos(), vec![(String::new(), "S".into()), (String::new(), "M".into())]);
        assert_eq!(p.for_variant("", "M").unwrap().name, "Remera · M");
        assert_eq!(p.for_variant("", "M").unwrap().id, "p2~~M");
    }
}
