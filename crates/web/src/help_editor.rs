//! Películas del editor para la Ayuda (ver `help.rs`).
//!
//! La pantalla es el editor dibujado con sus mismas clases, y los bloques de la
//! página se dibujan con `widgets::render_widget`, igual que el editor de verdad.
//! No lleva el panel de ajustes del bloque elegido (el de la derecha).

use crate::help::{has, Frame, Script, EDITOR_FILM};
use domain::{Node, Product, Size, Widget, GRID_COLS, ROW_PX};
use editor::Icon;
use leptos::prelude::*;
use widgets::{render_widget, RenderCtx};

// Qué hay en pantalla (bits de `Frame::flags`).
const E_DRAG: u32 = 1;
const E_DROP: u32 = 2;
const E_ADDED: u32 = 4;
const E_SEL: u32 = 8;
const E_RESIZED: u32 = 16;
const E_MOVED: u32 = 32;
const E_EDIT: u32 = 64;
const E_DRAWER: u32 = 128;
const E_THEME: u32 = 256;
const E_MENU: u32 = 512;
const E_PAGE2: u32 = 1024;
const E_BUSY: u32 = 2048;
const E_DIRTY: u32 = 4096;
const E_PUBLISHED: u32 = 8192;

/// Guion de la película `k` del editor (0 = agregar un bloque).
pub(crate) fn script(k: usize) -> Vec<Frame> {
    let mut s = Script::with_flags(if k == 5 { E_DIRTY } else { 0 });
    match k {
        0 => {
            s.go(
                "pal-banner",
                "Elegí un bloque de la lista de la izquierda. Pasá el mouse para verlo en grande.",
                2200,
            )
            .click()
            .set(E_DRAG)
            .push(500);
            s.set(E_DROP).go(
                "drop",
                "Arrastralo hasta donde lo quieras en tu página.",
                1800,
            );
            s.click()
                .clear(E_DRAG | E_DROP)
                .set(E_ADDED | E_DIRTY)
                .push(600);
            s.say(
                "Listo: el bloque ya está en tu página. También podés tocarlo en la lista para sumarlo al final.",
                3400,
            );
        }
        1 => {
            s.go("text-node", "Tocá un bloque para elegirlo.", 1800)
                .click()
                .set(E_SEL)
                .push(900);
            s.go(
                "handle-se",
                "Arrastrá el borde o la esquina para cambiar el tamaño.",
                2000,
            )
            .click()
            .set(E_RESIZED | E_DIRTY)
            .push(1800);
            s.go(
                "text-node",
                "Arrastralo desde el medio para moverlo. Se acomoda solo en la grilla.",
                2000,
            )
            .click()
            .set(E_MOVED)
            .push(3000);
        }
        2 => {
            s.go(
                "text-node",
                "Hacé doble clic sobre cualquier texto de tu tienda.",
                1800,
            )
            .click()
            .click()
            .set(E_SEL | E_EDIT)
            .push(500);
            s.typing(0, "Bienvenidos a Taller Norte");
            s.say("Tocá afuera para terminar. Se guarda solo.", 3200);
        }
        3 => {
            s.go(
                "store-menu",
                "Abrí \"Mi tienda\": ahí están los colores, las letras y los márgenes de toda la tienda.",
                2400,
            )
            .click()
            .set(E_DRAWER)
            .push(1000);
            s.go(
                "theme-cupcake",
                "Elegí un tema de colores. Se ve al instante en tu página.",
                2000,
            )
            .click()
            .set(E_THEME | E_DIRTY)
            .push(3200);
        }
        4 => {
            s.go("newpage", "Tocá \"Nueva página\".", 1800)
                .click()
                .set(E_MENU)
                .push(900);
            s.go(
                "tpl-contact",
                "Elegí un modelo: Quiénes somos, Contacto o Preguntas frecuentes.",
                2200,
            )
            .click()
            .clear(E_MENU)
            .set(E_PAGE2 | E_DIRTY)
            .push(600);
            s.say(
                "Ya tenés la página lista, con el mismo menú y pie. Editala como cualquier otra.",
                3400,
            );
        }
        _ => {
            s.set(E_DIRTY).say(
                "Tus cambios se guardan solos, pero tus clientes no los ven hasta que publiques.",
                2600,
            );
            s.go("publish", "Tocá \"Publicar\".", 1600)
                .click()
                .set(E_BUSY)
                .push(1100);
            s.clear(E_BUSY | E_DIRTY).set(E_PUBLISHED).push(300);
            s.say("Listo: tu tienda ya muestra estos cambios.", 3600);
        }
    }
    s.finish()
}

const PAGE_W: f64 = 1120.0;

fn col_px() -> f64 {
    PAGE_W / f64::from(GRID_COLS)
}

fn sample_products() -> Vec<Product> {
    [
        ("Taza de cerámica", 1_850_000, 1),
        ("Plato hondo", 2_400_000, 2),
        ("Set de cuencos", 3_990_000, 3),
        ("Jarra esmaltada", 2_900_000, 4),
    ]
    .into_iter()
    .map(|(name, price, n)| Product {
        id: format!("demo-{n}"),
        name: name.into(),
        price_cents: price,
        image_url: format!("/img/product-{n}.webp"),
        ..Product::default()
    })
    .collect()
}

/// Un bloque de la página: el nodo y cómo se ve (elegido, con asas).
struct Item {
    node: Node,
    selected: bool,
    editing: bool,
}

fn node(id: &str, kind: &str) -> Node {
    Node::new(id, Widget::defaults(kind).expect("tipo de bloque conocido"))
}

/// Los bloques de la página según la película y lo que ya pasó.
fn page(k: usize, f: RwSignal<Frame>) -> Vec<Item> {
    let mut out: Vec<Item> = Vec::new();
    let plain = |n: Node| Item {
        node: n,
        selected: false,
        editing: false,
    };
    out.push(plain(node("nav", "Navbar")));
    if has(f, E_PAGE2) {
        out.push(plain(node("contact", "Contact")));
        return stack(out);
    }
    if k == 0 && has(f, E_ADDED) {
        out.push(plain(node("banner", "Banner")));
    }
    let mut t = node("t", "Text");
    if let Widget::Text(p) = &mut t.widget {
        let typed = f.with(|f| f.t[0].clone());
        if !typed.is_empty() {
            p.heading = typed;
        }
    }
    t.size = if has(f, E_RESIZED) {
        Size { w: 30, h: 22 }
    } else {
        Size { w: 48, h: 16 }
    };
    if has(f, E_MOVED) {
        t.pos.x = 18;
    }
    out.push(Item {
        node: t,
        selected: has(f, E_SEL),
        editing: has(f, E_EDIT),
    });
    out.push(plain(node("grid", "ProductGrid")));
    stack(out)
}

/// Apila los bloques de arriba abajo, uno debajo del otro.
fn stack(mut items: Vec<Item>) -> Vec<Item> {
    let mut y = 0u16;
    for it in &mut items {
        it.node.pos.y = y;
        y += u16::from(it.node.size.h) + 2;
    }
    items
}

#[component]
pub(crate) fn EditorFilm(f: RwSignal<Frame>, film: usize) -> impl IntoView {
    let theme = move || if has(f, E_THEME) { "cupcake" } else { "light" };
    let dirty = move || has(f, E_DIRTY);
    let ctx = RenderCtx {
        products: sample_products(),
        editing: true,
        product: None,
    };
    let ctx = StoredValue::new(ctx);
    let page_name = move || {
        if has(f, E_PAGE2) {
            "Contacto"
        } else {
            "Inicio"
        }
    };
    let mode = "btn btn-sm gap-1.5 border-0 shadow-none";
    view! {
        <div data-theme="light" class="relative flex flex-col bg-base-200 text-base-content"
            style=format!("width:{}px;height:{}px", EDITOR_FILM.0, EDITOR_FILM.1)>
            <header class="flex min-h-16 items-center gap-3 border-b border-base-300 bg-base-100 px-4">
                <span class="btn btn-ghost btn-square h-11 w-11"><Icon name="arrow-left" class="h-5 w-5"/></span>
                <span data-t="store-menu"
                    class=move || format!(
                        "btn h-11 gap-2.5 pl-1.5 pr-2.5 font-normal shadow-none {}",
                        if has(f, E_DRAWER) { "border-primary bg-primary/10" } else { "border-base-300 bg-base-100" })>
                    <span data-theme=theme class="flex h-8 w-8 items-center justify-center gap-0.5 rounded-field border border-base-content/15 bg-base-100">
                        <span class="h-1.5 w-1.5 rounded-full bg-primary"></span>
                        <span class="h-1.5 w-1.5 rounded-full bg-secondary"></span>
                        <span class="h-1.5 w-1.5 rounded-full bg-accent"></span>
                    </span>
                    <span class="flex flex-col items-start leading-tight">
                        <span class="text-[0.9375rem] font-bold">"Mi tienda"</span>
                        <span class="text-xs text-base-content/75">"Ajustes de toda la tienda"</span>
                    </span>
                    {move || if has(f, E_DRAWER) {
                        view! { <Icon name="close" class="h-3.5 w-3.5"/> }.into_any()
                    } else {
                        view! { <Icon name="chevron-down" class="h-3.5 w-3.5"/> }.into_any()
                    }}
                </span>
                <span class="h-7 w-px bg-base-300"></span>
                <div class="flex items-center gap-1">
                    <label class="select select-sm h-9 w-48 gap-2">
                        <Icon name="page" class="h-4 w-4 text-base-content/70"/>
                        <select class="font-semibold" tabindex="-1" disabled>
                            <option>{page_name}</option>
                        </select>
                    </label>
                    <div class="relative">
                        <span data-t="newpage" class="btn btn-ghost btn-sm gap-1">
                            <Icon name="plus" class="h-4 w-4"/>"Nueva página"
                        </span>
                        {move || has(f, E_MENU).then(|| view! {
                            <ul class="menu absolute left-0 top-full z-50 mt-1 w-56 rounded-box border border-base-300 bg-base-100 p-1.5 shadow-lg">
                                {domain::PAGE_TEMPLATES.iter().map(|(path, name)| {
                                    let t = if path.contains("contact") { "tpl-contact" } else { "" };
                                    view! { <li><span data-t=t>{*name}</span></li> }
                                }).collect_view()}
                                <li class="menu-title pt-2 text-xs font-normal whitespace-normal">
                                    "Para otra página, poné un enlace en el menú y tocalo."
                                </li>
                            </ul>
                        })}
                    </div>
                </div>
                <div class="flex flex-1 justify-center">
                    <div class="flex gap-0.5 rounded-field bg-base-200 p-0.5">
                        <span class=format!("{mode} bg-base-100 shadow-sm")><Icon name="pencil"/>"Editar"</span>
                        <span class=format!("{mode} bg-transparent")><Icon name="eye"/>"Vista previa"</span>
                    </div>
                </div>
                <div class="flex gap-0.5">
                    <span class="btn btn-ghost btn-sm btn-square" class:opacity-40=move || !dirty()><Icon name="undo" class="h-5 w-5"/></span>
                    <span class="btn btn-ghost btn-sm btn-square opacity-40"><Icon name="redo" class="h-5 w-5"/></span>
                </div>
                <span class="h-7 w-px bg-base-300"></span>
                <span class="flex items-center gap-1.5 whitespace-nowrap text-sm">
                    {move || if has(f, E_BUSY) {
                        view! {
                            <span class="loading loading-spinner loading-xs"></span>
                            <span>"Publicando…"</span>
                        }.into_any()
                    } else if dirty() {
                        view! {
                            <span class="h-2 w-2 rounded-full bg-warning"></span>
                            <span>"Guardado, sin publicar"</span>
                        }.into_any()
                    } else {
                        view! {
                            <Icon name="check" class="h-4 w-4 text-success"/>
                            <span>"Publicado"</span>
                        }.into_any()
                    }}
                </span>
                <span class="btn btn-outline btn-sm max-w-44 gap-1.5">
                    <Icon name="external" class="h-4 w-4 shrink-0"/><span class="truncate">"Ver tienda"</span>
                </span>
                <span data-t="publish" class="btn btn-sm btn-primary" class:btn-disabled=move || !dirty() || has(f, E_BUSY)>
                    <Icon name="up"/>{move || if has(f, E_BUSY) { "Publicando…" } else { "Publicar" }}
                </span>
            </header>
            <div class="relative flex min-h-0 flex-1">
                <nav class="relative z-30 flex w-44 shrink-0 flex-col border-r border-base-300 bg-base-100">
                    <div class="border-b border-base-300 p-2.5">
                        <span class="btn btn-sm w-full justify-start gap-2 border-base-300 bg-base-100 font-semibold">
                            <Icon name="image" class="h-4 w-4"/>
                            <span class="flex-1 truncate text-left">"Encabezado"</span>
                            <Icon name="chevron-down" class="h-4 w-4 opacity-70"/>
                        </span>
                    </div>
                    <ul class="flex flex-1 flex-col gap-2.5 p-3.5">
                        <li data-t="pal-banner"><Thumb kind="Banner" ctx/></li>
                        <li><Thumb kind="Navbar" ctx/></li>
                        <li><Thumb kind="Carousel" ctx/></li>
                        <li><Thumb kind="Announcement" ctx/></li>
                    </ul>
                </nav>
                <div class="relative min-w-0 flex-1 overflow-hidden p-4">
                    <div data-theme=theme class="mk-store relative mx-auto bg-base-100 text-base-content"
                        style=format!("width:{PAGE_W}px;height:720px")>
                        {move || {
                            let nodes = page(film, f);
                            let cp = col_px();
                            let rp = f64::from(ROW_PX);
                            nodes.into_iter().map(|it| {
                                let n = it.node;
                                let style = format!(
                                    "position:absolute;left:{}px;top:{}px;width:{}px;height:{}px",
                                    f64::from(n.pos.x) * cp, f64::from(n.pos.y) * rp,
                                    f64::from(n.size.w) * cp, f64::from(n.size.h) * rp);
                                let target = if n.id == "t" { "text-node" } else { "" };
                                let sel = it.selected;
                                let badge = sel && has(f, E_RESIZED) && !has(f, E_MOVED);
                                let (w, h) = (n.size.w, n.size.h);
                                let content = ctx.with_value(|c| render_widget(&n, c));
                                view! {
                                    <div class="mk-anode" class:mk-selected=sel style=style
                                        data-t=target>
                                        <div class="mk-anode-box">
                                            <div class="mk-content h-full">{content}</div>
                                            {sel.then(|| view! {
                                                <div class="mk-handle mk-handle-e"></div>
                                                <div class="mk-handle mk-handle-s"></div>
                                                <div class="mk-handle mk-handle-se" data-t="handle-se"></div>
                                            })}
                                            {badge.then(|| view! {
                                                <span class="badge badge-primary badge-sm absolute right-2 top-2 z-10">{format!("{w} × {h}")}</span>
                                            })}
                                            {it.editing.then(|| view! {
                                                <span class="badge badge-sm absolute -top-3 left-2 z-10 border-0 bg-primary text-primary-content">"Escribiendo"</span>
                                            })}
                                        </div>
                                    </div>
                                }
                            }).collect_view()
                        }}
                    </div>
                </div>
                {move || has(f, E_DRAWER).then(|| view! {
                    <aside class="absolute inset-y-0 right-0 z-20 flex w-[368px] flex-col gap-4 border-l border-base-300 bg-base-100 p-5 shadow-xl">
                        <h2 class="text-lg font-bold">"Mi tienda"</h2>
                        <div>
                            <p class="mb-2 text-sm font-semibold">"Colores"</p>
                            <div class="grid grid-cols-2 gap-2">
                                {domain::THEMES.iter().map(|t| {
                                    let t = *t;
                                    let on = move || theme() == t;
                                    let tag = if t == "cupcake" { "theme-cupcake" } else { "" };
                                    view! {
                                        <span data-t=tag data-theme=t
                                            class="flex items-center gap-2 rounded-box border bg-base-100 px-3 py-2 text-sm text-base-content"
                                            class=("border-primary", on) class=("ring-2", on) class=("ring-primary", on)
                                            class=("border-base-300", move || !on())>
                                            <span class="flex gap-0.5">
                                                <span class="h-2.5 w-2.5 rounded-full bg-primary"></span>
                                                <span class="h-2.5 w-2.5 rounded-full bg-secondary"></span>
                                                <span class="h-2.5 w-2.5 rounded-full bg-accent"></span>
                                            </span>
                                            {domain::theme_label(t)}
                                        </span>
                                    }
                                }).collect_view()}
                            </div>
                        </div>
                        <div>
                            <p class="mb-1 text-sm font-semibold">"Letras"</p>
                            <p class="text-sm text-base-content/60">"Tipo y tamaño de los textos de toda la tienda."</p>
                        </div>
                        <div>
                            <p class="mb-1 text-sm font-semibold">"Márgenes"</p>
                            <p class="text-sm text-base-content/60">"El borde de la pantalla, de cada lado."</p>
                        </div>
                    </aside>
                })}
                // El bloque que se arrastra desde la lista hasta la página.
                {move || has(f, E_DRAG).then(|| view! {
                    <div class="pointer-events-none absolute z-40 transition-transform duration-700 ease-out motion-reduce:transition-none"
                        style=move || if has(f, E_DROP) {
                            "left:14px;top:67px;width:147px;transform:translate(673px,71px) scale(1.5)".to_string()
                        } else {
                            "left:14px;top:67px;width:147px".to_string()
                        }>
                        <Thumb kind="Banner" ctx/>
                    </div>
                })}
                <span data-t="drop" class="absolute size-0" style="left:760px;top:170px"></span>
            </div>
            {move || has(f, E_PUBLISHED).then(|| view! {
                <div role="status" class="alert alert-success absolute bottom-6 left-1/2 z-50 -translate-x-1/2 shadow-lg">
                    <span>"Publicado. Tus clientes ya ven estos cambios."</span>
                </div>
            })}
        </div>
    }
}

/// Miniatura de un bloque de la lista de la izquierda (el bloque dibujado de verdad).
#[component]
fn Thumb(kind: &'static str, ctx: StoredValue<RenderCtx>) -> impl IntoView {
    const TW: f64 = 147.0;
    const TH: f64 = 64.0;
    let Some(widget) = Widget::defaults(kind) else {
        return ().into_any();
    };
    let n = Node::new("thumb", widget);
    let w = (f64::from(n.size.w) * PAGE_W / f64::from(GRID_COLS)).clamp(1.0, 600.0);
    let h = f64::from(n.size.h) * f64::from(ROW_PX);
    let scale = TW / w;
    let vis = (h * scale).min(TH);
    let content = ctx.with_value(|c| render_widget(&n, c));
    view! {
        <span data-theme="light" class="mk-store relative block overflow-hidden rounded-sm bg-base-100 shadow-sm ring-1 ring-black/10"
            style=format!("width:{TW}px;height:{vis}px")>
            <span class="pointer-events-none block select-none"
                style=format!("width:{w}px;height:{h}px;transform:scale({scale});transform-origin:0 0;padding:8px;box-sizing:border-box")>
                {content}
            </span>
        </span>
    }
    .into_any()
}
