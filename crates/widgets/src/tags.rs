//! Bloques para filtrar productos por etiqueta: en fila ([`Widget::TagFilter`]),
//! en columna ([`Widget::TagList`]), con foto ([`Widget::TagCircles`]), un
//! buscador con menú ([`Widget::TagSearch`]) y una cuadrícula con pestañas
//! ([`Widget::TagTabs`]).
//!
//! Los cuatro primeros cambian el [`ProductFilter`] de la página, que siguen
//! todas las cuadrículas de productos. Las pestañas filtran solo su propia
//! cuadrícula.

use domain::Product;
use leptos::prelude::*;

use crate::ProductCard;

/// Lo que eligió el comprador para ver, por contexto: las cuadrículas de la
/// página muestran solo esos productos. Está en la tienda y en la vista
/// previa; en el lienzo del editor los filtros se ven pero no filtran.
/// Cómo se dibuja la cuadrícula de los diseños nuevos.
#[derive(Clone, Copy)]
pub struct NwGrid {
    pub cols: i64,
    pub show_category: bool,
    pub show_installments: bool,
    pub favorite: bool,
}

#[derive(Clone, Copy)]
pub struct ProductFilter {
    /// Etiquetas elegidas (vacío: todas). Con varias, alcanza con una.
    pub tags: RwSignal<Vec<String>>,
    /// Lo que se escribió en el buscador.
    pub query: RwSignal<String>,
    /// Categoría elegida (vacío: todas) y su subcategoría (vacío: ninguna).
    pub category: RwSignal<String>,
    pub sub: RwSignal<String>,
    /// Página de la cuadrícula (0 es la primera). Va en `?pagina=` (desde 1).
    pub page: RwSignal<usize>,
    /// La página va en la dirección (en la tienda, no en el editor).
    pub url: bool,
    /// Ya hay una cuadrícula que usa `page`: las demás de la página llevan
    /// su propia página (si no, pasaban todas juntas).
    pub page_taken: StoredValue<bool>,
}

impl ProductFilter {
    pub fn new() -> Self {
        Self {
            tags: RwSignal::new(Vec::new()),
            query: RwSignal::new(String::new()),
            category: RwSignal::new(String::new()),
            sub: RwSignal::new(String::new()),
            page: RwSignal::new(0),
            url: false,
            page_taken: StoredValue::new(false),
        }
    }

    pub fn provide() {
        provide_context(Self::new());
    }

    /// Como [`Self::provide`], y además toma la búsqueda de `?buscar=` (la
    /// manda el buscador del menú desde una página sin productos), la
    /// categoría de `?categoria=` y la página de `?pagina=`. Se lee
    /// después de hidratar, así la página del servidor y la del navegador
    /// arrancan iguales.
    pub fn provide_from_url() {
        let f = Self {
            url: true,
            ..Self::new()
        };
        provide_context(f);
        Effect::new(move |_| {
            let Ok(search) = window().location().search() else {
                return;
            };
            let Ok(params) = web_sys::UrlSearchParams::new_with_str(&search) else {
                return;
            };
            if let Some(q) = params.get("buscar").filter(|q| !q.trim().is_empty()) {
                f.query.set(q);
            }
            // Los enlaces "Categoría" del editor (`domain::category_href`).
            if let Some(c) = params.get("categoria").filter(|c| !c.trim().is_empty()) {
                let sub = params.get("subcategoria").unwrap_or_default();
                f.pick_category(c.trim(), sub.trim());
            }
            let tags: Vec<String> = params
                .get_all("etiqueta")
                .iter()
                .filter_map(|t| t.as_string())
                .collect();
            let tags = domain::clean_tags(tags.iter().map(String::as_str));
            if !tags.is_empty() {
                f.tags.set(tags);
                f.reveal();
            }
            if let Some(n) = params.get("pagina").and_then(|n| n.parse::<usize>().ok()) {
                f.page.set(n.saturating_sub(1));
            }
        });
        // Lo elegido queda en la dirección: se puede copiar y mandar el
        // enlace de una búsqueda, y al recargar no se pierde. No suma pasos
        // al botón Atrás.
        Effect::new(move |_| {
            let (query, tags) = (f.query.get(), f.tags.get());
            let (cat, sub) = (f.category.get(), f.sub.get());
            crate::edit_url(false, |params| {
                let put = |key: &str, value: &str| {
                    if value.trim().is_empty() {
                        params.delete(key);
                    } else {
                        params.set(key, value.trim());
                    }
                };
                put("buscar", &query);
                put("categoria", &cat);
                put("subcategoria", if cat.is_empty() { "" } else { &sub });
                params.delete("etiqueta");
                for t in &tags {
                    params.append("etiqueta", t);
                }
            });
        });
    }

    fn clear(self) {
        self.tags.set(Vec::new());
        self.query.set(String::new());
        self.pick_category("", "");
    }

    /// Deja elegida la categoría `cat` y su subcategoría `sub` (vacías: todas).
    pub(crate) fn pick_category(self, cat: &str, sub: &str) {
        self.category.set(cat.to_string());
        self.sub.set(sub.to_string());
        self.reveal();
    }

    /// En la tienda, si la cuadrícula filtrada no está a la vista, la pantalla
    /// baja (o sube) hasta ella: si el filtro está lejos, se ve que pasó algo.
    fn reveal(self) {
        if !self.url {
            return;
        }
        request_animation_frame(|| {
            let Some(grid) = document().query_selector(".mk-pgrid").ok().flatten() else {
                return;
            };
            let r = grid.get_bounding_client_rect();
            let vh = window()
                .inner_height()
                .ok()
                .and_then(|v| v.as_f64())
                .unwrap_or(800.0);
            if r.top() < 0.0 || r.top() > vh * 0.6 {
                let opts = web_sys::ScrollIntoViewOptions::new();
                let calm = window()
                    .match_media("(prefers-reduced-motion: reduce)")
                    .ok()
                    .flatten()
                    .is_some_and(|m| m.matches());
                opts.set_behavior(if calm {
                    web_sys::ScrollBehavior::Auto
                } else {
                    web_sys::ScrollBehavior::Smooth
                });
                opts.set_block(web_sys::ScrollLogicalPosition::Start);
                grid.scroll_into_view_with_scroll_into_view_options(&opts);
            }
        });
    }
}

impl Default for ProductFilter {
    fn default() -> Self {
        Self::new()
    }
}

/// Los productos de una cuadrícula: los que pasan el filtro, de a `limit`
/// por página, con un aviso arriba de qué se está viendo. Con `paged`, si
/// hay más de `limit` abajo aparecen los números de página.
#[component]
pub fn FilteredProducts(
    products: Vec<Product>,
    limit: usize,
    filter: Option<ProductFilter>,
    /// Mostrar «3 productos con «Hogar»» arriba cuando hay filtro.
    #[prop(default = true)]
    summary: bool,
    /// Paginar en lugar de cortar en `limit`.
    #[prop(default = false)]
    paged: bool,
    /// El filtro de la página, cuando `filter` es uno propio (las pestañas):
    /// la búsqueda y la categoría del menú valen también acá. Si no, al
    /// buscar la pantalla baja a una cuadrícula que no cambió.
    #[prop(optional_no_strip)]
    outer: Option<ProductFilter>,
    /// Con esto, las tarjetas y la cuadrícula son las de los diseños nuevos
    /// (`nw-products`): columnas, categoría, cuotas y corazón.
    #[prop(optional_no_strip)]
    nw: Option<NwGrid>,
) -> impl IntoView {
    let empty = products.is_empty();
    let limit = limit.max(1);
    // Las columnas se reparten según cuántos productos hay: si no, con 4
    // productos en una fila de 6 columnas queda un tercio vacío.
    let shown_count = products.len().min(limit);
    let few = (1..=3).contains(&shown_count) && limit >= 4;
    // Los agotados van al final (el orden entre los demás se mantiene).
    let mut products = products;
    products.sort_by_key(|p| !p.in_stock);
    let products = StoredValue::new(products);
    let chosen = move || filter.map_or_else(Default::default, |f| (f.tags.get(), f.query.get()));
    let category =
        move || filter.map_or_else(Default::default, |f| (f.category.get(), f.sub.get()));
    let outer_chosen =
        move || outer.map_or_else(Default::default, |f| (f.tags.get(), f.query.get()));
    let outer_category =
        move || outer.map_or_else(Default::default, |f| (f.category.get(), f.sub.get()));
    let matching = Memo::new(move |_| {
        let (tags, query) = chosen();
        let (cat, sub) = category();
        let (otags, oquery) = outer_chosen();
        let (ocat, osub) = outer_category();
        products.with_value(|ps| {
            ps.iter()
                .filter(|p| domain::passes_filter(p, &tags, &query))
                .filter(|p| domain::passes_category(p, &cat, &sub))
                .filter(|p| domain::passes_filter(p, &otags, &oquery))
                .filter(|p| domain::passes_category(p, &ocat, &osub))
                .cloned()
                .collect::<Vec<_>>()
        })
    });
    // La primera cuadrícula con páginas usa la del filtro (la de `?pagina=`);
    // las demás, una propia que no va en la dirección.
    let (page, url) = match filter {
        Some(f) if paged && !f.page_taken.get_value() => {
            f.page_taken.set_value(true);
            (f.page, f.url)
        }
        _ => (RwSignal::new(0), false),
    };
    let pages = move || {
        if paged {
            domain::page_count(matching.with(Vec::len), limit)
        } else {
            1
        }
    };
    let current = move || page.get().min(pages() - 1);
    // Otro filtro: vuelve a la primera página (la primera vez no, así se
    // respeta la `?pagina=` con la que se entró).
    Effect::new(move |seen: Option<()>| {
        chosen();
        category();
        outer_chosen();
        outer_category();
        if seen.is_some() {
            page.set(0);
        }
    });
    let shown = move || {
        let skip = current() * limit;
        matching.with(|ps| {
            ps.iter()
                .skip(skip)
                .take(limit)
                .cloned()
                .collect::<Vec<_>>()
        })
    };
    let notice =
        move || {
            // Con el filtro de la página encima, el aviso habla de ese (el propio
            // ya se ve en las pestañas). Cuenta lo mismo que se muestra.
            let (tags, query) = if outer.is_some() {
                outer_chosen()
            } else {
                chosen()
            };
            let text = domain::filter_summary(matching.with(Vec::len), &tags, &query)?;
            (summary || outer.is_some()).then(|| view! {
            <p class="mk-filtered -mt-1 flex flex-wrap items-center gap-x-3 text-sm opacity-80">
                <span>{text}</span>
                <button type="button" class="link link-hover font-medium"
                    on:click=move |_| {
                        if let Some(f) = filter { f.clear(); }
                        if let Some(f) = outer { f.clear(); }
                    }>"Ver todos"</button>
            </p>
        })
        };
    let grid = NodeRef::<leptos::html::Div>::new();
    let go = move |n: usize| {
        page.set(n);
        if url {
            set_page_in_url(n);
        }
        // Que se vea el principio de la página nueva, no el final de la anterior.
        if let Some(el) = grid.get_untracked() {
            let top = el.get_bounding_client_rect().top();
            if top < 0.0 {
                let opts = web_sys::ScrollIntoViewOptions::new();
                opts.set_behavior(web_sys::ScrollBehavior::Smooth);
                opts.set_block(web_sys::ScrollLogicalPosition::Start);
                el.scroll_into_view_with_scroll_into_view_options(&opts);
            }
        }
    };
    view! {
        <div class="contents" aria-live="polite">{notice}</div>
        {empty.then(|| view! { <div class="alert">"Todavía no hay productos."</div> })}
        // Con pocos productos (y lugar para más), cada tarjeta guarda el tamaño
        // de una fila de 4 y quedan centradas: si no, cada una ocupa media pantalla.
        {match nw {
            Some(g) => view! {
                <div node_ref=grid class="nw-products" style=format!("--n:{}", g.cols)>
                    {move || shown().into_iter().map(|p| view! {
                        <crate::product_page::NwCard product=p show_category=g.show_category
                            show_installments=g.show_installments favorite=g.favorite/>
                    }).collect_view()}
                </div>
            }.into_any(),
            None => view! {
                <div node_ref=grid class=format!("mk-pgrid grid min-h-0 flex-1 auto-rows-fr scroll-mt-20 gap-3 {}{}", fill_cols(shown_count), if few { " mk-pgrid-few" } else { "" })
                    style=few.then(|| format!("--mk-few: {shown_count}"))>
                    {move || shown().into_iter().map(|p| view! { <ProductCard product=p show_price=true/> }).collect_view()}
                </div>
            }.into_any(),
        }}
        {move || (pages() > 1).then(|| view! { <Pager current=current() pages=pages() go/> })}
    }
}

/// Los números de página abajo de la cuadrícula: círculos, el actual lleno, y
/// flechas que se apagan en la primera y la última. Con muchas páginas, «…».
#[component]
fn Pager(
    current: usize,
    pages: usize,
    go: impl Fn(usize) + Copy + Send + Sync + 'static,
) -> impl IntoView {
    let items = domain::page_items(current, pages);
    view! {
        <nav class="mk-pager flex shrink-0 flex-wrap items-center justify-center gap-1" aria-label="Páginas de productos">
            <button type="button" class="btn btn-ghost btn-circle" aria-label="Página anterior"
                disabled={current == 0} on:click=move |_| go(current.saturating_sub(1))>
                <span aria-hidden="true">"‹"</span>
            </button>
            {items.into_iter().map(|item| match item {
                Some(n) if n == current => view! {
                    <button type="button" class="btn btn-circle btn-primary" aria-current="page"
                        aria-label=format!("Página {}", n + 1)>{(n + 1).to_string()}</button>
                }.into_any(),
                Some(n) => view! {
                    <button type="button" class="btn btn-ghost btn-circle" aria-label=format!("Página {}", n + 1)
                        on:click=move |_| go(n)>{(n + 1).to_string()}</button>
                }.into_any(),
                None => view! { <span class="px-1 opacity-60" aria-hidden="true">"…"</span> }.into_any(),
            }).collect_view()}
            <button type="button" class="btn btn-ghost btn-circle" aria-label="Página siguiente"
                disabled={current + 1 >= pages} on:click=move |_| go(current + 1)>
                <span aria-hidden="true">"›"</span>
            </button>
        </nav>
    }
}

/// Deja `?pagina=` en la dirección sin agregar un paso al «atrás»: si se
/// recarga o se comparte, abre en la misma página. La primera no lo lleva.
fn set_page_in_url(n: usize) {
    crate::edit_url(false, |params| {
        if n == 0 {
            params.delete("pagina");
        } else {
            params.set("pagina", &(n + 1).to_string());
        }
    });
}

/// Cantidad al lado de una etiqueta (y en palabras para el lector de pantalla).
pub(crate) fn count_badge(show: bool, n: usize) -> impl IntoView {
    show.then(|| {
        view! {
            <span class="mk-chip-n" aria-hidden="true">{n}</span>
            <span class="sr-only">{if n == 1 { ", 1 producto".to_string() } else { format!(", {n} productos") }}</span>
        }
    })
}

pub(crate) fn label_or(text: &str, default: &str) -> String {
    if text.trim().is_empty() {
        default.to_string()
    } else {
        text.to_string()
    }
}

/// Columnas de una cuadrícula con `n` productos: que la fila llene el ancho.
pub fn fill_cols(n: usize) -> &'static str {
    match n {
        0 | 1 => "grid-cols-1 @sm:grid-cols-2",
        2 => "grid-cols-2",
        3 => "grid-cols-2 @lg:grid-cols-3",
        4 => "grid-cols-2 @3xl:grid-cols-4",
        5 => "grid-cols-2 @lg:grid-cols-3 @3xl:grid-cols-5",
        6 => "grid-cols-2 @lg:grid-cols-3 @5xl:grid-cols-6",
        8 | 16 => "grid-cols-2 @3xl:grid-cols-4",
        9 => "grid-cols-2 @lg:grid-cols-3",
        10 => "grid-cols-2 @3xl:grid-cols-5",
        12 => "grid-cols-2 @lg:grid-cols-3 @3xl:grid-cols-4 @5xl:grid-cols-6",
        _ => "grid-cols-2 @lg:grid-cols-3 @3xl:grid-cols-4",
    }
}
