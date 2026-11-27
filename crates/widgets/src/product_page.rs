//! Bloques de la ficha del producto con varios estilos: la foto (clásico,
//! galería grande, mosaico, miniaturas al costado y visor) y el precio (solo
//! el precio, con cuotas, transferencia destacada y cajita de formas de pago).
//!
//! Las fotos que se deslizan usan un carril con `scroll-snap` (se desliza con
//! el dedo sin JavaScript); las flechas, los puntitos y las miniaturas solo
//! mueven ese carril. Las cuotas y el descuento por transferencia salen de
//! Cobros, por el contexto [`domain::StoreInfo`].
use domain::{Product, ProductSheetProps};
use leptos::prelude::*;
use ui::{format_price, Image};

/// El color y el talle elegidos en la ficha abierta (vacíos: sin elegir).
/// Lo comparten el selector del botón y la foto.
#[derive(Clone, Copy)]
pub struct VariantPick {
    pub color: RwSignal<String>,
    pub size: RwSignal<String>,
}

impl VariantPick {
    pub fn new() -> Self {
        Self {
            color: RwSignal::new(String::new()),
            size: RwSignal::new(String::new()),
        }
    }
}

impl Default for VariantPick {
    fn default() -> Self {
        Self::new()
    }
}

/// El botón invisible que cubre la tarjeta y abre la ficha (en la tienda).
fn open_cover(id: String, name: &str) -> impl IntoView {
    use_context::<crate::OpenProduct>().map(|crate::OpenProduct(open)| view! {
        <button type="button"
            class="absolute inset-0 cursor-pointer rounded-box focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-primary"
            aria-haspopup="dialog" aria-label=format!("Ver {name}")
            on:click=move |_| open.set(Some(id.clone()))></button>
    })
}

/// Tarjeta de producto de los diseños nuevos de «Cuadrícula de productos»:
/// foto, nombre, categoría, precio (con el anterior tachado) y cuotas. La
/// forma la da el CSS del diseño elegido (`.nw-product`).
#[component]
pub fn NwCard(
    product: Product,
    show_category: bool,
    show_installments: bool,
    favorite: bool,
) -> impl IntoView {
    let offers = use_context::<domain::StoreInfo>()
        .map(|s| s.offers)
        .unwrap_or_default();
    let cents = product.price_cents;
    let per = show_installments
        .then(|| offers.installment_cents(cents))
        .flatten();
    let n = offers.installments;
    let fit = product.fit(&product.image_url);
    view! {
        <article class="nw-product" data-product-id=product.id.clone()>
            <div class="nw-img">
                <Image src=product.image_url.clone() alt=product.name.clone() fit initial=true/>
                {favorite.then(|| view! { <crate::FavoriteHeart product_id=product.id.clone() name=product.name.clone()/> })}
                {(!product.in_stock).then(|| view! { <span class="nw-badge">"Agotado"</span> })}
            </div>
            <div class="nw-product-info">
                <h5>{product.name.clone()}</h5>
                {(show_category && !product.category.is_empty()).then(|| view! { <p>{product.category.clone()}</p> })}
                <strong>
                    {format_price(cents)}
                    {(product.compare_at_cents > cents).then(|| view! {
                        <s><span class="sr-only">"Antes "</span>{format_price(product.compare_at_cents)}</s>
                    })}
                </strong>
                {per.map(|c| view! { <p>{format!("{n} cuotas sin interés de {}", format_price(c))}</p> })}
            </div>
            {open_cover(product.id.clone(), &product.name)}
        </article>
    }
}

/// Debajo del precio de una tarjeta: los puntos de color del producto (hasta
/// 5 y «+N»), «3 colores» si algún color no tiene punto, o los talles si solo
/// tiene talles. Sin variantes deja el lugar si hace falta para alinear.
#[component]
pub fn ColorDots(product: Product) -> impl IntoView {
    const MAX: usize = 5;
    let v = &product.variants;
    if !v.colors.is_empty() {
        let names = v
            .colors
            .iter()
            .map(|c| c.name.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        if v.colors.iter().any(|c| c.hex.is_empty()) {
            let n = v.colors.len();
            return view! {
                <p class="mk-cdots mt-1.5 min-h-4 truncate text-xs opacity-70" title=names>
                    {if n == 1 { "1 color".to_string() } else { format!("{n} colores") }}
                </p>
            }
            .into_any();
        }
        let more = v.colors.len().saturating_sub(MAX);
        let dots = v
            .colors
            .iter()
            .take(MAX)
            .map(|c| {
                let gone = v.qty_of_color(&c.name) == 0;
                view! {
                    <span aria-hidden="true" class="mk-dot size-3.5 shrink-0 rounded-full border border-base-content/25"
                        class:opacity-35=gone style=format!("background-color:{}", c.hex)></span>
                }
            })
            .collect_view();
        return view! {
            <p class="mk-cdots mt-1.5 flex min-h-4 items-center gap-1.5">
                <span class="sr-only">{format!("Colores: {names}")}</span>
                {dots}
                {(more > 0).then(|| view! { <span aria-hidden="true" class="text-xs opacity-70">{format!("+{more}")}</span> })}
            </p>
        }
        .into_any();
    }
    let word = domain::size_word(&v.sizes);
    match (v.sizes.first(), v.sizes.last()) {
        (Some(a), Some(z)) if a != z => view! {
            <p class="mk-cdots mt-1.5 min-h-4 truncate text-xs opacity-70">{format!("{word}s {a} al {z}")}</p>
        }
        .into_any(),
        (Some(a), _) => view! { <p class="mk-cdots mt-1.5 min-h-4 truncate text-xs opacity-70">{format!("{word} {a}")}</p> }.into_any(),
        // Sin variantes: un renglón vacío que solo ocupa lugar si otra
        // tarjeta del mismo bloque muestra colores o talles (ver
        // `.mk-cdots-none` en el CSS). Así todas las fotos de la fila quedan
        // del mismo alto.
        _ => view! { <p class="mk-cdots-none mt-1.5 hidden h-4" aria-hidden="true"></p> }.into_any(),
    }
}

/// Tarjeta «Completa»: carteles sobre la foto, precio con el anterior
/// tachado, y las líneas de transferencia y cuotas de Cobros (cada una se
/// prende o se apaga en el bloque).
#[allow(non_snake_case)]
pub fn FullCard(product: Product, show_price: bool, look: &domain::CardOptions) -> impl IntoView {
    let offers = use_context::<domain::StoreInfo>()
        .map(|s| s.offers)
        .unwrap_or_default();
    let cents = product.price_cents;
    let transfer = (show_price && look.card_transfer)
        .then(|| offers.transfer_cents(cents))
        .flatten();
    let per = (show_price && look.card_installments)
        .then(|| offers.installment_cents(cents))
        .flatten();
    let off = show_price.then(|| product.discount_pct()).flatten();
    let fit = product.fit(&product.image_url);
    let n = offers.installments;
    view! {
        <div class="relative h-full" data-product-id=product.id.clone()>
            <div class="mk-item mk-cfull flex h-full min-h-0 flex-col overflow-hidden rounded-box bg-base-100">
                <div class="mk-cphoto relative min-h-0 flex-1 overflow-hidden rounded-box bg-base-200">
                    <div class="absolute inset-0"><Image src=product.image_url.clone() alt=product.name.clone() fit initial=true/></div>
                    {look.card_favorite.then(|| view! { <crate::FavoriteHeart product_id=product.id.clone() name=product.name.clone()/> })}
                    <div class="absolute left-2 top-2 flex flex-col items-start gap-1">
                        {(!product.in_stock).then(|| view! { <span class="badge badge-neutral badge-sm">"Agotado"</span> })}
                        {off.map(|n| view! { <span class="badge badge-error badge-sm">{format!("-{n}%")}</span> })}
                        {product.few_left().map(|t| view! { <span class="badge badge-warning badge-sm">{t}</span> })}
                    </div>
                </div>
                <div class="flex shrink-0 flex-col gap-0.5 px-1 pt-2 pb-1">
                    <h3 class="mk-sub truncate text-sm font-semibold">{product.name.clone()}</h3>
                    // La categoría solo se ve en los temas de la tienda (ver tailwind.css).
                    {(!product.category.is_empty()).then(|| view! {
                        <p class="mk-cfull-cat truncate text-xs">{product.category.clone()}</p>
                    })}
                    {show_price.then(|| view! {
                        <p class="mk-cfull-price flex flex-wrap items-baseline gap-x-2 text-sm font-bold">
                            {format_price(cents)}
                            {(product.compare_at_cents > cents).then(|| view! {
                                <s class="text-xs font-normal text-error"><span class="sr-only">"Antes "</span>{format_price(product.compare_at_cents)}</s>
                            })}
                        </p>
                    })}
                    {transfer.map(|c| view! { <p class="mk-cfull-tr truncate text-xs font-semibold">{format!("{} con transferencia", format_price(c))}</p> })}
                    {per.map(|c| view! { <p class="truncate text-xs opacity-70">{format!("{n} cuotas sin interés de {}", format_price(c))}</p> })}
                    {look.card_colors.then(|| view! { <ColorDots product=product.clone()/> })}
                    // El mismo botón que la tarjeta Clásica, debajo de los precios.
                    <crate::CardCartButton product=product.clone()/>
                </div>
            </div>
            {open_cover(product.id.clone(), &product.name)}
        </div>
    }
}

/// Tarjeta «Limpia con Añadir»: tarjeta blanca redondeada, nombre en
/// mayúsculas y un «+» que se estira a «Añadir» (suma uno al carrito).
#[allow(non_snake_case)]
pub fn CleanCard(
    product: Product,
    show_price: bool,
    favorite: bool,
    colors: bool,
) -> impl IntoView {
    let fit = product.fit(&product.image_url);
    let cart = use_context::<crate::Cart>();
    let id = StoredValue::new(product.id.clone());
    let added = RwSignal::new(false);
    let tag = if !product.in_stock {
        Some("Agotado".to_string())
    } else if product.few_left().is_some() {
        Some("Últimas".to_string())
    } else {
        product.discount_pct().map(|n| format!("-{n}%"))
    };
    let variants = product.has_variants();
    let open_id = StoredValue::new(product.id.clone());
    let add = product.in_stock.then(|| {
        let label = if variants {
            format!("Elegir color o talle de {}", product.name)
        } else {
            format!("Agregar {} al carrito", product.name)
        };
        view! {
            // Un «+» redondo que, al pasar el mouse o con el teclado, se estira y
            // dice «Añadir» (ver `.mk-cclean-add` en el CSS). En el editor no
            // hay carrito: se ve igual, pero no hace nada.
            <button type="button" class="mk-cclean-add btn btn-sm relative z-10" aria-label=label
                class:is-on=move || added.get()
                on:click=move |_| {
                    if variants {
                        open_id.with_value(|id| open_product(id));
                    } else if let Some(cart) = cart {
                        id.with_value(|id| cart.add(id, 1));
                        added.set(true);
                        set_timeout(move || added.set(false), std::time::Duration::from_millis(1800));
                    }
                }>
                <span class="mk-cclean-icon" aria-hidden="true">{move || if added.get() { "✓" } else { "+" }}</span>
                <span class="mk-cclean-txt" aria-hidden="true">{move || if added.get() { "Listo" } else { "Añadir" }}</span>
            </button>
        }
    });
    view! {
        <div class="relative h-full" data-product-id=product.id.clone()>
            <div class="mk-item mk-cclean flex h-full min-h-0 flex-col overflow-hidden">
                <div class="mk-cphoto relative min-h-0 flex-1 overflow-hidden bg-base-200">
                    <div class="absolute inset-0"><Image src=product.image_url.clone() alt=product.name.clone() fit initial=true/></div>
                    {tag.map(|t| view! { <span class="mk-cclean-tag absolute left-2 top-2">{t}</span> })}
                    {favorite.then(|| view! { <crate::FavoriteHeart product_id=product.id.clone() name=product.name.clone()/> })}
                </div>
                <div class="flex shrink-0 flex-col gap-1 p-3">
                    <h3 class="mk-sub truncate text-sm font-bold uppercase tracking-wide">{product.name.clone()}</h3>
                    <div class="flex items-center justify-between gap-2">
                        {show_price.then(|| view! {
                            <span class="flex min-w-0 flex-wrap items-baseline gap-x-1.5 text-sm">
                                <b>{format_price(product.price_cents)}</b>
                                {(product.compare_at_cents > product.price_cents).then(|| view! {
                                    <s class="text-xs opacity-70"><span class="sr-only">"Antes "</span>{format_price(product.compare_at_cents)}</s>
                                })}
                            </span>
                        })}
                        {add}
                    </div>
                    {colors.then(|| view! { <ColorDots product=product.clone()/> })}
                </div>
            </div>
            {open_cover(product.id.clone(), &product.name)}
        </div>
    }
}

/// Abre la ficha de un producto (para elegir color y talle antes de
/// agregarlo). Sin ficha en la página no hace nada.
pub fn open_product(id: &str) {
    if let Some(crate::OpenProduct(open)) = use_context::<crate::OpenProduct>() {
        open.set(Some(id.to_string()));
    }
}

/// Ficha de un producto con colores y talles (diseño A aprobado): las fotos
/// de cada color y cuadros de talle, lo agotado tachado y sin poder
/// elegirse, y la cantidad con «Agregar al carrito». Cada opción es un
/// `<input type="radio">` de verdad dentro de un `<fieldset>` con su
/// `<legend>`, así se maneja con el teclado y el lector de pantalla.
#[component]
pub fn VariantBuy(
    product: Product,
    #[prop(into)] label: String,
    cart: Option<crate::Cart>,
    /// En el celular, la cantidad arriba y el botón abajo.
    #[prop(optional)]
    stacked: bool,
) -> impl IntoView {
    let pick = use_context::<VariantPick>().unwrap_or_default();
    let v = product.variants.clone();
    let colors = v.colors.clone();
    let sizes = v.sizes.clone();
    let size_word = domain::size_word(&sizes);
    let need_color = !colors.is_empty();
    let need_size = !sizes.is_empty();
    if colors.len() == 1 && pick.color.get_untracked().is_empty() {
        pick.color.set(colors[0].name.clone());
    }
    if sizes.len() == 1 && pick.size.get_untracked().is_empty() {
        pick.size.set(sizes[0].clone());
    }
    let v = StoredValue::new(v);
    let pid = StoredValue::new(product.id.clone());
    let main_photo = product.image_url.clone();
    let qty = RwSignal::new(1u32);
    let missing = RwSignal::new(false);
    let added = RwSignal::new(false);
    // Hay unidades de esa combinación (con el color o el talle todavía sin
    // elegir, alcanza con que alguna tenga).
    let available = move |color: &str, size: &str| {
        v.with_value(|v| {
            v.stock.iter().any(|s| {
                s.qty > 0
                    && (color.is_empty() || s.color == color)
                    && (size.is_empty() || s.size == size)
            })
        })
    };
    let line = move || {
        let (c, s) = (pick.color.get(), pick.size.get());
        if (need_color && c.is_empty()) || (need_size && s.is_empty()) {
            None
        } else {
            Some((
                domain::line_id(&pid.get_value(), &c, &s),
                v.with_value(|v| v.qty(&c, &s)),
            ))
        }
    };
    let what_missing = move || match (
        need_color && pick.color.get().is_empty(),
        need_size && pick.size.get().is_empty(),
    ) {
        (true, true) if size_word == "Talle" => {
            "Elegí un color y un talle para agregarlo al carrito."
        }
        (true, true) => "Elegí un color y un tamaño para agregarlo al carrito.",
        (true, false) => "Elegí un color para agregarlo al carrito.",
        (false, true) if size_word == "Talle" => "Elegí un talle para agregarlo al carrito.",
        (false, true) => "Elegí un tamaño para agregarlo al carrito.",
        _ => "",
    };
    let group = format!("mk-var-{}", product.id);
    let color_set = need_color.then(|| {
        let name_attr = format!("{group}-color");
        view! {
            <fieldset class="mk-pvar-set">
                <legend class="mk-pvar-lbl">"Color: " <span>{move || { let c = pick.color.get(); if c.is_empty() { "elegí uno".to_string() } else { c } }}</span></legend>
                <div class="mk-pswatches">
                    {colors.into_iter().map(|c| {
                        let name = c.name.clone();
                        let n2 = c.name.clone();
                        let n3 = c.name.clone();
                        let out = !available(&c.name, "");
                        let photo = if c.photo.is_empty() { main_photo.clone() } else { c.photo.clone() };
                        let face = if !c.hex.is_empty() && c.photo.is_empty() {
                            view! { <span class="mk-pdot" style=format!("background:{}", c.hex)></span> }.into_any()
                        } else {
                            view! { <Image src=photo alt=""/> }.into_any()
                        };
                        let shown_name = name.clone();
                        view! {
                            // El nombre a la vista debajo de cada color: dos
                            // colores con la misma foto se distinguen.
                            <div class="mk-pswatch-wrap">
                            <label class="mk-pswatch" class:mk-out=out title=name.clone()>
                                <input type="radio" class="sr-only" name=name_attr.clone() value=name.clone() disabled=out
                                    prop:checked=move || pick.color.get() == n2
                                    on:change=move |_| {
                                        pick.color.set(n3.clone());
                                        missing.set(false);
                                        // El talle elegido puede no estar en este color.
                                        let s = pick.size.get_untracked();
                                        if !s.is_empty() && !available(&n3, &s) { pick.size.set(String::new()); }
                                    }/>
                                {face}
                                <span class="sr-only">{if out { format!("{name}, agotado") } else { name.clone() }}</span>
                            </label>
                            <span class="mk-pswatch-name" aria-hidden="true">{shown_name}</span>
                            </div>
                        }
                    }).collect_view()}
                </div>
            </fieldset>
        }
    });
    let size_set = need_size.then(|| {
        let name_attr = format!("{group}-size");
        view! {
            <fieldset class="mk-pvar-set">
                <legend class="mk-pvar-lbl">{size_word}": " <span>{move || { let s = pick.size.get(); if s.is_empty() { "elegí uno".to_string() } else { s } }}</span></legend>
                <div class="mk-psizes">
                    {sizes.into_iter().map(|s| {
                        let s2 = s.clone();
                        let s3 = s.clone();
                        let s4 = StoredValue::new(s.clone());
                        let out = move || s4.with_value(|s4| !available(&pick.color.get(), s4));
                        view! {
                            <label class="mk-psize" class:mk-out=out>
                                <input type="radio" class="sr-only" name=name_attr.clone() value=s.clone() disabled=out
                                    prop:checked=move || pick.size.get() == s2
                                    on:change=move |_| { pick.size.set(s3.clone()); missing.set(false); }/>
                                <span aria-hidden="true">{s.clone()}</span>
                                <span class="sr-only">{let s = s.clone(); move || if out() { format!("{s}, agotado") } else { s.clone() }}</span>
                            </label>
                        }
                    }).collect_view()}
                </div>
            </fieldset>
        }
    });
    let note = move || {
        if missing.get() {
            let t = what_missing();
            if !t.is_empty() {
                return view! { <p class="mk-pvar-err">{t}</p> }.into_any();
            }
        }
        // Las que quedan ya están en el carrito: no se puede sumar más.
        if let (Some(cart), Some((id, left))) = (cart, line()) {
            if left > 0 && cart.qty_of(&id) >= left.min(crate::cart::MAX_QTY) {
                let t = if left == 1 {
                    "La única que queda ya está en tu carrito.".to_string()
                } else {
                    format!("Las {left} que quedan ya están en tu carrito.")
                };
                return view! { <p class="mk-pvar-low">{t}</p> }.into_any();
            }
        }
        match line() {
            Some((_, left)) if (1..=3).contains(&left) => {
                let which = domain::variant_label(&pick.color.get(), &pick.size.get());
                let t = if left == 1 {
                    format!("Queda 1 en {which}")
                } else {
                    format!("Quedan {left} en {which}")
                };
                view! { <p class="mk-pvar-low">{t}</p> }.into_any()
            }
            _ => ().into_any(),
        }
    };
    // Lo que todavía se puede sumar: lo que queda menos lo que ya está en el carrito.
    let room = move || {
        line().map(|(id, q)| {
            let in_cart = cart.map_or(0, |c| c.qty_of(&id));
            q.min(crate::cart::MAX_QTY).saturating_sub(in_cart)
        })
    };
    let max = Signal::derive(move || room().map_or(1, |r| r.max(1)));
    let notice = RwSignal::new(None::<String>);
    view! {
        <div class="mk-pbtn mk-pvar flex h-full flex-col gap-3">
            {color_set}
            {size_set}
            <div aria-live="polite" class="min-h-4 empty:hidden">{note}</div>
            <div class="mk-pvar-buy flex flex-nowrap items-start gap-2" class:mk-buy-b=stacked>
                {move || view! { <crate::cart::QtyStepper value=qty label="Cantidad" max=max.get() disabled=cart.is_none()/> }}
                <button type="button" class="btn btn-primary mk-btn min-w-0 flex-1"
                    disabled=move || room() == Some(0) && !added.get()
                    on:click=move |_| {
                        let Some(cart) = cart else { return };
                        match line() {
                            None => missing.set(true),
                            Some((_, _)) if room() == Some(0) => {}
                            Some((id, _)) => {
                                let q = qty.get();
                                cart.add(&id, q);
                                qty.set(1);
                                added.set(true);
                                let which = domain::variant_label(&pick.color.get_untracked(), &pick.size.get_untracked());
                                notice.set(Some(if which.is_empty() {
                                    format!("Agregaste {q} al carrito")
                                } else {
                                    format!("Agregaste {q} al carrito · {which}")
                                }));
                                set_timeout(move || added.set(false), std::time::Duration::from_millis(2000));
                            }
                        }
                    }>
                    {move || if added.get() { "Agregado ✓".to_string() } else { label.clone() }}
                </button>
            </div>
            {cart.map(|cart| view! { <crate::cart::AddedNotice notice cart/> })}
        </div>
    }
}

/// La ficha completa (diseños: columnas clásicas, panel enmarcado, centrada).
/// La foto con sus miniaturas, el nombre, el precio con las ofertas de
/// Cobros, las opciones, la compra y la descripción (un `<details>`).
#[component]
pub fn ProductSheetWidget(
    props: ProductSheetProps,
    product: Option<Product>,
    editing: bool,
) -> impl IntoView {
    let Some(p) = product else {
        return view! { <div class="mk-nw"><div class="nw-pdp"></div></div> }.into_any();
    };
    let mut offers = use_context::<domain::StoreInfo>()
        .map(|s| s.offers)
        .unwrap_or_default();
    if editing {
        offers = offers.or_sample();
    }
    let cents = p.price_cents;
    let off = p.discount_pct();
    let per = props
        .show_installments
        .then(|| offers.installment_cents(cents))
        .flatten();
    let transfer = props
        .show_transfer
        .then(|| offers.transfer_cents(cents))
        .flatten();
    let (n, pct) = (offers.installments, offers.transfer_discount);
    let few = props.show_stock.then(|| p.few_left()).flatten();

    // Fotos y selección: al elegir un color, la foto va a la de ese color.
    let mut photos = p.all_photos();
    if photos.is_empty() {
        photos.push(String::new());
    }
    let colors = p.variants.colors.clone();
    let sel = RwSignal::new(0usize);
    if let Some(pick) = use_context::<VariantPick>().filter(|_| !colors.is_empty()) {
        let all = photos.clone();
        Effect::new(move |_| {
            let c = pick.color.get();
            let found = colors
                .iter()
                .find(|x| x.name == c)
                .and_then(|x| all.iter().position(|u| *u == x.photo));
            if let Some(i) = found {
                sel.set(i);
            }
        });
    }
    let photos = StoredValue::new(photos);
    let total = photos.with_value(Vec::len);
    let fits = StoredValue::new(p.photo_fits.clone());
    let name = p.name.clone();
    let alt = name.clone();
    let main = move || {
        let i = sel.get().min(total - 1);
        let src = photos.with_value(|v| v[i].clone());
        let fit = fits.with_value(|f| f.get(&src).copied());
        view! { <Image src alt=alt.clone() fit initial=true/> }
    };
    let thumbs = (props.show_thumbs && total > 1).then(|| {
        view! {
            <div class="nw-pdp-thumbs">
                {(0..total).map(|i| {
                    let src = photos.with_value(|v| v[i].clone());
                    let alt = format!("{}, foto {} de {total}", p.name, i + 1);
                    let label = alt.clone();
                    view! {
                        <button type="button" class="nw-pdp-thumb" class:on=move || sel.get() == i
                            aria-label=label aria-pressed=move || (sel.get() == i).to_string()
                            on:click=move |_| sel.set(i)>
                            <span class="nw-img"><Image src alt/></span>
                        </button>
                    }
                }).collect_view()}
            </div>
        }
    });
    let tag = props.badge.trim().to_string();
    let heart = props.show_favorite.then(|| {
        view! { <crate::FavoriteHeart product_id=p.id.clone() name=p.name.clone()/> }
    });
    let share = (props.show_share && !editing)
        .then(|| view! { <crate::ShareButton product=p.clone() on_photo=false inline=true/> });
    let share_static = (props.show_share && editing).then(|| {
        view! { <span class="nw-pdp-share"><span class="nw-pdp-sharebtn">"Compartir"</span></span> }
    });
    let category = (props.show_category && !p.category.is_empty()).then(|| p.category.clone());
    let trust = props.show_trust.then(|| {
        props
            .trust
            .lines()
            .map(str::trim)
            .filter(|l| !l.is_empty())
            .map(|l| view! { <li>{l.to_string()}</li> })
            .collect_view()
    });
    let desc = (props.show_description && !p.description.is_empty()).then(|| {
        view! {
            <details class="nw-pdp-desc" open=props.desc_open>
                <summary>{props.desc_title.clone()}</summary>
                <p class="whitespace-pre-line">{p.description.clone()}</p>
            </details>
        }
    });
    let buy = if p.in_stock {
        view! {
            <div class="nw-pdp-buy" style=crate::nw_on_color(&props.button_color)>
                <crate::cart::AddToCart product=Some(p.clone()) label=crate::product_button_label(&props.button_label) editing/>
            </div>
        }
        .into_any()
    } else {
        view! { <div class="nw-pdp-buy"><button type="button" class="btn btn-primary mk-btn" disabled>"Agotado"</button></div> }
            .into_any()
    };
    let mut style = format!(
        "--pdp-ratio:{};--radius:{}px;",
        props.ratio,
        props.radius.clamp(0, 24)
    );
    style += &crate::nw_colors(&props.bg, &props.ink);
    view! {
        <div class="mk-nw">
            <section class="nw-pdp" style=style>
                <div class="nw-pdp-gallery">
                    <div class="nw-pdp-main">
                        <div class="nw-img">{main}</div>
                        {(!tag.is_empty()).then(|| view! { <span class="nw-pdp-tag">{tag}</span> })}
                        {heart}
                    </div>
                    {thumbs}
                </div>
                <div class="nw-pdp-info">
                    <div class="nw-pdp-top">
                        <span>{category}</span>
                        {share}{share_static}
                    </div>
                    <h3>{name}</h3>
                    <div class="nw-pdp-price">
                        <strong>{format_price(cents)}</strong>
                        {(p.compare_at_cents > cents).then(|| view! {
                            <s><span class="sr-only">"Antes "</span>{format_price(p.compare_at_cents)}</s>
                        })}
                        {off.map(|o| view! { <em>{format!("-{o}%")}</em> })}
                    </div>
                    {transfer.map(|c| view! {
                        <p class="nw-pdp-line"><b>{format_price(c)}</b>{format!(" con transferencia ({pct} % menos)")}</p>
                    })}
                    {per.map(|c| view! {
                        <p class="nw-pdp-line">{format!("{n} cuotas sin interés de ")}<b>{format_price(c)}</b></p>
                    })}
                    {few.map(|t| view! { <p class="nw-pdp-stock">{t}</p> })}
                    {buy}
                    {trust.map(|t| view! { <ul class="nw-pdp-trust">{t}</ul> })}
                    {desc}
                </div>
            </section>
        </div>
    }
    .into_any()
}
