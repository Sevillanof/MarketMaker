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
    /// Foto de la primera pantalla (banner de arriba): se pide enseguida y
    /// con prioridad, sin esperar a que el navegador decida si está a la vista.
    #[prop(optional)]
    eager: bool,
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
            <Picture src alt class style=fit.map(|f| f.css()) eager/>
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
    /// Ver [`Image`]: sin carga diferida y con prioridad alta.
    #[prop(optional)]
    eager: bool,
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
                loading=if eager { "eager" } else { "lazy" }
                fetchpriority=eager.then_some("high")
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

/// Una línea de texto de skeleton: `w` es la clase de ancho (`w-1/3`…).
fn sk_line(w: &'static str) -> impl IntoView {
    view! { <Skeleton class=format!("h-3.5 {w}")/> }
}

/// Una tarjeta blanca del panel para armar skeletons.
fn sk_card(class: &'static str, children: Children) -> impl IntoView {
    view! { <div class=format!("flex flex-col gap-3 rounded-box border border-base-300 bg-base-100 p-4 {class}")>{children()}</div> }
}

/// Encabezado de una página del panel: título, texto y, a la derecha, botones.
fn sk_head(buttons: usize) -> impl IntoView {
    view! {
        <div class="flex items-center gap-3">
            <div class="flex grow flex-col gap-2">
                <Skeleton class="h-8 w-56"/>
                {sk_line("w-80 max-w-full")}
            </div>
            {(0..buttons).map(|_| view! { <Skeleton class="h-10 w-32"/> }).collect_view()}
        </div>
    }
}

/// Los skeletons del panel: cada uno copia el diseño de su pantalla (mismas
/// medidas y huecos) para que al llegar los datos nada se mueva.
/// Regla del proyecto: si cambia el diseño de una pantalla, se revisa su skeleton.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PanelSkeletonKind {
    /// Inicio: encabezado, cuatro números, últimos pedidos y checklist.
    Home,
    /// Productos: buscador y filas con foto.
    List,
    /// Pedidos: cuatro columnas con tarjetas.
    Board,
    /// Cobros, Entregas, Datos y Mi cuenta: formulario en una tarjeta.
    Form,
    /// Mis fotos: grilla de fotos.
    Photos,
}

#[component]
pub fn PanelSkeleton(kind: PanelSkeletonKind) -> impl IntoView {
    let body = match kind {
        PanelSkeletonKind::Home => view! {
            {sk_head(2)}
            <div class="grid grid-cols-2 gap-4 lg:grid-cols-4">
                {(0..4).map(|_| sk_card("", Box::new(|| view! {
                    {sk_line("w-1/3")}<Skeleton class="h-9 w-1/2"/>{sk_line("w-2/3")}
                }.into_any()))).collect_view()}
            </div>
            <div class="grid gap-4 lg:grid-cols-[1.4fr_1fr]">
                {sk_card("", Box::new(|| view! {
                    {sk_line("w-1/4")}
                    {(0..3).map(|_| view! {
                        <div class="flex items-center gap-3"><Skeleton class="size-11"/><div class="grow">{sk_line("w-full")}</div></div>
                    }).collect_view()}
                }.into_any()))}
                {sk_card("", Box::new(|| view! {
                    {sk_line("w-1/3")}{sk_line("w-full")}{sk_line("w-full")}{sk_line("w-2/3")}{sk_line("w-full")}
                }.into_any()))}
            </div>
        }
        .into_any(),
        PanelSkeletonKind::List => view! {
            {sk_head(2)}
            <Skeleton class="h-10 w-80 max-w-full"/>
            {sk_card("divide-y divide-base-300 !gap-0 !py-1", Box::new(|| view! {
                {(0..5).map(|_| view! {
                    <div class="grid grid-cols-[3.5rem_1fr_6rem_5rem] items-center gap-4 py-3">
                        <Skeleton class="size-14"/>
                        <div class="flex flex-col gap-2">{sk_line("w-2/3")}<Skeleton class="h-2.5 w-1/3"/></div>
                        {sk_line("w-full")}{sk_line("w-full")}
                    </div>
                }).collect_view()}
            }.into_any()))}
        }
        .into_any(),
        PanelSkeletonKind::Board => view! {
            <div class="flex flex-col gap-2"><Skeleton class="h-8 w-40"/>{sk_line("w-80 max-w-full")}</div>
            <div class="grid grid-cols-2 gap-4 lg:grid-cols-4">
                {[2, 1, 1, 0].into_iter().map(|n| view! {
                    <div class="flex min-h-64 flex-col gap-2.5 rounded-box bg-base-300/50 p-3.5">
                        <div class="flex items-center gap-2">{sk_line("w-1/2")}<Skeleton class="ml-auto h-4 w-6"/></div>
                        {(0..n).map(|_| sk_card("!gap-2 !p-3", Box::new(|| view! {
                            {sk_line("w-2/3")}{sk_line("w-1/3")}<Skeleton class="h-2.5 w-full"/>
                        }.into_any()))).collect_view()}
                    </div>
                }).collect_view()}
            </div>
        }
        .into_any(),
        PanelSkeletonKind::Form => view! {
            <div class="flex flex-col gap-2"><Skeleton class="h-8 w-52"/>{sk_line("w-96 max-w-full")}</div>
            {sk_card("max-w-3xl gap-4", Box::new(|| view! {
                {(0..2).map(|_| view! {
                    <div class="flex items-center gap-3">
                        <Skeleton class="size-11"/>
                        <div class="flex grow flex-col gap-2">{sk_line("w-1/2")}<Skeleton class="h-2.5 w-full"/></div>
                        <Skeleton class="h-6 w-12 rounded-full"/>
                    </div>
                }).collect_view()}
                {sk_line("w-1/4")}<Skeleton class="h-10 w-full"/>
                {sk_line("w-1/4")}<Skeleton class="h-10 w-full"/>
                <Skeleton class="h-10 w-32"/>
            }.into_any()))}
        }
        .into_any(),
        PanelSkeletonKind::Photos => view! {
            {sk_head(1)}
            <div class="grid grid-cols-2 gap-3.5 sm:grid-cols-4 lg:grid-cols-5">
                {(0..10).map(|_| view! { <Skeleton class="aspect-square w-full"/> }).collect_view()}
            </div>
        }
        .into_any(),
    };
    view! { <div class="flex flex-col gap-5" aria-busy="true" aria-label="Cargando">{body}</div> }
}

/// El editor mientras se baja y llega tu tienda: la misma barra, el riel de
/// bloques, el lienzo (775 px, como el real) y el panel de la derecha. El
/// mensaje cambia cada 2 segundos (en CSS: anda aunque la página no haya
/// cargado del todo) y se queda quieto si pediste menos movimiento.
#[component]
pub fn EditorSkeleton() -> impl IntoView {
    // Los textos los eligió Fran; cada uno se ve 2 s (ver `.mk-load-msg` en
    // style/tailwind.css: si cambia la cantidad, cambia `--n` y los porcentajes).
    let msgs = [
        "Abriendo tu editor…",
        "Cargando tu diseño…",
        "Alistando tus productos…",
        "Ya casi está…",
        "Está tardando…",
        "Vamoo, apura…",
        "Ya casi…",
    ];

    view! {
        <div data-theme="light" class="relative grid h-screen grid-cols-[76px_1fr_368px] grid-rows-[64px_1fr] bg-base-100 text-base-content" aria-busy="true" role="status" aria-label="Abriendo el editor">
            <div class="col-span-3 flex items-center gap-3.5 border-b border-base-300 px-4">
                <Skeleton class="size-7 rounded-full"/>
                <div class="flex w-56 items-center gap-2.5 rounded-box border border-base-300 p-2">
                    <Skeleton class="size-9"/>
                    <div class="flex grow flex-col gap-1.5">{sk_line("w-2/3")}<Skeleton class="h-2.5 w-full"/></div>
                </div>
                <Skeleton class="h-9 w-48"/><Skeleton class="h-5 w-28"/>
                <Skeleton class="ml-8 h-9 w-48"/>
                <div class="grow"></div>
                <Skeleton class="h-5 w-14"/><Skeleton class="h-5 w-28"/><Skeleton class="h-9 w-24"/>
            </div>
            <div class="flex flex-col items-center gap-5 border-r border-base-300 pt-4">
                {(0..6).map(|_| view! {
                    <div class="flex flex-col items-center gap-1.5"><Skeleton class="size-[22px]"/><Skeleton class="h-2 w-10"/></div>
                }).collect_view()}
            </div>
            <div class="flex justify-center overflow-hidden bg-base-200 py-3">
                <div class="flex w-[775px] flex-col gap-4">
                    <Skeleton class="h-44 w-full"/>
                    {sk_line("w-28")}
                    <div class="grid grid-cols-6 gap-2">
                        {(0..8).map(|_| view! { <Skeleton class="h-36 w-full"/> }).collect_view()}
                    </div>
                </div>
            </div>
            <div class="flex flex-col gap-3 border-l border-base-300 p-5">
                <Skeleton class="h-6 w-3/4"/>{sk_line("w-full")}{sk_line("w-2/3")}<Skeleton class="mt-1.5 h-16 w-full"/>
            </div>
            <div class="pointer-events-none absolute inset-0 grid place-items-center">
                <div class="mk-load-msg relative h-11 w-72 rounded-box border border-base-300 bg-base-100 font-semibold shadow-lg">
                    {msgs.into_iter().enumerate().map(|(i, m)| view! {
                        <span class="mk-load-msg-n absolute inset-0 grid place-items-center" style=format!("--i:{i}")>{m}</span>
                    }).collect_view()}
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
        "menu" => "M4 6h16M4 12h16M4 18h16",
        "store" => "M4 10v10h16V10M3 4h18l-1.5 6h-15zM9.5 20v-5h5v5",
        "chart" => "M4 20V10M10 20V4M16 20v-7M22 20H2",
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
        // Carta ya enviada (con tilde).
        "mail-check" => "M21 11V5H3v14h9M3 7l9 6 9-6M15.5 18l2.2 2.2 4.3-4.7",
        "key" => "M8 11a4 4 0 1 0 0 8 4 4 0 0 0 0-8M10.8 12.2L20 3M17 6l3 3M14 9l2 2",
        // Contacto de la tienda.
        "whatsapp" => "M3.5 20.5l1.3-4.2A8.5 8.5 0 1 1 8 19.3zM9 8c-.3 3.4 3.2 7 7 7l1-1.6-2.2-1.1-1 .9c-1.2-.5-2.4-1.7-2.9-2.9l.9-1L10.6 7z",
        "instagram" => "M8 3h8a5 5 0 0 1 5 5v8a5 5 0 0 1-5 5H8a5 5 0 0 1-5-5V8a5 5 0 0 1 5-5zM12 8a4 4 0 1 0 0 8 4 4 0 0 0 0-8M17.5 6.5v.01",
        "pin" => "M12 21s-7-6.2-7-11a7 7 0 0 1 14 0c0 4.8-7 11-7 11zM12 7.5a2.5 2.5 0 1 0 0 5 2.5 2.5 0 0 0 0-5",
        "arrow-right" => "M5 12h14M13 6l6 6-6 6",
        "chevron-left" => "M15 6l-6 6 6 6",
        "chevron-right" => "M9 6l6 6-6 6",
        "close" => "M6 6l12 12M18 6L6 18",
        "facebook" => "M14 8h3V4h-3a4 4 0 0 0-4 4v2H7v4h3v6h4v-6h3l1-4h-4V8z",
        "tiktok" => "M14 4v10.5a3.5 3.5 0 1 1-3.5-3.5M14 4c.4 2.600 2 4 5 4",
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

/// Skeleton del cajón de la cuenta del comprador con sesión (ver
/// `web::buyer_ui`): perfil, tarjeta del pedido en curso, tres opciones y el
/// botón de salir anclado al fondo. Si cambia ese diseño, se cambia este.
#[component]
pub fn BuyerDrawerSkeleton() -> impl IntoView {
    view! {
        <div class="flex h-full flex-col px-6 py-5" aria-busy="true" aria-label="Cargando tu cuenta">
            <div class="flex items-center gap-3">
                <Skeleton class="size-10 shrink-0 rounded-full"/>
                <div class="flex flex-1 flex-col gap-2"><Skeleton class="h-3.5 w-32"/><Skeleton class="h-3 w-44"/></div>
            </div>
            <Skeleton class="mt-5 h-36 w-full rounded-box"/>
            <div class="mt-3 flex flex-1 flex-col">
                {(0..3).map(|_| view! {
                    <div class="flex items-center justify-between border-b border-base-200 py-4">
                        <Skeleton class="h-3.5 w-2/5"/><Skeleton class="h-3.5 w-4"/>
                    </div>
                }).collect_view()}
            </div>
            <Skeleton class="h-11 w-full rounded-field"/>
        </div>
    }
}

/// Qué pantalla de la cuenta del comprador imita [`BuyerPageSkeleton`].
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum BuyerPageKind {
    /// Mis pedidos: título, pestañas y tarjetas de pedido.
    Orders,
    /// Detalle de un pedido: estado, seguimiento y productos.
    Order,
    /// Mis datos y Direcciones: avatar y formulario en una tarjeta.
    Form,
}

/// Skeleton de las páginas de la cuenta del comprador (`/mi-cuenta/…`): el
/// encabezado de la tienda y el contenido de cada pantalla con sus medidas.
#[component]
pub fn BuyerPageSkeleton(kind: BuyerPageKind) -> impl IntoView {
    let body = match kind {
        BuyerPageKind::Orders => view! {
            <Skeleton class="h-7 w-44"/>
            <div class="flex gap-2"><Skeleton class="h-9 w-24"/><Skeleton class="h-9 w-24"/><Skeleton class="h-9 w-28"/></div>
            {(0..2).map(|_| view! {
                <div class="card bg-base-100 shadow-sm"><div class="card-body gap-4 p-5">
                    <div class="flex items-center gap-3"><Skeleton class="h-4 w-28"/><Skeleton class="h-5 w-16 rounded-full"/><Skeleton class="ml-auto h-3.5 w-12"/></div>
                    <div class="flex items-center gap-2"><Skeleton class="size-10"/><Skeleton class="size-10"/><Skeleton class="ml-auto h-4 w-24"/></div>
                </div></div>
            }).collect_view()}
        }
        .into_any(),
        BuyerPageKind::Order => view! {
            <Skeleton class="h-4 w-32"/>
            <div class="card bg-base-100 shadow-sm"><div class="card-body gap-5 p-6">
                <div class="flex items-center gap-3"><Skeleton class="h-7 w-40"/><Skeleton class="h-6 w-20 rounded-full"/></div>
                <Skeleton class="h-3.5 w-48"/>
                <div class="grid grid-cols-4 gap-3">
                    {(0..4).map(|_| view! { <div class="flex flex-col gap-2"><Skeleton class="size-4 rounded-full"/><Skeleton class="h-3.5 w-3/4"/></div> }).collect_view()}
                </div>
                <Skeleton class="h-20 w-full rounded-box"/>
                {(0..2).map(|_| view! {
                    <div class="flex items-center gap-3 border-t border-base-200 pt-4">
                        <Skeleton class="size-12"/>
                        <div class="flex flex-1 flex-col gap-2"><Skeleton class="h-3.5 w-3/5"/><Skeleton class="h-3 w-1/4"/></div>
                        <Skeleton class="h-3.5 w-16"/>
                    </div>
                }).collect_view()}
            </div></div>
        }
        .into_any(),
        BuyerPageKind::Form => view! {
            <Skeleton class="h-7 w-44"/>
            <div class="flex gap-2"><Skeleton class="h-9 w-24"/><Skeleton class="h-9 w-24"/><Skeleton class="h-9 w-28"/></div>
            <div class="card bg-base-100 shadow-sm"><div class="card-body gap-5 p-6">
                <div class="flex items-center gap-4"><Skeleton class="size-20 rounded-full"/><Skeleton class="h-9 w-32"/></div>
                {(0..3).map(|_| view! { <div class="flex flex-col gap-2"><Skeleton class="h-3.5 w-24"/><Skeleton class="h-11 w-full"/></div> }).collect_view()}
                <Skeleton class="h-11 w-36"/>
            </div></div>
        }
        .into_any(),
    };
    view! {
        <div class="min-h-screen bg-base-200" aria-busy="true" aria-label="Cargando">
            <div class="mx-auto flex max-w-[1120px] flex-col gap-5 px-4 py-4">
                <div class="flex h-18 items-center rounded-box bg-base-100 px-6 shadow-sm">
                    <Skeleton class="h-5 w-36"/><Skeleton class="ml-auto h-4 w-32"/>
                </div>
                <div class="mx-auto flex w-full max-w-2xl flex-col gap-4">{body}</div>
            </div>
        </div>
    }
}
