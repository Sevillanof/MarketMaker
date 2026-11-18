//! El diseño con el que arranca una tienda nueva: menú con su nombre, banner,
//! productos, un texto para contar quiénes son y el pie. Los textos se ven
//! claramente como ejemplo, para cambiarlos antes de publicar.

use crate::{
    BannerProps, FooterProps, LinkItem, NavbarProps, Node, PageDoc, Pos, ProductGridProps,
    TextProps, Widget,
};

/// Textos del diseño inicial que la persona tiene que cambiar. Si alguno
/// sigue igual al publicar, el editor avisa.
pub const STARTER_PLACEHOLDERS: &[&str] = &[
    "Escribí acá una frase sobre tu tienda",
    "Contá acá quiénes son, qué venden y cómo trabajan.",
    "Una frase corta sobre tu tienda.",
    "Contá acá cómo pueden escribirte y en qué horarios respondés.",
];

/// Páginas que se crean con "+ Nueva página": dirección y nombre.
pub const PAGE_TEMPLATES: &[(&str, &str)] = &[
    ("/quienes-somos", "Quiénes somos"),
    ("/contacto", "Contacto"),
    ("/preguntas-frecuentes", "Preguntas frecuentes"),
];

/// Los bloques con los que arranca la página modelo `path` (entre el menú y
/// el pie). Vacío si no es una de [`PAGE_TEMPLATES`].
pub fn template_blocks(path: &str) -> Vec<Widget> {
    let text = |heading: &str, body: &str| {
        Widget::Text(TextProps {
            heading: heading.to_string(),
            body: body.to_string(),
            align: "left".to_string(),
        })
    };
    match path {
        "/quienes-somos" => vec![text("Quiénes somos", STARTER_PLACEHOLDERS[1])],
        "/contacto" => Widget::defaults("Contact").into_iter().collect(),
        "/preguntas-frecuentes" => Widget::defaults("Faq").into_iter().collect(),
        _ => Vec::new(),
    }
}

/// El documento de una tienda nueva llamada `name`.
pub fn starter_doc(name: &str) -> PageDoc {
    let s = |v: &str| v.to_string();
    let widgets = vec![
        Widget::Navbar(NavbarProps {
            brand: String::new(),
            logo_url: String::new(),
            menu: s("list"),
            links: vec![LinkItem {
                label: s("Inicio"),
                href: s("/"),
                ..LinkItem::default()
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
            brand: String::new(),
            description: s(STARTER_PLACEHOLDERS[2]),
            links: Vec::new(),
            legal: String::new(),
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

/// Un bloque con un texto de ejemplo sin cambiar.
#[derive(Clone, Debug, PartialEq)]
pub struct PlaceholderNode {
    pub id: String,
    pub layer: crate::Layer,
    /// El bloque, por ejemplo "Banner".
    pub widget: &'static str,
    /// El primer texto de ejemplo que tiene.
    pub text: String,
}

/// Los bloques de `doc` que tienen textos de ejemplo sin cambiar, en el
/// orden de la tienda (Inicio primero), para llevar a cada uno desde el
/// aviso de antes de publicar.
pub fn placeholder_nodes(doc: &PageDoc) -> Vec<PlaceholderNode> {
    fn first(v: &serde_json::Value) -> Option<String> {
        match v {
            serde_json::Value::String(s) if is_placeholder(s) => Some(s.trim().to_string()),
            serde_json::Value::Array(a) => a.iter().find_map(first),
            serde_json::Value::Object(o) => o.values().find_map(first),
            _ => None,
        }
    }
    let mut layers: Vec<crate::Layer> = (0..=doc.pages.len()).map(crate::Layer::Page).collect();
    layers.push(crate::Layer::ProductModal);
    let mut out = Vec::new();
    for layer in layers {
        for n in doc.layer(layer) {
            if let Some(text) = serde_json::to_value(&n.widget)
                .ok()
                .as_ref()
                .and_then(first)
            {
                out.push(PlaceholderNode {
                    id: n.id.clone(),
                    layer,
                    widget: n.widget.label(),
                    text,
                });
            }
        }
    }
    out
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
        let nodes = placeholder_nodes(&doc);
        assert_eq!(nodes.len(), 3, "{nodes:?}");
        assert!(nodes.iter().all(|n| n.layer == crate::Layer::Page(0)));
        let mut faq = doc.clone();
        faq.nodes
            .push(Node::new("f", Widget::defaults("Faq").unwrap()));
        assert_eq!(placeholders_left(&faq), 6);
        // Las páginas modelo quedan entre el menú y el pie.
        for (path, name) in PAGE_TEMPLATES {
            let mut d = doc.clone();
            let i = d.add_page_with(path.to_string(), name.to_string(), 0, template_blocks(path));
            let mut types: Vec<_> = d
                .layer(crate::Layer::Page(i))
                .iter()
                .map(|n| (n.pos.y, n.widget.type_name()))
                .collect();
            types.sort();
            let types: Vec<_> = types.into_iter().map(|(_, t)| t).collect();
            assert_eq!(types.len(), 3, "{path}");
            assert_eq!((types[0], types[2]), ("Navbar", "Footer"), "{path}");
            assert!(
                crate::validate(&d).is_ok(),
                "{path}: {:?}",
                crate::validate(&d)
            );
            assert!(placeholders_left(&d) > placeholders_left(&doc), "{path}");
        }
        // Contacto usa el bloque que toma los datos de la tienda.
        assert_eq!(template_blocks("/contacto")[0].type_name(), "Contact");
        // Nada se pisa: cada bloque empieza donde termina el anterior.
        for pair in doc.nodes.windows(2) {
            assert_eq!(pair[0].pos.y + u16::from(pair[0].size.h), pair[1].pos.y);
        }
    }
}
