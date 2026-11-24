//! Importar productos desde una planilla (CSV): leerla, completar lo que
//! falta fila por fila y convertirla en productos de la tienda.

use crate::{fold, parse_price, tidy_product, Product};
use serde::{Deserialize, Serialize};

/// Cuántas filas se leen como máximo de una planilla.
pub const IMPORT_MAX_ROWS: usize = 500;

/// La planilla modelo que se baja: ya con las columnas y un producto de
/// ejemplo. Separada por `;`, que es lo que abre bien el Excel en español.
pub const IMPORT_TEMPLATE: &str = "\u{feff}nombre;precio;precio anterior;stock;categoria;subcategoria;etiquetas;descripcion;visible\r\nTaza de ceramica;6500;;40;Hogar;Cocina;regalo, cocina;Hecha a mano. Apta para microondas.;si\r\n";

/// Una fila de la planilla, con todo como texto para poder corregirlo en la
/// pantalla antes de importar.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
pub struct ImportRow {
    /// Número propio de la fila; no cambia aunque se quiten otras.
    pub id: u32,
    pub name: String,
    pub price: String,
    pub was: String,
    pub stock: String,
    pub category: String,
    pub subcategory: String,
    pub tags: String,
    pub description: String,
    pub visible: String,
    /// Direcciones de las fotos (de «Mis fotos»).
    pub photos: Vec<String>,
}

/// Lo que se leyó de la planilla.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
pub struct ImportSheet {
    pub rows: Vec<ImportRow>,
    /// Filas vacías o de más que se dejaron afuera.
    pub ignored: usize,
}

impl ImportRow {
    /// El producto de esta fila, o los problemas en palabras simples.
    pub fn to_product(&self) -> Result<Product, Vec<String>> {
        let mut errors = Vec::new();
        let mut p = Product {
            name: self.name.clone(),
            description: self.description.clone(),
            category: self.category.clone(),
            subcategory: self.subcategory.clone(),
            photos: self.photos.clone(),
            tags: self
                .tags
                .split([',', ';', '|'])
                .map(str::to_string)
                .collect(),
            visible: parse_yes(&self.visible),
            ..Product::default()
        };
        match parse_price(&self.price) {
            Some(c) => p.price_cents = c,
            None if self.price.trim().is_empty() => {}
            None => errors.push("Revisá el precio: tiene que ser un número de pesos.".into()),
        }
        if !self.was.trim().is_empty() {
            match parse_price(&self.was) {
                Some(c) => p.compare_at_cents = c,
                None => errors.push("Revisá el precio anterior.".into()),
            }
        }
        if !self.stock.trim().is_empty() {
            match self.stock.trim().replace('.', "").parse::<u32>() {
                Ok(n) => p.stock = Some(n),
                Err(_) => errors.push("La cantidad tiene que ser un número entero.".into()),
            }
        }
        if !errors.is_empty() {
            // Primero lo que no se pudo leer; el resto se revisa recién después.
            return Err(errors);
        }
        let problems = tidy_product(&mut p);
        if problems.is_empty() {
            Ok(p)
        } else {
            Err(problems)
        }
    }

    /// El primer problema de la fila, si lo tiene.
    pub fn problem(&self) -> Option<String> {
        self.to_product().err().and_then(|e| e.into_iter().next())
    }

    /// ¿La fila ya tiene alguna foto?
    pub fn has_photos(&self) -> bool {
        !self.photos.is_empty()
    }
}

/// «sí», «si», «1», «true»… → visible. Vacío también: lo normal es venderlo.
fn parse_yes(s: &str) -> bool {
    !matches!(
        fold(s.trim()).as_str(),
        "no" | "n" | "0" | "false" | "oculto" | "oculta"
    )
}

/// Las columnas que se entienden, por el nombre del encabezado (sin tildes,
/// espacios ni guiones).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Col {
    Name,
    Price,
    Was,
    Stock,
    Category,
    Subcategory,
    Tags,
    Description,
    Visible,
}

fn column(header: &str) -> Option<Col> {
    let key: String = fold(header)
        .chars()
        .filter(|c| c.is_alphanumeric())
        .collect();
    Some(match key.as_str() {
        "nombre" | "producto" | "titulo" | "name" => Col::Name,
        "precio" | "price" | "preciodeventa" => Col::Price,
        "precioanterior" | "preciolista" | "preciotachado" | "antes" | "was" => Col::Was,
        "stock" | "cantidad" | "unidades" => Col::Stock,
        "categoria" | "category" => Col::Category,
        "subcategoria" | "subcategory" => Col::Subcategory,
        "etiquetas" | "etiqueta" | "tags" => Col::Tags,
        "descripcion" | "description" | "detalle" => Col::Description,
        "visible" | "enlatienda" | "mostrar" => Col::Visible,
        _ => return None,
    })
}

/// Convierte los bytes de un archivo en texto: UTF-8 y, si no lo es (el CSV
/// que guarda Excel en Windows), Windows-1252.
pub fn decode_text(bytes: &[u8]) -> String {
    let bytes = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(bytes);
    match std::str::from_utf8(bytes) {
        Ok(s) => s.to_string(),
        Err(_) => bytes.iter().map(|&b| cp1252(b)).collect(),
    }
}

fn cp1252(b: u8) -> char {
    match b {
        0x80 => '€',
        0x82 => '‚',
        0x83 => 'ƒ',
        0x84 => '„',
        0x85 => '…',
        0x86 => '†',
        0x87 => '‡',
        0x88 => 'ˆ',
        0x89 => '‰',
        0x8A => 'Š',
        0x8B => '‹',
        0x8C => 'Œ',
        0x8E => 'Ž',
        0x91 => '‘',
        0x92 => '’',
        0x93 => '“',
        0x94 => '”',
        0x95 => '•',
        0x96 => '–',
        0x97 => '—',
        0x98 => '˜',
        0x99 => '™',
        0x9A => 'š',
        0x9B => '›',
        0x9C => 'œ',
        0x9E => 'ž',
        0x9F => 'Ÿ',
        b => char::from(b),
    }
}

/// Parte el texto en filas y celdas: separador `,`, `;` o tabulación (el que
/// más aparece en el encabezado) y comillas dobles para celdas con saltos o
/// separadores adentro.
fn split_csv(text: &str) -> Vec<Vec<String>> {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let first = text.lines().next().unwrap_or("");
    let sep = [';', ',', '\t']
        .into_iter()
        .max_by_key(|c| first.matches(*c).count())
        .filter(|c| first.contains(*c))
        .unwrap_or(';');
    let mut rows: Vec<Vec<String>> = Vec::new();
    let mut row: Vec<String> = Vec::new();
    let mut cell = String::new();
    let mut quoted = false;
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if quoted {
            if c == '"' {
                if chars.peek() == Some(&'"') {
                    cell.push('"');
                    chars.next();
                } else {
                    quoted = false;
                }
            } else {
                cell.push(c);
            }
            continue;
        }
        match c {
            '"' if cell.is_empty() => quoted = true,
            c if c == sep => row.push(std::mem::take(&mut cell)),
            '\r' => {}
            '\n' => {
                row.push(std::mem::take(&mut cell));
                rows.push(std::mem::take(&mut row));
            }
            c => cell.push(c),
        }
    }
    if !cell.is_empty() || !row.is_empty() {
        row.push(cell);
        rows.push(row);
    }
    rows
}

/// Lee la planilla. Falla, con un mensaje claro, si no tiene al menos las
/// columnas `nombre` y `precio`.
pub fn parse_import(text: &str) -> Result<ImportSheet, String> {
    let mut lines = split_csv(text).into_iter();
    let header = lines
        .next()
        .ok_or_else(|| "El archivo está vacío.".to_string())?;
    let cols: Vec<Option<Col>> = header.iter().map(|h| column(h)).collect();
    if !cols.contains(&Some(Col::Name)) || !cols.contains(&Some(Col::Price)) {
        return Err(
            "No encontramos las columnas «nombre» y «precio». Bajá la planilla modelo y usala de base."
                .into(),
        );
    }
    let mut sheet = ImportSheet::default();
    for line in lines {
        if line.iter().all(|c| c.trim().is_empty()) {
            sheet.ignored += 1;
            continue;
        }
        if sheet.rows.len() >= IMPORT_MAX_ROWS {
            sheet.ignored += 1;
            continue;
        }
        let mut row = ImportRow {
            id: sheet.rows.len() as u32 + 1,
            visible: String::new(),
            ..ImportRow::default()
        };
        for (i, cell) in line.into_iter().enumerate() {
            let cell = cell.trim().to_string();
            match cols.get(i).copied().flatten() {
                Some(Col::Name) => row.name = cell,
                Some(Col::Price) => row.price = cell,
                Some(Col::Was) => row.was = cell,
                Some(Col::Stock) => row.stock = cell,
                Some(Col::Category) => row.category = cell,
                Some(Col::Subcategory) => row.subcategory = cell,
                Some(Col::Tags) => row.tags = cell,
                Some(Col::Description) => row.description = cell,
                Some(Col::Visible) => row.visible = cell,
                None => {}
            }
        }
        sheet.rows.push(row);
    }
    if sheet.rows.is_empty() {
        return Err("La planilla no tiene productos.".into());
    }
    Ok(sheet)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lee_la_planilla_modelo() {
        let sheet = parse_import(IMPORT_TEMPLATE).unwrap();
        assert_eq!(sheet.rows.len(), 1);
        let p = sheet.rows[0].to_product().unwrap();
        assert_eq!(p.name, "Taza de ceramica");
        assert_eq!(p.price_cents, 650_000);
        assert_eq!(p.stock, Some(40));
        assert_eq!(p.tags, vec!["regalo", "cocina"]);
        assert!(p.visible);
    }

    #[test]
    fn separador_coma_y_comillas() {
        let t = "Nombre,Precio,Descripción\n\"Remera, azul\",\"1.500,50\",\"Dice \"\"hola\"\"\"\n";
        let sheet = parse_import(t).unwrap();
        let r = &sheet.rows[0];
        assert_eq!(r.name, "Remera, azul");
        assert_eq!(r.price, "1.500,50");
        assert_eq!(r.description, "Dice \"hola\"");
        assert_eq!(r.to_product().unwrap().price_cents, 150_050);
    }

    #[test]
    fn falta_el_precio_y_se_corrige() {
        let sheet = parse_import("nombre;precio\nParlante;\n").unwrap();
        let mut r = sheet.rows[0].clone();
        assert_eq!(r.problem().as_deref(), Some("Poné el precio del producto."));
        r.price = "12000".into();
        assert!(r.problem().is_none());
    }

    #[test]
    fn sin_columnas_obligatorias() {
        assert!(parse_import("a;b\n1;2\n").is_err());
        assert!(parse_import("").is_err());
        assert!(parse_import("nombre;precio\n").is_err());
    }

    #[test]
    fn filas_vacias_y_visible() {
        let sheet = parse_import("nombre;precio;visible\nA;10;no\n;;\nB;20;sí\n").unwrap();
        assert_eq!(sheet.rows.len(), 2);
        assert_eq!(sheet.ignored, 1);
        assert!(!sheet.rows[0].to_product().unwrap().visible);
        assert!(sheet.rows[1].to_product().unwrap().visible);
    }

    #[test]
    fn texto_de_excel_en_windows() {
        // «Ñandú» en Windows-1252.
        let bytes = [0xD1, b'a', b'n', b'd', 0xFA];
        assert_eq!(decode_text(&bytes), "Ñandú");
        assert_eq!(decode_text("Ñandú".as_bytes()), "Ñandú");
    }

    #[test]
    fn numeros_malos() {
        let r = ImportRow {
            name: "X".into(),
            price: "abc".into(),
            ..ImportRow::default()
        };
        assert!(r.problem().unwrap().contains("precio"));
        let r = ImportRow {
            name: "X".into(),
            price: "10".into(),
            stock: "mucho".into(),
            ..ImportRow::default()
        };
        assert!(r.problem().unwrap().contains("cantidad"));
    }
}
