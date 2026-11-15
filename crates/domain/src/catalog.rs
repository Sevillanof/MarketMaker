//! Productos: validar lo que carga el dueño, leer precios en pesos y ordenar
//! etiquetas.

use crate::Product;
use serde::{Deserialize, Serialize};

pub const PRODUCT_NAME_MAX: usize = 120;
pub const PRODUCT_DESC_MAX: usize = 600;
pub const PRODUCT_PHOTOS_MAX: usize = 10;
pub const PRODUCT_TAGS_MAX: usize = 20;
pub const TAG_MAX: usize = 30;
/// $ 1.000.000.000,00: más que eso es seguro un error de tipeo.
pub const PRICE_MAX_CENTS: i64 = 100_000_000_000;

/// Lo que ve el dueño en la pantalla Productos de una tienda.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct StoreProducts {
    pub store_name: String,
    pub products: Vec<Product>,
    /// Todas las etiquetas que usa la tienda, para sugerir y filtrar.
    pub tags: Vec<String>,
}

/// Resultado de guardar un producto.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum ProductSave {
    Saved,
    Invalid { errors: Vec<String> },
}

/// Un producto para editar, con lo que hace falta alrededor.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct ProductEdit {
    pub store_name: String,
    /// Vacío (id `""`) para uno nuevo.
    pub product: Product,
    /// Las etiquetas que ya usa la tienda, para sugerir.
    pub store_tags: Vec<TagCount>,
}

/// Una etiqueta de la tienda y en cuántos productos está.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct TagCount {
    pub tag: String,
    pub products: i64,
}

/// Una fila del menú de etiquetas.
#[derive(Clone, Debug, PartialEq)]
pub enum TagOption {
    /// Una que ya usa la tienda, con en cuántos productos está.
    Existing(TagCount),
    /// Crear la etiqueta (ya limpia) que se está escribiendo.
    Create(String),
}

/// Lo que muestra el menú de etiquetas: las de la tienda que todavía no
/// tiene el producto y coinciden con lo escrito, y al final crear lo escrito
/// si no existe.
pub fn tag_options(store: &[TagCount], chosen: &[String], typed: &str) -> Vec<TagOption> {
    let q = fold(typed.trim());
    let mut out: Vec<TagOption> = store
        .iter()
        .filter(|t| !chosen.contains(&t.tag))
        .filter(|t| q.is_empty() || fold(&t.tag).contains(&q))
        .take(8)
        .cloned()
        .map(TagOption::Existing)
        .collect();
    if let Some(new) = clean_tag(typed) {
        let exists = store.iter().any(|t| t.tag == new) || chosen.contains(&new);
        if !exists {
            out.push(TagOption::Create(new));
        }
    }
    out
}

/// Resultado de guardar: el id (nuevo o el mismo) o los problemas.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct SaveResult {
    pub id: String,
    pub outcome: ProductSave,
}

/// Lee un precio escrito a la argentina: `45.999,90`, `$ 45999`, `45999,9`.
/// Los puntos son de miles y la coma separa los centavos. `None` si no es
/// un número válido.
pub fn parse_price(text: &str) -> Option<i64> {
    let t: String = text
        .chars()
        .filter(|c| !c.is_whitespace() && *c != '$' && *c != '.')
        .collect();
    if t.is_empty() {
        return None;
    }
    let (units, dec) = t.split_once(',').unwrap_or((&t, ""));
    if units.is_empty() || !units.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    if dec.len() > 2 || !dec.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    let units: i64 = units.parse().ok()?;
    let cents: i64 = match dec.len() {
        0 => 0,
        1 => dec.parse::<i64>().ok()? * 10,
        _ => dec.parse().ok()?,
    };
    units.checked_mul(100)?.checked_add(cents)
}

/// Precio para escribir en un campo: `45.999,90` (sin `$`).
pub fn price_input(cents: i64) -> String {
    let cents = cents.max(0);
    let units = (cents / 100).to_string();
    let mut grouped = String::new();
    for (i, ch) in units.chars().enumerate() {
        if i > 0 && (units.len() - i).is_multiple_of(3) {
            grouped.push('.');
        }
        grouped.push(ch);
    }
    match cents % 100 {
        0 => grouped,
        c => format!("{grouped},{c:02}"),
    }
}

/// Una etiqueta prolija: sin espacios de más, sin `#` adelante, en
/// minúsculas y corta. `None` si queda vacía.
pub fn clean_tag(tag: &str) -> Option<String> {
    let t = tag.trim().trim_start_matches('#').trim();
    let t: String = t.split_whitespace().collect::<Vec<_>>().join(" ");
    let t: String = t.to_lowercase().chars().take(TAG_MAX).collect();
    (!t.is_empty()).then_some(t)
}

/// Etiquetas limpias y sin repetir, en el orden en que vinieron.
pub fn clean_tags<'a>(tags: impl IntoIterator<Item = &'a str>) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for t in tags.into_iter().filter_map(clean_tag) {
        if !out.contains(&t) {
            out.push(t);
        }
    }
    out
}

/// Deja el producto prolijo antes de guardarlo y devuelve los problemas en
/// palabras simples (vacío si está bien).
pub fn tidy_product(p: &mut Product) -> Vec<String> {
    p.name = p.name.trim().to_string();
    p.description = p.description.trim().to_string();
    p.tags = clean_tags(p.tags.iter().map(String::as_str));
    p.photos.retain(|u| !u.trim().is_empty());
    p.photos.dedup();
    p.image_url = p.photos.first().cloned().unwrap_or_default();
    let mut errors = Vec::new();
    if p.name.is_empty() {
        errors.push("Poné un nombre al producto.".to_string());
    } else if p.name.chars().count() > PRODUCT_NAME_MAX {
        errors.push(format!(
            "El nombre puede tener hasta {PRODUCT_NAME_MAX} letras."
        ));
    }
    if !(0..=PRICE_MAX_CENTS).contains(&p.price_cents) {
        errors.push("Revisá el precio: tiene que ser un número de pesos.".to_string());
    }
    if p.compare_at_cents != 0 && p.compare_at_cents <= p.price_cents {
        errors.push(
            "El precio anterior tiene que ser mayor que el precio, o quedar vacío.".to_string(),
        );
    }
    if p.compare_at_cents > PRICE_MAX_CENTS || p.compare_at_cents < 0 {
        errors.push("Revisá el precio anterior.".to_string());
    }
    if p.description.chars().count() > PRODUCT_DESC_MAX {
        errors.push(format!(
            "La descripción puede tener hasta {PRODUCT_DESC_MAX} letras."
        ));
    }
    if p.photos.len() > PRODUCT_PHOTOS_MAX {
        errors.push(format!(
            "Podés poner hasta {PRODUCT_PHOTOS_MAX} fotos por producto."
        ));
    }
    if p.photos
        .iter()
        .any(|u| !(u.starts_with('/') || u.starts_with("https://")))
    {
        errors.push("Alguna foto no tiene una dirección válida.".to_string());
    }
    if p.tags.len() > PRODUCT_TAGS_MAX {
        errors.push(format!(
            "Podés poner hasta {PRODUCT_TAGS_MAX} etiquetas por producto."
        ));
    }
    errors
}

/// ¿El producto coincide con lo buscado? Busca en el nombre, la descripción
/// y las etiquetas, sin importar mayúsculas ni tildes.
pub fn product_matches(p: &Product, query: &str) -> bool {
    let q = fold(query.trim());
    if q.is_empty() {
        return true;
    }
    let hay = fold(&format!(
        "{} {} {}",
        p.name,
        p.description,
        p.tags.join(" ")
    ));
    q.split_whitespace().all(|w| hay.contains(w))
}

/// Minúsculas y sin tildes, para buscar.
pub fn fold(s: &str) -> String {
    s.to_lowercase()
        .chars()
        .map(|c| match c {
            'á' | 'à' | 'ä' | 'â' => 'a',
            'é' | 'è' | 'ë' | 'ê' => 'e',
            'í' | 'ì' | 'ï' | 'î' => 'i',
            'ó' | 'ò' | 'ö' | 'ô' => 'o',
            'ú' | 'ù' | 'ü' | 'û' => 'u',
            c => c,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_argentine_prices() {
        assert_eq!(parse_price("45.999,90"), Some(4_599_990));
        assert_eq!(parse_price("$ 45999"), Some(4_599_900));
        assert_eq!(parse_price("45999,9"), Some(4_599_990));
        assert_eq!(parse_price("1.000.000"), Some(100_000_000));
        assert_eq!(parse_price(""), None);
        assert_eq!(parse_price("12,345"), None);
        assert_eq!(parse_price("doce"), None);
        assert_eq!(price_input(4_599_990), "45.999,90");
        assert_eq!(price_input(4_599_900), "45.999");
    }

    #[test]
    fn tidy_tags_and_checks() {
        let mut p = Product {
            name: "  Taza  ".into(),
            price_cents: 100,
            compare_at_cents: 50,
            photos: vec!["/media/a/960.webp".into(), "".into()],
            tags: vec!["#Cocina".into(), "cocina ".into(), "  regalo   día ".into()],
            ..Product::default()
        };
        let errors = tidy_product(&mut p);
        assert_eq!(p.name, "Taza");
        assert_eq!(p.tags, ["cocina", "regalo día"]);
        assert_eq!(p.image_url, "/media/a/960.webp");
        assert_eq!(
            errors,
            ["El precio anterior tiene que ser mayor que el precio, o quedar vacío."]
        );
        p.compare_at_cents = 0;
        assert!(tidy_product(&mut p).is_empty());
        p.name = " ".into();
        assert_eq!(tidy_product(&mut p).len(), 1);
    }

    #[test]
    fn tag_menu_filters_and_offers_to_create() {
        let store = vec![
            TagCount {
                tag: "regalo".into(),
                products: 4,
            },
            TagCount {
                tag: "recuerdos".into(),
                products: 1,
            },
            TagCount {
                tag: "cocina".into(),
                products: 2,
            },
        ];
        let chosen = vec!["cocina".to_string()];
        assert_eq!(tag_options(&store, &chosen, "").len(), 2);
        let opts = tag_options(&store, &chosen, "Re");
        assert_eq!(opts.len(), 3);
        assert_eq!(opts[2], TagOption::Create("re".into()));
        assert_eq!(tag_options(&store, &chosen, "regalo").len(), 1);
        assert!(tag_options(&store, &chosen, "cocina").is_empty());
    }

    #[test]
    fn search_by_name_description_and_tags() {
        let p = Product {
            name: "Taza de cerámica".into(),
            tags: vec!["cocina".into()],
            ..Product::default()
        };
        assert!(product_matches(&p, "ceramica"));
        assert!(product_matches(&p, "TAZA cocina"));
        assert!(!product_matches(&p, "remera"));
        assert_eq!(
            Product {
                price_cents: 88,
                compare_at_cents: 100,
                ..p.clone()
            }
            .discount_pct(),
            Some(12)
        );
        assert_eq!(p.discount_pct(), None);
    }
}
