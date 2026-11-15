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
    Footer(FooterProps),
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
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
pub struct ProductNameProps {}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
pub struct ProductPriceProps {}

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
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct FeaturedProductProps {
    pub product_id: String,
    pub show_price: bool,
}

/// Enlace con texto, usado en la navbar y el footer.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct LinkItem {
    pub label: String,
    pub href: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct NavbarProps {
    pub brand: String,
    pub logo_url: String,
    pub links: Vec<LinkItem>,
    pub button_label: String,
    pub button_href: String,
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

/// Tipos de widget disponibles, con su nombre para mostrar, en el orden de la paleta.
pub const WIDGET_TYPES: &[(&str, &str)] = &[
    ("Navbar", "Menú superior"),
    ("Banner", "Banner"),
    ("Carousel", "Carrusel"),
    ("Categories", "Categorías"),
    ("ProductGrid", "Cuadrícula de productos"),
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
    ("Footer", "Pie de página"),
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
        "Cantidad de productos",
        FieldKind::Number { min: 1, max: 12 },
    ),
];
const FEATURED_PRODUCT_FIELDS: &[Field] = &[
    field("product_id", "Producto", FieldKind::ProductRef),
    adv("show_price", "Mostrar precio", FieldKind::Bool),
];

const LINK_ITEM: &[Field] = &[
    field("label", "Texto", text(30)),
    field("href", "Enlace", FieldKind::Url),
];
const NAVBAR_FIELDS: &[Field] = &[
    field("brand", "Nombre de la tienda", text(40)),
    field("logo_url", "Logo", FieldKind::Url),
    field(
        "links",
        "Enlaces",
        FieldKind::List {
            item: LINK_ITEM,
            min: 0,
            max: 8,
        },
    ),
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

const PRODUCT_IMAGE_FIELDS: &[Field] = &[field(
    "fit",
    "Ajuste",
    FieldKind::Select {
        options: IMAGE_FIT_OPTIONS,
    },
)];
/// El botón de la ficha agrega al carrito: ya no lleva enlace (`href`
/// queda solo para leer fichas viejas).
const PRODUCT_BUTTON_FIELDS: &[Field] = &[field("label", "Texto del botón", text(30))];

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
            Widget::Footer(_) => "Footer",
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
            Widget::ProductGrid(_) => limits(12, 48, 36, 144, 48, 60),
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
            Widget::Footer(_) => limits(24, 48, 24, 60, 48, 24),
            Widget::ProductImage(_) => limits(8, 48, 12, 120, 24, 52),
            Widget::ProductName(_) => limits(8, 48, 5, 24, 24, 9),
            Widget::ProductPrice(_) => limits(6, 48, 4, 16, 24, 6),
            Widget::ProductDescription(_) => limits(8, 48, 6, 96, 24, 28),
            Widget::ProductButton(_) => limits(8, 48, 6, 12, 24, 7),
        }
    }

    pub fn fields(&self) -> &'static [Field] {
        match self {
            Widget::Banner(_) => BANNER_FIELDS,
            Widget::Text(_) => TEXT_FIELDS,
            Widget::ProductGrid(_) => PRODUCT_GRID_FIELDS,
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
            Widget::Footer(_) => FOOTER_FIELDS,
            Widget::ProductImage(_) => PRODUCT_IMAGE_FIELDS,
            Widget::ProductName(_) | Widget::ProductPrice(_) | Widget::ProductDescription(_) => &[],
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
        };
        let widget = match type_name {
            "Banner" => Widget::Banner(BannerProps {
                title: s("Bienvenidos a nuestra tienda"),
                subtitle: s("Encontrá lo que buscás al mejor precio"),
                image_url: s("/img/banner.webp"),
                button_label: String::new(),
                button_href: String::new(),
            }),
            "Text" => Widget::Text(TextProps {
                heading: s("Sobre nosotros"),
                body: s("Somos una tienda local con envíos a todo el país."),
                align: s("left"),
            }),
            "ProductGrid" => Widget::ProductGrid(ProductGridProps {
                title: s("Productos"),
                limit: 4,
            }),
            "FeaturedProduct" => Widget::FeaturedProduct(FeaturedProductProps {
                product_id: String::new(),
                show_price: true,
            }),
            "Navbar" => Widget::Navbar(NavbarProps {
                brand: s("Mi tienda"),
                logo_url: s("/img/logo.svg"),
                links: vec![
                    link("Inicio", "/"),
                    link("Ofertas", "/ofertas"),
                    link("Categorías", "/categorias"),
                    link("Contacto", "/contacto"),
                ],
                button_label: String::new(),
                button_href: String::new(),
            }),
            "Carousel" => Widget::Carousel(CarouselProps {
                slides: (1..=3)
                    .map(|i| Slide {
                        image_url: format!("/img/slide-{i}.webp"),
                        title: s(["Nueva temporada", "Hasta 30% off", "Envío gratis"][i - 1]),
                        subtitle: s([
                            "Descubrí lo último en tecnología y hogar",
                            "En productos seleccionados por tiempo limitado",
                            "En compras desde $ 50.000 a todo el país",
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
                        title: s("Envío gratis"),
                        text: s("En compras desde $ 50.000"),
                    },
                    BenefitItem {
                        icon: s("card"),
                        title: s("Cuotas sin interés"),
                        text: s("Con todas las tarjetas"),
                    },
                    BenefitItem {
                        icon: s("refresh"),
                        title: s("Devolución gratis"),
                        text: s("Tenés 30 días para cambiarlo"),
                    },
                    BenefitItem {
                        icon: s("shield"),
                        title: s("Compra protegida"),
                        text: s("Tus datos siempre seguros"),
                    },
                ],
            }),
            "Countdown" => Widget::Countdown(CountdownProps {
                title: s("Ofertas relámpago"),
                subtitle: s("Aprovechá los descuentos antes de que terminen"),
                ends_at: s("2026-12-31T23:59"),
                ended_text: s("¡La oferta terminó!"),
                button_label: s("Ver ofertas"),
                button_href: s("/ofertas"),
            }),
            "Card" => Widget::Card(CardProps {
                image_url: s("/img/card.webp"),
                title: s("Novedades de la semana"),
                body: s("Conocé los productos que acaban de llegar."),
                button_label: s("Ver más"),
                button_href: s("/"),
                layout: s("vertical"),
            }),
            "Tabs" => Widget::Tabs(TabsProps {
                tabs: vec![
                    TabItem {
                        label: s("Envíos"),
                        body: s("Enviamos a todo el país en 24 a 72 horas hábiles."),
                    },
                    TabItem {
                        label: s("Pagos"),
                        body: s("Aceptamos tarjetas de crédito, débito y transferencias."),
                    },
                    TabItem {
                        label: s("Cambios"),
                        body: s("Tenés 30 días para cambiar o devolver tu compra."),
                    },
                ],
            }),
            "Faq" => Widget::Faq(FaqProps {
                title: s("Preguntas frecuentes"),
                items: vec![
                    FaqItem {
                        question: s("¿Cuánto tarda el envío?"),
                        answer: s("Entre 24 y 72 horas hábiles según tu zona."),
                    },
                    FaqItem {
                        question: s("¿Puedo pagar en cuotas?"),
                        answer: s("Sí, hasta 6 cuotas sin interés con tarjetas seleccionadas."),
                    },
                    FaqItem {
                        question: s("¿Cómo hago un cambio?"),
                        answer: s("Escribinos desde Contacto y te indicamos los pasos."),
                    },
                ],
            }),
            "HoverGallery" => Widget::HoverGallery(HoverGalleryProps {
                images: (1..=4)
                    .map(|i| GalleryImage {
                        image_url: format!("/img/gallery-{i}.webp"),
                    })
                    .collect(),
                title: s("Zapatillas running"),
                subtitle: s("Pasá el mouse para ver más fotos"),
                href: String::new(),
            }),
            "Testimonials" => Widget::Testimonials(TestimonialsProps {
                title: s("Lo que dicen nuestros clientes"),
                items: vec![
                    Testimonial {
                        name: s("Lucía G."),
                        text: s("Llegó rapidísimo y muy bien embalado. ¡Recomendable!"),
                        rating: 5,
                    },
                    Testimonial {
                        name: s("Martín R."),
                        text: s("Buena atención y precios competitivos."),
                        rating: 4,
                    },
                    Testimonial {
                        name: s("Sofía P."),
                        text: s("Ya es mi tercera compra. Todo perfecto."),
                        rating: 5,
                    },
                ],
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
            "Image" => Widget::Image(ImageProps {
                image_url: s("/img/card.webp"),
                alt: s("Imagen de ejemplo"),
                href: String::new(),
                fit: s("cover"),
            }),
            "Footer" => Widget::Footer(FooterProps {
                brand: s("Mi tienda"),
                description: s("Productos seleccionados con envío a todo el país."),
                links: vec![
                    link("Quiénes somos", "/nosotros"),
                    link("Envíos", "/envios"),
                    link("Cambios y devoluciones", "/cambios"),
                    link("Contacto", "/contacto"),
                ],
                legal: s("© 2026 Mi tienda. Todos los derechos reservados."),
            }),
            "ProductImage" => Widget::ProductImage(ProductImageProps { fit: s("cover") }),
            "ProductName" => Widget::ProductName(ProductNameProps {}),
            "ProductPrice" => Widget::ProductPrice(ProductPriceProps {}),
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
