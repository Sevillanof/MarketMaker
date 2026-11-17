//! Carrito de la tienda.
//!
//! Vive en el navegador del comprador (`localStorage`, clave [`STORAGE_KEY`]),
//! así que no hace falta cuenta. Cada tienda tiene su propia dirección
//! (`nombre.dominio`), por eso cada una tiene su propio carrito.
//!
//! [`Cart::provide`] lo deja en el contexto de la página. Sin contexto (en el
//! editor) los botones del carrito se dibujan igual pero no hacen nada.

use domain::Product;
use leptos::prelude::*;
use ui::{format_price, Image};

/// Clave del carrito en `localStorage`.
pub const STORAGE_KEY: &str = "mm_cart";
/// Máximo de unidades de un mismo producto.
pub const MAX_QTY: u32 = 99;

/// El carrito de la tienda: qué productos y cuántos de cada uno.
#[derive(Clone, Copy)]
pub struct Cart {
    lines: RwSignal<Vec<(String, u32)>>,
    products: StoredValue<Vec<Product>>,
    /// El cajón del carrito está abierto.
    pub open: RwSignal<bool>,
    /// Lo último que se quitó, para poder deshacerlo un rato.
    removed: RwSignal<Option<Removed>>,
    /// Ya se leyó del navegador (antes de hidratar el carrito está vacío).
    pub ready: RwSignal<bool>,
    /// Avisa de uso (ver [`crate::UsageHook`]); solo en la tienda real.
    hook: Option<crate::UsageHook>,
}

/// Un producto recién quitado: qué, cuántos y en qué lugar estaba.
#[derive(Clone, Debug, PartialEq)]
struct Removed {
    id: String,
    name: String,
    qty: u32,
    at: usize,
    /// Distingue un quitar de otro, para que el aviso viejo no borre el nuevo.
    token: u32,
}

/// Cuánto dura el aviso "Quitaste … · Deshacer".
const UNDO_MS: u64 = 6000;

/// Un renglón del carrito con su producto.
#[derive(Clone, Debug, PartialEq)]
pub struct CartItem {
    pub product: Product,
    pub qty: u32,
}

impl Cart {
    /// Crea el carrito, lo lee del navegador al hidratar, lo guarda en cada
    /// cambio y lo deja en el contexto.
    pub fn provide(products: Vec<Product>) -> Self {
        let cart = Self {
            lines: RwSignal::new(Vec::new()),
            products: StoredValue::new(products),
            open: RwSignal::new(false),
            removed: RwSignal::new(None),
            ready: RwSignal::new(false),
            hook: use_context::<crate::UsageHook>(),
        };
        let ready = cart.ready;
        let loaded = StoredValue::new(false);
        let lines = cart.lines;
        // Los efectos corren solo en el navegador, después de hidratar: el
        // HTML del servidor siempre sale con el carrito vacío.
        Effect::new(move |_| {
            let current = lines.get();
            if !loaded.get_value() {
                loaded.set_value(true);
                if let Some(saved) = storage().and_then(|s| s.get_item(STORAGE_KEY).ok().flatten())
                {
                    // Si ahora quedan menos unidades, se deja lo que hay.
                    let mut saved = parse(&saved);
                    for (id, q) in &mut saved {
                        let max = cart.limit(id).max(1);
                        *q = (*q).min(max);
                    }
                    lines.set(saved);
                }
                ready.set(true);
                return;
            }
            if let Some(s) = storage() {
                _ = s.set_item(STORAGE_KEY, &serialize(&current));
            }
        });
        provide_context(cart);
        cart
    }

    /// Unidades de un producto que hay en el carrito.
    pub fn qty_of(&self, id: &str) -> u32 {
        self.lines
            .with(|ls| ls.iter().find(|(i, _)| i == id).map_or(0, |(_, q)| *q))
    }

    /// Cuántas unidades de un producto se pueden llevar: las que quedan si
    /// la tienda las cuenta, y nunca más de [`MAX_QTY`].
    pub fn limit(&self, id: &str) -> u32 {
        self.products
            .with_value(|ps| ps.iter().find(|p| p.id == id).and_then(|p| p.stock))
            .map_or(MAX_QTY, |s| s.min(MAX_QTY))
    }

    /// Suma `n` unidades de un producto (hasta [`Cart::limit`]).
    pub fn add(&self, id: &str, n: u32) {
        let max = self.limit(id);
        self.lines.update(|ls| {
            add_line(ls, id, n);
            clamp_line(ls, id, max);
        });
        if n > 0 {
            if let Some(h) = self.hook {
                h.0.run("carrito");
            }
        }
    }

    /// Deja `n` unidades de un producto; con 0 lo quita.
    pub fn set(&self, id: &str, n: u32) {
        let n = n.min(self.limit(id));
        self.lines.update(|ls| set_line(ls, id, n));
    }

    /// Producto y cantidad de cada renglón, para armar el pedido.
    pub fn lines(&self) -> Vec<(String, u32)> {
        self.lines.get()
    }

    /// Vacía el carrito (después de hacer el pedido).
    pub fn clear(&self) {
        self.lines.set(Vec::new());
    }

    /// Quita un producto y deja "Deshacer" unos segundos.
    pub fn remove(&self, id: &str) {
        let Some(at) = self.lines.with(|ls| ls.iter().position(|(i, _)| i == id)) else {
            return;
        };
        let qty = self.qty_of(id);
        let name = self
            .products
            .with_value(|ps| ps.iter().find(|p| p.id == id).map(|p| p.name.clone()))
            .unwrap_or_default();
        let token = self
            .removed
            .with_untracked(|r| r.as_ref().map_or(0, |r| r.token + 1));
        self.lines.update(|ls| set_line(ls, id, 0));
        self.removed.set(Some(Removed {
            id: id.to_string(),
            name,
            qty,
            at,
            token,
        }));
        let removed = self.removed;
        set_timeout(
            move || {
                if removed.with_untracked(|r| r.as_ref().is_some_and(|r| r.token == token)) {
                    removed.set(None);
                }
            },
            std::time::Duration::from_millis(UNDO_MS),
        );
    }

    /// Vuelve a poner lo último que se quitó, en el mismo lugar.
    pub fn undo(&self) {
        let Some(r) = self.removed.get_untracked() else {
            return;
        };
        self.lines.update(|ls| {
            if !ls.iter().any(|(i, _)| *i == r.id) {
                ls.insert(r.at.min(ls.len()), (r.id.clone(), r.qty));
            }
        });
        self.removed.set(None);
    }

    /// Los renglones cuyos productos siguen en la tienda, en el orden en que
    /// se agregaron.
    pub fn items(&self) -> Vec<CartItem> {
        let lines = self.lines.get();
        self.products.with_value(|ps| {
            lines
                .into_iter()
                .filter_map(|(id, qty)| {
                    let product = ps.iter().find(|p| p.id == id)?.clone();
                    Some(CartItem { product, qty })
                })
                .collect()
        })
    }

    /// Unidades que se pueden comprar (sin las agotadas).
    pub fn count(&self) -> u32 {
        self.items()
            .iter()
            .filter(|i| i.product.in_stock)
            .map(|i| i.qty)
            .sum()
    }

    /// Total en centavos (sin las agotadas).
    pub fn total_cents(&self) -> i64 {
        total_cents(&self.items())
    }
}

fn storage() -> Option<web_sys::Storage> {
    window().local_storage().ok().flatten()
}

fn add_line(lines: &mut Vec<(String, u32)>, id: &str, n: u32) {
    match lines.iter_mut().find(|(i, _)| i == id) {
        Some((_, q)) => *q = (*q + n).min(MAX_QTY),
        None if n > 0 => lines.push((id.to_string(), n.min(MAX_QTY))),
        None => {}
    }
}

/// Si quedan menos unidades que las del carrito, deja las que quedan.
fn clamp_line(lines: &mut [(String, u32)], id: &str, max: u32) {
    if let Some((_, q)) = lines.iter_mut().find(|(i, _)| i == id) {
        *q = (*q).min(max.max(1));
    }
}

fn set_line(lines: &mut Vec<(String, u32)>, id: &str, n: u32) {
    if n == 0 {
        lines.retain(|(i, _)| i != id);
    } else if let Some((_, q)) = lines.iter_mut().find(|(i, _)| i == id) {
        *q = n.min(MAX_QTY);
    } else {
        lines.push((id.to_string(), n.min(MAX_QTY)));
    }
}

/// Total en centavos de los productos que hay en stock.
pub fn total_cents(items: &[CartItem]) -> i64 {
    items
        .iter()
        .filter(|i| i.product.in_stock)
        .map(|i| i.product.price_cents * i64::from(i.qty))
        .sum()
}

/// `id:cantidad` separados por comas. Lo que no se entiende se descarta.
pub fn parse(s: &str) -> Vec<(String, u32)> {
    let mut lines = Vec::new();
    for part in s.split(',') {
        let Some((id, qty)) = part.trim().rsplit_once(':') else {
            continue;
        };
        let Ok(qty) = qty.parse::<u32>() else {
            continue;
        };
        if !id.is_empty() {
            add_line(&mut lines, id, qty);
        }
    }
    lines
}

pub fn serialize(lines: &[(String, u32)]) -> String {
    lines
        .iter()
        .map(|(id, q)| format!("{id}:{q}"))
        .collect::<Vec<_>>()
        .join(",")
}

/// Precio corto para lugares chicos: sin `,00` si es redondo.
pub fn short_price(cents: i64) -> String {
    let full = format_price(cents);
    full.strip_suffix(",00").map(str::to_string).unwrap_or(full)
}

/// "1 producto" / "3 productos".
pub fn products_label(n: u32) -> String {
    if n == 1 {
        "1 producto".to_string()
    } else {
        format!("{n} productos")
    }
}

/// Ícono de bolsa de compras.
#[component]
pub fn BagIcon(#[prop(into, optional)] class: String) -> impl IntoView {
    let class = format!("inline-block shrink-0 {class}");
    view! {
        <svg class=class viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
            <path d="M6 7h12l1 13H5L6 7z"/><path d="M9 10V6a3 3 0 0 1 6 0v4"/>
        </svg>
    }
}

fn check_icon() -> impl IntoView {
    view! {
        <svg class="h-4 w-4 shrink-0" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.4" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M5 12l5 5L20 7"/></svg>
    }
}

/// Botón del carrito en el menú de la tienda: vacío es una bolsa; con algo
/// adentro muestra cuántos productos y el total. Abre el cajón del carrito.
#[component]
pub fn CartNavButton() -> impl IntoView {
    match use_context::<Cart>() {
        None => view! {
            <span class="btn btn-ghost btn-square mk-cart-nav shrink-0" aria-hidden="true">
                <BagIcon class="h-6 w-6"/>
            </span>
        }
        .into_any(),
        Some(cart) => view! {
            <button
                type="button"
                class=move || if cart.count() == 0 {
                    "btn btn-ghost btn-square mk-cart-nav shrink-0"
                } else {
                    "btn btn-primary mk-cart-nav shrink-0 rounded-full"
                }
                aria-haspopup="dialog"
                aria-label=move || match cart.count() {
                    0 => "Carrito, vacío".to_string(),
                    n => format!("Carrito, {}, total {}", products_label(n), format_price(cart.total_cents())),
                }
                on:click=move |_| cart.open.set(true)
            >
                <BagIcon class="h-6 w-6"/>
                {move || (cart.count() > 0).then(|| view! {
                    <span class="whitespace-nowrap">{format!("{} · {}", cart.count(), short_price(cart.total_cents()))}</span>
                })}
            </button>
        }
        .into_any(),
    }
}

fn trash_icon() -> impl IntoView {
    view! {
        <svg class="h-4 w-4 shrink-0" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M4 7h16M10 11v6M14 11v6M6 7l1 13h10l1-13M9 7V4h6v3"/></svg>
    }
}

/// Botón de una tarjeta de producto: "Agregar al carrito"; si ya está, un
/// selector − n + en el mismo lugar (así ese lugar siempre cambia la
/// cantidad). Sin stock no se muestra.
#[component]
pub fn CardCartButton(product: Product) -> impl IntoView {
    if !product.in_stock {
        return ().into_any();
    }
    let Some(cart) = use_context::<Cart>() else {
        return view! {
            <span class="btn btn-sm btn-block btn-soft btn-primary mk-cart-add mt-2" aria-hidden="true">"Agregar al carrito"</span>
        }
        .into_any();
    };
    let id = StoredValue::new(product.id);
    let name = product.name;
    // Solo cambia al entrar o salir del carrito: sumar uno no vuelve a
    // dibujar el selector y el foco se queda en el botón.
    let in_cart = Memo::new(move |_| id.with_value(|id| cart.qty_of(id)) > 0);
    let label = format!("Agregar {name} al carrito");
    view! {
        <Show when=move || in_cart.get() fallback=move || view! {
            <button type="button" class="btn btn-sm btn-block btn-soft btn-primary mk-cart-add relative z-10 mt-2"
                aria-label=label.clone()
                on:click=move |_| id.with_value(|id| cart.add(id, 1))>"Agregar al carrito"</button>
        }>
            <CartQty id name=name.clone() cart card=true/>
        </Show>
    }
    .into_any()
}

/// − n + atado al carrito. En 1, el − es un tachito que quita el producto
/// (con "Deshacer").
#[component]
fn CartQty(
    id: StoredValue<String>,
    name: String,
    cart: Cart,
    #[prop(optional)] card: bool,
) -> impl IntoView {
    let qty = move || id.with_value(|id| cart.qty_of(id));
    let (wrap, btn) = if card {
        (
            "mk-qty mk-cart-add relative z-10 mt-2 flex w-full items-center rounded-field bg-primary text-primary-content",
            "btn btn-sm btn-square btn-ghost text-primary-content hover:bg-black/15",
        )
    } else {
        (
            "mk-qty join shrink-0 items-center rounded-field border border-base-300",
            "btn btn-ghost btn-sm btn-square rounded-none",
        )
    };
    let out = if card {
        "min-w-0 flex-1 truncate whitespace-nowrap text-center text-sm font-semibold tabular-nums"
    } else {
        "min-w-8 border-x border-base-300 px-1 text-center font-semibold tabular-nums"
    };
    // Llegó al stock: cuántas quedan (solo si la tienda cuenta el stock).
    let at_stock = move || {
        let max = id.with_value(|id| cart.limit(id));
        (max < MAX_QTY && qty() >= max).then_some(max)
    };
    let minus_label = {
        let name = name.clone();
        move || {
            if qty() <= 1 {
                format!("Quitar {name} del carrito")
            } else {
                "Uno menos".to_string()
            }
        }
    };
    view! {
        <div class=wrap role="group" aria-label=format!("Cantidad de {name} en el carrito")>
            <button type="button" class=btn aria-label=minus_label
                on:click=move |_| id.with_value(|id| {
                    match cart.qty_of(id) {
                        0 | 1 => cart.remove(id),
                        q => cart.set(id, q - 1),
                    }
                })>
                {move || if qty() <= 1 { trash_icon().into_any() } else { view! { "−" }.into_any() }}
            </button>
            <output class=out aria-live="polite">
                {move || if card { format!("{} en carrito", qty()) } else { qty().to_string() }}
            </output>
            <button type="button" class=btn aria-label="Uno más"
                disabled=move || { let max = id.with_value(|id| cart.limit(id)); qty() >= max }
                title=move || at_stock().map(|max| format!("Quedan {max}, no podés agregar más"))
                on:click=move |_| id.with_value(|id| cart.add(id, 1))>"+"</button>
        </div>
        // En el carrito hay lugar para decirlo; en la tarjeta, solo el título.
        {(!card).then(|| view! {
            <p class="mk-stock-note order-last basis-full text-xs opacity-75 empty:hidden" aria-live="polite">
                {move || at_stock().map(|max| if max == 1 {
                    "Queda 1, no podés agregar más.".to_string()
                } else {
                    format!("Quedan {max}, no podés agregar más.")
                })}
            </p>
        })}
    }
}

/// "Quitaste … · Deshacer" durante unos segundos. Hay uno dentro del cajón y
/// otro en la página (el cajón deja inerte lo de atrás); se ve el que toca.
#[component]
fn UndoToast(cart: Cart, in_drawer: bool) -> impl IntoView {
    let show = move || cart.removed.with(|r| r.is_some()) && cart.open.get() == in_drawer;
    let class = if in_drawer {
        "pointer-events-none absolute inset-x-6 bottom-64 z-10 flex justify-center"
    } else {
        "pointer-events-none fixed inset-x-0 bottom-24 z-50 flex justify-center"
    };
    view! {
        <div class=class role="status" aria-live="polite">
            {move || show().then(|| {
                let name = cart.removed.with(|r| r.as_ref().map(|r| r.name.clone()).unwrap_or_default());
                view! {
                    <div class="mk-undo pointer-events-auto flex items-center gap-2 rounded-box bg-neutral py-2 pr-2 pl-4 text-sm text-neutral-content shadow-lg">
                        <span>"Quitaste "<b>{name}</b></span>
                        <button type="button" class="btn btn-sm btn-ghost text-neutral-content underline hover:bg-white/10"
                            on:click=move |_| cart.undo()>"Deshacer"</button>
                    </div>
                }
            })}
        </div>
    }
}

/// − cantidad + . Nunca baja de 1.
#[component]
pub fn QtyStepper(
    value: RwSignal<u32>,
    #[prop(into)] label: String,
    #[prop(optional)] small: bool,
    #[prop(optional)] disabled: bool,
    /// Lo más que se puede elegir.
    #[prop(default = MAX_QTY)]
    max: u32,
) -> impl IntoView {
    let btn = if small {
        "btn btn-ghost btn-sm btn-square rounded-none"
    } else {
        "btn btn-ghost btn-square rounded-none"
    };
    view! {
        <div class="mk-qty join shrink-0 items-center rounded-field border border-base-300" role="group" aria-label=label>
            <button type="button" class=btn aria-label="Uno menos" disabled=move || disabled || (value.get() <= 1)
                on:click=move |_| value.update(|v| *v = v.saturating_sub(1).max(1))>"−"</button>
            <output class="min-w-8 border-x border-base-300 px-1 text-center font-semibold tabular-nums" aria-live="polite">{move || value.get()}</output>
            <button type="button" class=btn aria-label="Uno más" disabled=move || disabled || (value.get() >= max)
                on:click=move |_| value.update(|v| *v = (*v + 1).min(max))>"+"</button>
        </div>
    }
}

/// Cantidad + "Agregar al carrito" de la ficha del producto.
#[component]
pub fn AddToCart(
    product: Option<Product>,
    #[prop(into)] label: String,
    editing: bool,
) -> impl IntoView {
    let qty = RwSignal::new(1u32);
    let cart = use_context::<Cart>().filter(|_| !editing);
    let Some((cart, product)) = cart.zip(product.filter(|p| !p.id.is_empty())) else {
        // En el editor: igual que en la tienda, pero quieto.
        return view! {
            <div class="mk-pbtn flex h-full flex-wrap items-start gap-2">
                <QtyStepper value=qty label="Cantidad" disabled=true/>
                <span class="btn btn-primary mk-btn flex-1">{label}</span>
            </div>
        }
        .into_any();
    };
    let id = StoredValue::new(product.id.clone());
    let added = RwSignal::new(false);
    let in_cart = move || id.with_value(|id| cart.qty_of(id));
    let has = Memo::new(move |_| in_cart() > 0);
    let open_product = use_context::<crate::OpenProduct>();
    // Todo en una línea: el bloque recorta lo que no entra.
    view! {
        <div class="mk-pbtn flex h-full flex-nowrap items-start gap-2">
            <QtyStepper value=qty label=format!("Cantidad de {}", product.name) max=id.with_value(|id| cart.limit(id)).max(1)/>
            <button type="button" class="btn btn-primary mk-btn min-w-0 flex-1"
                on:click=move |_| {
                    id.with_value(|id| cart.add(id, qty.get()));
                    qty.set(1);
                    added.set(true);
                    set_timeout(move || added.set(false), std::time::Duration::from_millis(2000));
                }>
                {move || if added.get() {
                    view! { {check_icon()}<span class="truncate">"Agregado"</span> }.into_any()
                } else {
                    view! { <span class="truncate">{label.clone()}</span> }.into_any()
                }}
            </button>
            <Show when=move || has.get()>
                <button type="button" class="btn btn-outline btn-primary mk-view-cart shrink-0" aria-haspopup="dialog"
                    aria-label=move || format!("Ver carrito, tenés {} de este producto", in_cart())
                    on:click=move |_| {
                        if let Some(crate::OpenProduct(open)) = open_product {
                            open.set(None);
                        }
                        cart.open.set(true);
                    }>
                    <BagIcon class="h-5 w-5"/>"Ver carrito"
                </button>
            </Show>
            <span class="sr-only" aria-live="polite">
                {move || added.get().then(|| format!("Agregado. Tenés {} en el carrito.", in_cart()))}
            </span>
        </div>
    }
    .into_any()
}

/// Barra fija abajo mientras haya algo en el carrito: fotos, cuántos
/// productos, total y "Ver carrito".
#[component]
pub fn CartBar() -> impl IntoView {
    let Some(cart) = use_context::<Cart>() else {
        return ().into_any();
    };
    let has_items = Memo::new(move |_| cart.count() > 0);
    let thumbs = Memo::new(move |_| {
        cart.items()
            .into_iter()
            .filter(|i| i.product.in_stock)
            .take(3)
            .map(|i| i.product.image_url)
            .collect::<Vec<_>>()
    });
    view! {
        <Show when=move || has_items.get()>
            // Lugar para que la barra no tape el final de la página.
            <div class="h-20" aria-hidden="true"></div>
            <div class="mk-cart-bar fixed inset-x-0 bottom-0 z-40 border-t border-base-300 bg-base-100 shadow-[0_-6px_24px_rgba(0,0,0,0.08)]">
                <div class="mx-auto flex h-18 max-w-[1120px] items-center gap-4 px-4">
                    <div class="flex shrink-0" aria-hidden="true">
                        {move || thumbs.get().into_iter().map(|src| view! {
                            <div class="-ml-2 h-10 w-10 overflow-hidden rounded-full border-2 border-base-100 first:ml-0">
                                <Image src alt=""/>
                            </div>
                        }).collect_view()}
                    </div>
                    <p class="min-w-0 truncate text-sm">
                        <b>{move || products_label(cart.count())}</b>" en tu carrito"
                    </p>
                    <span class="ml-auto whitespace-nowrap text-lg font-bold">{move || format_price(cart.total_cents())}</span>
                    <button type="button" class="btn btn-primary" aria-haspopup="dialog" on:click=move |_| cart.open.set(true)>"Ver carrito"</button>
                </div>
            </div>
        </Show>
        <UndoToast cart in_drawer=false/>
    }
    .into_any()
}

/// Cajón del carrito, desde la derecha. Un `<dialog>` modal: fondo inerte,
/// Esc lo cierra, tocar afuera también.
#[component]
pub fn CartDrawer() -> impl IntoView {
    let Some(cart) = use_context::<Cart>() else {
        return ().into_any();
    };
    let dialog_ref = NodeRef::<leptos::html::Dialog>::new();
    Effect::new(move |_| {
        let Some(dialog) = dialog_ref.get() else {
            return;
        };
        if cart.open.get() {
            if !dialog.open() {
                _ = dialog.show_modal();
            }
        } else if dialog.open() {
            dialog.close();
        }
    });
    let close = move || cart.open.set(false);
    let empty = Memo::new(move |_| cart.items().is_empty());
    view! {
        <dialog
            node_ref=dialog_ref
            class="mk-cart bg-base-100 text-base-content"
            aria-labelledby="mk-cart-title"
            on:close=move |_| close()
            on:click=move |ev| {
                // Tocar el fondo (el propio dialog, fuera del panel) cierra.
                use wasm_bindgen::JsCast;
                let on_backdrop = ev
                    .target()
                    .and_then(|t| t.dyn_into::<web_sys::Element>().ok())
                    .is_some_and(|el| el.tag_name() == "DIALOG");
                if on_backdrop {
                    close();
                }
            }
        >
            <div class="flex h-full flex-col">
                <header class="flex items-center gap-2 border-b border-base-300 px-6 py-5">
                    <h2 id="mk-cart-title" class="mk-h text-lg font-bold">"Tu carrito"</h2>
                    <span class="text-sm text-base-content/70">{move || products_label(cart.count())}</span>
                    <button type="button" class="btn btn-sm btn-circle btn-ghost ml-auto" aria-label="Cerrar" on:click=move |_| close()>
                        <svg class="h-4 w-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" aria-hidden="true"><path d="M6 6l12 12M18 6L6 18"/></svg>
                    </button>
                </header>
                <Show when=move || !empty.get() fallback=move || view! {
                    <div class="flex flex-1 flex-col items-center justify-center gap-3 p-8 text-center">
                        <BagIcon class="h-12 w-12 text-base-content/40"/>
                        <p class="font-semibold">"Tu carrito está vacío"</p>
                        <p class="text-sm text-base-content/70">"Tocá \"Agregar al carrito\" en un producto para sumarlo."</p>
                        <button type="button" class="btn btn-primary mt-2" on:click=move |_| close()>"Seguir comprando"</button>
                    </div>
                }>
                    <ul class="flex-1 overflow-y-auto px-6">
                        // Con clave: cambiar una cantidad no vuelve a dibujar la
                        // lista, así el foco se queda en el botón.
                        <For each=move || cart.items() key=|i| i.product.id.clone() let:item>
                            <CartRow item cart/>
                        </For>
                    </ul>
                    <footer class="border-t border-base-300 px-6 pt-5 pb-6">
                        <div class="flex justify-between text-sm text-base-content/75"><span>"Subtotal"</span><span>{move || format_price(cart.total_cents())}</span></div>
                        <div class="mt-1 flex justify-between text-sm text-base-content/75"><span>"Envío"</span><span>"Se calcula después"</span></div>
                        <div class="mt-3 mb-4 flex justify-between text-lg font-bold"><span>"Total"</span><span>{move || format_price(cart.total_cents())}</span></div>
                        <a class="btn btn-primary btn-block btn-lg" href="/comprar">"Finalizar compra"</a>
                        <button type="button" class="btn btn-link btn-block mt-1" on:click=move |_| close()>"Seguir comprando"</button>
                    </footer>
                </Show>
            </div>
            <UndoToast cart in_drawer=true/>
        </dialog>
    }
    .into_any()
}

/// Un producto del cajón: foto, nombre, precio, cantidad, Quitar y el total
/// del renglón.
#[component]
fn CartRow(item: CartItem, cart: Cart) -> impl IntoView {
    let product = item.product;
    let id = StoredValue::new(product.id.clone());
    let qty = move || id.with_value(|id| cart.qty_of(id));
    let price = product.price_cents;
    let unit = move || {
        if qty() > 1 {
            format!("{} c/u", format_price(price))
        } else {
            format_price(price)
        }
    };
    let line_total = move || format_price(price * i64::from(qty()));
    let name = product.name.clone();
    view! {
        <li class="flex items-start gap-4 border-b border-base-200 py-4">
            <div class="h-18 w-18 shrink-0 overflow-hidden rounded-box">
                <Image src=product.image_url.clone() alt="" fit=product.fit(&product.image_url)/>
            </div>
            <div class="min-w-0 flex-1">
                <div class="flex items-baseline gap-3">
                    <p class="min-w-0 flex-1 truncate text-sm font-semibold">{product.name.clone()}</p>
                    {product.in_stock.then(|| view! {
                        <p class="mk-line-total shrink-0 font-bold tabular-nums">{line_total}</p>
                    })}
                </div>
                {if product.in_stock {
                    view! {
                        <p class="mt-0.5 text-sm text-base-content/70">
                            {unit}
                            {product.few_left().map(|t| view! { <span class="ml-2 font-semibold text-warning">{t}</span> })}
                        </p>
                        <div class="mt-2 flex flex-wrap items-center gap-2">
                            <CartQty id name=name.clone() cart/>
                            <button type="button" class="btn btn-ghost btn-sm gap-1 text-base-content/70"
                                aria-label=format!("Quitar {name} del carrito")
                                on:click=move |_| id.with_value(|id| cart.remove(id))>{trash_icon()}"Quitar"</button>
                        </div>
                    }
                    .into_any()
                } else {
                    view! {
                        <p class="mt-1"><span class="badge badge-neutral badge-sm">"Agotado"</span>" "<span class="text-xs text-base-content/70">"No se va a cobrar"</span></p>
                        <button type="button" class="btn btn-ghost btn-sm mt-1 gap-1 text-base-content/70"
                            aria-label=format!("Quitar {name} del carrito")
                            on:click=move |_| id.with_value(|id| cart.remove(id))>{trash_icon()}"Quitar"</button>
                    }
                    .into_any()
                }}
            </div>
        </li>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_and_merges_lines() {
        let lines = parse("a:1, b:2,a:3,,bad,c:x,:4");
        assert_eq!(lines, vec![("a".to_string(), 4), ("b".to_string(), 2)]);
        assert_eq!(serialize(&lines), "a:4,b:2");
    }

    #[test]
    fn caps_and_removes() {
        let mut lines = Vec::new();
        add_line(&mut lines, "a", 150);
        assert_eq!(lines[0].1, MAX_QTY);
        set_line(&mut lines, "a", 0);
        assert!(lines.is_empty());
        add_line(&mut lines, "b", 0);
        assert!(lines.is_empty());
    }

    #[test]
    fn total_skips_sold_out() {
        let p = |id: &str, cents, in_stock| Product {
            id: id.into(),
            price_cents: cents,
            in_stock,
            ..Product::default()
        };
        let items = vec![
            CartItem {
                product: p("a", 650_000, true),
                qty: 2,
            },
            CartItem {
                product: p("b", 100, false),
                qty: 5,
            },
        ];
        assert_eq!(total_cents(&items), 1_300_000);
        assert_eq!(short_price(1_300_000), "$ 13.000");
        assert_eq!(short_price(1_350), "$ 13,50");
    }
}
