//! Tipos del panel y su validación. Sin Leptos y sin IO: compila a nativo y a WASM.

use serde::{Deserialize, Serialize};

pub const SCHEMA_VERSION: u32 = 1;
pub const GRID_COLS: u8 = 12;
pub const MAX_NODES: usize = 100;
pub const THEMES: &[&str] = &["light", "dark", "cupcake", "emerald", "corporate", "retro"];

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
    /// `id` de una opción de [`Widget::sizes`]. El ancho y el alto nunca se guardan.
    pub size: String,
}

impl Node {
    /// La opción de tamaño elegida, si es válida para el widget.
    pub fn preset(&self) -> Option<&'static SizePreset> {
        self.widget.size(&self.size)
    }
}

/// Posición en celdas desde la esquina superior izquierda.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
pub struct Pos {
    pub x: u8,
    pub y: u16,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(tag = "type", content = "props")]
pub enum Widget {
    Banner(BannerProps),
    Text(TextProps),
    ProductGrid(ProductGridProps),
    FeaturedProduct(FeaturedProductProps),
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct BannerProps {
    pub title: String,
    pub subtitle: String,
    pub image_url: String,
    pub button_label: String,
    pub button_href: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct TextProps {
    pub heading: String,
    pub body: String,
    pub align: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct ProductGridProps {
    pub title: String,
    pub limit: i64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct FeaturedProductProps {
    pub product_id: String,
    pub show_price: bool,
}

/// Opción de tamaño de un widget, en columnas × filas de la grilla.
#[derive(Debug, PartialEq)]
pub struct SizePreset {
    pub id: &'static str,
    pub label: &'static str,
    pub w: u8,
    pub h: u8,
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
}

#[derive(Debug, PartialEq)]
pub struct Field {
    pub key: &'static str,
    pub label: &'static str,
    pub kind: FieldKind,
    pub advanced: bool,
}

/// Tipos de widget disponibles, con su nombre para mostrar.
pub const WIDGET_TYPES: &[(&str, &str)] = &[
    ("Banner", "Banner"),
    ("Text", "Texto"),
    ("ProductGrid", "Grilla de productos"),
    ("FeaturedProduct", "Producto destacado"),
];

const BANNER_SIZES: &[SizePreset] = &[
    SizePreset {
        id: "full",
        label: "Ancho completo",
        w: 12,
        h: 4,
    },
    SizePreset {
        id: "half",
        label: "Mitad",
        w: 6,
        h: 4,
    },
];
const TEXT_SIZES: &[SizePreset] = &[
    SizePreset {
        id: "small",
        label: "Chico",
        w: 4,
        h: 2,
    },
    SizePreset {
        id: "medium",
        label: "Mediano",
        w: 6,
        h: 3,
    },
    SizePreset {
        id: "full",
        label: "Ancho completo",
        w: 12,
        h: 2,
    },
];
const PRODUCT_GRID_SIZES: &[SizePreset] = &[
    SizePreset {
        id: "one_row",
        label: "Una fila",
        w: 12,
        h: 5,
    },
    SizePreset {
        id: "two_rows",
        label: "Dos filas",
        w: 12,
        h: 9,
    },
    SizePreset {
        id: "half",
        label: "Mitad",
        w: 6,
        h: 9,
    },
];
const FEATURED_PRODUCT_SIZES: &[SizePreset] = &[
    SizePreset {
        id: "small",
        label: "Chico",
        w: 3,
        h: 4,
    },
    SizePreset {
        id: "medium",
        label: "Mediano",
        w: 4,
        h: 5,
    },
    SizePreset {
        id: "large",
        label: "Grande",
        w: 6,
        h: 5,
    },
];

const BANNER_FIELDS: &[Field] = &[
    Field {
        key: "title",
        label: "Título",
        kind: FieldKind::Text { max: 80 },
        advanced: false,
    },
    Field {
        key: "subtitle",
        label: "Subtítulo",
        kind: FieldKind::Text { max: 160 },
        advanced: false,
    },
    Field {
        key: "image_url",
        label: "URL de la imagen",
        kind: FieldKind::Url,
        advanced: false,
    },
    Field {
        key: "button_label",
        label: "Texto del botón",
        kind: FieldKind::Text { max: 30 },
        advanced: true,
    },
    Field {
        key: "button_href",
        label: "Enlace del botón",
        kind: FieldKind::Url,
        advanced: true,
    },
];
const TEXT_ALIGN_OPTIONS: &[(&str, &str)] = &[("left", "Izquierda"), ("center", "Centro")];
const TEXT_FIELDS: &[Field] = &[
    Field {
        key: "heading",
        label: "Título",
        kind: FieldKind::Text { max: 80 },
        advanced: false,
    },
    Field {
        key: "body",
        label: "Texto",
        kind: FieldKind::LongText { max: 1000 },
        advanced: false,
    },
    Field {
        key: "align",
        label: "Alineación",
        kind: FieldKind::Select {
            options: TEXT_ALIGN_OPTIONS,
        },
        advanced: true,
    },
];
const PRODUCT_GRID_FIELDS: &[Field] = &[
    Field {
        key: "title",
        label: "Título",
        kind: FieldKind::Text { max: 80 },
        advanced: false,
    },
    Field {
        key: "limit",
        label: "Cantidad de productos",
        kind: FieldKind::Number { min: 1, max: 12 },
        advanced: false,
    },
];
const FEATURED_PRODUCT_FIELDS: &[Field] = &[
    Field {
        key: "product_id",
        label: "Producto",
        kind: FieldKind::ProductRef,
        advanced: false,
    },
    Field {
        key: "show_price",
        label: "Mostrar precio",
        kind: FieldKind::Bool,
        advanced: true,
    },
];

impl Widget {
    /// Nombre del tipo tal como se guarda en `type`.
    pub fn type_name(&self) -> &'static str {
        match self {
            Widget::Banner(_) => "Banner",
            Widget::Text(_) => "Text",
            Widget::ProductGrid(_) => "ProductGrid",
            Widget::FeaturedProduct(_) => "FeaturedProduct",
        }
    }

    pub fn sizes(&self) -> &'static [SizePreset] {
        match self {
            Widget::Banner(_) => BANNER_SIZES,
            Widget::Text(_) => TEXT_SIZES,
            Widget::ProductGrid(_) => PRODUCT_GRID_SIZES,
            Widget::FeaturedProduct(_) => FEATURED_PRODUCT_SIZES,
        }
    }

    pub fn size(&self, id: &str) -> Option<&'static SizePreset> {
        self.sizes().iter().find(|s| s.id == id)
    }

    pub fn fields(&self) -> &'static [Field] {
        match self {
            Widget::Banner(_) => BANNER_FIELDS,
            Widget::Text(_) => TEXT_FIELDS,
            Widget::ProductGrid(_) => PRODUCT_GRID_FIELDS,
            Widget::FeaturedProduct(_) => FEATURED_PRODUCT_FIELDS,
        }
    }

    /// Widget nuevo con contenido de ejemplo, para que se vea bien sin configurar nada.
    pub fn defaults(type_name: &str) -> Option<Widget> {
        let widget = match type_name {
            "Banner" => Widget::Banner(BannerProps {
                title: "Bienvenidos a nuestra tienda".into(),
                subtitle: "Encontrá lo que buscás al mejor precio".into(),
                image_url: "https://picsum.photos/seed/marketmaker-banner/1600/600".into(),
                button_label: String::new(),
                button_href: String::new(),
            }),
            "Text" => Widget::Text(TextProps {
                heading: "Sobre nosotros".into(),
                body: "Somos una tienda local con envíos a todo el país.".into(),
                align: "left".into(),
            }),
            "ProductGrid" => Widget::ProductGrid(ProductGridProps {
                title: "Productos".into(),
                limit: 4,
            }),
            "FeaturedProduct" => Widget::FeaturedProduct(FeaturedProductProps {
                product_id: String::new(),
                show_price: true,
            }),
            _ => return None,
        };
        Some(widget)
    }
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

fn validate_fields(node: &Node, errors: &mut Vec<String>) {
    let value = match serde_json::to_value(&node.widget) {
        Ok(v) => v,
        Err(e) => {
            errors.push(format!("nodo {}: no se pudo leer: {e}", node.id));
            return;
        }
    };
    let props = value.get("props");
    for field in node.widget.fields() {
        let v = props.and_then(|p| p.get(field.key));
        let err = |msg: &str| format!("nodo {}: campo {}: {msg}", node.id, field.key);
        match &field.kind {
            FieldKind::Text { max } | FieldKind::LongText { max } => {
                match v.and_then(|v| v.as_str()) {
                    Some(s) if s.chars().count() <= *max => {}
                    Some(_) => errors.push(err(&format!("supera {max} caracteres"))),
                    None => errors.push(err("debe ser texto")),
                }
            }
            FieldKind::Url => match v.and_then(|v| v.as_str()) {
                Some(s) if url_ok(s) => {}
                Some(_) => errors.push(err("la URL debe empezar con https://, http:// o /")),
                None => errors.push(err("debe ser texto")),
            },
            FieldKind::Number { min, max } => match v.and_then(|v| v.as_i64()) {
                Some(n) if (*min..=*max).contains(&n) => {}
                Some(_) => errors.push(err(&format!("debe estar entre {min} y {max}"))),
                None => errors.push(err("debe ser un número entero")),
            },
            FieldKind::Bool => {
                if !v.is_some_and(|v| v.is_boolean()) {
                    errors.push(err("debe ser sí o no"));
                }
            }
            FieldKind::Select { options } => match v.and_then(|v| v.as_str()) {
                Some(s) if options.iter().any(|(k, _)| *k == s) => {}
                _ => errors.push(err("opción inexistente")),
            },
            FieldKind::ProductRef => {
                if !v.is_some_and(|v| v.is_string()) {
                    errors.push(err("debe ser un id de producto"));
                }
            }
        }
    }
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
    // (id, x, y, w, h) de los nodos con tamaño válido, para chequear superposiciones.
    let mut rects: Vec<(&str, u32, u32, u32, u32)> = Vec::new();
    for node in &doc.nodes {
        if node.id.is_empty() {
            errors.push("hay un nodo sin id".to_string());
        } else if !ids.insert(node.id.as_str()) {
            errors.push(format!("id repetido: {}", node.id));
        }

        match node.preset() {
            None => errors.push(format!(
                "nodo {}: tamaño {} no válido para {}",
                node.id,
                node.size,
                node.widget.type_name()
            )),
            Some(p) => {
                if u32::from(node.pos.x) + u32::from(p.w) > u32::from(GRID_COLS) {
                    errors.push(format!(
                        "nodo {}: se sale de las {GRID_COLS} columnas",
                        node.id
                    ));
                }
                rects.push((
                    node.id.as_str(),
                    u32::from(node.pos.x),
                    u32::from(node.pos.y),
                    u32::from(p.w),
                    u32::from(p.h),
                ));
            }
        }

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
      "schema": 1,
      "theme": "light",
      "nodes": [
        { "id": "n1", "type": "Banner",
          "props": { "title": "Ofertas de la semana", "subtitle": "", "image_url": "https://example.com/hero.jpg",
                     "button_label": "", "button_href": "" },
          "pos": { "x": 0, "y": 0 }, "size": "full" },
        { "id": "n2", "type": "ProductGrid",
          "props": { "title": "Destacados", "limit": 8 },
          "pos": { "x": 0, "y": 4 }, "size": "one_row" }
      ]
    }"#;

    fn example() -> PageDoc {
        serde_json::from_str(EXAMPLE).unwrap()
    }

    fn node(id: &str, type_name: &str, x: u8, y: u16) -> Node {
        let widget = Widget::defaults(type_name).unwrap();
        let size = widget.sizes()[0].id.to_string();
        Node {
            id: id.into(),
            widget,
            pos: Pos { x, y },
            size,
        }
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
    fn defaults_are_valid_for_every_widget() {
        for (i, (t, _)) in WIDGET_TYPES.iter().enumerate() {
            let doc = PageDoc {
                nodes: vec![node(&format!("n{i}"), t, 0, 0)],
                ..PageDoc::default()
            };
            assert_eq!(validate(&doc), Ok(()), "{t}");
        }
        assert_eq!(Widget::defaults("Nada"), None);
    }

    #[test]
    fn rejects_wrong_schema() {
        let mut doc = example();
        doc.schema = 2;
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
                .map(|i| node(&format!("n{i}"), "Text", 0, i * 2))
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
    fn rejects_invalid_size() {
        let mut doc = example();
        doc.nodes[0].size = "small".into();
        assert_invalid(&doc, "tamaño");
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
        doc.nodes[1].pos.y = 3;
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
        assert_invalid(&doc, "opción");
    }

    #[test]
    fn rejects_bad_url() {
        let mut doc = example();
        if let Widget::Banner(b) = &mut doc.nodes[0].widget {
            b.button_href = "javascript:alert(1)".into();
        }
        assert_invalid(&doc, "URL");
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
}
