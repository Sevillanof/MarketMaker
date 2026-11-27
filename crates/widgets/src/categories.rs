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

use crate::tags::{count_badge, label_or, ProductFilter};
use crate::RenderCtx;

pub(crate) fn render(widget: &Widget, ctx: &RenderCtx) -> AnyView {
    let products = ctx.products.clone();
    let editing = ctx.editing;
    match widget {
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

impl ProductFilter {
    /// ¿Está elegida justo esta categoría y subcategoría? (Vacías: ninguna.)
    fn is_cat(self, cat: &str, sub: &str) -> bool {
        self.category.with(|c| c == cat) && self.sub.with(|s| s == sub)
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
