//! Pedidos de una tienda, en la plataforma (`/tienda/:slug/pedidos`): un
//! tablero con una columna por estado. Se mueven arrastrando o con "Mover a…"
//! (que también anda con teclado), y al tocar uno se abre su detalle.

use crate::products::{error_text, slug_param, SignInFirst};
use domain::{Delivery, Order, OrderColumn, OrderStatus};
use leptos::prelude::*;
use ui::{format_price, Image};

/// Nombre de la tienda `slug` y sus pedidos. `None` si no entró.
#[server]
pub async fn store_orders(slug: String) -> Result<Option<(String, Vec<Order>)>, ServerFnError> {
    use crate::server::{internal, pool};
    let Some(tenant) = crate::products::server::owned_store(&slug).await? else {
        return Ok(None);
    };
    let orders = db::orders::list(&pool()?, tenant.id)
        .await
        .map_err(internal)?;
    Ok(Some((tenant.name, orders)))
}

#[server(input = leptos::server_fn::codec::Json)]
pub async fn move_order(
    slug: String,
    id: String,
    status: OrderStatus,
) -> Result<(), ServerFnError> {
    use crate::server::{internal, pool};
    let tenant = crate::products::server::require_store(&slug).await?;
    let id = uuid::Uuid::parse_str(&id).map_err(|_| ServerFnError::new("Ese pedido no existe."))?;
    if db::orders::set_status(&pool()?, tenant.id, id, status)
        .await
        .map_err(internal)?
    {
        Ok(())
    } else {
        Err(ServerFnError::new("Ese pedido no existe."))
    }
}

/// `/tienda/:slug/pedidos`.
#[component]
pub fn OrdersPage() -> impl IntoView {
    if matches!(crate::site::current_site(), domain::Site::Store(_)) {
        return view! { <crate::Home/> }.into_any();
    }
    let slug = slug_param();
    let data = Resource::new(move || slug.get(), store_orders);
    view! {
        <leptos_meta::Title text="Pedidos · MarketMaker"/>
        <crate::panel::StoreShell slug=slug.get_untracked() section=crate::panel::Section::Orders>
            <div class="mx-auto flex max-w-[1280px] flex-col gap-5 px-6 py-8">
                <Suspense fallback=|| view! { <ui::Skeleton class="h-96 w-full"/> }>
                    {move || Suspend::new(async move {
                        match data.await {
                            Ok(Some((_, orders))) => view! { <Board slug=slug.get_untracked() orders/> }.into_any(),
                            Ok(None) => view! { <SignInFirst back=format!("/tienda/{}/pedidos", slug.get_untracked())/> }.into_any(),
                            Err(e) => view! { <div role="alert" class="alert alert-error">{error_text(&e)}</div> }.into_any(),
                        }
                    })}
                </Suspense>
            </div>
        </crate::panel::StoreShell>
    }
    .into_any()
}

/// Lo que comparten las piezas del tablero.
#[derive(Clone, Copy)]
struct BoardCtx {
    orders: RwSignal<Vec<Order>>,
    /// El pedido abierto en el costado.
    open: RwSignal<Option<String>>,
    /// El que se está arrastrando.
    dragging: RwSignal<Option<String>>,
    problem: RwSignal<Option<String>>,
    /// Aviso para lectores de pantalla y para quien mira: "Pedido #1001 movido a…".
    notice: RwSignal<String>,
    slug: StoredValue<String>,
}

impl BoardCtx {
    fn status_of(&self, id: &str) -> Option<OrderStatus> {
        self.orders
            .with_untracked(|os| os.iter().find(|o| o.id == id).map(|o| o.status))
    }

    /// Mueve el pedido enseguida en pantalla y lo guarda; si falla, vuelve.
    fn set_status(self, id: String, status: OrderStatus) {
        let Some(before) = self.status_of(&id) else {
            return;
        };
        if before == status
            || (OrderColumn::of(before).is_some()
                && OrderColumn::of(before) == OrderColumn::of(status))
        {
            return;
        }
        let id_set = id.clone();
        let set = move |s: OrderStatus| {
            self.orders.update(|os| {
                if let Some(o) = os.iter_mut().find(|o| o.id == id_set) {
                    o.status = s;
                }
            });
        };
        set(status);
        self.problem.set(None);
        let number = self
            .orders
            .with_untracked(|os| os.iter().find(|o| o.id == id).map(|o| o.number))
            .unwrap_or_default();
        let place = match OrderColumn::of(status) {
            Some(c) => c.title().to_string(),
            None => "Cancelados".to_string(),
        };
        self.notice
            .set(format!("Pedido #{number} movido a {place}."));
        let id2 = id.clone();
        let slug = self.slug.get_value();
        leptos::task::spawn_local(async move {
            if let Err(e) = move_order(slug, id2, status).await {
                set(before);
                self.notice.set(String::new());
                self.problem.set(Some(format!(
                    "No pudimos mover el pedido #{number}. {}",
                    error_text(&e)
                )));
            }
        });
    }
}

#[component]
fn Board(slug: String, orders: Vec<Order>) -> impl IntoView {
    let ctx = BoardCtx {
        orders: RwSignal::new(orders),
        open: RwSignal::new(None),
        dragging: RwSignal::new(None),
        problem: RwSignal::new(None),
        notice: RwSignal::new(String::new()),
        slug: StoredValue::new(slug.clone()),
    };
    let show_cancelled = RwSignal::new(false);
    let cancelled = Memo::new(move |_| {
        ctx.orders.with(|os| {
            os.iter()
                .filter(|o| o.status == OrderStatus::Cancelled)
                .cloned()
                .collect::<Vec<_>>()
        })
    });
    let empty = ctx.orders.with_untracked(Vec::is_empty);
    view! {
        <div class="flex flex-wrap items-center gap-3">
            <div class="grow">
                <h1 class="text-3xl font-bold">"Pedidos"</h1>
                <p class="text-sm text-base-content/70">"Arrastrá un pedido a otra columna o usá \"Mover a…\"."</p>
            </div>
        </div>
        {move || ctx.problem.get().map(|e| view! { <div role="alert" class="alert alert-error text-sm">{e}</div> })}
        <p class="sr-only" role="status">{move || ctx.notice.get()}</p>
        {empty.then(|| view! {
            <div class="card bg-base-100 shadow-sm"><div class="card-body items-start">
                <h2 class="card-title">"Todavía no tenés pedidos"</h2>
                <p>"Con Mercado Pago, el pedido aparece acá, en \"Para preparar\", cuando se aprueba el pago. Con transferencia o a coordinar, aparece en \"Esperando pago\" hasta que toques \"Ya me pagó\"."</p>
            </div></div>
        })}
        <div class="grid grid-cols-4 items-start gap-4">
            {OrderColumn::ALL.into_iter().map(|c| view! { <Column ctx column=c/> }).collect_view()}
        </div>
        {move || (!cancelled.with(Vec::is_empty)).then(|| view! {
            <div>
                <button class="btn btn-ghost btn-sm" aria-expanded=move || show_cancelled.get().to_string()
                    on:click=move |_| show_cancelled.update(|v| *v = !*v)>
                    {move || format!("{} cancelados ({})", if show_cancelled.get() { "Ocultar" } else { "Ver" }, cancelled.with(Vec::len))}
                </button>
                <Show when=move || show_cancelled.get()>
                    <ul class="mt-2 grid grid-cols-4 gap-3" aria-label="Pedidos cancelados">
                        <For each=move || cancelled.get() key=|o| o.id.clone() let:o>
                            <li><OrderCard ctx order=o/></li>
                        </For>
                    </ul>
                </Show>
            </div>
        })}
        <OrderDrawer ctx/>
    }
}

#[component]
fn Column(ctx: BoardCtx, column: OrderColumn) -> impl IntoView {
    let over = RwSignal::new(false);
    let mine = Memo::new(move |_| {
        ctx.orders.with(|os| {
            os.iter()
                .filter(|o| OrderColumn::of(o.status) == Some(column))
                .cloned()
                .collect::<Vec<_>>()
        })
    });
    let heading = format!("col-{}", column.status().as_str());
    let labelled = heading.clone();
    view! {
        <section class="flex min-h-64 flex-col gap-2 rounded-box bg-base-300/50 p-3 transition-colors"
            class:outline-2=move || over.get() class:outline-dashed=move || over.get() class:outline-primary=move || over.get()
            aria-labelledby=labelled
            on:dragover=move |ev| {
                if ctx.dragging.with_untracked(Option::is_some) {
                    ev.prevent_default();
                    over.set(true);
                }
            }
            on:dragleave=move |_| over.set(false)
            on:drop=move |ev| {
                ev.prevent_default();
                over.set(false);
                if let Some(id) = ctx.dragging.get_untracked() {
                    ctx.dragging.set(None);
                    ctx.set_status(id, column.status());
                }
            }>
            <h2 id=heading class="flex items-center justify-between px-1 font-semibold">
                {column.title()}
                <span class="badge badge-ghost badge-sm">{move || mine.with(Vec::len)}</span>
            </h2>
            <ul class="flex flex-col gap-2">
                <For each=move || mine.get() key=|o| (o.id.clone(), o.status) let:o>
                    <li><OrderCard ctx order=o/></li>
                </For>
            </ul>
            {move || mine.with(Vec::is_empty).then(|| view! {
                <p class="px-1 py-6 text-center text-sm text-base-content/50">"Ninguno"</p>
            })}
        </section>
    }
}

fn where_short(o: &Order) -> String {
    match o.delivery {
        Delivery::Pickup => "Retiro".to_string(),
        Delivery::Shipping if o.city.is_empty() => "Envío".to_string(),
        Delivery::Shipping => format!("Envío {}", o.city),
    }
}

#[component]
fn OrderCard(ctx: BoardCtx, order: Order) -> impl IntoView {
    let id = order.id.clone();
    let id_drag = id.clone();
    let id_open = id.clone();
    let is_open = Memo::new(move |_| ctx.open.with(|o| o.as_deref() == Some(id_open.as_str())));
    let id_click = id.clone();
    let thumbs = order
        .items
        .iter()
        .take(4)
        .map(|i| i.image_url.clone())
        .collect::<Vec<_>>();
    let units: u32 = order.items.iter().map(|i| i.qty).sum();
    view! {
        <div class="group relative rounded-field bg-base-100 shadow-sm transition-shadow hover:shadow-md"
            class:outline-2=move || is_open.get() class:outline-primary=move || is_open.get()
            class:opacity-40=move || ctx.dragging.with(|d| d.as_deref() == Some(id.as_str()))
            draggable="true"
            on:dragstart=move |ev| {
                if let Some(dt) = ev.data_transfer() {
                    _ = dt.set_data("text/plain", &id_drag);
                    dt.set_effect_allowed("move");
                }
                ctx.dragging.set(Some(id_drag.clone()));
            }
            on:dragend=move |_| ctx.dragging.set(None)>
            <button type="button" class="block w-full cursor-pointer rounded-field p-3 pr-10 text-left"
                aria-haspopup="dialog"
                on:click=move |_| ctx.open.set(Some(id_click.clone()))>
                <span class="block truncate"><b>{format!("#{}", order.number)}</b>" · "{order.customer_name.clone()}</span>
                <span class="block text-sm text-base-content/70">{format!("{} · {}", format_price(order.total_cents), where_short(&order))}</span>
                <span class="mt-2 flex items-center gap-1">
                    {thumbs.into_iter().map(|src| view! {
                        <span class="block h-7 w-7 overflow-hidden rounded-sm"><Image src alt=""/></span>
                    }).collect_view()}
                    <span class="ml-auto text-xs text-base-content/60">
                        {format!("{} · {}", if units == 1 { "1 unidad".to_string() } else { format!("{units} unidades") }, domain::order_time_ar(&order.created_at))}
                    </span>
                </span>
            </button>
            <MoveMenu ctx id=order.id.clone() number=order.number status=order.status/>
        </div>
    }
}

/// "Mover a…": el camino con teclado (y sin arrastrar) para cambiar de columna.
#[component]
fn MoveMenu(ctx: BoardCtx, id: String, number: i32, status: OrderStatus) -> impl IntoView {
    let here = OrderColumn::of(status);
    let id = StoredValue::new(id);
    let close = move |ev: leptos::ev::MouseEvent| {
        use leptos::wasm_bindgen::JsCast;
        if let Some(d) = ev
            .target()
            .and_then(|t| t.dyn_into::<leptos::web_sys::Element>().ok())
            .and_then(|e| e.closest("details").ok().flatten())
        {
            _ = d.remove_attribute("open");
        }
    };
    view! {
        <details class="dropdown dropdown-end absolute top-2 right-2">
            <summary class="btn btn-ghost btn-xs btn-square list-none" title="Mover a…"
                aria-label=format!("Mover pedido #{number} a…")>
                <svg class="h-4 w-4" viewBox="0 0 24 24" fill="currentColor" aria-hidden="true"><circle cx="5" cy="12" r="1.8"/><circle cx="12" cy="12" r="1.8"/><circle cx="19" cy="12" r="1.8"/></svg>
            </summary>
            <ul class="menu dropdown-content z-10 w-52 rounded-box bg-base-100 p-2 shadow-lg">
                <li class="menu-title">"Mover a…"</li>
                {OrderColumn::ALL.into_iter().filter(|c| Some(*c) != here).map(|c| view! {
                    <li><button type="button" on:click=move |ev| { close(ev); ctx.set_status(id.get_value(), c.status()); }>{c.title()}</button></li>
                }).collect_view()}
                {(status != OrderStatus::Cancelled).then(|| view! {
                    <li><button type="button" class="text-error" on:click=move |ev| { close(ev); ctx.open.set(Some(id.get_value())); }>"Cancelar…"</button></li>
                })}
            </ul>
        </details>
    }
}

/// El detalle del pedido abierto, en un costado.
#[component]
fn OrderDrawer(ctx: BoardCtx) -> impl IntoView {
    let dialog = NodeRef::<leptos::html::Dialog>::new();
    let order = Memo::new(move |_| {
        let id = ctx.open.get()?;
        ctx.orders
            .with(|os| os.iter().find(|o| o.id == id).cloned())
    });
    let confirm_cancel = RwSignal::new(false);
    Effect::new(move |_| {
        let Some(d) = dialog.get() else { return };
        if ctx.open.get().is_some() {
            if !d.open() {
                _ = d.show_modal();
            }
        } else if d.open() {
            d.close();
        }
    });
    Effect::new(move |_| {
        ctx.open.track();
        confirm_cancel.set(false);
    });
    view! {
        <dialog node_ref=dialog class="mk-cart" aria-labelledby="order-title"
            on:close=move |_| ctx.open.set(None)
            on:click=move |ev| {
                // Tocar el fondo oscuro cierra.
                use leptos::wasm_bindgen::JsCast;
                if ev.target().and_then(|t| t.dyn_into::<leptos::web_sys::HtmlDialogElement>().ok()).is_some() {
                    ctx.open.set(None);
                }
            }>
            {move || order.get().map(|o| view! { <OrderDetail ctx order=o confirm_cancel/> })}
        </dialog>
    }
}

#[component]
fn OrderDetail(ctx: BoardCtx, order: Order, confirm_cancel: RwSignal<bool>) -> impl IntoView {
    let id = StoredValue::new(order.id.clone());
    let column = OrderColumn::of(order.status);
    let digits = domain::phone_digits(&order.phone);
    // Números argentinos: 54 9 + área + número para WhatsApp.
    let wa = if digits.starts_with("54") {
        digits.clone()
    } else {
        format!("549{}", digits.trim_start_matches('0'))
    };
    let where_ = match order.delivery {
        Delivery::Pickup => "Lo retira en el local".to_string(),
        Delivery::Shipping => format!("{}, {} ({})", order.address, order.city, order.postal_code),
    };
    let shipping_text = match order.delivery {
        Delivery::Pickup => "Gratis".to_string(),
        Delivery::Shipping if order.shipping_cents == 0 => "Gratis o a coordinar".to_string(),
        Delivery::Shipping => format_price(order.shipping_cents),
    };
    let delivery = order.delivery;
    view! {
        <div class="flex h-full flex-col">
            <header class="flex items-start gap-3 border-b border-base-200 p-6">
                <div class="grow">
                    <h2 id="order-title" class="text-2xl font-bold">{format!("Pedido #{}", order.number)}</h2>
                    <p class="text-sm text-base-content/70">{format!("{} · {} · {}", order.status.label(), order.pay.label(), domain::order_time_ar(&order.created_at))}</p>
                </div>
                <button type="button" class="btn btn-ghost btn-sm btn-square" aria-label="Cerrar" on:click=move |_| ctx.open.set(None)>"✕"</button>
            </header>
            <div class="grow overflow-y-auto p-6">
                <h3 class="font-semibold">{order.customer_name.clone()}</h3>
                <p class="text-sm text-base-content/75">{order.email.clone()}</p>
                <p class="text-sm text-base-content/75">{order.phone.clone()}</p>
                <div class="mt-3 flex gap-2">
                    <a class="btn btn-sm btn-success" href=format!("https://wa.me/{wa}?text={}", urlencode(&format!("Hola {}! Te escribo por tu pedido #{}.", order.customer_name.split_whitespace().next().unwrap_or(""), order.number))) target="_blank" rel="noopener">"Escribir por WhatsApp"</a>
                    <a class="btn btn-sm btn-ghost" href=format!("mailto:{}?subject={}", order.email, urlencode(&format!("Tu pedido #{}", order.number)))>"Email"</a>
                </div>
                <h3 class="mt-5 font-semibold">{order.delivery.label()}</h3>
                <p class="text-sm text-base-content/75">{where_}</p>
                {(!order.notes.is_empty()).then(|| view! {
                    <div class="mt-3 rounded-field bg-base-200 p-3 text-sm"><b>"Nota: "</b>{order.notes.clone()}</div>
                })}
                <ul class="mt-4">
                    {order.items.iter().map(|i| view! {
                        <li class="flex items-center gap-3 border-b border-base-200 py-3">
                            <div class="h-11 w-11 shrink-0 overflow-hidden rounded-field"><Image src=i.image_url.clone() alt=""/></div>
                            <div class="min-w-0 flex-1">
                                <p class="truncate text-sm font-semibold">{i.name.clone()}</p>
                                <p class="text-xs text-base-content/70">{format!("{} × {}", i.qty, format_price(i.unit_cents))}</p>
                            </div>
                            <span class="text-sm font-semibold tabular-nums">{format_price(i.total_cents())}</span>
                        </li>
                    }).collect_view()}
                </ul>
                <div class="mt-3 flex justify-between text-sm text-base-content/75"><span>"Subtotal"</span><span>{format_price(order.subtotal_cents)}</span></div>
                <div class="mt-1 flex justify-between text-sm text-base-content/75"><span>"Envío"</span><span>{shipping_text}</span></div>
                <div class="mt-2 flex justify-between text-lg font-bold"><span>"Total"</span><span>{format_price(order.total_cents)}</span></div>
            </div>
            <footer class="flex flex-col gap-2 border-t border-base-200 p-6">
                {match column {
                    Some(c) => view! {
                        {c.next().map(|n| view! {
                            <button type="button" class="btn btn-primary btn-block"
                                on:click=move |_| ctx.set_status(id.get_value(), n.status())>{n.action(delivery)}</button>
                        })}
                        <div class="flex gap-2">
                            <label class="select select-sm grow">
                                <span class="label">"Mover a"</span>
                                <select aria-label="Mover a otra columna"
                                    on:change=move |ev| {
                                        let v = event_target_value(&ev);
                                        if let Some(c) = OrderColumn::ALL.into_iter().find(|c| c.status().as_str() == v) {
                                            ctx.set_status(id.get_value(), c.status());
                                        }
                                    }>
                                    {OrderColumn::ALL.into_iter().map(|o| view! {
                                        <option value=o.status().as_str() selected=Some(o) == column>{o.title()}</option>
                                    }).collect_view()}
                                </select>
                            </label>
                            {move || if confirm_cancel.get() {
                                view! {
                                    <button type="button" class="btn btn-error btn-sm"
                                        on:click=move |_| ctx.set_status(id.get_value(), OrderStatus::Cancelled)>"Sí, cancelar"</button>
                                    <button type="button" class="btn btn-ghost btn-sm" on:click=move |_| confirm_cancel.set(false)>"No"</button>
                                }.into_any()
                            } else {
                                view! {
                                    <button type="button" class="btn btn-ghost btn-sm text-error" on:click=move |_| confirm_cancel.set(true)>"Cancelar pedido…"</button>
                                }.into_any()
                            }}
                        </div>
                    }.into_any(),
                    None => view! {
                        <p class="text-sm text-base-content/70">"Este pedido está cancelado."</p>
                        <button type="button" class="btn btn-outline btn-block"
                            on:click=move |_| ctx.set_status(id.get_value(), OrderColumn::ToPrepare.status())>"Recuperar a Para preparar"</button>
                    }.into_any(),
                }}
            </footer>
        </div>
    }
}

fn urlencode(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}
