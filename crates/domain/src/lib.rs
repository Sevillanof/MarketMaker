//! Tipos del panel y su validación. Sin Leptos y sin IO: compila a nativo y a WASM.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

mod account;
mod buyer;
mod catalog;
mod coupons;
mod import;
mod layout;
mod look;
mod media;
mod orders;
mod shipping;
mod site;
mod starter;
mod templates;
mod theme;
mod typography;
mod variants;
mod widgets;
pub mod wizard;

pub use account::*;
pub use buyer::*;
pub use catalog::*;
pub use coupons::*;
pub use import::*;
pub use layout::*;
pub use look::*;
pub use media::*;
pub use orders::*;
pub use shipping::*;
pub use site::*;
pub use starter::*;
pub use templates::*;
pub use theme::*;
pub use typography::*;
pub use variants::*;
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
/// Máximo de bloques en la ficha del producto.
pub const MAX_MODAL_NODES: usize = 30;
/// Máximo de páginas además de Inicio.
pub const MAX_PAGES: usize = 30;
/// Largo máximo de la dirección de una página (`/ofertas`).
pub const MAX_PATH_LEN: usize = 60;
/// Largo máximo del nombre de una página.
pub const MAX_PAGE_NAME: usize = 40;
/// Direcciones que usa la aplicación: ninguna página puede empezar así.
pub const RESERVED_PATHS: &[&str] = &[
    "/editor", "/entrar", "/dev", "/pkg", "/api", "/fonts", "/img",
];
/// Bloques que se copian a una página nueva para que se vea como parte de
/// la misma tienda: el menú y el pie.
pub const SHARED_ON_NEW_PAGE: &[&str] = &["Navbar", "Footer"];
/// Ancho, en píxeles, con el que se dibuja el contenido de la ficha del
/// producto (el `max-w` del cuadro menos su relleno).
pub const MODAL_W_PX: u16 = 800;
/// Todos los temas que se pueden leer: los 12 de la maqueta (los únicos que se
/// ofrecen) y los seis anteriores, que siguen funcionando en tiendas que ya
/// los usan hasta que su dueño elija uno nuevo.
pub const THEMES: &[&str] = &[
    "linea",
    "pulso",
    "noche",
    "brisa",
    "tinta",
    "citrico",
    "aura",
    "brote",
    "orbita",
    "dalia",
    "cacao",
    "sol",
    "light",
    "dark",
    "cupcake",
    "emerald",
    "corporate",
    "retro",
];

/// Cuántos de [`THEMES`] se ofrecen para elegir (los primeros).
pub const OFFERED_THEMES: usize = 12;

/// Nombre para mostrar de cada tema, en el mismo orden que [`THEMES`].
pub const THEME_LABELS: &[&str] = &[
    "Línea",
    "Pulso",
    "Noche",
    "Brisa",
    "Tinta",
    "Cítrico",
    "Aura",
    "Brote",
    "Órbita",
    "Dalia",
    "Cacao",
    "Sol",
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
    /// Letras de toda la tienda. Los documentos anteriores no la traen: se
    /// usa la de siempre.
    #[serde(default, skip_serializing_if = "Typography::is_default")]
    pub typography: Typography,
    /// Márgenes y ancho de toda la tienda. Los documentos anteriores no lo
    /// traen: se usa el de siempre.
    #[serde(default, skip_serializing_if = "StoreLayout::is_default")]
    pub layout: StoreLayout,
    /// Forma de los botones, esquinas y fondo de toda la tienda.
    #[serde(default, skip_serializing_if = "StoreStyle::is_default")]
    pub style: StoreStyle,
    pub nodes: Vec<Node>,
    /// Ficha que se abre al tocar un producto (en la cuadrícula o en el
    /// producto destacado). Es una sola para toda la tienda. Los documentos
    /// anteriores no la traen: se usa la de [`ProductModal::default`].
    #[serde(default, skip_serializing_if = "ProductModal::is_default")]
    pub product_modal: ProductModal,
    /// Las demás páginas de la tienda (Ofertas, Categorías…). Inicio son
    /// los `nodes` de arriba. Los documentos anteriores no las traen.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub pages: Vec<StorePage>,
}

/// Una página de la tienda que no es Inicio: su dirección, su nombre y sus
/// bloques, sobre la misma grilla que Inicio.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct StorePage {
    /// Dirección dentro de la tienda, como `/ofertas` (ver [`normalize_path`]).
    pub path: String,
    pub name: String,
    pub nodes: Vec<Node>,
}

/// Lleva el enlace `href` a la dirección de una página de la tienda:
/// `"/Ofertas/?x=1#y"` → `"/ofertas"`, `""` o `"/"` → `"/"`.
///
/// `None` si no es una dirección de la tienda (`https://…`, `mailto:…`,
/// `//otro.sitio`) o si tiene caracteres que no van en una dirección.
pub fn normalize_path(href: &str) -> Option<String> {
    let href = href.trim();
    if href.is_empty() {
        return Some("/".to_string());
    }
    if !href.starts_with('/') || href.starts_with("//") {
        return None;
    }
    let end = href.find(['?', '#']).unwrap_or(href.len());
    let path = href[..end].trim_end_matches('/').to_lowercase();
    if path.is_empty() {
        return Some("/".to_string());
    }
    let ok = path.len() <= MAX_PATH_LEN
        && path[1..].split('/').all(|seg| {
            !seg.is_empty()
                && seg
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_')
        });
    ok.then_some(path)
}

/// ¿La dirección la usa la aplicación (el editor, los archivos)?
pub fn is_reserved_path(path: &str) -> bool {
    RESERVED_PATHS.iter().any(|r| {
        path == *r
            || path
                .strip_prefix(r)
                .is_some_and(|rest| rest.starts_with('/'))
    })
}

/// Nombre para una página nueva a partir de su dirección: `/sobre-nosotros`
/// → `Sobre nosotros`.
pub fn name_from_path(path: &str) -> String {
    let last = path.rsplit('/').find(|s| !s.is_empty()).unwrap_or("Página");
    let words = last.replace(['-', '_'], " ");
    let mut chars = words.chars();
    match chars.next() {
        Some(c) => c.to_uppercase().chain(chars).collect(),
        None => "Página".to_string(),
    }
}

impl Default for PageDoc {
    fn default() -> Self {
        Self {
            schema: SCHEMA_VERSION,
            theme: THEMES[0].to_string(),
            typography: Typography::default(),
            layout: StoreLayout::fresh(),
            style: StoreStyle::default(),
            nodes: Vec::new(),
            product_modal: ProductModal::default(),
            pages: Vec::new(),
        }
    }
}

/// Dónde están los bloques que se editan: en la página o en la ficha del
/// producto.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Layer {
    /// Una página de la tienda: `0` es Inicio y `i` es `pages[i - 1]`.
    Page(usize),
    ProductModal,
}

/// Inicio.
pub const HOME: Layer = Layer::Page(0);

static NO_NODES: Vec<Node> = Vec::new();

impl Layer {
    /// Ancho, en píxeles, al que se dibuja el contenido de la capa.
    pub fn design_width(self) -> u16 {
        match self {
            Layer::Page(_) => 1120,
            Layer::ProductModal => MODAL_W_PX,
        }
    }
}

impl PageDoc {
    /// Bloques de una capa. Una página que no existe no tiene bloques.
    pub fn layer(&self, layer: Layer) -> &Vec<Node> {
        match layer {
            Layer::Page(0) => &self.nodes,
            Layer::Page(i) => self.pages.get(i - 1).map_or(&NO_NODES, |p| &p.nodes),
            Layer::ProductModal => &self.product_modal.nodes,
        }
    }

    /// Bloques de una capa, para cambiarlos. Una página que no existe se
    /// toma como Inicio.
    pub fn layer_mut(&mut self, layer: Layer) -> &mut Vec<Node> {
        match layer {
            Layer::Page(i) if i > 0 && i <= self.pages.len() => &mut self.pages[i - 1].nodes,
            Layer::Page(_) => &mut self.nodes,
            Layer::ProductModal => &mut self.product_modal.nodes,
        }
    }

    /// Cuántas páginas hay, Inicio incluida.
    pub fn page_count(&self) -> usize {
        self.pages.len() + 1
    }

    /// Dirección de la página `i` (`/` para Inicio).
    pub fn page_path(&self, i: usize) -> &str {
        match i {
            0 => "/",
            i => self.pages.get(i - 1).map_or("/", |p| p.path.as_str()),
        }
    }

    /// Nombre de la página `i`.
    pub fn page_name(&self, i: usize) -> &str {
        match i {
            0 => "Inicio",
            i => self.pages.get(i - 1).map_or("Inicio", |p| p.name.as_str()),
        }
    }

    /// Página a la que lleva el enlace `href`, si existe.
    pub fn page_index(&self, href: &str) -> Option<usize> {
        let path = normalize_path(href)?;
        if path == "/" {
            return Some(0);
        }
        self.pages
            .iter()
            .position(|p| p.path == path)
            .map(|i| i + 1)
    }

    /// Agrega la página `path` con ese nombre y la devuelve. Arranca con una
    /// copia del menú y del pie de la página `from` (ver
    /// [`SHARED_ON_NEW_PAGE`]), uno debajo del otro en el orden de `from`:
    /// el menú arriba y el pie justo debajo.
    pub fn add_page(&mut self, path: String, name: String, from: usize) -> usize {
        self.add_page_with(path, name, from, Vec::new())
    }

    /// Como [`PageDoc::add_page`], con los bloques `body` (de todo el ancho)
    /// entre el menú y el pie.
    pub fn add_page_with(
        &mut self,
        path: String,
        name: String,
        from: usize,
        body: Vec<Widget>,
    ) -> usize {
        let mut nodes: Vec<Node> = Vec::new();
        // Los bloques de origen quedan compartidos (si no lo estaban) y las
        // copias entran en el mismo grupo.
        for n in self.layer_mut(Layer::Page(from)) {
            if SHARED_ON_NEW_PAGE.contains(&n.widget.type_name()) && n.shared.is_empty() {
                n.shared = n.id.clone();
            }
        }
        let mut shared: Vec<Node> = self
            .layer(Layer::Page(from))
            .iter()
            .filter(|n| SHARED_ON_NEW_PAGE.contains(&n.widget.type_name()))
            .cloned()
            .collect();
        shared.sort_by_key(|n| (n.pos.y, n.pos.x));
        // El cuerpo va después del menú y antes de todo lo demás (el pie).
        let at = shared
            .iter()
            .take_while(|n| n.widget.type_name() == "Navbar")
            .count();
        let body: Vec<Node> = body
            .into_iter()
            .map(|w| {
                let mut n = Node::new(String::new(), w);
                n.size.w = GRID_COLS;
                n.pos.x = 0;
                n
            })
            .collect();
        let tail = shared.split_off(at);
        let mut y = 0_u16;
        for mut n in shared.into_iter().chain(body).chain(tail) {
            n.pos.y = y;
            y += u16::from(n.size.h);
            n.id = self.fresh_id(&nodes);
            nodes.push(n);
        }
        self.pages.push(StorePage { path, name, nodes });
        self.pages.len()
    }

    /// En cuántas páginas está el grupo compartido `group`.
    pub fn shared_pages(&self, group: &str) -> usize {
        if group.is_empty() {
            return 0;
        }
        (0..self.page_count())
            .filter(|&i| self.layer(Layer::Page(i)).iter().any(|n| n.shared == group))
            .count()
    }

    /// Une el menú y el pie de las páginas que se crearon antes de que se
    /// compartieran: cada bloque de [`SHARED_ON_NEW_PAGE`] de otra página
    /// que es igual (mismo contenido y letras) a uno de Inicio entra en su
    /// grupo. Solo actúa si el documento todavía no tiene ningún grupo.
    pub fn link_shared(&mut self) {
        if self.pages.is_empty() || self.all_nodes().any(|n| !n.shared.is_empty()) {
            return;
        }
        let home: Vec<(String, Widget, TextStyle)> = self
            .nodes
            .iter()
            .filter(|n| SHARED_ON_NEW_PAGE.contains(&n.widget.type_name()))
            .map(|n| (n.id.clone(), n.widget.clone(), n.style.clone()))
            .collect();
        let mut used = std::collections::HashSet::new();
        for page in &mut self.pages {
            for n in &mut page.nodes {
                if let Some((id, _, _)) = home
                    .iter()
                    .find(|(_, w, st)| *w == n.widget && *st == n.style)
                {
                    n.shared = id.clone();
                    used.insert(id.clone());
                }
            }
        }
        for n in &mut self.nodes {
            if used.contains(&n.id) {
                n.shared = n.id.clone();
            }
        }
    }

    /// Un id de bloque que no usa nadie (ni `extra`, que todavía no está en
    /// el documento).
    pub fn fresh_id(&self, extra: &[Node]) -> String {
        (1..)
            .map(|k| format!("n{k}"))
            .find(|id| self.all_nodes().chain(extra).all(|n| &n.id != id))
            .expect("siempre hay un id libre")
    }

    /// Todos los bloques, de todas las páginas y de la ficha. Los ids no se
    /// repiten.
    pub fn all_nodes(&self) -> impl Iterator<Item = &Node> {
        self.nodes
            .iter()
            .chain(self.pages.iter().flat_map(|p| p.nodes.iter()))
            .chain(self.product_modal.nodes.iter())
    }

    /// Bloque con ese id, esté donde esté.
    pub fn node(&self, id: &str) -> Option<&Node> {
        self.all_nodes().find(|n| n.id == id)
    }

    pub fn node_mut(&mut self, id: &str) -> Option<&mut Node> {
        self.nodes
            .iter_mut()
            .chain(self.pages.iter_mut().flat_map(|p| p.nodes.iter_mut()))
            .chain(self.product_modal.nodes.iter_mut())
            .find(|n| n.id == id)
    }

    /// Capa donde está el bloque `id`.
    pub fn layer_of(&self, id: &str) -> Option<Layer> {
        if self.nodes.iter().any(|n| n.id == id) {
            return Some(HOME);
        }
        if let Some(i) = self
            .pages
            .iter()
            .position(|p| p.nodes.iter().any(|n| n.id == id))
        {
            return Some(Layer::Page(i + 1));
        }
        self.product_modal
            .nodes
            .iter()
            .any(|n| n.id == id)
            .then_some(Layer::ProductModal)
    }
}

/// Diseño de la ficha del producto: bloques sobre la misma grilla de
/// [`GRID_COLS`] columnas, dibujada a [`MODAL_W_PX`] de ancho.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct ProductModal {
    pub nodes: Vec<Node>,
    /// Cómo se abre: `""` ventana al centro, `drawer` cajón a la derecha,
    /// `sheet` hoja desde abajo, `auto` cajón en compu y hoja en celular.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub open: String,
    /// Barra de compra que aparece al bajar: `""` sin barra, `light`
    /// flotante clara, `dark` flotante oscura, `color` pegada abajo con el
    /// color de la tienda.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub buybar: String,
    /// Dónde va el botón Compartir: `""` arriba, junto a cerrar; `photo`
    /// sobre la foto, debajo del corazón de favoritos.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub share: String,
}

/// Los lugares del botón Compartir de la ficha.
pub const MODAL_SHARE_OPTIONS: &[(&str, &str)] = &[
    ("", "Arriba, junto a cerrar"),
    ("photo", "Sobre la foto, debajo del corazón"),
];

/// Las formas de abrir la ficha.
pub const MODAL_OPEN_OPTIONS: &[(&str, &str)] = &[
    ("", "Ventana al centro"),
    ("drawer", "Cajón a la derecha"),
    ("sheet", "Hoja que sube desde abajo"),
    ("auto", "Cajón en computadora, hoja en celular"),
];

/// Los estilos de la barra de compra al bajar.
pub const MODAL_BUYBAR_OPTIONS: &[(&str, &str)] = &[
    ("", "Sin barra"),
    ("light", "Flotante clara"),
    ("dark", "Flotante oscura"),
    ("color", "Pegada abajo, color de la tienda"),
];

impl ProductModal {
    pub fn is_default(&self) -> bool {
        *self == Self::default()
    }
}

impl Default for ProductModal {
    /// La foto a la izquierda, algo más de la mitad; nombre, precio,
    /// descripción y botón a la derecha, uno debajo del otro sin huecos.
    fn default() -> Self {
        let at = |id: &str, t: &str, x: u8, y: u16, w: u8, h: u8| {
            let mut n = Node::new(id, Widget::defaults(t).expect("tipo conocido"));
            n.pos = Pos { x, y };
            n.size = Size { w, h };
            n
        };
        Self {
            nodes: vec![
                at("m1", "ProductImage", 0, 0, 27, 48),
                at("m2", "ProductName", 27, 0, 21, 6),
                at("m3", "ProductPrice", 27, 6, 21, 6),
                at("m4", "ProductDescription", 27, 12, 21, 29),
                at("m5", "ProductButton", 27, 41, 21, 7),
            ],
            open: String::new(),
            buybar: String::new(),
            share: String::new(),
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
    /// Letras propias del bloque; lo vacío se toma de la tienda.
    #[serde(default, skip_serializing_if = "TextStyle::is_default")]
    pub style: TextStyle,
    /// Grupo de bloques compartidos entre páginas (el menú y el pie): los
    /// bloques con el mismo grupo muestran lo mismo en todas las páginas, y
    /// cambiar uno cambia todos (ver [`sync_shared`]). Vacío: no se comparte.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub shared: String,
    /// Diseño (uno de cinco) y color del bloque.
    #[serde(default, skip_serializing_if = "Look::is_default")]
    pub look: Look,
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
            style: TextStyle::default(),
            shared: String::new(),
            look: Look::default(),
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
    /// Medida con deslizador: número entero entre `min` y `max`, con su
    /// unidad escrita al lado (`"px"`, `"%"`).
    Range {
        min: i64,
        max: i64,
        unit: &'static str,
    },
    /// Color propio `#rrggbb`. Vacío: el del tema de la tienda.
    Color,
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
    let mut doc: PageDoc = serde_json::from_value(value).map_err(|e| e.to_string())?;
    doc.link_shared();
    Ok(doc)
}

/// Después de un cambio (`before` → `after`): si cambió el contenido o las
/// letras de un bloque compartido, se copian a los demás bloques de su grupo,
/// en todas las páginas. La posición y el tamaño son de cada página.
pub fn sync_shared(before: &PageDoc, after: &mut PageDoc) {
    let changed: Vec<(String, Widget, TextStyle, Look)> = after
        .all_nodes()
        .filter(|n| !n.shared.is_empty())
        .filter(|n| {
            before
                .node(&n.id)
                .is_some_and(|b| b.widget != n.widget || b.style != n.style || b.look != n.look)
        })
        .map(|n| {
            (
                n.shared.clone(),
                n.widget.clone(),
                n.style.clone(),
                n.look.clone(),
            )
        })
        .collect();
    let mut seen = std::collections::HashSet::new();
    for (group, widget, style, look) in changed {
        if !seen.insert(group.clone()) {
            continue;
        }
        let pages = after
            .nodes
            .iter_mut()
            .chain(after.pages.iter_mut().flat_map(|p| p.nodes.iter_mut()));
        for n in pages {
            if n.shared == group && n.widget.type_name() == widget.type_name() {
                n.widget = widget.clone();
                n.style = style.clone();
                n.look = look.clone();
            }
        }
    }
}

/// Lo que necesita el marketplace (y el editor) para mostrar una página.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct PageView {
    pub doc: PageDoc,
    /// Versión de `doc`: en el editor, la del borrador; en la tienda, la publicada.
    pub version: u32,
    /// Versión que ven los visitantes (0 si nunca se publicó).
    #[serde(default)]
    pub published: u32,
    pub products: Vec<Product>,
    /// Nombre de la tienda, para el título de la pestaña y al compartir.
    #[serde(default)]
    pub store_name: String,
    /// Nombre, logo y contacto: el menú y el pie los usan si sus campos
    /// quedan vacíos.
    #[serde(default)]
    pub store: StoreInfo,
    /// Tienda de pruebas (usa la aplicación de pruebas de Mercado Pago): se
    /// muestra una etiqueta aclarando que no es una tienda real.
    #[serde(default)]
    pub test_store: bool,
    /// Los cupones que sirven hoy (para el bloque Cupón).
    #[serde(default)]
    pub coupons: Vec<CouponBrief>,
    /// La pausó su dueño: los compradores ven "Volvemos pronto".
    #[serde(default)]
    pub paused: bool,
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

/// Producto de la tienda, tal como lo usan los widgets y la pantalla
/// Productos.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Product {
    pub id: String,
    pub name: String,
    pub price_cents: i64,
    /// La foto principal (la primera de `photos`, o vacía).
    pub image_url: String,
    #[serde(default)]
    pub description: String,
    /// Precio anterior, que se muestra tachado. 0 si no hay oferta.
    #[serde(default)]
    pub compare_at_cents: i64,
    #[serde(default = "yes")]
    pub in_stock: bool,
    /// Unidades que quedan, si la tienda las cuenta (`None`: no las cuenta).
    /// Con 0, `in_stock` es `false`.
    #[serde(default)]
    pub stock: Option<u32>,
    /// Si aparece en la tienda.
    #[serde(default = "yes")]
    pub visible: bool,
    /// Todas las fotos, la principal primero.
    #[serde(default)]
    pub photos: Vec<String>,
    /// Etiquetas para encontrar y agrupar productos.
    #[serde(default)]
    pub tags: Vec<String>,
    /// Categoría del producto (una sola; vacío: sin categoría).
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub category: String,
    /// Subcategoría dentro de `category` (vacío: ninguna).
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub subcategory: String,
    /// Cómo se encuadra cada foto en este producto (por dirección de la
    /// foto). Las que no están se ven centradas y sin zoom.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub photo_fits: BTreeMap<String, PhotoFit>,
    /// De ejemplo: lo trae la tienda nueva para que el diseño no se vea vacío.
    /// Los compradores no lo ven. Deja de serlo cuando el dueño lo guarda.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub example: bool,
    /// Peso y medidas para cotizar el envío. `None`: el paquete de la tienda.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parcel: Option<Parcel>,
    /// Colores y talles con su stock (ver [`Variants`]). Vacío: el producto
    /// se vende como siempre.
    #[serde(default, skip_serializing_if = "Variants::is_empty")]
    pub variants: Variants,
    /// El dueño confirmó que se regala: solo así se publica a $ 0.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub free: bool,
}

fn yes() -> bool {
    true
}

impl Default for Product {
    fn default() -> Self {
        Self {
            id: String::new(),
            name: String::new(),
            price_cents: 0,
            image_url: String::new(),
            description: String::new(),
            compare_at_cents: 0,
            in_stock: true,
            stock: None,
            visible: true,
            photos: Vec::new(),
            tags: Vec::new(),
            category: String::new(),
            subcategory: String::new(),
            photo_fits: BTreeMap::new(),
            example: false,
            parcel: None,
            variants: Variants::default(),
            free: false,
        }
    }
}

/// Con tan pocas unidades, la tienda muestra "Quedan N".
pub const FEW_LEFT: u32 = 3;

impl Product {
    /// "Quedan 2" cuando quedan pocas (y la tienda las cuenta).
    pub fn few_left(&self) -> Option<String> {
        match self.stock {
            Some(1) => Some("Queda 1".to_string()),
            Some(n) if n > 0 && n <= FEW_LEFT => Some(format!("Quedan {n}")),
            _ => None,
        }
    }

    /// Las fotos para mostrar: `photos`, o la principal si no hay lista.
    /// El encuadre de la foto `url` en este producto, si se ajustó.
    pub fn fit(&self, url: &str) -> Option<PhotoFit> {
        self.photo_fits.get(url).copied()
    }

    pub fn all_photos(&self) -> Vec<String> {
        if self.photos.is_empty() && !self.image_url.is_empty() {
            vec![self.image_url.clone()]
        } else {
            self.photos.clone()
        }
    }

    /// Descuento redondeado hacia abajo (`12` para -12 %), si hay oferta.
    pub fn discount_pct(&self) -> Option<i64> {
        (self.compare_at_cents > self.price_cents && self.price_cents >= 0)
            .then(|| (self.compare_at_cents - self.price_cents) * 100 / self.compare_at_cents)
            .filter(|p| *p > 0)
    }
}

fn url_ok(s: &str) -> bool {
    s.is_empty()
        || s.starts_with("https://")
        || s.starts_with("http://")
        || s.starts_with('/')
        || s.starts_with('#')
        || s.starts_with("mailto:")
        || s.starts_with("tel:")
}

/// Completa un enlace escrito a mano: `www.x.com` o `x.com/a` → `https://…`,
/// `hola@x.com` → `mailto:…`, `ofertas` → `/ofertas`. Lo que ya es válido
/// o no se entiende queda igual (y lo marca [`field_error`]).
pub fn tidy_link(s: &str) -> String {
    let s = s.trim();
    if s.is_empty() || url_ok(s) || s.contains(char::is_whitespace) {
        return s.to_string();
    }
    let host = s.split(['/', '?', '#']).next().unwrap_or_default();
    if s.contains('@') && !s.contains('/') && !s.contains(':') {
        return format!("mailto:{s}");
    }
    if host.contains('.') && !host.contains(':') {
        return format!("https://{s}");
    }
    if !s.contains(':') {
        return format!("/{s}");
    }
    s.to_string()
}

/// Enlace que abre la ficha del producto `id` en el Inicio de la tienda.
pub fn product_href(id: &str) -> String {
    format!("/?ver={id}")
}

/// Enlace al Inicio de la tienda con la categoría `cat` ya elegida en los
/// productos (`/?categoria=Ropa%20de%20cama`).
pub fn category_href(cat: &str) -> String {
    format!("/?categoria={}", encode_query(cat.trim()))
}

/// Enlace al Inicio de la tienda con la etiqueta `tag` ya elegida en los
/// productos (`/?etiqueta=hogar`).
pub fn tag_href(tag: &str) -> String {
    format!("/?etiqueta={}", encode_query(tag.trim()))
}

/// Adónde lleva un cuadro de «Categorías». Las plantillas viejas venían con
/// enlaces a `/categorias/<nombre>`, páginas que nadie creó: si esa página no
/// existe (`has_page`), el cuadro muestra los productos de la categoría con
/// ese nombre o, si la tienda no tiene esa categoría, los de la etiqueta.
pub fn category_tile_href(
    href: &str,
    label: &str,
    has_page: bool,
    categories: &[String],
) -> String {
    if has_page || !href.trim().starts_with("/categorias/") {
        return href.to_string();
    }
    let label = label.trim();
    let wanted = label.to_lowercase();
    match categories.iter().find(|c| c.to_lowercase() == wanted) {
        Some(c) => category_href(c),
        None => tag_href(&label.to_lowercase()),
    }
}

pub fn encode_query(text: &str) -> String {
    let mut out = String::new();
    for b in text.bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b'~') {
            out.push(char::from(b));
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

/// A qué lleva un enlace, para el selector del editor.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LinkKind {
    /// Una página de la tienda (`/`, `/ofertas`).
    Page,
    /// La ficha de un producto ([`product_href`]).
    Product,
    /// Una categoría ([`category_href`]).
    Category,
    /// Otro sitio, un email, una parte de la página…
    Other,
}

impl LinkKind {
    pub fn of(href: &str) -> Self {
        let h = href.trim();
        if h.starts_with("/?ver=") {
            LinkKind::Product
        } else if h.starts_with("/?categoria=") {
            LinkKind::Category
        } else if h.is_empty()
            || (h.starts_with('/') && !h.starts_with("//") && !h.contains(['?', '#']))
        {
            LinkKind::Page
        } else {
            LinkKind::Other
        }
    }
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
            Some(_) => Some(
                "Escribí una página de tu tienda (/ofertas), una dirección (https://…) o un email."
                    .to_string(),
            ),
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
        FieldKind::Range { min, max, .. } => match v.and_then(|v| v.as_i64()) {
            Some(n) if (*min..=*max).contains(&n) => None,
            _ => Some(format!("Elegí un valor entre {min} y {max}.")),
        },
        FieldKind::Color => match v.and_then(|v| v.as_str()) {
            Some("") => None,
            Some(s) if crate::look::parse_hex(s).is_some() => None,
            _ => Some("Elegí un color o dejalo del tema.".to_string()),
        },
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
    node.style.errors(&path, errors);
    node.look.errors(&path, errors);
}

/// Valida los bloques de una capa: ids (únicos en todo el documento),
/// tipos permitidos, tamaños, columnas, campos y superposiciones.
fn validate_nodes<'a>(
    nodes: &'a [Node],
    layer: Layer,
    ids: &mut std::collections::HashSet<&'a str>,
    errors: &mut Vec<String>,
) {
    // (id, x, y, w, h) de cada nodo, para chequear superposiciones.
    let mut rects: Vec<(&str, u32, u32, u32, u32)> = Vec::new();
    for node in nodes {
        if node.id.is_empty() {
            errors.push("hay un nodo sin id".to_string());
        } else if !ids.insert(node.id.as_str()) {
            errors.push(format!("id repetido: {}", node.id));
        }
        if !allowed_in(node.widget.type_name(), layer) {
            errors.push(match layer {
                Layer::Page(_) => format!(
                    "{} solo puede ir en la ficha del producto",
                    node.widget.label()
                ),
                Layer::ProductModal => format!(
                    "{} no puede ir en la ficha del producto",
                    node.widget.label()
                ),
            });
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

        validate_fields(node, errors);
    }

    for (i, a) in rects.iter().enumerate() {
        for b in &rects[i + 1..] {
            let overlap = a.1 < b.1 + b.3 && b.1 < a.1 + a.3 && a.2 < b.2 + b.4 && b.2 < a.2 + a.4;
            if overlap {
                errors.push(format!("los nodos {} y {} se superponen", a.0, b.0));
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
    doc.typography.errors(&mut errors);
    doc.layout.errors(&mut errors);
    doc.style.errors(&mut errors);
    if doc.nodes.len() > MAX_NODES {
        errors.push(format!(
            "hay {} nodos; el máximo es {MAX_NODES}",
            doc.nodes.len()
        ));
    }
    if doc.product_modal.nodes.len() > MAX_MODAL_NODES {
        errors.push(format!(
            "la ficha del producto tiene {} bloques; el máximo es {MAX_MODAL_NODES}",
            doc.product_modal.nodes.len()
        ));
    }
    if !MODAL_OPEN_OPTIONS
        .iter()
        .any(|(v, _)| *v == doc.product_modal.open)
    {
        errors.push(format!(
            "forma de abrir la ficha desconocida: {}",
            doc.product_modal.open
        ));
    }
    if !MODAL_BUYBAR_OPTIONS
        .iter()
        .any(|(v, _)| *v == doc.product_modal.buybar)
    {
        errors.push(format!(
            "barra de compra desconocida: {}",
            doc.product_modal.buybar
        ));
    }

    if doc.pages.len() > MAX_PAGES {
        errors.push(format!(
            "hay {} páginas; el máximo es {}",
            doc.page_count(),
            MAX_PAGES + 1
        ));
    }
    let mut paths = std::collections::HashSet::new();
    for page in &doc.pages {
        let name = &page.name;
        if name.trim().is_empty() {
            errors.push(format!("la página {} no tiene nombre", page.path));
        } else if name.chars().count() > MAX_PAGE_NAME {
            errors.push(format!(
                "Página {name}: usá {MAX_PAGE_NAME} caracteres como máximo en el nombre."
            ));
        }
        if normalize_path(&page.path).as_deref() != Some(page.path.as_str()) || page.path == "/" {
            errors.push(format!(
                "Página {name}: la dirección {} no es válida (usá minúsculas, números y guiones, como /ofertas)",
                page.path
            ));
        } else if is_reserved_path(&page.path) {
            errors.push(format!(
                "Página {name}: la dirección {} la usa la aplicación",
                page.path
            ));
        }
        if !paths.insert(page.path.as_str()) {
            errors.push(format!("hay dos páginas con la dirección {}", page.path));
        }
        if page.nodes.len() > MAX_NODES {
            errors.push(format!(
                "la página {name} tiene {} bloques; el máximo es {MAX_NODES}",
                page.nodes.len()
            ));
        }
    }

    let mut groups: std::collections::HashMap<&str, &str> = std::collections::HashMap::new();
    for n in doc.all_nodes().filter(|n| !n.shared.is_empty()) {
        let t = n.widget.type_name();
        if *groups.entry(n.shared.as_str()).or_insert(t) != t {
            errors.push(format!(
                "el grupo compartido {} mezcla bloques de distinto tipo",
                n.shared
            ));
        }
    }

    let mut ids = std::collections::HashSet::new();
    validate_nodes(&doc.nodes, HOME, &mut ids, &mut errors);
    for (i, page) in doc.pages.iter().enumerate() {
        validate_nodes(&page.nodes, Layer::Page(i + 1), &mut ids, &mut errors);
    }
    validate_nodes(
        &doc.product_modal.nodes,
        Layer::ProductModal,
        &mut ids,
        &mut errors,
    );

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Las fichas guardadas antes de los estilos de foto y precio se siguen
    /// viendo como antes (estilo clásico y solo el precio).
    #[test]
    fn old_product_photo_and_price_keep_their_look() {
        let w: Widget = serde_json::from_value(
            serde_json::json!({"type": "ProductImage", "props": {"fit": "cover"}}),
        )
        .unwrap();
        let Widget::ProductImage(p) = w else {
            panic!("foto")
        };
        assert_eq!(
            (p.variant.as_str(), p.badge.as_str(), p.stock_note),
            ("a", "", true)
        );
        let w: Widget =
            serde_json::from_value(serde_json::json!({"type": "ProductPrice", "props": {}}))
                .unwrap();
        let Widget::ProductPrice(p) = w else {
            panic!("precio")
        };
        assert_eq!(p.variant, "a");
        assert!(p.show_installments && p.show_transfer);
    }

    /// Menús y anuncios guardados antes de los estilos nuevos: se ven igual.
    #[test]
    fn old_navbar_and_announcement_keep_their_look() {
        let w: Widget = serde_json::from_value(serde_json::json!({"type": "Navbar", "props": {
            "brand": "", "logo_url": "", "links": [], "button_label": "", "button_href": ""
        }}))
        .unwrap();
        let Widget::Navbar(p) = w else {
            panic!("menú")
        };
        assert!(!p.search && !p.upper);
        assert_eq!(p.size, 26);
        // Un anuncio guardado con el formato anterior sigue abriéndose.
        let w: Widget = serde_json::from_value(serde_json::json!({"type": "Announcement", "props": {
            "variant": "b", "items": [{"text": "Hola", "link_label": "", "link_href": ""}]
        }}))
        .unwrap();
        let Widget::Announcement(p) = w else {
            panic!("anuncio")
        };
        assert_eq!((p.align.as_str(), p.pad), ("center", 12));
    }

    /// Los enlaces guardados antes de los desplegables no traen `children`
    /// ni `image_url`; la tienda tiene que seguir abriendo igual.
    #[test]
    fn old_links_without_dropdown_still_parse() {
        let item: LinkItem =
            serde_json::from_value(serde_json::json!({"label": "Inicio", "href": "/"})).unwrap();
        assert_eq!(item.label, "Inicio");
        assert!(item.children.is_empty());
        assert!(item.image_url.is_empty());
    }

    const EXAMPLE: &str = r#"{
      "schema": 3,
      "theme": "light",
      "nodes": [
        { "id": "n1", "type": "Banner",
          "props": { "kicker": "", "title": "Ofertas de la semana", "subtitle": "", "image_url": "https://example.com/hero.jpg",
                     "button_label": "", "button_href": "" },
          "pos": { "x": 0, "y": 0 }, "size": { "w": 48, "h": 48 } },
        { "id": "n2", "type": "ProductGrid",
          "props": { "title": "Destacados", "limit": 8, "card": "", "card_transfer": true, "card_installments": true, "card_favorite": true, "card_colors": true },
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
            let mut doc = PageDoc::default();
            let n = node(&format!("n{i}"), t, 0, 0);
            if allowed_in(t, HOME) {
                doc.nodes.push(n);
            } else {
                doc.product_modal.nodes = vec![n];
            }
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
    fn product_modal_open_and_buybar_are_checked() {
        let m: ProductModal = serde_json::from_value(serde_json::json!({"nodes": []})).unwrap();
        assert_eq!((m.open.as_str(), m.buybar.as_str()), ("", ""));
        let mut doc = PageDoc::default();
        doc.product_modal.open = "drawer".into();
        doc.product_modal.buybar = "dark".into();
        assert!(validate(&doc).is_ok());
        doc.product_modal.open = "flotante".into();
        assert!(validate(&doc).is_err());
    }

    #[test]
    fn product_modal_has_a_valid_default_and_is_not_saved_until_changed() {
        let doc = example();
        assert_eq!(doc.product_modal, ProductModal::default());
        assert_eq!(doc.product_modal.nodes.len(), 5);
        assert_eq!(validate(&doc), Ok(()));
        // Sin cambios no viaja en el JSON (ver `example_round_trips_through_json`).
        assert!(serde_json::to_value(&doc)
            .unwrap()
            .get("product_modal")
            .is_none());

        let mut doc = doc;
        doc.product_modal.nodes.remove(4);
        let json = serde_json::to_value(&doc).unwrap();
        assert_eq!(json["product_modal"]["nodes"].as_array().unwrap().len(), 4);
        assert_eq!(parse_doc(json), Ok(doc.clone()));
        assert_eq!(doc.layer_of("m1"), Some(Layer::ProductModal));
        assert_eq!(doc.layer_of("n1"), Some(HOME));
        assert!(doc.node("m2").is_some());
    }

    #[test]
    fn product_blocks_only_go_in_the_modal() {
        let mut doc = example();
        doc.nodes.push(node("p1", "ProductPrice", 0, 200));
        assert_invalid(&doc, "solo puede ir en la ficha");

        let mut doc = example();
        doc.product_modal.nodes.push(node("x1", "Navbar", 0, 100));
        assert_invalid(&doc, "no puede ir en la ficha");

        let mut doc = example();
        doc.product_modal.nodes.push(node("x1", "Faq", 0, 100));
        assert_eq!(validate(&doc), Ok(()));
        // Los ids no se pueden repetir entre la página y la ficha.
        doc.product_modal.nodes[5].id = "n1".into();
        assert_invalid(&doc, "repetido");
        // Las superposiciones se miran dentro de cada capa.
        doc.product_modal.nodes[5].id = "x1".into();
        doc.product_modal.nodes[5].pos.y = 40;
        assert_invalid(&doc, "superponen");
    }

    #[test]
    fn old_category_tiles_get_a_link_that_works() {
        let cats = ["Hogar".to_string()];
        // La página existe: el enlace queda como está.
        assert_eq!(
            category_tile_href("/categorias/hogar", "Hogar", true, &cats),
            "/categorias/hogar"
        );
        // No existe: la categoría con ese nombre o, si no hay, la etiqueta.
        assert_eq!(
            category_tile_href("/categorias/hogar", "hogar", false, &cats),
            "/?categoria=Hogar"
        );
        assert_eq!(
            category_tile_href("/categorias/tecnologia", "Tecnología", false, &cats),
            "/?etiqueta=tecnolog%C3%ADa"
        );
        // Otros enlaces no se tocan.
        assert_eq!(
            category_tile_href("/ofertas", "Ofertas", false, &cats),
            "/ofertas"
        );
    }

    #[test]
    fn normalizes_store_paths() {
        assert_eq!(
            normalize_path("/Ofertas/?x=1#y").as_deref(),
            Some("/ofertas")
        );
        assert_eq!(normalize_path("").as_deref(), Some("/"));
        assert_eq!(normalize_path("/").as_deref(), Some("/"));
        assert_eq!(
            normalize_path("/categorias/ropa").as_deref(),
            Some("/categorias/ropa")
        );
        assert_eq!(normalize_path("https://ejemplo.com"), None);
        assert_eq!(normalize_path("//ejemplo.com"), None);
        assert_eq!(normalize_path("/con espacio"), None);
        assert_eq!(normalize_path("/a//b"), None);
        assert!(is_reserved_path("/editor"));
        assert!(is_reserved_path("/pkg/x.js"));
        assert!(!is_reserved_path("/editores"));
        assert_eq!(name_from_path("/sobre-nosotros"), "Sobre nosotros");
    }

    #[test]
    fn new_pages_copy_the_menu_and_footer_and_validate() {
        let mut doc = example();
        doc.nodes.insert(0, node("nav", "Navbar", 0, 0));
        for n in doc.nodes.iter_mut().skip(1) {
            n.pos.y += 12;
        }
        let bottom = doc
            .nodes
            .iter()
            .map(|n| n.pos.y + u16::from(n.size.h))
            .max()
            .unwrap();
        doc.nodes.push(node("foot", "Footer", 0, bottom));
        assert_eq!(validate(&doc), Ok(()));

        let i = doc.add_page("/ofertas".into(), "Ofertas".into(), 0);
        assert_eq!(i, 1);
        assert_eq!(doc.page_index("/Ofertas/"), Some(1));
        assert_eq!(doc.page_index("/"), Some(0));
        assert_eq!(doc.page_index("/nada"), None);
        assert_eq!(doc.page_name(1), "Ofertas");
        let page = doc.layer(Layer::Page(1));
        let types: Vec<_> = page.iter().map(|n| n.widget.type_name()).collect();
        assert_eq!(types, ["Navbar", "Footer"]);
        // El pie sube hasta debajo del menú.
        assert_eq!(page[1].pos.y, 12);
        assert_eq!(validate(&doc), Ok(()));
        assert_eq!(doc.layer_of(&page[0].id.clone()), Some(Layer::Page(1)));

        // Ida y vuelta por JSON.
        let json = serde_json::to_value(&doc).unwrap();
        assert_eq!(json["pages"][0]["path"], "/ofertas");
        assert_eq!(parse_doc(json), Ok(doc.clone()));

        // Una página que no existe no tiene bloques.
        assert!(doc.layer(Layer::Page(9)).is_empty());

        let mut bad = doc.clone();
        bad.pages[0].path = "/editor".into();
        assert_invalid(&bad, "la usa la aplicación");
        let mut bad = doc.clone();
        bad.pages[0].path = "/Mal Escrito".into();
        assert_invalid(&bad, "no es válida");
        let mut bad = doc.clone();
        bad.add_page("/ofertas".into(), "Otra".into(), 0);
        assert_invalid(&bad, "dos páginas");
        let mut bad = doc.clone();
        bad.pages[0].nodes[0].id = "nav".into();
        assert_invalid(&bad, "repetido");
    }

    #[test]
    fn shared_menu_changes_on_every_page() {
        let mut doc = example();
        doc.nodes.insert(0, node("nav", "Navbar", 0, 200));
        let p = doc.add_page("/ofertas".into(), "Ofertas".into(), 0);
        doc.add_page("/contacto".into(), "Contacto".into(), 0);
        assert_eq!(doc.nodes[0].shared, "nav");
        assert_eq!(doc.shared_pages("nav"), 3);
        assert_eq!(validate(&doc), Ok(()));

        // Cambiar el menú en Ofertas lo cambia en Inicio y en Contacto.
        let before = doc.clone();
        let copy = doc.layer(Layer::Page(p))[0].id.clone();
        if let Some(Widget::Navbar(nb)) = doc.node_mut(&copy).map(|n| &mut n.widget) {
            nb.brand = "Otra marca".into();
        }
        doc.node_mut(&copy).unwrap().pos.x = 0;
        doc.node_mut(&copy).unwrap().style.bold = true;
        sync_shared(&before, &mut doc);
        for i in 0..3 {
            let n = &doc.layer(Layer::Page(i))[0];
            assert!(
                matches!(&n.widget, Widget::Navbar(nb) if nb.brand == "Otra marca"),
                "{i}"
            );
            assert!(n.style.bold);
        }
        // La posición es de cada página.
        assert_eq!(doc.nodes[0].pos.y, 200);
        assert_eq!(doc.layer(Layer::Page(p))[0].pos.y, 0);

        // Grupo con tipos mezclados: inválido.
        let mut bad = doc.clone();
        bad.nodes[1].shared = "nav".into();
        assert_invalid(&bad, "distinto tipo");
    }

    #[test]
    fn old_pages_get_their_menu_linked_on_load() {
        let mut doc = example();
        doc.nodes.insert(0, node("nav", "Navbar", 0, 200));
        doc.add_page("/ofertas".into(), "Ofertas".into(), 0);
        // Como quedaban antes: sin grupos.
        let mut old = doc.clone();
        for n in old.nodes.iter_mut().chain(old.pages[0].nodes.iter_mut()) {
            n.shared.clear();
        }
        let json = serde_json::to_value(&old).unwrap();
        assert!(json["nodes"][0].get("shared").is_none());
        let loaded = parse_doc(json).unwrap();
        assert_eq!(loaded.nodes[0].shared, "nav");
        assert_eq!(loaded.pages[0].nodes[0].shared, "nav");
        assert!(loaded.nodes[1].shared.is_empty());
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
    fn link_kinds_and_hrefs() {
        assert_eq!(
            category_href("Ropa de cama"),
            "/?categoria=Ropa%20de%20cama"
        );
        assert_eq!(
            LinkKind::of(&category_href("Mesa y cocina")),
            LinkKind::Category
        );
        assert_eq!(LinkKind::of(&product_href("ab-12")), LinkKind::Product);
        assert_eq!(LinkKind::of("/ofertas"), LinkKind::Page);
        assert_eq!(LinkKind::of(""), LinkKind::Page);
        assert_eq!(LinkKind::of("#contacto"), LinkKind::Other);
        assert_eq!(LinkKind::of("https://x.com"), LinkKind::Other);
        assert!(url_ok(&category_href("Mesa y cocina")));
        assert!(url_ok(&product_href("0b1c-22")));
    }

    #[test]
    fn rejects_number_out_of_range() {
        let mut doc = example();
        if let Widget::ProductGrid(g) = &mut doc.nodes[1].widget {
            g.limit = 25;
        }
        assert_invalid(&doc, "entre 1 y 24");
    }

    #[test]
    fn rejects_unknown_select_option() {
        let mut doc = PageDoc {
            nodes: vec![node("c", "Card", 0, 0)],
            ..PageDoc::default()
        };
        if let Widget::Card(c) = &mut doc.nodes[0].widget {
            c.layout = "diagonal".into();
        }
        assert_invalid(&doc, "Tarjeta › Diseño: Elegí una de las opciones");
    }

    #[test]
    fn typography_and_block_style_round_trip_and_validate() {
        let mut doc = example();
        // Sin letras propias, el JSON no cambia (ver `example_round_trips_through_json`).
        assert!(serde_json::to_value(&doc)
            .unwrap()
            .get("typography")
            .is_none());
        doc.typography = Typography {
            heading: "playfair".into(),
            body: "inter".into(),
            size: "large".into(),
        };
        doc.nodes[0].style = TextStyle {
            font: "oswald".into(),
            size: "larger".into(),
            bold: true,
            italic: true,
            align: "right".into(),
            animation: "wave".into(),
            anim_text: String::new(),
        };
        assert_eq!(validate(&doc), Ok(()));
        let json = serde_json::to_value(&doc).unwrap();
        assert_eq!(json["typography"]["heading"], "playfair");
        assert_eq!(json["nodes"][0]["style"]["font"], "oswald");
        assert_eq!(json["nodes"][0]["style"]["animation"], "wave");
        assert!(json["nodes"][1].get("style").is_none());
        assert_eq!(parse_doc(json), Ok(doc.clone()));

        // Un estilo a medias completa el resto con los valores por defecto.
        let mut partial = serde_json::to_value(example()).unwrap();
        partial["nodes"][0]["style"] = serde_json::json!({ "bold": true });
        let parsed = parse_doc(partial).unwrap();
        assert!(parsed.nodes[0].style.bold);
        assert_eq!(parsed.nodes[0].style.font, "");

        doc.typography.body = "comic-sans".into();
        assert_invalid(&doc, "Fuente del texto");
        doc.typography.body = "inter".into();
        doc.typography.size = "enorme".into();
        assert_invalid(&doc, "Letras › Tamaño");
        doc.typography.size = "normal".into();
        doc.nodes[0].style.align = "justify".into();
        assert_invalid(&doc, "Banner › Alineación");
        doc.nodes[0].style.align = String::new();
        doc.nodes[0].style.size = "gigante".into();
        assert_invalid(&doc, "Banner › Tamaño de letra");
        doc.nodes[0].style.size = String::new();
        doc.nodes[0].style.font = "papyrus".into();
        assert_invalid(&doc, "Banner › Fuente");
    }

    #[test]
    fn rejects_bad_url() {
        let mut doc = example();
        if let Widget::Banner(b) = &mut doc.nodes[0].widget {
            b.button_href = "javascript:alert(1)".into();
        }
        assert_invalid(
            &doc,
            "Banner › Enlace del botón: Escribí una página de tu tienda",
        );
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
        assert_eq!(tidy_link(" www.google.com "), "https://www.google.com");
        assert_eq!(
            tidy_link("instagram.com/macetas"),
            "https://instagram.com/macetas"
        );
        assert_eq!(
            tidy_link("hola@macetas.com.ar"),
            "mailto:hola@macetas.com.ar"
        );
        assert_eq!(tidy_link("ofertas"), "/ofertas");
        assert_eq!(tidy_link("#contacto"), "#contacto");
        assert_eq!(tidy_link("javascript:alert(1)"), "javascript:alert(1)");
        assert!(!url_ok("javascript:alert(1)"));
        assert!(url_ok("mailto:hola@x.com") && url_ok("tel:+541155551234") && url_ok("#a"));
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
