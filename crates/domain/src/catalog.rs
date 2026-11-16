//! Productos: validar lo que carga el dueño, leer precios en pesos y ordenar
//! etiquetas.

use crate::Product;
use serde::{Deserialize, Serialize};

pub const PRODUCT_NAME_MAX: usize = 120;
/// Unidades de un producto: más es seguro un error de tipeo.
pub const STOCK_MAX: u32 = 1_000_000;
pub const PRODUCT_DESC_MAX: usize = 600;
pub const PRODUCT_PHOTOS_MAX: usize = 10;
pub const PRODUCT_TAGS_MAX: usize = 20;
pub const TAG_MAX: usize = 30;
/// $ 1.000.000.000,00: más que eso es seguro un error de tipeo.
pub const PRICE_MAX_CENTS: i64 = 100_000_000_000;

/// Cómo se encuadra una foto dentro de su lugar en la tienda (tarjeta,
/// ficha, carrito), sin tocar la foto guardada: qué punto queda a la vista y
/// cuánto se acerca. La tienda decide la forma del lugar; este encuadre vale
/// para todas.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub struct PhotoFit {
    /// Punto que queda a la vista, en % del ancho (0 izquierda, 100 derecha).
    pub x: u8,
    /// Punto que queda a la vista, en % del alto (0 arriba, 100 abajo).
    pub y: u8,
    /// Zoom en %: 100 es la foto entera llenando el lugar.
    pub zoom: u16,
}

pub const ZOOM_MIN: u16 = 100;
pub const ZOOM_MAX: u16 = 300;

impl Default for PhotoFit {
    fn default() -> Self {
        Self {
            x: 50,
            y: 50,
            zoom: ZOOM_MIN,
        }
    }
}

impl PhotoFit {
    /// El mismo encuadre dentro de los límites.
    pub fn clamped(self) -> Self {
        Self {
            x: self.x.min(100),
            y: self.y.min(100),
            zoom: self.zoom.clamp(ZOOM_MIN, ZOOM_MAX),
        }
    }

    /// ¿Es como si no se hubiera ajustado?
    pub fn is_default(&self) -> bool {
        *self == Self::default()
    }

    /// Estilo para el `<img>` (con `object-fit: cover` o `contain`).
    pub fn css(&self) -> String {
        let Self { x, y, zoom } = self.clamped();
        let mut css = format!("object-position:{x}% {y}%");
        if zoom != ZOOM_MIN {
            let z = f32::from(zoom) / 100.0;
            css.push_str(&format!(";transform:scale({z});transform-origin:{x}% {y}%"));
        }
        css
    }
}

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

/// Las etiquetas de `products` con en cuántos están: la más usada primero
/// y, a igual cantidad, en orden alfabético.
pub fn count_tags(products: &[Product]) -> Vec<TagCount> {
    let mut out: Vec<TagCount> = Vec::new();
    for t in products.iter().flat_map(|p| &p.tags) {
        match out.iter_mut().find(|c| &c.tag == t) {
            Some(c) => c.products += 1,
            None => out.push(TagCount {
                tag: t.clone(),
                products: 1,
            }),
        }
    }
    out.sort_by(|a, b| b.products.cmp(&a.products).then_with(|| a.tag.cmp(&b.tag)));
    out
}

/// La etiqueta como se muestra en la tienda: con la primera letra en
/// mayúscula (se guardan en minúsculas).
pub fn tag_label(tag: &str) -> String {
    let mut chars = tag.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}

/// Las etiquetas de un producto después de guardar la etiqueta `old` con el
/// nombre `new` (`old` vacío: es nueva). `has` dice si el producto la lleva.
/// Si ya tenía `new`, no la repite; si ya tiene el máximo, no la agrega.
pub fn retag(tags: &[String], old: &str, new: &str, has: bool) -> Vec<String> {
    let mut out: Vec<String> = tags
        .iter()
        .filter(|t| t.as_str() != old && t.as_str() != new)
        .cloned()
        .collect();
    if has && out.len() < PRODUCT_TAGS_MAX {
        // En el mismo lugar que tenía la vieja (o la nueva), si estaba.
        let anchor = tags
            .iter()
            .position(|t| !old.is_empty() && t == old)
            .or_else(|| tags.iter().position(|t| t == new));
        let at = anchor.map_or(out.len(), |i| {
            tags[..i]
                .iter()
                .filter(|t| t.as_str() != old && t.as_str() != new)
                .count()
        });
        out.insert(at, new.to_string());
    }
    out
}

/// ¿Se ve `p` con el filtro del comprador? Sin etiquetas elegidas pasan
/// todos; con varias, alcanza con que tenga una. `query` busca como en
/// [`product_matches`].
pub fn passes_filter(p: &Product, tags: &[String], query: &str) -> bool {
    (tags.is_empty() || tags.iter().any(|t| p.tags.contains(t))) && product_matches(p, query)
}

/// El aviso arriba de una cuadrícula filtrada: «3 productos con Hogar o
/// Regalos». `None` si no hay filtro.
pub fn filter_summary(n: usize, tags: &[String], query: &str) -> Option<String> {
    let query = query.trim();
    if tags.is_empty() && query.is_empty() {
        return None;
    }
    let what = match n {
        0 => "Ningún producto".to_string(),
        1 => "1 producto".to_string(),
        n => format!("{n} productos"),
    };
    let mut parts = Vec::new();
    if !tags.is_empty() {
        let names: Vec<String> = tags.iter().map(|t| tag_label(t)).collect();
        let list = match names.split_last() {
            Some((last, rest)) if !rest.is_empty() => format!("{} o {last}", rest.join(", ")),
            _ => names.concat(),
        };
        parts.push(format!("con «{list}»"));
    }
    if !query.is_empty() {
        parts.push(format!("que coinciden con «{query}»"));
    }
    Some(format!("{what} {}", parts.join(" y ")))
}

/// Resultado de guardar o borrar una etiqueta desde "Etiquetas".
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum TagSave {
    /// Todos los productos de la tienda, ya con el cambio.
    Saved {
        products: Vec<Product>,
    },
    Invalid {
        error: String,
    },
}

/// Revisa el nombre de una etiqueta que se crea o renombra: `Ok` con el
/// nombre limpio, o el problema en palabras simples.
pub fn check_tag_name(name: &str) -> Result<String, String> {
    clean_tag(name).ok_or_else(|| "Escribí un nombre para la etiqueta.".to_string())
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
    // Solo los encuadres de fotos que el producto tiene, y que cambian algo.
    let photos = &p.photos;
    p.photo_fits.retain(|url, _| photos.contains(url));
    for fit in p.photo_fits.values_mut() {
        *fit = fit.clamped();
    }
    // Un paquete con algo en 0 o fuera de rango no sirve para cotizar.
    if let Some(x) = p.parcel {
        let sides = [x.length_mm, x.width_mm, x.height_mm];
        if x.weight_g == 0
            || x.weight_g > crate::PARCEL_MAX_G
            || sides.iter().any(|s| *s == 0 || *s > crate::PARCEL_MAX_MM)
        {
            p.parcel = None;
        }
    }
    p.photo_fits.retain(|_, fit| !fit.is_default());
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
    if p.stock.is_some_and(|s| s > STOCK_MAX) {
        errors.push(format!("La cantidad puede ser hasta {STOCK_MAX}."));
    }
    if p.stock.is_some() {
        // La cantidad manda: en 0 se ve "Agotado".
        p.in_stock = true;
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
    fn counts_tags_most_used_first() {
        let p = |tags: &[&str]| Product {
            tags: tags.iter().map(|t| t.to_string()).collect(),
            ..Product::default()
        };
        let c = count_tags(&[p(&["moda", "ofertas"]), p(&["ofertas"]), p(&["hogar"])]);
        let got: Vec<_> = c.iter().map(|t| (t.tag.as_str(), t.products)).collect();
        assert_eq!(got, [("ofertas", 2), ("hogar", 1), ("moda", 1)]);
        assert_eq!(tag_label("ñandú"), "Ñandú");
        assert_eq!(tag_label(""), "");
    }

    #[test]
    fn filters_and_summarizes() {
        let p = Product {
            name: "Taza".into(),
            tags: vec!["hogar".into(), "regalos".into()],
            ..Product::default()
        };
        let t = |v: &[&str]| v.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        assert!(passes_filter(&p, &[], ""));
        assert!(passes_filter(&p, &t(&["moda", "hogar"]), ""));
        assert!(!passes_filter(&p, &t(&["moda"]), ""));
        assert!(!passes_filter(&p, &[], "mochila"));
        assert_eq!(filter_summary(3, &[], " "), None);
        assert_eq!(
            filter_summary(1, &t(&["hogar"]), "").unwrap(),
            "1 producto con «Hogar»"
        );
        assert_eq!(
            filter_summary(3, &t(&["moda", "hogar", "regalos"]), "taza").unwrap(),
            "3 productos con «Moda, Hogar o Regalos» y que coinciden con «taza»"
        );
    }

    #[test]
    fn retags_products() {
        let v = |t: &[&str]| t.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        // Nueva: se agrega al final, o no se agrega.
        assert_eq!(retag(&v(&["a"]), "", "b", true), v(&["a", "b"]));
        assert_eq!(retag(&v(&["a"]), "", "b", false), v(&["a"]));
        // Renombrar: queda en el mismo lugar.
        assert_eq!(
            retag(&v(&["a", "b", "c"]), "b", "x", true),
            v(&["a", "x", "c"])
        );
        // Renombrar a una que ya tenía: no se repite.
        assert_eq!(retag(&v(&["a", "b", "c"]), "b", "c", true), v(&["a", "c"]));
        // Sacarla.
        assert_eq!(retag(&v(&["a", "b"]), "b", "b", false), v(&["a"]));
        // Ya la tenía: queda igual.
        assert_eq!(retag(&v(&["a", "b"]), "", "a", true), v(&["a", "b"]));
        // Con el máximo, no se agrega una más.
        let full: Vec<String> = (0..PRODUCT_TAGS_MAX).map(|i| i.to_string()).collect();
        assert_eq!(retag(&full, "", "nueva", true), full);
    }

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
    fn photo_fits_keep_only_real_adjustments() {
        let fit = |x, y, zoom| PhotoFit { x, y, zoom };
        let mut p = Product {
            name: "Taza".into(),
            photos: vec!["/media/a/960.webp".into(), "/media/b/960.webp".into()],
            photo_fits: [
                ("/media/a/960.webp".to_string(), fit(80, 120, 900)),
                ("/media/b/960.webp".to_string(), fit(50, 50, 100)),
                ("/media/viejo/960.webp".to_string(), fit(10, 10, 150)),
            ]
            .into(),
            ..Product::default()
        };
        assert!(tidy_product(&mut p).is_empty());
        assert_eq!(
            p.photo_fits.into_iter().collect::<Vec<_>>(),
            [("/media/a/960.webp".to_string(), fit(80, 100, ZOOM_MAX))]
        );
        assert_eq!(PhotoFit::default().css(), "object-position:50% 50%");
        assert_eq!(
            fit(80, 30, 135).css(),
            "object-position:80% 30%;transform:scale(1.35);transform-origin:80% 30%"
        );
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
