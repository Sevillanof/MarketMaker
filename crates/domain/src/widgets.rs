//! Catálogo de widgets: props de cada tipo, campos del inspector, límites de
//! tamaño y contenido de ejemplo.

use serde::{Deserialize, Serialize};

use crate::{Field, FieldKind, Layer, Size, SizeLimits};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(tag = "type", content = "props")]
pub enum Widget {
    Banner(BannerProps),
    Text(TextProps),
    ProductGrid(ProductGridProps),
    CategoryPage(CategoryFilterProps),
    Navbar(NavbarProps),
    Categories(CategoriesProps),
    Benefits(BenefitsProps),
    Card(CardProps),
    Faq(FaqProps),
    Testimonials(TestimonialsProps),
    Contact(ContactProps),
    Footer(FooterProps),
    Announcement(AnnouncementProps),
    /// La ficha completa en un solo bloque (foto, datos, compra y descripción).
    ProductSheet(ProductSheetProps),
}

/// La ficha del producto (diseños: columnas clásicas, panel enmarcado,
/// centrada). Reúne la foto con miniaturas, el nombre, el precio, las
/// opciones, la compra y la descripción en un solo bloque.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default)]
pub struct ProductSheetProps {
    /// Cartel sobre la foto, por ejemplo «Más vendido». Vacío: no se muestra.
    pub badge: String,
    pub show_thumbs: bool,
    pub show_favorite: bool,
    pub show_category: bool,
    pub show_share: bool,
    pub show_installments: bool,
    pub show_transfer: bool,
    /// «Quedan N» cuando quedan pocas unidades.
    pub show_stock: bool,
    pub show_description: bool,
    pub desc_title: String,
    pub desc_open: bool,
    pub show_trust: bool,
    /// Un renglón por línea (envíos, cambios, etc.).
    pub trust: String,
    /// Texto del botón de compra.
    pub button_label: String,
    /// Color propio del botón (`#rrggbb`); vacío, el del tema.
    pub button_color: String,
    /// Proporción de la foto principal: `1`, `0.8`, `1.25` o `1.5`.
    pub ratio: String,
    /// Esquinas, en px.
    pub radius: i64,
    pub bg: String,
    pub ink: String,
}

pub const SHEET_RATIO_OPTIONS: &[(&str, &str)] = &[
    ("1", "Cuadrada"),
    ("0.8", "Vertical (4:5)"),
    ("1.25", "Apaisada (5:4)"),
    ("1.5", "Panorámica (3:2)"),
];

impl Default for ProductSheetProps {
    fn default() -> Self {
        Self {
            badge: String::new(),
            show_thumbs: true,
            show_favorite: true,
            show_category: true,
            show_share: true,
            show_installments: true,
            show_transfer: true,
            show_stock: true,
            show_description: true,
            desc_title: "Descripción".to_string(),
            desc_open: true,
            show_trust: true,
            trust: "Envíos a todo el país\nCambios dentro de los 30 días".to_string(),
            button_label: "Agregar al carrito".to_string(),
            button_color: String::new(),
            ratio: "1".to_string(),
            radius: 8,
            bg: String::new(),
            ink: String::new(),
        }
    }
}

/// Portada (diseños: marco editorial, fotografía inmersiva, arco).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default)]
pub struct BannerProps {
    /// Antetítulo chico sobre el título.
    pub kicker: String,
    pub show_kicker: bool,
    pub title: String,
    pub subtitle: String,
    pub image_url: String,
    /// Descripción de la foto para quien no la ve.
    pub image_alt: String,
    pub show_button: bool,
    pub button_label: String,
    pub button_href: String,
    /// Color propio del botón (`#rrggbb`); vacío, el del tema.
    pub button_color: String,
    /// Oscuridad del velo sobre la foto en «Fotografía inmersiva» (60 a 85).
    pub veil: i64,
    /// Alineación del texto: `start` o `center`.
    pub align: String,
    /// Fondo y texto propios (`#rrggbb`); vacío, los del tema.
    pub bg: String,
    pub ink: String,
}

impl Default for BannerProps {
    fn default() -> Self {
        Self {
            kicker: String::new(),
            show_kicker: true,
            title: "Bienvenidos a nuestra tienda".to_string(),
            subtitle: "Escribí acá una frase sobre tu tienda".to_string(),
            image_url: "/img/banner.webp".to_string(),
            image_alt: String::new(),
            show_button: true,
            button_label: String::new(),
            button_href: String::new(),
            button_color: String::new(),
            veil: 68,
            align: "start".to_string(),
            bg: String::new(),
            ink: String::new(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct TextProps {
    pub heading: String,
    pub body: String,
    /// Alineación de los documentos anteriores a [`crate::TextStyle`]; la del
    /// estilo del bloque, si hay, tiene prioridad.
    #[serde(default)]
    pub align: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct ProductGridProps {
    pub title: String,
    pub limit: i64,
    #[serde(flatten)]
    pub cards: CardOptions,
    /// Antetítulo chico sobre el título (diseños nuevos).
    #[serde(default)]
    pub kicker: String,
    /// Enlace de la derecha del encabezado («Ver productos»).
    #[serde(default = "yes")]
    pub show_all: bool,
    #[serde(default = "grid_all_label")]
    pub all_label: String,
    #[serde(default)]
    pub all_href: String,
    /// Columnas en escritorio (2 a 6). El diseño «Lateral» usa siempre 2.
    #[serde(default = "grid_cols")]
    pub cols: i64,
    /// Mostrar la categoría del producto bajo el nombre.
    #[serde(default = "yes")]
    pub show_category: bool,
    /// Mostrar las cuotas bajo el precio.
    #[serde(default = "yes")]
    pub show_installments: bool,
    /// Esquinas de las fotos y tarjetas, en px.
    #[serde(default = "grid_radius")]
    pub radius: i64,
    /// Opacidad del panel del diseño «Flotante translúcida» (75 a 100 %).
    #[serde(default = "grid_panel_opacity")]
    pub panel_opacity: i64,
    /// Fondo y texto propios (`#rrggbb`); vacío, los del tema.
    #[serde(default)]
    pub bg: String,
    #[serde(default)]
    pub ink: String,
}

fn grid_panel_opacity() -> i64 {
    75
}
fn grid_all_label() -> String {
    "Ver productos".to_string()
}
fn grid_cols() -> i64 {
    4
}
fn grid_radius() -> i64 {
    2
}

impl Default for ProductGridProps {
    fn default() -> Self {
        Self {
            title: "Hechos para acompañarte".to_string(),
            limit: 8,
            cards: CardOptions::default(),
            kicker: String::new(),
            show_all: true,
            all_label: grid_all_label(),
            all_href: String::new(),
            cols: grid_cols(),
            show_category: true,
            show_installments: true,
            radius: grid_radius(),
            panel_opacity: grid_panel_opacity(),
            bg: String::new(),
            ink: String::new(),
        }
    }
}

/// Cómo se ven las tarjetas de producto de un bloque (cuadrícula, pestañas,
/// carrusel o producto destacado). Van "planas" en las props, así el
/// inspector las edita como cualquier otro campo.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct CardOptions {
    /// `""` clásica, `full` completa, `clean` limpia con Agregar.
    #[serde(default)]
    pub card: String,
    /// En la completa: el precio con transferencia (de Cobros).
    #[serde(default = "yes")]
    pub card_transfer: bool,
    /// En la completa: las cuotas sin interés (de Cobros).
    #[serde(default = "yes")]
    pub card_installments: bool,
    /// El corazón para guardar el producto en favoritos (en las tarjetas y la ficha).
    #[serde(default = "yes")]
    pub card_favorite: bool,
    /// Los puntos de color (o los talles) de los productos con variantes.
    #[serde(default = "yes")]
    pub card_colors: bool,
}

impl Default for CardOptions {
    fn default() -> Self {
        Self {
            card: String::new(),
            card_transfer: true,
            card_installments: true,
            card_favorite: true,
            card_colors: true,
        }
    }
}

pub const CARD_STYLE_OPTIONS: &[(&str, &str)] = &[
    ("", "Clásica"),
    ("full", "Completa: precios, cuotas y «Agregar al carrito»"),
    ("clean", "Limpia con «Añadir»"),
];

/// Las categorías de los productos, para que el comprador filtre las
/// cuadrículas de la misma página (los cinco bloques de categorías).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct CategoryFilterProps {
    pub title: String,
    /// Texto de la opción que muestra todos los productos.
    pub all_label: String,
    /// Mostrar cuántos productos tiene cada categoría.
    #[serde(default)]
    pub show_counts: bool,
}

/// Enlace con texto, usado en la navbar y el footer. En la navbar puede
/// tener `children`: las opciones de un desplegable.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
pub struct LinkItem {
    pub label: String,
    pub href: String,
    /// Opciones del desplegable (solo en el menú superior). Las páginas
    /// guardadas antes de los desplegables no lo traen.
    #[serde(default)]
    pub children: Vec<LinkItem>,
    /// Foto del panel ancho del desplegable (solo en el menú superior).
    #[serde(default)]
    pub image_url: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct NavbarProps {
    /// Cómo se abre un enlace con opciones: `list` (lista) o `panel` (panel ancho).
    #[serde(default = "variant_list")]
    pub menu: String,
    pub brand: String,
    pub logo_url: String,
    pub links: Vec<LinkItem>,
    pub button_label: String,
    pub button_href: String,
    /// Enlaces en mayúsculas.
    #[serde(default)]
    pub upper: bool,
    /// Buscador de productos (filtra los productos de la página).
    #[serde(default)]
    pub search: bool,
    /// Tamaño del nombre de la tienda, en píxeles.
    #[serde(default = "brand_size")]
    pub size: i64,
    /// Fondo y texto propios (`#rrggbb`); vacío, los del tema.
    #[serde(default)]
    pub bg: String,
    #[serde(default)]
    pub ink: String,
}

fn brand_size() -> i64 {
    26
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct CategoryItem {
    pub label: String,
    pub image_url: String,
    pub href: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct CategoriesProps {
    pub title: String,
    pub items: Vec<CategoryItem>,
    #[serde(default)]
    pub kicker: String,
    #[serde(default)]
    pub show_all: bool,
    #[serde(default = "cats_all_label")]
    pub all_label: String,
    #[serde(default)]
    pub all_href: String,
    /// Proporción de las fotos: `1.25`, `1`, `0.8` o `1.78`. En «Círculos» son cuadradas.
    #[serde(default = "cats_ratio")]
    pub ratio: String,
    /// Esquinas de las fotos, en px.
    #[serde(default = "cats_radius")]
    pub radius: i64,
    /// La flecha ↗ junto al nombre.
    #[serde(default = "yes")]
    pub arrow: bool,
    #[serde(default)]
    pub bg: String,
    #[serde(default)]
    pub ink: String,
}

fn cats_all_label() -> String {
    "Ver todo".to_string()
}
fn cats_ratio() -> String {
    "1.25".to_string()
}
fn cats_radius() -> i64 {
    2
}

impl Default for CategoriesProps {
    fn default() -> Self {
        Self {
            title: "Explorá por categoría".to_string(),
            items: Vec::new(),
            kicker: String::new(),
            show_all: false,
            all_label: cats_all_label(),
            all_href: String::new(),
            ratio: cats_ratio(),
            radius: cats_radius(),
            arrow: true,
            bg: String::new(),
            ink: String::new(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct BenefitItem {
    pub icon: String,
    pub title: String,
    pub text: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct BenefitsProps {
    pub items: Vec<BenefitItem>,
    /// Mostrar el texto chico bajo cada título.
    #[serde(default = "yes")]
    pub show_text: bool,
    /// Color propio de los íconos; vacío, el del tema.
    #[serde(default)]
    pub icon_color: String,
    #[serde(default)]
    pub bg: String,
    #[serde(default)]
    pub ink: String,
}

impl Default for BenefitsProps {
    fn default() -> Self {
        Self {
            items: Vec::new(),
            show_text: true,
            icon_color: String::new(),
            bg: String::new(),
            ink: String::new(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct CardProps {
    /// Antetítulo chico sobre el título (opcional; los documentos anteriores no lo traen).
    #[serde(default)]
    pub kicker: String,
    pub image_url: String,
    /// Descripción de la foto para quien no la ve.
    #[serde(default)]
    pub image_alt: String,
    pub title: String,
    pub body: String,
    pub button_label: String,
    pub button_href: String,
    /// Diseño de los documentos viejos (`side`, `overlay`, `vertical`); el de
    /// ahora se elige en «Estilo y color» del bloque.
    #[serde(default)]
    pub layout: String,
    /// Mostrar el botón.
    #[serde(default = "yes")]
    pub show_button: bool,
    /// Color propio del botón (`#rrggbb`); vacío, el del tema.
    #[serde(default)]
    pub button_color: String,
    /// De qué lado va la foto en «Lado a lado» y «Retrato»: `left` o `right`.
    #[serde(default = "card_side")]
    pub side: String,
    /// Fondo y texto propios (`#rrggbb`); vacío, los del tema.
    #[serde(default)]
    pub bg: String,
    #[serde(default)]
    pub ink: String,
}

fn card_side() -> String {
    "left".to_string()
}

impl Default for CardProps {
    fn default() -> Self {
        Self {
            kicker: String::new(),
            image_url: String::new(),
            image_alt: String::new(),
            title: "Contá tu historia".to_string(),
            body: String::new(),
            button_label: String::new(),
            button_href: String::new(),
            layout: String::new(),
            show_button: true,
            button_color: String::new(),
            side: card_side(),
            bg: String::new(),
            ink: String::new(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct FaqItem {
    pub question: String,
    pub answer: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct FaqProps {
    pub title: String,
    pub items: Vec<FaqItem>,
    #[serde(default)]
    pub kicker: String,
    /// La primera pregunta se ve abierta.
    #[serde(default = "yes")]
    pub open_first: bool,
    #[serde(default)]
    pub bg: String,
    #[serde(default)]
    pub ink: String,
}

impl Default for FaqProps {
    fn default() -> Self {
        Self {
            title: "Preguntas frecuentes".to_string(),
            items: Vec::new(),
            kicker: String::new(),
            open_first: true,
            bg: String::new(),
            ink: String::new(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Testimonial {
    /// Quién la dice.
    pub name: String,
    pub text: String,
    pub rating: i64,
    /// Aclaración chica junto al nombre («Opinión de ejemplo»).
    #[serde(default)]
    pub note: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct TestimonialsProps {
    /// Título de los documentos viejos; el diseño nuevo no lo muestra.
    #[serde(default)]
    pub title: String,
    pub items: Vec<Testimonial>,
    #[serde(default = "yes")]
    pub show_stars: bool,
    /// Color propio de las estrellas; vacío, el del tema.
    #[serde(default)]
    pub star_color: String,
    #[serde(default)]
    pub bg: String,
    #[serde(default)]
    pub ink: String,
}

impl Default for TestimonialsProps {
    fn default() -> Self {
        Self {
            title: String::new(),
            items: Vec::new(),
            show_stars: true,
            star_color: String::new(),
            bg: String::new(),
            ink: String::new(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct FooterProps {
    pub brand: String,
    pub description: String,
    /// Las columnas: cada enlace con `children` es una columna (su texto es
    /// el título). Los enlaces sueltos de los documentos viejos van juntos.
    pub links: Vec<LinkItem>,
    pub legal: String,
    /// El punto de acento después del nombre.
    #[serde(default = "yes")]
    pub show_dot: bool,
    /// Mostrar la descripción bajo el nombre.
    #[serde(default = "yes")]
    pub show_description: bool,
    #[serde(default)]
    pub bg: String,
    #[serde(default)]
    pub ink: String,
}

impl Default for FooterProps {
    fn default() -> Self {
        Self {
            brand: String::new(),
            description: String::new(),
            links: Vec::new(),
            legal: String::new(),
            show_dot: true,
            show_description: true,
            bg: String::new(),
            ink: String::new(),
        }
    }
}

/// Página de contacto: un lado con el título, el texto, el horario y la
/// dirección; el otro con los medios de contacto de "Datos de la tienda".
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct ContactProps {
    pub title: String,
    pub text: String,
    /// Días y horarios, uno por línea. Vacío: no se muestra.
    pub hours: String,
    /// Dónde retirar o visitar la tienda. Vacío: no se muestra.
    pub address: String,
}

fn yes() -> bool {
    true
}

fn variant_list() -> String {
    "list".to_string()
}

/// Franja fina con un mensaje corto (diseños: franja directa, etiqueta suave,
/// nota enmarcada). Todo se edita: texto, etiqueta, enlace, colores y alto.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default)]
pub struct AnnouncementProps {
    pub message: String,
    /// Etiqueta chica junto al mensaje (se ve en el diseño «Etiqueta suave»).
    pub tag: String,
    pub show_tag: bool,
    /// Texto del enlace, aparte del mensaje. Vacío: todo el mensaje es el
    /// enlace (como en las barras de antes).
    pub link_label: String,
    /// A dónde lleva la barra al tocarla. Vacío: no es un enlace.
    pub href: String,
    /// `center` o `left`.
    pub align: String,
    /// Alto de la barra: relleno de arriba y abajo, en píxeles.
    pub pad: i64,
    /// Color de fondo propio (`#rrggbb`); vacío, el del tema.
    pub bg: String,
    /// Color del texto propio (`#rrggbb`); vacío, el del tema.
    pub ink: String,
}

impl Default for AnnouncementProps {
    fn default() -> Self {
        Self {
            message: "Envíos a todo el país · 3 cuotas sin interés".to_string(),
            tag: "Novedades".to_string(),
            show_tag: true,
            link_label: String::new(),
            href: String::new(),
            align: "center".to_string(),
            pad: 12,
            bg: String::new(),
            ink: String::new(),
        }
    }
}

/// Tipos de widget disponibles, con su nombre para mostrar, en el orden de la paleta.
pub const WIDGET_TYPES: &[(&str, &str)] = &[
    ("Navbar", "Menú superior"),
    ("Banner", "Banner"),
    ("Categories", "Accesos con foto"),
    ("ProductGrid", "Cuadrícula de productos"),
    ("CategoryPage", "Categoría con subcategorías"),
    ("Benefits", "Beneficios"),
    ("Card", "Tarjeta"),
    ("Text", "Texto"),
    ("Faq", "Preguntas frecuentes"),
    ("Testimonials", "Testimonios"),
    ("Contact", "Contacto"),
    ("Footer", "Pie de página"),
    ("Announcement", "Barra de anuncio"),
    ("ProductSheet", "Ficha del producto"),
];

/// Bloques que solo tienen sentido dentro de la ficha de un producto.
pub const PRODUCT_MODAL_ONLY: &[&str] = &["ProductSheet"];

/// Bloques de la página que también se pueden poner en la ficha.
pub const PRODUCT_MODAL_SHARED: &[&str] = &["Text", "Benefits", "Faq", "Testimonials"];

/// ¿Se puede poner un bloque de este tipo en `layer`?
pub fn allowed_in(type_name: &str, layer: Layer) -> bool {
    let only_modal = PRODUCT_MODAL_ONLY.contains(&type_name);
    match layer {
        Layer::Page(_) => !only_modal,
        Layer::ProductModal => only_modal || PRODUCT_MODAL_SHARED.contains(&type_name),
    }
}

// ---------------------------------------------------------------------------
// Campos del inspector

const fn field(key: &'static str, label: &'static str, kind: FieldKind) -> Field {
    Field {
        key,
        label,
        kind,
        advanced: false,
    }
}

const fn adv(key: &'static str, label: &'static str, kind: FieldKind) -> Field {
    Field {
        key,
        label,
        kind,
        advanced: true,
    }
}

const fn text(max: usize) -> FieldKind {
    FieldKind::Text { max }
}

const fn long(max: usize) -> FieldKind {
    FieldKind::LongText { max }
}

pub const CARD_LAYOUT_OPTIONS: &[(&str, &str)] = &[
    ("vertical", "Imagen arriba"),
    ("side", "Imagen al costado"),
    ("overlay", "Texto sobre la imagen"),
];
pub const IMAGE_FIT_OPTIONS: &[(&str, &str)] = &[("cover", "Cubrir"), ("contain", "Ajustar")];
pub const BENEFIT_ICON_OPTIONS: &[(&str, &str)] = &[
    ("truck", "Envío"),
    ("shield", "Seguridad"),
    ("refresh", "Devoluciones"),
    ("card", "Pagos"),
    ("chat", "Atención"),
    ("star", "Calidad"),
    ("gift", "Regalo"),
    ("clock", "Rapidez"),
];

const BANNER_FIELDS: &[Field] = &[
    field("kicker", "Antetítulo", text(40)),
    field("title", "Título", text(80)),
    field("subtitle", "Descripción", text(160)),
    field("image_url", "Foto de la portada", FieldKind::Url),
    field("image_alt", "Descripción de la fotografía", text(120)),
    field("button_label", "Texto del botón", text(30)),
    field("button_href", "Destino del botón", FieldKind::Url),
    field(
        "veil",
        "Oscurecer la fotografía",
        FieldKind::Range {
            min: 60,
            max: 85,
            unit: "%",
        },
    ),
];
// La alineación del texto se elige en "Letras de este bloque" (`Node::style`);
// `TextProps::align` queda como valor de partida de los documentos viejos.
const TEXT_FIELDS: &[Field] = &[
    field("heading", "Título", text(80)),
    field("body", "Texto", long(1000)),
];
const PRODUCT_GRID_FIELDS: &[Field] = &[
    field("title", "Título", text(80)),
    field(
        "limit",
        "Productos por página",
        FieldKind::Number { min: 1, max: 24 },
    ),
    field("all_label", "Texto del enlace", text(30)),
    field("all_href", "Destino del enlace", FieldKind::Url),
    field("show_category", "Mostrar categoría", FieldKind::Bool),
    field(
        "show_installments",
        "Mostrar cuotas de ejemplo",
        FieldKind::Bool,
    ),
    field("card_favorite", "Mostrar favorito", FieldKind::Bool),
    field(
        "panel_opacity",
        "Opacidad del panel",
        FieldKind::Range {
            min: 75,
            max: 100,
            unit: "%",
        },
    ),
];
const TAG_FILTER_FIELDS: &[Field] = &[
    field("title", "Título (opcional)", text(60)),
    adv("all_label", "Texto del botón de todos", text(20)),
    adv(
        "show_counts",
        "Mostrar cuántos productos hay",
        FieldKind::Bool,
    ),
];
pub const NAV_MENU_OPTIONS: &[(&str, &str)] = &[
    ("list", "Lista simple"),
    ("panel", "Panel ancho con foto"),
    ("columns", "En columnas con título"),
];
pub const VARIANT_NAVBAR: &[(&str, &str)] = &[
    ("a", "Clásico"),
    ("b", "Flotante sobre la portada"),
    ("c", "Logo al centro"),
];
pub const NAV_TONE_OPTIONS: &[(&str, &str)] = &[
    ("", "Claro"),
    ("glass", "Vidrio (se ve la foto a través)"),
    ("dark", "Oscuro"),
    ("color", "Color del bloque"),
    ("soft", "Color suave del bloque"),
];
const SUBLINK_LEAF: &[Field] = &[
    field("label", "Texto", text(30)),
    field("href", "Enlace", FieldKind::Url),
];
const SUBLINK_ITEM: &[Field] = &[
    field("label", "Texto", text(30)),
    field("href", "Enlace", FieldKind::Url),
    adv(
        "children",
        "Enlaces de la columna (desplegable en columnas)",
        FieldKind::List {
            item: SUBLINK_LEAF,
            min: 0,
            max: 8,
        },
    ),
];
const NAV_LINK_ITEM: &[Field] = &[
    field("label", "Texto", text(30)),
    field("href", "Enlace", FieldKind::Url),
    adv(
        "children",
        "Opciones del desplegable",
        FieldKind::List {
            item: SUBLINK_ITEM,
            min: 0,
            max: 9,
        },
    ),
    adv("image_url", "Foto del panel ancho", FieldKind::Url),
];
const NAVBAR_FIELDS: &[Field] = &[
    field("brand", "Nombre de la tienda", text(40)),
    field("logo_url", "Logo", FieldKind::Url),
    field(
        "links",
        "Enlaces",
        FieldKind::List {
            item: NAV_LINK_ITEM,
            min: 0,
            max: 8,
        },
    ),
    field(
        "menu",
        "Desplegable",
        FieldKind::Select {
            options: NAV_MENU_OPTIONS,
        },
    ),
    field("search", "Buscador de productos", FieldKind::Bool),
];

const CATEGORY_ITEM: &[Field] = &[
    field("label", "Nombre", text(40)),
    field("image_url", "Imagen", FieldKind::Url),
    field("href", "Enlace", FieldKind::Url),
];
const PRODUCT_SHEET_FIELDS: &[Field] = &[
    field("badge", "Cartel sobre la foto", text(24)),
    field("show_thumbs", "Mostrar miniaturas", FieldKind::Bool),
    field("show_favorite", "Corazón de favoritos", FieldKind::Bool),
    field("show_category", "Mostrar la categoría", FieldKind::Bool),
    field("show_share", "Botón Compartir", FieldKind::Bool),
    field("show_transfer", "Precio con transferencia", FieldKind::Bool),
    field("show_installments", "Cuotas sin interés", FieldKind::Bool),
    field("show_stock", "Aviso «Quedan N»", FieldKind::Bool),
    field("button_label", "Texto del botón", text(30)),
    field("button_color", "Color del botón", FieldKind::Color),
    field(
        "show_description",
        "Mostrar la descripción",
        FieldKind::Bool,
    ),
    field("desc_title", "Título de la descripción", text(30)),
    field("desc_open", "Descripción abierta", FieldKind::Bool),
    field("show_trust", "Mostrar envíos y cambios", FieldKind::Bool),
    field("trust", "Texto de envíos y cambios", long(300)),
    field(
        "ratio",
        "Proporción de la foto",
        FieldKind::Select {
            options: SHEET_RATIO_OPTIONS,
        },
    ),
    field(
        "radius",
        "Esquinas",
        FieldKind::Range {
            min: 0,
            max: 24,
            unit: "px",
        },
    ),
    field("bg", "Color de fondo", FieldKind::Color),
    field("ink", "Color del texto", FieldKind::Color),
];
const CATEGORY_RATIO_OPTIONS: &[(&str, &str)] = &[
    ("1.25", "Apaisada (5:4)"),
    ("1", "Cuadrada"),
    ("0.8", "Vertical (4:5)"),
    ("1.78", "Panorámica (16:9)"),
];
const CATEGORIES_FIELDS: &[Field] = &[
    field("kicker", "Antetítulo", text(40)),
    field("title", "Título", text(80)),
    field("show_all", "Mostrar enlace «Ver todo»", FieldKind::Bool),
    field("all_label", "Texto del enlace", text(30)),
    field("all_href", "Adónde lleva el enlace", FieldKind::Url),
    field("arrow", "Flecha ↗ junto al nombre", FieldKind::Bool),
    field(
        "ratio",
        "Proporción de las fotos",
        FieldKind::Select {
            options: CATEGORY_RATIO_OPTIONS,
        },
    ),
    field(
        "radius",
        "Esquinas de las fotos",
        FieldKind::Range {
            min: 0,
            max: 24,
            unit: "px",
        },
    ),
    field(
        "items",
        "Categorías",
        FieldKind::List {
            item: CATEGORY_ITEM,
            min: 1,
            max: 12,
        },
    ),
];

const BENEFIT_ITEM: &[Field] = &[
    field(
        "icon",
        "Ícono",
        FieldKind::Select {
            options: BENEFIT_ICON_OPTIONS,
        },
    ),
    field("title", "Título", text(40)),
    field("text", "Texto", text(100)),
];
const BENEFITS_FIELDS: &[Field] = &[
    field(
        "items",
        "Beneficios",
        FieldKind::List {
            item: BENEFIT_ITEM,
            min: 1,
            max: 6,
        },
    ),
];

const CARD_FIELDS: &[Field] = &[
    field("title", "Título", text(80)),
    field("body", "Descripción", long(300)),
    field("button_label", "Texto del botón", text(30)),
    field("button_href", "Destino del botón", FieldKind::Url),
    field("image_url", "Foto de la historia", FieldKind::Url),
    field("image_alt", "Descripción de la fotografía", text(120)),
];

const FAQ_ITEM: &[Field] = &[
    field("question", "Pregunta", text(120)),
    field("answer", "Respuesta", long(600)),
];
const FAQ_FIELDS: &[Field] = &[
    field("title", "Título", text(80)),
    field(
        "items",
        "Preguntas",
        FieldKind::List {
            item: FAQ_ITEM,
            min: 1,
            max: 12,
        },
    ),
];

const TESTIMONIAL_ITEM: &[Field] = &[
    field("text", "Opinión", long(300)),
    field("name", "Quién la dice", text(40)),
    field("note", "Aclaración", text(40)),
    field(
        "rating",
        "Puntaje",
        FieldKind::Range {
            min: 1,
            max: 5,
            unit: "★",
        },
    ),
];
const TESTIMONIALS_FIELDS: &[Field] = &[
    field(
        "items",
        "Opiniones",
        FieldKind::List {
            item: TESTIMONIAL_ITEM,
            min: 1,
            max: 3,
        },
    ),
    field("show_stars", "Mostrar estrellas", FieldKind::Bool),
];

pub const VARIANT_PRODUCT_IMAGE: &[(&str, &str)] = &[
    ("a", "Clásico: foto y miniaturas abajo"),
    ("b", "Galería grande con flechas"),
    ("c", "Mosaico de fotos"),
    ("d", "Miniaturas al costado"),
    ("e", "Visor que se desliza"),
];
pub const VARIANT_PRODUCT_PRICE: &[(&str, &str)] = &[
    ("a", "Solo el precio"),
    ("b", "Con cuotas"),
    ("c", "Transferencia destacada"),
    ("d", "Cajita con formas de pago"),
    ("e", "Etiquetas chicas"),
];
/// El botón de la ficha agrega al carrito: ya no lleva enlace (`href`
/// queda solo para leer fichas viejas).
/// Cómo se ve la cantidad en el celular (ver [`ProductButtonProps::qty`]).
pub const QTY_STYLES: &[(&str, &str)] = &[
    ("a", "Botones redondos al lado"),
    ("b", "Cantidad arriba, botón abajo"),
];

const CONTACT_FIELDS: &[Field] = &[
    field("title", "Título", text(60)),
    field("text", "Texto", long(300)),
    field("hours", "Horario de atención", long(200)),
    field("address", "Dirección (opcional)", text(120)),
];

/// Enlace del pie: con "enlaces del grupo" la lista se arma en columnas con título.
const FOOTER_LINK_ITEM: &[Field] = &[
    field("label", "Título de la columna", text(30)),
    field(
        "children",
        "Enlaces",
        FieldKind::List {
            item: SUBLINK_LEAF,
            min: 0,
            max: 8,
        },
    ),
    adv("href", "Enlace del título (opcional)", FieldKind::Url),
];
const FOOTER_FIELDS: &[Field] = &[
    field("brand", "Nombre de la tienda", text(40)),
    field("show_description", "Mostrar descripción", FieldKind::Bool),
    field("description", "Descripción", text(200)),
    field(
        "links",
        "Columnas de enlaces",
        FieldKind::List {
            item: FOOTER_LINK_ITEM,
            min: 0,
            max: 3,
        },
    ),
    field("legal", "Texto legal", text(120)),
];

pub const COUPON_LOOK_OPTIONS: &[(&str, &str)] = &[
    ("", "Recortable (con muescas)"),
    ("solid", "Sólido"),
    ("soft", "Suave"),
    ("line", "Minimal, con línea"),
];
const BAR_ALIGN_OPTIONS: &[(&str, &str)] = &[("center", "Centrado"), ("left", "A la izquierda")];
const ANNOUNCEMENT_FIELDS: &[Field] = &[
    field("message", "Mensaje", text(120)),
    field("show_tag", "Mostrar etiqueta", FieldKind::Bool),
    field("tag", "Etiqueta", text(20)),
    field("link_label", "Texto del enlace", text(30)),
    field("href", "Destino del enlace", FieldKind::Url),
    field(
        "align",
        "Alineación",
        FieldKind::Select {
            options: BAR_ALIGN_OPTIONS,
        },
    ),
];

pub const VARIANT_STATS: &[(&str, &str)] = &[
    ("a", "En una caja"),
    ("b", "Sin caja, centradas"),
    ("c", "Tarjetas con ícono"),
];
pub const VARIANT_STEPS: &[(&str, &str)] = &[
    ("a", "En línea con números"),
    ("b", "Línea de pasos (solo títulos)"),
    ("c", "Tarjetas con número grande"),
];
pub const VARIANT_PRODUCT_CAROUSEL: &[(&str, &str)] = &[
    ("a", "Flechas arriba a la derecha"),
    ("b", "Flechas a los costados"),
    ("c", "Sin caja, la última tarjeta asoma"),
];
pub const VARIANT_SOCIAL: &[(&str, &str)] = &[
    ("a", "Íconos y botón flotante de WhatsApp"),
    ("b", "Botón flotante con texto"),
    ("c", "Franja de redes con usuario"),
];
/// Fin de una cuenta regresiva nueva: `days` días después de `unix_secs`, a
/// las 23:59, como lo da `<input type="datetime-local">` (`2026-11-27T23:59`).
/// Con una fecha fija en los valores de ejemplo, desde ese día todas las
/// cuentas regresivas nuevas nacían terminadas.
pub fn countdown_end(unix_secs: i64, days: i64) -> String {
    // Días desde 1970 → fecha civil (algoritmo de Howard Hinnant).
    let z = unix_secs.div_euclid(86_400) + days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!("{y:04}-{m:02}-{d:02}T23:59")
}

#[cfg(test)]
mod countdown_tests {
    use super::countdown_end;

    #[test]
    fn countdown_end_dates() {
        assert_eq!(countdown_end(0, 0), "1970-01-01T23:59");
        // 2026-11-20 12:00 UTC + 7 días
        assert_eq!(countdown_end(1_795_176_000, 7), "2026-11-27T23:59");
        // cruza el año y un 29 de febrero
        assert_eq!(countdown_end(1_795_176_000, 41), "2026-12-31T23:59");
        assert_eq!(countdown_end(1_795_176_000, 42), "2027-01-01T23:59");
        assert_eq!(countdown_end(1_826_798_400, 100), "2028-02-29T23:59");
    }
}

/// Alto (en filas de la grilla) para que una cuadrícula de `per_page`
/// productos, `w` columnas de ancho y con `total` productos en la tienda
/// muestre las fotos en proporción 4:3, con lugar para el título y los
/// números de página si hacen falta. Es una cuenta aproximada para el ancho
/// por defecto de la tienda; el dueño lo puede ajustar a mano después.
pub fn product_grid_rows(per_page: usize, w: u8, total: usize, has_title: bool) -> u8 {
    let width =
        f64::from(crate::layout::DEFAULT_WIDTH) * f64::from(w) / f64::from(crate::GRID_COLS);
    let n = per_page.min(total.max(1)).max(1);
    // Igual que `widgets::fill_cols`: 4 desde 768 px, 3 desde 512 px.
    let cols = if width >= 768.0 {
        4
    } else if width >= 512.0 && n != 8 && n != 16 {
        3
    } else {
        2
    }
    .min(n) as f64;
    let (pad, gap, body) = (32.0, 12.0, 96.0);
    let card_w = (width - pad - (cols - 1.0) * gap) / cols;
    let rows = (n as f64 / cols).ceil();
    let mut h = pad + rows * (card_w * 0.75 + body) + (rows - 1.0) * gap;
    if has_title {
        h += 40.0;
    }
    if total > per_page {
        h += 52.0;
    }
    (h / f64::from(crate::ROW_PX)).ceil().clamp(1.0, 255.0) as u8
}

// ---------------------------------------------------------------------------
// Límites de tamaño

const fn limits(min_w: u8, max_w: u8, min_h: u8, max_h: u8, w: u8, h: u8) -> SizeLimits {
    SizeLimits {
        min_w,
        max_w,
        min_h,
        max_h,
        default: Size { w, h },
    }
}

impl Widget {
    /// Nombre del tipo tal como se guarda en `type`.
    pub fn type_name(&self) -> &'static str {
        match self {
            Widget::Banner(_) => "Banner",
            Widget::Text(_) => "Text",
            Widget::ProductGrid(_) => "ProductGrid",
            Widget::CategoryPage(_) => "CategoryPage",
            Widget::Navbar(_) => "Navbar",
            Widget::Categories(_) => "Categories",
            Widget::Benefits(_) => "Benefits",
            Widget::Card(_) => "Card",
            Widget::Faq(_) => "Faq",
            Widget::Testimonials(_) => "Testimonials",
            Widget::Contact(_) => "Contact",
            Widget::Footer(_) => "Footer",
            Widget::Announcement(_) => "Announcement",
            Widget::ProductSheet(_) => "ProductSheet",
        }
    }

    /// Los widgets nuevos (de la maqueta de 30 diseños): su CSS está en
    /// `style/widgets-new.css` y no usan los cinco estilos de siempre.
    pub fn is_new_design(&self) -> bool {
        matches!(
            self,
            Widget::Announcement(_)
                | Widget::Navbar(_)
                | Widget::Banner(_)
                | Widget::Benefits(_)
                | Widget::Categories(_)
                | Widget::ProductGrid(_)
                | Widget::Card(_)
                | Widget::Testimonials(_)
                | Widget::Faq(_)
                | Widget::Footer(_)
                | Widget::ProductSheet(_)
        )
    }

    /// Los diseños que ofrece el widget: (número, nombre, descripción).
    pub fn designs(&self) -> &'static [(u8, &'static str, &'static str)] {
        match self {
            Widget::Announcement(_) => &[
                (
                    1,
                    "Franja directa",
                    "Un mensaje centrado con el color principal",
                ),
                (
                    2,
                    "Etiqueta suave",
                    "La información aparece junto a una pequeña etiqueta",
                ),
                (
                    3,
                    "Nota enmarcada",
                    "Una superficie contenida con acento lateral",
                ),
            ],
            Widget::Navbar(_) => &[
                (
                    1,
                    "Equilibrado",
                    "Marca a un lado y navegación en una línea",
                ),
                (
                    2,
                    "Marca al centro",
                    "Dos niveles: marca y accesos arriba, páginas abajo",
                ),
                (
                    3,
                    "Menú contenido",
                    "Una barra que se separa suavemente del fondo",
                ),
            ],
            Widget::Banner(_) => &[
                (
                    1,
                    "Marco editorial",
                    "Texto y fotografía contenidos, con aire interior",
                ),
                (
                    2,
                    "Fotografía inmersiva",
                    "La imagen ocupa el fondo y el texto queda en primer plano",
                ),
                (
                    3,
                    "Arco",
                    "Un recorte curvo acompaña el texto sobre fondo abierto",
                ),
            ],
            Widget::Benefits(_) => &[
                (
                    1,
                    "Línea editorial",
                    "Beneficios en fila, separados por filetes",
                ),
                (
                    2,
                    "Tarjetas ligeras",
                    "Cada beneficio en su tarjeta, con ícono redondo",
                ),
                (3, "Centrado", "Íconos arriba, en círculo, y texto centrado"),
            ],
            Widget::Categories(_) => &[
                (1, "Sobre la imagen", "El nombre sobre la foto, con degradé"),
                (2, "Pie editorial", "Foto con el nombre debajo y una línea"),
                (3, "Círculos", "Fotos redondas con el nombre centrado"),
            ],
            Widget::ProductGrid(_) => &[
                (1, "Lateral", "Foto a un lado y el texto al otro, de a dos"),
                (2, "Enmarcada", "Cada producto en una tarjeta con marco"),
                (
                    3,
                    "Flotante translúcida",
                    "El texto flota sobre la foto con un velo",
                ),
            ],
            Widget::Card(_) => &[
                (1, "Lado a lado", "Foto y texto en dos mitades"),
                (2, "Panorámica", "Foto ancha arriba y el texto debajo"),
                (
                    3,
                    "Retrato",
                    "Foto circular y texto con una línea de acento",
                ),
            ],
            Widget::Testimonials(_) => &[
                (1, "Editorial", "Cita centrada, sin caja"),
                (
                    2,
                    "Enmarcada",
                    "La cita dentro de una tarjeta sobre fondo suave",
                ),
                (
                    3,
                    "Cita lateral",
                    "Comillas de color y la cita entre filetes",
                ),
            ],
            Widget::Faq(_) => &[
                (1, "Líneas simples", "Preguntas separadas por filetes"),
                (2, "Acordeones enmarcados", "Cada pregunta en su recuadro"),
                (
                    3,
                    "Índice lateral",
                    "Título a un lado y preguntas numeradas",
                ),
            ],
            Widget::ProductSheet(_) => &[
                (
                    1,
                    "Columnas clásicas",
                    "Miniaturas al costado de la foto y el texto al lado",
                ),
                (
                    2,
                    "Panel enmarcado",
                    "La información dentro de un panel con marco",
                ),
                (3, "Centrada", "Foto grande arriba y todo centrado debajo"),
            ],
            Widget::Footer(_) => &[
                (1, "Columnas", "Marca y columnas de enlaces en una fila"),
                (
                    2,
                    "Marca protagonista",
                    "La marca centrada arriba y los enlaces abajo",
                ),
                (
                    3,
                    "Bloque contenido",
                    "Todo dentro de un recuadro sobre el fondo",
                ),
            ],
            _ => crate::DESIGNS,
        }
    }

    /// Nombre para mostrar del tipo.
    pub fn label(&self) -> &'static str {
        let t = self.type_name();
        WIDGET_TYPES
            .iter()
            .find(|(k, _)| *k == t)
            .map(|(_, l)| *l)
            .unwrap_or("Widget")
    }

    /// Ancho (columnas de 48) y alto (filas de 8 px) mínimos, máximos y por
    /// defecto.
    pub fn limits(&self) -> SizeLimits {
        match self {
            Widget::Banner(_) => limits(16, 48, 24, 120, 48, 68),
            Widget::Text(_) => limits(8, 48, 12, 96, 24, 24),
            Widget::ProductGrid(_) => limits(12, 48, 36, 255, 48, 70),
            Widget::CategoryPage(_) => limits(12, 48, 20, 96, 48, 36),
            Widget::Navbar(_) => limits(24, 48, 12, 24, 48, 12),
            Widget::Categories(_) => limits(12, 48, 24, 96, 48, 64),
            Widget::Benefits(_) => limits(12, 48, 12, 48, 48, 14),
            Widget::Card(_) => limits(8, 48, 36, 96, 48, 56),
            Widget::Faq(_) => limits(12, 48, 24, 120, 48, 56),
            Widget::Testimonials(_) => limits(16, 48, 36, 72, 48, 40),
            Widget::Contact(_) => limits(24, 48, 36, 96, 48, 52),
            Widget::Footer(_) => limits(24, 48, 14, 60, 48, 32),
            Widget::Announcement(_) => limits(24, 48, 4, 12, 48, 5),
            Widget::ProductSheet(_) => limits(24, 48, 40, 160, 48, 90),
        }
    }

    pub fn fields(&self) -> &'static [Field] {
        match self {
            Widget::Banner(_) => BANNER_FIELDS,
            Widget::Text(_) => TEXT_FIELDS,
            Widget::ProductGrid(_) => PRODUCT_GRID_FIELDS,
            Widget::Navbar(_) => NAVBAR_FIELDS,
            Widget::Categories(_) => CATEGORIES_FIELDS,
            Widget::Benefits(_) => BENEFITS_FIELDS,
            Widget::Card(_) => CARD_FIELDS,
            Widget::Faq(_) => FAQ_FIELDS,
            Widget::Testimonials(_) => TESTIMONIALS_FIELDS,
            Widget::Contact(_) => CONTACT_FIELDS,
            Widget::Footer(_) => FOOTER_FIELDS,
            Widget::Announcement(_) => ANNOUNCEMENT_FIELDS,
            Widget::CategoryPage(_) => TAG_FILTER_FIELDS,
            Widget::ProductSheet(_) => PRODUCT_SHEET_FIELDS,
        }
    }

    /// Widget nuevo con contenido de ejemplo, para que se vea bien sin configurar nada.
    /// Las imágenes de ejemplo se sirven desde `public/img`, sin depender de terceros.
    pub fn defaults(type_name: &str) -> Option<Widget> {
        let s = |v: &str| v.to_string();
        let link = |label: &str, href: &str| LinkItem {
            label: s(label),
            href: s(href),
            ..LinkItem::default()
        };
        let widget = match type_name {
            "Banner" => Widget::Banner(BannerProps {
                kicker: String::new(),
                title: s("Bienvenidos a nuestra tienda"),
                subtitle: s("Escribí acá una frase sobre tu tienda"),
                image_url: s("/img/banner.webp"),
                button_label: String::new(),
                button_href: String::new(),
            ..Default::default()
        }),
            "Text" => Widget::Text(TextProps {
                heading: s("Quiénes somos"),
                body: s("Contá acá quiénes son, qué venden y cómo trabajan."),
                align: s("left"),
            }),
            "ProductGrid" => Widget::ProductGrid(ProductGridProps {
                cards: CardOptions::default(),
                title: s("Productos"),
                limit: 8, ..Default::default() }),
            "CategoryPage" => Widget::CategoryPage(CategoryFilterProps {
                title: String::new(),
                all_label: s("Todas"),
                show_counts: false,
            }),
            "Navbar" => Widget::Navbar(NavbarProps {
                menu: s("list"),
                // Vacío: muestra el nombre de Datos de la tienda.
                brand: String::new(),
                logo_url: s("/img/logo.svg"),
                links: vec![link("Inicio", "/")],
                button_label: String::new(),
                button_href: String::new(),
                upper: false,
                search: false,
                size: 26,
                bg: String::new(),
                ink: String::new(),
            }),
            "Categories" => Widget::Categories(CategoriesProps {
                title: s("Categorías"),
                items: [
                    ("Tecnología", "tecnologia", "tecnología"),
                    ("Hogar", "hogar", "hogar"),
                    ("Moda", "moda", "accesorios"),
                    ("Deportes", "deportes", "deporte"),
                ]
                .iter()
                // Cada cuadro muestra los productos de ejemplo con esa etiqueta.
        .map(|(label, slug, tag)| CategoryItem {
                    label: s(label),
                    image_url: format!("/img/category-{slug}.webp"),
                    href: crate::tag_href(tag),
                })
                .collect(), ..Default::default() }),
            "Benefits" => Widget::Benefits(BenefitsProps {
                items: vec![
                    BenefitItem {
                        icon: s("truck"),
                        title: s("Envíos"),
                        text: s("Contá acá cómo y adónde enviás"),
                    },
                    BenefitItem {
                        icon: s("card"),
                        title: s("Medios de pago"),
                        text: s("Contá acá cómo te pueden pagar"),
                    },
                    BenefitItem {
                        icon: s("refresh"),
                        title: s("Cambios"),
                        text: s("Contá acá cómo son los cambios"),
                    },
                    BenefitItem {
                        icon: s("shield"),
                        title: s("Atención"),
                        text: s("Contá acá cómo te pueden contactar"),
                    },
                ], ..Default::default() }),
            "Card" => Widget::Card(CardProps {
                kicker: String::new(),
                image_url: s("/img/card.webp"),
                title: s("Novedades de la semana"),
                body: s("Contá acá qué hay de nuevo."),
                button_label: s("Ver más"),
                button_href: s("/"),
                layout: s("vertical"), ..Default::default() }),
            "Faq" => Widget::Faq(FaqProps {
                title: s("Preguntas frecuentes"),
                items: vec![
                    FaqItem {
                        question: s("¿Cuánto tarda el envío?"),
                        answer: s("Escribí acá la respuesta."),
                    },
                    FaqItem {
                        question: s("¿Cómo puedo pagar?"),
                        answer: s("Escribí acá la respuesta."),
                    },
                    FaqItem {
                        question: s("¿Cómo hago un cambio?"),
                        answer: s("Escribí acá la respuesta."),
                    },
                ], ..Default::default() }),
            "Testimonials" => Widget::Testimonials(TestimonialsProps {
                title: s("Lo que dicen nuestros clientes"),
                // Sin nombres inventados: se completan con opiniones reales.
                items: (1..=3)
                    .map(|i| Testimonial {
                        name: format!("Cliente {i}"),
                        text: s("Copiá acá lo que te dijo un cliente real."),
                        rating: 5, note: String::new() })
                    .collect(), ..Default::default() }),
            // Otra foto que la de la Tarjeta: agregadas juntas no se repiten.
            "Contact" => Widget::Contact(ContactProps {
                title: s("Hablemos"),
                text: s("¿Tenés dudas sobre un producto, un envío o tu pedido? Escribinos y te respondemos a la brevedad."),
                hours: s("Contá acá tus días y horarios de atención."),
                address: String::new(),
            }),
            "Footer" => Widget::Footer(FooterProps {
                // Vacíos: muestran el nombre de Datos de la tienda.
                brand: String::new(),
                description: s("Una frase corta sobre tu tienda."),
                links: vec![link("Inicio", "/")],
                legal: String::new(), ..Default::default() }),
            "Announcement" => Widget::Announcement(AnnouncementProps::default()),
            // El código se elige de los cupones de la tienda (lo completa el editor).
            "ProductSheet" => Widget::ProductSheet(ProductSheetProps::default()),
            _ => return None,
        };
        Some(widget)
    }
}
