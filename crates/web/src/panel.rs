//! El panel de la plataforma: el menú al costado con el selector de tienda
//! (que envuelve Productos, Pedidos, Entregas, Cobros y Mis fotos) y el
//! Inicio de cada tienda (`/tienda/:slug`).

use crate::products::{error_text, slug_param, SignInFirst};
use crate::stores::open_store;
use domain::{OrderColumn, OrderStatus, PanelNav, PublishState, StoreOverview};
use leptos::prelude::*;

/// En qué parte del panel se está, para marcarla en el menú.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Section {
    Home,
    Products,
    Orders,
    Delivery,
    Payments,
    StoreInfo,
    Photos,
    /// Mi cuenta: no está en el menú, se abre desde tu nombre.
    Account,
}

/// Quién entró y sus tiendas, para el menú. `None` si no entró.
#[server]
pub async fn load_panel_nav() -> Result<Option<PanelNav>, ServerFnError> {
    use crate::server::{internal, pool};
    let Some(user) = crate::auth::current_user().await? else {
        return Ok(None);
    };
    let pool = pool()?;
    let rows = db::auth::stores_for_user(&pool, user.id)
        .await
        .map_err(internal)?;
    let counts = db::orders::to_prepare_by_store(&pool, user.id)
        .await
        .map_err(internal)?;
    let stores = rows
        .into_iter()
        .map(|r| domain::NavStore {
            url: crate::stores::store_url(&r.slug),
            to_prepare: counts
                .iter()
                .find(|(s, _)| *s == r.slug)
                .map_or(0, |(_, n)| *n),
            name: r.name,
            slug: r.slug,
        })
        .collect();
    let is_admin = crate::auth::is_admin(&user.email);
    Ok(Some(PanelNav {
        email: user.email,
        name: user.name,
        avatar: user.avatar_id.map(domain::avatar_url),
        stores,
        is_admin,
    }))
}

/// Los números y pendientes de la tienda `slug`. `None` si no entró.
#[server]
pub async fn load_store_overview(slug: String) -> Result<Option<StoreOverview>, ServerFnError> {
    use crate::server::{internal, pool};
    let Some(tenant) = crate::products::server::owned_store(&slug).await? else {
        return Ok(None);
    };
    let pool = pool()?;
    let page = db::get_current_page(&pool, tenant.id, "home")
        .await
        .map_err(internal)?;
    let publish = page.map_or(PublishState::Never, |p| {
        PublishState::of(p.version, p.published)
    });
    let sums = db::orders::summary(&pool, tenant.id)
        .await
        .map_err(internal)?;
    let delivery = db::orders::delivery(&pool, tenant.id)
        .await
        .map_err(internal)?;
    let mp_linked = db::mp::link(&pool, tenant.id)
        .await
        .map_err(internal)?
        .is_some();
    let options = db::orders::payment_options(&pool, tenant.id)
        .await
        .map_err(internal)?;
    Ok(Some(StoreOverview {
        url: crate::stores::store_url(&tenant.slug),
        name: tenant.name,
        slug: tenant.slug,
        publish,
        awaiting_payment: sums.awaiting_payment,
        to_prepare: sums.to_prepare,
        month_orders: sums.month_orders,
        month_cents: sums.month_cents,
        products: db::products::count_real_visible(&pool, tenant.id)
            .await
            .map_err(internal)?,
        examples: db::products::count_examples(&pool, tenant.id)
            .await
            .map_err(internal)?,
        out_of_stock: db::products::out_of_stock(&pool, tenant.id)
            .await
            .map_err(internal)?,
        delivery_ready: (delivery.pickup_enabled || delivery.shipping_enabled)
            && db::orders::delivery_set(&pool, tenant.id)
                .await
                .map_err(internal)?,
        mp_linked,
        pay_ready: mp_linked || options.transfer || options.coordinate,
        recent: db::orders::recent(&pool, tenant.id, 5)
            .await
            .map_err(internal)?,
    }))
}

/// Borra los productos de ejemplo que trajo la tienda nueva.
#[server]
pub async fn delete_examples(slug: String) -> Result<(), ServerFnError> {
    use crate::server::{internal, pool};
    let tenant = crate::products::server::require_store(&slug).await?;
    db::products::delete_examples(&pool()?, tenant.id)
        .await
        .map_err(internal)?;
    Ok(())
}

/// Abre el editor de una tienda (en su propia dirección, con la sesión ya
/// abierta). Devuelve la acción y el error para mostrar.
fn editor_opener() -> (
    Action<String, Result<String, ServerFnError>>,
    RwSignal<Option<String>>,
) {
    let open = Action::new(|slug: &String| open_store(slug.clone()));
    let problem = RwSignal::new(None::<String>);
    Effect::new(move |_| match open.value().get() {
        Some(Ok(url)) => crate::platform::go(&url),
        Some(Err(_)) => problem.set(Some(
            "No pudimos abrir el editor. Recargá la página y probá de nuevo.".into(),
        )),
        None => {}
    });
    (open, problem)
}

/// Inicial de la tienda sobre un fondo de color, siempre el mismo para la
/// misma dirección.
#[component]
fn StoreMark(name: String, slug: String, #[prop(optional)] small: bool) -> impl IntoView {
    const COLORS: [&str; 6] = [
        "from-indigo-300 to-pink-300",
        "from-amber-200 to-rose-300",
        "from-emerald-200 to-sky-300",
        "from-sky-200 to-violet-300",
        "from-rose-200 to-orange-300",
        "from-lime-200 to-teal-300",
    ];
    let pick = slug.bytes().map(usize::from).sum::<usize>() % COLORS.len();
    let letter = name
        .chars()
        .next()
        .unwrap_or('?')
        .to_uppercase()
        .to_string();
    let size = if small {
        "size-6 text-xs rounded-md"
    } else {
        "size-9 text-base rounded-lg"
    };
    view! {
        <span aria-hidden="true" class=format!(
            "grid shrink-0 place-items-center bg-gradient-to-br font-bold text-white {size} {}",
            COLORS[pick]
        )>{letter}</span>
    }
}

/// La foto de perfil de la persona, o su inicial en un círculo naranja.
#[component]
pub fn Avatar(
    letter: String,
    #[prop(into)] size: String,
    #[prop(default = None)] photo: Option<String>,
) -> impl IntoView {
    match photo {
        Some(src) => view! {
            <img src=src alt="" class=format!("shrink-0 rounded-full bg-base-200 object-cover {size}")/>
        }
        .into_any(),
        None => view! {
            <span aria-hidden="true" class=format!(
                "grid shrink-0 place-items-center rounded-full bg-gradient-to-br from-[#ffb547] to-[#ff7a59] font-bold text-white {size}"
            )>{letter}</span>
        }
        .into_any(),
    }
}

/// El último menú del panel que se cargó (se guarda en el contexto de la app).
#[derive(Clone, Copy)]
pub struct NavCache(pub RwSignal<Option<PanelNav>>);

/// En qué parte del panel está una dirección `/tienda/:slug/...`.
fn section_of(path: &str) -> Section {
    let rest = path
        .trim_start_matches("/tienda/")
        .split_once('/')
        .map_or("", |(_, r)| r);
    match rest.split('/').next().unwrap_or("") {
        "productos" => Section::Products,
        "pedidos" => Section::Orders,
        "ventas" => Section::Delivery,
        "cobros" => Section::Payments,
        "datos" => Section::StoreInfo,
        _ => Section::Home,
    }
}

/// El marco de todas las páginas de `/tienda/:slug/...`: el menú al costado se
/// arma una sola vez y queda en pantalla mientras cambia solo el contenido
/// (por eso no hay recarga ni parpadeo al tocar una opción).
#[component]
pub fn PanelLayout() -> impl IntoView {
    if matches!(crate::site::current_site(), domain::Site::Store(_)) {
        return view! { <crate::Home/> }.into_any();
    }
    let slug = slug_param();
    let location = leptos_router::hooks::use_location();
    let section = Memo::new(move |_| section_of(&location.pathname.get()));
    view! {
        <Shell slug=slug.into() section=section.into()>
            <leptos_router::components::Outlet/>
        </Shell>
    }
    .into_any()
}

/// Las páginas de una tienda en la plataforma: menú al costado y contenido.
/// (Las que no cuelgan de `/tienda/:slug`, como Mi cuenta y Mis fotos.)
#[component]
pub fn StoreShell(slug: String, section: Section, children: Children) -> impl IntoView {
    view! {
        <Shell slug=Signal::stored(slug) section=Signal::stored(section)>{children()}</Shell>
    }
}

#[component]
fn Shell(slug: Signal<String>, section: Signal<Section>, children: Children) -> impl IntoView {
    let location = leptos_router::hooks::use_location();
    // Se vuelve a pedir en cada cambio de página para refrescar el número de
    // pedidos por preparar; con `Transition` el menú no se vacía mientras tanto.
    let nav = Resource::new(
        move || (slug.get(), location.pathname.get()),
        |_| load_panel_nav(),
    );
    // Lo último que se supo del menú: al pasar a Mi cuenta o Mis fotos (que
    // arman su propio marco) se muestra al instante en vez de un hueco.
    let last = use_context::<NavCache>().map(|c| c.0);
    Effect::new(move |_| {
        if let (Some(last), Some(Ok(Some(n)))) = (last, nav.get()) {
            last.set(Some(n));
        }
    });
    // En pantallas chicas, al elegir una opción el menú se cierra solo.
    Effect::new(move |_| {
        location.pathname.track();
        if let Some(el) = document().get_element_by_id("mk-panel-nav") {
            use leptos::wasm_bindgen::JsCast;
            if let Ok(input) = el.dyn_into::<leptos::web_sys::HtmlInputElement>() {
                input.set_checked(false);
            }
        }
    });
    // La Whitelist se abre desde el menú de la cuenta, pero su ventana va fuera
    // del cajón del costado: ahí adentro quedaba apretada al ancho del menú.
    let whitelist = NodeRef::<leptos::html::Dialog>::new();
    let whitelist_version = RwSignal::new(0u32);
    // La ventana existe solo en el navegador: se arma después de cargar la
    // página y solo para el administrador.
    let on_client = RwSignal::new(false);
    Effect::new(move |_| on_client.set(true));
    let is_admin = move || {
        on_client.get()
            && nav
                .get()
                .and_then(|r| r.ok())
                .flatten()
                .is_some_and(|n| n.is_admin)
    };
    view! {
        // En pantallas chicas el menú se esconde y se abre con el botón de
        // arriba (cajón de daisyUI: funciona aunque la página no cargó el JS).
        <div data-theme="light" class="drawer min-h-screen bg-base-200 text-base-content lg:drawer-open">
            <input id="mk-panel-nav" type="checkbox" class="drawer-toggle" aria-hidden="true" tabindex="-1"/>
            <div class="drawer-content min-w-0">
                <header class="sticky top-0 z-20 flex h-14 items-center gap-2 border-b border-base-300 bg-base-100 px-2 lg:hidden">
                    <label for="mk-panel-nav" role="button" tabindex="0" class="btn btn-ghost btn-square" aria-label="Abrir el menú"
                        on:keydown=|ev| {
                            if ev.key() == "Enter" || ev.key() == " " {
                                ev.prevent_default();
                                if let Some(el) = document().get_element_by_id("mk-panel-nav") {
                                    use leptos::wasm_bindgen::JsCast;
                                    if let Ok(input) = el.dyn_into::<leptos::web_sys::HtmlInputElement>() {
                                        input.set_checked(!input.checked());
                                    }
                                }
                            }
                        }>
                        <ui::Icon class="size-6" name="menu"/>
                    </label>
                    <a href="/" class="text-lg font-bold">"MarketMaker"</a>
                </header>
                {children()}
            </div>
            <div class="drawer-side z-30">
                <label for="mk-panel-nav" aria-label="Cerrar el menú" class="drawer-overlay"></label>
                <aside class="sticky top-0 flex h-screen w-64 shrink-0 flex-col gap-3 border-r border-base-300 bg-base-100 p-3">
                    <a href="/" class="px-2 pt-1 text-lg font-bold">"MarketMaker"</a>
                    <Transition fallback=move || match last.and_then(|l| l.get_untracked()) {
                            Some(nav) => view! { <SideNav nav slug=slug.get_untracked() section whitelist whitelist_version/> }.into_any(),
                            None => view! { <ui::Skeleton class="h-12 w-full"/> }.into_any(),
                        }>
                        {move || Suspend::new(async move {
                            match nav.await {
                                Ok(Some(nav)) => view! { <SideNav nav slug=slug.get_untracked() section whitelist whitelist_version/> }.into_any(),
                                _ => ().into_any(),
                            }
                        })}
                    </Transition>
                </aside>
            </div>
            {move || is_admin().then(|| view! {
                <crate::whitelist::WhitelistDialog dialog=whitelist version=whitelist_version/>
            })}
        </div>
    }
}

#[component]
fn SideNav(
    nav: PanelNav,
    slug: String,
    section: Signal<Section>,
    whitelist: NodeRef<leptos::html::Dialog>,
    whitelist_version: RwSignal<u32>,
) -> impl IntoView {
    let current = nav
        .stores
        .iter()
        .find(|s| s.slug == slug)
        .or(nav.stores.first())
        .cloned();
    let (open, open_problem) = editor_opener();
    let out = Action::new(|_: &()| crate::auth::sign_out());
    Effect::new(move |_| {
        if let Some(Ok(())) = out.value().get() {
            crate::platform::go("/");
        }
    });
    let dialog = NodeRef::<leptos::html::Dialog>::new();
    let is_admin = nav.is_admin;
    let menu = NodeRef::<leptos::html::Details>::new();
    let account_menu = NodeRef::<leptos::html::Details>::new();
    let open_dialog = move |_| {
        if let Some(m) = menu.get() {
            m.set_open(false);
        }
        if let Some(d) = dialog.get() {
            _ = d.show_modal();
        }
    };
    let close_dialog = Callback::new(move |_: ()| {
        if let Some(d) = dialog.get() {
            d.close();
        }
    });

    let item = move |s: Section,
                     href: String,
                     icon: &'static str,
                     label: &'static str,
                     badge: i64| {
        let on = move || s == section.get();
        view! {
            <li>
                <a href=href class=move || if on() { "bg-primary/10 font-semibold text-primary" } else { "" } aria-current=move || on().then_some("page")>
                    <ui::Icon class="size-[18px]" name=icon/>
                    {label}
                    {(badge > 0).then(|| view! {
                        <span class="badge badge-primary badge-sm ml-auto" aria-label=format!("{badge} para preparar")>{badge}</span>
                    })}
                </a>
            </li>
        }
    };

    let switcher = current.clone().map(|c| {
        let others = nav.stores.clone();
        view! {
            <details node_ref=menu class="dropdown w-full">
                <summary class="flex cursor-pointer list-none items-center gap-2.5 rounded-xl border border-base-300 p-2 hover:bg-base-200">
                    <StoreMark name=c.name.clone() slug=c.slug.clone()/>
                    <span class="min-w-0 grow">
                        <span class="block truncate font-semibold">{c.name.clone()}</span>
                        <span class="block text-xs text-base-content/60">"Cambiar de tienda"</span>
                    </span>
                    <ui::Icon class="size-[18px]" name="chevron-down"/>
                </summary>
                <ul class="dropdown-content menu z-20 mt-1 w-full rounded-box border border-base-300 bg-base-100 p-1.5 shadow-lg">
                    <li class="menu-title">"Tus tiendas"</li>
                    {others.into_iter().map(|s| {
                        let on = s.slug == c.slug;
                        // Mis fotos y Mi cuenta leen la tienda una sola vez: ahí se
                        // recarga la página. Las demás cambian de tienda sin recargar.
                        let (href, external) = match section.get_untracked() {
                            Section::Photos => (format!("/mis-fotos?tienda={}", s.slug), true),
                            Section::Account => (format!("/cuenta?tienda={}", s.slug), true),
                            _ => (format!("/tienda/{}", s.slug), false),
                        };
                        view! {
                            <li><a href=href rel=external.then_some("external") class=if on { "bg-base-200 font-semibold" } else { "" }>
                                <StoreMark name=s.name.clone() slug=s.slug.clone() small=true/>
                                <span class="grow truncate">{s.name.clone()}</span>
                                {on.then(|| view! { <ui::Icon class="size-[18px]" name="check"/> })}
                            </a></li>
                        }
                    }).collect_view()}
                    <li class="mt-1 border-t border-base-300 pt-1">
                        <button type="button" class="font-semibold text-primary" on:click=open_dialog>
                            <ui::Icon class="size-[18px]" name="plus"/>"Crear tienda"
                        </button>
                    </li>
                </ul>
            </details>
        }
    });

    let s = current.clone().map(|c| c.slug).unwrap_or_default();
    let to_prepare = current.as_ref().map_or(0, |c| c.to_prepare);
    let base = format!("/tienda/{s}");
    let design_slug = s.clone();
    let initial = nav
        .name
        .chars()
        .next()
        .or(nav.email.chars().next())
        .unwrap_or('?')
        .to_uppercase()
        .to_string();
    let shown_name = if nav.name.is_empty() {
        "Mi cuenta".to_string()
    } else {
        nav.name.clone()
    };
    view! {
        {switcher}
        {move || open_problem.get().map(|e| view! { <div role="alert" class="alert alert-error text-xs">{e}</div> })}
        <nav aria-label="Tu tienda" class="grow overflow-y-auto">
            <ul class="menu w-full gap-0.5 p-0">
                {item(Section::Home, base.clone(), "home", "Inicio", 0)}
                <li>
                    <button type="button" disabled=move || open.pending().get()
                        on:click=move |_| { open.dispatch(design_slug.clone()); }>
                        <ui::Icon class="size-[18px]" name="pencil"/>"Diseño"
                        <span class="ml-auto text-xs text-base-content/70">"Abre el editor"</span>
                    </button>
                </li>
                {item(Section::Products, format!("{base}/productos"), "box", "Productos", 0)}
                {item(Section::Orders, format!("{base}/pedidos"), "bag", "Pedidos", to_prepare)}
                <li class="menu-title mt-3 text-base-content/75">"Cómo vendés"</li>
                {item(Section::Delivery, format!("{base}/ventas"), "truck", "Entregas", 0)}
                {item(Section::Payments, format!("{base}/cobros"), "card", "Cobros", 0)}
                {item(Section::StoreInfo, format!("{base}/datos"), "store", "Datos de la tienda", 0)}
                <li class="menu-title mt-3 text-base-content/75">"Tu cuenta"</li>
                {item(Section::Photos, format!("/mis-fotos?tienda={s}"), "photo", "Mis fotos", 0)}
            </ul>
        </nav>
        <details node_ref=account_menu class="dropdown dropdown-top w-full border-t border-base-300 pt-3">
            <summary class=move || if section.get() == Section::Account { "flex cursor-pointer list-none items-center gap-2 rounded-lg bg-base-200 p-1.5" } else { "flex cursor-pointer list-none items-center gap-2 rounded-lg p-1.5 hover:bg-base-200" }
                aria-label="Tu cuenta">
                <Avatar letter=initial.clone() size="size-8 text-sm" photo=nav.avatar.clone()/>
                <span class="flex min-w-0 grow flex-col leading-tight">
                    <span class="truncate text-sm font-semibold">{shown_name.clone()}</span>
                    <span class="truncate text-xs text-base-content/60">{nav.email.clone()}</span>
                </span>
            </summary>
            <div class="dropdown-content z-30 mb-2 w-64 rounded-2xl border border-base-300 bg-base-100 p-1.5 shadow-xl">
                <div class="mb-1 flex items-center gap-3 border-b border-base-300 px-2.5 pt-2.5 pb-3">
                    <Avatar letter=initial size="size-10 text-base" photo=nav.avatar.clone()/>
                    <div class="min-w-0">
                        <div class="truncate font-bold">{shown_name}</div>
                        <div class="truncate text-xs text-base-content/60">{nav.email.clone()}</div>
                    </div>
                </div>
                <ul class="menu w-full p-0">
                    <li><a href=format!("/cuenta?tienda={s}") class=move || if section.get() == Section::Account { "bg-base-200" } else { "" }>
                        <ui::Icon class="size-[18px] text-base-content/60" name="user"/>"Mi cuenta"
                    </a></li>
                    {is_admin.then(|| view! {
                        <li><a href="/admin/uso">
                            <ui::Icon class="size-[18px] text-base-content/60" name="chart"/>"Uso y abandono"
                        </a></li>
                    })}
                    {is_admin.then(|| view! {
                        <li><button type="button" on:click=move |_| {
                            if let Some(m) = account_menu.get() { m.set_open(false); }
                            // daisyUI mantiene abierto el menú mientras tiene el foco.
                            if let Some(el) = document().active_element() {
                                use leptos::wasm_bindgen::JsCast;
                                if let Ok(el) = el.dyn_into::<leptos::web_sys::HtmlElement>() { _ = el.blur(); }
                            }
                            crate::whitelist::open_whitelist(whitelist, whitelist_version);
                        }>
                            <ui::Icon class="size-[18px] text-base-content/60" name="shield"/>"Whitelist"
                        </button></li>
                    })}
                    <li class="mt-1 border-t border-base-300 pt-1"><button type="button" on:click=move |_| { out.dispatch(()); }>
                        <ui::Icon class="size-[18px] text-base-content/60" name="log-out"/>"Salir"
                    </button></li>
                </ul>
            </div>
        </details>
        <dialog node_ref=dialog class="modal" aria-labelledby="crear-tienda-titulo">
            <div class="modal-box max-w-lg">
                <crate::platform::NewStore first=false on_cancel=close_dialog/>
            </div>
            <form method="dialog" class="modal-backdrop"><button>"Cerrar"</button></form>
        </dialog>
    }
}

/// `/tienda/:slug`: el Inicio de la tienda.
#[component]
pub fn StoreHomePage() -> impl IntoView {
    if matches!(crate::site::current_site(), domain::Site::Store(_)) {
        return view! { <crate::Home/> }.into_any();
    }
    let slug = slug_param();
    let data = Resource::new(move || slug.get(), load_store_overview);
    view! {
        <leptos_meta::Title text="Inicio · MarketMaker"/>
            <div class="mx-auto flex max-w-6xl flex-col gap-6 px-8 py-8">
                <Suspense fallback=|| view! { <ui::Skeleton class="h-96 w-full"/> }>
                    {move || Suspend::new(async move {
                        match data.await {
                            Ok(Some(o)) => view! { <Overview o/> }.into_any(),
                            Ok(None) => view! { <SignInFirst back=format!("/tienda/{}", slug.get_untracked())/> }.into_any(),
                            Err(e) => view! { <div role="alert" class="alert alert-error">{error_text(&e)}</div> }.into_any(),
                        }
                    })}
                </Suspense>
            </div>
    }
    .into_any()
}

/// Colores de las etiquetas de estado (texto oscuro, contraste AA).
const OK: &str = "bg-emerald-100 text-emerald-800";
const WARN: &str = "bg-amber-100 text-amber-800";

fn status_badge(status: OrderStatus) -> impl IntoView {
    let (label, class) = match OrderColumn::of(status) {
        Some(OrderColumn::AwaitingPayment) => ("Esperando pago", WARN),
        Some(OrderColumn::ToPrepare) => ("Para preparar", "bg-primary/10 text-primary"),
        Some(OrderColumn::Shipped) => ("Enviado", "bg-base-200 text-base-content/80"),
        Some(OrderColumn::Delivered) => ("Entregado", OK),
        None => ("Cancelado", "bg-base-200 text-base-content/60"),
    };
    view! { <span class=format!("badge badge-sm border-0 font-semibold {class}")>{label}</span> }
}

#[component]
fn Kpi(
    label: &'static str,
    value: String,
    #[prop(optional)] link: Option<(String, &'static str)>,
    #[prop(optional)] note: Option<String>,
) -> impl IntoView {
    view! {
        <div class="card border border-base-300 bg-base-100">
            <div class="card-body gap-1 p-5">
                <h2 class="text-xs font-bold uppercase tracking-wide text-base-content/60">{label}</h2>
                <p class="text-3xl font-bold tabular-nums">{value}</p>
                {link.map(|(href, text)| view! { <a class="link link-primary link-hover text-sm font-semibold" href=href>{text}</a> })}
                {note.map(|n| view! { <p class="text-sm text-base-content/60">{n}</p> })}
            </div>
        </div>
    }
}

/// Aviso de los productos de ejemplo de la tienda nueva, con el botón para
/// borrarlos.
#[component]
fn ExamplesNotice(slug: String, count: i64) -> impl IntoView {
    let remove = Action::new(move |_: &()| delete_examples(slug.clone()));
    let gone = move || matches!(remove.value().get(), Some(Ok(())));
    let failed = move || matches!(remove.value().get(), Some(Err(_)));
    view! {
        <Show when=move || !gone()>
            <div role="region" aria-label="Productos de ejemplo" class="flex flex-wrap items-center gap-3 rounded-box border border-amber-300 bg-amber-50 px-4 py-3 text-amber-900">
                <p class="grow text-sm">
                    <b>{format!("Tu tienda trae {count} productos de ejemplo.")}</b>
                    " Sirven para ver cómo queda el diseño: tus clientes no los ven y no se pueden comprar. Si editás uno y lo guardás, pasa a ser tuyo."
                </p>
                <button type="button" class="btn btn-sm" disabled=move || remove.pending().get()
                    on:click=move |_| { remove.dispatch(()); }>
                    {move || if remove.pending().get() { "Borrando…" } else { "Borrar los de ejemplo" }}
                </button>
                {move || failed().then(|| view! { <p class="w-full text-sm text-error">"No pudimos borrarlos. Probá de nuevo."</p> })}
            </div>
        </Show>
    }
}

#[component]
fn Overview(o: StoreOverview) -> impl IntoView {
    let (open, open_problem) = editor_opener();
    let base = format!("/tienda/{}", o.slug);
    let address = o
        .url
        .split_once("://")
        .map_or(o.url.clone(), |(_, a)| a.to_string());
    let publish_class = match o.publish {
        PublishState::Published => OK,
        PublishState::Changes | PublishState::Never => WARN,
    };
    let slug = o.slug.clone();
    let orders_href = format!("{base}/pedidos");
    let month_note = match o.month_orders {
        0 => "Sin ventas todavía".to_string(),
        1 => "1 pedido".to_string(),
        n => format!("{n} pedidos"),
    };

    // Lo que falta para vender: cada paso con su enlace.
    let steps: Vec<(bool, &'static str, String, Option<(String, &'static str)>)> = vec![
        (
            o.products > 0,
            "Cargá tus productos",
            match o.products {
                0 => "Con nombre, precio y fotos.".to_string(),
                1 => "1 producto".to_string(),
                n => format!("{n} productos"),
            },
            Some((format!("{base}/productos"), "Cargar")),
        ),
        (
            o.publish != PublishState::Never,
            "Publicá tu tienda",
            match o.publish {
                PublishState::Changes => "Tenés cambios guardados sin publicar.".to_string(),
                PublishState::Published => "Tus clientes ya la pueden ver.".to_string(),
                PublishState::Never => {
                    "Tocá \"Editar diseño\", cambiá lo que quieras y tocá Publicar.".to_string()
                }
            },
            None,
        ),
        (
            o.delivery_ready,
            "Elegí cómo entregás",
            "Retiro en el local, envío o los dos.".to_string(),
            Some((format!("{base}/ventas"), "Elegir")),
        ),
        (
            o.pay_ready,
            "Elegí cómo te pagan",
            if o.pay_ready {
                "Mercado Pago, transferencia o coordinar.".to_string()
            } else {
                "Sin una forma de cobro, tu tienda no puede vender.".to_string()
            },
            Some((format!("{base}/cobros"), "Elegir")),
        ),
    ];
    let done = steps.iter().filter(|s| s.0).count();
    let total = steps.len();

    view! {
        <header class="flex flex-wrap items-center gap-4">
            <div class="grow">
                <h1 class="text-3xl font-bold">{o.name.clone()}</h1>
                <p class="mt-1 flex flex-wrap items-center gap-2 text-sm text-base-content/70">
                    <span class="font-mono">{address}</span>
                    <span class=format!("badge badge-sm border-0 font-semibold {publish_class}")>{o.publish.label()}</span>
                </p>
            </div>
            <a class="btn btn-outline max-w-72" href=o.url.clone() target="_blank" rel="noopener">
                <ui::Icon class="size-[18px]" name="external"/><span class="truncate">{format!("Ver {}", o.name)}</span><span class="sr-only">" (se abre en otra pestaña)"</span>
            </a>
            <button type="button" class="btn btn-primary" disabled=move || open.pending().get()
                on:click=move |_| { open.dispatch(slug.clone()); }>
                <ui::Icon class="size-[18px]" name="pencil"/>"Editar diseño"
            </button>
        </header>
        {move || open_problem.get().map(|e| view! { <div role="alert" class="alert alert-error text-sm">{e}</div> })}
        {(o.examples > 0).then(|| view! { <ExamplesNotice slug=o.slug.clone() count=o.examples/> })}

        <section aria-label="Resumen" class="grid grid-cols-2 gap-4 lg:grid-cols-4">
            <Kpi label="Para preparar" value=o.to_prepare.to_string() link=(orders_href.clone(), "Ver pedidos")/>
            <Kpi label="Esperando pago" value=o.awaiting_payment.to_string() link=(orders_href.clone(), "Ver pedidos")/>
            <Kpi label="Vendido este mes" value=ui::format_price(o.month_cents).trim_end_matches(",00").to_string() note=month_note/>
            <Kpi label="Sin stock" value=o.out_of_stock.to_string() link=(format!("{base}/productos"), "Ver productos")/>
        </section>

        <div class="grid items-start gap-4 lg:grid-cols-[3fr_2fr]">
            <section class="card border border-base-300 bg-base-100" aria-labelledby="ultimos-pedidos">
                <div class="card-body gap-0 p-5">
                    <div class="mb-2 flex items-center">
                        <h2 id="ultimos-pedidos" class="grow text-lg font-semibold">"Últimos pedidos"</h2>
                        {(!o.recent.is_empty()).then(|| view! {
                            <a class="link link-primary link-hover text-sm font-semibold" href=orders_href.clone()>"Ver todos"</a>
                        })}
                    </div>
                    {if o.recent.is_empty() {
                        view! {
                            <p class="border-t border-base-300 pt-4 text-base-content/70">
                                "Todavía no hay pedidos. Cuando alguien compre en tu tienda, lo vas a ver acá."
                            </p>
                        }.into_any()
                    } else {
                        view! {
                            <ul>
                                {o.recent.into_iter().map(|r| view! {
                                    <li>
                                        <a href=orders_href.clone() class="flex items-center gap-3 border-t border-base-300 py-3 hover:bg-base-200/60">
                                            <span class="font-bold tabular-nums">"#"{r.number}</span>
                                            <span class="min-w-0 grow truncate">{r.customer_name.clone()}</span>
                                            <span class="text-sm text-base-content/60">{domain::order_time_ar(&r.created_at)}</span>
                                            {status_badge(r.status)}
                                            <span class="w-28 text-right tabular-nums">{ui::format_price(r.total_cents)}</span>
                                        </a>
                                    </li>
                                }).collect_view()}
                            </ul>
                        }.into_any()
                    }}
                </div>
            </section>

            <section class="card border border-base-300 bg-base-100" aria-labelledby="lista-para-vender">
                <div class="card-body gap-0 p-5">
                    <div class="mb-2 flex items-center">
                        <h2 id="lista-para-vender" class="grow text-lg font-semibold">"Tu tienda, lista para vender"</h2>
                        <span class="text-sm text-base-content/60">{format!("{done} de {total}")}</span>
                    </div>
                    <ul>
                        {steps.into_iter().map(|(ok, title, detail, action)| view! {
                            <li class="flex items-start gap-3 border-t border-base-300 py-3">
                                <span class=if ok {
                                    "mt-0.5 grid size-6 shrink-0 place-items-center rounded-full bg-success text-success-content"
                                } else {
                                    "mt-0.5 size-6 shrink-0 rounded-full border-2 border-base-300"
                                }>
                                    {ok.then(|| view! { <ui::Icon class="size-[18px]" name="check"/> })}
                                </span>
                                <span class="grow">
                                    <span class="block">{title}<span class="sr-only">{if ok { " (listo)" } else { " (pendiente)" }}</span></span>
                                    <span class="block text-sm text-base-content/60">{detail}</span>
                                </span>
                                {(!ok).then(|| action.map(|(href, text)| view! {
                                    <a class="btn btn-outline btn-sm" href=href>{text}</a>
                                }))}
                            </li>
                        }).collect_view()}
                    </ul>
                </div>
            </section>
        </div>
    }
}
