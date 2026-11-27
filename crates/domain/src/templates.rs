//! Plantillas para arrancar una tienda nueva: además del diseño simple de
//! [`starter_doc`], tres diseños completos hechos solo con los bloques del
//! editor, para que la persona elija el que más se parece a lo que quiere y
//! cambie los textos y las fotos.
//!
//! Son diseños propios, inspirados en estilos comunes de tiendas online
//! (vidriera de moda, emprendimiento artesanal, tecnología en modo oscuro).

use crate::{
    BannerProps, BenefitItem, BenefitsProps, CardProps, FaqItem, FaqProps, FooterProps, LinkItem,
    Look, NavbarProps, Node, PageDoc, Pos, ProductGridProps, StoreStyle, TestimonialsProps,
    TextStyle, Typography, Widget, GRID_COLS,
};

/// Una plantilla que se puede elegir al crear la tienda.
#[derive(Debug, PartialEq)]
pub struct StoreTemplate {
    pub id: &'static str,
    pub name: &'static str,
    /// Una frase para que la persona sepa para qué sirve.
    pub hint: &'static str,
    /// Vista previa (`/img/plantillas/<id>.webp` y `.avif`).
    pub preview: &'static str,
}

/// Las plantillas, en el orden en que se muestran. La primera es la de
/// siempre y es la que se usa si no se elige ninguna.
pub const STORE_TEMPLATES: &[StoreTemplate] = &[
    StoreTemplate {
        id: "simple",
        name: "Simple",
        hint: "Lo justo para empezar: banner, productos y quiénes son.",
        preview: "/img/plantillas/simple",
    },
    StoreTemplate {
        id: "editorial",
        name: "Vidriera editorial",
        hint: "Portada con foto, categorías, productos, historia y preguntas.",
        preview: "/img/plantillas/editorial",
    },
    StoreTemplate {
        id: "vidriera",
        name: "Vidriera",
        hint: "Para ropa y accesorios: carrusel, categorías y ofertas.",
        preview: "/img/plantillas/vidriera",
    },
    StoreTemplate {
        id: "taller",
        name: "Taller",
        hint: "Para emprendimientos y productos hechos a mano.",
        preview: "/img/plantillas/taller",
    },
    StoreTemplate {
        id: "noche",
        name: "Noche",
        hint: "Fondo oscuro, para tecnología y productos modernos.",
        preview: "/img/plantillas/noche",
    },
];

/// El documento de una tienda nueva llamada `name` con la plantilla
/// `template` (un id de [`STORE_TEMPLATES`]). Si no se conoce, la simple.
pub fn template_doc(template: &str, name: &str) -> PageDoc {
    match template {
        "vidriera" => vidriera(name),
        "taller" => taller(name),
        "noche" => noche(name),
        "editorial" => editorial(name),
        _ => crate::starter_doc(name),
    }
}

// --- Ayudas para armar los documentos ---------------------------------------

fn s(v: &str) -> String {
    v.to_string()
}

fn link(label: &str, href: &str) -> LinkItem {
    LinkItem {
        label: s(label),
        href: s(href),
        ..LinkItem::default()
    }
}

/// Un bloque con su ancho (en columnas de [`GRID_COLS`]), su alto (filas de
/// 8 px; 0 = el de siempre), su diseño y su color.
struct Block {
    widget: Widget,
    w: u8,
    h: u8,
    look: Look,
    style: TextStyle,
}

fn block(widget: Widget) -> Block {
    Block {
        widget,
        w: GRID_COLS,
        h: 0,
        look: Look::default(),
        style: TextStyle::default(),
    }
}

impl Block {
    fn look(mut self, design: u8, color: &str) -> Self {
        self.look = Look {
            design,
            color: s(color),
            ..Look::default()
        };
        self
    }
}

/// Pone las filas una debajo de la otra; los bloques de una fila quedan uno
/// al lado del otro, todos con el alto del más alto.
fn place(rows: Vec<Vec<Block>>) -> Vec<Node> {
    let mut nodes = Vec::new();
    let mut y = 0u16;
    for row in rows {
        let height = row
            .iter()
            .map(|b| {
                if b.h == 0 {
                    b.widget.limits().default.h
                } else {
                    b.h
                }
            })
            .max()
            .unwrap_or(0);
        let mut x = 0u8;
        for b in row {
            let mut node = Node::new(format!("n{}", nodes.len() + 1), b.widget);
            node.size.w = b.w;
            node.size.h = height;
            node.pos = Pos { x, y };
            node.look = b.look;
            node.style = b.style;
            x += b.w;
            nodes.push(node);
        }
        y += u16::from(height);
    }
    nodes
}

/// Arma el documento: Inicio con `rows` y, detrás, las páginas modelo de
/// Contacto y Preguntas frecuentes con el mismo menú y pie.
fn doc(theme: &str, typography: Typography, style: StoreStyle, rows: Vec<Vec<Block>>) -> PageDoc {
    let mut d = PageDoc {
        theme: s(theme),
        typography,
        style,
        nodes: place(rows),
        ..PageDoc::default()
    };
    for (path, name) in [
        ("/contacto", "Contacto"),
        ("/preguntas-frecuentes", "Preguntas frecuentes"),
    ] {
        d.add_page_with(s(path), s(name), 0, crate::template_blocks(path));
    }
    d
}

fn fonts(heading: &str, body: &str) -> Typography {
    Typography {
        heading: s(heading),
        body: s(body),
        ..Typography::default()
    }
}

fn shape(buttons: &str, corners: &str, background: &str) -> StoreStyle {
    StoreStyle {
        buttons: s(buttons),
        corners: s(corners),
        background: s(background),
        ..StoreStyle::default()
    }
}

fn navbar(links: Vec<LinkItem>, button: (&str, &str)) -> Widget {
    Widget::Navbar(NavbarProps {
        // Vacío: muestra el nombre y el logo de "Datos de la tienda".
        brand: String::new(),
        logo_url: String::new(),
        links,
        button_label: s(button.0),
        button_href: s(button.1),
        menu: s("list"),
        upper: false,
        search: false,
        size: 26,
        bg: String::new(),
        ink: String::new(),
    })
}

fn benefit(icon: &str, title: &str, text: &str) -> BenefitItem {
    BenefitItem {
        icon: s(icon),
        title: s(title),
        text: s(text),
    }
}

fn category(label: &str, image: &str) -> crate::CategoryItem {
    crate::CategoryItem {
        label: s(label),
        image_url: s(image),
        href: s("/#productos"),
    }
}

// --- Las plantillas -----------------------------------------------------------

/// Cómo cambia cada plantilla la tienda de ejemplo: el tema, las letras, la
/// forma de botones y esquinas, y el diseño (1 a 3) de cada bloque.
struct Variant {
    theme: &'static str,
    fonts: (&'static str, &'static str),
    shape: (&'static str, &'static str, &'static str),
    announcement: u8,
    navbar: u8,
    hero: u8,
    benefits: u8,
    categories: u8,
    products: u8,
    story: u8,
    testimonial: u8,
    faq: u8,
    footer: u8,
    /// Título del bloque de productos y de la portada (cambian el tono).
    hero_title: &'static str,
    hero_kicker: &'static str,
    products_title: &'static str,
}

const EJEMPLO: Variant = Variant {
    theme: "linea",
    fonts: ("", ""),
    shape: ("", "", "base"),
    announcement: 1,
    navbar: 1,
    hero: 1,
    benefits: 1,
    categories: 1,
    products: 2,
    story: 1,
    testimonial: 1,
    faq: 1,
    footer: 1,
    hero_title: "Los buenos momentos están en los detalles.",
    hero_kicker: "Una selección para vos",
    products_title: "Hechos para acompañarte",
};

/// La tienda de ejemplo: anuncio, menú, portada, beneficios, categorías con
/// foto, productos, historia, cita, preguntas y pie, en el diseño `v`.
/// Todos los bloques se pueden cambiar o borrar.
fn example_store(name: &str, v: &Variant) -> PageDoc {
    let typography = if v.fonts.0.is_empty() {
        Typography::default()
    } else {
        fonts(v.fonts.0, v.fonts.1)
    };
    let announcement =
        Widget::defaults("Announcement").expect("Announcement tiene valores por defecto");
    let mut d = doc(
        v.theme,
        typography,
        shape(v.shape.0, v.shape.1, v.shape.2),
        vec![
            vec![block(announcement).look(v.announcement, "")],
            vec![block(navbar(
                vec![
                    link("Inicio", "/"),
                    link("Productos", "/#productos"),
                    link("Preguntas frecuentes", "/preguntas-frecuentes"),
                    link("Contacto", "/contacto"),
                ],
                ("", ""),
            ))
            .look(v.navbar, "")],
            vec![block(Widget::Banner(BannerProps {
                kicker: s(v.hero_kicker),
                title: s(v.hero_title),
                subtitle: s("Una colección de objetos que te acompañan todos los días, a tu manera."),
                image_url: s("/img/category-hogar.webp"),
                button_label: s("Explorar la colección"),
                button_href: s("/#productos"),
                ..Default::default()
            }))
            .look(v.hero, "")],
            vec![block(Widget::Benefits(BenefitsProps {
                items: vec![
                    benefit("truck", "Envíos a todo el país", "Recibí donde estés"),
                    benefit("card", "Pagá como quieras", "Opciones para vos"),
                    benefit("refresh", "Cambios simples", "Comprá con tranquilidad"),
                ],
                ..Default::default()
            }))
            .look(v.benefits, "")],
            vec![block(Widget::Categories(crate::CategoriesProps {
                kicker: s("Encontrá tu próximo favorito"),
                title: s("Explorá por categoría"),
                show_all: true,
                items: vec![
                    category("Hogar", "/img/category-hogar.webp"),
                    category("Moda", "/img/category-moda.webp"),
                    category("Tecnología", "/img/category-tecnologia.webp"),
                ],
                ..Default::default()
            }))
            .look(v.categories, "")],
            vec![block(Widget::ProductGrid(ProductGridProps {
                kicker: s("Elegidos de la semana"),
                title: s(v.products_title),
                limit: 4,
                ..Default::default()
            }))
            .look(v.products, "")],
            vec![block(Widget::Card(CardProps {
                kicker: s("Nuestra historia"),
                image_url: s("/img/gallery-1.webp"),
                title: s("Pequeñas cosas, grandes momentos."),
                body: s("Creemos en elegir con intención. Cada objeto de nuestra colección está pensado para acompañarte."),
                button_label: s("Conocé más"),
                button_href: s("/contacto"),
                ..Default::default()
            }))
            .look(v.story, "")],
            vec![block(Widget::Testimonials(TestimonialsProps {
                items: vec![crate::Testimonial {
                    name: s("Cliente de ejemplo"),
                    text: s("Una experiencia simple, de esas a las que siempre querés volver."),
                    rating: 5,
                    note: s("Opinión de ejemplo"),
                }],
                ..Default::default()
            }))
            .look(v.testimonial, "")],
            vec![block(Widget::Faq(FaqProps {
                kicker: s("Estamos para ayudarte"),
                title: s("Preguntas frecuentes"),
                items: vec![
                    FaqItem {
                        question: s("¿Cómo recibo mi pedido?"),
                        answer: s("Contá acá cómo y cuándo llegan los pedidos."),
                    },
                    FaqItem {
                        question: s("¿Qué medios de pago aceptan?"),
                        answer: s("Contá acá con qué se puede pagar."),
                    },
                ],
                ..Default::default()
            }))
            .look(v.faq, "")],
            vec![block(Widget::Footer(FooterProps {
                brand: String::new(),
                description: format!(
                    "Objetos para disfrutar lo cotidiano. Una tienda hecha para vos, {name}."
                ),
                links: vec![
                    LinkItem {
                        label: s("Explorar"),
                        children: vec![
                            link("Productos", "/#productos"),
                            link("Nosotros", "/contacto"),
                            link("Contacto", "/contacto"),
                        ],
                        ..LinkItem::default()
                    },
                    LinkItem {
                        label: s("Ayuda"),
                        children: vec![
                            link("Preguntas frecuentes", "/preguntas-frecuentes"),
                            link("Contacto", "/contacto"),
                        ],
                        ..LinkItem::default()
                    },
                ],
                ..Default::default()
            }))
            .look(v.footer, "")],
        ],
    );
    d.theme = s(v.theme);
    d
}

/// Vidriera editorial: la tienda de ejemplo tal cual, con el tema Línea.
fn editorial(name: &str) -> PageDoc {
    example_store(name, &EJEMPLO)
}

/// Vidriera: fotos grandes y mucho producto, como una tienda de moda.
fn vidriera(name: &str) -> PageDoc {
    example_store(
        name,
        &Variant {
            theme: "dalia",
            fonts: ("montserrat", "inter"),
            shape: ("pill", "round", "base"),
            announcement: 2,
            navbar: 2,
            hero: 2,
            benefits: 2,
            categories: 3,
            products: 3,
            story: 2,
            testimonial: 2,
            faq: 2,
            footer: 2,
            hero_title: "Nueva temporada, nuevos favoritos.",
            hero_kicker: "Recién llegado",
            products_title: "Lo más elegido",
        },
    )
}

/// Taller: cálida y artesanal, para emprendimientos y cosas hechas a mano.
fn taller(name: &str) -> PageDoc {
    example_store(
        name,
        &Variant {
            theme: "cacao",
            fonts: ("nunito", "nunito"),
            shape: ("pill", "round", "primary"),
            announcement: 3,
            navbar: 3,
            hero: 3,
            benefits: 3,
            categories: 2,
            products: 2,
            story: 3,
            testimonial: 3,
            faq: 3,
            footer: 3,
            hero_title: "Hecho a mano, pensado para vos.",
            hero_kicker: "Piezas únicas",
            products_title: "Nuestros productos",
        },
    )
}

/// Noche: oscura y prolija, con los productos al frente.
fn noche(name: &str) -> PageDoc {
    example_store(
        name,
        &Variant {
            theme: "noche",
            fonts: ("oswald", "inter"),
            shape: ("square", "soft", "base"),
            announcement: 1,
            navbar: 1,
            hero: 2,
            benefits: 3,
            categories: 1,
            products: 1,
            story: 1,
            testimonial: 2,
            faq: 2,
            footer: 3,
            hero_title: "Tecnología que te acompaña.",
            hero_kicker: "Novedades",
            products_title: "Lo último",
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_template_is_valid_and_carries_the_name() {
        for id in STORE_TEMPLATES.iter().map(|t| t.id).chain(["editorial"]) {
            let d = template_doc(id, "La Huerta");
            assert!(
                crate::validate(&d).is_ok(),
                "{}: {:?}",
                id,
                crate::validate(&d)
            );
            let json = serde_json::to_string(&d).unwrap();
            assert!(json.contains("La Huerta"), "{}", id);
            // Quedan textos de ejemplo para que el editor avise al publicar.
            assert!(crate::placeholders_left(&d) >= 3, "{}", id);
        }
        assert_eq!(template_doc("no-existe", "X"), crate::starter_doc("X"));
    }

    #[test]
    fn blocks_do_not_overlap_and_fit_the_grid() {
        for t in STORE_TEMPLATES {
            let d = template_doc(t.id, "La Huerta");
            let mut layers = vec![d.nodes.clone()];
            layers.extend(d.pages.iter().map(|p| p.nodes.clone()));
            for nodes in layers {
                for n in &nodes {
                    let l = n.widget.limits();
                    assert!(n.pos.x + n.size.w <= GRID_COLS, "{}: {}", t.id, n.id);
                    assert!(
                        (l.min_w..=l.max_w).contains(&n.size.w)
                            && (l.min_h..=l.max_h).contains(&n.size.h),
                        "{}: {} {:?} fuera de {:?}",
                        t.id,
                        n.widget.type_name(),
                        n.size,
                        l
                    );
                }
                for (i, a) in nodes.iter().enumerate() {
                    for b in &nodes[i + 1..] {
                        let apart = a.pos.x + a.size.w <= b.pos.x
                            || b.pos.x + b.size.w <= a.pos.x
                            || a.pos.y + u16::from(a.size.h) <= b.pos.y
                            || b.pos.y + u16::from(b.size.h) <= a.pos.y;
                        assert!(apart, "{}: {} pisa a {}", t.id, a.id, b.id);
                    }
                }
            }
        }
    }

    #[test]
    fn templates_have_the_model_pages_with_the_shared_menu() {
        for t in STORE_TEMPLATES.iter().filter(|t| t.id != "simple") {
            let d = template_doc(t.id, "La Huerta");
            let paths: Vec<_> = d.pages.iter().map(|p| p.path.as_str()).collect();
            assert_eq!(paths, ["/contacto", "/preguntas-frecuentes"], "{}", t.id);
            let navbar = d
                .nodes
                .iter()
                .find(|n| n.widget.type_name() == "Navbar")
                .unwrap();
            assert!(!navbar.shared.is_empty(), "{}", t.id);
        }
    }
}
