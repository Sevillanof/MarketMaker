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
    /// Sin foto, muestra la inicial de `alt` sobre un color suave (el color
    /// sale del nombre). Para productos; en el resto, el cuadro liso de siempre.
    #[prop(optional)]
    initial: bool,
) -> impl IntoView {
    if src.is_empty() && initial {
        if let Some((letter, style)) = initial_tile(&alt) {
            return view! {
                <div class="mk-img mk-initial grid h-full w-full place-items-center overflow-hidden" data-state="empty" role="img" aria-label=alt style=style>
                    <span class="select-none font-bold leading-none" aria-hidden="true">{letter}</span>
                </div>
            }
            .into_any();
        }
    }
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

/// La inicial de `name` y el color suave que le toca (siempre el mismo para
/// el mismo nombre). `None` si no hay nombre.
pub fn initial_tile(name: &str) -> Option<(String, String)> {
    let letter = name.chars().find(|c| c.is_alphanumeric())?;
    let hue = name
        .bytes()
        .fold(7u32, |h, b| h.wrapping_mul(31).wrapping_add(u32::from(b)))
        % 360;
    Some((
        letter.to_uppercase().collect(),
        format!(
            "background:oklch(94% 0.04 {hue});color:oklch(48% 0.14 {hue});container-type:size"
        ),
    ))
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
    let (avif, webp) = match srcsets(&src).or_else(|| library_srcsets(&src)) {
        Some((a, w)) => (Some(a), Some(w)),
        None => (avif_variant(&src), None),
    };
    // La foto de arriba (banner, primera del carrusel) ocupa todo el ancho.
    let sizes = webp
        .as_ref()
        .map(|_| if eager { "100vw" } else { PHOTO_SIZES });
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

/// Fotos anchas de la biblioteca (1600 px) que también vienen en 800 px
/// (`<nombre>-800.webp`), para el celular.
const WIDE_LIBRARY: &[&str] = &["banner", "hero", "slide-1", "slide-2", "slide-3"];

/// `srcset` AVIF y WebP (800 y 1600 px) de una foto ancha de la biblioteca.
pub fn library_srcsets(src: &str) -> Option<(String, String)> {
    let name = src.strip_prefix("/img/")?.strip_suffix(".webp")?;
    WIDE_LIBRARY.contains(&name).then(|| {
        let set = |ext: &str| format!("/img/{name}-800.{ext} 800w, /img/{name}.{ext} 1600w");
        (set("avif"), set("webp"))
    })
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

/// Skeleton de la ventana Reportes: la lista a la izquierda (número, estado y
/// dos líneas), el detalle a la derecha (captura, texto y datos) y, abajo, la
/// fila de botones de estado, con las mismas medidas que la ventana real.
#[component]
pub fn ReportsSkeleton() -> impl IntoView {
    view! {
        <div class="min-h-0 grow overflow-hidden" aria-busy="true" aria-label="Cargando">
            <div class="grid border-t border-base-300 md:grid-cols-[2fr_3fr]">
                <ul class="divide-y divide-base-300">
                    {(0..4).map(|_| view! {
                        <li class="flex flex-col gap-2 px-6 py-2.5">
                            <div class="flex items-center gap-2">
                                <Skeleton class="h-4 w-8"/><Skeleton class="h-4 w-14 rounded-full"/><Skeleton class="ml-auto h-3 w-16"/>
                            </div>
                            <Skeleton class="h-3.5 w-full"/><Skeleton class="h-3.5 w-2/3"/>
                        </li>
                    }).collect_view()}
                </ul>
                <section class="flex flex-col gap-3 p-5 max-md:hidden md:border-l md:border-base-300">
                    <div class="flex items-center gap-2"><Skeleton class="h-5 w-32"/><Skeleton class="h-4 w-14 rounded-full"/></div>
                    <Skeleton class="h-56 w-full"/>
                    <Skeleton class="h-3.5 w-full"/><Skeleton class="h-3.5 w-4/5"/>
                </section>
            </div>
            <div class="flex items-center gap-2 border-t border-base-300 p-4">
                <Skeleton class="h-4 w-12"/><Skeleton class="h-8 w-20"/><Skeleton class="h-8 w-24"/><Skeleton class="h-8 w-24"/>
            </div>
        </div>
    }
}

/// Una línea de texto de skeleton: `w` es la clase de ancho (`w-1/3`…).
fn sk_line(w: &'static str) -> impl IntoView {
    view! { <Skeleton class=format!("h-3.5 {w}")/> }
}

/// La tarjeta "Equipo" de Datos de la tienda: título, campo con rol y botón,
/// y filas de personas.
#[component]
pub fn TeamSkeleton() -> impl IntoView {
    view! {
        <div aria-busy="true" aria-label="Cargando el equipo">
            {sk_card("gap-3 p-6", Box::new(|| view! {
                <Skeleton class="h-5 w-24"/>{sk_line("w-96 max-w-full")}
                <div class="flex gap-2"><Skeleton class="h-10 grow"/><Skeleton class="h-10 w-32"/><Skeleton class="h-10 w-24"/></div>
                {sk_line("w-40")}
                {(0..3).map(|_| view! {
                    <div class="flex items-center gap-3 border-t border-base-200 pt-3">
                        <Skeleton class="size-10 rounded-full"/>
                        <div class="flex grow flex-col gap-2">{sk_line("w-44")}{sk_line("w-56")}</div>
                        <Skeleton class="h-8 w-28"/>
                    </div>
                }).collect_view()}
            }.into_any()))}
        </div>
    }
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
    /// Productos: la lista (buscador, filtros, filas con precio y stock) y
    /// el panel rápido a la derecha.
    List,
    /// Pedidos: cuatro columnas con tarjetas.
    Board,
    /// Cobros y Entregas: formulario en una tarjeta.
    Form,
    /// Mis fotos: grilla de fotos.
    Photos,
    /// La ficha completa de un producto, en su ventana: cabecera, campos con
    /// la vista previa al costado y el pie con Guardar.
    Product,
    /// Cupones: la tabla de cupones y el formulario «Nuevo cupón» en dos
    /// columnas.
    Coupons,
    /// Datos de la tienda: nombre, logo, contacto en dos columnas,
    /// descripción y horarios, la vista previa del pie y "Tu tienda".
    StoreInfo,
    /// Mi cuenta: filas de datos y, aparte, Salir y Borrar mi cuenta.
    Account,
}

#[component]
pub fn PanelSkeleton(kind: PanelSkeletonKind) -> impl IntoView {
    // La ficha va en su ventana, pegada a los bordes (cabecera y pie).
    let gap = if matches!(kind, PanelSkeletonKind::Product) {
        "flex min-h-0 grow flex-col"
    } else {
        "flex flex-col gap-5"
    };
    let body = match kind {
        PanelSkeletonKind::Home => view! {
            {sk_head(2)}
            <div class="grid grid-cols-2 gap-4 lg:grid-cols-4">
                // Tarjetas centradas: título, número y enlace.
                {(0..4).map(|_| sk_card("items-center", Box::new(|| view! {
                    {sk_line("w-1/3")}<Skeleton class="h-9 w-1/2"/>{sk_line("w-2/3")}
                }.into_any()))).collect_view()}
            </div>
            // «Tu tienda esta semana» (barras y tres cifras) y «Lo más vendido».
            <div class="grid gap-4 lg:grid-cols-[3fr_2fr]">
                {sk_card("", Box::new(|| view! {
                    {sk_line("w-1/3")}<Skeleton class="h-28 w-full"/>
                    <div class="grid grid-cols-3 gap-2">{(0..3).map(|_| view! { <Skeleton class="h-14 w-full"/> }).collect_view()}</div>
                }.into_any()))}
                {sk_card("", Box::new(|| view! {
                    {sk_line("w-1/3")}
                    {(0..3).map(|_| view! { {sk_line("w-full")} }).collect_view()}
                }.into_any()))}
            </div>
            <div class="grid gap-4 lg:grid-cols-[1.4fr_1fr]">
                {sk_card("", Box::new(|| view! {
                    {sk_line("w-1/4")}
                    {(0..3).map(|_| view! {
                        <div class="flex items-center gap-3"><Skeleton class="size-11"/><div class="grow">{sk_line("w-full")}</div></div>
                    }).collect_view()}
                }.into_any()))}
                // Primeros pasos: título, barra de progreso y un paso por fila (círculo, dos líneas y botones).
                {sk_card("", Box::new(|| view! {
                    {sk_line("w-1/3")}<Skeleton class="h-2 w-full"/>
                    {(0..4).map(|_| view! {
                        <div class="flex items-center gap-3">
                            <Skeleton class="size-6 shrink-0 rounded-full"/>
                            <div class="flex grow flex-col gap-2">{sk_line("w-2/3")}{sk_line("w-1/2")}</div>
                            <Skeleton class="h-8 w-24 shrink-0"/>
                        </div>
                    }).collect_view()}
                }.into_any()))}
            </div>
        }
        .into_any(),
        PanelSkeletonKind::List => view! {
            // Título y cantidad; Importar (solo compu), Etiquetas, Categorías y + Nuevo producto (en el
            // teléfono, + Nuevo producto a todo el ancho arriba).
            <div class="flex flex-wrap items-center gap-3">
                <Skeleton class="h-11 w-full sm:hidden"/>
                <div class="flex grow flex-col gap-2"><Skeleton class="h-8 w-40 max-sm:h-7"/>{sk_line("w-56 max-w-full")}</div>
                <Skeleton class="h-10 w-20 max-sm:hidden"/>
                <Skeleton class="h-10 w-24 max-sm:h-11 max-sm:flex-1"/><Skeleton class="h-10 w-28 max-sm:h-11 max-sm:flex-1"/>
                <Skeleton class="h-10 w-40 max-sm:hidden"/>
            </div>
            // La lista (buscador, filtros y filas con foto, precio y stock) y,
            // en compu, el panel rápido a la derecha.
            <div class="grid items-start gap-4 md:grid-cols-[320px_1fr] lg:grid-cols-[400px_1fr]">
                <div class="card overflow-hidden bg-base-100 shadow-sm">
                    <div class="flex flex-col gap-2 border-b border-base-200 p-3">
                        <Skeleton class="h-10 w-full max-sm:h-12"/>
                        <Skeleton class="h-8 w-full max-sm:h-10"/>
                    </div>
                    {(0..6).map(|_| view! {
                        <div class="flex items-center gap-3 border-b border-base-200 px-3.5 py-2.5 last:border-0">
                            <Skeleton class="size-11 shrink-0 max-sm:size-12"/>
                            <div class="flex grow flex-col gap-2">{sk_line("w-2/3")}<Skeleton class="h-2.5 w-1/3"/></div>
                            <div class="flex flex-col items-end gap-1.5"><Skeleton class="h-3.5 w-14"/><Skeleton class="h-4 w-10 rounded-full"/></div>
                        </div>
                    }).collect_view()}
                </div>
                <div class="card bg-base-100 shadow-sm max-md:hidden">
                    <div class="card-body gap-4 p-5">
                        <div class="flex items-center gap-2">{sk_line("w-14")}<Skeleton class="ml-auto h-8 w-24"/><Skeleton class="h-8 w-24"/></div>
                        <div class="grid gap-5 lg:grid-cols-[minmax(0,15rem)_1fr]">
                            <div class="flex flex-col gap-2">
                                <Skeleton class="aspect-square w-full"/>
                                <div class="grid grid-cols-4 gap-2">{(0..4).map(|_| view! { <Skeleton class="aspect-square"/> }).collect_view()}</div>
                                // «En la tienda», debajo de las fotos.
                                <Skeleton class="mt-2 h-16 w-full"/>
                            </div>
                            <div class="flex flex-col gap-3">
                                <Skeleton class="h-8 w-3/4"/>
                                <div class="flex gap-2"><Skeleton class="h-7 w-20 rounded-full"/><Skeleton class="h-7 w-24 rounded-full"/></div>
                                <div class="grid grid-cols-2 gap-3">{(0..4).map(|_| view! { <div class="flex flex-col gap-1.5">{sk_line("w-1/2")}<Skeleton class="h-10 w-full"/></div> }).collect_view()}</div>
                                {sk_line("w-12")}<Skeleton class="h-16 w-full"/>
                            </div>
                        </div>
                        {sk_line("w-24")}<Skeleton class="h-32 w-full"/>
                        <div class="grid grid-cols-[1fr_auto_1fr] items-center gap-3 border-t border-base-200 pt-4">
                            <div class="flex gap-1"><Skeleton class="h-10 w-36"/><Skeleton class="h-10 w-24"/></div><Skeleton class="h-8 w-16"/>
                            <div class="flex justify-end gap-2"><Skeleton class="h-10 w-24"/><Skeleton class="h-10 w-24"/></div>
                        </div>
                    </div>
                </div>
            </div>
        }
        .into_any(),
        PanelSkeletonKind::Board => view! {
            <div class="flex flex-col gap-2"><Skeleton class="h-8 w-40 max-sm:h-7"/>{sk_line("w-80 max-w-full")}</div>
            // Buscador y botón ⋯ (fechas y exportar).
            <div class="flex gap-2"><Skeleton class="h-10 w-full max-w-md max-sm:h-12"/><Skeleton class="size-10 shrink-0 max-sm:size-12"/></div>
            // Teléfono: las pestañas de estado y los pedidos de una columna.
            <div class="flex flex-col gap-3 sm:hidden">
                <div class="flex gap-2 overflow-hidden">
                    {(0..3).map(|_| view! { <Skeleton class="h-10 w-28 shrink-0 rounded-full"/> }).collect_view()}
                </div>
                {(0..3).map(|_| sk_card("!gap-2 !p-3", Box::new(|| view! {
                    {sk_line("w-2/3")}{sk_line("w-1/3")}<Skeleton class="h-11 w-full"/>
                }.into_any()))).collect_view()}
            </div>
            <div class="grid grid-cols-2 gap-4 max-sm:hidden lg:grid-cols-4">
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
            {sk_card("gap-4", Box::new(|| view! {
                {(0..2).map(|_| view! {
                    <div class="flex items-center gap-3">
                        <Skeleton class="size-11"/>
                        <div class="flex grow flex-col gap-2">{sk_line("w-1/2")}<Skeleton class="h-2.5 w-full"/></div>
                        <Skeleton class="h-6 w-12 rounded-full"/>
                    </div>
                }).collect_view()}
                {sk_line("w-1/4")}<Skeleton class="h-10 w-full"/>
                {sk_line("w-1/4")}<Skeleton class="h-10 w-full"/>
                // Pedido mínimo (Entregas): un campo corto.
                {sk_line("w-1/5")}<Skeleton class="h-10 w-full max-w-xs"/>
                <Skeleton class="h-10 w-32"/>
            }.into_any()))}
        }
        .into_any(),
        PanelSkeletonKind::Product => view! {
            // La ficha completa en su ventana: cabecera (nombre, ‹ › y ✕), el
            // formulario con la vista previa a la derecha y el pie con Guardar.
            <div class="flex items-center gap-2 border-b border-base-200 px-5 py-3 max-sm:px-3">
                <div class="flex grow flex-col gap-1.5"><Skeleton class="h-2.5 w-24 max-sm:hidden"/><Skeleton class="h-6 w-56 max-w-full"/></div>
                <Skeleton class="size-8 max-sm:size-11"/><Skeleton class="size-8 max-sm:size-11"/><Skeleton class="size-8 max-sm:size-11"/>
            </div>
            <div class="grid min-h-0 grow items-start gap-6 overflow-hidden px-5 py-4 max-sm:px-4 lg:grid-cols-[1fr_340px]">
                <div class="flex flex-col gap-4">
                    {sk_line("w-24")}<Skeleton class="h-10 w-full"/>
                    <div class="grid gap-4 sm:grid-cols-2">
                        {(0..2).map(|_| view! { <div class="flex flex-col gap-2">{sk_line("w-28")}<Skeleton class="h-10 w-full"/></div> }).collect_view()}
                    </div>
                    {sk_line("w-20")}
                    <div class="flex gap-3">{(0..3).map(|_| view! { <Skeleton class="size-20 shrink-0"/> }).collect_view()}</div>
                    // Descripción, con «Ver en la tienda» a la derecha.
                    <div class="flex items-center justify-between">{sk_line("w-28")}<Skeleton class="h-4 w-28"/></div>
                    <Skeleton class="h-28 w-full"/>
                    <div class="grid gap-4 sm:grid-cols-2">
                        {(0..2).map(|_| view! { <div class="flex flex-col gap-2">{sk_line("w-32")}<Skeleton class="h-10 w-full"/></div> }).collect_view()}
                    </div>
                </div>
                <div class="flex flex-col gap-3 max-lg:hidden">{sk_line("w-40")}<Skeleton class="h-80 w-full"/>{sk_line("w-3/4")}</div>
            </div>
            <div class="flex flex-col gap-2 border-t border-base-200 px-5 py-3 max-sm:px-4">
                <div class="grid grid-cols-[1fr_auto_1fr] items-center gap-3 max-sm:grid-cols-[1fr_auto]">
                    <span></span><Skeleton class="h-8 w-16 max-sm:hidden"/>
                    <div class="flex justify-end gap-2"><Skeleton class="h-10 w-24"/><Skeleton class="h-10 w-24 max-sm:hidden"/></div>
                </div>
                <Skeleton class="h-12 w-full sm:hidden"/>
            </div>
        }
        .into_any(),
        PanelSkeletonKind::Coupons => view! {
            <div class="flex flex-col gap-2"><Skeleton class="h-8 w-40"/>{sk_line("w-96 max-w-full")}</div>
            // Tus cupones: título y botón, y una fila por cupón (código, descuento,
            // condición, usos, interruptor y acciones).
            {sk_card("gap-3 p-6", Box::new(|| view! {
                <div class="flex items-center justify-between"><Skeleton class="h-6 w-36"/><Skeleton class="h-8 w-32"/></div>
                {(0..3).map(|_| view! {
                    <div class="grid grid-cols-[7rem_4rem_1fr_4rem_3rem_6rem] items-center gap-4 border-t border-base-300 py-3 max-sm:grid-cols-[6rem_1fr_3rem]">
                        <Skeleton class="h-7 w-full"/>{sk_line("w-full")}{sk_line("w-3/4")}{sk_line("w-full")}<Skeleton class="h-5 w-10 rounded-full"/>{sk_line("w-full")}
                    </div>
                }).collect_view()}
            }.into_any()))}
            // Nuevo cupón: título y dos columnas de campos.
            {sk_card("gap-4 p-6", Box::new(|| view! {
                <Skeleton class="h-6 w-32"/>
                <div class="grid gap-6 sm:grid-cols-2">
                    {(0..2).map(|n| view! {
                        <div class="flex flex-col gap-3">
                            {(0..if n == 0 { 2 } else { 3 }).map(|_| view! { <div class="flex flex-col gap-2">{sk_line("w-28")}<Skeleton class="h-10 w-full"/></div> }).collect_view()}
                        </div>
                    }).collect_view()}
                </div>
                <Skeleton class="h-10 w-32"/>
            }.into_any()))}
        }
        .into_any(),
        PanelSkeletonKind::StoreInfo => view! {
            <div class="flex flex-col gap-2"><Skeleton class="h-8 w-52"/>{sk_line("w-[36rem] max-w-full")}</div>
            {sk_card("gap-4 p-6", Box::new(|| view! {
                {sk_line("w-48")}<Skeleton class="h-10 w-full"/>
                {sk_line("w-32")}
                <div class="flex items-center gap-4">
                    <Skeleton class="size-20"/>
                    <div class="flex flex-col gap-2"><Skeleton class="h-8 w-28"/>{sk_line("w-64")}</div>
                </div>
                <div class="grid gap-4 sm:grid-cols-2">
                    {(0..2).map(|_| view! { <div class="flex flex-col gap-2">{sk_line("w-52")}<Skeleton class="h-10 w-full"/></div> }).collect_view()}
                </div>
                {sk_line("w-56")}<Skeleton class="h-10 w-full"/>
                <div class="grid gap-4 sm:grid-cols-2">
                    <div class="flex flex-col gap-2">{sk_line("w-56")}<Skeleton class="h-16 w-full"/></div>
                    <div class="flex flex-col gap-2">{sk_line("w-60")}<Skeleton class="h-10 w-full"/></div>
                </div>
                {sk_line("w-36")}
                <div class="flex flex-col gap-3 rounded-box border border-base-300 px-6 py-5">
                    <div class="flex justify-between"><Skeleton class="h-5 w-44"/><Skeleton class="h-4 w-48"/></div>
                    {sk_line("w-2/3")}
                </div>
                <Skeleton class="h-10 w-28"/>
            }.into_any()))}
            // "Avisos a compradores": un interruptor con su ayuda.
            {sk_card("gap-3 p-5", Box::new(|| view! {
                <Skeleton class="h-5 w-44"/>
                <div class="flex items-start gap-3"><Skeleton class="h-6 w-12 shrink-0 rounded-full"/><div class="flex grow flex-col gap-2">{sk_line("w-56")}{sk_line("w-full max-w-xl")}</div></div>
            }.into_any()))}
            // "Equipo": campo para sumar gente y la lista (solo dueño y administradores).
            <TeamSkeleton/>
            // "Tu tienda": dirección, pausar y borrar.
            {sk_card("!gap-0", Box::new(|| view! {
                <Skeleton class="mb-3 h-5 w-28"/>
                {(0..3).map(|_| view! {
                    <div class="flex items-center gap-3 border-b border-base-200 py-3 last:border-0">
                        <div class="flex grow flex-col gap-2">{sk_line("w-32")}<Skeleton class="h-2.5 w-64 max-w-full"/></div>
                        <Skeleton class="h-8 w-20"/>
                    </div>
                }).collect_view()}
            }.into_any()))}
        }
        .into_any(),
        PanelSkeletonKind::Account => view! {
            // Mi cuenta: foto, nombre, email, contraseña y acceso rápido; abajo
            // Salir y Borrar mi cuenta.
            <div class="flex flex-col gap-2"><Skeleton class="h-8 w-40"/>{sk_line("w-96 max-w-full")}</div>
            {[5, 2].into_iter().map(|n| view! {
                <div class="rounded-2xl border border-base-300 bg-base-100">
                    {(0..n).map(|_| view! {
                        <div class="flex items-center gap-4 border-t border-base-300 px-6 py-4 first:border-0 max-sm:px-4">
                            <Skeleton class="size-[18px] shrink-0 max-sm:hidden"/>
                            <Skeleton class="h-3.5 w-28 shrink-0"/>
                            <div class="grow">{sk_line("w-1/2")}</div>
                            <Skeleton class="h-8 w-20"/>
                        </div>
                    }).collect_view()}
                </div>
            }).collect_view()}
        }
        .into_any(),
        PanelSkeletonKind::Photos => view! {
            // Arriba ya están el título y los botones: acá, cuánto usás.
            {sk_card("", Box::new(|| view! { {sk_line("w-1/3")}<Skeleton class="h-2 w-full"/> }.into_any()))}
            // Mis fotos: grilla compacta, cada foto cuadrada con nombre y si se usa.
            <div class="grid grid-cols-2 gap-3.5 sm:grid-cols-3 lg:grid-cols-4">
                {(0..8).map(|_| view! {
                    <div class="overflow-hidden rounded-box border border-base-300 bg-base-100">
                        <Skeleton class="aspect-square w-full rounded-none"/>
                        <div class="flex flex-col gap-1.5 px-2.5 py-2">{sk_line("w-2/3")}<Skeleton class="h-2.5 w-1/3"/></div>
                    </div>
                }).collect_view()}
            </div>
            <Skeleton class="fixed right-4 bottom-4 h-13 w-40 rounded-full sm:hidden"/>
        }
        .into_any(),
    };
    view! { <div class=gap aria-busy="true" aria-label="Cargando">{body}</div> }
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
        <div data-theme="light" class="relative grid h-screen grid-cols-[176px_1fr_368px] grid-rows-[64px_1fr] bg-base-100 text-base-content" aria-busy="true" role="status" aria-label="Abriendo el editor">
            <div class="col-span-3 grid grid-cols-[1fr_auto_1fr] items-center gap-3.5 border-b border-base-300 px-4">
                // Izquierda: "← Panel", la tienda y la página.
                <div class="flex items-center gap-3.5">
                    <Skeleton class="h-7 w-20"/>
                    <div class="flex w-56 items-center gap-2.5 rounded-box border border-base-300 p-2">
                        <Skeleton class="size-9"/>
                        <div class="flex grow flex-col gap-1.5">{sk_line("w-2/3")}<Skeleton class="h-2.5 w-full"/></div>
                    </div>
                    <Skeleton class="h-9 w-48"/>
                </div>
                // Centro: Editar / Vista previa y Computadora / Celular.
                <div class="flex items-center gap-3"><Skeleton class="h-9 w-48"/><Skeleton class="h-9 w-20"/></div>
                // Derecha: estado, deshacer / rehacer, ver tienda y Publicar.
                <div class="flex items-center justify-end gap-3"><Skeleton class="h-5 w-28"/><Skeleton class="h-9 w-20"/><Skeleton class="size-9"/><Skeleton class="h-9 w-24"/></div>
            </div>
            // Bloques para agregar: la categoría y cada bloque con su dibujo,
            // su nombre y para qué sirve.
            <div class="flex flex-col gap-4 overflow-hidden border-r border-base-300 p-2.5">
                <Skeleton class="h-8 w-full"/>
                {(0..4).map(|_| view! {
                    <div class="flex flex-col gap-1.5"><Skeleton class="h-24 w-full"/>{sk_line("w-2/3")}<Skeleton class="h-2.5 w-full"/></div>
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
            // Panel del bloque: cabecera con ícono, Duplicar / Borrar, la sección
            // Contenido abierta (tres campos) y las otras cuatro plegadas.
            <div class="flex flex-col gap-3 overflow-hidden border-l border-base-300 p-4">
                <div class="flex items-start gap-3"><Skeleton class="size-9"/><div class="flex grow flex-col gap-1.5"><Skeleton class="h-5 w-3/4"/>{sk_line("w-full")}</div></div>
                <div class="flex gap-2"><Skeleton class="h-8 w-24"/><Skeleton class="h-8 w-20"/></div>
                <div class="flex flex-col gap-3 rounded-box border border-base-300 p-3">
                    <Skeleton class="h-5 w-24"/>
                    {(0..3).map(|_| view! { <div class="flex flex-col gap-1.5">{sk_line("w-1/4")}<Skeleton class="h-8 w-full"/></div> }).collect_view()}
                </div>
                {(0..3).map(|_| view! { <Skeleton class="h-11 w-full"/> }).collect_view()}
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
        "flag" => "M5 21V4M5 4h11l-2 4 2 4H5",
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
        "eye" => "M2 12s3.5-7 10-7 10 7 10 7-3.5 7-10 7S2 12 2 12zM12 9a3 3 0 1 0 0 6 3 3 0 0 0 0-6",
        "trash" => "M4 7h16M9 7V4h6v3M6 7l1 13h10l1-13M10 11v6M14 11v6",
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
        "play" => "M8 5l11 7-11 7z",
        "search" => "M11 4a7 7 0 1 0 0 14 7 7 0 0 0 0-14zM20 20l-4-4",
        "copy" => "M9 9h10v11H9zM5 15V4h10",
        "ticket" => "M4 7h16v3a2 2 0 0 0 0 4v3H4v-3a2 2 0 0 0 0-4zM14 7v10",
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

/// Precio con formato argentino a partir de centavos: `$ 12.345`.
/// Ayuda de un campo (ⓘ junto a su título): se abre al pasar el mouse, al
/// llegar con el teclado o al tocarla, y se cierra con Esc o al salir. El
/// globo se ubica debajo del título y ocupa a lo sumo su ancho, así nunca se
/// sale de la pantalla: el título (o su contenedor) va con `relative`.
/// `id` es el del globo, para que el campo lo nombre en `aria-describedby`.
#[component]
pub fn HelpTip(
    #[prop(into)] text: Signal<String>,
    #[prop(into)] id: String,
    /// De qué campo es la ayuda, para lectores de pantalla.
    #[prop(into)]
    label: String,
) -> impl IntoView {
    let hover = RwSignal::new(false);
    let focused = RwSignal::new(false);
    let open = move || hover.get() || focused.get();
    let btn = NodeRef::<leptos::html::Button>::new();
    view! {
        <button type="button" node_ref=btn class="mk-tip-btn" aria-label=format!("Ayuda: {label}")
            aria-describedby=id.clone() aria-expanded=move || open().to_string()
            on:mouseenter=move |_| hover.set(true)
            on:mouseleave=move |_| hover.set(false)
            on:focus=move |_| focused.set(true)
            on:blur=move |_| { focused.set(false); hover.set(false); }
            // Safari no enfoca los botones al tocarlos: lo enfocamos a mano.
            on:click=move |ev| {
                ev.prevent_default();
                if let Some(b) = btn.get() { _ = b.focus(); }
                focused.set(true);
            }
            on:keydown=move |ev| if ev.key() == "Escape" && open() {
                // Cierra solo la ayuda, no la ventana en la que está.
                ev.prevent_default();
                ev.stop_propagation();
                hover.set(false);
                focused.set(false);
            }>
            "i"
        </button>
        <span id=id role="tooltip" class="mk-tip" hidden=move || !open()>{move || text.get()}</span>
    }
}

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
                <s class="mk-price-was text-[max(0.85em,0.75rem)] opacity-70">
                    <span class="sr-only">"Antes "</span>{format_price(was)}
                </s>
            })}
        </span>
    }
}

/// Formatea centavos como `$ 12.345`; los centavos (`$ 12.345,50`) solo
/// cuando existen: en pesos casi nunca hay y alargan el precio.
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
    match cents % 100 {
        0 => format!("{sign}$ {grouped}"),
        c => format!("{sign}$ {grouped},{c:02}"),
    }
}

#[cfg(test)]
mod tests {
    use super::{avif_variant, format_price, library_srcsets, srcsets};

    #[test]
    fn wide_library_photos_come_in_two_widths() {
        let (avif, webp) = library_srcsets("/img/hero.webp").unwrap();
        assert_eq!(avif, "/img/hero-800.avif 800w, /img/hero.avif 1600w");
        assert_eq!(webp, "/img/hero-800.webp 800w, /img/hero.webp 1600w");
        assert_eq!(library_srcsets("/img/card.webp"), None);
        for name in super::WIDE_LIBRARY {
            for ext in ["webp", "avif"] {
                let path = format!(
                    "{}/../../public/img/{name}-800.{ext}",
                    env!("CARGO_MANIFEST_DIR")
                );
                assert!(std::path::Path::new(&path).exists(), "falta {path}");
            }
        }
    }

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
        assert_eq!(format_price(1_234_500), "$ 12.345");
        assert_eq!(format_price(5), "$ 0,05");
        assert_eq!(format_price(99_900), "$ 999");
        assert_eq!(format_price(123_456_789), "$ 1.234.567,89");
        assert_eq!(format_price(-100), "-$ 1");
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
                {(0..4).map(|_| view! {
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
    /// Mis favoritos: título, pestañas y una grilla de tarjetas de producto.
    Favorites,
}

/// Skeleton de las páginas de la cuenta del comprador (`/mi-cuenta/…`): el
/// encabezado de la tienda y el contenido de cada pantalla con sus medidas.
#[component]
pub fn BuyerPageSkeleton(kind: BuyerPageKind) -> impl IntoView {
    let body = match kind {
        BuyerPageKind::Orders => view! {
            <Skeleton class="h-7 w-44"/>
            <div class="flex gap-2"><Skeleton class="h-9 w-20"/><Skeleton class="h-9 w-24"/><Skeleton class="h-9 w-24"/><Skeleton class="h-9 w-28"/></div>
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
        BuyerPageKind::Favorites => view! {
            <Skeleton class="h-7 w-44"/>
            <div class="flex gap-2"><Skeleton class="h-9 w-20"/><Skeleton class="h-9 w-24"/><Skeleton class="h-9 w-24"/><Skeleton class="h-9 w-28"/></div>
            <div class="grid grid-cols-2 gap-3 sm:grid-cols-3">
                {(0..6).map(|_| view! {
                    <div class="flex flex-col gap-2">
                        <Skeleton class="aspect-[4/5] w-full rounded-box"/>
                        <Skeleton class="h-3.5 w-4/5"/><Skeleton class="h-3.5 w-2/5"/>
                        <Skeleton class="h-8 w-full rounded-field"/>
                    </div>
                }).collect_view()}
            </div>
        }
        .into_any(),
        BuyerPageKind::Form => view! {
            <Skeleton class="h-7 w-44"/>
            <div class="flex gap-2"><Skeleton class="h-9 w-20"/><Skeleton class="h-9 w-24"/><Skeleton class="h-9 w-24"/><Skeleton class="h-9 w-28"/></div>
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
