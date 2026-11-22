//! Cinco bloques para filtrar productos por categoría: pestañas
//! ([`Widget::CategoryTabs`]), columna al costado ([`Widget::CategorySide`]),
//! tarjetas con foto ([`Widget::CategoryCards`]), un menú desplegable
//! ([`Widget::CategoryDrop`]) y una página con migas y subcategorías en
//! círculos ([`Widget::CategoryPage`]).
//!
//! Todos cambian la categoría del [`ProductFilter`] de la página, que siguen
//! todas las cuadrículas de productos. «Todas» queda siempre fija primero;
//! el resto se desliza de costado cuando no entra.

use crate::AnimText;
use domain::{CategoryCount, CategoryFilterProps, Product, Widget};
use leptos::prelude::*;

use crate::tags::{count_badge, group_label, label_or, ProductFilter};
use crate::RenderCtx;

pub(crate) fn render(widget: &Widget, ctx: &RenderCtx) -> AnyView {
    let products = ctx.products.clone();
    let editing = ctx.editing;
    match widget {
        Widget::CategoryTabs(p) => {
            view! { <CategoryTabs props=p.clone() products editing/> }.into_any()
        }
        Widget::CategorySide(p) => {
            view! { <CategorySide props=p.clone() products editing/> }.into_any()
        }
        Widget::CategoryCards(p) => {
            view! { <CategoryCards props=p.clone() products editing/> }.into_any()
        }
        Widget::CategoryDrop(p) => {
            view! { <CategoryDrop props=p.clone() products editing/> }.into_any()
        }
        Widget::CategoryPage(p) => {
            view! { <CategoryPage props=p.clone() products editing/> }.into_any()
        }
        _ => ().into_any(),
    }
}

/// Sin categorías: en la tienda no se muestra nada; en el editor, cómo llenarlo.
fn no_cats(editing: bool) -> AnyView {
    if editing {
        view! {
            <div class="flex h-full items-center justify-center rounded-box border border-dashed border-base-300 bg-base-200 p-3 text-center text-sm">
                "Tus productos todavía no tienen categoría. Elegila en Productos y aparece acá."
            </div>
        }
        .into_any()
    } else {
        ().into_any()
    }
}

/// Tarjetas sin categorías todavía: en el editor se ve la forma del bloque con
/// «Tu categoría» (sin fotos de muestra de otro rubro); en la tienda, nada.
fn no_cat_cards(title: &str, editing: bool) -> AnyView {
    if !editing {
        return ().into_any();
    }
    view! {
        <div class="mk-box mk-pad mk-catcards @container flex h-full flex-col gap-3 overflow-hidden rounded-box">
            <h2 class="mk-title text-lg font-bold truncate"><AnimText text={label_or(title, "Categorías")}/></h2>
            <div class="grid min-h-0 flex-1 auto-rows-max content-start grid-cols-2 gap-3 @lg:grid-cols-3">
                {(0..3).map(|_| view! {
                    <div class="flex aspect-[4/3] items-center justify-center rounded-box border border-dashed border-base-content/25 bg-base-200 text-sm font-semibold opacity-70">
                        "Tu categoría"
                    </div>
                }).collect_view()}
            </div>
            <p class="text-xs opacity-60">"Elegí la categoría de cada producto en Productos y aparecen acá con su foto."</p>
        </div>
    }
    .into_any()
}

impl ProductFilter {
    /// ¿Está elegida justo esta categoría y subcategoría? (Vacías: ninguna.)
    fn is_cat(self, cat: &str, sub: &str) -> bool {
        self.category.with(|c| c == cat) && self.sub.with(|s| s == sub)
    }

    /// ¿Está elegida la categoría `cat`, con cualquier subcategoría?
    fn in_cat(self, cat: &str) -> bool {
        self.category.with(|c| c == cat)
    }
}

/// La primera foto de los productos de la categoría (y subcategoría).
fn photo(products: &[Product], cat: &str, sub: &str) -> String {
    products
        .iter()
        .filter(|p| domain::passes_category(p, cat, sub))
        .find(|p| !p.image_url.is_empty())
        .map(|p| p.image_url.clone())
        .unwrap_or_default()
}

fn find<'a>(cats: &'a [CategoryCount], name: &str) -> Option<&'a CategoryCount> {
    cats.iter().find(|c| c.name == name)
}

/// 2 · Pastillas: «Todas» fija a la izquierda y las categorías deslizándose.
#[component]
fn CategoryTabs(
    props: CategoryFilterProps,
    products: Vec<Product>,
    editing: bool,
) -> impl IntoView {
    let cats = store_categories(&products);
    if cats.is_empty() {
        return no_cats(editing);
    }
    let filter = use_context::<ProductFilter>();
    let counts = props.show_counts;
    let chip = move |text: String, cat: String, n: usize| {
        let (pressed_cat, click_cat) = (cat.clone(), cat);
        view! {
            <button type="button" class="mk-chip"
                tabindex=editing.then_some("-1")
                aria-pressed=move || filter.map_or(pressed_cat.is_empty(), |f| f.in_cat(&pressed_cat)).to_string()
                on:click=move |_| { if let Some(f) = filter { f.pick_category(&click_cat, ""); } }>
                {text}
                {count_badge(counts, n)}
            </button>
        }
    };
    view! {
        <div class="mk-box mk-pad mk-catrow flex h-full flex-col justify-center gap-2 overflow-hidden rounded-box">
            {(!props.title.is_empty()).then(|| view! {
                <h2 class="mk-title text-lg font-bold truncate"><AnimText text={props.title.clone()}/></h2>
            })}
            <div class="flex items-center gap-2 py-1" role="group" aria-label=group_label_cat(&props.title)>
                {chip(label_or(&props.all_label, "Todas"), String::new(), products.len())}
                <div class="mk-scrollrow flex min-w-0 grow gap-2 relative overflow-x-auto py-1">
                    {cats.into_iter().map(|c| { let n = c.products; chip(c.name.clone(), c.name, n) }).collect_view()}
                </div>
            </div>
        </div>
    }
    .into_any()
}

fn group_label_cat(title: &str) -> String {
    if title.is_empty() {
        "Ver productos por categoría".to_string()
    } else {
        group_label(title)
    }
}

/// 1 · Columna al costado, con cantidades y las subcategorías de la elegida.
#[component]
fn CategorySide(
    props: CategoryFilterProps,
    products: Vec<Product>,
    editing: bool,
) -> impl IntoView {
    let cats = store_categories(&products);
    if cats.is_empty() {
        return no_cats(editing);
    }
    let filter = use_context::<ProductFilter>();
    let counts = props.show_counts;
    let row = move |text: String, cat: String, sub: String, n: usize, indent: bool| {
        let (pc, ps) = (cat.clone(), sub.clone());
        view! {
            <button type="button" class="mk-catrow-btn" class:ml-4=indent
                tabindex=editing.then_some("-1")
                aria-pressed=move || filter.map_or(pc.is_empty() && ps.is_empty(), |f| f.is_cat(&pc, &ps)).to_string()
                on:click=move |_| { if let Some(f) = filter { f.pick_category(&cat, &sub); } }>
                <span class="truncate">{text}</span>
                {count_badge(counts, n)}
            </button>
        }
    };
    let cats2 = cats.clone();
    view! {
        <nav class="mk-box mk-pad mk-catside flex h-full flex-col gap-0.5 overflow-y-auto rounded-box" aria-label=label_or(&props.title, "Categorías")>
            <h2 class="mk-title mb-1 text-sm font-bold uppercase tracking-wide opacity-70"><AnimText text={label_or(&props.title, "Categorías")}/></h2>
            {row(label_or(&props.all_label, "Todas"), String::new(), String::new(), products.len(), false)}
            {cats.into_iter().map(|c| {
                let name = c.name.clone();
                let name2 = c.name.clone();
                let cats = cats2.clone();
                view! {
                    {row(c.name.clone(), c.name.clone(), String::new(), c.products, false)}
                    {move || {
                        let open = filter.is_some_and(|f| f.in_cat(&name));
                        open.then(|| find(&cats, &name2).map(|c| c.subs.clone()).unwrap_or_default()
                            .into_iter()
                            .map(|(s, n)| row(s.clone(), name2.clone(), s, n, true))
                            .collect_view())
                    }}
                }
            }).collect_view()}
        </nav>
    }
    .into_any()
}

/// 3 · Tarjetas con la foto de un producto de cada categoría.
#[component]
fn CategoryCards(
    props: CategoryFilterProps,
    products: Vec<Product>,
    editing: bool,
) -> impl IntoView {
    // Solo las categorías de los productos propios: las de los productos de
    // ejemplo son de otro rubro (en la tienda esos ni se ven).
    let own: Vec<Product> = products.iter().filter(|p| !p.example).cloned().collect();
    let cats = store_categories(&own);
    if cats.is_empty() {
        return no_cat_cards(&props.title, editing);
    }
    let filter = use_context::<ProductFilter>();
    let counts = props.show_counts;
    let any = move || filter.is_some_and(|f| f.category.with(|c| !c.is_empty()));
    let all = label_or(&props.all_label, "Todas");
    view! {
        <div class="mk-box mk-pad mk-catcards @container flex h-full flex-col gap-3 overflow-hidden rounded-box">
            <div class="flex items-baseline justify-between gap-3">
                <h2 class="mk-title text-lg font-bold truncate"><AnimText text={label_or(&props.title, "Categorías")}/></h2>
                <button type="button" class="link link-hover text-sm font-medium"
                    tabindex=editing.then_some("-1")
                    class:invisible=move || !any() && !editing
                    on:click=move |_| { if let Some(f) = filter { f.pick_category("", ""); } }>{all}</button>
            </div>
            // Filas del alto de las tarjetas (`auto-rows-max`): con filas automáticas
            // en un alto fijo, la grilla las achicaba y las fotos se encimaban.
            // Lo que no entra se ve bajando dentro del bloque.
            <div class="grid min-h-0 flex-1 auto-rows-max content-start grid-cols-2 gap-3 overflow-y-auto @lg:grid-cols-3 @3xl:grid-cols-4" role="group" aria-label="Ver productos por categoría">
                {cats.into_iter().map(|c| {
                    let src = photo(&products, &c.name, "");
                    let (pressed, click) = (c.name.clone(), c.name.clone());
                    view! {
                        <button type="button" class="mk-catcard"
                            tabindex=editing.then_some("-1")
                            aria-pressed=move || filter.is_some_and(|f| f.in_cat(&pressed)).to_string()
                            on:click=move |_| { if let Some(f) = filter { f.pick_category(&click, ""); } }>
                            <span class="absolute inset-0 block"><ui::Image src alt=""/></span>
                            <span class="mk-catcard-text">
                                <span class="font-bold">{c.name.clone()}</span>
                                {count_badge(counts, c.products)}
                            </span>
                        </button>
                    }
                }).collect_view()}
            </div>
        </div>
    }
    .into_any()
}

/// 4 · Un botón «Categorías ▾» que abre un panel con todas y sus subcategorías.
#[component]
fn CategoryDrop(
    props: CategoryFilterProps,
    products: Vec<Product>,
    editing: bool,
) -> impl IntoView {
    let cats = store_categories(&products);
    if cats.is_empty() {
        return no_cats(editing);
    }
    let filter = use_context::<ProductFilter>();
    let counts = props.show_counts;
    let menu = NodeRef::<leptos::html::Div>::new();
    let button = NodeRef::<leptos::html::Button>::new();
    let title = label_or(&props.title, "Categorías");
    // Como el menú de filtros: un `popover` (capa superior) que no recorta el
    // bloque, ubicado debajo del botón al abrirlo.
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
    let pick = move |cat: String, sub: String| {
        move |_| {
            if let Some(f) = filter {
                f.pick_category(&cat, &sub);
            }
            if let Some(m) = menu.get() {
                _ = m.hide_popover();
            }
        }
    };
    let menu_id = format!("mk-catmenu-{}", products.len());
    let label = {
        let title = title.clone();
        move || match filter.map(|f| (f.category.get(), f.sub.get())) {
            Some((c, s)) if !s.is_empty() => format!("{c} › {s} ▾"),
            Some((c, _)) if !c.is_empty() => format!("{c} ▾"),
            _ => format!("{title} ▾"),
        }
    };
    view! {
        <div class="mk-box mk-pad mk-catdrop flex h-full min-h-14 items-center rounded-box py-2">
            <button type="button" node_ref=button class="mk-filterbtn shrink-0"
                tabindex=editing.then_some("-1")
                aria-haspopup="true"
                aria-controls=menu_id.clone()
                on:click=open_menu>
                {label}
            </button>
            <div node_ref=menu id=menu_id popover="auto" class="mk-tagmenu mk-catmenu" role="group" aria-label=title>
                <button type="button" class="mk-catmenu-item font-semibold" on:click=pick(String::new(), String::new())>
                    {label_or(&props.all_label, "Todas")}
                </button>
                {cats.into_iter().map(|c| {
                    let name = c.name.clone();
                    view! {
                        <div class="mk-catmenu-group">
                            <button type="button" class="mk-catmenu-item font-semibold" on:click=pick(name.clone(), String::new())>
                                <span class="grow">{c.name.clone()}</span>
                                {count_badge(counts, c.products)}
                            </button>
                            {c.subs.into_iter().map(|(s, n)| {
                                let on_pick = pick(name.clone(), s.clone());
                                view! {
                                    <button type="button" class="mk-catmenu-item mk-catmenu-sub" on:click=on_pick>
                                        <span class="grow">{s}</span>
                                        {count_badge(counts, n)}
                                    </button>
                                }
                            }).collect_view()}
                        </div>
                    }
                }).collect_view()}
            </div>
        </div>
    }
    .into_any()
}

/// 5 · Migas («Todas › Botas › Con taco»), el título de lo que se ve y
/// círculos: las categorías, o las subcategorías de la elegida.
#[component]
fn CategoryPage(
    props: CategoryFilterProps,
    products: Vec<Product>,
    editing: bool,
) -> impl IntoView {
    let cats = store_categories(&products);
    if cats.is_empty() {
        return no_cats(editing);
    }
    let filter = use_context::<ProductFilter>();
    let counts = props.show_counts;
    let all = label_or(&props.all_label, "Todas");
    let cur = move || {
        filter
            .map(|f| (f.category.get(), f.sub.get()))
            .unwrap_or_default()
    };
    let all2 = all.clone();
    let crumbs = move || {
        let (c, s) = cur();
        let mut parts: Vec<AnyView> = Vec::new();
        let root = all2.clone();
        let link = move |text: String, cat: String, sub: String| {
            view! {
                <button type="button" class="link link-hover" tabindex=editing.then_some("-1")
                    on:click=move |_| { if let Some(f) = filter { f.pick_category(&cat, &sub); } }>{text}</button>
            }
            .into_any()
        };
        let last = |text: String| {
            view! { <span class="font-semibold" aria-current="page">{text}</span> }.into_any()
        };
        // Al principio (todas las categorías) no hay camino que mostrar: el
        // título ya dice «Todas».
        if c.is_empty() {
            let _ = (last, root);
        } else {
            parts.push(link(root, String::new(), String::new()));
            if s.is_empty() {
                parts.push(last(c));
            } else {
                parts.push(link(c.clone(), c, String::new()));
                parts.push(last(s));
            }
        }
        let n = parts.len();
        parts
            .into_iter()
            .enumerate()
            .map(|(i, p)| view! { {p}{(i + 1 < n).then(|| view! { <span aria-hidden="true" class="opacity-50">"›"</span> })} })
            .collect_view()
    };
    let heading = {
        let all = all.clone();
        let title = props.title.clone();
        move || {
            let (c, s) = cur();
            if !s.is_empty() {
                s
            } else if !c.is_empty() {
                c
            } else if !title.is_empty() {
                title.clone()
            } else {
                all.clone()
            }
        }
    };
    let circle = {
        let products = products.clone();
        move |text: String, cat: String, sub: String, n: usize| {
            let src = photo(&products, &cat, &sub);
            let (pc, ps) = (cat.clone(), sub.clone());
            view! {
                <button type="button" class="mk-circle"
                    tabindex=editing.then_some("-1")
                    aria-pressed=move || filter.map_or(pc.is_empty() && ps.is_empty(), |f| f.is_cat(&pc, &ps)).to_string()
                    on:click=move |_| { if let Some(f) = filter { f.pick_category(&cat, &sub); } }>
                    <span class="mk-circle-ring"><span class="mk-circle-img"><ui::Image src alt=""/></span></span>
                    <span class="mk-circle-label">{text}{count_badge(counts, n)}</span>
                </button>
            }
        }
    };
    let total = products.len();
    let all_c = all.clone();
    let cats2 = cats.clone();
    let items = move || {
        let (c, _) = cur();
        match find(&cats2, &c) {
            // Dentro de una categoría: «Todas» (toda la categoría) y sus subcategorías.
            Some(cc) => {
                let first = circle(all_c.clone(), c.clone(), String::new(), cc.products);
                let rest = cc
                    .subs
                    .clone()
                    .into_iter()
                    .map(|(s, n)| circle(s.clone(), c.clone(), s, n))
                    .collect_view();
                (first.into_any(), rest.into_any())
            }
            None => {
                let first = circle(all_c.clone(), String::new(), String::new(), total);
                let rest = cats2
                    .clone()
                    .into_iter()
                    .map(|cc| {
                        let n = cc.products;
                        circle(cc.name.clone(), cc.name, String::new(), n)
                    })
                    .collect_view();
                (first.into_any(), rest.into_any())
            }
        }
    };
    view! {
        <div class="mk-box mk-pad mk-catpage @container flex h-full flex-col gap-2 overflow-hidden rounded-box">
            <nav aria-label="Dónde estás" class="flex flex-wrap items-center gap-x-2 text-sm opacity-80">{crumbs}</nav>
            <h2 class="mk-title text-xl font-bold truncate" aria-live="polite">{move || view! { <AnimText text=heading()/> }}</h2>
            <div class="flex items-start gap-4 py-1" role="group" aria-label="Ver productos por categoría">
                {move || { let (first, rest) = items(); view! { {first} <div class="mk-scrollrow flex min-w-0 grow gap-4 relative overflow-x-auto py-1">{rest}</div> } }}
            </div>
        </div>
    }
    .into_any()
}

/// Las categorías de los productos en el orden que eligió la tienda.
fn store_categories(products: &[domain::Product]) -> Vec<domain::CategoryCount> {
    let order = use_context::<domain::StoreInfo>()
        .map(|s| s.category_order)
        .unwrap_or_default();
    domain::order_categories(domain::count_categories(products), &order, false)
}
