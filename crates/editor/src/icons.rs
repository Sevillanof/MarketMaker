//! Íconos de línea (24×24, trazo `currentColor`) del editor.

use leptos::prelude::*;

/// Trazo SVG de cada ícono. Un nombre desconocido dibuja un cuadrado.
fn path(name: &str) -> &'static str {
    match name {
        // Acciones
        "undo" => "M9 14L4 9l5-5M4 9h11a5 5 0 0 1 0 10h-3",
        "redo" => "M15 14l5-5-5-5M20 9H9a5 5 0 0 0 0 10h3",
        "chevron-down" => "M6 9l6 6 6-6",
        "arrow-left" => "M19 12H5M11 18l-6-6 6-6",
        "pencil" => "M4 20h4L19 9l-4-4L4 16zM13.5 6.5l4 4",
        "up" => "M12 19V5M5 12l7-7 7 7",
        "down" => "M12 5v14M19 12l-7 7-7-7",
        "copy" => "M8 8h12v12H8zM4 16V4h12",
        "trash" => "M4 7h16M10 11v6M14 11v6M6 7l1 13h10l1-13M9 7V4h6v3",
        "external" => "M14 4h6v6M20 4l-9 9M18 14v6H4V6h6",
        "eye" => "M2 12s4-7 10-7 10 7 10 7-4 7-10 7S2 12 2 12zM12 9a3 3 0 1 0 0 6 3 3 0 0 0 0-6",
        "plus" => "M12 5v14M5 12h14",
        "search" => "M11 4a7 7 0 1 0 0 14 7 7 0 0 0 0-14M20 20l-4-4",
        "close" => "M6 6l12 12M18 6L6 18",
        "check" => "M5 12l5 5 9-10",
        "alert" => "M12 4l9 16H3zM12 10v4M12 17v.01",
        "info" => "M12 3a9 9 0 1 0 0 18 9 9 0 0 0 0-18M12 11v5M12 8v.01",
        "image" => "M4 5h16v14H4zM4 16l5-5 4 4 3-3 4 4M15 9v.01",
        "type" => "M4 7V5h16v2M12 5v14M9 19h6",
        "align-left" => "M4 6h16M4 10h10M4 14h16M4 18h10",
        "align-center" => "M4 6h16M7 10h10M4 14h16M7 18h10",
        "align-right" => "M4 6h16M10 10h10M4 14h16M10 18h10",
        "align-auto" => "M4 6h16M4 10h16M4 14h16M4 18h16",
        "star" => "M12 3l2.8 5.7 6.2.9-4.5 4.4 1 6.2L12 17.3 6.5 20.2l1-6.2L3 9.6l6.2-.9z",
        // Bloques
        "more" => "M5 10.75a1.25 1.25 0 1 1 0 2.5a1.25 1.25 0 1 1 0-2.5zM12 10.75a1.25 1.25 0 1 1 0 2.5a1.25 1.25 0 1 1 0-2.5zM19 10.75a1.25 1.25 0 1 1 0 2.5a1.25 1.25 0 1 1 0-2.5z",
        "tag" => "M3 12V4h8l10 10-8 8zM7.5 8v.01",
        "button" => "M3 8h18v8H3zM8 12h8",
        "modal" => "M3 4h18v16H3zM7 8h10v8H7z",
        "page" => "M6 3h8l4 4v14H6zM14 3v4h4M9 12h6M9 16h6",
        "navbar" => "M3 5h18v4H3zM6 7h3M14 7h2M18 7h1M3 13h18M3 17h12",
        "banner" => "M3 5h18v11H3zM7 9h8M7 12h5M3 20h18",
        "carousel" => "M6 6h12v10H6zM2 9v4M22 9v4M10 20h.01M12 20h.01M14 20h.01",
        "categories" => "M4 4h7v7H4zM13 4h7v7h-7zM4 13h7v7H4zM13 13h7v7h-7z",
        "grid" => "M3 4h5v6H3zM10 4h5v6h-5zM17 4h4v6h-4zM3 14h5v6H3zM10 14h5v6h-5zM17 14h4v6h-4z",
        "featured" => "M4 4h9v16H4zM16 7h5M16 11h4M16 15h5",
        "benefits" => "M12 3l7 3v5c0 4.5-3 8-7 10-4-2-7-5.5-7-10V6zM9 12l2 2 4-4",
        "countdown" => "M12 4a9 9 0 1 0 0 18 9 9 0 0 0 0-18M12 8v5l3 2M9 1h6",
        "card" => "M4 3h16v18H4zM4 11h16M7 14h10M7 17h6",
        "text" => "M4 6h16M4 10h16M4 14h12M4 18h9",
        "tabs" => "M3 8h18v12H3zM3 4h6v4M9 4h6v4",
        "faq" => "M4 5h16v11H9l-5 4zM10 9a2 2 0 1 1 2 2v1M12 14v.01",
        "gallery" => "M3 6h14v12H3zM20 8v12H6M3 15l4-4 3 3 3-3 4 4",
        "testimonials" => "M4 5h16v11H9l-5 4zM8 9h8M8 12h5",
        "tags" => "M3 12V4h8l10 10-8 8L3 12zM7.5 8.5h.01",
        "logos" => "M4 8h4v8H4zM10 6h4v12h-4zM16 9h4v6h-4z",
        "footer" => "M3 4h18v16H3zM3 14h18M6 17h4M13 17h5",
        "contact" => "M3 5h18v14H3zM3 7l9 6 9-6",
        _ => "M4 4h16v16H4z",
    }
}

/// Ícono decorativo (`aria-hidden`): el texto del control lleva el nombre.
#[component]
pub fn Icon(name: &'static str, #[prop(optional, into)] class: String) -> impl IntoView {
    let class = if class.is_empty() {
        "inline-block h-4 w-4 shrink-0".to_string()
    } else {
        format!("inline-block shrink-0 {class}")
    };
    view! {
        <svg class=class viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true" focusable="false">
            <path d=path(name)/>
        </svg>
    }
}
