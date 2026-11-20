//! Bloques para filtrar productos por etiqueta: en fila ([`Widget::TagFilter`]),
//! en columna ([`Widget::TagList`]), con foto ([`Widget::TagCircles`]), un
//! buscador con menú ([`Widget::TagSearch`]) y una cuadrícula con pestañas
//! ([`Widget::TagTabs`]).
//!
//! Los cuatro primeros cambian el [`ProductFilter`] de la página, que siguen
//! todas las cuadrículas de productos. Las pestañas filtran solo su propia
//! cuadrícula.

use domain::{Product, TagCount, TagFilterProps, Widget};
use leptos::prelude::*;

use crate::{ProductCard, RenderCtx};

/// Lo que eligió el comprador para ver, por contexto: las cuadrículas de la
/// página muestran solo esos productos. Está en la tienda y en la vista
/// previa; en el lienzo del editor los filtros se ven pero no filtran.
#[derive(Clone, Copy)]
pub struct ProductFilter {
    /// Etiquetas elegidas (vacío: todas). Con varias, alcanza con una.
    pub tags: RwSignal<Vec<String>>,
    /// Lo que se escribió en el buscador.
    pub query: RwSignal<String>,
}

impl ProductFilter {
    pub fn new() -> Self {
        Self {
            tags: RwSignal::new(Vec::new()),
            query: RwSignal::new(String::new()),
        }
    }

    pub fn provide() {
        provide_context(Self::new());
    }

    /// Como [`Self::provide`], y además toma la búsqueda de `?buscar=` (la
    /// manda el buscador del menú desde una página sin productos). Se lee
    /// después de hidratar, así la página del servidor y la del navegador
    /// arrancan iguales.
    pub fn provide_from_url() {
        let f = Self::new();
        provide_context(f);
        Effect::new(move |_| {
            let Ok(search) = window().location().search() else { return };
            let Ok(params) = web_sys::UrlSearchParams::new_with_str(&search) else { return };
            if let Some(q) = params.get("buscar").filter(|q| !q.trim().is_empty()) {
                f.query.set(q);
            }
        });
    }

    fn clear(self) {
        self.tags.set(Vec::new());
        self.query.set(String::new());
    }

    /// Deja elegida solo `tag` (`None`: todas).
    fn pick(self, tag: Option<String>) {
        self.tags.set(tag.into_iter().collect());
    }

    /// Agrega o saca `tag` de las elegidas.
    fn toggle(self, tag: &str) {
        self.tags.update(|l| {
            if let Some(i) = l.iter().position(|t| t == tag) {
                l.remove(i);
            } else {
                l.push(tag.to_string());
            }
        });
    }

    fn has(self, tag: &str) -> bool {
        self.tags.with(|l| l.iter().any(|t| t == tag))
    }

    /// ¿Está elegida justo esta etiqueta (o ninguna, con `None`)?
    fn is_only(self, tag: Option<&str>) -> bool {
        self.tags.with(|l| match tag {
            None => l.is_empty(),
            Some(t) => l.len() == 1 && l[0] == t,
        })
    }
}

impl Default for ProductFilter {
    fn default() -> Self {
        Self::new()
    }
}

pub(crate) fn render(widget: &Widget, ctx: &RenderCtx) -> AnyView {
    let products = ctx.products.clone();
    let editing = ctx.editing;
    match widget {
        Widget::TagFilter(p) => view! { <TagRow props=p.clone() products editing/> }.into_any(),
        Widget::TagList(p) => view! { <TagColumn props=p.clone() products editing/> }.into_any(),
        Widget::TagCircles(p) => view! { <TagPhotos props=p.clone() products editing/> }.into_any(),
        Widget::TagSearch(p) => view! { <TagSearch props=p.clone() products editing/> }.into_any(),
        Widget::TagTabs(p) => view! { <TagTabs props=p.clone() products editing/> }.into_any(),
        _ => ().into_any(),
    }
}

/// Los productos de una cuadrícula: los primeros `limit` de los que pasan
/// el filtro, con un aviso arriba de qué se está viendo.
#[component]
pub fn FilteredProducts(
    products: Vec<Product>,
    limit: usize,
    filter: Option<ProductFilter>,
    /// Mostrar «3 productos con «Hogar»» arriba cuando hay filtro.
    #[prop(default = true)]
    summary: bool,
) -> impl IntoView {
    let empty = products.is_empty();
    // Las columnas se reparten según cuántos productos hay: si no, con 4
    // productos en una fila de 6 columnas queda un tercio vacío.
    let shown_count = products.len().min(limit);
    let few = (1..=3).contains(&shown_count) && limit >= 4;
    // Los agotados van al final (el orden entre los demás se mantiene).
    let mut products = products;
    products.sort_by_key(|p| !p.in_stock);
    let products = StoredValue::new(products);
    let chosen = move || filter.map_or_else(Default::default, |f| (f.tags.get(), f.query.get()));
    let shown = move || {
        let (tags, query) = chosen();
        products.with_value(|ps| {
            ps.iter()
                .filter(|p| domain::passes_filter(p, &tags, &query))
                .take(limit)
                .cloned()
                .collect::<Vec<_>>()
        })
    };
    let notice = move || {
        let (tags, query) = chosen();
        let n = products.with_value(|ps| {
            ps.iter()
                .filter(|p| domain::passes_filter(p, &tags, &query))
                .count()
        });
        let text = domain::filter_summary(n, &tags, &query)?;
        summary.then(|| view! {
            <p class="mk-filtered -mt-1 flex flex-wrap items-center gap-x-3 text-sm opacity-80">
                <span>{text}</span>
                <button type="button" class="link link-hover font-medium"
                    on:click=move |_| { if let Some(f) = filter { f.clear(); } }>"Ver todos"</button>
            </p>
        })
    };
    view! {
        <div class="contents" aria-live="polite">{notice}</div>
        {empty.then(|| view! { <div class="alert">"Todavía no hay productos."</div> })}
        // Con pocos productos (y lugar para más), cada tarjeta guarda el tamaño
        // de una fila de 4 y quedan centradas: si no, cada una ocupa media pantalla.
        <div class=format!("mk-pgrid grid min-h-0 flex-1 auto-rows-fr gap-3 {}{}", fill_cols(shown_count), if few { " mk-pgrid-few" } else { "" })
            style=few.then(|| format!("--mk-few: {shown_count}"))>
            {move || shown().into_iter().map(|p| view! { <ProductCard product=p show_price=true/> }).collect_view()}
        </div>
    }
}

/// Sin etiquetas: en la tienda no se muestra nada; en el editor, cómo llenarlo.
fn no_tags(editing: bool) -> AnyView {
    if editing {
        view! {
            <div class="flex h-full items-center justify-center rounded-box border border-dashed border-base-300 bg-base-200 p-3 text-center text-sm">
                "Tus productos todavía no tienen etiquetas. Ponelas en Productos y aparecen acá."
            </div>
        }
        .into_any()
    } else {
        ().into_any()
    }
}

fn group_label(title: &str) -> String {
    if title.is_empty() {
        "Filtrar productos por etiqueta".to_string()
    } else {
        title.to_string()
    }
}

fn count(t: &TagCount) -> usize {
    usize::try_from(t.products).unwrap_or(0)
}

/// Cantidad al lado de una etiqueta (y en palabras para el lector de pantalla).
fn count_badge(show: bool, n: usize) -> impl IntoView {
    show.then(|| {
        view! {
            <span class="mk-chip-n" aria-hidden="true">{n}</span>
            <span class="sr-only">{if n == 1 { ", 1 producto".to_string() } else { format!(", {n} productos") }}</span>
        }
    })
}

/// A · Las etiquetas en una fila de pastillas; se elige una a la vez.
#[component]
fn TagRow(props: TagFilterProps, products: Vec<Product>, editing: bool) -> impl IntoView {
    let tags = domain::count_tags(&products);
    if tags.is_empty() {
        return no_tags(editing);
    }
    let filter = use_context::<ProductFilter>();
    let counts = props.show_counts;
    let chip = move |text: String, tag: Option<String>, n: usize| {
        let pressed = {
            let tag = tag.clone();
            move || filter.map_or(tag.is_none(), |f| f.is_only(tag.as_deref()))
        };
        view! {
            <button type="button" class="mk-chip"
                tabindex=editing.then_some("-1")
                aria-pressed=move || pressed().to_string()
                on:click=move |_| { if let Some(f) = filter { f.pick(tag.clone()); } }>
                {text}
                {count_badge(counts, n)}
            </button>
        }
    };
    let all = label_or(&props.all_label, "Todos");
    view! {
        <div class="mk-box mk-pad mk-tagrow flex h-full flex-col justify-center gap-2 overflow-hidden rounded-box">
            {(!props.title.is_empty()).then(|| view! {
                <h2 class="mk-title text-lg font-bold truncate">{props.title.clone()}</h2>
            })}
            <div class="flex gap-2 overflow-x-auto py-1" role="group" aria-label=group_label(&props.title)>
                {chip(all, None, products.len())}
                {tags.into_iter().map(|t| {
                    let n = count(&t);
                    chip(domain::tag_label(&t.tag), Some(t.tag), n)
                }).collect_view()}
            </div>
        </div>
    }
    .into_any()
}

fn label_or(text: &str, default: &str) -> String {
    if text.trim().is_empty() {
        default.to_string()
    } else {
        text.to_string()
    }
}

/// B · Una columna con casillas; se pueden marcar varias.
#[component]
fn TagColumn(props: TagFilterProps, products: Vec<Product>, editing: bool) -> impl IntoView {
    let tags = domain::count_tags(&products);
    if tags.is_empty() {
        return no_tags(editing);
    }
    let filter = use_context::<ProductFilter>();
    let counts = props.show_counts;
    let any = move || filter.is_some_and(|f| f.tags.with(|l| !l.is_empty()));
    let clear = label_or(&props.all_label, "Limpiar filtros");
    view! {
        <div class="mk-box mk-pad mk-tagcol flex h-full flex-col gap-1 overflow-hidden rounded-box">
            <fieldset class="flex min-h-0 flex-col gap-1">
                <legend class="mk-title mb-1 text-lg font-bold">{label_or(&props.title, "Filtrar")}</legend>
                <div class="flex min-h-0 flex-col overflow-y-auto pr-1">
                    {tags.into_iter().map(|t| {
                        let n = count(&t);
                        let tag = t.tag.clone();
                        let tag2 = t.tag.clone();
                        view! {
                            <label class="mk-tagcheck flex cursor-pointer items-center gap-3 py-1.5">
                                <input type="checkbox" class="checkbox checkbox-sm mk-check"
                                    tabindex=editing.then_some("-1")
                                    prop:checked=move || filter.is_some_and(|f| f.has(&tag))
                                    on:change=move |_| { if let Some(f) = filter { f.toggle(&tag2); } }/>
                                <span class="grow truncate">{domain::tag_label(&t.tag)}</span>
                                {count_badge(counts, n)}
                            </label>
                        }
                    }).collect_view()}
                </div>
            </fieldset>
            <div class="mk-tagcol-foot mt-auto pt-2">
                <button type="button" class="link link-hover text-sm font-medium"
                    tabindex=editing.then_some("-1")
                    class:invisible=move || !any() && !editing
                    on:click=move |_| { if let Some(f) = filter { f.pick(None); } }>{clear}</button>
            </div>
        </div>
    }
    .into_any()
}

/// E · Círculos con la foto de un producto de cada etiqueta, como las
/// historias de Instagram.
#[component]
fn TagPhotos(props: TagFilterProps, products: Vec<Product>, editing: bool) -> impl IntoView {
    let tags = domain::count_tags(&products);
    if tags.is_empty() {
        return no_tags(editing);
    }
    let filter = use_context::<ProductFilter>();
    let counts = props.show_counts;
    let photo = |tag: Option<&str>| {
        products
            .iter()
            .filter(|p| tag.is_none_or(|t| p.tags.iter().any(|x| x == t)))
            .find(|p| !p.image_url.is_empty())
            .map(|p| p.image_url.clone())
            .unwrap_or_default()
    };
    let circle = move |text: String, tag: Option<String>, src: String, n: usize| {
        let pressed = {
            let tag = tag.clone();
            move || filter.map_or(tag.is_none(), |f| f.is_only(tag.as_deref()))
        };
        view! {
            <button type="button" class="mk-circle"
                tabindex=editing.then_some("-1")
                aria-pressed=move || pressed().to_string()
                on:click=move |_| { if let Some(f) = filter { f.pick(tag.clone()); } }>
                <span class="mk-circle-ring">
                    <span class="mk-circle-img"><ui::Image src alt=""/></span>
                </span>
                <span class="mk-circle-label">{text}{count_badge(counts, n)}</span>
            </button>
        }
    };
    let all = circle(
        label_or(&props.all_label, "Todos"),
        None,
        photo(None),
        products.len(),
    );
    let items = tags
        .iter()
        .map(|t| {
            circle(
                domain::tag_label(&t.tag),
                Some(t.tag.clone()),
                photo(Some(&t.tag)),
                count(t),
            )
        })
        .collect_view();
    view! {
        <div class="mk-box mk-pad mk-circles @container flex h-full flex-col justify-center gap-2 overflow-hidden rounded-box">
            {(!props.title.is_empty()).then(|| view! {
                <h2 class="mk-title text-lg font-bold truncate">{props.title.clone()}</h2>
            })}
            <div class="mk-circles-row flex gap-4 overflow-x-auto py-1" role="group" aria-label=group_label(&props.title)>
                {all}
                {items}
            </div>
        </div>
    }
    .into_any()
}

/// F · Un buscador y un botón «Filtrar» que abre un menú con casillas. Las
/// etiquetas elegidas quedan como pastillas que se sacan con ✕.
#[component]
fn TagSearch(
    props: domain::TagSearchProps,
    products: Vec<Product>,
    editing: bool,
) -> impl IntoView {
    let tags = domain::count_tags(&products);
    let filter = use_context::<ProductFilter>();
    let counts = props.show_counts;
    let menu = NodeRef::<leptos::html::Div>::new();
    let button = NodeRef::<leptos::html::Button>::new();
    let chosen = move || filter.map(|f| f.tags.get()).unwrap_or_default();
    // El menú es un `popover` (capa superior), así no lo recorta el bloque;
    // se ubica debajo del botón al abrirlo.
    let open_menu = move |_| {
        if editing {
            return;
        }
        let (Some(m), Some(b)) = (menu.get(), button.get()) else {
            return;
        };
        let r = b.get_bounding_client_rect();
        _ = m.set_attribute(
            "style",
            &format!("top: {}px; left: {}px", r.bottom() + 6.0, r.left()),
        );
        _ = m.toggle_popover();
    };
    let has_tags = !tags.is_empty();
    let menu_id = format!("mk-tagmenu-{}", products.len());
    view! {
        <div class="mk-box mk-pad mk-tagsearch flex h-full items-center gap-3 overflow-hidden rounded-box">
            <label class="input mk-search w-72 max-w-[45%] shrink-0">
                <span class="opacity-60" aria-hidden="true">"⌕"</span>
                <input type="search" placeholder=label_or(&props.placeholder, "Buscar productos")
                    aria-label="Buscar productos"
                    tabindex=editing.then_some("-1")
                    prop:value=move || filter.map(|f| f.query.get()).unwrap_or_default()
                    on:input=move |ev| { if let Some(f) = filter { f.query.set(event_target_value(&ev)); } }/>
            </label>
            {has_tags.then(|| view! {
                <button type="button" node_ref=button class="mk-filterbtn shrink-0"
                    tabindex=editing.then_some("-1")
                    aria-haspopup="true"
                    aria-controls=menu_id.clone()
                    on:click=open_menu>
                    {move || match chosen().len() {
                        0 => "Filtrar ▾".to_string(),
                        n => format!("Filtrar · {n} ▾"),
                    }}
                </button>
            })}
            <div class="flex min-w-0 gap-2 overflow-x-auto py-1">
                {move || chosen().into_iter().map(|t| {
                    let t2 = t.clone();
                    view! {
                        <button type="button" class="mk-pill" aria-label=format!("Sacar el filtro {}", domain::tag_label(&t))
                            on:click=move |_| { if let Some(f) = filter { f.toggle(&t2); } }>
                            {domain::tag_label(&t)} <span aria-hidden="true">"✕"</span>
                        </button>
                    }
                }).collect_view()}
            </div>
            <div node_ref=menu id=menu_id popover="auto" class="mk-tagmenu" role="group" aria-label="Filtrar por etiqueta">
                {tags.into_iter().map(|t| {
                    let n = count(&t);
                    let tag = t.tag.clone();
                    let tag2 = t.tag.clone();
                    view! {
                        <label class="flex cursor-pointer items-center gap-3 px-4 py-2 hover:bg-base-200">
                            <input type="checkbox" class="checkbox checkbox-sm mk-check"
                                prop:checked=move || filter.is_some_and(|f| f.has(&tag))
                                on:change=move |_| { if let Some(f) = filter { f.toggle(&tag2); } }/>
                            <span class="grow">{domain::tag_label(&t.tag)}</span>
                            {count_badge(counts, n)}
                        </label>
                    }
                }).collect_view()}
                <div class="flex items-center justify-between gap-3 border-t border-base-300 px-4 pt-2 pb-1">
                    <button type="button" class="link link-hover text-sm"
                        on:click=move |_| { if let Some(f) = filter { f.pick(None); } }>"Limpiar"</button>
                    <button type="button" class="btn btn-sm mk-btn"
                        on:click=move |_| { if let Some(m) = menu.get() { _ = m.hide_popover(); } }>"Listo"</button>
                </div>
            </div>
        </div>
    }
    .into_any()
}

/// D · Una cuadrícula de productos con las etiquetas como pestañas arriba.
/// Filtra solo su propia cuadrícula.
#[component]
fn TagTabs(props: domain::TagTabsProps, products: Vec<Product>, editing: bool) -> impl IntoView {
    let tags = domain::count_tags(&products);
    let local = ProductFilter::new();
    let limit = usize::try_from(props.limit).unwrap_or(0);
    let tab = move |text: String, tag: Option<String>| {
        let pressed = {
            let tag = tag.clone();
            move || local.is_only(tag.as_deref())
        };
        view! {
            <button type="button" class="mk-tab"
                tabindex=editing.then_some("-1")
                aria-pressed=move || pressed().to_string()
                on:click=move |_| local.pick(tag.clone())>
                {text}
            </button>
        }
    };
    let all = tab(label_or(&props.all_label, "Todos"), None);
    let tabs = tags
        .into_iter()
        .map(|t| tab(domain::tag_label(&t.tag), Some(t.tag)))
        .collect_view();
    view! {
        <div class="mk-box mk-pad mk-tagtabs @container flex h-full flex-col gap-3 overflow-hidden rounded-box">
            <div class="mk-tabs-head flex items-end gap-8">
                {(!props.title.is_empty()).then(|| view! {
                    <h2 class="mk-title shrink-0 pb-2 text-xl font-bold">{props.title.clone()}</h2>
                })}
                <div class="flex min-w-0 grow gap-6 overflow-x-auto" role="group" aria-label="Ver por etiqueta">
                    {all}
                    {tabs}
                </div>
            </div>
            <FilteredProducts products limit filter=Some(local) summary=false/>
        </div>
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
