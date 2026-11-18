//! La cuenta del comprador en la tienda: el cajón del encabezado, las páginas
//! "Mis pedidos", "Mis datos" y "Direcciones" y la pantalla que canjea el
//! enlace del email. Todo toma el tema de la tienda (daisyUI).
//!
//! El cajón sigue el diseño que eligió Fran: arriba el perfil con la foto
//! (se cambia tocándola), después el pedido en curso con su seguimiento, una
//! lista simple de opciones y "Cerrar sesión" anclado al fondo, que pregunta
//! antes de salir.

use crate::buyer::{
    buyer_delete_account, buyer_logout, buyer_account, buyer_order, buyer_orders, buyer_redeem,
    buyer_request_link, buyer_save_avatar, buyer_save_profile,
};
use crate::checkout::{StoreFrame, StoreHeader};
use crate::products::error_text;
use domain::{BuyerMe, BuyerProfile, CodeRequest, Delivery, Order, OrderStatus};
use leptos::prelude::*;
use leptos::task::spawn_local;
use ui::{format_price, BuyerPageKind, BuyerPageSkeleton, Icon, Image};
use widgets::{Account, AccountState, BuyerAvatar};

// ───────────────────────── Datos chicos ─────────────────────────

const MONTHS: [&str; 12] = [
    "enero",
    "febrero",
    "marzo",
    "abril",
    "mayo",
    "junio",
    "julio",
    "agosto",
    "septiembre",
    "octubre",
    "noviembre",
    "diciembre",
];

/// `2026-11-14T10:00:00Z` → (14, 11, 2026).
fn parse_date(iso: &str) -> Option<(u32, usize, u32)> {
    let mut p = iso.get(..10)?.split('-');
    let y = p.next()?.parse().ok()?;
    let m: usize = p.next()?.parse().ok()?;
    let d = p.next()?.parse().ok()?;
    (1..=12).contains(&m).then_some((d, m, y))
}

/// "14 nov".
fn short_date(iso: &str) -> String {
    parse_date(iso)
        .map(|(d, m, _)| format!("{d} {}", &MONTHS[m - 1][..3]))
        .unwrap_or_default()
}

/// "14 de noviembre de 2026".
fn long_date(iso: &str) -> String {
    parse_date(iso)
        .map(|(d, m, y)| format!("{d} de {} de {y}", MONTHS[m - 1]))
        .unwrap_or_default()
}

/// La clase del distintivo de cada estado (colores del tema de la tienda).
fn badge_class(status: OrderStatus) -> &'static str {
    match status {
        OrderStatus::Delivered => "badge badge-success badge-soft",
        OrderStatus::Shipped => "badge badge-info badge-soft",
        OrderStatus::Paid => "badge badge-primary badge-soft",
        OrderStatus::Cancelled => "badge badge-error badge-soft",
        OrderStatus::New | OrderStatus::PendingPayment => "badge badge-warning badge-soft",
    }
}

/// Un pedido que todavía no llegó: pagado o en camino.
fn in_progress(o: &Order) -> bool {
    matches!(o.status, OrderStatus::Paid | OrderStatus::Shipped)
}

/// Cuánto avanzó, de 0 a 100, para la barra del cajón.
fn progress(status: OrderStatus) -> u8 {
    match status {
        OrderStatus::New | OrderStatus::PendingPayment => 10,
        OrderStatus::Paid => 35,
        OrderStatus::Shipped => 70,
        OrderStatus::Delivered => 100,
        OrderStatus::Cancelled => 0,
    }
}

/// El título de la tarjeta del pedido en curso.
fn hero_title(o: &Order) -> &'static str {
    match (o.status, o.delivery) {
        (OrderStatus::Shipped, _) => "Está en camino",
        (OrderStatus::Paid, Delivery::Pickup) => "Lo estamos preparando para que lo retires",
        _ => "Lo estamos preparando",
    }
}

/// La línea chica de la tarjeta: el estado de Andreani si lo hay.
fn hero_note(o: &Order) -> String {
    match &o.shipment {
        Some(s) if !s.status.is_empty() => format!("Andreani · {}", s.status),
        Some(s) if !s.tracking.is_empty() => format!("Andreani · seguimiento {}", s.tracking),
        _ => format!("Hecho el {}", short_date(&o.created_at)),
    }
}

/// Reduce una foto a un cuadrado de 256 px y la devuelve como data URL (WebP
/// o, si el navegador no lo sabe, JPEG), de menos de 75 KB. Solo en el
/// navegador.
async fn shrink_photo(file: web_sys::File) -> Result<String, String> {
    use wasm_bindgen::{JsCast, JsValue};
    let fail = || "No pudimos usar esa foto. Probá con otra imagen JPG, PNG o WebP.".to_string();
    let f = js_sys::Function::new_with_args(
        "file",
        r#"return (async () => {
            const bmp = await createImageBitmap(file);
            const s = Math.min(bmp.width, bmp.height);
            const c = document.createElement('canvas');
            c.width = c.height = 256;
            c.getContext('2d').drawImage(bmp, (bmp.width - s) / 2, (bmp.height - s) / 2, s, s, 0, 0, 256, 256);
            for (const q of [0.85, 0.7, 0.5, 0.35]) {
                for (const type of ['image/webp', 'image/jpeg']) {
                    const url = c.toDataURL(type, q);
                    if (url.startsWith('data:' + type) && url.length < 75000) return url;
                }
            }
            throw new Error('grande');
        })();"#,
    );
    let promise = f.call1(&JsValue::NULL, file.as_ref()).map_err(|_| fail())?;
    let promise: js_sys::Promise = promise.dyn_into().map_err(|_| fail())?;
    let value = wasm_bindgen_futures::JsFuture::from(promise)
        .await
        .map_err(|_| fail())?;
    value.as_string().ok_or_else(fail)
}

// ───────────────────────── El estado de la cuenta ─────────────────────────

/// Deja la cuenta del comprador en el contexto de la tienda, la lee del
/// servidor al cargar y dibuja el cajón. El botón del encabezado vive en
/// `widgets::AccountNavButton` y abre este cajón.
#[component]
pub(crate) fn BuyerScope(store_name: String, children: Children) -> impl IntoView {
    let account = Account::new();
    provide_context(account);
    // Solo en el navegador: así la página (y su caché) es igual para todos
    // los visitantes sin sesión.
    Effect::new(move |_| {
        spawn_local(async move {
            let state = match buyer_account().await {
                Ok(Some(me)) => AccountState::In(me),
                _ => AccountState::Out,
            };
            account.state.set(state);
        });
    });
    view! {
        {children()}
        <BuyerDrawer account store_name/>
    }
}

/// Cierra la sesión y vuelve a la tienda.
fn sign_out(account: Account) {
    spawn_local(async move {
        _ = buyer_logout().await;
        account.state.set(AccountState::Out);
        account.open.set(false);
        _ = window().location().set_href("/");
    });
}

// ───────────────────────── El cajón ─────────────────────────

#[component]
fn BuyerDrawer(account: Account, store_name: String) -> impl IntoView {
    let dialog_ref = NodeRef::<leptos::html::Dialog>::new();
    Effect::new(move |_| {
        let Some(dialog) = dialog_ref.get() else {
            return;
        };
        if account.open.get() {
            if !dialog.open() {
                _ = dialog.show_modal();
            }
        } else if dialog.open() {
            dialog.close();
        }
    });
    let close = move || account.open.set(false);
    let store_name = StoredValue::new(store_name);
    view! {
        <dialog
            node_ref=dialog_ref
            class="mk-cart mk-acct bg-base-100 text-base-content"
            aria-labelledby="mk-acct-title"
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
            {move || match account.state.get() {
                AccountState::Loading => view! { <ui::BuyerDrawerSkeleton/> }.into_any(),
                AccountState::Out => view! {
                    <div class="flex h-full flex-col px-6 py-5">
                        <button type="button" class="btn btn-sm btn-circle btn-ghost self-end" aria-label="Cerrar" on:click=move |_| close()>
                            <Icon name="close" class="h-4 w-4"/>
                        </button>
                        <div class="flex flex-1 flex-col justify-center pb-10">
                            <SignInForm store_name=store_name.get_value() title_id="mk-acct-title"/>
                        </div>
                    </div>
                }.into_any(),
                AccountState::In(me) => view! { <SignedInPanel me account/> }.into_any(),
            }}
        </dialog>
    }
}

/// El cajón con sesión: perfil, pedido en curso, opciones y salir.
#[component]
fn SignedInPanel(me: BuyerMe, account: Account) -> impl IntoView {
    let orders = RwSignal::new(None::<Vec<Order>>);
    // Cada vez que se abre, trae los pedidos al día.
    Effect::new(move |_| {
        if account.open.get() {
            spawn_local(async move {
                orders.set(buyer_orders().await.ok().flatten());
            });
        }
    });
    let confirm_ref = NodeRef::<leptos::html::Dialog>::new();
    let close = move || account.open.set(false);
    let name = me.display_name();
    let email = me.email.clone();
    view! {
        <div class="flex h-full flex-col">
            <div class="flex-1 overflow-y-auto px-6 pt-5">
                <header class="flex items-center gap-3">
                    <AvatarPicker me=me.clone() account/>
                    <div class="min-w-0 flex-1">
                        <h2 id="mk-acct-title" class="mk-h truncate text-base font-bold">{name}</h2>
                        <p class="truncate text-sm text-base-content/70">{email}</p>
                    </div>
                    <button type="button" class="btn btn-sm btn-circle btn-ghost" aria-label="Cerrar" on:click=move |_| close()>
                        <Icon name="close" class="h-4 w-4"/>
                    </button>
                </header>
                {move || match orders.get() {
                    None => view! { <div class="skeleton mt-5 h-36 w-full rounded-box"></div> }.into_any(),
                    Some(list) => match list.iter().find(|o| in_progress(o)).cloned() {
                        Some(o) => view! { <ActiveOrder order=o/> }.into_any(),
                        None if list.is_empty() => view! {
                            <div class="mt-5 flex flex-col items-center gap-1 rounded-box border border-dashed border-base-300 p-5 text-center">
                                <Icon name="box" class="h-7 w-7 text-base-content/50"/>
                                <p class="font-semibold">"Todavía no tenés pedidos"</p>
                                <p class="text-sm text-base-content/70">"Cuando compres, vas a ver acá cómo va tu pedido."</p>
                                <a class="btn btn-sm btn-primary btn-soft mt-2" href="/">"Ver productos"</a>
                            </div>
                        }.into_any(),
                        None => ().into_any(),
                    },
                }}
                <nav class="mt-3" aria-label="Mi cuenta">
                    <ul class="flex flex-col">
                        <li><a class="flex items-center justify-between border-b border-base-200 py-4" href="/mi-cuenta/pedidos">
                            <span>"Todos mis pedidos"</span>
                            <span class="flex items-center gap-1 text-sm text-base-content/70">
                                {move || orders.get().map(|l| l.len().to_string())}
                                <Icon name="chevron-right" class="h-4 w-4"/>
                            </span>
                        </a></li>
                        <li><a class="flex items-center justify-between border-b border-base-200 py-4" href="/mi-cuenta/datos">
                            <span>"Mis datos"</span><Icon name="chevron-right" class="h-4 w-4 text-base-content/70"/>
                        </a></li>
                        <li><a class="flex items-center justify-between border-b border-base-200 py-4" href="/mi-cuenta/direcciones">
                            <span>"Direcciones"</span><Icon name="chevron-right" class="h-4 w-4 text-base-content/70"/>
                        </a></li>
                    </ul>
                </nav>
            </div>
            // Anclado al fondo: el contenido se desplaza, este botón no.
            <footer class="border-t border-base-300 px-6 py-4">
                <button type="button" class="btn btn-block btn-outline btn-error"
                    on:click=move |_| { if let Some(d) = confirm_ref.get() { _ = d.show_modal(); } }>
                    <Icon name="log-out" class="h-4 w-4"/>"Cerrar sesión"
                </button>
            </footer>
            <dialog node_ref=confirm_ref class="m-auto w-[min(22rem,calc(100vw-2rem))] rounded-box bg-base-100 p-6 text-center text-base-content shadow-xl backdrop:bg-black/50"
                aria-labelledby="mk-signout-title">
                <div class="mx-auto grid size-12 place-items-center rounded-full bg-error/15 text-error" aria-hidden="true">
                    <Icon name="log-out" class="size-5"/>
                </div>
                <h3 id="mk-signout-title" class="mk-h mt-3 text-lg font-bold">"¿Cerrar sesión?"</h3>
                <p class="mt-1.5 text-sm text-base-content/70">"Vas a salir de tu cuenta en esta tienda. Podés volver a entrar cuando quieras con tu email."</p>
                <div class="mt-5 flex flex-col gap-2">
                    <button type="button" class="btn btn-error" on:click=move |_| sign_out(account)>"Sí, cerrar sesión"</button>
                    // La acción segura recibe el foco.
                    <button type="button" class="btn btn-ghost" autofocus
                        on:click=move |_| { if let Some(d) = confirm_ref.get() { d.close(); } }>"Cancelar"</button>
                </div>
            </dialog>
        </div>
    }
}

/// La tarjeta del pedido en curso: qué pasa con su compra y cómo seguirla.
#[component]
fn ActiveOrder(order: Order) -> impl IntoView {
    let pct = progress(order.status);
    let href = format!("/mi-cuenta/pedidos/{}", order.id);
    view! {
        <section class="mt-5 rounded-box bg-primary p-4 text-primary-content" aria-label="Tu pedido en curso">
            <p class="text-xs opacity-80">{format!("Tu pedido #{}", order.number)}</p>
            <p class="mk-h text-lg font-bold">{hero_title(&order)}</p>
            <div class="mt-3 h-1.5 overflow-hidden rounded-full bg-primary-content/30" role="progressbar" aria-valuemin="0" aria-valuemax="100" aria-valuenow=pct.to_string()>
                <div class="h-full rounded-full bg-primary-content" style=format!("width:{pct}%")></div>
            </div>
            <p class="mt-2 text-xs opacity-80">{hero_note(&order)}</p>
            <a class="btn btn-sm mt-3 border-0 bg-base-100 text-base-content" href=href>"Ver seguimiento"</a>
        </section>
    }
}

/// El avatar del cajón: tocarlo abre el selector de fotos y la cambia.
#[component]
fn AvatarPicker(
    me: BuyerMe,
    account: Account,
    /// Clases de tamaño del avatar.
    #[prop(into, default = "h-12 w-12 text-lg".to_string())]
    size: String,
) -> impl IntoView {
    let input_ref = NodeRef::<leptos::html::Input>::new();
    let problem = RwSignal::new(None::<String>);
    let busy = RwSignal::new(false);
    let me_for_save = StoredValue::new(me.clone());
    let on_change = move |ev: leptos::ev::Event| {
        let input = event_target::<web_sys::HtmlInputElement>(&ev);
        let Some(file) = input.files().and_then(|f| f.get(0)) else {
            return;
        };
        // Para poder elegir la misma foto otra vez.
        input.set_value("");
        problem.set(None);
        busy.set(true);
        spawn_local(async move {
            let result = match shrink_photo(file).await {
                Ok(url) => buyer_save_avatar(Some(url.clone()))
                    .await
                    .map(|_| url)
                    .map_err(|e| error_text(&e)),
                Err(e) => Err(e),
            };
            match result {
                Ok(url) => {
                    let mut updated = account_me(account).unwrap_or_else(|| me_for_save.get_value());
                    updated.avatar = Some(url);
                    account.state.set(AccountState::In(updated));
                }
                Err(e) => problem.set(Some(e)),
            }
            busy.set(false);
        });
    };
    view! {
        <div class="relative shrink-0">
            <button type="button" class="group relative block rounded-full" aria-label="Cambiar mi foto de perfil"
                disabled=move || busy.get()
                on:click=move |_| { if let Some(i) = input_ref.get() { i.click(); } }>
                <BuyerAvatar me=me.clone() size=size.clone()/>
                <span class="absolute -right-1 -bottom-1 grid size-6 place-items-center rounded-full border-2 border-base-100 bg-base-200 text-base-content group-hover:bg-base-300" aria-hidden="true">
                    {move || if busy.get() {
                        view! { <span class="loading loading-spinner size-3"></span> }.into_any()
                    } else {
                        view! { <Icon name="camera" class="size-3.5"/> }.into_any()
                    }}
                </span>
            </button>
            <input node_ref=input_ref type="file" accept="image/jpeg,image/png,image/webp,image/*" class="hidden" tabindex="-1" aria-hidden="true" on:change=on_change/>
            {move || problem.get().map(|m| view! { <p role="alert" class="absolute top-full left-0 z-10 mt-2 w-56 text-xs text-error">{m}</p> })}
        </div>
    }
}

/// La cuenta del comprador que está en el contexto, si entró.
fn account_me(account: Account) -> Option<BuyerMe> {
    match account.state.get_untracked() {
        AccountState::In(me) => Some(me),
        _ => None,
    }
}

// ───────────────────────── Ingresar ─────────────────────────

#[derive(Clone, PartialEq)]
enum Step {
    Email,
    Sent(String),
}

/// El formulario para entrar: el email y, después, "Revisá tu email".
#[component]
fn SignInForm(store_name: String, #[prop(into)] title_id: String) -> impl IntoView {
    let email = RwSignal::new(String::new());
    let step = RwSignal::new(Step::Email);
    let busy = RwSignal::new(false);
    let problem = RwSignal::new(None::<String>);
    let send = move |addr: String| {
        if addr.trim().is_empty() {
            problem.set(Some("Escribí tu email.".into()));
            return;
        }
        problem.set(None);
        busy.set(true);
        spawn_local(async move {
            match buyer_request_link(addr.clone()).await {
                Ok(CodeRequest::Sent) => step.set(Step::Sent(addr.trim().to_lowercase())),
                Ok(CodeRequest::BadEmail) => problem.set(Some(
                    "Revisá el email: no parece una dirección válida.".into(),
                )),
                Ok(CodeRequest::TooMany | CodeRequest::NotAllowed) => problem.set(Some(
                    "Pediste muchos enlaces seguidos. Probá de nuevo en un rato.".into(),
                )),
                Err(e) => problem.set(Some(error_text(&e))),
            }
            busy.set(false);
        });
    };
    let link = "link link-primary";
    let store_name = StoredValue::new(store_name);
    view! {
        {move || match step.get() {
            Step::Email => view! {
                <form class="flex flex-col" novalidate on:submit=move |ev: leptos::ev::SubmitEvent| {
                    ev.prevent_default();
                    send(email.get_untracked());
                }>
                    <h2 id=title_id.clone() class="mk-h text-2xl font-bold">"Ingresá a tu cuenta"</h2>
                    <p class="mt-1.5 text-sm text-base-content/70">{format!("Para ver tus pedidos en {}.", store_name.get_value())}</p>
                    <label class="mt-5 text-sm font-semibold" for="mk-buyer-email">"Tu email"</label>
                    <input id="mk-buyer-email" class="input mt-1.5 w-full" type="email" name="email" inputmode="email"
                        autocomplete="email" placeholder="nombre@email.com" autofocus
                        aria-invalid=move || problem.get().is_some().to_string()
                        aria-describedby="mk-buyer-email-note"
                        prop:value=move || email.get()
                        on:input=move |ev| { email.set(event_target_value(&ev)); problem.set(None); }/>
                    <p id="mk-buyer-email-note" role="status" class="mt-1.5 min-h-4 text-xs text-error">{move || problem.get()}</p>
                    <button type="submit" class="btn btn-primary btn-block mt-3" disabled=move || busy.get()>
                        {move || if busy.get() { "Enviando…" } else { "Enviarme el enlace" }}
                    </button>
                    <p class="mt-3 text-center text-xs text-base-content/70">"Sin contraseña: te mandamos un enlace a tu email."</p>
                    <p class="mt-4 rounded-field bg-base-200 px-3.5 py-3 text-xs text-base-content/70">
                        {format!("Entrás a la cuenta de {}. Es distinta de tus cuentas en otras tiendas.", store_name.get_value())}
                    </p>
                </form>
            }.into_any(),
            Step::Sent(addr) => view! {
                <div class="flex flex-col items-center text-center">
                    <div class="grid size-14 place-items-center rounded-full bg-primary/10 text-primary" aria-hidden="true">
                        <Icon name="mail" class="size-6"/>
                    </div>
                    <h2 id=title_id.clone() class="mk-h mt-4 text-2xl font-bold">"Revisá tu email"</h2>
                    // La respuesta es la misma tenga o no pedidos: no decimos si esa
                    // dirección compró acá.
                    <p role="status" class="mt-2 text-sm leading-relaxed text-base-content/70">"Si podés entrar con esta dirección, te mandamos un enlace."</p>
                    <p class="mt-4 inline-flex max-w-full items-center gap-2 rounded-full bg-base-200 px-4 py-2 text-sm font-semibold">
                        <Icon name="mail" class="size-4 shrink-0 text-base-content/50"/>
                        <span class="truncate">{addr.clone()}</span>
                    </p>
                    <p class="mt-3 flex items-center gap-1.5 text-xs text-base-content/60">
                        <Icon name="clock" class="size-4"/>
                        {format!("El enlace vence en {} minutos.", db_link_minutes())}
                    </p>
                    {cfg!(debug_assertions).then(|| view! {
                        <p class="mt-3 text-xs text-base-content/70">"En desarrollo el email no se manda: el enlace aparece en la terminal del servidor."</p>
                    })}
                    <div class="mt-6 flex w-full flex-col gap-1.5 border-t border-base-300 pt-4 text-sm text-base-content/70">
                        <p>
                            "¿No te llegó? Revisá el correo no deseado o "
                            <button type="button" class=link disabled=move || busy.get() on:click={
                                let addr = addr.clone();
                                move |_| send(addr.clone())
                            }>"mandalo de nuevo"</button>
                            "."
                        </p>
                        <p>
                            "¿Te equivocaste de dirección? "
                            <button type="button" class=link on:click=move |_| step.set(Step::Email)>"Usá otro email"</button>
                            "."
                        </p>
                        <p role="status" class="min-h-4 text-xs text-error">{move || problem.get()}</p>
                    </div>
                </div>
            }.into_any(),
        }}
    }
}

/// Cuánto dura el enlace (el mismo número que usa el servidor).
fn db_link_minutes() -> i32 {
    15
}

// ───────────────────────── Páginas ─────────────────────────

/// Pestañas de la cuenta: Pedidos, Mis datos y Direcciones.
#[component]
fn AccountTabs(active: &'static str) -> impl IntoView {
    let tab = move |key: &'static str, href: &'static str, label: &'static str| {
        let on = key == active;
        view! {
            <a href=href aria-current=on.then_some("page")
                class=if on { "border-b-2 border-primary px-4 py-2.5 text-sm font-semibold text-primary" } else { "border-b-2 border-transparent px-4 py-2.5 text-sm font-semibold text-base-content/70 hover:text-base-content" }>
                {label}
            </a>
        }
    };
    view! {
        <nav class="flex overflow-x-auto border-b border-base-300" aria-label="Mi cuenta">
            {tab("pedidos", "/mi-cuenta/pedidos", "Pedidos")}
            {tab("datos", "/mi-cuenta/datos", "Mis datos")}
            {tab("direcciones", "/mi-cuenta/direcciones", "Direcciones")}
        </nav>
    }
}

/// El marco común de las páginas de la cuenta: la tienda y una columna.
#[component]
fn AccountFrame(view: domain::PageView, children: Children) -> impl IntoView {
    let name = view.store_name.clone();
    view! {
        <StoreFrame view>
            <StoreHeader name/>
            <div class="mx-auto flex w-full max-w-2xl flex-col gap-4">{children()}</div>
        </StoreFrame>
    }
}

/// Quien no entró ve el formulario para ingresar, en el mismo marco.
#[component]
fn SignedOut(view: domain::PageView) -> impl IntoView {
    let name = view.store_name.clone();
    view! {
        <AccountFrame view>
            <div class="card mx-auto w-full max-w-md bg-base-100 shadow-sm"><div class="card-body p-6">
                <SignInForm store_name=name title_id="mk-signin-title"/>
            </div></div>
        </AccountFrame>
    }
}

fn not_a_store() -> AnyView {
    view! { <crate::platform::NotAStore/> }.into_any()
}

/// `/mi-cuenta/pedidos`: todos los pedidos que hizo con su email.
#[component]
pub fn BuyerOrdersPage() -> impl IntoView {
    if crate::site::current_site() == domain::Site::Platform {
        return not_a_store();
    }
    let page = Resource::new(|| (), |_| crate::load_page("home".to_string()));
    let orders = Resource::new(|| (), |_| buyer_orders());
    view! {
        <leptos_meta::Title text="Mis pedidos"/>
        <Suspense fallback=|| view! { <BuyerPageSkeleton kind=BuyerPageKind::Orders/> }>
            {move || Suspend::new(async move {
                match (page.await, orders.await) {
                    (Ok(view), Ok(Some(list))) => view! { <OrdersList view list/> }.into_any(),
                    (Ok(view), Ok(None)) => view! { <SignedOut view/> }.into_any(),
                    _ => view! { <crate::platform::StoreNotFound/> }.into_any(),
                }
            })}
        </Suspense>
    }
    .into_any()
}

#[component]
fn OrdersList(view: domain::PageView, list: Vec<Order>) -> impl IntoView {
    let empty = list.is_empty();
    view! {
        <AccountFrame view>
            <h1 class="mk-h text-2xl font-bold">"Mi cuenta"</h1>
            <AccountTabs active="pedidos"/>
            {empty.then(|| view! {
                <div class="card bg-base-100 shadow-sm"><div class="card-body items-center gap-2 py-10 text-center">
                    <Icon name="box" class="h-9 w-9 text-base-content/40"/>
                    <h2 class="mk-h text-lg font-bold">"Todavía no tenés pedidos"</h2>
                    <p class="text-base-content/70">"Cuando compres, vas a ver acá cómo va cada pedido."</p>
                    <a class="btn btn-primary mt-2" href="/">"Ver productos"</a>
                </div></div>
            })}
            <ul class="flex flex-col gap-3">
                {list.into_iter().map(|o| view! { <li><OrderCard order=o/></li> }).collect_view()}
            </ul>
        </AccountFrame>
    }
}

/// Un pedido en la lista: toda la tarjeta lleva al detalle.
#[component]
fn OrderCard(order: Order) -> impl IntoView {
    let count: u32 = order.items.iter().map(|i| i.qty).sum();
    let extra = order.items.len().saturating_sub(3);
    let href = format!("/mi-cuenta/pedidos/{}", order.id);
    view! {
        <a href=href class="card bg-base-100 shadow-sm transition hover:shadow-md focus-visible:outline-2 focus-visible:outline-primary">
            <div class="card-body gap-3 p-5">
                <div class="flex items-center gap-2.5">
                    <h2 class="font-bold">{format!("Pedido #{}", order.number)}</h2>
                    <span class=badge_class(order.status)>{order.status.label()}</span>
                    <span class="ml-auto text-sm text-base-content/70">{short_date(&order.created_at)}</span>
                </div>
                <div class="flex items-center gap-2">
                    {order.items.iter().take(3).map(|i| view! {
                        <div class="size-10 shrink-0 overflow-hidden rounded-field"><Image src=i.image_url.clone() alt=""/></div>
                    }).collect_view()}
                    {(extra > 0).then(|| view! { <span class="text-sm text-base-content/70">{format!("+{extra}")}</span> })}
                    <span class="ml-2 text-sm text-base-content/70">{if count == 1 { "1 producto".to_string() } else { format!("{count} productos") }}</span>
                    <span class="ml-auto font-bold tabular-nums">{format_price(order.total_cents)}</span>
                </div>
            </div>
        </a>
    }
}

/// `/mi-cuenta/pedidos/:id`: el detalle de un pedido.
#[component]
pub fn BuyerOrderPage() -> impl IntoView {
    if crate::site::current_site() == domain::Site::Platform {
        return not_a_store();
    }
    let params = leptos_router::hooks::use_params_map();
    let page = Resource::new(|| (), |_| crate::load_page("home".to_string()));
    let order = Resource::new(
        move || params.with(|p| p.get("id").unwrap_or_default()),
        buyer_order,
    );
    let first_page = Resource::new(|| (), |_| buyer_orders());
    view! {
        <leptos_meta::Title text="Mi pedido"/>
        <Suspense fallback=|| view! { <BuyerPageSkeleton kind=BuyerPageKind::Order/> }>
            {move || Suspend::new(async move {
                match (page.await, order.await, first_page.await) {
                    (Ok(view), Ok(Some(order)), _) => view! { <OrderDetail view order/> }.into_any(),
                    (Ok(view), Ok(None), Ok(None)) => view! { <SignedOut view/> }.into_any(),
                    (Ok(view), Ok(None), _) => view! { <OrderMissing view/> }.into_any(),
                    _ => view! { <crate::platform::StoreNotFound/> }.into_any(),
                }
            })}
        </Suspense>
    }
    .into_any()
}

#[component]
fn OrderMissing(view: domain::PageView) -> impl IntoView {
    view! {
        <AccountFrame view>
            <div class="card bg-base-100 shadow-sm"><div class="card-body items-center gap-2 py-10 text-center">
                <h1 class="mk-h text-xl font-bold">"No encontramos ese pedido"</h1>
                <p class="text-base-content/70">"Puede que sea de otra cuenta o que ya no exista."</p>
                <a class="btn btn-primary mt-2" href="/mi-cuenta/pedidos">"Ver mis pedidos"</a>
            </div></div>
        </AccountFrame>
    }
}

#[component]
fn OrderDetail(view: domain::PageView, order: Order) -> impl IntoView {
    let delivery = Delivery::label(order.delivery);
    let rank = match order.status {
        OrderStatus::New | OrderStatus::PendingPayment => 0,
        OrderStatus::Paid => 1,
        OrderStatus::Shipped => 2,
        OrderStatus::Delivered | OrderStatus::Cancelled => 3,
    };
    let cancelled = order.status == OrderStatus::Cancelled;
    let pickup = matches!(order.delivery, Delivery::Pickup);
    let steps = [
        ("Recibido", true),
        ("Pagado", rank >= 1),
        (if pickup { "Listo" } else { "Enviado" }, rank >= 2),
        (if pickup { "Retirado" } else { "Entregado" }, rank >= 3 && !cancelled),
    ];
    let where_ = order.destination().unwrap_or_default();
    let needs_payment = matches!(order.status, OrderStatus::New);
    let pay_href = format!("/pedido/{}", order.id);
    let tracking = order.shipment.clone().filter(|s| !s.tracking.is_empty());
    let shipping_text = if matches!(order.delivery, Delivery::Pickup) || order.shipping_cents == 0 {
        "Gratis".to_string()
    } else {
        format_price(order.shipping_cents)
    };
    view! {
        <AccountFrame view>
            <a class="link link-hover self-start text-sm text-base-content/70" href="/mi-cuenta/pedidos">"← Mis pedidos"</a>
            <div class="card bg-base-100 shadow-sm"><div class="card-body gap-4 p-6">
                <div class="flex flex-wrap items-center gap-3">
                    <h1 class="mk-h text-2xl font-bold">{format!("Pedido #{}", order.number)}</h1>
                    <span class=badge_class(order.status)>{order.status.label()}</span>
                </div>
                <p class="text-sm text-base-content/70">{format!("Hecho el {}", long_date(&order.created_at))}</p>
                {if cancelled {
                    view! { <p class="rounded-field bg-base-200 px-4 py-3 text-sm">"Este pedido fue cancelado. Si tenés dudas, escribile a la tienda."</p> }.into_any()
                } else {
                    view! {
                        <ol class="grid grid-cols-2 gap-4 sm:grid-cols-4" aria-label="Estado del pedido">
                            {steps.into_iter().map(|(label, done)| view! {
                                <li class="flex flex-col gap-1.5 text-sm">
                                    <span class=if done { "size-4 rounded-full bg-primary" } else { "size-4 rounded-full bg-base-300" }></span>
                                    <span class=if done { "font-semibold" } else { "text-base-content/60" }>{label}</span>
                                    <span class="sr-only">{if done { "listo" } else { "pendiente" }}</span>
                                </li>
                            }).collect_view()}
                        </ol>
                    }.into_any()
                }}
                {needs_payment.then(|| view! {
                    <div class="flex flex-wrap items-center justify-between gap-3 rounded-box bg-warning/10 p-4">
                        <p class="text-sm">"Falta el pago para que la tienda empiece a prepararlo."</p>
                        <a class="btn btn-sm btn-primary" href=pay_href.clone()>"Ver cómo pagar"</a>
                    </div>
                })}
                {tracking.map(|s| view! {
                    <div class="rounded-box bg-primary/8 p-4">
                        <p class="font-semibold">"Seguimiento de Andreani"</p>
                        <p class="mt-1 text-sm text-base-content/70">
                            {format!("Código {}", s.tracking)}
                            {(!s.status.is_empty()).then(|| format!(" · {}", s.status))}
                        </p>
                        <a class="btn btn-sm btn-primary btn-soft mt-3" href=domain::tracking_url(&s.tracking) target="_blank" rel="noopener noreferrer">
                            "Ver seguimiento"<Icon name="external" class="size-3.5"/>
                        </a>
                    </div>
                })}
                <ul>
                    {order.items.iter().map(|i| view! {
                        <li class="flex items-center gap-3 border-t border-base-200 py-3">
                            <div class="size-12 shrink-0 overflow-hidden rounded-field"><Image src=i.image_url.clone() alt=""/></div>
                            <div class="min-w-0 flex-1">
                                <p class="truncate text-sm font-semibold">{i.name.clone()}</p>
                                <p class="text-xs text-base-content/70">{format!("{} × {}", i.qty, format_price(i.unit_cents))}</p>
                            </div>
                            <span class="text-sm font-semibold tabular-nums">{format_price(i.total_cents())}</span>
                        </li>
                    }).collect_view()}
                </ul>
                <div class="flex justify-between text-sm text-base-content/75"><span>"Subtotal"</span><span>{format_price(order.subtotal_cents)}</span></div>
                <div class="-mt-2 flex justify-between text-sm text-base-content/75"><span>{delivery}</span><span>{shipping_text}</span></div>
                <div class="flex justify-between text-lg font-bold"><span>"Total"</span><span>{format_price(order.total_cents)}</span></div>
                {(!where_.is_empty()).then(|| view! {
                    <div>
                        <h2 class="font-semibold">"Entrega"</h2>
                        <p class="text-sm text-base-content/75">{where_}</p>
                    </div>
                })}
            </div></div>
        </AccountFrame>
    }
}

/// `/mi-cuenta/datos` y `/mi-cuenta/direcciones`: lo que guarda la tienda de
/// la persona. `section` es `"datos"` o `"direcciones"`.
#[component]
pub fn BuyerProfilePage(section: &'static str) -> impl IntoView {
    if crate::site::current_site() == domain::Site::Platform {
        return not_a_store();
    }
    let page = Resource::new(|| (), |_| crate::load_page("home".to_string()));
    let me = Resource::new(|| (), |_| buyer_account());
    view! {
        <leptos_meta::Title text="Mi cuenta"/>
        <Suspense fallback=|| view! { <BuyerPageSkeleton kind=BuyerPageKind::Form/> }>
            {move || Suspend::new(async move {
                match (page.await, me.await) {
                    (Ok(view), Ok(Some(me))) => view! { <ProfileForm view me section/> }.into_any(),
                    (Ok(view), Ok(None)) => view! { <SignedOut view/> }.into_any(),
                    _ => view! { <crate::platform::StoreNotFound/> }.into_any(),
                }
            })}
        </Suspense>
    }
    .into_any()
}

#[component]
fn ProfileForm(view: domain::PageView, me: BuyerMe, section: &'static str) -> impl IntoView {
    // La página no tiene el cajón: su propia cuenta alcanza para el avatar.
    let account = Account::new();
    account.state.set(AccountState::In(me.clone()));
    let name = RwSignal::new(me.name.clone());
    let phone = RwSignal::new(me.phone.clone());
    let address = RwSignal::new(me.address.clone());
    let postal = RwSignal::new(me.postal_code.clone());
    let city = RwSignal::new(me.city.clone());
    let saved = RwSignal::new(false);
    let problem = RwSignal::new(None::<String>);
    let busy = RwSignal::new(false);
    let confirm_delete = RwSignal::new(false);
    let save = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        saved.set(false);
        problem.set(None);
        busy.set(true);
        let profile = BuyerProfile {
            name: name.get_untracked(),
            phone: phone.get_untracked(),
            address: address.get_untracked(),
            postal_code: postal.get_untracked(),
            city: city.get_untracked(),
        };
        spawn_local(async move {
            match buyer_save_profile(profile).await {
                Ok(()) => saved.set(true),
                Err(e) => problem.set(Some(error_text(&e))),
            }
            busy.set(false);
        });
    };
    let remove_photo = move |_| {
        spawn_local(async move {
            if buyer_save_avatar(None).await.is_ok() {
                if let Some(mut m) = account_me(account) {
                    m.avatar = None;
                    account.state.set(AccountState::In(m));
                }
            }
        });
    };
    let delete = move |_| {
        spawn_local(async move {
            if buyer_delete_account().await.is_ok() {
                _ = window().location().set_href("/");
            }
        });
    };
    let email = me.email.clone();
    let on_data = section == "datos";
    view! {
        <AccountFrame view>
            <h1 class="mk-h text-2xl font-bold">"Mi cuenta"</h1>
            <AccountTabs active=section/>
            <form class="card bg-base-100 shadow-sm" novalidate on:submit=save>
                <div class="card-body gap-5 p-6">
                    {on_data.then(|| view! {
                        <div class="flex items-center gap-4">
                            {move || match account.state.get() {
                                AccountState::In(m) => view! { <AvatarPicker me=m account size="size-20 text-3xl"/> }.into_any(),
                                _ => ().into_any(),
                            }}
                            <div class="flex flex-col items-start gap-1">
                                <p class="font-semibold">"Tu foto de perfil"</p>
                                <p class="text-xs text-base-content/70">"Tocala para cambiarla."</p>
                                {move || account_me(account).and_then(|m| m.avatar).map(|_| view! {
                                    <button type="button" class="link link-hover text-xs text-base-content/70" on:click=remove_photo>"Quitar foto"</button>
                                })}
                            </div>
                        </div>
                        <div class="flex flex-col gap-1.5">
                            <label class="text-sm font-semibold" for="mk-p-email">"Email"</label>
                            <input id="mk-p-email" class="input w-full" type="email" value=email.clone() readonly autocomplete="email"/>
                            <p class="text-xs text-base-content/60">"Con este email entrás a tu cuenta y vemos tus pedidos."</p>
                        </div>
                        <div class="flex flex-col gap-1.5">
                            <label class="text-sm font-semibold" for="mk-p-name">"Nombre y apellido"</label>
                            <input id="mk-p-name" class="input w-full" type="text" autocomplete="name" maxlength="80"
                                prop:value=move || name.get() on:input=move |ev| name.set(event_target_value(&ev))/>
                        </div>
                        <div class="flex flex-col gap-1.5">
                            <label class="text-sm font-semibold" for="mk-p-phone">"Teléfono"</label>
                            <input id="mk-p-phone" class="input w-full" type="tel" inputmode="tel" autocomplete="tel" maxlength="30"
                                prop:value=move || phone.get() on:input=move |ev| phone.set(event_target_value(&ev))/>
                        </div>
                    })}
                    {(!on_data).then(|| view! {
                        <p class="text-sm text-base-content/70">"La usamos para completar tu compra más rápido."</p>
                        <div class="flex flex-col gap-1.5">
                            <label class="text-sm font-semibold" for="mk-p-address">"Calle y número"</label>
                            <input id="mk-p-address" class="input w-full" type="text" autocomplete="address-line1" maxlength="160"
                                prop:value=move || address.get() on:input=move |ev| address.set(event_target_value(&ev))/>
                        </div>
                        <div class="grid grid-cols-3 gap-3">
                            <div class="col-span-2 flex flex-col gap-1.5">
                                <label class="text-sm font-semibold" for="mk-p-city">"Ciudad"</label>
                                <input id="mk-p-city" class="input w-full" type="text" autocomplete="address-level2" maxlength="80"
                                    prop:value=move || city.get() on:input=move |ev| city.set(event_target_value(&ev))/>
                            </div>
                            <div class="flex flex-col gap-1.5">
                                <label class="text-sm font-semibold" for="mk-p-postal">"Código postal"</label>
                                <input id="mk-p-postal" class="input w-full" type="text" inputmode="numeric" autocomplete="postal-code" maxlength="12"
                                    prop:value=move || postal.get() on:input=move |ev| postal.set(event_target_value(&ev))/>
                            </div>
                        </div>
                    })}
                    <div class="flex items-center gap-3">
                        <button type="submit" class="btn btn-primary" disabled=move || busy.get()>
                            {move || if busy.get() { "Guardando…" } else { "Guardar" }}
                        </button>
                        <p role="status" class="text-sm text-success">{move || saved.get().then_some("Guardamos tus datos.")}</p>
                        <p role="status" class="text-xs text-error">{move || problem.get()}</p>
                    </div>
                </div>
            </form>
            {on_data.then(|| view! {
                <div class="card bg-base-100 shadow-sm"><div class="card-body gap-3 p-6">
                    <h2 class="font-semibold">"Salir y borrar"</h2>
                    <div class="flex flex-wrap gap-3">
                        <button type="button" class="btn btn-outline btn-sm" on:click=move |_| {
                            spawn_local(async move { _ = buyer_logout().await; _ = window().location().set_href("/"); });
                        }>"Cerrar sesión"</button>
                        <Show when=move || !confirm_delete.get() fallback=move || view! {
                            <div class="flex w-full flex-col gap-2 rounded-field bg-error/10 p-3 text-sm">
                                <p>"Se borra tu cuenta en esta tienda: tu foto y tus datos guardados. Los pedidos que hiciste quedan en la tienda."</p>
                                <div class="flex gap-2">
                                    <button type="button" class="btn btn-error btn-sm" on:click=delete>"Sí, borrar mi cuenta"</button>
                                    <button type="button" class="btn btn-ghost btn-sm" autofocus on:click=move |_| confirm_delete.set(false)>"Cancelar"</button>
                                </div>
                            </div>
                        }>
                            <button type="button" class="btn btn-ghost btn-sm text-error" on:click=move |_| confirm_delete.set(true)>"Borrar mi cuenta"</button>
                        </Show>
                    </div>
                </div></div>
            })}
        </AccountFrame>
    }
}

/// `/mi-cuenta/entrar?k=…`: canjea el enlace del email. Se hace con un pedido
/// desde la página (no con solo abrir el enlace) para que los programas que
/// revisan los enlaces de los emails no lo gasten antes que la persona.
#[component]
pub fn BuyerEnterPage() -> impl IntoView {
    if crate::site::current_site() == domain::Site::Platform {
        return not_a_store();
    }
    let page = Resource::new(|| (), |_| crate::load_page("home".to_string()));
    let query = leptos_router::hooks::use_query_map();
    let failed = RwSignal::new(false);
    Effect::new(move |_| {
        let key = query.with_untracked(|q| q.get("k").unwrap_or_default());
        spawn_local(async move {
            match buyer_redeem(key).await {
                Ok(true) => _ = window().location().set_href("/mi-cuenta/pedidos"),
                _ => failed.set(true),
            }
        });
    });
    view! {
        <leptos_meta::Title text="Entrar a mi cuenta"/>
        <Suspense fallback=|| view! { <BuyerPageSkeleton kind=BuyerPageKind::Orders/> }>
            {move || Suspend::new(async move {
                match page.await {
                    Ok(view) => {
                        let name = view.store_name.clone();
                        view! {
                            <AccountFrame view>
                                <div class="card mx-auto w-full max-w-md bg-base-100 shadow-sm"><div class="card-body p-6">
                                    {move || if failed.get() {
                                        view! {
                                            <div class="flex flex-col gap-1">
                                                <h1 class="mk-h text-xl font-bold">"Este enlace ya no sirve"</h1>
                                                <p class="mb-4 text-sm text-base-content/70">"Venció o ya se usó. Pedí uno nuevo y te lo mandamos al instante."</p>
                                                <SignInForm store_name=name.clone() title_id="mk-enter-title"/>
                                            </div>
                                        }.into_any()
                                    } else {
                                        view! {
                                            <div class="flex flex-col items-center gap-3 py-6 text-center" role="status">
                                                <span class="loading loading-spinner loading-lg text-primary"></span>
                                                <p class="font-semibold">"Entrando a tu cuenta…"</p>
                                            </div>
                                        }.into_any()
                                    }}
                                </div></div>
                            </AccountFrame>
                        }.into_any()
                    }
                    Err(_) => view! { <crate::platform::StoreNotFound/> }.into_any(),
                }
            })}
        </Suspense>
    }
    .into_any()
}
