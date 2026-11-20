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
    TagFilter(TagFilterProps),
    TagList(TagFilterProps),
    TagCircles(TagFilterProps),
    TagSearch(TagSearchProps),
    TagTabs(TagTabsProps),
    CategoryTabs(CategoryFilterProps),
    CategorySide(CategoryFilterProps),
    CategoryCards(CategoryFilterProps),
    CategoryDrop(CategoryFilterProps),
    CategoryPage(CategoryFilterProps),
    FeaturedProduct(FeaturedProductProps),
    Navbar(NavbarProps),
    Carousel(CarouselProps),
    Categories(CategoriesProps),
    Benefits(BenefitsProps),
    Countdown(CountdownProps),
    Card(CardProps),
    Tabs(TabsProps),
    Faq(FaqProps),
    HoverGallery(HoverGalleryProps),
    Testimonials(TestimonialsProps),
    Logos(LogosProps),
    Image(ImageProps),
    Contact(ContactProps),
    Footer(FooterProps),
    Announcement(AnnouncementProps),
    Coupon(CouponProps),
    Stats(StatsProps),
    Steps(StepsProps),
    ProductCarousel(ProductCarouselProps),
    Social(SocialProps),
    // Bloques de la ficha del producto: muestran el producto que se abrió.
    ProductImage(ProductImageProps),
    ProductName(ProductNameProps),
    ProductPrice(ProductPriceProps),
    ProductDescription(ProductDescriptionProps),
    ProductButton(ProductButtonProps),
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
pub struct ProductImageProps {
    pub fit: String,
    /// `a` clásico (foto grande y miniaturas abajo), `b` galería grande,
    /// `c` mosaico, `d` miniaturas al costado, `e` visor con flechas.
    #[serde(default = "variant_a")]
    pub variant: String,
    /// Cartel sobre la foto, por ejemplo "Más vendido". Vacío: no se muestra.
    #[serde(default)]
    pub badge: String,
    /// Avisa cuando quedan pocas unidades ("Quedan 3").
    #[serde(default = "yes")]
    pub stock_note: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
pub struct ProductNameProps {}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
pub struct ProductPriceProps {
    /// `a` solo el precio, `b` con cuotas, `c` transferencia destacada,
    /// `d` cajita con las formas de pago.
    #[serde(default = "variant_a")]
    pub variant: String,
    /// Las cuotas sin interés de Cobros (si la tienda las configuró).
    #[serde(default = "yes")]
    pub show_installments: bool,
    /// El precio con transferencia de Cobros (si la tienda tiene descuento).
    #[serde(default = "yes")]
    pub show_transfer: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
pub struct ProductDescriptionProps {}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
pub struct ProductButtonProps {
    pub label: String,
    #[serde(default)]
    pub href: String,
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
    ("full", "Completa: precio, transferencia y cuotas"),
    ("clean", "Limpia con «Añadir»"),
];

/// Las etiquetas de los productos, para que el comprador filtre las
/// cuadrículas de la misma página.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct TagFilterProps {
    pub title: String,
    /// Texto del botón que muestra todos los productos.
    pub all_label: String,
    /// Mostrar cuántos productos tiene cada etiqueta.
    #[serde(default)]
    pub show_counts: bool,
}

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

/// Buscador con un menú para filtrar por etiquetas.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct TagSearchProps {
    pub placeholder: String,
    #[serde(default)]
    pub show_counts: bool,
}

/// Cuadrícula de productos con las etiquetas como pestañas arriba.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct TagTabsProps {
    pub title: String,
    pub limit: i64,
    pub all_label: String,
    #[serde(flatten)]
    pub cards: CardOptions,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct FeaturedProductProps {
    pub product_id: String,
    pub show_price: bool,
    #[serde(flatten)]
    pub cards: CardOptions,
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
    /// `a` clásico, `b` flotante sobre la portada, `c` logo al centro.
    #[serde(default = "variant_a")]
    pub variant: String,
    /// Fondo: `""` claro, `glass` vidrio, `dark` oscuro, `color` color del
    /// bloque, `soft` color suave del bloque.
    #[serde(default)]
    pub tone: String,
    /// Enlaces en mayúsculas.
    #[serde(default)]
    pub upper: bool,
    /// Buscador de productos (filtra los productos de la página).
    #[serde(default)]
    pub search: bool,
    /// En el desplegable «En columnas»: fondo con el color del bloque.
    #[serde(default)]
    pub menu_color: bool,
}

impl NavbarProps {
    /// Flota sobre el bloque de abajo (la portada).
    pub fn floats(&self) -> bool {
        self.variant == "b"
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Slide {
    pub image_url: String,
    pub title: String,
    pub subtitle: String,
    pub button_label: String,
    pub button_href: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct CarouselProps {
    pub slides: Vec<Slide>,
    /// Segundos entre diapositivas; 0 la deja quieta.
    pub autoplay: i64,
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
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct CountdownProps {
    pub title: String,
    pub subtitle: String,
    /// Fecha y hora local de fin, como la da `<input type="datetime-local">`:
    /// `2026-12-24T20:00`.
    pub ends_at: String,
    pub ended_text: String,
    pub button_label: String,
    pub button_href: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct CardProps {
    pub image_url: String,
    pub title: String,
    pub body: String,
    pub button_label: String,
    pub button_href: String,
    pub layout: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct TabItem {
    pub label: String,
    pub body: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct TabsProps {
    pub tabs: Vec<TabItem>,
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
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct GalleryImage {
    pub image_url: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct HoverGalleryProps {
    pub images: Vec<GalleryImage>,
    pub title: String,
    pub subtitle: String,
    pub href: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Testimonial {
    pub name: String,
    pub text: String,
    pub rating: i64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct TestimonialsProps {
    pub title: String,
    pub items: Vec<Testimonial>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct LogoItem {
    pub image_url: String,
    pub alt: String,
    pub href: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct LogosProps {
    pub title: String,
    pub items: Vec<LogoItem>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct ImageProps {
    pub image_url: String,
    pub alt: String,
    pub href: String,
    pub fit: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct FooterProps {
    pub brand: String,
    pub description: String,
    pub links: Vec<LinkItem>,
    pub legal: String,
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

/// Estilo por defecto de los widgets con variantes (`a`, `b` o `c`).
fn variant_a() -> String {
    "a".to_string()
}

fn yes() -> bool {
    true
}

fn variant_list() -> String {
    "list".to_string()
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct AnnouncementItem {
    pub text: String,
    pub link_label: String,
    pub link_href: String,
}

/// Franja fina con un mensaje corto. Estilos: `a` simple, `b` varios
/// mensajes que rotan, `c` con botón para cerrarla.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct AnnouncementProps {
    #[serde(default = "variant_a")]
    pub variant: String,
    pub items: Vec<AnnouncementItem>,
    /// Estilo `d` (cupón): el código que se copia, por ejemplo `HOTSALE`.
    #[serde(default)]
    pub code: String,
    /// Estilo `d`: aclaración corta al costado, por ejemplo "Hasta el domingo".
    #[serde(default)]
    pub note: String,
    /// Estilo `d`: `notch` recortable, `solid` sólido, `soft` suave, `line` línea.
    #[serde(default)]
    pub coupon: String,
}

/// Franja con un código de descuento para copiar. El código es uno de los
/// cupones de la tienda: si se pausa o vence, el bloque no se ve.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(default)]
pub struct CouponProps {
    /// El código del cupón (de la pantalla Cupones), por ejemplo `HOTSALE`.
    pub code: String,
    /// El mensaje, por ejemplo "10 % en toda la tienda".
    pub text: String,
    /// Aclaración corta al costado, por ejemplo "Hasta el domingo".
    pub note: String,
    /// Forma: vacío recortable, `solid`, `soft` o `line`.
    pub coupon: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct StatItem {
    pub value: String,
    pub label: String,
    #[serde(default)]
    pub icon: String,
}

/// Números grandes que dan confianza. Estilos: `a` en una caja, `b` sin
/// caja, `c` tarjetas con ícono.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct StatsProps {
    #[serde(default = "variant_a")]
    pub variant: String,
    pub items: Vec<StatItem>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct StepItem {
    pub title: String,
    pub text: String,
}

/// «Cómo comprar» en pasos. Estilos: `a` en línea con números, `b` línea
/// de pasos (solo títulos), `c` tarjetas con número grande.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct StepsProps {
    #[serde(default = "variant_a")]
    pub variant: String,
    pub title: String,
    pub items: Vec<StepItem>,
}

/// Los productos en una fila que se desliza. Estilos: `a` flechas arriba a
/// la derecha, `b` flechas a los costados, `c` sin caja y con la última
/// tarjeta asomando.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct ProductCarouselProps {
    #[serde(default = "variant_a")]
    pub variant: String,
    pub title: String,
    pub limit: i64,
    #[serde(default)]
    pub link_label: String,
    #[serde(default)]
    pub link_href: String,
    #[serde(flatten)]
    pub cards: CardOptions,
}

/// Redes de la tienda y botón de WhatsApp. WhatsApp e Instagram salen de
/// "Datos de la tienda". Estilos: `a` íconos + botón flotante redondo, `b`
/// botón flotante con texto, `c` franja de redes con el usuario.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct SocialProps {
    #[serde(default = "variant_a")]
    pub variant: String,
    pub title: String,
    #[serde(default)]
    pub facebook: String,
    #[serde(default)]
    pub tiktok: String,
    /// Texto del botón flotante con texto (estilo `b`).
    #[serde(default)]
    pub label: String,
}

/// Tipos de widget disponibles, con su nombre para mostrar, en el orden de la paleta.
pub const WIDGET_TYPES: &[(&str, &str)] = &[
    ("Navbar", "Menú superior"),
    ("Banner", "Banner"),
    ("Carousel", "Carrusel"),
    ("Categories", "Categorías"),
    ("ProductGrid", "Cuadrícula de productos"),
    ("TagFilter", "Etiquetas en fila"),
    ("TagList", "Etiquetas en columna"),
    ("TagCircles", "Etiquetas con foto"),
    ("TagSearch", "Buscador con filtros"),
    ("TagTabs", "Productos con pestañas"),
    ("CategoryTabs", "Categorías en pestañas"),
    ("CategorySide", "Categorías al costado"),
    ("CategoryCards", "Categorías con foto"),
    ("CategoryDrop", "Menú de categorías"),
    ("CategoryPage", "Categoría con subcategorías"),
    ("FeaturedProduct", "Producto destacado"),
    ("Benefits", "Beneficios"),
    ("Countdown", "Oferta con cuenta regresiva"),
    ("Card", "Tarjeta"),
    ("Text", "Texto"),
    ("Tabs", "Pestañas"),
    ("Faq", "Preguntas frecuentes"),
    ("HoverGallery", "Galería de fotos"),
    ("Testimonials", "Testimonios"),
    ("Logos", "Logos de marcas"),
    ("Image", "Imagen"),
    ("Contact", "Contacto"),
    ("Footer", "Pie de página"),
    ("Announcement", "Barra de anuncio"),
    ("Coupon", "Cupón"),
    ("Stats", "Cifras destacadas"),
    ("Steps", "Pasos"),
    ("ProductCarousel", "Carrusel de productos"),
    ("Social", "Redes y WhatsApp"),
    ("ProductImage", "Foto del producto"),
    ("ProductName", "Nombre del producto"),
    ("ProductPrice", "Precio del producto"),
    ("ProductDescription", "Descripción del producto"),
    ("ProductButton", "Botón de la ficha"),
];

/// Bloques que solo tienen sentido dentro de la ficha de un producto.
pub const PRODUCT_MODAL_ONLY: &[&str] = &[
    "ProductImage",
    "ProductName",
    "ProductPrice",
    "ProductDescription",
    "ProductButton",
];

/// Bloques de la página que también se pueden poner en la ficha.
pub const PRODUCT_MODAL_SHARED: &[&str] =
    &["Text", "Image", "Benefits", "Tabs", "Faq", "Testimonials"];

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
    field("title", "Título", text(80)),
    field("subtitle", "Subtítulo", text(160)),
    field("image_url", "Imagen", FieldKind::Url),
    adv("button_label", "Texto del botón", text(30)),
    adv("button_href", "Enlace del botón", FieldKind::Url),
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
    field(
        "card",
        "Tarjetas",
        FieldKind::Select {
            options: CARD_STYLE_OPTIONS,
        },
    ),
    adv(
        "card_transfer",
        "Tarjeta completa: precio con transferencia",
        FieldKind::Bool,
    ),
    adv(
        "card_installments",
        "Tarjeta completa: cuotas sin interés",
        FieldKind::Bool,
    ),
    adv(
        "card_favorite",
        "Corazón de favoritos en las tarjetas y la ficha",
        FieldKind::Bool,
    ),
    adv(
        "card_colors",
        "Puntos de color en las tarjetas",
        FieldKind::Bool,
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
const TAG_SEARCH_FIELDS: &[Field] = &[
    field("placeholder", "Texto del buscador", text(40)),
    adv(
        "show_counts",
        "Mostrar cuántos productos hay",
        FieldKind::Bool,
    ),
];
const TAG_TABS_FIELDS: &[Field] = &[
    field("title", "Título", text(80)),
    field(
        "limit",
        "Cantidad de productos",
        FieldKind::Number { min: 1, max: 12 },
    ),
    adv("all_label", "Texto de la pestaña de todos", text(20)),
    field(
        "card",
        "Tarjetas",
        FieldKind::Select {
            options: CARD_STYLE_OPTIONS,
        },
    ),
    adv(
        "card_transfer",
        "Tarjeta completa: precio con transferencia",
        FieldKind::Bool,
    ),
    adv(
        "card_installments",
        "Tarjeta completa: cuotas sin interés",
        FieldKind::Bool,
    ),
    adv(
        "card_favorite",
        "Corazón de favoritos en las tarjetas y la ficha",
        FieldKind::Bool,
    ),
    adv(
        "card_colors",
        "Puntos de color en las tarjetas",
        FieldKind::Bool,
    ),
];
const FEATURED_PRODUCT_FIELDS: &[Field] = &[
    field("product_id", "Producto", FieldKind::ProductRef),
    adv("show_price", "Mostrar precio", FieldKind::Bool),
    field(
        "card",
        "Tarjetas",
        FieldKind::Select {
            options: CARD_STYLE_OPTIONS,
        },
    ),
    adv(
        "card_transfer",
        "Tarjeta completa: precio con transferencia",
        FieldKind::Bool,
    ),
    adv(
        "card_installments",
        "Tarjeta completa: cuotas sin interés",
        FieldKind::Bool,
    ),
    adv(
        "card_favorite",
        "Corazón de favoritos en las tarjetas y la ficha",
        FieldKind::Bool,
    ),
    adv(
        "card_colors",
        "Puntos de color en las tarjetas",
        FieldKind::Bool,
    ),
];

const LINK_ITEM: &[Field] = &[
    field("label", "Texto", text(30)),
    field("href", "Enlace", FieldKind::Url),
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
    field(
        "variant",
        "Estilo",
        FieldKind::Select {
            options: VARIANT_NAVBAR,
        },
    ),
    field(
        "tone",
        "Fondo",
        FieldKind::Select {
            options: NAV_TONE_OPTIONS,
        },
    ),
    field("brand", "Nombre de la tienda", text(40)),
    field("logo_url", "Logo", FieldKind::Url),
    field(
        "menu",
        "Desplegable",
        FieldKind::Select {
            options: NAV_MENU_OPTIONS,
        },
    ),
    field(
        "links",
        "Enlaces",
        FieldKind::List {
            item: NAV_LINK_ITEM,
            min: 0,
            max: 8,
        },
    ),
    adv(
        "menu_color",
        "Desplegable en columnas con fondo de color",
        FieldKind::Bool,
    ),
    adv("search", "Buscador de productos", FieldKind::Bool),
    adv("upper", "Enlaces en mayúsculas", FieldKind::Bool),
    adv("button_label", "Texto del botón", text(30)),
    adv("button_href", "Enlace del botón", FieldKind::Url),
];

const SLIDE_ITEM: &[Field] = &[
    field("image_url", "Imagen", FieldKind::Url),
    field("title", "Título", text(80)),
    field("subtitle", "Subtítulo", text(160)),
    field("button_label", "Texto del botón", text(30)),
    field("button_href", "Enlace del botón", FieldKind::Url),
];
const CAROUSEL_FIELDS: &[Field] = &[
    field(
        "slides",
        "Diapositivas",
        FieldKind::List {
            item: SLIDE_ITEM,
            min: 1,
            max: 8,
        },
    ),
    adv(
        "autoplay",
        "Segundos entre diapositivas (0 = quieto)",
        FieldKind::Number { min: 0, max: 30 },
    ),
];

const CATEGORY_ITEM: &[Field] = &[
    field("label", "Nombre", text(40)),
    field("image_url", "Imagen", FieldKind::Url),
    field("href", "Enlace", FieldKind::Url),
];
const CATEGORIES_FIELDS: &[Field] = &[
    field("title", "Título", text(80)),
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
const BENEFITS_FIELDS: &[Field] = &[field(
    "items",
    "Beneficios",
    FieldKind::List {
        item: BENEFIT_ITEM,
        min: 1,
        max: 6,
    },
)];

const COUNTDOWN_FIELDS: &[Field] = &[
    field("title", "Título", text(80)),
    field("subtitle", "Subtítulo", text(160)),
    field("ends_at", "Termina", FieldKind::DateTime),
    adv("ended_text", "Texto al terminar", text(80)),
    adv("button_label", "Texto del botón", text(30)),
    adv("button_href", "Enlace del botón", FieldKind::Url),
];

const CARD_FIELDS: &[Field] = &[
    field("image_url", "Imagen", FieldKind::Url),
    field("title", "Título", text(80)),
    field("body", "Texto", long(300)),
    field(
        "layout",
        "Diseño",
        FieldKind::Select {
            options: CARD_LAYOUT_OPTIONS,
        },
    ),
    adv("button_label", "Texto del botón", text(30)),
    adv("button_href", "Enlace del botón", FieldKind::Url),
];

const TAB_ITEM: &[Field] = &[
    field("label", "Pestaña", text(30)),
    field("body", "Contenido", long(1000)),
];
const TABS_FIELDS: &[Field] = &[field(
    "tabs",
    "Pestañas",
    FieldKind::List {
        item: TAB_ITEM,
        min: 1,
        max: 6,
    },
)];

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

const GALLERY_ITEM: &[Field] = &[field("image_url", "Imagen", FieldKind::Url)];
const HOVER_GALLERY_FIELDS: &[Field] = &[
    field(
        "images",
        "Imágenes",
        FieldKind::List {
            item: GALLERY_ITEM,
            min: 1,
            max: 10,
        },
    ),
    field("title", "Título", text(60)),
    field("subtitle", "Subtítulo", text(80)),
    adv("href", "Enlace", FieldKind::Url),
];

const TESTIMONIAL_ITEM: &[Field] = &[
    field("name", "Nombre", text(40)),
    field("text", "Opinión", long(300)),
    field("rating", "Estrellas", FieldKind::Number { min: 1, max: 5 }),
];
const TESTIMONIALS_FIELDS: &[Field] = &[
    field("title", "Título", text(80)),
    field(
        "items",
        "Testimonios",
        FieldKind::List {
            item: TESTIMONIAL_ITEM,
            min: 1,
            max: 6,
        },
    ),
];

const LOGO_ITEM: &[Field] = &[
    field("image_url", "Logo", FieldKind::Url),
    field("alt", "Nombre de la marca", text(40)),
    field("href", "Enlace", FieldKind::Url),
];
const LOGOS_FIELDS: &[Field] = &[
    field("title", "Título", text(80)),
    field(
        "items",
        "Marcas",
        FieldKind::List {
            item: LOGO_ITEM,
            min: 1,
            max: 12,
        },
    ),
];

const IMAGE_FIELDS: &[Field] = &[
    field("image_url", "Imagen", FieldKind::Url),
    field("alt", "Descripción", text(120)),
    adv("href", "Enlace", FieldKind::Url),
    adv(
        "fit",
        "Ajuste",
        FieldKind::Select {
            options: IMAGE_FIT_OPTIONS,
        },
    ),
];

pub const VARIANT_PRODUCT_IMAGE: &[(&str, &str)] = &[
    ("a", "Clásico: foto y miniaturas abajo"),
    ("b", "Galería grande con flechas"),
    ("c", "Mosaico de fotos"),
    ("d", "Miniaturas al costado"),
    ("e", "Visor que se desliza"),
];
const PRODUCT_IMAGE_FIELDS: &[Field] = &[
    field(
        "variant",
        "Estilo",
        FieldKind::Select {
            options: VARIANT_PRODUCT_IMAGE,
        },
    ),
    field("badge", "Cartel sobre la foto (opcional)", text(24)),
    field("stock_note", "Avisar cuando quedan pocas", FieldKind::Bool),
    adv(
        "fit",
        "Ajuste",
        FieldKind::Select {
            options: IMAGE_FIT_OPTIONS,
        },
    ),
];

pub const VARIANT_PRODUCT_PRICE: &[(&str, &str)] = &[
    ("a", "Solo el precio"),
    ("b", "Con cuotas"),
    ("c", "Transferencia destacada"),
    ("d", "Cajita con formas de pago"),
];
const PRODUCT_PRICE_FIELDS: &[Field] = &[
    field(
        "variant",
        "Estilo",
        FieldKind::Select {
            options: VARIANT_PRODUCT_PRICE,
        },
    ),
    field(
        "show_installments",
        "Mostrar cuotas sin interés (se eligen en Cobros)",
        FieldKind::Bool,
    ),
    field(
        "show_transfer",
        "Mostrar precio con transferencia (se elige en Cobros)",
        FieldKind::Bool,
    ),
];
/// El botón de la ficha agrega al carrito: ya no lleva enlace (`href`
/// queda solo para leer fichas viejas).
const PRODUCT_BUTTON_FIELDS: &[Field] = &[field("label", "Texto del botón", text(30))];

const CONTACT_FIELDS: &[Field] = &[
    field("title", "Título", text(60)),
    field("text", "Texto", long(300)),
    field("hours", "Horario de atención", long(200)),
    field("address", "Dirección (opcional)", text(120)),
];

const FOOTER_FIELDS: &[Field] = &[
    field("brand", "Nombre de la tienda", text(40)),
    field("description", "Descripción", text(200)),
    field(
        "links",
        "Enlaces",
        FieldKind::List {
            item: LINK_ITEM,
            min: 0,
            max: 12,
        },
    ),
    field("legal", "Texto legal", text(120)),
];

pub const VARIANT_ANNOUNCEMENT: &[(&str, &str)] = &[
    ("a", "Simple (muestra el primer mensaje)"),
    ("b", "Mensajes que rotan"),
    ("c", "Con botón para cerrarla"),
    ("d", "Cupón con código para copiar"),
];
pub const COUPON_LOOK_OPTIONS: &[(&str, &str)] = &[
    ("", "Recortable (con muescas)"),
    ("solid", "Sólido"),
    ("soft", "Suave"),
    ("line", "Minimal, con línea"),
];
const ANNOUNCEMENT_ITEM: &[Field] = &[
    field("text", "Mensaje", text(100)),
    field("link_label", "Texto del enlace", text(30)),
    field("link_href", "Enlace", FieldKind::Url),
];
const COUPON_FIELDS: &[Field] = &[
    field("code", "Cupón", text(24)),
    field("text", "Mensaje", text(80)),
    field("note", "Aclaración (opcional)", text(40)),
    field(
        "coupon",
        "Forma",
        FieldKind::Select {
            options: COUPON_LOOK_OPTIONS,
        },
    ),
];
const ANNOUNCEMENT_FIELDS: &[Field] = &[
    field(
        "variant",
        "Estilo",
        FieldKind::Select {
            options: VARIANT_ANNOUNCEMENT,
        },
    ),
    field(
        "items",
        "Mensajes",
        FieldKind::List {
            item: ANNOUNCEMENT_ITEM,
            min: 1,
            max: 5,
        },
    ),
    field("code", "Código del cupón (estilo cupón)", text(24)),
    field(
        "note",
        "Aclaración (estilo cupón), ej. «Hasta el domingo»",
        text(40),
    ),
    field(
        "coupon",
        "Forma del cupón",
        FieldKind::Select {
            options: COUPON_LOOK_OPTIONS,
        },
    ),
];

pub const VARIANT_STATS: &[(&str, &str)] = &[
    ("a", "En una caja"),
    ("b", "Sin caja, centradas"),
    ("c", "Tarjetas con ícono"),
];
const STAT_ITEM: &[Field] = &[
    field("value", "Número", text(12)),
    field("label", "Qué mide", text(60)),
    adv(
        "icon",
        "Ícono (estilo tarjetas)",
        FieldKind::Select {
            options: BENEFIT_ICON_OPTIONS,
        },
    ),
];
const STATS_FIELDS: &[Field] = &[
    field(
        "variant",
        "Estilo",
        FieldKind::Select {
            options: VARIANT_STATS,
        },
    ),
    field(
        "items",
        "Cifras",
        FieldKind::List {
            item: STAT_ITEM,
            min: 2,
            max: 4,
        },
    ),
];

pub const VARIANT_STEPS: &[(&str, &str)] = &[
    ("a", "En línea con números"),
    ("b", "Línea de pasos (solo títulos)"),
    ("c", "Tarjetas con número grande"),
];
const STEP_ITEM: &[Field] = &[
    field("title", "Título", text(40)),
    field("text", "Texto", text(100)),
];
const STEPS_FIELDS: &[Field] = &[
    field(
        "variant",
        "Estilo",
        FieldKind::Select {
            options: VARIANT_STEPS,
        },
    ),
    field("title", "Título", text(80)),
    field(
        "items",
        "Pasos",
        FieldKind::List {
            item: STEP_ITEM,
            min: 2,
            max: 5,
        },
    ),
];

pub const VARIANT_PRODUCT_CAROUSEL: &[(&str, &str)] = &[
    ("a", "Flechas arriba a la derecha"),
    ("b", "Flechas a los costados"),
    ("c", "Sin caja, la última tarjeta asoma"),
];
const PRODUCT_CAROUSEL_FIELDS: &[Field] = &[
    field(
        "variant",
        "Estilo",
        FieldKind::Select {
            options: VARIANT_PRODUCT_CAROUSEL,
        },
    ),
    field("title", "Título", text(80)),
    field(
        "limit",
        "Cantidad de productos",
        FieldKind::Number { min: 2, max: 24 },
    ),
    adv("link_label", "Texto de «Ver todos»", text(30)),
    adv("link_href", "Enlace de «Ver todos»", FieldKind::Url),
    field(
        "card",
        "Tarjetas",
        FieldKind::Select {
            options: CARD_STYLE_OPTIONS,
        },
    ),
    adv(
        "card_transfer",
        "Tarjeta completa: precio con transferencia",
        FieldKind::Bool,
    ),
    adv(
        "card_installments",
        "Tarjeta completa: cuotas sin interés",
        FieldKind::Bool,
    ),
    adv(
        "card_favorite",
        "Corazón de favoritos en las tarjetas y la ficha",
        FieldKind::Bool,
    ),
    adv(
        "card_colors",
        "Puntos de color en las tarjetas",
        FieldKind::Bool,
    ),
];

pub const VARIANT_SOCIAL: &[(&str, &str)] = &[
    ("a", "Íconos y botón flotante de WhatsApp"),
    ("b", "Botón flotante con texto"),
    ("c", "Franja de redes con usuario"),
];
const SOCIAL_FIELDS: &[Field] = &[
    field(
        "variant",
        "Estilo",
        FieldKind::Select {
            options: VARIANT_SOCIAL,
        },
    ),
    field("title", "Título", text(40)),
    field("label", "Texto del botón flotante", text(30)),
    adv("facebook", "Facebook (enlace)", FieldKind::Url),
    adv("tiktok", "TikTok (enlace)", FieldKind::Url),
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

impl crate::PageDoc {
    /// Pone `ends_at` en todas las cuentas regresivas de la tienda (para una
    /// tienda recién creada desde una plantilla).
    pub fn set_countdowns_end(&mut self, ends_at: &str) {
        let pages = self.pages.iter_mut().map(|p| &mut p.nodes);
        for nodes in std::iter::once(&mut self.nodes).chain(pages) {
            for n in nodes.iter_mut() {
                if let Widget::Countdown(c) = &mut n.widget {
                    c.ends_at = ends_at.to_string();
                }
            }
        }
    }
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
            Widget::TagFilter(_) => "TagFilter",
            Widget::TagList(_) => "TagList",
            Widget::TagCircles(_) => "TagCircles",
            Widget::TagSearch(_) => "TagSearch",
            Widget::TagTabs(_) => "TagTabs",
            Widget::CategoryTabs(_) => "CategoryTabs",
            Widget::CategorySide(_) => "CategorySide",
            Widget::CategoryCards(_) => "CategoryCards",
            Widget::CategoryDrop(_) => "CategoryDrop",
            Widget::CategoryPage(_) => "CategoryPage",
            Widget::FeaturedProduct(_) => "FeaturedProduct",
            Widget::Navbar(_) => "Navbar",
            Widget::Carousel(_) => "Carousel",
            Widget::Categories(_) => "Categories",
            Widget::Benefits(_) => "Benefits",
            Widget::Countdown(_) => "Countdown",
            Widget::Card(_) => "Card",
            Widget::Tabs(_) => "Tabs",
            Widget::Faq(_) => "Faq",
            Widget::HoverGallery(_) => "HoverGallery",
            Widget::Testimonials(_) => "Testimonials",
            Widget::Logos(_) => "Logos",
            Widget::Image(_) => "Image",
            Widget::Contact(_) => "Contact",
            Widget::Footer(_) => "Footer",
            Widget::Announcement(_) => "Announcement",
            Widget::Coupon(_) => "Coupon",
            Widget::Stats(_) => "Stats",
            Widget::Steps(_) => "Steps",
            Widget::ProductCarousel(_) => "ProductCarousel",
            Widget::Social(_) => "Social",
            Widget::ProductImage(_) => "ProductImage",
            Widget::ProductName(_) => "ProductName",
            Widget::ProductPrice(_) => "ProductPrice",
            Widget::ProductDescription(_) => "ProductDescription",
            Widget::ProductButton(_) => "ProductButton",
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
            Widget::Banner(_) => limits(16, 48, 24, 96, 48, 48),
            Widget::Text(_) => limits(8, 48, 12, 96, 24, 24),
            Widget::ProductGrid(_) => limits(12, 48, 36, 255, 48, 96),
            Widget::TagFilter(_) => limits(12, 48, 7, 24, 48, 9),
            Widget::TagList(_) => limits(8, 24, 20, 144, 12, 44),
            Widget::TagCircles(_) => limits(12, 48, 15, 36, 48, 19),
            Widget::TagSearch(_) => limits(16, 48, 7, 16, 48, 9),
            Widget::TagTabs(_) => limits(12, 48, 36, 144, 48, 96),
            Widget::CategoryTabs(_) => limits(12, 48, 7, 24, 48, 9),
            Widget::CategorySide(_) => limits(8, 24, 20, 144, 12, 44),
            Widget::CategoryCards(_) => limits(12, 48, 20, 96, 48, 40),
            Widget::CategoryDrop(_) => limits(6, 24, 5, 12, 12, 6),
            Widget::CategoryPage(_) => limits(12, 48, 20, 96, 48, 36),
            Widget::FeaturedProduct(_) => limits(8, 24, 36, 96, 12, 48),
            Widget::Navbar(_) => limits(24, 48, 12, 24, 48, 12),
            Widget::Carousel(_) => limits(24, 48, 36, 96, 48, 48),
            Widget::Categories(_) => limits(12, 48, 24, 96, 48, 36),
            Widget::Benefits(_) => limits(12, 48, 12, 48, 48, 24),
            Widget::Countdown(_) => limits(16, 48, 36, 72, 48, 36),
            Widget::Card(_) => limits(8, 48, 36, 96, 16, 48),
            Widget::Tabs(_) => limits(12, 48, 24, 96, 24, 36),
            Widget::Faq(_) => limits(12, 48, 24, 120, 24, 48),
            Widget::HoverGallery(_) => limits(8, 24, 36, 96, 12, 48),
            Widget::Testimonials(_) => limits(16, 48, 36, 72, 48, 36),
            Widget::Logos(_) => limits(12, 48, 12, 36, 48, 24),
            Widget::Image(_) => limits(4, 48, 12, 120, 16, 36),
            Widget::Contact(_) => limits(24, 48, 36, 96, 48, 52),
            Widget::Footer(_) => limits(24, 48, 14, 60, 48, 18),
            Widget::Announcement(_) => limits(24, 48, 4, 12, 48, 5),
            Widget::Coupon(_) => limits(24, 48, 5, 12, 48, 7),
            Widget::Stats(_) => limits(16, 48, 10, 40, 48, 16),
            Widget::Steps(_) => limits(16, 48, 16, 60, 48, 28),
            Widget::ProductCarousel(_) => limits(16, 48, 36, 120, 48, 58),
            Widget::Social(_) => limits(12, 48, 4, 40, 48, 12),
            Widget::ProductImage(_) => limits(8, 48, 12, 120, 24, 52),
            Widget::ProductName(_) => limits(8, 48, 5, 24, 24, 9),
            Widget::ProductPrice(_) => limits(6, 48, 4, 32, 24, 9),
            Widget::ProductDescription(_) => limits(8, 48, 6, 96, 24, 28),
            Widget::ProductButton(_) => limits(8, 48, 6, 12, 24, 7),
        }
    }

    pub fn fields(&self) -> &'static [Field] {
        match self {
            Widget::Banner(_) => BANNER_FIELDS,
            Widget::Text(_) => TEXT_FIELDS,
            Widget::ProductGrid(_) => PRODUCT_GRID_FIELDS,
            Widget::TagFilter(_) | Widget::TagList(_) | Widget::TagCircles(_) => TAG_FILTER_FIELDS,
            Widget::TagSearch(_) => TAG_SEARCH_FIELDS,
            Widget::TagTabs(_) => TAG_TABS_FIELDS,
            Widget::CategoryTabs(_)
            | Widget::CategorySide(_)
            | Widget::CategoryCards(_)
            | Widget::CategoryDrop(_)
            | Widget::CategoryPage(_) => TAG_FILTER_FIELDS,
            Widget::FeaturedProduct(_) => FEATURED_PRODUCT_FIELDS,
            Widget::Navbar(_) => NAVBAR_FIELDS,
            Widget::Carousel(_) => CAROUSEL_FIELDS,
            Widget::Categories(_) => CATEGORIES_FIELDS,
            Widget::Benefits(_) => BENEFITS_FIELDS,
            Widget::Countdown(_) => COUNTDOWN_FIELDS,
            Widget::Card(_) => CARD_FIELDS,
            Widget::Tabs(_) => TABS_FIELDS,
            Widget::Faq(_) => FAQ_FIELDS,
            Widget::HoverGallery(_) => HOVER_GALLERY_FIELDS,
            Widget::Testimonials(_) => TESTIMONIALS_FIELDS,
            Widget::Logos(_) => LOGOS_FIELDS,
            Widget::Image(_) => IMAGE_FIELDS,
            Widget::Contact(_) => CONTACT_FIELDS,
            Widget::Footer(_) => FOOTER_FIELDS,
            Widget::Announcement(_) => ANNOUNCEMENT_FIELDS,
            Widget::Coupon(_) => COUPON_FIELDS,
            Widget::Stats(_) => STATS_FIELDS,
            Widget::Steps(_) => STEPS_FIELDS,
            Widget::ProductCarousel(_) => PRODUCT_CAROUSEL_FIELDS,
            Widget::Social(_) => SOCIAL_FIELDS,
            Widget::ProductImage(_) => PRODUCT_IMAGE_FIELDS,
            Widget::ProductPrice(_) => PRODUCT_PRICE_FIELDS,
            Widget::ProductName(_) | Widget::ProductDescription(_) => &[],
            Widget::ProductButton(_) => PRODUCT_BUTTON_FIELDS,
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
                title: s("Bienvenidos a nuestra tienda"),
                subtitle: s("Escribí acá una frase sobre tu tienda"),
                image_url: s("/img/banner.webp"),
                button_label: String::new(),
                button_href: String::new(),
            }),
            "Text" => Widget::Text(TextProps {
                heading: s("Quiénes somos"),
                body: s("Contá acá quiénes son, qué venden y cómo trabajan."),
                align: s("left"),
            }),
            "ProductGrid" => Widget::ProductGrid(ProductGridProps {
                cards: CardOptions::default(),
                title: s("Productos"),
                limit: 8,
            }),
            "TagFilter" => Widget::TagFilter(TagFilterProps {
                title: String::new(),
                all_label: s("Todos"),
                show_counts: true,
            }),
            "TagList" => Widget::TagList(TagFilterProps {
                title: s("Filtrar"),
                all_label: s("Limpiar filtros"),
                show_counts: true,
            }),
            "TagCircles" => Widget::TagCircles(TagFilterProps {
                title: String::new(),
                all_label: s("Todos"),
                show_counts: false,
            }),
            "CategoryTabs" => Widget::CategoryTabs(CategoryFilterProps {
                title: String::new(),
                all_label: s("Todas"),
                show_counts: false,
            }),
            "CategorySide" => Widget::CategorySide(CategoryFilterProps {
                title: s("Categorías"),
                all_label: s("Todas"),
                show_counts: true,
            }),
            "CategoryCards" => Widget::CategoryCards(CategoryFilterProps {
                title: s("Comprá por categoría"),
                all_label: s("Todas"),
                show_counts: true,
            }),
            "CategoryDrop" => Widget::CategoryDrop(CategoryFilterProps {
                title: s("Categorías"),
                all_label: s("Todas"),
                show_counts: false,
            }),
            "CategoryPage" => Widget::CategoryPage(CategoryFilterProps {
                title: String::new(),
                all_label: s("Todas"),
                show_counts: false,
            }),
            "TagSearch" => Widget::TagSearch(TagSearchProps {
                placeholder: s("Buscar productos"),
                show_counts: true,
            }),
            "TagTabs" => Widget::TagTabs(TagTabsProps {
                cards: CardOptions::default(),
                title: s("Productos"),
                limit: 8,
                all_label: s("Todos"),
            }),
            "FeaturedProduct" => Widget::FeaturedProduct(FeaturedProductProps {
                cards: CardOptions::default(),
                product_id: String::new(),
                show_price: true,
            }),
            "Navbar" => Widget::Navbar(NavbarProps {
                menu: s("list"),
                // Vacío: muestra el nombre de Datos de la tienda.
                brand: String::new(),
                logo_url: s("/img/logo.svg"),
                links: vec![link("Inicio", "/")],
                button_label: String::new(),
                button_href: String::new(),
                variant: s("a"),
                tone: String::new(),
                upper: false,
                search: false,
                menu_color: false,
            }),
            "Carousel" => Widget::Carousel(CarouselProps {
                slides: (1..=3)
                    .map(|i| Slide {
                        image_url: format!("/img/slide-{i}.webp"),
                        title: s(["Novedades", "Ofertas", "Envíos"][i - 1]),
                        subtitle: s([
                            "Contá acá qué hay de nuevo",
                            "Contá acá qué está en oferta",
                            "Contá acá cómo y adónde enviás",
                        ][i - 1]),
                        button_label: s("Ver más"),
                        button_href: s("/"),
                    })
                    .collect(),
                autoplay: 5,
            }),
            "Categories" => Widget::Categories(CategoriesProps {
                title: s("Categorías"),
                items: [
                    ("Tecnología", "tecnologia"),
                    ("Hogar", "hogar"),
                    ("Moda", "moda"),
                    ("Deportes", "deportes"),
                ]
                .iter()
                .map(|(label, slug)| CategoryItem {
                    label: s(label),
                    image_url: format!("/img/category-{slug}.webp"),
                    href: format!("/categorias/{slug}"),
                })
                .collect(),
            }),
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
                ],
            }),
            "Countdown" => Widget::Countdown(CountdownProps {
                title: s("Ofertas relámpago"),
                subtitle: s("Contá acá qué está en oferta y hasta cuándo"),
                ends_at: s("2026-12-31T23:59"),
                ended_text: s("¡La oferta terminó!"),
                button_label: s("Ver ofertas"),
                button_href: s("/ofertas"),
            }),
            "Card" => Widget::Card(CardProps {
                image_url: s("/img/card.webp"),
                title: s("Novedades de la semana"),
                body: s("Contá acá qué hay de nuevo."),
                button_label: s("Ver más"),
                button_href: s("/"),
                layout: s("vertical"),
            }),
            "Tabs" => Widget::Tabs(TabsProps {
                tabs: vec![
                    TabItem {
                        label: s("Envíos"),
                        body: s("Contá acá cómo y adónde enviás, y cuánto tarda."),
                    },
                    TabItem {
                        label: s("Pagos"),
                        body: s("Contá acá cómo te pueden pagar."),
                    },
                    TabItem {
                        label: s("Cambios"),
                        body: s("Contá acá cómo son los cambios y las devoluciones."),
                    },
                ],
            }),
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
                ],
            }),
            "HoverGallery" => Widget::HoverGallery(HoverGalleryProps {
                images: (1..=4)
                    .map(|i| GalleryImage {
                        image_url: format!("/img/gallery-{i}.webp"),
                    })
                    .collect(),
                title: s("Nuestra colección"),
                subtitle: s("Tocá o pasá el mouse para ver más fotos"),
                href: String::new(),
            }),
            "Testimonials" => Widget::Testimonials(TestimonialsProps {
                title: s("Lo que dicen nuestros clientes"),
                // Sin nombres inventados: se completan con opiniones reales.
                items: (1..=3)
                    .map(|i| Testimonial {
                        name: format!("Cliente {i}"),
                        text: s("Copiá acá lo que te dijo un cliente real."),
                        rating: 5,
                    })
                    .collect(),
            }),
            "Logos" => Widget::Logos(LogosProps {
                title: s("Marcas que trabajamos"),
                items: [
                    "Nórdika",
                    "Altavista",
                    "Kumo",
                    "Brisa",
                    "Valle Sur",
                    "Oriente",
                ]
                .iter()
                .enumerate()
                .map(|(i, name)| LogoItem {
                    image_url: format!("/img/brand-{}.svg", i + 1),
                    alt: s(name),
                    href: String::new(),
                })
                .collect(),
            }),
            // Otra foto que la de la Tarjeta: agregadas juntas no se repiten.
            "Image" => Widget::Image(ImageProps {
                image_url: s("/img/category-hogar.webp"),
                alt: s("Imagen de ejemplo"),
                href: String::new(),
                fit: s("cover"),
            }),
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
                legal: String::new(),
            }),
            "Announcement" => Widget::Announcement(AnnouncementProps {
                variant: s("a"),
                items: vec![
                    AnnouncementItem {
                        text: s("Escribí acá un mensaje corto, por ejemplo tu envío gratis"),
                        link_label: s("Ver productos"),
                        link_href: s("/"),
                    },
                    AnnouncementItem {
                        text: s("Escribí acá otro mensaje, por ejemplo tus cuotas"),
                        link_label: String::new(),
                        link_href: String::new(),
                    },
                ],
                code: s("BIENVENIDA"),
                note: String::new(),
                coupon: String::new(),
            }),
            // El código se elige de los cupones de la tienda (lo completa el editor).
            "Coupon" => Widget::Coupon(CouponProps {
                code: String::new(),
                text: s("Usá este código y ahorrá en tu compra"),
                note: String::new(),
                coupon: String::new(),
            }),
            "Stats" => Widget::Stats(StatsProps {
                variant: s("a"),
                items: vec![
                    StatItem {
                        value: s(crate::STAT_PLACEHOLDER),
                        label: s("Ej.: clientes felices"),
                        icon: s("truck"),
                    },
                    StatItem {
                        value: s(crate::STAT_PLACEHOLDER),
                        label: s("Ej.: puntaje en reseñas"),
                        icon: s("star"),
                    },
                    StatItem {
                        value: s(crate::STAT_PLACEHOLDER),
                        label: s("Ej.: tiempo de entrega"),
                        icon: s("clock"),
                    },
                ],
            }),
            "Steps" => Widget::Steps(StepsProps {
                variant: s("a"),
                title: s("Comprar es muy fácil"),
                items: vec![
                    StepItem {
                        title: s("Elegí"),
                        text: s("Mirá el catálogo y sumá al carrito."),
                    },
                    StepItem {
                        title: s("Pagá"),
                        text: s("Contá acá cómo te pueden pagar."),
                    },
                    StepItem {
                        title: s("Lo preparamos"),
                        text: s("Contá acá cuánto tardás en despachar."),
                    },
                    StepItem {
                        title: s("Recibilo"),
                        text: s("Contá acá cómo les llega el pedido."),
                    },
                ],
            }),
            "ProductCarousel" => Widget::ProductCarousel(ProductCarouselProps {
                cards: CardOptions::default(),
                variant: s("a"),
                title: s("Lo más vendido"),
                limit: 8,
                link_label: String::new(),
                link_href: String::new(),
            }),
            "Social" => Widget::Social(SocialProps {
                variant: s("a"),
                title: s("Seguinos"),
                facebook: String::new(),
                tiktok: String::new(),
                label: s("¿Dudas? Escribinos"),
            }),
            "ProductImage" => Widget::ProductImage(ProductImageProps {
                fit: s("cover"),
                variant: s("a"),
                badge: String::new(),
                stock_note: true,
            }),
            "ProductName" => Widget::ProductName(ProductNameProps {}),
            "ProductPrice" => Widget::ProductPrice(ProductPriceProps {
                variant: s("b"),
                show_installments: true,
                show_transfer: true,
            }),
            "ProductDescription" => Widget::ProductDescription(ProductDescriptionProps {}),
            "ProductButton" => Widget::ProductButton(ProductButtonProps {
                label: s("Agregar al carrito"),
                href: String::new(),
            }),
            _ => return None,
        };
        Some(widget)
    }
}
