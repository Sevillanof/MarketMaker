//! Primitivas de presentación sobre clases de daisyUI.
//!
//! Las clases se escriben completas y literales para que Tailwind las encuentre.

use leptos::prelude::*;

/// Botón primario. Con `href` se dibuja como enlace. Con `disabled` (en el
/// editor) se ve igual pero no navega ni recibe el foco: el enlace queda en
/// `data-href` y el editor lleva a esa página al tocarlo (ver `MaybeLink`).
#[component]
pub fn Button(
    #[prop(into)] label: String,
    #[prop(optional, into)] href: Option<String>,
    #[prop(optional)] disabled: bool,
) -> impl IntoView {
    let href = href.filter(|h| !h.is_empty());
    match (href, disabled) {
        (Some(href), false) => view! {
            <a class="btn btn-primary mk-btn" href=href>{label}</a>
        }
        .into_any(),
        (href, true) => view! {
            <span class="btn btn-primary mk-btn" data-href=href>{label}</span>
        }
        .into_any(),
        (None, false) => view! {
            <button type="button" class="btn btn-primary mk-btn">{label}</button>
        }
        .into_any(),
    }
}

/// Tarjeta que ocupa todo el alto disponible y recorta lo que sobra.
#[component]
pub fn Card(children: Children) -> impl IntoView {
    view! {
        <div class="card card-sm mk-item bg-base-100 shadow-sm border border-base-300 h-full min-h-0 overflow-hidden">
            {children()}
        </div>
    }
}

/// Imagen que llena su contenedor sin deformarse.
///
/// Mientras carga se ve un skeleton de daisyUI; si falla, un aviso en lugar
/// del ícono de imagen rota. Los handlers `onload`/`onerror` van en línea en
/// el HTML para que funcionen también antes de que hidrate el WASM.
///
/// Las imágenes propias (`/img/*.webp`) se sirven en AVIF con WebP de
/// respaldo. No se puede arrastrar, para no interferir con el editor.
#[component]
pub fn Image(
    #[prop(into)] src: String,
    #[prop(into)] alt: String,
    /// `object-fit`: `"cover"` (por defecto) o `"contain"`.
    #[prop(optional)]
    contain: bool,
    /// Encuadre de la foto en este producto (punto a la vista y zoom).
    #[prop(optional_no_strip)]
    fit: Option<domain::PhotoFit>,
) -> impl IntoView {
    if src.is_empty() {
        return view! {
            <div class="mk-img h-full w-full bg-base-200" data-state="empty" role="img" aria-label=alt></div>
        }
        .into_any();
    }
    let class = if contain {
        "block h-full w-full object-contain"
    } else {
        "block h-full w-full object-cover"
    };
    view! {
        <div class="mk-img skeleton h-full w-full overflow-hidden rounded-none">
            <Picture src alt class style=fit.map(|f| f.css())/>
        </div>
    }
    .into_any()
}

/// `<picture>` con AVIF para las imágenes propias y la imagen original de
/// respaldo. Al cargar (o fallar) marca el `.skeleton` más cercano con
/// `data-state`, que apaga la animación (ver `style/tailwind.css`).
#[component]
pub fn Picture(
    #[prop(into)] src: String,
    #[prop(into)] alt: String,
    #[prop(into)] class: String,
    /// Estilo en línea del `<img>` (por ejemplo, el encuadre de la foto).
    #[prop(optional_no_strip)]
    style: Option<String>,
) -> impl IntoView {
    // Las fotos de Mis fotos vienen en tres anchos: el navegador elige.
    let (avif, webp) = match srcsets(&src) {
        Some((a, w)) => (Some(a), Some(w)),
        None => (avif_variant(&src), None),
    };
    let sizes = webp.as_ref().map(|_| PHOTO_SIZES);
    view! {
        <picture>
            {avif.map(|a| view! { <source type="image/avif" srcset=a sizes=sizes/> })}
            <img
                class=class
                style=style
                src=src
                srcset=webp
                sizes=sizes
                alt=alt
                draggable="false"
                loading="lazy"
                decoding="async"
                onload="var s=this.closest('.skeleton');if(s)s.dataset.state='ok'"
                onerror="var s=this.closest('.skeleton');if(s)s.dataset.state='error'"
            />
        </picture>
    }
}

/// Imágenes de la biblioteca propia (`public/img`), con una descripción para
/// mostrar en el selector del editor.
pub const LIBRARY_IMAGES: &[(&str, &str)] = &[
    ("/img/hero.webp", "Portada"),
    ("/img/banner.webp", "Banner de ofertas"),
    ("/img/card.webp", "Tarjeta"),
    ("/img/slide-1.webp", "Diapositiva 1"),
    ("/img/slide-2.webp", "Diapositiva 2"),
    ("/img/slide-3.webp", "Diapositiva 3"),
    ("/img/category-moda.webp", "Moda"),
    ("/img/category-tecnologia.webp", "Tecnología"),
    ("/img/category-hogar.webp", "Hogar"),
    ("/img/category-deportes.webp", "Deportes"),
    ("/img/gallery-1.webp", "Galería 1"),
    ("/img/gallery-2.webp", "Galería 2"),
    ("/img/gallery-3.webp", "Galería 3"),
    ("/img/gallery-4.webp", "Galería 4"),
    ("/img/product-1.webp", "Producto 1"),
    ("/img/product-2.webp", "Producto 2"),
    ("/img/product-3.webp", "Producto 3"),
    ("/img/product-4.webp", "Producto 4"),
    ("/img/product-5.webp", "Producto 5"),
    ("/img/product-6.webp", "Producto 6"),
    ("/img/product-7.webp", "Producto 7"),
    ("/img/product-8.webp", "Producto 8"),
    ("/img/logo.svg", "Logo de la tienda"),
    ("/img/brand-1.svg", "Marca 1"),
    ("/img/brand-2.svg", "Marca 2"),
    ("/img/brand-3.svg", "Marca 3"),
    ("/img/brand-4.svg", "Marca 4"),
    ("/img/brand-5.svg", "Marca 5"),
    ("/img/brand-6.svg", "Marca 6"),
];

/// Para `/img/x.webp` devuelve `/img/x.avif`: las imágenes propias se generan
/// en los dos formatos (ver `scripts/gen_images.py`).
pub fn avif_variant(src: &str) -> Option<String> {
    src.strip_prefix("/img/")
        .and_then(|rest| rest.strip_suffix(".webp"))
        .map(|name| format!("/img/{name}.avif"))
}

/// Qué ancho ocupa una foto en pantalla, para elegir entre los tres tamaños.
const PHOTO_SIZES: &str = "(max-width: 640px) 100vw, 50vw";

/// Para una foto de Mis fotos (`/media/<id>/960.webp`), los `srcset` AVIF y
/// WebP con sus tres anchos.
pub fn srcsets(src: &str) -> Option<(String, String)> {
    let base = domain::media_base(src)?;
    let set = |ext: &str| {
        domain::MEDIA_WIDTHS
            .iter()
            .map(|w| format!("{base}/{w}.{ext} {w}w"))
            .collect::<Vec<_>>()
            .join(", ")
    };
    Some((set("avif"), set("webp")))
}

/// Bloque de carga de daisyUI.
#[component]
pub fn Skeleton(#[prop(into, optional)] class: String) -> impl IntoView {
    let class = format!("skeleton {class}");
    view! { <div class=class></div> }
}

/// Skeleton de una página mientras llega del servidor: barra, banner y una
/// grilla de tarjetas, con la misma grilla que el marketplace.
#[component]
pub fn PageSkeleton() -> impl IntoView {
    view! {
        <div class="min-h-screen bg-base-200" aria-busy="true" aria-label="Cargando">
            <div class="mx-auto max-w-6xl p-4">
                <div class="mk-grid">
                    <div class="mk-node skeleton" style="--x:1; --y:1; --w:48; --h:12"></div>
                    <div class="mk-node skeleton" style="--x:1; --y:13; --w:48; --h:48"></div>
                    <div class="mk-node skeleton" style="--x:1; --y:61; --w:12; --h:48"></div>
                    <div class="mk-node skeleton" style="--x:13; --y:61; --w:12; --h:48"></div>
                    <div class="mk-node skeleton" style="--x:25; --y:61; --w:12; --h:48"></div>
                    <div class="mk-node skeleton" style="--x:37; --y:61; --w:12; --h:48"></div>
                </div>
            </div>
        </div>
    }
}

/// Ícono de línea (24×24) de la lista de beneficios y del panel. Si el nombre no existe
/// se dibuja una estrella.
#[component]
pub fn Icon(#[prop(into)] name: String, #[prop(into, optional)] class: String) -> impl IntoView {
    let d = match name.as_str() {
        "truck" => {
            "M3 6h11v9H3zM14 9h4l3 3v3h-7zM7 18.5a1.5 1.5 0 1 0 0-.01M17 18.5a1.5 1.5 0 1 0 0-.01"
        }
        "store" => "M4 10v10h16V10M3 4h18l-1.5 6h-15zM9.5 20v-5h5v5",
        "shield" => "M12 3l7 3v5c0 4.5-3 8-7 10-4-2-7-5.5-7-10V6zM9 12l2 2 4-4",
        "refresh" => "M4 12a8 8 0 0 1 14-5.3L20 9M20 4v5h-5M20 12a8 8 0 0 1-14 5.3L4 15M4 20v-5h5",
        "card" => "M3 6h18v12H3zM3 10h18M7 15h4",
        "chat" => "M4 5h16v11H9l-5 4z",
        "gift" => "M4 10h16v10H4zM3 7h18v3H3zM12 7v13M12 7c-2-4-6-3-5 0M12 7c2-4 6-3 5 0",
        "clock" => "M12 3a9 9 0 1 0 0 18 9 9 0 0 0 0-18M12 7v5l3 2",
        // Menú del panel.
        "home" => "M3 10l9-7 9 7v10h-6v-6H9v6H3z",
        "pencil" => "M4 20h4L19 9l-4-4L4 16zM13.5 6.5l4 4",
        "box" => "M21 8l-9-5-9 5v8l9 5 9-5zM3 8l9 5 9-5M12 13v8",
        "bag" => "M6 3L3 7v13h18V7l-3-4zM3 7h18M16 11a4 4 0 0 1-8 0",
        "photo" => "M3 3h18v18H3zM9 7a2 2 0 1 0 0 4 2 2 0 0 0 0-4M21 15l-5-5L5 21",
        "camera" => "M3 8h4l2-3h6l2 3h4v12H3zM12 10a3.5 3.5 0 1 0 0 7 3.5 3.5 0 0 0 0-7",
        "plus" => "M12 5v14M5 12h14",
        "check" => "M5 12l5 5 9-10",
        "chevron-down" => "M6 9l6 6 6-6",
        "external" => "M14 4h6v6M20 4l-9 9M18 14v6H4V6h6",
        "log-out" => "M9 21H4V3h5M16 17l5-5-5-5M21 12H9",
        // Mi cuenta.
        "user" => "M12 4a4 4 0 1 0 0 8 4 4 0 0 0 0-8M4 21c1-4 4-6 8-6s7 2 8 6",
        "mail" => "M3 5h18v14H3zM3 7l9 6 9-6",
        "key" => "M8 11a4 4 0 1 0 0 8 4 4 0 0 0 0-8M10.8 12.2L20 3M17 6l3 3M14 9l2 2",
        // Contacto de la tienda.
        "whatsapp" => "M3.5 20.5l1.3-4.2A8.5 8.5 0 1 1 8 19.3zM9 8c-.3 3.4 3.2 7 7 7l1-1.6-2.2-1.1-1 .9c-1.2-.5-2.4-1.7-2.9-2.9l.9-1L10.6 7z",
        "instagram" => "M8 3h8a5 5 0 0 1 5 5v8a5 5 0 0 1-5 5H8a5 5 0 0 1-5-5V8a5 5 0 0 1 5-5zM12 8a4 4 0 1 0 0 8 4 4 0 0 0 0-8M17.5 6.5v.01",
        "pin" => "M12 21s-7-6.2-7-11a7 7 0 0 1 14 0c0 4.8-7 11-7 11zM12 7.5a2.5 2.5 0 1 0 0 5 2.5 2.5 0 0 0 0-5",
        "arrow-right" => "M5 12h14M13 6l6 6-6 6",
        _ => "M12 3l2.8 5.7 6.2.9-4.5 4.4 1 6.2L12 17.3 6.5 20.2l1-6.2L3 9.6l6.2-.9z",
    };
    let class = format!("inline-block {class}");
    view! {
        <svg class=class viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
            <path d=d/>
        </svg>
    }
}

/// Precio con formato argentino a partir de centavos: `$ 12.345,00`.
#[component]
pub fn Price(cents: i64) -> impl IntoView {
    view! { <span class="mk-price font-semibold">{format_price(cents)}</span> }
}

/// Precio con el anterior tachado al lado, si hay oferta (`was` > `cents`).
#[component]
pub fn PriceWas(cents: i64, was: i64) -> impl IntoView {
    view! {
        <span class="inline-flex flex-wrap items-baseline gap-x-2">
            <Price cents/>
            {(was > cents).then(|| view! {
                <s class="mk-price-was text-[0.85em] opacity-55">
                    <span class="sr-only">"Antes "</span>{format_price(was)}
                </s>
            })}
        </span>
    }
}

/// Formatea centavos como `$ 12.345,00`.
pub fn format_price(cents: i64) -> String {
    let sign = if cents < 0 { "-" } else { "" };
    let cents = cents.unsigned_abs();
    let units = (cents / 100).to_string();
    let mut grouped = String::new();
    for (i, ch) in units.chars().enumerate() {
        if i > 0 && (units.len() - i).is_multiple_of(3) {
            grouped.push('.');
        }
        grouped.push(ch);
    }
    format!("{sign}$ {grouped},{:02}", cents % 100)
}

#[cfg(test)]
mod tests {
    use super::{avif_variant, format_price, srcsets};

    #[test]
    fn library_photos_get_three_widths() {
        let (avif, webp) = srcsets("/media/ab-12/960.webp").unwrap();
        assert_eq!(
            avif,
            "/media/ab-12/480.avif 480w, /media/ab-12/960.avif 960w, /media/ab-12/1600.avif 1600w"
        );
        assert!(webp.starts_with("/media/ab-12/480.webp 480w"));
        assert_eq!(srcsets("/img/hero.webp"), None);
    }

    #[test]
    fn avif_only_for_own_webp_images() {
        assert_eq!(
            avif_variant("/img/hero.webp").as_deref(),
            Some("/img/hero.avif")
        );
        assert_eq!(avif_variant("/img/brand-1.svg"), None);
        assert_eq!(avif_variant("https://example.com/a.webp"), None);
    }

    #[test]
    fn formats_argentine_prices() {
        assert_eq!(format_price(1_234_500), "$ 12.345,00");
        assert_eq!(format_price(5), "$ 0,05");
        assert_eq!(format_price(99_900), "$ 999,00");
        assert_eq!(format_price(123_456_789), "$ 1.234.567,89");
        assert_eq!(format_price(-100), "-$ 1,00");
    }
}
