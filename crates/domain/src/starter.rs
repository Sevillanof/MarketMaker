//! El diseño con el que arranca una tienda nueva: menú con su nombre, banner,
//! productos, un texto para contar quiénes son y el pie. Los textos se ven
//! claramente como ejemplo, para cambiarlos antes de publicar.

use crate::{
    BannerProps, FooterProps, LinkItem, Node, NavbarProps, PageDoc, Pos, ProductGridProps,
    TextProps, Widget,
};

/// Textos del diseño inicial que la persona tiene que cambiar. Si alguno
/// sigue igual al publicar, el editor avisa.
pub const STARTER_PLACEHOLDERS: &[&str] = &[
    "Escribí acá una frase sobre tu tienda",
    "Contá acá quiénes son, qué venden y cómo trabajan.",
    "Una frase corta sobre tu tienda.",
];

/// El documento de una tienda nueva llamada `name`.
pub fn starter_doc(name: &str) -> PageDoc {
    let s = |v: &str| v.to_string();
    let widgets = vec![
        Widget::Navbar(NavbarProps {
            brand: s(name),
            logo_url: String::new(),
            links: vec![LinkItem {
                label: s("Inicio"),
                href: s("/"),
            }],
            button_label: String::new(),
            button_href: String::new(),
        }),
        Widget::Banner(BannerProps {
            title: format!("Bienvenidos a {name}"),
            subtitle: s(STARTER_PLACEHOLDERS[0]),
            image_url: s("/img/banner.webp"),
            button_label: String::new(),
            button_href: String::new(),
        }),
        Widget::ProductGrid(ProductGridProps {
            title: s("Productos"),
            limit: 8,
        }),
        Widget::Text(TextProps {
            heading: s("Quiénes somos"),
            body: s(STARTER_PLACEHOLDERS[1]),
            align: s("left"),
        }),
        Widget::Footer(FooterProps {
            brand: s(name),
            description: s(STARTER_PLACEHOLDERS[2]),
            links: Vec::new(),
            legal: format!("© {name}"),
        }),
    ];
    let mut y = 0u16;
    let nodes = widgets
        .into_iter()
        .enumerate()
        .map(|(i, widget)| {
            let mut node = Node::new(format!("n{}", i + 1), widget);
            node.size.w = crate::GRID_COLS;
            node.pos = Pos { x: 0, y };
            y += u16::from(node.size.h);
            node
        })
        .collect();
    PageDoc {
        nodes,
        ..PageDoc::default()
    }
}

/// ¿Es un texto de ejemplo de los bloques nuevos o del diseño inicial?
pub fn is_placeholder(text: &str) -> bool {
    let t = text.trim();
    ["Contá acá", "Escribí acá", "Copiá acá"]
        .iter()
        .any(|p| t.starts_with(p))
        || STARTER_PLACEHOLDERS.contains(&t)
        || t.ends_with("Nombre de tu tienda")
}

/// Cuántos textos de ejemplo siguen sin cambiar en `doc` (en todas sus
/// páginas), para avisar antes de publicar.
pub fn placeholders_left(doc: &PageDoc) -> usize {
    fn walk(v: &serde_json::Value, n: &mut usize) {
        match v {
            serde_json::Value::String(s) if is_placeholder(s) => *n += 1,
            serde_json::Value::Array(a) => a.iter().for_each(|v| walk(v, n)),
            serde_json::Value::Object(o) => o.values().for_each(|v| walk(v, n)),
            _ => {}
        }
    }
    let mut n = 0;
    walk(&serde_json::to_value(doc).unwrap_or_default(), &mut n);
    n
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_starter_is_valid_and_carries_the_name() {
        let doc = starter_doc("La Huerta");
        assert!(crate::validate(&doc).is_ok(), "{:?}", crate::validate(&doc));
        let json = serde_json::to_string(&doc).unwrap();
        assert!(json.contains("La Huerta"));
        assert!(!json.contains("Mi tienda"));
        assert_eq!(placeholders_left(&doc), 3);
        let mut faq = doc.clone();
        faq.nodes.push(Node::new("f", Widget::defaults("Faq").unwrap()));
        assert_eq!(placeholders_left(&faq), 6);
        // Nada se pisa: cada bloque empieza donde termina el anterior.
        for pair in doc.nodes.windows(2) {
            assert_eq!(pair[0].pos.y + u16::from(pair[0].size.h), pair[1].pos.y);
        }
    }
}
