//! Ruta de desarrollo `/dev/widgets`: cada widget en su tamaño mínimo, por
//! defecto y máximo.
//! Solo responde en compilaciones de desarrollo (`cargo leptos watch`); en
//! `--release` muestra "Página no encontrada".

use domain::{Node, Product, Size, Widget, WIDGET_TYPES};
use leptos::prelude::*;
use widgets::{node_style, render_widget, RenderCtx};

/// Datos de una tienda de ejemplo, para ver el contacto del pie y del
/// bloque Contacto.
fn provide_sample_store() {
    provide_context(domain::StoreInfo {
        name: "La Huerta".to_string(),
        logo_url: String::new(),
        whatsapp: "11 5555-1234".to_string(),
        instagram: "lahuerta".to_string(),
        email: "hola@lahuerta.com.ar".to_string(),
        offers: domain::PriceOffers {
            installments: 3,
            transfer_discount: 10,
            coordinate: true,
        },
    });
}

fn sample_products() -> Vec<Product> {
    let names = [
        "Auriculares inalámbricos",
        "Mochila urbana",
        "Lámpara de escritorio",
        "Taza de cerámica",
        "Zapatillas running",
        "Reloj analógico",
        "Cuaderno de tapa dura",
        "Parlante portátil",
    ];
    names
        .iter()
        .enumerate()
        .map(|(i, name)| Product {
            id: format!("p{i}"),
            name: (*name).to_string(),
            price_cents: 1_234_500 + i as i64 * 250_050,
            image_url: format!("/img/product-{}.webp", i + 1),
            description: format!("{name} de ejemplo, con envío a todo el país."),
            compare_at_cents: if i % 3 == 0 {
                1_234_500 + i as i64 * 250_050 + 300_000
            } else {
                0
            },
            in_stock: i != 2,
            ..Product::default()
        })
        .collect()
}

#[component]
pub fn DevWidgetsPage() -> impl IntoView {
    if !cfg!(debug_assertions) {
        return view! { "Página no encontrada." }.into_any();
    }
    provide_sample_store();
    let products = sample_products();
    let first_id = products[0].id.clone();
    let ctx = RenderCtx {
        products: products.clone(),
        editing: false,
        product: Some(products[0].clone()),
    };

    let cases: Vec<(String, Node)> = WIDGET_TYPES
        .iter()
        .flat_map(|(type_name, label)| {
            let mut widget = Widget::defaults(type_name).expect("tipo conocido");
            if let Widget::FeaturedProduct(p) = &mut widget {
                p.product_id = first_id.clone();
            }
            let l = widget.limits();
            [
                (
                    "mínimo",
                    Size {
                        w: l.min_w,
                        h: l.min_h,
                    },
                ),
                ("por defecto", l.default),
                (
                    "máximo",
                    Size {
                        w: l.max_w,
                        h: l.max_h,
                    },
                ),
            ]
            .into_iter()
            .map(|(name, size)| {
                let mut node = Node::new(format!("{type_name}-{name}"), widget.clone());
                node.size = size;
                (format!("{label} · {name} ({}×{})", size.w, size.h), node)
            })
            .collect::<Vec<_>>()
        })
        .collect();

    view! {
        <div data-theme="light" class="min-h-screen bg-base-200 p-6 text-base-content">
            <h1 class="text-2xl font-bold mb-6">"Widgets: " {cases.len()} " combinaciones"</h1>
            {cases
                .into_iter()
                .map(|(title, node)| {
                    let style = node_style(&node);
                    let content = render_widget(&node, &ctx);
                    view! {
                        <section class="mb-10">
                            <h2 class="text-lg font-semibold mb-2">{title}</h2>
                            <div class="mk-grid rounded-box bg-base-100 p-4">
                                <div class="mk-node outline outline-1 outline-dashed outline-base-300" style=style>
                                    {content}
                                </div>
                            </div>
                        </section>
                    }
                })
                .collect_view()}
        </div>
    }
    .into_any()
}

/// Ruta de desarrollo `/dev/designs`: cada widget en sus cinco diseños, cada
/// uno con otro color. `?theme=dark` (o cualquier tema) lo dibuja en ese tema.
#[component]
pub fn DevDesignsPage() -> impl IntoView {
    if !cfg!(debug_assertions) {
        return view! { "Página no encontrada." }.into_any();
    }
    provide_sample_store();
    let theme = leptos_router::hooks::use_query_map()
        .with_untracked(|q| q.get("theme"))
        .filter(|t| domain::THEMES.contains(&t.as_str()))
        .unwrap_or_else(|| "light".to_string());
    let only = leptos_router::hooks::use_query_map().with_untracked(|q| q.get("only"));
    let products = sample_products();
    let first_id = products[0].id.clone();
    let ctx = RenderCtx {
        products: products.clone(),
        editing: false,
        product: Some(products[0].clone()),
    };
    let colors = ["", "secondary", "accent", "neutral", "#e85d04"];
    let rows: Vec<(String, Vec<(String, Node)>)> = WIDGET_TYPES
        .iter()
        .filter(|(t, _)| {
            only.as_deref()
                .is_none_or(|o| o.split(',').any(|o| o == *t))
        })
        .map(|(type_name, label)| {
            let mut widget = Widget::defaults(type_name).expect("tipo conocido");
            if let Widget::FeaturedProduct(p) = &mut widget {
                p.product_id = first_id.clone();
            }
            let cells = domain::DESIGNS
                .iter()
                .zip(colors)
                .map(|((n, name, _), color)| {
                    let mut node = Node::new(format!("{type_name}-{n}"), widget.clone());
                    node.size.w = node.size.w.min(48);
                    node.look = domain::Look {
                        design: *n,
                        color: color.to_string(),
                    };
                    let shown = if color.is_empty() { "principal" } else { color };
                    (format!("{n} · {name} · {shown}"), node)
                })
                .collect();
            ((*label).to_string(), cells)
        })
        .collect();

    view! {
        <div data-theme=theme class="mk-store min-h-screen bg-base-200 p-6 text-base-content">
            {rows
                .into_iter()
                .map(|(label, cells)| view! {
                    <section class="mb-10">
                        <h2 class="mb-3 text-xl font-bold">{label}</h2>
                        {cells
                            .into_iter()
                            .map(|(title, mut node)| {
                                node.pos = domain::Pos { x: 0, y: 0 };
                                let style = node_style(&node);
                                let content = render_widget(&node, &ctx);
                                view! {
                                    <h3 class="mt-3 mb-1 text-sm font-semibold opacity-75">{title}</h3>
                                    <div class="mk-grid" style="width:1136px">
                                        <div class="mk-node" style=style>{content}</div>
                                    </div>
                                }
                            })
                            .collect_view()}
                    </section>
                })
                .collect_view()}
        </div>
    }
    .into_any()
}

/// Ruta de desarrollo `/dev/ficha`: los estilos de la foto y del precio de
/// la ficha, con un producto de varias fotos y pocas unidades.
#[component]
pub fn DevProductStylesPage() -> impl IntoView {
    if !cfg!(debug_assertions) {
        return view! { "Página no encontrada." }.into_any();
    }
    provide_sample_store();
    let mut product = sample_products().remove(0);
    product.photos = (1..=5).map(|i| format!("/img/product-{i}.webp")).collect();
    product.stock = Some(3);
    let mut grid_products = sample_products();
    grid_products[1].stock = Some(2);
    let ctx = RenderCtx {
        products: grid_products,
        editing: false,
        product: Some(product),
    };
    let mut cases: Vec<(String, Node)> = Vec::new();
    for (v, label) in domain::VARIANT_PRODUCT_IMAGE {
        let mut w = Widget::defaults("ProductImage").expect("tipo conocido");
        if let Widget::ProductImage(p) = &mut w {
            p.variant = (*v).to_string();
            p.badge = "Más vendido".to_string();
        }
        let mut node = Node::new(format!("img-{v}"), w);
        node.size = domain::Size { w: 28, h: 56 };
        cases.push((format!("Foto · {label}"), node));
    }
    for (v, label) in domain::VARIANT_PRODUCT_PRICE {
        let mut w = Widget::defaults("ProductPrice").expect("tipo conocido");
        if let Widget::ProductPrice(p) = &mut w {
            p.variant = (*v).to_string();
        }
        let mut node = Node::new(format!("price-{v}"), w);
        node.size = domain::Size { w: 20, h: if *v == "d" { 22 } else { 10 } };
        cases.push((format!("Precio · {label}"), node));
    }
    for (v, label) in domain::CARD_STYLE_OPTIONS {
        let mut w = Widget::defaults("ProductGrid").expect("tipo conocido");
        if let Widget::ProductGrid(p) = &mut w {
            p.cards.card = (*v).to_string();
            p.limit = 4;
            p.title = "Novedades".to_string();
        }
        let mut node = Node::new(format!("cards-{}", if v.is_empty() { "classic" } else { v }), w);
        node.size = domain::Size { w: 48, h: 50 };
        cases.push((format!("Tarjetas · {label}"), node));
    }
    // Las formas de abrir la ficha, con barra de compra (ficha larga para
    // que el botón quede arriba al bajar).
    let dialog_product = ctx.product.clone().unwrap_or_default();
    let dialogs = [("", "light"), ("drawer", "dark"), ("sheet", "color"), ("auto", "light")]
        .into_iter()
        .map(|(open_mode, bar)| {
            let mut modal = domain::ProductModal::default();
            modal.open = open_mode.to_string();
            modal.buybar = bar.to_string();
            for n in &mut modal.nodes {
                match n.id.as_str() {
                    "m5" => n.pos.y = 18,
                    "m4" => {
                        n.pos.y = 25;
                        n.size.h = 120;
                    }
                    _ => {}
                }
            }
            let open = RwSignal::new(None::<String>);
            let id = dialog_product.id.clone();
            let label = format!("Abrir ficha «{}» con barra «{bar}»", if open_mode.is_empty() { "centro" } else { open_mode });
            view! {
                <button type="button" class="btn btn-sm mr-2 mb-2" data-open=open_mode.to_string() on:click=move |_| open.set(Some(id.clone()))>{label}</button>
                <widgets::ProductDialog modal products=vec![dialog_product.clone()] open/>
            }
        })
        .collect_view();
    view! {
        <div data-theme="light" class="mk-store min-h-screen bg-base-200 p-6 text-base-content">
            <section class="mb-8">{dialogs}</section>
            {cases.into_iter().map(|(title, mut node)| {
                node.pos = domain::Pos { x: 0, y: 0 };
                let style = node_style(&node);
                let content = render_widget(&node, &ctx);
                view! {
                    <section class="mb-8" data-case=node.id.clone()>
                        <h2 class="mb-2 text-sm font-semibold opacity-75">{title}</h2>
                        <div class="mk-grid rounded-box bg-base-100 p-4" style="width:1136px">
                            <div class="mk-node" style=style>{content}</div>
                        </div>
                    </section>
                }
            }).collect_view()}
        </div>
    }
    .into_any()
}

/// Ruta de desarrollo `/dev/menus`: los estilos del menú (con la portada
/// debajo cuando flota), el desplegable en columnas y el anuncio «Cupón».
#[component]
pub fn DevMenusPage() -> impl IntoView {
    if !cfg!(debug_assertions) {
        return view! { "Página no encontrada." }.into_any();
    }
    provide_sample_store();
    let account = widgets::Account::new();
    account.state.set(widgets::AccountState::In(domain::BuyerMe {
        email: "fran@example.com".to_string(),
        name: "Francisco Sevillano".to_string(),
        ..Default::default()
    }));
    provide_context(account);
    widgets::ProductFilter::provide();
    let ctx = RenderCtx {
        products: sample_products(),
        editing: false,
        product: None,
    };
    let leaf = |l: &str| domain::LinkItem {
        label: l.to_string(),
        href: "/".to_string(),
        ..Default::default()
    };
    let col = |t: &str, items: &[&str]| domain::LinkItem {
        children: items.iter().map(|l| leaf(l)).collect(),
        ..leaf(t)
    };
    let mega = domain::LinkItem {
        children: vec![
            col("Destacados", &["Lo nuevo", "Más vendidos", "Ofertas"]),
            col("Zapatillas", &["Running", "Urbanas", "Training", "Básquet"]),
            col("Ropa", &["Remeras", "Buzos", "Shorts"]),
            col("Accesorios", &["Medias", "Mochilas", "Gorras"]),
        ],
        image_url: "/img/product-5.webp".to_string(),
        ..leaf("Hombre")
    };
    let links = vec![leaf("Ofertas"), mega, leaf("Mujer"), leaf("Niños")];
    let nav = |variant: &str, tone: &str, upper: bool, search: bool, menu: &str, menu_color: bool| {
        Widget::Navbar(domain::NavbarProps {
            menu: menu.to_string(),
            brand: "Casa Luna".to_string(),
            logo_url: "/img/logo.svg".to_string(),
            links: links.clone(),
            button_label: String::new(),
            button_href: String::new(),
            variant: variant.to_string(),
            tone: tone.to_string(),
            upper,
            search,
            menu_color,
        })
    };
    // (título, menú, color del bloque, con portada debajo)
    let cases = vec![
        ("Flotante · claro", nav("b", "", true, false, "list", false), "", true),
        ("Flotante · vidrio", nav("b", "glass", true, false, "list", false), "", true),
        ("Flotante · oscuro", nav("b", "dark", true, false, "list", false), "", true),
        ("Flotante · color del bloque", nav("b", "color", true, false, "list", false), "#e85d04", true),
        ("Logo al centro · claro con buscador", nav("c", "", false, true, "columns", false), "", false),
        ("Logo al centro · oscuro en mayúsculas", nav("c", "dark", true, true, "list", false), "", false),
        ("Logo al centro · suave, sin buscador", nav("c", "soft", false, false, "list", false), "secondary", false),
        ("Clásico · desplegable en columnas", nav("a", "", false, true, "columns", false), "", false),
        ("Clásico · columnas con fondo de color", nav("a", "", false, false, "columns", true), "#e3001b", false),
        ("Clásico · columnas con fondo de color (azul noche)", nav("a", "color", false, false, "columns", true), "#1e1b4b", false),
    ];
    let menus = cases
        .into_iter()
        .enumerate()
        .map(|(i, (title, widget, color, cover))| {
            let mut n = Node::new(format!("nav{i}"), widget);
            n.size = Size { w: 48, h: 11 };
            n.look.color = color.to_string();
            let mut nodes = vec![n];
            if cover {
                let mut b = Node::new(format!("ban{i}"), Widget::defaults("Banner").expect("tipo conocido"));
                if let Widget::Banner(p) = &mut b.widget {
                    p.title = "Zapatillas para todos los días".to_string();
                    p.subtitle = "Nueva temporada".to_string();
                    p.image_url = "/img/hero.webp".to_string();
                }
                b.pos.y = 11;
                b.size = Size { w: 48, h: 44 };
                nodes.push(b);
            }
            let grid = widgets::page_nodes(&nodes)
                .into_iter()
                .zip(nodes)
                .map(|((class, style), node)| {
                    let content = render_widget(&node, &ctx);
                    view! { <div class=class style=style>{content}</div> }
                })
                .collect_view();
            view! {
                <section class="mb-8" data-case=format!("nav{i}")>
                    <h2 class="mb-2 text-sm font-semibold opacity-75">{title}</h2>
                    <div class="mk-grid" style="width:min(1136px,100%)">{grid}</div>
                </section>
            }
        })
        .collect_view();
    let coupons = [
        ("", "#e85d04", "15% OFF en toda la tienda", "HOTSALE", "Hasta el domingo"),
        ("", "#111111", "15% OFF en toda la tienda", "HOTSALE", "Hasta el domingo"),
        ("solid", "", "Usá el código y llevate 10% off en tu primera compra", "BIENVENIDA", ""),
        ("soft", "", "10% off en tu primera compra con", "BIENVENIDA", ""),
        ("line", "accent", "20% OFF con el código · solo hoy", "AMIGOS20", ""),
    ]
    .into_iter()
    .enumerate()
    .map(|(i, (look, color, text, code, note))| {
        let mut w = Widget::defaults("Announcement").expect("tipo conocido");
        if let Widget::Announcement(p) = &mut w {
            p.variant = "d".to_string();
            p.coupon = look.to_string();
            p.code = code.to_string();
            p.note = note.to_string();
            p.items.truncate(1);
            p.items[0].text = text.to_string();
            p.items[0].link_label.clear();
        }
        let mut n = Node::new(format!("cup{i}"), w);
        n.size = Size { w: 48, h: 9 };
        n.look.color = color.to_string();
        let style = node_style(&n);
        let content = render_widget(&n, &ctx);
        view! {
            <section class="mb-6" data-case=format!("cup{i}")>
                <h2 class="mb-2 text-sm font-semibold opacity-75">{format!("Cupón · {}", if look.is_empty() { "recortable" } else { look })}</h2>
                <div class="mk-grid" style="width:min(1136px,100%)"><div class="mk-node" style=style>{content}</div></div>
            </section>
        }
    })
    .collect_view();
    view! {
        <div data-theme="light" class="mk-store min-h-screen bg-base-200 p-6 text-base-content max-md:p-2">
            {menus}
            {coupons}
        </div>
    }
    .into_any()
}
