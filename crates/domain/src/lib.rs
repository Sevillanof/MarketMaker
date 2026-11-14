//! Tipos del panel y su validación. Sin Leptos y sin IO: compila a nativo y a WASM.

use serde::{Deserialize, Serialize};

mod widgets;

pub use widgets::*;

/// Versión del formato del documento. La 1 guardaba el tamaño como el id de
/// una opción fija (`"size": "full"`); la 2 guarda ancho y alto libres en una
/// grilla de 12 columnas y filas de 96 px; la 3, en una grilla fina de
/// [`GRID_COLS`] columnas y filas de [`ROW_PX`] px.
pub const SCHEMA_VERSION: u32 = 3;
pub const GRID_COLS: u8 = 48;
/// Alto de una fila de la grilla, en píxeles (el hueco entre bloques incluido).
pub const ROW_PX: u16 = 8;
/// Cuántas columnas y filas de la grilla fina equivalen a una celda del schema 2.
pub const SCHEMA_2_SCALE: (u16, u16) = (4, 12);
pub const MAX_NODES: usize = 100;
pub const THEMES: &[&str] = &["light", "dark", "cupcake", "emerald", "corporate", "retro"];

/// Nombre para mostrar de cada tema, en el mismo orden que [`THEMES`].
pub const THEME_LABELS: &[&str] = &[
    "Claro",
    "Oscuro",
    "Pastel",
    "Esmeralda",
    "Corporativo",
    "Retro",
];

/// Nombre para mostrar de un tema; si no se conoce, el nombre interno.
pub fn theme_label(theme: &str) -> &str {
    THEMES
        .iter()
        .position(|t| *t == theme)
        .map_or(theme, |i| THEME_LABELS[i])
}

/// Documento del panel de una página. Se guarda entero en `page_versions.doc`.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct PageDoc {
    pub schema: u32,
    pub theme: String,
    pub nodes: Vec<Node>,
}

impl Default for PageDoc {
    fn default() -> Self {
        Self {
            schema: SCHEMA_VERSION,
            theme: THEMES[0].to_string(),
            nodes: Vec::new(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Node {
    pub id: String,
    #[serde(flatten)]
    pub widget: Widget,
    pub pos: Pos,
    pub size: Size,
}

impl Node {
    /// Nodo nuevo en `(0, 0)` con el tamaño por defecto del widget.
    pub fn new(id: impl Into<String>, widget: Widget) -> Self {
        let size = widget.limits().default;
        Self {
            id: id.into(),
            widget,
            pos: Pos { x: 0, y: 0 },
            size,
        }
    }
}

/// Posición en celdas desde la esquina superior izquierda.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
pub struct Pos {
    pub x: u8,
    pub y: u16,
}

/// Tamaño en celdas: `w` columnas (de [`GRID_COLS`]) por `h` filas de [`ROW_PX`] px.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub struct Size {
    pub w: u8,
    pub h: u8,
}

/// Ancho y alto permitidos para un tipo de widget, en celdas.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SizeLimits {
    pub min_w: u8,
    pub max_w: u8,
    pub min_h: u8,
    pub max_h: u8,
    pub default: Size,
}

impl SizeLimits {
    /// Lleva `w` y `h` al rango permitido.
    pub fn clamp(&self, w: i32, h: i32) -> Size {
        Size {
            w: w.clamp(i32::from(self.min_w), i32::from(self.max_w)) as u8,
            h: h.clamp(i32::from(self.min_h), i32::from(self.max_h)) as u8,
        }
    }

    pub fn contains(&self, size: Size) -> bool {
        (self.min_w..=self.max_w).contains(&size.w) && (self.min_h..=self.max_h).contains(&size.h)
    }
}

#[derive(Debug, PartialEq)]
pub enum FieldKind {
    Text {
        max: usize,
    },
    LongText {
        max: usize,
    },
    Url,
    Number {
        min: i64,
        max: i64,
    },
    Bool,
    Select {
        options: &'static [(&'static str, &'static str)],
    },
    ProductRef,
    /// Fecha y hora local `AAAA-MM-DDTHH:MM`.
    DateTime,
    /// Lista de elementos, cada uno con los campos de `item`.
    List {
        item: &'static [Field],
        min: usize,
        max: usize,
    },
}

#[derive(Debug, PartialEq)]
pub struct Field {
    pub key: &'static str,
    pub label: &'static str,
    pub kind: FieldKind,
    pub advanced: bool,
}

/// Tamaños de las opciones fijas del schema 1, por tipo de widget.
const LEGACY_SIZES: &[(&str, &str, u8, u8)] = &[
    ("Banner", "full", 12, 4),
    ("Banner", "half", 6, 4),
    ("Text", "small", 4, 2),
    ("Text", "medium", 6, 3),
    ("Text", "full", 12, 2),
    ("ProductGrid", "one_row", 12, 5),
    ("ProductGrid", "two_rows", 12, 9),
    ("ProductGrid", "half", 6, 9),
    ("FeaturedProduct", "small", 3, 4),
    ("FeaturedProduct", "medium", 4, 5),
    ("FeaturedProduct", "large", 6, 5),
];

/// Lee un documento guardado, de cualquier versión, y lo lleva a la actual.
///
/// Los documentos del schema 1 traen `"size": "<opción>"`; se convierten a
/// `{ "w", "h" }` con la tabla de las opciones de entonces. Los del schema 2
/// usan celdas grandes (12 columnas, filas de 96 px) y se pasan a la grilla
/// fina multiplicando por [`SCHEMA_2_SCALE`], así se ven igual que antes.
pub fn parse_doc(mut value: serde_json::Value) -> Result<PageDoc, String> {
    let mut schema = value.get("schema").and_then(|s| s.as_u64()).unwrap_or(0);
    if schema == 1 {
        if let Some(nodes) = value.get_mut("nodes").and_then(|n| n.as_array_mut()) {
            for node in nodes {
                let t = node
                    .get("type")
                    .and_then(|t| t.as_str())
                    .unwrap_or_default();
                let preset = node
                    .get("size")
                    .and_then(|s| s.as_str())
                    .unwrap_or_default();
                let (w, h) = LEGACY_SIZES
                    .iter()
                    .find(|(lt, lp, _, _)| *lt == t && *lp == preset)
                    .map(|(_, _, w, h)| (*w, *h))
                    .or_else(|| {
                        // El tamaño por defecto actual, en celdas del schema 2.
                        Widget::defaults(t).map(|w| {
                            let d = w.limits().default;
                            let (sx, sy) = SCHEMA_2_SCALE;
                            ((u16::from(d.w) / sx) as u8, (u16::from(d.h) / sy) as u8)
                        })
                    })
                    .unwrap_or((12, 2));
                node["size"] = serde_json::json!({ "w": w, "h": h });
            }
        }
        schema = 2;
    }
    if schema == 2 {
        let (sx, sy) = SCHEMA_2_SCALE;
        let scale = |v: &mut serde_json::Value, key: &str, k: u16| {
            if let Some(n) = v.get(key).and_then(|n| n.as_u64()) {
                v[key] = serde_json::json!(n * u64::from(k));
            }
        };
        if let Some(nodes) = value.get_mut("nodes").and_then(|n| n.as_array_mut()) {
            for node in nodes {
                if let Some(pos) = node.get_mut("pos") {
                    scale(pos, "x", sx);
                    scale(pos, "y", sy);
                }
                if let Some(size) = node.get_mut("size") {
                    scale(size, "w", sx);
                    scale(size, "h", sy);
                }
            }
        }
        value["schema"] = serde_json::json!(SCHEMA_VERSION);
    }
    serde_json::from_value(value).map_err(|e| e.to_string())
}

/// Lo que necesita el marketplace (y el editor) para mostrar una página.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct PageView {
    pub doc: PageDoc,
    pub version: u32,
    pub products: Vec<Product>,
}

/// Resultado de intentar guardar un documento.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum SaveOutcome {
    /// Se guardó como la versión `version`.
    Saved { version: u32 },
    /// La versión base quedó vieja: alguien guardó antes. `current` es la vigente.
    Conflict { current: u32 },
    /// El documento no pasó la validación; no se escribió nada.
    Invalid { errors: Vec<String> },
}

/// Producto de ejemplo del tenant, tal como lo usan los widgets.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Product {
    pub id: String,
    pub name: String,
    pub price_cents: i64,
    pub image_url: String,
}

fn url_ok(s: &str) -> bool {
    s.is_empty() || s.starts_with("https://") || s.starts_with("http://") || s.starts_with('/')
}

/// `AAAA-MM-DDTHH:MM` (o con segundos), o vacío.
fn datetime_ok(s: &str) -> bool {
    if s.is_empty() {
        return true;
    }
    let b = s.as_bytes();
    let digits =
        |r: std::ops::Range<usize>| b.get(r).is_some_and(|d| d.iter().all(u8::is_ascii_digit));
    (s.len() == 16 || s.len() == 19)
        && digits(0..4)
        && b[4] == b'-'
        && digits(5..7)
        && b[7] == b'-'
        && digits(8..10)
        && b[10] == b'T'
        && digits(11..13)
        && b[13] == b':'
        && digits(14..16)
        && (s.len() == 16 || (b[16] == b':' && digits(17..19)))
}

/// Problema de un valor según el tipo de su campo, en palabras para el
/// usuario (sin nombres internos). `None` si el valor es válido.
///
/// Las listas solo se chequean por cantidad: sus elementos se validan campo
/// por campo con esta misma función.
pub fn field_error(kind: &FieldKind, v: Option<&serde_json::Value>) -> Option<String> {
    match kind {
        FieldKind::Text { max } | FieldKind::LongText { max } => match v.and_then(|v| v.as_str()) {
            Some(s) if s.chars().count() <= *max => None,
            Some(_) => Some(format!("Usá {max} caracteres como máximo.")),
            None => Some("Tiene que ser un texto.".to_string()),
        },
        FieldKind::Url => match v.and_then(|v| v.as_str()) {
            Some(s) if url_ok(s) => None,
            Some(_) => {
                Some("Escribí una dirección que empiece con https://, http:// o /.".to_string())
            }
            None => Some("Tiene que ser un texto.".to_string()),
        },
        FieldKind::Number { min, max } => match v.and_then(|v| v.as_i64()) {
            Some(n) if (*min..=*max).contains(&n) => None,
            Some(_) => Some(format!("Elegí un número entre {min} y {max}.")),
            None => Some("Escribí un número entero.".to_string()),
        },
        FieldKind::Bool => {
            (!v.is_some_and(|v| v.is_boolean())).then(|| "Tiene que ser sí o no.".to_string())
        }
        FieldKind::Select { options } => match v.and_then(|v| v.as_str()) {
            Some(s) if options.iter().any(|(k, _)| *k == s) => None,
            _ => Some("Elegí una de las opciones.".to_string()),
        },
        FieldKind::ProductRef => {
            (!v.is_some_and(|v| v.is_string())).then(|| "Elegí un producto.".to_string())
        }
        FieldKind::DateTime => match v.and_then(|v| v.as_str()) {
            Some(s) if datetime_ok(s) => None,
            _ => Some("Elegí una fecha y una hora.".to_string()),
        },
        FieldKind::List { min, max, .. } => match v.and_then(|v| v.as_array()) {
            Some(items) if (*min..=*max).contains(&items.len()) => None,
            Some(_) => Some(format!("Tiene que tener entre {min} y {max} elementos.")),
            None => Some("Tiene que ser una lista.".to_string()),
        },
    }
}

fn validate_props(
    fields: &[Field],
    props: Option<&serde_json::Value>,
    path: &str,
    errors: &mut Vec<String>,
) {
    for field in fields {
        let v = props.and_then(|p| p.get(field.key));
        if let Some(msg) = field_error(&field.kind, v) {
            errors.push(format!("{path}{}: {msg}", field.label));
        }
        if let FieldKind::List { item, .. } = &field.kind {
            for (i, it) in v
                .and_then(|v| v.as_array())
                .into_iter()
                .flatten()
                .enumerate()
            {
                let sub = format!("{path}{} {} › ", field.label, i + 1);
                validate_props(item, Some(it), &sub, errors);
            }
        }
    }
}

fn validate_fields(node: &Node, errors: &mut Vec<String>) {
    let value = match serde_json::to_value(&node.widget) {
        Ok(v) => v,
        Err(e) => {
            errors.push(format!("nodo {}: no se pudo leer: {e}", node.id));
            return;
        }
    };
    let path = format!("{} › ", node.widget.label());
    validate_props(node.widget.fields(), value.get("props"), &path, errors);
}

/// Valida un documento completo. El servidor rechaza el guardado si hay algún error.
pub fn validate(doc: &PageDoc) -> Result<(), Vec<String>> {
    let mut errors = Vec::new();

    if doc.schema != SCHEMA_VERSION {
        errors.push(format!(
            "schema {} no soportado (se espera {SCHEMA_VERSION})",
            doc.schema
        ));
    }
    if !THEMES.contains(&doc.theme.as_str()) {
        errors.push(format!("tema desconocido: {}", doc.theme));
    }
    if doc.nodes.len() > MAX_NODES {
        errors.push(format!(
            "hay {} nodos; el máximo es {MAX_NODES}",
            doc.nodes.len()
        ));
    }

    let mut ids = std::collections::HashSet::new();
    // (id, x, y, w, h) de cada nodo, para chequear superposiciones.
    let mut rects: Vec<(&str, u32, u32, u32, u32)> = Vec::new();
    for node in &doc.nodes {
        if node.id.is_empty() {
            errors.push("hay un nodo sin id".to_string());
        } else if !ids.insert(node.id.as_str()) {
            errors.push(format!("id repetido: {}", node.id));
        }

        let l = node.widget.limits();
        if !l.contains(node.size) {
            errors.push(format!(
                "nodo {}: tamaño {}×{} fuera de los límites de {} ({}–{} columnas, {}–{} filas)",
                node.id,
                node.size.w,
                node.size.h,
                node.widget.label(),
                l.min_w,
                l.max_w,
                l.min_h,
                l.max_h
            ));
        }
        if u32::from(node.pos.x) + u32::from(node.size.w) > u32::from(GRID_COLS) {
            errors.push(format!(
                "nodo {}: se sale de las {GRID_COLS} columnas",
                node.id
            ));
        }
        rects.push((
            node.id.as_str(),
            u32::from(node.pos.x),
            u32::from(node.pos.y),
            u32::from(node.size.w),
            u32::from(node.size.h),
        ));

        validate_fields(node, &mut errors);
    }

    for (i, a) in rects.iter().enumerate() {
        for b in &rects[i + 1..] {
            let overlap = a.1 < b.1 + b.3 && b.1 < a.1 + a.3 && a.2 < b.2 + b.4 && b.2 < a.2 + a.4;
            if overlap {
                errors.push(format!("los nodos {} y {} se superponen", a.0, b.0));
            }
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const EXAMPLE: &str = r#"{
      "schema": 3,
      "theme": "light",
      "nodes": [
        { "id": "n1", "type": "Banner",
          "props": { "title": "Ofertas de la semana", "subtitle": "", "image_url": "https://example.com/hero.jpg",
                     "button_label": "", "button_href": "" },
          "pos": { "x": 0, "y": 0 }, "size": { "w": 48, "h": 48 } },
        { "id": "n2", "type": "ProductGrid",
          "props": { "title": "Destacados", "limit": 8 },
          "pos": { "x": 0, "y": 48 }, "size": { "w": 48, "h": 60 } }
      ]
    }"#;

    fn example() -> PageDoc {
        serde_json::from_str(EXAMPLE).unwrap()
    }

    fn node(id: &str, type_name: &str, x: u8, y: u16) -> Node {
        let mut n = Node::new(id, Widget::defaults(type_name).unwrap());
        n.pos = Pos { x, y };
        n
    }

    fn assert_invalid(doc: &PageDoc, needle: &str) {
        let errors = validate(doc).expect_err("debía ser inválido");
        assert!(
            errors.iter().any(|e| e.contains(needle)),
            "se esperaba un error con {needle:?}, hubo {errors:?}"
        );
    }

    #[test]
    fn example_round_trips_through_json() {
        let doc = example();
        assert_eq!(doc.nodes.len(), 2);
        assert!(
            matches!(&doc.nodes[0].widget, Widget::Banner(b) if b.title == "Ofertas de la semana")
        );
        let json = serde_json::to_value(&doc).unwrap();
        let original: serde_json::Value = serde_json::from_str(EXAMPLE).unwrap();
        assert_eq!(json, original);
        let back: PageDoc = serde_json::from_value(json).unwrap();
        assert_eq!(back, doc);
        assert_eq!(validate(&doc), Ok(()));
    }

    #[test]
    fn schema_1_docs_are_migrated() {
        let v1 = serde_json::json!({
            "schema": 1,
            "theme": "light",
            "nodes": [
                { "id": "n1", "type": "Banner",
                  "props": { "title": "Hola", "subtitle": "", "image_url": "", "button_label": "", "button_href": "" },
                  "pos": { "x": 0, "y": 0 }, "size": "half" },
                { "id": "n2", "type": "FeaturedProduct",
                  "props": { "product_id": "", "show_price": true },
                  "pos": { "x": 6, "y": 0 }, "size": "large" },
                { "id": "n3", "type": "Text",
                  "props": { "heading": "", "body": "", "align": "left" },
                  "pos": { "x": 0, "y": 5 }, "size": "inexistente" }
            ]
        });
        let doc = parse_doc(v1).unwrap();
        assert_eq!(doc.schema, SCHEMA_VERSION);
        assert_eq!(doc.nodes[0].size, Size { w: 24, h: 48 });
        assert_eq!(doc.nodes[1].size, Size { w: 24, h: 60 });
        assert_eq!(doc.nodes[1].pos, Pos { x: 24, y: 0 });
        assert_eq!(doc.nodes[2].pos, Pos { x: 0, y: 60 });
        assert_eq!(
            doc.nodes[2].size,
            Widget::defaults("Text").unwrap().limits().default
        );
        assert_eq!(validate(&doc), Ok(()));
        // Un documento actual pasa sin cambios.
        assert_eq!(parse_doc(serde_json::to_value(&doc).unwrap()), Ok(doc));
    }

    #[test]
    fn schema_2_docs_are_scaled_to_the_fine_grid() {
        let v2 = serde_json::json!({
            "schema": 2,
            "theme": "light",
            "nodes": [
                { "id": "n1", "type": "Text",
                  "props": { "heading": "", "body": "", "align": "left" },
                  "pos": { "x": 0, "y": 0 }, "size": { "w": 6, "h": 2 } },
                { "id": "n2", "type": "Text",
                  "props": { "heading": "", "body": "", "align": "left" },
                  "pos": { "x": 6, "y": 0 }, "size": { "w": 4, "h": 3 } },
                { "id": "n3", "type": "Text",
                  "props": { "heading": "", "body": "", "align": "left" },
                  "pos": { "x": 0, "y": 3 }, "size": { "w": 12, "h": 1 } }
            ]
        });
        let doc = parse_doc(v2).unwrap();
        assert_eq!(doc.schema, SCHEMA_VERSION);
        assert_eq!(doc.nodes[0].size, Size { w: 24, h: 24 });
        assert_eq!(doc.nodes[1].pos, Pos { x: 24, y: 0 });
        assert_eq!(doc.nodes[1].size, Size { w: 16, h: 36 });
        assert_eq!(doc.nodes[2].pos, Pos { x: 0, y: 36 });
        assert_eq!(doc.nodes[2].size, Size { w: 48, h: 12 });
    }

    #[test]
    fn defaults_are_valid_for_every_widget() {
        for (i, (t, _)) in WIDGET_TYPES.iter().enumerate() {
            let doc = PageDoc {
                nodes: vec![node(&format!("n{i}"), t, 0, 0)],
                ..PageDoc::default()
            };
            assert_eq!(validate(&doc), Ok(()), "{t}");
            let w = Widget::defaults(t).unwrap();
            assert_eq!(w.type_name(), *t);
            let l = w.limits();
            assert!(
                l.min_w <= l.max_w && l.max_w <= GRID_COLS && l.min_w >= 1,
                "{t}"
            );
            assert!(l.min_h <= l.max_h && l.min_h >= 1, "{t}");
        }
        assert_eq!(Widget::defaults("Nada"), None);
    }

    #[test]
    fn clamp_respects_limits() {
        let l = Widget::defaults("Navbar").unwrap().limits();
        assert_eq!(l.clamp(1, 90), Size { w: 24, h: 24 });
        assert_eq!(l.clamp(80, 0), Size { w: 48, h: 12 });
    }

    #[test]
    fn rejects_wrong_schema() {
        let mut doc = example();
        doc.schema = 1;
        assert_invalid(&doc, "schema");
    }

    #[test]
    fn rejects_unknown_theme() {
        let mut doc = example();
        doc.theme = "synthwave".into();
        assert_invalid(&doc, "tema");
    }

    #[test]
    fn rejects_too_many_nodes() {
        let doc = PageDoc {
            nodes: (0..=MAX_NODES as u16)
                .map(|i| node(&format!("n{i}"), "Text", 0, i * 24))
                .collect(),
            ..PageDoc::default()
        };
        assert_invalid(&doc, "máximo");
    }

    #[test]
    fn rejects_empty_id() {
        let mut doc = example();
        doc.nodes[0].id.clear();
        assert_invalid(&doc, "sin id");
    }

    #[test]
    fn rejects_duplicate_id() {
        let mut doc = example();
        doc.nodes[1].id = "n1".into();
        assert_invalid(&doc, "repetido");
    }

    #[test]
    fn rejects_size_outside_limits() {
        let mut doc = example();
        doc.nodes[0].size = Size { w: 8, h: 48 };
        assert_invalid(&doc, "límites");
        doc.nodes[0].size = Size { w: 48, h: 97 };
        assert_invalid(&doc, "límites");
    }

    #[test]
    fn rejects_node_outside_columns() {
        let mut doc = example();
        doc.nodes[0].pos.x = 1;
        assert_invalid(&doc, "columnas");
    }

    #[test]
    fn rejects_overlapping_nodes() {
        let mut doc = example();
        doc.nodes[1].pos.y = 47;
        assert_invalid(&doc, "superponen");
    }

    #[test]
    fn rejects_text_too_long() {
        let mut doc = example();
        if let Widget::Banner(b) = &mut doc.nodes[0].widget {
            b.title = "a".repeat(81);
        }
        assert_invalid(&doc, "80 caracteres");
    }

    #[test]
    fn rejects_number_out_of_range() {
        let mut doc = example();
        if let Widget::ProductGrid(g) = &mut doc.nodes[1].widget {
            g.limit = 13;
        }
        assert_invalid(&doc, "entre 1 y 12");
    }

    #[test]
    fn rejects_unknown_select_option() {
        let mut doc = PageDoc {
            nodes: vec![node("t", "Text", 0, 0)],
            ..PageDoc::default()
        };
        if let Widget::Text(t) = &mut doc.nodes[0].widget {
            t.align = "right".into();
        }
        assert_invalid(&doc, "Alineación: Elegí una de las opciones");
    }

    #[test]
    fn rejects_bad_url() {
        let mut doc = example();
        if let Widget::Banner(b) = &mut doc.nodes[0].widget {
            b.button_href = "javascript:alert(1)".into();
        }
        assert_invalid(&doc, "Banner › Enlace del botón: Escribí una dirección");
    }

    #[test]
    fn accepts_empty_and_relative_urls() {
        let mut doc = example();
        if let Widget::Banner(b) = &mut doc.nodes[0].widget {
            b.image_url = String::new();
            b.button_href = "/productos".into();
        }
        assert_eq!(validate(&doc), Ok(()));
    }

    #[test]
    fn validates_list_items_and_length() {
        let mut doc = PageDoc {
            nodes: vec![node("c", "Carousel", 0, 0)],
            ..PageDoc::default()
        };
        if let Widget::Carousel(c) = &mut doc.nodes[0].widget {
            c.slides[1].button_href = "javascript:alert(1)".into();
        }
        assert_invalid(&doc, "Carrusel › Diapositivas 2 › Enlace del botón");
        if let Widget::Carousel(c) = &mut doc.nodes[0].widget {
            c.slides.clear();
        }
        assert_invalid(&doc, "entre 1 y 8 elementos");
    }

    #[test]
    fn validates_datetime() {
        assert!(datetime_ok("2026-12-31T23:59"));
        assert!(datetime_ok("2026-12-31T23:59:00"));
        assert!(datetime_ok(""));
        assert!(!datetime_ok("31/12/2026"));
        let mut doc = PageDoc {
            nodes: vec![node("o", "Countdown", 0, 0)],
            ..PageDoc::default()
        };
        if let Widget::Countdown(c) = &mut doc.nodes[0].widget {
            c.ends_at = "mañana".into();
        }
        assert_invalid(&doc, "fecha");
    }
}
