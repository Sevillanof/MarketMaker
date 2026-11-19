//! Plantillas para arrancar una tienda nueva: además del diseño simple de
//! [`starter_doc`], tres diseños completos hechos solo con los bloques del
//! editor, para que la persona elija el que más se parece a lo que quiere y
//! cambie los textos y las fotos.
//!
//! Son diseños propios, inspirados en estilos comunes de tiendas online
//! (vidriera de moda, emprendimiento artesanal, tecnología en modo oscuro).

use crate::{
    BannerProps, BenefitItem, BenefitsProps, CardProps, CarouselProps, CategoriesProps,
    CategoryItem, CountdownProps, FaqItem, FaqProps, FooterProps, ImageProps, LinkItem, Look,
    NavbarProps, Node, PageDoc, Pos, ProductGridProps, Slide, StoreStyle, TabItem, TabsProps,
    TagSearchProps, TagTabsProps, TestimonialsProps, TextProps, TextStyle, Typography, Widget,
    GRID_COLS,
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
    fn w(mut self, w: u8) -> Self {
        self.w = w;
        self
    }
    fn h(mut self, h: u8) -> Self {
        self.h = h;
        self
    }
    fn look(mut self, design: u8, color: &str) -> Self {
        self.look = Look {
            design,
            color: s(color),
        };
        self
    }
    fn text(mut self, size: &str, align: &str) -> Self {
        self.style.size = s(size);
        self.style.align = s(align);
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
        variant: s("a"),
        tone: String::new(),
        upper: false,
        search: false,
        menu_color: false,
    })
}

fn footer(description: &str) -> Widget {
    Widget::Footer(FooterProps {
        brand: String::new(),
        description: s(description),
        links: vec![
            link("Inicio", "/"),
            link("Preguntas frecuentes", "/preguntas-frecuentes"),
            link("Contacto", "/contacto"),
        ],
        legal: String::new(),
    })
}

fn benefit(icon: &str, title: &str, text: &str) -> BenefitItem {
    BenefitItem {
        icon: s(icon),
        title: s(title),
        text: s(text),
    }
}

fn card(image: &str, title: &str, body: &str, button: (&str, &str), layout: &str) -> Widget {
    Widget::Card(CardProps {
        image_url: s(image),
        title: s(title),
        body: s(body),
        button_label: s(button.0),
        button_href: s(button.1),
        layout: s(layout),
    })
}

fn text(heading: &str, body: &str) -> Widget {
    Widget::Text(TextProps {
        heading: s(heading),
        body: s(body),
        align: s("left"),
    })
}

fn testimonials(title: &str) -> Widget {
    Widget::defaults("Testimonials")
        .map(|w| match w {
            Widget::Testimonials(p) => Widget::Testimonials(TestimonialsProps {
                title: s(title),
                ..p
            }),
            w => w,
        })
        .expect("Testimonials tiene valores por defecto")
}

fn categories(title: &str) -> Widget {
    Widget::Categories(CategoriesProps {
        title: s(title),
        items: [
            ("Moda", "moda"),
            ("Hogar", "hogar"),
            ("Tecnología", "tecnologia"),
            ("Deportes", "deportes"),
        ]
        .iter()
        .map(|(label, slug)| CategoryItem {
            label: s(label),
            image_url: format!("/img/category-{slug}.webp"),
            href: format!("/categorias/{slug}"),
        })
        .collect(),
    })
}

// --- Las plantillas -----------------------------------------------------------

/// Vidriera: clara, fotos grandes y mucho producto, como una tienda de moda.
fn vidriera(name: &str) -> PageDoc {
    let slide = |i: usize, title: &str, subtitle: &str, button: &str| Slide {
        image_url: format!("/img/slide-{i}.webp"),
        title: s(title),
        subtitle: s(subtitle),
        button_label: s(button),
        button_href: s("/#productos"),
    };
    doc(
        "light",
        fonts("montserrat", "inter"),
        shape("pill", "round", "base"),
        vec![
            vec![block(navbar(
                vec![
                    link("Inicio", "/"),
                    link("Preguntas frecuentes", "/preguntas-frecuentes"),
                    link("Contacto", "/contacto"),
                ],
                ("Ver productos", "/#productos"),
            ))],
            vec![block(Widget::Carousel(CarouselProps {
                slides: vec![
                    slide(
                        1,
                        &format!("Nueva temporada en {name}"),
                        "Escribí acá una frase sobre lo nuevo que llegó",
                        "Ver novedades",
                    ),
                    slide(
                        2,
                        "Hasta 30% off",
                        "Contá acá qué está en oferta y hasta cuándo",
                        "Ver ofertas",
                    ),
                    slide(
                        3,
                        "Envíos a todo el país",
                        "Contá acá cómo y adónde enviás",
                        "Comprar ahora",
                    ),
                ],
                autoplay: 6,
            }))
            .h(60)],
            vec![block(Widget::Benefits(BenefitsProps {
                items: vec![
                    benefit(
                        "truck",
                        "Envío a todo el país",
                        "Contá acá desde cuánto es gratis",
                    ),
                    benefit(
                        "card",
                        "Cuotas y transferencia",
                        "Contá acá cómo te pueden pagar",
                    ),
                    benefit(
                        "refresh",
                        "Cambios sin vueltas",
                        "Contá acá cuántos días tienen",
                    ),
                    benefit(
                        "gift",
                        "Listo para regalar",
                        "Contá acá si envolvés para regalo",
                    ),
                ],
            }))
            .look(2, "neutral")],
            vec![block(Widget::TagTabs(TagTabsProps {
                cards: crate::CardOptions::default(),
                title: s("Lo más elegido"),
                limit: 8,
                all_label: s("Todo"),
            }))],
            vec![block(categories("Comprá por categoría")).look(5, "")],
            vec![block(Widget::Countdown(CountdownProps {
                title: s("Ofertas de fin de temporada"),
                subtitle: s("Contá acá qué está en oferta y hasta cuándo"),
                ends_at: s("2026-12-31T23:59"),
                ended_text: s("La oferta terminó. ¡Gracias por comprar!"),
                button_label: s("Ver ofertas"),
                button_href: s("/#productos"),
            }))
            .look(4, "neutral")],
            vec![block(Widget::ProductGrid(ProductGridProps {
                cards: crate::CardOptions::default(),
                title: s("Novedades"),
                limit: 4,
            }))
            .h(60)],
            vec![
                block(card(
                    "/img/card.webp",
                    "¿Buscás un regalo?",
                    "Contá acá cómo pueden pedirte ayuda para elegir.",
                    ("Escribinos", "/contacto"),
                    "overlay",
                ))
                .w(24)
                .h(40),
                block(card(
                    "/img/banner.webp",
                    "Retiro en el local",
                    "Contá acá dónde y cuándo pueden retirar su compra.",
                    ("Cómo llegar", "/contacto"),
                    "overlay",
                ))
                .w(24)
                .h(40),
            ],
            vec![block(footer(crate::STARTER_PLACEHOLDERS[2]))],
        ],
    )
}

/// Taller: cálida y cercana, para contar la historia detrás de los productos.
fn taller(name: &str) -> PageDoc {
    doc(
        "cupcake",
        fonts("nunito", "nunito"),
        shape("pill", "round", "primary"),
        vec![
            vec![block(navbar(
                vec![
                    link("Inicio", "/"),
                    link("Preguntas frecuentes", "/preguntas-frecuentes"),
                ],
                ("Escribinos", "/contacto"),
            ))],
            vec![
                block(text(
                    &format!("Hecho a mano en {name}"),
                    "Escribí acá una frase sobre tu tienda: qué hacés, con qué materiales y para quién.",
                ))
                .w(22)
                .h(48)
                .look(5, "")
                .text("largest", "left"),
                block(Widget::Image(ImageProps {
                    image_url: s("/img/hero.webp"),
                    alt: s("Foto de ejemplo de la tienda"),
                    href: String::new(),
                    fit: s("cover"),
                }))
                .w(26)
                .h(48),
            ],
            vec![block(Widget::Benefits(BenefitsProps {
                items: vec![
                    benefit("star", "Piezas únicas", "Contá acá qué hace especial lo que hacés"),
                    benefit("clock", "Hecho por encargo", "Contá acá cuánto tarda un pedido"),
                    benefit("truck", "Envíos", "Contá acá cómo y adónde enviás"),
                    benefit("chat", "Te asesoramos", "Contá acá cómo pueden escribirte"),
                ],
            }))
            .look(3, "")],
            vec![block(Widget::ProductGrid(ProductGridProps {
                cards: crate::CardOptions::default(),
                title: s("Nuestros productos"),
                limit: 8,
            }))],
            vec![
                block(card("/img/gallery-1.webp", "Pedidos personalizados", "Contá acá cómo funcionan los pedidos a medida.", ("Consultar", "/contacto"), "vertical")).w(16).h(52).look(2, ""),
                block(card("/img/gallery-2.webp", "Regalos empresariales", "Contá acá qué ofrecés para empresas y eventos.", ("Consultar", "/contacto"), "vertical")).w(16).h(52).look(2, ""),
                block(card("/img/gallery-3.webp", "Talleres y clases", "Contá acá si das clases y cuándo son.", ("Consultar", "/contacto"), "vertical")).w(16).h(52).look(2, ""),
            ],
            vec![block(testimonials("Lo que dicen quienes nos compraron")).look(3, "secondary")],
            vec![block(text("Nuestra historia", crate::STARTER_PLACEHOLDERS[1])).h(24).text("", "center")],
            vec![block(card("/img/banner.webp", "¿Tenés una idea?", "Contá acá cómo pueden pedirte algo especial.", ("Escribinos", "/contacto"), "side")).h(40).look(4, "")],
            vec![block(footer(crate::STARTER_PLACEHOLDERS[2])).look(3, "")],
        ],
    )
}

/// Noche: oscura y prolija, con buscador y productos al frente.
fn noche(name: &str) -> PageDoc {
    doc(
        "dark",
        fonts("oswald", "inter"),
        shape("square", "soft", "base"),
        vec![
            vec![block(navbar(
                vec![
                    link("Inicio", "/"),
                    link("Preguntas frecuentes", "/preguntas-frecuentes"),
                    link("Contacto", "/contacto"),
                ],
                ("Comprar", "/#productos"),
            ))],
            vec![block(Widget::Banner(BannerProps {
                title: format!("{name}: tecnología que te acompaña"),
                subtitle: s(crate::STARTER_PLACEHOLDERS[0]),
                image_url: s("/img/banner.webp"),
                button_label: s("Ver productos"),
                button_href: s("/#productos"),
            }))
            .h(56)
            .text("larger", "center")],
            vec![block(Widget::TagSearch(TagSearchProps {
                placeholder: s("¿Qué estás buscando?"),
                show_counts: true,
            }))],
            vec![block(Widget::TagTabs(TagTabsProps {
                cards: crate::CardOptions::default(),
                title: s("Catálogo"),
                limit: 8,
                all_label: s("Todo"),
            }))],
            vec![
                block(card(
                    "/img/slide-1.webp",
                    "Lo último",
                    "Contá acá qué producto nuevo llegó.",
                    ("Ver más", "/#productos"),
                    "overlay",
                ))
                .w(24)
                .h(44)
                .look(2, "accent"),
                block(card(
                    "/img/slide-2.webp",
                    "Combos con descuento",
                    "Contá acá qué combos armaste.",
                    ("Ver combos", "/#productos"),
                    "overlay",
                ))
                .w(24)
                .h(44)
                .look(2, "accent"),
            ],
            vec![block(Widget::Benefits(BenefitsProps {
                items: vec![
                    benefit(
                        "shield",
                        "Garantía oficial",
                        "Contá acá cuánto dura la garantía",
                    ),
                    benefit(
                        "truck",
                        "Envío en el día",
                        "Contá acá adónde llegás en el día",
                    ),
                    benefit("card", "Hasta 6 cuotas", "Contá acá cómo te pueden pagar"),
                    benefit(
                        "chat",
                        "Soporte por WhatsApp",
                        "Contá acá en qué horarios respondés",
                    ),
                ],
            }))
            .look(5, "accent")],
            vec![
                block(Widget::Tabs(TabsProps {
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
                            label: s("Garantía"),
                            body: s("Contá acá qué cubre la garantía y cómo se usa."),
                        },
                    ],
                }))
                .w(24)
                .h(44),
                block(Widget::Faq(FaqProps {
                    title: s("Preguntas frecuentes"),
                    items: vec![
                        FaqItem {
                            question: s("¿Los productos son originales?"),
                            answer: s("Escribí acá la respuesta."),
                        },
                        FaqItem {
                            question: s("¿Hacen factura?"),
                            answer: s("Escribí acá la respuesta."),
                        },
                        FaqItem {
                            question: s("¿Puedo retirar en persona?"),
                            answer: s("Escribí acá la respuesta."),
                        },
                    ],
                }))
                .w(24)
                .h(44)
                .look(2, ""),
            ],
            vec![block(testimonials("Opiniones de clientes")).look(2, "")],
            vec![block(footer(crate::STARTER_PLACEHOLDERS[2]))],
        ],
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_template_is_valid_and_carries_the_name() {
        for t in STORE_TEMPLATES {
            let d = template_doc(t.id, "La Huerta");
            assert!(
                crate::validate(&d).is_ok(),
                "{}: {:?}",
                t.id,
                crate::validate(&d)
            );
            let json = serde_json::to_string(&d).unwrap();
            assert!(json.contains("La Huerta"), "{}", t.id);
            // Quedan textos de ejemplo para que el editor avise al publicar.
            assert!(crate::placeholders_left(&d) >= 3, "{}", t.id);
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
