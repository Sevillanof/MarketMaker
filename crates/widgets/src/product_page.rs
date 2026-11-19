//! Bloques de la ficha del producto con varios estilos: la foto (clásico,
//! galería grande, mosaico, miniaturas al costado y visor) y el precio (solo
//! el precio, con cuotas, transferencia destacada y cajita de formas de pago).
//!
//! Las fotos que se deslizan usan un carril con `scroll-snap` (se desliza con
//! el dedo sin JavaScript); las flechas, los puntitos y las miniaturas solo
//! mueven ese carril. Las cuotas y el descuento por transferencia salen de
//! Cobros, por el contexto [`domain::StoreInfo`].
use domain::{PriceOffers, Product, ProductImageProps, ProductPriceProps};
use leptos::html::Div;
use leptos::prelude::*;
use ui::{format_price, Icon, Image, PriceWas};

/// Los carteles sobre la foto: el que escribió el dueño y "Quedan N" (del
/// stock real, nunca inventado).
fn photo_tags(badge: String, few: Option<String>) -> impl IntoView {
    (!badge.is_empty() || few.is_some()).then(|| {
        view! {
            <div class="mk-ptags pointer-events-none absolute left-3 top-3 z-1 flex flex-col items-start gap-1.5">
                {(!badge.is_empty()).then(|| view! { <span class="badge badge-neutral badge-sm">{badge}</span> })}
                {few.map(|t| view! { <span class="mk-ptag"><i class="mk-ptag-dot" aria-hidden="true"></i>{t}</span> })}
            </div>
        }
    })
}

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

#[component]
pub fn ProductImageWidget(props: ProductImageProps, product: Option<Product>) -> impl IntoView {
    // Al elegir un color en la ficha, la foto va a la de ese color.
    let colors = product.as_ref().map(|p| p.variants.colors.clone()).unwrap_or_default();
    let all = product.as_ref().map(Product::all_photos).unwrap_or_default();
    let target = use_context::<VariantPick>().filter(|_| !colors.is_empty()).map(|pick| {
        Memo::new(move |_| {
            let c = pick.color.get();
            let photo = colors.iter().find(|x| x.name == c).map(|x| x.photo.clone())?;
            all.iter().position(|u| *u == photo)
        })
    });
    let few = product
        .as_ref()
        .filter(|_| props.stock_note)
        .and_then(Product::few_left);
    let (mut photos, alt, fits) =
        product.map_or_else(Default::default, |p| (p.all_photos(), p.name, p.photo_fits));
    if photos.is_empty() {
        photos.push(String::new());
    }
    let contain = props.fit == "contain";
    let badge = props.badge.trim().to_string();
    let total = photos.len();
    let photos = StoredValue::new(photos);
    let fits = StoredValue::new(fits);
    let alt = StoredValue::new(alt);
    let alt_for = move |i: usize| {
        alt.with_value(|a| {
            if total > 1 {
                format!("{a}, foto {} de {total}", i + 1)
            } else {
                a.clone()
            }
        })
    };
    let photo = move |i: usize| {
        let src = photos.with_value(|p| p[i].clone());
        let fit = fits.with_value(|f| f.get(&src).copied());
        view! { <Image src alt=alt_for(i) contain fit/> }.into_any()
    };

    match props.variant.as_str() {
        // Mosaico: dos fotos altas y una ancha, y así sigue (se desplaza
        // dentro del bloque si hay más).
        "c" => view! {
            <div class="mk-image mk-pmosaic relative h-full overflow-y-auto" data-n=total.min(3).to_string()>
                {(0..total).map(|i| view! {
                    <div class="mk-box relative overflow-hidden rounded-box bg-base-200">
                        {photo(i)}
                    </div>
                }).collect_view()}
                {photo_tags(badge, few)}
            </div>
        }
        .into_any(),
        "b" | "d" | "e" if total > 1 => {
            slider(props.variant.clone(), total, photo, badge, few, photos, fits, target).into_any()
        }
        "e" => view! {
            <div class="mk-box mk-image mk-pvisor flex h-full flex-col">
                <div class="mk-pvisor-img relative min-h-0 flex-1 overflow-hidden">{photo(0)}{photo_tags(badge, few)}</div>
            </div>
        }
        .into_any(),
        "b" | "d" => view! {
            <div class="mk-box mk-image relative h-full overflow-hidden rounded-box bg-base-200">
                {photo(0)}{photo_tags(badge, few)}
            </div>
        }
        .into_any(),
        _ => classic(total, photo, badge, few, photos, fits, target).into_any(),
    }
}

/// Clásico: una foto, o la grande arriba y las miniaturas abajo.
fn classic(
    total: usize,
    photo: impl Fn(usize) -> AnyView + Copy + Send + Sync + 'static,
    badge: String,
    few: Option<String>,
    photos: StoredValue<Vec<String>>,
    fits: StoredValue<std::collections::BTreeMap<String, domain::PhotoFit>>,
    target: Option<Memo<Option<usize>>>,
) -> impl IntoView {
    if total < 2 {
        return view! {
            <div class="mk-box mk-image relative h-full overflow-hidden rounded-box bg-base-200">
                {photo(0)}{photo_tags(badge, few)}
            </div>
        }
        .into_any();
    }
    let shown = RwSignal::new(0usize);
    if let Some(t) = target {
        Effect::new(move |_| {
            if let Some(i) = t.get().filter(|i| *i < total) {
                shown.set(i);
            }
        });
    }
    view! {
        <div class="mk-image flex h-full flex-col gap-2">
            <div class="mk-box relative min-h-0 flex-1 overflow-hidden rounded-box bg-base-200">
                {move || photo(shown.get())}
                {photo_tags(badge, few)}
            </div>
            <div class="flex shrink-0 gap-2 overflow-x-auto" role="group" aria-label="Fotos del producto">
                {(0..total).map(|i| view! {
                    <button type="button"
                        class="h-14 w-14 shrink-0 overflow-hidden rounded-field -outline-offset-2 outline-primary"
                        class:outline-2=move || shown.get() == i
                        aria-label=format!("Ver foto {}", i + 1)
                        aria-pressed=move || (shown.get() == i).to_string()
                        on:click=move |_| shown.set(i)>
                        {thumb(photos, fits, i)}
                    </button>
                }).collect_view()}
            </div>
        </div>
    }
    .into_any()
}

fn thumb(
    photos: StoredValue<Vec<String>>,
    fits: StoredValue<std::collections::BTreeMap<String, domain::PhotoFit>>,
    i: usize,
) -> impl IntoView {
    let src = photos.with_value(|p| p[i].clone());
    let fit = fits.with_value(|f| f.get(&src).copied());
    view! { <Image src alt="" fit/> }
}

/// Galería grande (`b`), miniaturas al costado (`d`) o visor (`e`), con
/// varias fotos: un carril que se desliza con el dedo, flechas y puntitos (o
/// miniaturas) que lo mueven.
fn slider(
    variant: String,
    total: usize,
    photo: impl Fn(usize) -> AnyView + Copy + Send + Sync + 'static,
    badge: String,
    few: Option<String>,
    photos: StoredValue<Vec<String>>,
    fits: StoredValue<std::collections::BTreeMap<String, domain::PhotoFit>>,
    target: Option<Memo<Option<usize>>>,
) -> impl IntoView {
    let track = NodeRef::<Div>::new();
    let current = RwSignal::new(0usize);
    // Al tocar una flecha, un puntito o una miniatura se marca enseguida la
    // foto elegida; mientras el carril se desliza hasta ella no se recalcula.
    let pending = StoredValue::new(None::<usize>);
    let go = move |i: usize| {
        let i = i.min(total - 1);
        if let Some(el) = track.get_untracked() {
            pending.set_value(Some(i));
            current.set(i);
            el.set_scroll_left(el.client_width() * i as i32);
        }
    };
    if let Some(t) = target {
        Effect::new(move |_| {
            if let Some(i) = t.get() {
                go(i);
            }
        });
    }
    let rail = view! {
        <div node_ref=track class="mk-ptrack" tabindex="0" aria-label="Fotos del producto"
            on:pointerdown=move |_| pending.set_value(None)
            on:wheel=move |_| pending.set_value(None)
            on:keydown=move |_| pending.set_value(None)
            on:scroll=move |_| {
                if let Some(el) = track.get_untracked() {
                    let w = el.client_width().max(1);
                    let i = (((el.scroll_left() + w / 2) / w) as usize).min(total - 1);
                    if let Some(target) = pending.get_value() {
                        if (el.scroll_left() - w * target as i32).abs() <= 2 {
                            pending.set_value(None);
                        }
                        return;
                    }
                    if current.get_untracked() != i {
                        current.set(i);
                    }
                }
            }>
            {(0..total).map(|i| view! { <div class="mk-pslide">{photo(i)}</div> }).collect_view()}
        </div>
    };
    let arrow = move |dir: i32, class: &'static str| {
        let (label, icon) = if dir < 0 {
            ("Foto anterior", "chevron-left")
        } else {
            ("Foto siguiente", "chevron-right")
        };
        view! {
            <button type="button" class=format!("mk-parrow btn btn-circle btn-sm {class}") aria-label=label
                disabled=move || if dir < 0 { current.get() == 0 } else { current.get() + 1 >= total }
                on:click=move |_| go((current.get_untracked() as i32 + dir).max(0) as usize)>
                <Icon name=icon class="h-4 w-4"/>
            </button>
        }
    };
    let dots = move |class: &'static str| {
        view! {
            <div class=format!("mk-pdots {class}") role="group" aria-label="Elegir foto">
                {(0..total).map(|i| view! {
                    <button type="button" aria-label=format!("Ver foto {} de {total}", i + 1)
                        aria-current=move || (current.get() == i).then_some("true")
                        on:click=move |_| go(i)><i></i></button>
                }).collect_view()}
            </div>
        }
    };
    match variant.as_str() {
        "d" => view! {
            <div class="mk-image mk-pside grid h-full gap-2.5">
                <div class="mk-pthumbs flex min-h-0 flex-col gap-2 overflow-y-auto" role="group" aria-label="Miniaturas">
                    {(0..total).map(|i| view! {
                        <button type="button" class="mk-pthumb"
                            aria-label=format!("Ver foto {} de {total}", i + 1)
                            aria-pressed=move || (current.get() == i).to_string()
                            on:click=move |_| go(i)>
                            {thumb(photos, fits, i)}
                        </button>
                    }).collect_view()}
                </div>
                <div class="mk-box relative min-h-0 overflow-hidden rounded-box bg-base-200">
                    {rail}
                    {photo_tags(badge, few)}
                    {arrow(-1, "absolute left-2.5 top-1/2 -translate-y-1/2")}
                    {arrow(1, "absolute right-2.5 top-1/2 -translate-y-1/2")}
                </div>
            </div>
        }
        .into_any(),
        "e" => view! {
            <div class="mk-box mk-image mk-pvisor flex h-full flex-col items-center gap-3">
                <div class="mk-pvisor-img relative min-h-0 w-full flex-1 overflow-hidden">
                    {rail}
                    {photo_tags(badge, few)}
                    {arrow(-1, "mk-parrow-glass absolute left-3 top-1/2 -translate-y-1/2")}
                    {arrow(1, "mk-parrow-glass absolute right-3 top-1/2 -translate-y-1/2")}
                </div>
                {dots("mk-pdots-dark")}
            </div>
        }
        .into_any(),
        _ => view! {
            <div class="mk-box mk-image relative h-full overflow-hidden rounded-box bg-base-200">
                {rail}
                {photo_tags(badge, few)}
                {dots("mk-pdots-light absolute bottom-3 left-3")}
                <div class="absolute bottom-3 right-3 flex gap-2">
                    {arrow(-1, "")}
                    {arrow(1, "")}
                </div>
            </div>
        }
        .into_any(),
    }
}

/// Tarjeta chica (el ícono de las cuotas).
fn card_icon() -> impl IntoView {
    view! {
        <svg class="h-[1.15em] w-[1.15em] shrink-0" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" aria-hidden="true">
            <rect x="2" y="5" width="20" height="14" rx="2"/><path d="M2 10h20"/>
        </svg>
    }
}

/// El bloque Precio de la ficha.
#[component]
pub fn ProductPriceWidget(
    props: ProductPriceProps,
    product: Option<Product>,
    editing: bool,
) -> impl IntoView {
    let (cents, was, off) = product.as_ref().map_or((0, 0, None), |p| {
        (p.price_cents, p.compare_at_cents, p.discount_pct())
    });
    let mut offers = use_context::<domain::StoreInfo>()
        .map(|s| s.offers)
        .unwrap_or_default();
    // En el editor, si la tienda todavía no eligió cuotas ni descuento, se
    // ve con valores de muestra para que se note el estilo.
    if editing {
        offers = offers.or_sample();
    }
    let PriceOffers {
        installments: n,
        transfer_discount: pct,
        coordinate,
    } = offers;
    let per = props
        .show_installments
        .then(|| offers.installment_cents(cents))
        .flatten();
    let transfer = props
        .show_transfer
        .then(|| offers.transfer_cents(cents))
        .flatten();
    let head = view! {
        <div class="flex flex-wrap items-center gap-2 text-xl leading-tight">
            <span class="mk-pricetag"><PriceWas cents was/></span>
            {off.map(|n| view! { <span class="badge badge-error">{format!("-{n}%")}</span> })}
        </div>
    };
    let body = match props.variant.as_str() {
        "b" => per
            .map(|c| view! {
                <p class="mk-pcuotas flex items-center gap-2 text-sm">{card_icon()}<span><b>{format!("{n} cuotas sin interés")}</b>{format!(" de {}", format_price(c))}</span></p>
            })
            .into_any(),
        "c" => view! {
            {transfer.map(|c| view! {
                <p class="flex flex-wrap items-center gap-2 text-sm">
                    <b class="text-lg">{format_price(c)}</b>" con transferencia"
                    <span class="mk-psave">{format!("{pct}% off")}</span>
                </p>
            })}
            {per.map(|c| view! {
                <p class="text-sm opacity-75">{format!("{n} cuotas sin interés de {}", format_price(c))}</p>
            })}
        }
        .into_any(),
        "d" if per.is_some() || transfer.is_some() || coordinate => view! {
            <div class="mk-item mk-paybox">
                {per.map(|c| view! {
                    <div><span>{format!("💳 {n} cuotas sin interés")}</span><b>{format_price(c)}</b></div>
                })}
                {transfer.map(|c| view! {
                    <div><span>{format!("🏦 Transferencia ({pct}% off)")}</span><b>{format_price(c)}</b></div>
                })}
                {coordinate.then(|| view! {
                    <div><span>"💵 Efectivo o a acordar"</span><b>{format_price(cents)}</b></div>
                })}
            </div>
        }
        .into_any(),
        _ => ().into_any(),
    };
    view! {
        <div class="mk-pprice flex h-full flex-col items-start gap-1 overflow-hidden">
            {head}
            {body}
        </div>
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
                    <div class="absolute inset-0"><Image src=product.image_url.clone() alt=product.name.clone() fit/></div>
                    {look.card_favorite.then(|| view! { <crate::FavoriteHeart product_id=product.id.clone() name=product.name.clone()/> })}
                    <div class="absolute left-2 top-2 flex flex-col items-start gap-1">
                        {(!product.in_stock).then(|| view! { <span class="badge badge-neutral badge-sm">"Agotado"</span> })}
                        {off.map(|n| view! { <span class="badge badge-error badge-sm">{format!("-{n}%")}</span> })}
                        {product.few_left().map(|t| view! { <span class="badge badge-warning badge-sm">{t}</span> })}
                    </div>
                </div>
                <div class="flex shrink-0 flex-col gap-0.5 px-1 pt-2 pb-1">
                    <h3 class="mk-sub truncate text-sm font-semibold">{product.name.clone()}</h3>
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
                </div>
            </div>
            {open_cover(product.id.clone(), &product.name)}
        </div>
    }
}

/// Tarjeta «Limpia con Agregar»: tarjeta blanca redondeada, nombre en
/// mayúsculas y «Agregar» siempre a la vista (suma uno al carrito).
#[allow(non_snake_case)]
pub fn CleanCard(product: Product, show_price: bool) -> impl IntoView {
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
            <button type="button" class="mk-cclean-add btn btn-sm relative z-10" aria-label=label
                disabled=cart.is_none()
                on:click=move |_| {
                    if variants {
                        open_id.with_value(|id| open_product(id));
                    } else if let Some(cart) = cart {
                        id.with_value(|id| cart.add(id, 1));
                        added.set(true);
                        set_timeout(move || added.set(false), std::time::Duration::from_millis(1800));
                    }
                }>
                {move || if added.get() { view! { "✓ Listo" }.into_any() } else if variants { view! { "Elegir" }.into_any() } else { view! { <crate::cart::BagIcon class="h-4 w-4"/>"Agregar" }.into_any() }}
            </button>
        }
    });
    view! {
        <div class="relative h-full" data-product-id=product.id.clone()>
            <div class="mk-item mk-cclean flex h-full min-h-0 flex-col overflow-hidden">
                <div class="mk-cphoto relative min-h-0 flex-1 overflow-hidden bg-base-200">
                    <div class="absolute inset-0"><Image src=product.image_url.clone() alt=product.name.clone() fit/></div>
                    {tag.map(|t| view! { <span class="mk-cclean-tag absolute left-2 top-2">{t}</span> })}
                </div>
                <div class="flex shrink-0 flex-col gap-1 p-3">
                    <h3 class="mk-sub truncate text-sm font-bold uppercase tracking-wide">{product.name.clone()}</h3>
                    <div class="flex items-center justify-between gap-2">
                        {show_price.then(|| view! {
                            <span class="flex min-w-0 flex-wrap items-baseline gap-x-1.5 text-sm">
                                <b>{format_price(product.price_cents)}</b>
                                {(product.compare_at_cents > product.price_cents).then(|| view! {
                                    <s class="text-xs opacity-55"><span class="sr-only">"Antes "</span>{format_price(product.compare_at_cents)}</s>
                                })}
                            </span>
                        })}
                        {add}
                    </div>
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
pub fn VariantBuy(product: Product, #[prop(into)] label: String, cart: Option<crate::Cart>) -> impl IntoView {
    let pick = use_context::<VariantPick>().unwrap_or_default();
    let v = product.variants.clone();
    let colors = v.colors.clone();
    let sizes = v.sizes.clone();
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
            Some((domain::line_id(&pid.get_value(), &c, &s), v.with_value(|v| v.qty(&c, &s))))
        }
    };
    let what_missing = move || match (need_color && pick.color.get().is_empty(), need_size && pick.size.get().is_empty()) {
        (true, true) => "Elegí un color y un talle para agregarlo al carrito.",
        (true, false) => "Elegí un color para agregarlo al carrito.",
        (false, true) => "Elegí un talle para agregarlo al carrito.",
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
                        view! {
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
                <legend class="mk-pvar-lbl">"Talle"</legend>
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
        match line() {
            Some((_, left)) if (1..=3).contains(&left) => {
                let which = domain::variant_label(&pick.color.get(), &pick.size.get());
                let t = if left == 1 { format!("Queda 1 en {which}") } else { format!("Quedan {left} en {which}") };
                view! { <p class="mk-pvar-low">{t}</p> }.into_any()
            }
            _ => ().into_any(),
        }
    };
    let max = Signal::derive(move || line().map_or(1, |(_, q)| q.clamp(1, crate::cart::MAX_QTY)));
    view! {
        <div class="mk-pbtn mk-pvar flex h-full flex-col gap-3">
            {color_set}
            {size_set}
            <div aria-live="polite" class="min-h-4 empty:hidden">{note}</div>
            <div class="flex flex-nowrap items-start gap-2">
                {move || view! { <crate::cart::QtyStepper value=qty label="Cantidad" max=max.get() disabled=cart.is_none()/> }}
                <button type="button" class="btn btn-primary mk-btn min-w-0 flex-1"
                    on:click=move |_| {
                        let Some(cart) = cart else { return };
                        match line() {
                            None => missing.set(true),
                            Some((id, _)) => {
                                cart.add(&id, qty.get());
                                qty.set(1);
                                added.set(true);
                                set_timeout(move || added.set(false), std::time::Duration::from_millis(2000));
                            }
                        }
                    }>
                    {move || if added.get() { "Agregado ✓".to_string() } else { label.clone() }}
                </button>
            </div>
        </div>
    }
}
