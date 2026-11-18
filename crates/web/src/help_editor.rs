//! El recorrido del editor para la Ayuda (ver `help.rs`): 10 pasos seguidos,
//! de la vista general del editor a una compra en la tienda publicada.
//!
//! La pantalla es el editor dibujado con sus mismas clases (barra superior,
//! lista de bloques, lienzo, panel de ajustes del bloque y cajón de "Mi
//! tienda"), y los bloques de la página se dibujan con `widgets::render_widget`,
//! igual que el editor de verdad. Si cambia el editor, hay que actualizar acá.

use crate::help::{has, Frame, Script, EDITOR_FILM};
use domain::{Node, Product, Size, Widget, GRID_COLS, ROW_PX};
use editor::Icon;
use leptos::prelude::*;
use widgets::{render_widget, RenderCtx};

/// Cantidad de pasos del recorrido.
pub(crate) const STEPS: usize = 10;

/// Título de cada paso.
pub(crate) const TITLES: [&str; STEPS] = [
    "Vista general del editor",
    "La lista de bloques",
    "Poné un bloque en tu página",
    "Mi tienda: el aspecto de toda la tienda",
    "Elegí un bloque que ya está",
    "Personalizá el bloque",
    "Tus páginas",
    "Vista previa",
    "Publicar",
    "Tu tienda publicada: una compra",
];

// Qué hay en pantalla (bits de `Frame::flags`).
const E_CATMENU: u64 = 1 << 0;
const E_CATBEN: u64 = 1 << 1;
const E_LUPA: u64 = 1 << 2;
const E_DRAG: u64 = 1 << 3;
const E_DROP: u64 = 1 << 4;
const E_ADDED: u64 = 1 << 5;
const E_SEL_BANNER: u64 = 1 << 6;
const E_SEL_NEW: u64 = 1 << 7;
const E_DRAWER: u64 = 1 << 8;
const E_THEME: u64 = 1 << 9;
const E_ROUND: u64 = 1 << 10;
const E_SCROLL1: u64 = 1 << 11;
const E_SCROLL2: u64 = 1 << 12;
const E_RELIEF: u64 = 1 << 13;
const E_TAB_DESIGN: u64 = 1 << 14;
const E_DESIGN3: u64 = 1 << 15;
const E_COLOR: u64 = 1 << 16;
const E_TALL: u64 = 1 << 17;
const E_PAGEMENU: u64 = 1 << 18;
const E_PAGE2: u64 = 1 << 19;
const E_NEWMENU: u64 = 1 << 20;
const E_PREVIEW: u64 = 1 << 21;
const E_BUSY: u64 = 1 << 22;
const E_DIRTY: u64 = 1 << 23;
const E_PUBLISHED: u64 = 1 << 24;
const E_DIALOG: u64 = 1 << 25;
const E_SHOP: u64 = 1 << 26;
const E_FICHA: u64 = 1 << 27;
const E_CART: u64 = 1 << 28;
const E_CHECKOUT: u64 = 1 << 29;
const E_STEP2: u64 = 1 << 30;
const E_STEP3: u64 = 1 << 31;
const E_THANKS: u64 = 1 << 32;
const E_SPOT_HEAD: u64 = 1 << 33;
const E_SPOT_PAL: u64 = 1 << 34;
const E_SPOT_CANVAS: u64 = 1 << 35;
const E_SPOT_INSP: u64 = 1 << 36;
const E_ENVIO: u64 = 1 << 37;
const E_EDITING: u64 = 1 << 38;

/// Guion del paso `k` (0 a 9).
pub(crate) fn script(k: usize) -> Vec<Frame> {
    let base = match k {
        0..=2 => 0,
        3 => E_ADDED | E_DIRTY,
        4..=8 => E_ADDED | E_THEME | E_DIRTY,
        _ => E_ADDED | E_THEME,
    };
    let start = match k {
        2 => E_CATBEN,
        5 => E_SEL_BANNER,
        _ => 0,
    };
    let mut s = Script::with_flags(base | start);
    match k {
        0 => {
            s.say("Este es el editor. Acá armás tu tienda.", 2000);
            s.set(E_SPOT_HEAD).say(
                "Arriba están tu tienda, tus páginas y el botón Publicar.",
                3200,
            );
            s.clear(E_SPOT_HEAD).set(E_SPOT_PAL).say(
                "A la izquierda están los bloques que podés sumar a tu página.",
                3200,
            );
            s.clear(E_SPOT_PAL).set(E_SPOT_CANVAS).say(
                "En el centro armás tu página. Es lo que van a ver tus clientes.",
                3400,
            );
            s.clear(E_SPOT_CANVAS).set(E_SEL_BANNER | E_SPOT_INSP).say(
                "Cuando tocás un bloque, a la derecha aparecen sus ajustes. Vamos por partes.",
                3800,
            );
        }
        1 => {
            s.go(
                "cat-btn",
                "Elegí una categoría: Encabezado, Textos, Productos, Beneficios…",
                2000,
            )
            .click()
            .set(E_CATMENU)
            .push(2200);
            s.go("cat-ben", "", 1400)
                .click()
                .clear(E_CATMENU)
                .set(E_CATBEN)
                .push(1600);
            s.clear(E_CATBEN).go(
                "pal-banner",
                "Pasá el mouse por un bloque y lo ves en grande.",
                1800,
            );
            s.set(E_LUPA).push(3000);
            s.clear(E_LUPA).go(
                "pal-carousel",
                "Cada bloque tiene su estilo. Elegí el que más te guste.",
                2600,
            );
        }
        2 => {
            s.go(
                "pal-new",
                "Para sumar un bloque, arrastralo hasta tu página.",
                2000,
            )
            .click()
            .set(E_DRAG)
            .push(500);
            s.set(E_DROP).go(
                "drop",
                "Mientras arrastrás, el lugar se marca en la página.",
                2400,
            );
            s.click()
                .clear(E_DRAG | E_DROP)
                .set(E_ADDED | E_SEL_NEW | E_DIRTY)
                .push(800);
            s.say(
                "Listo: el bloque quedó en tu página y se guarda solo. A la derecha aparecen sus ajustes.",
                3800,
            );
        }
        3 => {
            s.go(
                "store-menu",
                "\"Mi tienda\" cambia el aspecto de toda la tienda de una vez.",
                2000,
            )
            .click()
            .set(E_DRAWER)
            .push(1200);
            s.go(
                "theme-cupcake",
                "Plantillas: elegí una y toda la tienda cambia al instante.",
                2200,
            )
            .click()
            .set(E_THEME)
            .push(1800);
            s.go(
                "radius-round",
                "Esquinas: rectas, suaves o redondas, para tarjetas, botones y casillas.",
                2400,
            )
            .click()
            .set(E_ROUND)
            .push(1500);
            s.set(E_SCROLL1).push(1200);
            s.go(
                "relief",
                "Efectos: relieve o textura, si querés un toque especial.",
                2200,
            )
            .click()
            .set(E_RELIEF)
            .push(900);
            s.set(E_SCROLL2).push(1200);
            s.say(
                "Más abajo: tamaños, grosor de bordes y esquema oscuro.",
                2600,
            );
        }
        4 => {
            s.go("n-banner", "Tocá un bloque que ya esté en tu página.", 2200)
                .click()
                .set(E_SEL_BANNER)
                .push(1000);
            s.set(E_SPOT_INSP).say(
                "Aparece su panel de ajustes a la derecha, con el nombre del bloque arriba.",
                3600,
            );
        }
        5 => {
            s.go(
                "f-title",
                "En \"Contenido\" cambiás los textos y la foto.",
                1800,
            )
            .click()
            .set(E_EDITING)
            .push(300);
            s.typing(0, "Nueva colección");
            s.clear(E_EDITING);
            s.go(
                "tab-design",
                "En \"Diseño\" cambiás cómo se ve el bloque.",
                2000,
            )
            .click()
            .set(E_TAB_DESIGN)
            .push(900);
            s.go("design-3", "Elegí uno de los 5 diseños…", 1800)
                .click()
                .set(E_DESIGN3)
                .push(1000);
            s.go("color-info", "…y el color.", 1600)
                .click()
                .set(E_COLOR)
                .push(1000);
            s.go("w-half", "Tamaño: elegí el ancho…", 1600)
                .click()
                .push(500);
            s.go("h-more", "…y qué tan alto es.", 1600)
                .click()
                .set(E_TALL)
                .push(2400);
        }
        6 => {
            s.go(
                "page-select",
                "Acá elegís qué página editar. Cada página tiene sus propios bloques.",
                2400,
            )
            .click()
            .set(E_PAGEMENU)
            .push(1200);
            s.go("page-contact", "", 1400)
                .click()
                .clear(E_PAGEMENU | E_SEL_BANNER)
                .set(E_PAGE2)
                .push(900);
            s.say("Listo: ahora editás la página Contacto.", 2200);
            s.go(
                "newpage",
                "Con \"Nueva página\" sumás otra: Quiénes somos, Contacto o Preguntas frecuentes.",
                2600,
            )
            .click()
            .set(E_NEWMENU)
            .push(3000);
        }
        7 => {
            s.go(
                "mode-preview",
                "Tocá \"Vista previa\" para ver tu tienda como la ve tu cliente.",
                2400,
            )
            .click()
            .set(E_PREVIEW)
            .clear(E_SEL_BANNER)
            .push(1200);
            s.say(
                "Sin lista de bloques ni ajustes: así la va a ver tu cliente cuando publiques.",
                3600,
            );
            s.go(
                "n-products",
                "Probá tocar un producto: se abre su ficha, igual que en tu tienda.",
                2800,
            );
            s.go(
                "mode-edit",
                "Para seguir editando, volvé a \"Editar\" o a \"Volver a editar\".",
                2200,
            )
            .click()
            .clear(E_PREVIEW)
            .push(900);
        }
        8 => {
            s.say(
                "Tus cambios se guardan solos, pero tus clientes no los ven hasta que publiques.",
                3400,
            );
            s.go("publish", "Cuando esté listo, tocá \"Publicar\".", 2000)
                .click()
                .set(E_DIALOG)
                .push(1000);
            s.say(
                "Antes de publicar te avisamos si falta algo: productos de ejemplo, textos sin cambiar y fotos de muestra.",
                4400,
            );
            s.go(
                "publish-anyway",
                "Si ya está todo como querés, tocá \"Publicar igual\".",
                2400,
            )
            .click()
            .clear(E_DIALOG)
            .set(E_BUSY)
            .push(1300);
            s.clear(E_BUSY | E_DIRTY).set(E_PUBLISHED).push(400);
            s.say("Listo: tu tienda ya muestra estos cambios.", 3400);
        }
        _ => {
            s.go(
                "see-store",
                "Con \"Ver tienda\" abrís tu tienda publicada.",
                2200,
            )
            .click()
            .set(E_SHOP)
            .push(1400);
            s.say("Así la ve tu cliente. Veamos una compra.", 2200);
            s.go("shop-product", "Tu cliente toca un producto…", 1800)
                .click()
                .set(E_FICHA)
                .push(1400);
            s.go("add-cart", "…y lo agrega al carrito.", 1900)
                .click()
                .clear(E_FICHA)
                .set(E_CART)
                .push(1400);
            s.say("Se abre el carrito con lo que eligió.", 2200);
            s.go("checkout-btn", "Toca \"Finalizar compra\".", 2000)
                .click()
                .clear(E_CART)
                .set(E_CHECKOUT)
                .push(1100);
            s.go("c-name", "Paso 1: sus datos.", 1200).click();
            s.typing(1, "Lucía Fernández");
            s.go("c-email", "", 900).click();
            s.typing(2, "lucia@mail.com");
            s.go("c-next1", "", 900).click().set(E_STEP2).push(900);
            s.go("c-delivery", "Paso 2: cómo recibe el pedido.", 2000)
                .click()
                .set(E_ENVIO)
                .push(700);
            s.go("c-next2", "", 900).click().set(E_STEP3).push(900);
            s.go("c-pay", "Paso 3: cómo paga. Elige Mercado Pago.", 2000)
                .click();
            s.go("c-mp", "Y toca \"Pagar con Mercado Pago\".", 2000)
                .click()
                .set(E_THANKS)
                .push(900);
            s.say(
                "¡Listo! Pago aprobado. Vos recibís el aviso del pedido y tu cliente, su comprobante.",
                4400,
            );
        }
    }
    s.finish()
}

const PAGE_W: f64 = 1120.0;

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

/// Un bloque de la página: el nodo y si está elegido (con asas).
struct Item {
    node: Node,
    selected: bool,
    editing: bool,
}

fn node(id: &str, kind: &str) -> Node {
    Node::new(id, Widget::defaults(kind).expect("tipo de bloque conocido"))
}

/// Los bloques de la página según lo que ya pasó.
fn page(f: RwSignal<Frame>) -> Vec<Item> {
    let plain = |n: Node| Item {
        node: n,
        selected: false,
        editing: false,
    };
    let preview = has(f, E_PREVIEW);
    let mut out: Vec<Item> = vec![plain(node("nav", "Navbar"))];
    if has(f, E_PAGE2) {
        out.push(plain(node("contact", "Contact")));
        return stack(out);
    }
    let mut b = node("banner", "Banner");
    if let Widget::Banner(p) = &mut b.widget {
        let typed = f.with(|f| f.t[0].clone());
        if !typed.is_empty() {
            p.title = typed;
        }
    }
    if has(f, E_DESIGN3) {
        b.look.design = 3;
    }
    if has(f, E_COLOR) {
        b.look.color = "info".into();
    }
    if has(f, E_TALL) {
        b.size = Size {
            w: b.size.w,
            h: b.size.h + 8,
        };
    }
    out.push(Item {
        node: b,
        selected: !preview && has(f, E_SEL_BANNER),
        editing: has(f, E_EDITING),
    });
    if has(f, E_ADDED) {
        out.push(Item {
            node: node("new", "Benefits"),
            selected: !preview && has(f, E_SEL_NEW),
            editing: false,
        });
    }
    out.push(plain(node("grid", "ProductGrid")));
    out.push(plain(node("t", "Text")));
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

/// Marca de lugar con cada paso del cajón "Mi tienda".
fn tile(on: bool) -> &'static str {
    if on {
        "flex items-center gap-1.5 rounded-field border border-primary bg-primary/5 px-2 py-1.5 ring-1 ring-primary"
    } else {
        "flex items-center gap-1.5 rounded-field border border-base-300 px-2 py-1.5"
    }
}

#[component]
pub(crate) fn EditorFilm(f: RwSignal<Frame>) -> impl IntoView {
    let theme = move || if has(f, E_THEME) { "cupcake" } else { "light" };
    let dirty = move || has(f, E_DIRTY);
    let preview = move || has(f, E_PREVIEW);
    let ctx = StoredValue::new(RenderCtx {
        products: sample_products(),
        editing: true,
        product: None,
    });
    let inspector = move || !preview() && (has(f, E_SEL_BANNER) || has(f, E_SEL_NEW));
    let drawer = move || !preview() && has(f, E_DRAWER);
    let page_w = move || {
        if inspector() || drawer() {
            700.0
        } else {
            PAGE_W
        }
    };
    let page_name = move || {
        if has(f, E_PAGE2) {
            "Contacto"
        } else {
            "Inicio"
        }
    };
    let mode = "btn btn-sm gap-1.5 border-0 shadow-none";
    view! {
        <div data-theme="light" class="relative flex flex-col overflow-hidden bg-base-200 text-base-content"
            style=format!("width:{}px;height:{}px", EDITOR_FILM.0, EDITOR_FILM.1)>
            <header class="flex min-h-16 items-center gap-3 border-b border-base-300 bg-base-100 px-4">
                <span class="btn btn-ghost btn-square h-11 w-11"><Icon name="arrow-left" class="h-5 w-5"/></span>
                <span data-t="store-menu"
                    class=move || format!(
                        "btn h-11 gap-2.5 pl-1.5 pr-2.5 font-normal shadow-none {}",
                        if drawer() { "border-primary bg-primary/10" } else { "border-base-300 bg-base-100" })>
                    <span data-theme=theme class="flex h-8 w-8 items-center justify-center gap-0.5 rounded-field border border-base-content/15 bg-base-100">
                        <span class="h-1.5 w-1.5 rounded-full bg-primary"></span>
                        <span class="h-1.5 w-1.5 rounded-full bg-secondary"></span>
                        <span class="h-1.5 w-1.5 rounded-full bg-accent"></span>
                    </span>
                    <span class="flex flex-col items-start leading-tight">
                        <span class="text-[0.9375rem] font-bold">"Mi tienda"</span>
                        <span class="text-xs text-base-content/75">"Ajustes de toda la tienda"</span>
                    </span>
                    {move || if drawer() {
                        view! { <Icon name="close" class="h-3.5 w-3.5"/> }.into_any()
                    } else {
                        view! { <Icon name="chevron-down" class="h-3.5 w-3.5"/> }.into_any()
                    }}
                </span>
                <span class="h-7 w-px bg-base-300"></span>
                <div class="flex items-center gap-1">
                    <div class="relative">
                        <label data-t="page-select" class="select select-sm h-9 w-48 gap-2">
                            <Icon name="page" class="h-4 w-4 text-base-content/70"/>
                            <select class="font-semibold" tabindex="-1" disabled>
                                <option>{page_name}</option>
                            </select>
                        </label>
                        {move || has(f, E_PAGEMENU).then(|| view! {
                            <ul class="menu absolute left-0 top-full z-50 mt-1 w-48 rounded-box border border-base-300 bg-base-100 p-1.5 shadow-lg">
                                <li><span class="menu-active">"Inicio"</span></li>
                                <li><span data-t="page-contact">"Contacto"</span></li>
                            </ul>
                        })}
                    </div>
                    <div class="relative">
                        <span data-t="newpage" class="btn btn-ghost btn-sm gap-1">
                            <Icon name="plus" class="h-4 w-4"/>"Nueva página"
                        </span>
                        {move || has(f, E_NEWMENU).then(|| view! {
                            <ul class="menu absolute left-0 top-full z-50 mt-1 w-56 rounded-box border border-base-300 bg-base-100 p-1.5 shadow-lg">
                                {domain::PAGE_TEMPLATES.iter().map(|(_, name)| view! { <li><span>{*name}</span></li> }).collect_view()}
                                <li class="menu-title pt-2 text-xs font-normal whitespace-normal">
                                    "Para otra página, poné un enlace en el menú y tocalo."
                                </li>
                            </ul>
                        })}
                    </div>
                </div>
                <div class="flex flex-1 justify-center">
                    <div class="flex gap-0.5 rounded-field bg-base-200 p-0.5">
                        <span data-t="mode-edit" class=move || format!("{mode} {}", if preview() { "bg-transparent" } else { "bg-base-100 shadow-sm" })><Icon name="pencil"/>"Editar"</span>
                        <span data-t="mode-preview" class=move || format!("{mode} {}", if preview() { "bg-base-100 shadow-sm" } else { "bg-transparent" })><Icon name="eye"/>"Vista previa"</span>
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
                <span data-t="see-store" class="btn btn-outline btn-sm max-w-44 gap-1.5">
                    <Icon name="external" class="h-4 w-4 shrink-0"/><span class="truncate">"Ver tienda"</span>
                </span>
                <span data-t="publish" class="btn btn-sm btn-primary" class:btn-disabled=move || !dirty() || has(f, E_BUSY)>
                    <Icon name="up"/>{move || if has(f, E_BUSY) { "Publicando…" } else { "Publicar" }}
                </span>
            </header>
            <div class="relative flex min-h-0 flex-1">
                // Lista de bloques (a la izquierda).
                {move || (!preview()).then(|| view! { <Palette f ctx/> })}
                // Cajón "Mi tienda".
                {move || drawer().then(|| view! { <Drawer f theme/> })}
                // Lienzo.
                <section class="relative min-w-0 flex-1 overflow-hidden">
                    {move || preview().then(|| view! {
                        <div class="flex items-center gap-3 border-b border-base-300 bg-base-100 px-4 py-2 text-sm">
                            <Icon name="eye" class="h-4 w-4"/>
                            <span class="flex-1">"Vista previa: así se va a ver tu tienda cuando publiques. Tocá un producto para ver su ficha."</span>
                            <span class="btn btn-sm btn-primary">"Volver a editar"</span>
                        </div>
                    })}
                    <div class="p-4">
                        <div data-theme=theme
                            class="mk-store relative mx-auto bg-base-100 text-base-content"
                            style=move || format!("width:{}px;height:720px{}", page_w(),
                                if has(f, E_ROUND) { ";--radius-box:1.5rem;--radius-field:1rem;--radius-selector:1rem" } else { "" })>
                            {move || {
                                let nodes = page(f);
                                let cp = page_w() / f64::from(GRID_COLS);
                                let rp = f64::from(ROW_PX);
                                nodes.into_iter().map(|it| {
                                    let n = it.node;
                                    let style = format!(
                                        "position:absolute;left:{}px;top:{}px;width:{}px;height:{}px",
                                        f64::from(n.pos.x) * cp, f64::from(n.pos.y) * rp,
                                        f64::from(n.size.w) * cp, f64::from(n.size.h) * rp);
                                    let target = match n.id.as_str() {
                                        "banner" => "n-banner",
                                        "grid" => "n-products",
                                        _ => "",
                                    };
                                    let sel = it.selected;
                                    let content = ctx.with_value(|c| render_widget(&n, c));
                                    view! {
                                        <div class="mk-anode" class:mk-selected=sel style=style data-t=target>
                                            <div class="mk-anode-box">
                                                <div class="mk-content h-full">{content}</div>
                                                {sel.then(|| view! {
                                                    <div class="mk-handle mk-handle-e"></div>
                                                    <div class="mk-handle mk-handle-s"></div>
                                                    <div class="mk-handle mk-handle-se"></div>
                                                })}
                                                {it.editing.then(|| view! {
                                                    <span class="badge badge-sm absolute -top-3 left-2 z-10 border-0 bg-primary text-primary-content">"Escribiendo"</span>
                                                })}
                                            </div>
                                        </div>
                                    }
                                }).collect_view()
                            }}
                            // El lugar donde cae el bloque que se arrastra.
                            {move || has(f, E_DROP).then(|| view! {
                                <div class="pointer-events-none absolute inset-x-0 border-y-2 border-dashed border-primary bg-primary/10"
                                    style="top:340px;height:110px"></div>
                            })}
                            <span data-t="drop" class="absolute size-0" style="left:260px;top:360px"></span>
                        </div>
                    </div>
                </section>
                // Ajustes del bloque elegido (a la derecha).
                {move || inspector().then(|| view! { <Inspector f/> })}
                // El bloque que se arrastra desde la lista hasta la página.
                {move || has(f, E_DRAG).then(|| view! {
                    <div class="pointer-events-none absolute z-40 transition-transform duration-700 ease-out motion-reduce:transition-none"
                        style=move || if has(f, E_DROP) {
                            "left:14px;top:67px;width:147px;transform:translate(310px,270px) scale(1.6)".to_string()
                        } else {
                            "left:14px;top:67px;width:147px".to_string()
                        }>
                        <Thumb kind="Benefits" ctx/>
                    </div>
                })}
            </div>
            // Foco sobre una parte del editor (paso 1).
            <div class="pointer-events-none absolute z-30 rounded-box transition-all duration-700 motion-reduce:transition-none"
                style=move || {
                    let (l, t, w, h) = if has(f, E_SPOT_HEAD) { (0, 0, 1360, 64) }
                        else if has(f, E_SPOT_PAL) { (0, 64, 176, 736) }
                        else if has(f, E_SPOT_CANVAS) { (180, 70, 760, 720) }
                        else if has(f, E_SPOT_INSP) { (992, 64, 368, 736) }
                        else { (680, 400, 0, 0) };
                    let on = has(f, E_SPOT_HEAD) || has(f, E_SPOT_PAL) || has(f, E_SPOT_CANVAS) || has(f, E_SPOT_INSP);
                    format!("left:{l}px;top:{t}px;width:{w}px;height:{h}px;opacity:{};box-shadow:0 0 0 2000px rgba(15,17,30,.55);outline:3px solid #818cf8;outline-offset:-3px", if on { 1 } else { 0 })
                }></div>
            {move || has(f, E_DIALOG).then(|| view! { <PublishDialog/> })}
            {move || has(f, E_PUBLISHED).then(|| view! {
                <div role="status" class="alert alert-success absolute bottom-6 left-1/2 z-50 -translate-x-1/2 shadow-lg">
                    <span>"Publicado. Tus clientes ya ven estos cambios."</span>
                </div>
            })}
            {move || has(f, E_SHOP).then(|| view! { <Shop f/> })}
        </div>
    }
}

/// Lista de bloques (a la izquierda).
#[component]
fn Palette(f: RwSignal<Frame>, ctx: StoredValue<RenderCtx>) -> impl IntoView {
    view! {
        <nav class="relative z-30 flex w-44 shrink-0 flex-col border-r border-base-300 bg-base-100">
            <div class="relative border-b border-base-300 p-2.5">
                <span data-t="cat-btn" class="btn btn-sm w-full justify-start gap-2 border-base-300 bg-base-100 font-semibold">
                    <Icon name="image" class="h-4 w-4"/>
                    <span class="flex-1 truncate text-left">{move || if has(f, E_CATBEN) { "Beneficios" } else { "Encabezado" }}</span>
                    <Icon name="chevron-down" class="h-4 w-4 opacity-70"/>
                </span>
                {move || has(f, E_CATMENU).then(|| view! {
                    <ul class="menu absolute left-2.5 right-2.5 top-full z-50 mt-1 rounded-box border border-base-300 bg-base-100 p-1 shadow-lg">
                        <li><span class="menu-active">"Encabezado"</span></li>
                        <li><span>"Textos"</span></li>
                        <li><span>"Productos"</span></li>
                        <li><span data-t="cat-ben">"Beneficios"</span></li>
                        <li><span>"Contacto"</span></li>
                    </ul>
                })}
            </div>
            <ul class="flex flex-1 flex-col gap-2.5 p-3.5">
                {move || if has(f, E_CATBEN) {
                    view! {
                        <li data-t="pal-new"><Thumb kind="Benefits" ctx/></li>
                        <li><Thumb kind="Testimonials" ctx/></li>
                        <li><Thumb kind="Faq" ctx/></li>
                    }.into_any()
                } else {
                    view! {
                        <li data-t="pal-banner" class=move || if has(f, E_LUPA) { "scale-105 transition-transform" } else { "transition-transform" }><Thumb kind="Banner" ctx/></li>
                        <li><Thumb kind="Navbar" ctx/></li>
                        <li data-t="pal-carousel"><Thumb kind="Carousel" ctx/></li>
                        <li><Thumb kind="Announcement" ctx/></li>
                    }.into_any()
                }}
            </ul>
            // Vista ampliada del bloque al pasar el mouse.
            {move || has(f, E_LUPA).then(|| view! {
                <div data-theme="light" class="pointer-events-none absolute left-[184px] top-20 z-50 w-[420px] rounded-box bg-base-100 p-2.5 shadow-2xl">
                    <div class="mk-store overflow-hidden rounded-field">
                        <Thumb kind="Banner" ctx big=true/>
                    </div>
                </div>
            })}
        </nav>
    }
}

/// Cajón "Mi tienda" (a la izquierda del lienzo).
#[component]
fn Drawer(
    f: RwSignal<Frame>,
    theme: impl Fn() -> &'static str + Copy + Send + Sync + 'static,
) -> impl IntoView {
    let radius = move |round: bool| -> &'static str {
        if round {
            "btn btn-xs btn-primary"
        } else {
            "btn btn-xs"
        }
    };
    view! {
        <aside class="flex w-80 shrink-0 flex-col border-r border-base-300 bg-base-100">
            <div class="flex h-12 shrink-0 items-center gap-2 border-b border-base-300 px-4">
                <h2 class="font-bold">"Mi tienda"</h2>
                <span class="btn btn-ghost btn-sm btn-square ml-auto"><Icon name="close"/></span>
            </div>
            <div class="flex-1 overflow-hidden px-3 pb-4">
                <div class="flex flex-col gap-4 pt-3 transition-transform duration-700 motion-reduce:transition-none"
                    style=move || if has(f, E_SCROLL2) { "transform:translateY(-640px)" } else if has(f, E_SCROLL1) { "transform:translateY(-330px)" } else { "" }>
                    <section class="flex flex-col gap-2">
                        <h3 class="text-sm font-semibold">"Plantilla"</h3>
                        <div class="grid grid-cols-2 gap-1">
                            {domain::THEMES.iter().zip(domain::THEME_LABELS).take(8).map(|(t, l)| {
                                let t = *t;
                                let on = move || theme() == t;
                                let tag = if t == "cupcake" { "theme-cupcake" } else { "" };
                                view! {
                                    <span data-t=tag class=move || tile(on())>
                                        <span data-theme=t class="flex h-5 w-9 shrink-0 items-center justify-center gap-0.5 rounded-[3px] border border-base-content/10 bg-base-100">
                                            <span class="h-2 w-2 rounded-full bg-primary"></span>
                                            <span class="h-2 w-2 rounded-full bg-secondary"></span>
                                            <span class="h-2 w-2 rounded-full bg-accent"></span>
                                        </span>
                                        <span class="min-w-0 truncate text-[11px] font-medium leading-tight">{*l}</span>
                                    </span>
                                }
                            }).collect_view()}
                        </div>
                    </section>
                    <section class="flex flex-col gap-2">
                        <h3 class="text-sm font-semibold">"Esquinas"</h3>
                        {[("Tarjetas", "tarjetas, ventanas, avisos"), ("Botones y campos", "botones, campos, pestañas"), ("Casillas e interruptores", "casillas, interruptores, etiquetas")]
                            .into_iter().enumerate().map(|(i, (title, hint))| view! {
                            <div class="flex items-center justify-between gap-2">
                                <div class="min-w-0">
                                    <span class="block text-[13px] font-semibold">{title}</span>
                                    <span class="block text-[10.5px] text-base-content/55">{hint}</span>
                                </div>
                                <div class="join">
                                    <span class=move || format!("join-item {}", radius(false))>"Rectas"</span>
                                    <span class=move || format!("join-item {}", radius(!has(f, E_ROUND)))>"Suaves"</span>
                                    <span data-t=if i == 0 { "radius-round" } else { "" }
                                        class=move || format!("join-item {}", radius(has(f, E_ROUND)))>"Redondas"</span>
                                </div>
                            </div>
                        }).collect_view()}
                        <h4 class="mt-1 text-[13px] font-semibold">"Márgenes"</h4>
                        <div class="grid grid-cols-3 items-center gap-1 text-center text-xs">
                            <span></span><span class="input input-sm justify-center">"16"</span><span></span>
                            <span class="input input-sm justify-center">"16"</span><span>"▢"</span><span class="input input-sm justify-center">"16"</span>
                            <span></span><span class="input input-sm justify-center">"16"</span><span></span>
                        </div>
                    </section>
                    <section class="flex flex-col gap-1">
                        <h3 class="text-sm font-semibold">"Efectos"</h3>
                        <div class="flex items-center justify-between gap-3 py-1.5">
                            <span><span class="block text-[13px] font-semibold">"Relieve"</span>
                                <span class="block text-[10.5px] text-base-content/55">"Profundidad en campos y selectores"</span></span>
                            <input data-t="relief" type="checkbox" class="toggle toggle-sm" tabindex="-1" prop:checked=move || has(f, E_RELIEF)/>
                        </div>
                        <div class="flex items-center justify-between gap-3 py-1.5">
                            <span><span class="block text-[13px] font-semibold">"Textura"</span>
                                <span class="block text-[10.5px] text-base-content/55">"Un granito sutil de fondo"</span></span>
                            <input type="checkbox" class="toggle toggle-sm" tabindex="-1"/>
                        </div>
                    </section>
                    <section class="flex flex-col gap-2">
                        <h3 class="text-sm font-semibold">"Tamaños"</h3>
                        <input type="range" class="range range-xs" value="40" tabindex="-1"/>
                        <h3 class="mt-2 text-sm font-semibold">"Grosor de bordes"</h3>
                        <input type="range" class="range range-xs" value="20" tabindex="-1"/>
                    </section>
                    <section class="flex flex-col gap-1">
                        <h3 class="text-sm font-semibold">"Opciones"</h3>
                        <div class="flex items-center justify-between gap-3 py-1.5">
                            <span><span class="block text-[13px] font-semibold">"Esquema de color oscuro"</span>
                                <span class="block text-[10.5px] text-base-content/55">"Formularios y barras del navegador en oscuro"</span></span>
                            <input type="checkbox" class="toggle toggle-sm" tabindex="-1"/>
                        </div>
                    </section>
                    <div class="h-40"></div>
                </div>
            </div>
        </aside>
    }
}

/// Ajustes del bloque elegido (a la derecha): pestañas Contenido y Diseño.
#[component]
fn Inspector(f: RwSignal<Frame>) -> impl IntoView {
    let new = move || has(f, E_SEL_NEW);
    let design = move || has(f, E_TAB_DESIGN);
    let title = move || {
        let t = f.with(|f| f.t[0].clone());
        if t.is_empty() {
            "Ofertas de la semana".to_string()
        } else {
            t
        }
    };
    view! {
        <aside class="w-[23rem] shrink-0 overflow-hidden border-l border-base-300 bg-base-100 p-4" aria-label="Propiedades del bloque">
            <div class="flex flex-col gap-4">
                <div class="flex items-start gap-3">
                    <span class="rounded-field bg-primary/10 p-2 text-primary"><Icon name="image" class="h-5 w-5"/></span>
                    <div class="min-w-0 flex-1">
                        <h2 class="text-lg font-bold leading-tight">{move || if new() { "Beneficios" } else { "Banner" }}</h2>
                        <p class="text-sm text-base-content/75">{move || if new() { "Tres ventajas de comprar en tu tienda" } else { "Título, texto y botón" }}</p>
                    </div>
                </div>
                <div role="tablist" class="tabs tabs-box">
                    <span class="tab font-medium" class:tab-active=move || !design()>"Contenido"</span>
                    <span data-t="tab-design" class="tab font-medium" class:tab-active=design>"Diseño"</span>
                </div>
                {move || if !design() {
                    view! {
                        <div class="flex flex-col gap-3">
                            <fieldset class="fieldset"><legend class="fieldset-legend">"Título"</legend>
                                <input data-t="f-title" class="input w-full" class:input-primary=move || has(f, E_EDITING) readonly tabindex="-1" prop:value=title/>
                            </fieldset>
                            <fieldset class="fieldset"><legend class="fieldset-legend">"Subtítulo"</legend>
                                <input class="input w-full" readonly tabindex="-1" value="Hasta 30% de descuento"/>
                            </fieldset>
                            <fieldset class="fieldset"><legend class="fieldset-legend">"Texto del botón"</legend>
                                <input class="input w-full" readonly tabindex="-1" value="Ver ofertas"/>
                            </fieldset>
                            <fieldset class="fieldset"><legend class="fieldset-legend">"Foto de fondo"</legend>
                                <span class="btn btn-sm w-max">"Elegir de Mis fotos"</span>
                            </fieldset>
                        </div>
                    }.into_any()
                } else {
                    view! {
                        <div class="flex flex-col gap-4">
                            <section class="flex flex-col gap-3">
                                <div>
                                    <h3 class="text-sm font-semibold">"Diseño y color"</h3>
                                    <p class="text-xs text-base-content/75">"Cada diseño cambia cómo se ve el bloque. Tus textos e imágenes no cambian."</p>
                                </div>
                                <div class="grid grid-cols-2 gap-2">
                                    {domain::DESIGNS.iter().take(4).map(|(n, name, hint)| {
                                        let n = *n;
                                        let on = move || (n == 3 && has(f, E_DESIGN3)) || (n == 1 && !has(f, E_DESIGN3));
                                        view! {
                                            <div data-t=if n == 3 { "design-3" } else { "" }
                                                class=move || format!("flex flex-col gap-0.5 rounded-box p-1.5 outline-2 outline-offset-0 {}", if on() { "bg-primary/5 outline outline-primary" } else { "outline-transparent" })>
                                                <div class="h-10 rounded-field border border-base-300 bg-base-200"></div>
                                                <span class="text-sm font-medium">{*name}</span>
                                                <span class="text-xs leading-tight text-base-content/75">{*hint}</span>
                                            </div>
                                        }
                                    }).collect_view()}
                                </div>
                                <div class="grid grid-cols-3 gap-1.5">
                                    {[("", "Principal", "bg-primary"), ("secondary", "Secundario", "bg-secondary"), ("info", "Celeste", "bg-info")].into_iter().map(|(id, name, bg)| {
                                        let on = move || (id == "info" && has(f, E_COLOR)) || (id.is_empty() && !has(f, E_COLOR));
                                        view! {
                                            <div data-t=if id == "info" { "color-info" } else { "" }
                                                class=move || format!("flex items-center gap-1.5 rounded-field border px-1.5 py-1 text-xs {}", if on() { "border-base-content font-semibold" } else { "border-base-300" })>
                                                <span class=format!("{bg} h-5 w-5 shrink-0 rounded-full ring-1 ring-base-content/20")></span>
                                                <span class="truncate">{name}</span>
                                            </div>
                                        }
                                    }).collect_view()}
                                </div>
                            </section>
                            <section class="flex flex-col gap-3">
                                <h3 class="text-sm font-semibold">"Tamaño"</h3>
                                <div>
                                    <p class="mb-1 text-sm font-medium">"Ancho"</p>
                                    <div class="join w-full">
                                        <span class="btn btn-sm btn-primary join-item flex-1">"Completo"</span>
                                        <span class="btn btn-sm join-item flex-1">"2/3"</span>
                                        <span data-t="w-half" class="btn btn-sm join-item flex-1">"Mitad"</span>
                                        <span class="btn btn-sm join-item flex-1">"1/3"</span>
                                    </div>
                                </div>
                                <div>
                                    <p class="mb-1 text-sm font-medium">"Alto"</p>
                                    <div class="flex items-center gap-2">
                                        <span class="btn btn-sm flex-1">"Más bajo"</span>
                                        <span class="min-w-16 text-center text-sm tabular-nums">{move || if has(f, E_TALL) { "256 px" } else { "192 px" }}</span>
                                        <span data-t="h-more" class="btn btn-sm flex-1">"Más alto"</span>
                                    </div>
                                </div>
                            </section>
                            <section class="flex flex-col gap-1">
                                <h3 class="text-sm font-semibold">"Letras de este bloque"</h3>
                                <p class="text-xs text-base-content/75">"Igual que la tienda, salvo que elijas otras."</p>
                            </section>
                        </div>
                    }.into_any()
                }}
            </div>
        </aside>
    }
}

/// El aviso de antes de publicar (el real: `PublishCheck` del editor).
#[component]
fn PublishDialog() -> impl IntoView {
    view! {
        <div class="absolute inset-0 z-50 grid place-items-center bg-black/40">
            <div class="modal-box max-w-sm border-t-4 border-warning" role="alertdialog">
                <div class="mb-3 flex items-center gap-3">
                    <span class="grid h-9 w-9 shrink-0 place-items-center rounded-full bg-warning/20 text-warning">
                        <Icon name="alert" class="h-5 w-5"/>
                    </span>
                    <h2 class="text-lg font-extrabold">"Hay 2 textos sin cambiar"</h2>
                </div>
                <ul class="flex flex-col gap-2">
                    {[("Banner", "Escribí acá una frase sobre tu tienda"), ("Texto", "Contá acá quiénes son, qué venden y cómo trabajan.")].into_iter().map(|(w, t)| view! {
                        <li class="flex w-full items-center gap-3 rounded-box border border-base-300 border-l-4 border-l-warning bg-base-100 px-3 py-2.5 text-left">
                            <span class="shrink-0 text-sm font-semibold">{w}</span>
                            <span class="min-w-0 grow truncate text-sm text-base-content/60">{t}</span>
                            <Icon name="arrow-left" class="h-4 w-4 shrink-0 rotate-180 text-base-content/60"/>
                        </li>
                    }).collect_view()}
                </ul>
                <p class="mt-3 text-sm text-base-content/75">"También quedan 3 fotos de muestra. Cambialas por las tuyas en Mis fotos."</p>
                <div class="modal-action">
                    <span data-t="publish-anyway" class="btn btn-ghost">"Publicar igual"</span>
                    <span class="btn btn-primary">"Corregir"</span>
                </div>
            </div>
        </div>
    }
}

/// La tienda publicada: producto, carrito y "Finalizar compra" en 3 pasos.
#[component]
fn Shop(f: RwSignal<Frame>) -> impl IntoView {
    let cart = move || has(f, E_CART);
    let step_card = |n: u8, title: &'static str, f: RwSignal<Frame>| {
        let open = move || {
            if has(f, E_STEP3) {
                n == 3
            } else if has(f, E_STEP2) {
                n == 2
            } else {
                n == 1
            }
        };
        let done = move || {
            if has(f, E_STEP3) {
                n < 3
            } else if has(f, E_STEP2) {
                n < 2
            } else {
                false
            }
        };
        (open, done, title)
    };
    let (o1, d1, t1) = step_card(1, "Tus datos", f);
    let (o2, d2, t2) = step_card(2, "Entrega", f);
    let (o3, d3, t3) = step_card(3, "Pago", f);
    let head = move |n: u8, title: &'static str, open: bool, done: bool| {
        view! {
            <div class="flex items-center gap-3 px-5 py-4">
                <span class=format!("grid h-7 w-7 shrink-0 place-items-center rounded-full text-sm font-bold {}",
                    if done { "bg-success text-success-content" } else if open { "bg-primary text-primary-content" } else { "bg-base-200" })>
                    {if done { "✓".to_string() } else { n.to_string() }}
                </span>
                <h2 class=format!("text-lg font-bold {}", if !open && !done { "opacity-60" } else { "" })>{title}</h2>
            </div>
        }
    };
    let section = |open: bool| {
        format!(
            "rounded-box border bg-base-100 shadow-sm {}",
            if open {
                "border-primary"
            } else {
                "border-base-200"
            }
        )
    };
    view! {
        <div data-theme="light" class="absolute inset-0 z-[60] bg-base-300 px-24 pt-4">
            <div class="flex h-full flex-col overflow-hidden rounded-t-box bg-base-100 shadow-2xl">
                <div class="flex h-11 items-center gap-2 bg-base-200 px-4">
                    <span class="size-3 rounded-full bg-base-300"></span><span class="size-3 rounded-full bg-base-300"></span><span class="size-3 rounded-full bg-base-300"></span>
                    <span class="ml-3 flex-1 rounded-full bg-base-100 px-4 py-1 text-sm text-base-content/70">"🔒 mitienda.mkr.com.ar"</span>
                </div>
                <div data-theme="cupcake" class="mk-store relative min-h-0 flex-1 overflow-hidden bg-base-100 text-base-content">
                    {move || if has(f, E_CHECKOUT) {
                        view! {
                            <div class="grid h-full items-start gap-5 overflow-hidden bg-base-200 p-8 pt-6 grid-cols-[1fr_340px]">
                                <div class="flex min-w-0 flex-col gap-3">
                                    <h1 class="mk-h px-1 text-2xl font-bold">"Finalizar compra"</h1>
                                    <section class=move || section(o1())>
                                        {move || head(1, t1, o1(), d1())}
                                        {move || o1().then(|| view! {
                                            <div class="flex flex-col gap-2 px-5 pb-4">
                                                <label class="text-sm font-semibold">"Nombre y apellido"</label>
                                                <input data-t="c-name" class="input w-full" readonly tabindex="-1" prop:value=move || f.with(|f| f.t[1].clone())/>
                                                <label class="text-sm font-semibold">"Email"</label>
                                                <input data-t="c-email" class="input w-full" readonly tabindex="-1" prop:value=move || f.with(|f| f.t[2].clone())/>
                                                <span data-t="c-next1" class="btn btn-primary mt-2 self-start">"Seguir"</span>
                                            </div>
                                        })}
                                        {move || d1().then(|| view! { <p class="px-5 pb-4 pl-[3.75rem] -mt-3 text-sm text-base-content/70">"Lucía Fernández · lucia@mail.com"</p> })}
                                    </section>
                                    <section class=move || section(o2())>
                                        {move || head(2, t2, o2(), d2())}
                                        {move || o2().then(|| view! {
                                            <div class="flex flex-col gap-2 px-5 pb-4">
                                                <label data-t="c-delivery" class=move || format!("flex items-center gap-3 rounded-box border p-3 {}", if has(f, E_ENVIO) { "border-primary bg-primary/5" } else { "border-base-300" })>
                                                    <span class="font-semibold">"Envío a domicilio"</span><span class="ml-auto font-bold">"$ 6.500"</span>
                                                </label>
                                                <label class="flex items-center gap-3 rounded-box border border-base-300 p-3">
                                                    <span class="font-semibold">"Retiro por el local"</span><span class="ml-auto font-bold">"Gratis"</span>
                                                </label>
                                                <span data-t="c-next2" class="btn btn-primary mt-2 self-start">"Seguir al pago"</span>
                                            </div>
                                        })}
                                        {move || d2().then(|| view! { <p class="px-5 pb-4 pl-[3.75rem] -mt-3 text-sm text-base-content/70">"Envío a domicilio · $ 6.500"</p> })}
                                    </section>
                                    <section class=move || section(o3())>
                                        {move || head(3, t3, o3(), d3())}
                                        {move || o3().then(|| view! {
                                            <div class="flex flex-col gap-2 px-5 pb-4">
                                                <label data-t="c-pay" class="flex items-center gap-3 rounded-box border border-primary bg-primary/5 p-3">
                                                    <span class="font-semibold">"Mercado Pago"</span><span class="ml-auto text-sm text-base-content/70">"Tarjeta, débito o dinero en cuenta"</span>
                                                </label>
                                                <label class="flex items-center gap-3 rounded-box border border-base-300 p-3"><span class="font-semibold">"Transferencia"</span></label>
                                                <span data-t="c-mp" class="btn btn-primary mt-2 self-start">"Pagar con Mercado Pago"</span>
                                            </div>
                                        })}
                                    </section>
                                </div>
                                <aside class="rounded-box border border-base-200 bg-base-100 p-5 shadow-sm">
                                    <h2 class="mk-h text-lg font-bold">"Tu compra"</h2>
                                    <div class="mt-3 flex items-center gap-3 border-b border-base-200 pb-3">
                                        <span class="size-10 rounded-field bg-error"></span><span class="flex-1">"Zapatillas × 1"</span><b>"$ 89.999"</b>
                                    </div>
                                    <div class="mt-3 flex justify-between text-sm"><span>"Envío"</span><b>{move || if has(f, E_ENVIO) { "$ 6.500" } else { "—" }}</b></div>
                                    <div class="flex justify-between"><span>"Total"</span><b>{move || if has(f, E_ENVIO) { "$ 96.499" } else { "$ 89.999" }}</b></div>
                                </aside>
                            </div>
                        }.into_any()
                    } else {
                        view! {
                            <div class="relative h-full">
                                <div class="navbar border-b border-base-200 px-6">
                                    <b class="mk-h flex-1 text-lg">"Mi tienda"</b>
                                    <span class="px-3 text-sm font-semibold">"Inicio"</span>
                                    <span class="px-3 text-sm font-semibold">"Productos"</span>
                                    <span class="px-3 text-sm font-semibold">"Contacto"</span>
                                    <span class="btn btn-ghost btn-sm ml-2 gap-1">"🛒"<span class="badge badge-primary badge-sm">{move || if cart() { "1" } else { "0" }}</span></span>
                                </div>
                                <div class="grid grid-cols-4 gap-4 p-6">
                                    {sample_products().into_iter().enumerate().map(|(i, p)| view! {
                                        <div data-t=if i == 0 { "shop-product" } else { "" }>
                                            <widgets::ProductCard product=p show_price=true/>
                                        </div>
                                    }).collect_view()}
                                </div>
                                {move || (has(f, E_FICHA) || cart()).then(|| view! { <div class="absolute inset-0 bg-black/45"></div> })}
                                {move || has(f, E_FICHA).then(|| view! {
                                    <div class="modal-box absolute left-40 right-40 top-8 grid max-w-none grid-cols-2 gap-6 opacity-100" style="position:absolute">
                                        <div class="overflow-hidden rounded-box bg-base-200" style="min-height:260px"><ui::Image src="/img/product-1.webp" alt="Taza de cerámica"/></div>
                                        <div class="flex flex-col gap-3">
                                            <h3 class="mk-h text-2xl font-bold">"Taza de cerámica"</h3>
                                            <b class="text-xl text-primary">"$ 18.500"</b>
                                            <p class="text-base-content/70">"Hecha a mano, apta lavavajillas. 300 ml."</p>
                                            <span data-t="add-cart" class="btn btn-primary mt-2">"Agregar al carrito"</span>
                                        </div>
                                    </div>
                                })}
                                {move || cart().then(|| view! {
                                    <aside class="absolute inset-y-0 right-0 flex w-[360px] flex-col gap-3 bg-base-100 p-5 shadow-2xl">
                                        <h2 class="mk-h text-lg font-bold">"Tu carrito"</h2>
                                        <div class="flex items-center gap-3 border-b border-base-200 pb-3">
                                            <span class="size-12 overflow-hidden rounded-field bg-base-200"><ui::Image src="/img/product-1.webp" alt=""/></span>
                                            <span class="flex-1"><b>"Taza de cerámica"</b><br/><small class="text-base-content/70">"1 unidad"</small></span>
                                            <b>"$ 18.500"</b>
                                        </div>
                                        <div class="flex justify-between"><span>"Total"</span><b>"$ 18.500"</b></div>
                                        <span data-t="checkout-btn" class="btn btn-primary">"Finalizar compra"</span>
                                    </aside>
                                })}
                            </div>
                        }.into_any()
                    }}
                    {move || has(f, E_THANKS).then(|| view! {
                        <div class="absolute inset-0 z-10 grid place-items-center bg-base-100 text-center">
                            <div class="flex max-w-xl flex-col items-center gap-3">
                                <div class="grid h-14 w-14 place-items-center rounded-full bg-success/15 text-success" aria-hidden="true">"✓"</div>
                                <h1 class="mk-h text-2xl font-bold">"¡Gracias, Lucía! Tu pago está aprobado"</h1>
                                <p class="text-base-content/75">"Pedido #1004 · la tienda ya lo está preparando. Te enviamos el comprobante por email."</p>
                            </div>
                        </div>
                    })}
                </div>
            </div>
        </div>
    }
}

/// Miniatura de un bloque de la lista de la izquierda (el bloque dibujado de verdad).
#[component]
fn Thumb(
    kind: &'static str,
    ctx: StoredValue<RenderCtx>,
    #[prop(optional)] big: bool,
) -> impl IntoView {
    let (tw, th) = if big { (400.0, 190.0) } else { (147.0, 64.0) };
    let Some(widget) = Widget::defaults(kind) else {
        return ().into_any();
    };
    let n = Node::new("thumb", widget);
    let w = (f64::from(n.size.w) * PAGE_W / f64::from(GRID_COLS)).clamp(1.0, 600.0);
    let h = f64::from(n.size.h) * f64::from(ROW_PX);
    let scale = tw / w;
    let vis = (h * scale).min(th);
    let content = ctx.with_value(|c| render_widget(&n, c));
    view! {
        <span data-theme="light" class="mk-store relative block overflow-hidden rounded-sm bg-base-100 shadow-sm ring-1 ring-black/10"
            style=format!("width:{tw}px;height:{vis}px")>
            <span class="pointer-events-none block select-none"
                style=format!("width:{w}px;height:{h}px;transform:scale({scale});transform-origin:0 0;padding:8px;box-sizing:border-box")>
                {content}
            </span>
        </span>
    }
    .into_any()
}
