//! Productos de una tienda, en la plataforma: la lista con búsqueda y filtro
//! por etiqueta (`/tienda/:slug/productos`) y la pantalla para cargar o
//! editar uno (`/tienda/:slug/productos/:id`, `nuevo` para uno nuevo).

use domain::{Product, ProductEdit, ProductSave, SaveResult, StoreProducts};
use leptos::prelude::*;
use leptos_router::hooks::use_params_map;

#[cfg(feature = "ssr")]
pub(crate) mod server {
    use crate::server::{internal, pool};
    use leptos::prelude::ServerFnError;

    /// La tienda `slug`, si quien entró es su dueño. `Ok(None)` si no entró.
    pub async fn owned_store(slug: &str) -> Result<Option<db::Tenant>, ServerFnError> {
        let Some(user) = crate::auth::current_user().await? else {
            return Ok(None);
        };
        let pool = pool()?;
        let tenant = db::get_tenant_by_slug(&pool, slug)
            .await
            .map_err(internal)?
            .ok_or_else(|| ServerFnError::new("Esa tienda no existe."))?;
        match db::auth::role_in(&pool, tenant.id, user.id)
            .await
            .map_err(internal)?
        {
            Some(_) => Ok(Some(tenant)),
            None => Err(ServerFnError::new("Tu cuenta no puede editar esta tienda.")),
        }
    }

    pub async fn require_store(slug: &str) -> Result<db::Tenant, ServerFnError> {
        owned_store(slug)
            .await?
            .ok_or_else(|| ServerFnError::new("Tenés que entrar con tu cuenta."))
    }

    pub fn parse_id(id: &str) -> Result<uuid::Uuid, ServerFnError> {
        uuid::Uuid::parse_str(id).map_err(|_| ServerFnError::new("Ese producto no existe."))
    }
}

/// Todos los productos de la tienda `slug` y sus etiquetas. `None` si no
/// entró con su cuenta.
#[server]
pub async fn list_store_products(slug: String) -> Result<Option<StoreProducts>, ServerFnError> {
    use crate::server::{internal, pool};
    let Some(tenant) = server::owned_store(&slug).await? else {
        return Ok(None);
    };
    let pool = pool()?;
    Ok(Some(StoreProducts {
        store_name: tenant.name,
        products: db::products::list_all(&pool, tenant.id)
            .await
            .map_err(internal)?,
        tags: db::products::tags(&pool, tenant.id)
            .await
            .map_err(internal)?,
    }))
}

/// El producto `id` para editarlo (o uno vacío si `id` es `nuevo`).
#[server]
pub async fn product_to_edit(
    slug: String,
    id: String,
) -> Result<Option<ProductEdit>, ServerFnError> {
    use crate::server::{internal, pool};
    let Some(tenant) = server::owned_store(&slug).await? else {
        return Ok(None);
    };
    let pool = pool()?;
    let product = if id == "nuevo" {
        // Arranca oculto, así se puede preparar sin que lo vean.
        Product {
            visible: false,
            ..Product::default()
        }
    } else {
        db::products::get(&pool, tenant.id, server::parse_id(&id)?)
            .await
            .map_err(internal)?
            .ok_or_else(|| ServerFnError::new("Ese producto ya no está en la tienda."))?
    };
    Ok(Some(ProductEdit {
        store_name: tenant.name,
        product,
        store_tags: db::products::tag_counts(&pool, tenant.id)
            .await
            .map_err(internal)?,
    }))
}

/// Guarda el producto: lo crea si `id` está vacío.
#[server]
pub async fn save_product(
    slug: String,
    id: String,
    product: Product,
) -> Result<SaveResult, ServerFnError> {
    use crate::server::{internal, pool};
    let tenant = server::require_store(&slug).await?;
    let pool = pool()?;
    let mut product = product;
    let errors = domain::tidy_product(&mut product);
    if !errors.is_empty() {
        return Ok(SaveResult {
            id,
            outcome: ProductSave::Invalid { errors },
        });
    }
    if id.is_empty() {
        let count = db::products::count(&pool, tenant.id)
            .await
            .map_err(internal)?;
        if count >= db::products::MAX_PER_TENANT {
            return Ok(SaveResult {
                id,
                outcome: ProductSave::Invalid {
                    errors: vec![format!(
                        "La tienda ya tiene {count} productos, que es el máximo por ahora."
                    )],
                },
            });
        }
        let new = db::products::create(&pool, tenant.id, &product)
            .await
            .map_err(internal)?;
        return Ok(SaveResult {
            id: new.to_string(),
            outcome: ProductSave::Saved,
        });
    }
    if !db::products::save(&pool, tenant.id, server::parse_id(&id)?, &product)
        .await
        .map_err(internal)?
    {
        return Err(ServerFnError::new("Ese producto ya no está en la tienda."));
    }
    Ok(SaveResult {
        id,
        outcome: ProductSave::Saved,
    })
}

/// Muestra u oculta el producto en la tienda.
#[server]
pub async fn set_product_visible(
    slug: String,
    id: String,
    visible: bool,
) -> Result<(), ServerFnError> {
    use crate::server::{internal, pool};
    let tenant = server::require_store(&slug).await?;
    if !db::products::set_visible(&pool()?, tenant.id, server::parse_id(&id)?, visible)
        .await
        .map_err(internal)?
    {
        return Err(ServerFnError::new("Ese producto ya no está en la tienda."));
    }
    Ok(())
}

#[server]
pub async fn delete_product(slug: String, id: String) -> Result<(), ServerFnError> {
    use crate::server::{internal, pool};
    let tenant = server::require_store(&slug).await?;
    db::products::delete(&pool()?, tenant.id, server::parse_id(&id)?)
        .await
        .map_err(internal)?;
    Ok(())
}

pub(crate) fn error_text(e: &ServerFnError) -> String {
    match e {
        ServerFnError::ServerError(m) => m.clone(),
        _ => "No pudimos conectarnos con el servidor. Revisá tu conexión y probá de nuevo.".into(),
    }
}

/// Encabezado de las pantallas de la plataforma.
#[component]
pub(crate) fn Shell(children: Children) -> impl IntoView {
    view! {
        <div data-theme="light" class="min-h-screen bg-base-200 text-base-content">
            <header class="flex items-center gap-3 border-b border-base-300 bg-base-100 px-6 py-3">
                <a href="/" class="text-lg font-bold">"MarketMaker"</a>
            </header>
            {children()}
        </div>
    }
}

#[component]
pub(crate) fn SignInFirst(back: String) -> impl IntoView {
    view! {
        <div class="card bg-base-100 shadow-sm"><div class="card-body items-start">
            <p>"Entrá con tu cuenta para ver tu tienda."</p>
            <a class="btn btn-primary" href=format!("/entrar?volver={back}")>"Entrar"</a>
        </div></div>
    }
}

fn photo_count(n: usize) -> String {
    match n {
        0 => "Sin fotos".into(),
        1 => "1 foto".into(),
        n => format!("{n} fotos"),
    }
}

pub(crate) fn slug_param() -> Memo<String> {
    let params = use_params_map();
    Memo::new(move |_| params.read().get("slug").unwrap_or_default())
}

/// `/tienda/:slug/productos`.
#[component]
pub fn ProductsPage() -> impl IntoView {
    if matches!(crate::site::current_site(), domain::Site::Store(_)) {
        return view! { <crate::Home/> }.into_any();
    }
    let slug = slug_param();
    let data = Resource::new(move || slug.get(), list_store_products);
    view! {
        <leptos_meta::Title text="Productos · MarketMaker"/>
        <Shell>
            <div class="mx-auto flex max-w-5xl flex-col gap-5 px-4 py-8">
                <Suspense fallback=|| view! { <ui::Skeleton class="h-96 w-full"/> }>
                    {move || Suspend::new(async move {
                        match data.await {
                            Ok(Some(list)) => view! { <ProductList slug=slug.get_untracked() data=list/> }.into_any(),
                            Ok(None) => view! { <SignInFirst back=format!("/tienda/{}/productos", slug.get_untracked())/> }.into_any(),
                            Err(e) => view! { <div role="alert" class="alert alert-error">{error_text(&e)}</div> }.into_any(),
                        }
                    })}
                </Suspense>
            </div>
        </Shell>
    }
    .into_any()
}

#[component]
fn ProductList(slug: String, data: StoreProducts) -> impl IntoView {
    let products = RwSignal::new(data.products);
    let tags = StoredValue::new(data.tags);
    let query = RwSignal::new(String::new());
    let tag = RwSignal::new(None::<String>);
    let problem = RwSignal::new(None::<String>);
    let base = format!("/tienda/{slug}/productos");
    let slug = StoredValue::new(slug);

    let summary = move || {
        products.with(|ps| {
            let visible = ps.iter().filter(|p| p.visible).count();
            match ps.len() {
                1 => format!(
                    "1 producto, {}",
                    if visible == 1 { "visible" } else { "oculto" }
                ),
                n => format!("{n} productos, {visible} visibles"),
            }
        })
    };
    let shown = Memo::new(move |_| {
        let q = query.get();
        let t = tag.get();
        products.with(|ps| {
            ps.iter()
                .filter(|p| domain::product_matches(p, &q))
                .filter(|p| t.as_ref().is_none_or(|t| p.tags.contains(t)))
                .cloned()
                .collect::<Vec<_>>()
        })
    });
    let toggle = Action::new(move |(id, on): &(String, bool)| {
        let (id, on) = (id.clone(), *on);
        async move {
            let r = set_product_visible(slug.get_value(), id.clone(), on).await;
            match r {
                Ok(()) => {
                    problem.set(None);
                    products.update(|ps| {
                        if let Some(p) = ps.iter_mut().find(|p| p.id == id) {
                            p.visible = on;
                        }
                    });
                }
                Err(e) => problem.set(Some(error_text(&e))),
            }
        }
    });

    let new_href = format!("{base}/nuevo");
    let empty = products.with_untracked(Vec::is_empty);
    view! {
        <div class="flex flex-wrap items-center gap-3">
            <a href="/" class="btn btn-ghost btn-sm">"← Tus tiendas"</a>
            <div class="grow">
                <h1 class="text-3xl font-bold">"Productos"</h1>
                <p class="text-sm text-base-content/70">{data.store_name} " · " {summary}</p>
            </div>
            <a class="btn btn-ghost" href="/mis-fotos">"Mis fotos"</a>
            <a class="btn btn-primary" href=new_href.clone()>"+ Nuevo producto"</a>
        </div>
        {move || problem.get().map(|e| view! { <div role="alert" class="alert alert-error text-sm">{e}</div> })}
        {if empty {
            view! {
                <div class="card bg-base-100 shadow-sm"><div class="card-body items-start">
                    <h2 class="card-title">"Todavía no cargaste productos"</h2>
                    <p>"Cargá el primero con su nombre, precio y fotos. Las grillas de tu tienda lo muestran solas."</p>
                    <a class="btn btn-primary" href=new_href>"+ Nuevo producto"</a>
                </div></div>
            }.into_any()
        } else {
            view! {
                <div class="flex flex-col gap-3">
                    <label class="input w-full max-w-md">
                        <span class="text-base-content/50" aria-hidden="true">"⌕"</span>
                        <input type="search" placeholder="Buscar por nombre, descripción o etiqueta"
                            aria-label="Buscar productos"
                            prop:value=move || query.get()
                            on:input=move |ev| query.set(event_target_value(&ev))/>
                    </label>
                    {(!tags.with_value(Vec::is_empty)).then(|| view! {
                        <div class="flex flex-wrap items-center gap-2" role="group" aria-label="Filtrar por etiqueta">
                            <span class="text-sm text-base-content/70">"Etiquetas:"</span>
                            <button type="button" class="badge badge-lg cursor-pointer"
                                class:badge-primary=move || tag.get().is_none()
                                class:badge-outline=move || tag.get().is_some()
                                aria-pressed=move || tag.get().is_none().to_string()
                                on:click=move |_| tag.set(None)>"Todas"</button>
                            {tags.get_value().into_iter().map(|t| {
                                let t2 = t.clone();
                                let t3 = t.clone();
                                let on = Memo::new(move |_| tag.get().as_deref() == Some(t2.as_str()));
                                view! {
                                    <button type="button" class="badge badge-lg cursor-pointer"
                                        class:badge-primary=move || on.get()
                                        class:badge-outline=move || !on.get()
                                        aria-pressed=move || on.get().to_string()
                                        on:click=move |_| tag.set((!on.get_untracked()).then(|| t3.clone()))>
                                        {t}
                                    </button>
                                }
                            }).collect_view()}
                        </div>
                    })}
                </div>
                <div class="card overflow-x-auto bg-base-100 shadow-sm">
                    <table class="table">
                        <thead><tr><th>"Producto"</th><th>"Precio"</th><th>"Stock"</th><th>"En la tienda"</th><th><span class="sr-only">"Acciones"</span></th></tr></thead>
                        <tbody>
                            <For each=move || shown.get() key=|p| (p.id.clone(), p.visible) let:p>
                                {
                                    let href = format!("{base}/{}", p.id);
                                    let id = p.id.clone();
                                    let visible = p.visible;
                                    let label = format!("Mostrar {} en la tienda", p.name);
                                    view! {
                                        <tr class:opacity-60=!visible>
                                            <td>
                                                <a href=href.clone() class="flex items-center gap-3">
                                                    <div class="h-11 w-14 shrink-0 overflow-hidden rounded-field bg-base-200">
                                                        <ui::Image src=p.image_url.clone() alt=""/>
                                                    </div>
                                                    <div class="min-w-0">
                                                        <div class="font-medium">{p.name.clone()}</div>
                                                        <div class="text-xs text-base-content/60">
                                                            {if visible { photo_count(p.all_photos().len()) } else { "Oculto · no aparece en la tienda".into() }}
                                                        </div>
                                                        {(!p.tags.is_empty()).then(|| view! {
                                                            <div class="mt-1 flex flex-wrap gap-1">
                                                                {p.tags.iter().map(|t| view! { <span class="badge badge-ghost badge-sm">{t.clone()}</span> }).collect_view()}
                                                            </div>
                                                        })}
                                                    </div>
                                                </a>
                                            </td>
                                            <td>
                                                <div class="font-semibold">{ui::format_price(p.price_cents)}</div>
                                                {(p.compare_at_cents > p.price_cents).then(|| view! {
                                                    <s class="text-xs text-base-content/50">{ui::format_price(p.compare_at_cents)}</s>
                                                })}
                                            </td>
                                            <td>
                                                {if p.in_stock {
                                                    view! { <span class="badge badge-success badge-soft">"Hay stock"</span> }.into_any()
                                                } else {
                                                    view! { <span class="badge badge-warning badge-soft">"Agotado"</span> }.into_any()
                                                }}
                                            </td>
                                            <td>
                                                <input type="checkbox" class="toggle toggle-sm toggle-primary"
                                                    aria-label=label
                                                    prop:checked=visible
                                                    disabled=move || toggle.pending().get()
                                                    on:change=move |ev| { toggle.dispatch((id.clone(), event_target_checked(&ev))); }/>
                                            </td>
                                            <td><a class="btn btn-ghost btn-sm" href=href>"Editar"</a></td>
                                        </tr>
                                    }
                                }
                            </For>
                        </tbody>
                    </table>
                    {move || shown.with(Vec::is_empty).then(|| view! {
                        <p class="p-6 text-center text-base-content/70">"Ningún producto coincide con la búsqueda."</p>
                    })}
                </div>
            }.into_any()
        }}
    }
}

/// `/tienda/:slug/productos/:id`.
#[component]
pub fn ProductEditPage() -> impl IntoView {
    if matches!(crate::site::current_site(), domain::Site::Store(_)) {
        return view! { <crate::Home/> }.into_any();
    }
    let slug = slug_param();
    let params = use_params_map();
    let id = Memo::new(move |_| params.read().get("id").unwrap_or_default());
    let data = Resource::new(
        move || (slug.get(), id.get()),
        |(s, i)| product_to_edit(s, i),
    );
    view! {
        <leptos_meta::Title text="Producto · MarketMaker"/>
        <Shell>
            <div class="mx-auto flex max-w-6xl flex-col gap-5 px-4 py-8">
                <Suspense fallback=|| view! { <ui::Skeleton class="h-96 w-full"/> }>
                    {move || Suspend::new(async move {
                        let back = format!("/tienda/{}/productos", slug.get_untracked());
                        match data.await {
                            Ok(Some(edit)) => {
                                let id = id.get_untracked();
                                let id = if id == "nuevo" { String::new() } else { id };
                                view! { <ProductForm slug=slug.get_untracked() id edit/> }.into_any()
                            }
                            Ok(None) => view! { <SignInFirst back/> }.into_any(),
                            Err(e) => view! {
                                <a href=back class="btn btn-ghost btn-sm self-start">"← Productos"</a>
                                <div role="alert" class="alert alert-error">{error_text(&e)}</div>
                            }.into_any(),
                        }
                    })}
                </Suspense>
            </div>
        </Shell>
    }
    .into_any()
}

/// Lo que se está escribiendo en el formulario.
#[derive(Clone, Copy)]
struct Form {
    name: RwSignal<String>,
    price: RwSignal<String>,
    was: RwSignal<String>,
    description: RwSignal<String>,
    photos: RwSignal<Vec<String>>,
    tags: RwSignal<Vec<String>>,
    in_stock: RwSignal<bool>,
    /// Cantidad disponible; vacío si no se cuenta.
    stock: RwSignal<String>,
    visible: RwSignal<bool>,
}

impl Form {
    fn new(p: &Product) -> Self {
        let money = |c: i64| {
            if c > 0 {
                domain::price_input(c)
            } else {
                String::new()
            }
        };
        Self {
            name: RwSignal::new(p.name.clone()),
            price: RwSignal::new(if p.id.is_empty() && p.price_cents == 0 {
                String::new()
            } else {
                domain::price_input(p.price_cents)
            }),
            was: RwSignal::new(money(p.compare_at_cents)),
            description: RwSignal::new(p.description.clone()),
            photos: RwSignal::new(p.all_photos()),
            tags: RwSignal::new(p.tags.clone()),
            in_stock: RwSignal::new(p.in_stock || p.stock.is_some()),
            stock: RwSignal::new(p.stock.map(|s| s.to_string()).unwrap_or_default()),
            visible: RwSignal::new(p.visible),
        }
    }

    /// El producto tal como quedaría, o los problemas con los precios.
    fn product(&self, id: &str) -> Result<Product, Vec<String>> {
        let mut errors = Vec::new();
        let price = self.price.get();
        let price_cents = if price.trim().is_empty() {
            errors.push("Poné el precio.".to_string());
            0
        } else {
            domain::parse_price(&price).unwrap_or_else(|| {
                errors.push(
                    "El precio tiene que ser un número, como 45.999 o 45.999,90.".to_string(),
                );
                0
            })
        };
        let was = self.was.get();
        let compare_at_cents = if was.trim().is_empty() {
            0
        } else {
            domain::parse_price(&was).unwrap_or_else(|| {
                errors.push(
                    "El precio anterior tiene que ser un número, o quedar vacío.".to_string(),
                );
                0
            })
        };
        let stock_text = self.stock.get();
        let stock = if stock_text.trim().is_empty() {
            None
        } else {
            match stock_text.trim().replace('.', "").parse::<u32>() {
                Ok(n) => Some(n),
                Err(_) => {
                    errors.push("La cantidad tiene que ser un número entero, como 5, o quedar vacía.".to_string());
                    None
                }
            }
        };
        let photos = self.photos.get();
        let p = Product {
            id: id.to_string(),
            name: self.name.get(),
            price_cents,
            image_url: photos.first().cloned().unwrap_or_default(),
            description: self.description.get(),
            compare_at_cents,
            in_stock: self.in_stock.get() || stock.is_some(),
            stock,
            visible: self.visible.get(),
            photos,
            tags: self.tags.get(),
        };
        if errors.is_empty() {
            Ok(p)
        } else {
            Err(errors)
        }
    }

    fn add_tags(&self, text: &str) {
        let new = domain::clean_tags(text.split(','));
        self.tags.update(|t| {
            for tag in new {
                if !t.contains(&tag) {
                    t.push(tag);
                }
            }
        });
    }
}

#[component]
fn ProductForm(slug: String, id: String, edit: ProductEdit) -> impl IntoView {
    let form = Form::new(&edit.product);
    let id = RwSignal::new(id);
    let list_href = format!("/tienda/{slug}/productos");
    let slug = StoredValue::new(slug);
    let store_tags = StoredValue::new(edit.store_tags);
    let errors = RwSignal::new(Vec::<String>::new());
    let saved = RwSignal::new(false);
    let dirty = RwSignal::new(false);
    // Cualquier cambio en el formulario saca el "Guardado".
    Effect::new(move |first: Option<()>| {
        form.name.track();
        form.price.track();
        form.was.track();
        form.description.track();
        form.photos.track();
        form.tags.track();
        form.in_stock.track();
        form.stock.track();
        form.visible.track();
        if first.is_some() {
            dirty.set(true);
            saved.set(false);
            errors.set(Vec::new());
        }
    });

    let save = Action::new(move |p: &Product| {
        let p = p.clone();
        async move { save_product(slug.get_value(), id.get_untracked(), p).await }
    });
    Effect::new(move |_| match save.value().get() {
        Some(Ok(SaveResult {
            id: new_id,
            outcome: ProductSave::Saved,
        })) => {
            errors.set(Vec::new());
            dirty.set(false);
            saved.set(true);
            if id.get_untracked().is_empty() {
                id.set(new_id.clone());
                // Cambia la dirección sin volver a cargar el formulario.
                if let Ok(history) = window().history() {
                    let url = format!("/tienda/{}/productos/{new_id}", slug.get_value());
                    _ = history.replace_state_with_url(
                        &leptos::wasm_bindgen::JsValue::NULL,
                        "",
                        Some(&url),
                    );
                }
            }
        }
        Some(Ok(SaveResult {
            outcome: ProductSave::Invalid { errors: e },
            ..
        })) => errors.set(e),
        Some(Err(e)) => errors.set(vec![error_text(&e)]),
        None => {}
    });
    let on_save = move || match form.product(&id.get_untracked()) {
        Ok(p) => {
            errors.set(Vec::new());
            save.dispatch(p);
        }
        Err(e) => errors.set(e),
    };

    let confirm_delete = RwSignal::new(false);
    let delete = Action::new(move |_: &()| async move {
        delete_product(slug.get_value(), id.get_untracked()).await
    });
    let list_href2 = list_href.clone();
    Effect::new(move |_| match delete.value().get() {
        Some(Ok(())) => {
            let _ = window().location().set_href(&list_href2);
        }
        Some(Err(e)) => errors.set(vec![error_text(&e)]),
        None => {}
    });

    let title = move || {
        let n = form.name.get();
        if !n.trim().is_empty() {
            n
        } else if id.get().is_empty() {
            "Nuevo producto".into()
        } else {
            "Producto sin nombre".into()
        }
    };
    let preview = move || {
        form.product(&id.get()).ok().map(|mut p| {
            _ = domain::tidy_product(&mut p);
            p
        })
    };

    view! {
        <div class="flex flex-wrap items-center gap-3">
            <a class="btn btn-ghost btn-sm" href=list_href>"← Productos"</a>
            <h1 class="grow truncate text-3xl font-bold">{title}</h1>
            <span class="text-sm text-base-content/60" role="status" aria-live="polite">
                {move || if save.pending().get() { "Guardando…" } else if saved.get() && !dirty.get() { "Guardado ✓" } else { "" }}
            </span>
            {move || (!id.get().is_empty()).then(|| view! {
                <button type="button" class="btn btn-ghost" on:click=move |_| confirm_delete.set(true)>"Borrar"</button>
            })}
            <button type="button" class="btn btn-primary" disabled=move || save.pending().get()
                on:click=move |_| on_save()>
                {move || if id.get().is_empty() { "Crear producto" } else { "Guardar" }}
            </button>
        </div>
        {move || confirm_delete.get().then(|| view! {
            <div role="alert" class="alert alert-warning flex flex-wrap">
                <span class="grow">"¿Borrar este producto? Deja de verse en la tienda y no se puede deshacer. Sus fotos siguen en Mis fotos."</span>
                <button type="button" class="btn btn-sm" on:click=move |_| confirm_delete.set(false)>"No, dejarlo"</button>
                <button type="button" class="btn btn-sm btn-error" disabled=move || delete.pending().get()
                    on:click=move |_| { delete.dispatch(()); }>"Sí, borrar"</button>
            </div>
        })}
        {move || {
            let e = errors.get();
            (!e.is_empty()).then(|| view! {
                <ul role="alert" class="alert alert-error flex flex-col items-start text-sm">
                    {e.into_iter().map(|e| view! { <li>{e}</li> }).collect_view()}
                </ul>
            })
        }}
        <div class="grid items-start gap-6 lg:grid-cols-[1fr_340px]">
            <div class="card bg-base-100 shadow-sm"><div class="card-body gap-5">
                <fieldset class="fieldset">
                    <legend class="fieldset-legend">"Nombre"</legend>
                    <input class="input w-full" maxlength=domain::PRODUCT_NAME_MAX placeholder="Por ejemplo: Taza de cerámica"
                        prop:value=move || form.name.get()
                        on:input=move |ev| form.name.set(event_target_value(&ev))/>
                </fieldset>
                <div class="grid gap-4 sm:grid-cols-2">
                    <fieldset class="fieldset">
                        <legend class="fieldset-legend">"Precio"</legend>
                        <label class="input w-full"><span>"$"</span>
                            <input inputmode="decimal" placeholder="45.999,90" aria-label="Precio en pesos"
                                prop:value=move || form.price.get()
                                on:input=move |ev| form.price.set(event_target_value(&ev))/>
                        </label>
                    </fieldset>
                    <fieldset class="fieldset">
                        <legend class="fieldset-legend">"Precio anterior " <span class="font-normal text-base-content/60">"(opcional, se muestra tachado)"</span></legend>
                        <label class="input w-full"><span>"$"</span>
                            <input inputmode="decimal" placeholder="Vacío si no está en oferta" aria-label="Precio anterior en pesos"
                                prop:value=move || form.was.get()
                                on:input=move |ev| form.was.set(event_target_value(&ev))/>
                        </label>
                    </fieldset>
                </div>
                <PhotosField photos=form.photos/>
                <fieldset class="fieldset">
                    <legend class="fieldset-legend">"Descripción"</legend>
                    <textarea class="textarea h-28 w-full" maxlength=domain::PRODUCT_DESC_MAX
                        placeholder="Qué es, de qué está hecho, medidas, cómo se usa…"
                        prop:value=move || form.description.get()
                        on:input=move |ev| form.description.set(event_target_value(&ev))></textarea>
                    <p class="label">{move || format!("{} de {} letras", form.description.with(|d| d.chars().count()), domain::PRODUCT_DESC_MAX)}</p>
                </fieldset>
                <TagsField form store_tags=store_tags.get_value()/>
                <div class="grid gap-4 sm:grid-cols-2">
                    <fieldset class="fieldset">
                        <legend class="fieldset-legend">"Stock"</legend>
                        <label class="fieldset-label" for="p-stock">"Cantidad disponible " <span class="text-base-content/60">"(opcional)"</span></label>
                        <input id="p-stock" class="input w-40" inputmode="numeric" placeholder="Sin contar"
                            aria-describedby="p-stock-hint"
                            prop:value=move || form.stock.get()
                            on:input=move |ev| form.stock.set(event_target_value(&ev))/>
                        {move || form.stock.with(|s| s.trim().is_empty()).then(|| view! {
                            <div class="join mt-1" role="group" aria-label="Stock">
                                <button type="button" class="btn btn-sm join-item" class:btn-primary=move || form.in_stock.get()
                                    aria-pressed=move || form.in_stock.get().to_string()
                                    on:click=move |_| form.in_stock.set(true)>"Hay stock"</button>
                                <button type="button" class="btn btn-sm join-item" class:btn-primary=move || !form.in_stock.get()
                                    aria-pressed=move || (!form.in_stock.get()).to_string()
                                    on:click=move |_| form.in_stock.set(false)>"Sin stock"</button>
                            </div>
                        })}
                        <p id="p-stock-hint" class="label whitespace-normal">
                            {move || if form.stock.with(|s| s.trim().is_empty()) {
                                "Dejala vacía si no querés llevar la cuenta. Sin stock se ve \"Agotado\"."
                            } else {
                                "Cada venta descuenta. En 0 se ve \"Agotado\"; con 3 o menos, \"Quedan N\"."
                            }}
                        </p>
                    </fieldset>
                    <fieldset class="fieldset">
                        <legend class="fieldset-legend">"En la tienda"</legend>
                        <label class="flex items-center gap-3">
                            <input type="checkbox" class="toggle toggle-primary"
                                prop:checked=move || form.visible.get()
                                on:change=move |ev| form.visible.set(event_target_checked(&ev))/>
                            {move || if form.visible.get() { "Visible" } else { "Oculto" }}
                        </label>
                        <p class="label">"Apagalo para prepararlo sin que lo vean."</p>
                    </fieldset>
                </div>
            </div></div>
            <aside class="flex flex-col gap-3 lg:sticky lg:top-4">
                <span class="text-sm font-medium text-base-content/70">"Así se ve en tu tienda"</span>
                <div class="h-80">
                    {move || match preview() {
                        Some(p) => view! { <widgets::ProductCard product=p show_price=true/> }.into_any(),
                        None => view! {
                            <div class="flex h-full items-center justify-center rounded-box border border-dashed border-base-300 p-6 text-center text-sm text-base-content/60">
                                "Poné un precio para ver cómo queda."
                            </div>
                        }.into_any(),
                    }}
                </div>
                <p class="text-xs text-base-content/60">
                    {move || if form.visible.get() {
                        "Al guardar, se ve en las grillas de tu tienda sin tocar el editor."
                    } else {
                        "Está oculto: no aparece en la tienda hasta que lo hagas visible."
                    }}
                </p>
            </aside>
        </div>
    }
}

/// Las fotos del producto: elegir de Mis fotos o subir, ordenar y sacar.
#[component]
fn PhotosField(photos: RwSignal<Vec<String>>) -> impl IntoView {
    let library = editor::MyPhotos::new();
    let picking = RwSignal::new(false);
    let full = move || photos.with(|p| p.len() >= domain::PRODUCT_PHOTOS_MAX);
    let add = move |url: String| {
        photos.update(|p| {
            if !p.contains(&url) && p.len() < domain::PRODUCT_PHOTOS_MAX {
                p.push(url);
            }
        });
    };
    let shift = move |i: usize, to: usize| photos.update(|p| p.swap(i, to));
    view! {
        <fieldset class="fieldset">
            <legend class="fieldset-legend">"Fotos " <span class="font-normal text-base-content/60">"· la primera es la principal; usá las flechas para ordenarlas"</span></legend>
            <ul class="flex flex-wrap gap-3" aria-label="Fotos del producto">
                {move || {
                    let list = photos.get();
                    let last = list.len().saturating_sub(1);
                    list.into_iter().enumerate().map(|(i, url)| {
                        let n = i + 1;
                        view! {
                            <li class="group relative h-24 w-32 overflow-hidden rounded-box bg-base-200"
                                class:ring-2=i == 0 class:ring-primary=i == 0>
                                <ui::Image src=url alt=format!("Foto {n}")/>
                                {(i == 0).then(|| view! { <span class="badge badge-primary badge-sm absolute left-1 top-1">"Principal"</span> })}
                                <div class="absolute inset-x-1 bottom-1 flex justify-between gap-1">
                                    <button type="button" class="btn btn-xs btn-square" aria-label=format!("Mover la foto {n} a la izquierda")
                                        disabled=i == 0 on:click=move |_| shift(i, i - 1)>"←"</button>
                                    <button type="button" class="btn btn-xs btn-square" aria-label=format!("Sacar la foto {n}")
                                        on:click=move |_| photos.update(|p| { p.remove(i); })>"✕"</button>
                                    <button type="button" class="btn btn-xs btn-square" aria-label=format!("Mover la foto {n} a la derecha")
                                        disabled=i == last on:click=move |_| shift(i, i + 1)>"→"</button>
                                </div>
                            </li>
                        }
                    }).collect_view()
                }}
                <li>
                    <button type="button" class="flex h-24 w-32 flex-col items-center justify-center gap-1 rounded-box border-2 border-dashed border-base-300 text-sm text-base-content/70 hover:border-primary"
                        aria-expanded=move || picking.get().to_string()
                        disabled=full
                        on:click=move |_| { library.ensure_loaded(); picking.update(|v| *v = !*v); }>
                        <span class="text-xl" aria-hidden="true">"+"</span>"Elegir de Mis fotos"
                    </button>
                </li>
                <li>
                    <label for="producto-subir" class="flex h-24 w-32 cursor-pointer flex-col items-center justify-center gap-1 rounded-box border-2 border-dashed border-base-300 text-sm text-base-content/70 hover:border-primary"
                        class:pointer-events-none=move || library.progress.get().is_some() || full()>
                        <span class="text-xl" aria-hidden="true">"⇪"</span>"Subir fotos"
                    </label>
                    <input id="producto-subir" type="file" class="sr-only" multiple
                        accept="image/jpeg,image/png,image/webp,image/gif"
                        on:change=move |ev| library.upload_each(editor::take_files(&ev), Callback::new(move |item: domain::MediaItem| add(item.url)))/>
                </li>
            </ul>
            <div role="status" aria-live="polite" class="flex flex-col gap-1 text-sm">
                {move || library.progress.get()}
                {move || library.saved.get().into_iter().map(|t| view! { <span>{t}</span> }).collect_view()}
            </div>
            {move || {
                let e = library.upload_errors.get();
                (!e.is_empty()).then(|| view! {
                    <ul role="alert" class="alert alert-error flex flex-col items-start text-sm">
                        {e.into_iter().map(|e| view! { <li>{e}</li> }).collect_view()}
                    </ul>
                })
            }}
            {move || full().then(|| view! { <p class="label">{format!("Llegaste al máximo de {} fotos.", domain::PRODUCT_PHOTOS_MAX)}</p> })}
            {move || picking.get().then(|| view! {
                <div class="mt-2 rounded-box border border-base-300 p-3">
                    <div class="mb-2 flex items-center gap-2">
                        <span class="grow text-sm font-medium">"Tocá una foto para sumarla"</span>
                        <button type="button" class="btn btn-ghost btn-xs" on:click=move |_| picking.set(false)>"Listo"</button>
                    </div>
                    <div class="mb-3 flex flex-wrap items-center gap-2">
                        <span class="text-xs text-base-content/70">"¿Está en otro sitio?"</span>
                        <editor::LinkImport photos=library done=Callback::new(|_| ()) each=Callback::new(move |item: domain::MediaItem| add(item.url)) uid="producto" small=true/>
                    </div>
                    {move || library.load_error.get().map(|e| view! { <p role="alert" class="text-sm text-error">{e}</p> })}
                    {move || match library.items.get() {
                        None => view! { <ui::Skeleton class="h-24 w-full"/> }.into_any(),
                        Some(items) if items.is_empty() => view! {
                            <p class="text-sm text-base-content/70">"Todavía no tenés fotos. Usá \"Subir fotos\"."</p>
                        }.into_any(),
                        Some(items) => view! {
                            <div class="grid max-h-72 grid-cols-4 gap-2 overflow-y-auto sm:grid-cols-6">
                                {items.into_iter().map(|item| {
                                    let url = item.url.clone();
                                    let url2 = item.url.clone();
                                    let chosen = Memo::new(move |_| photos.with(|p| p.contains(&url2)));
                                    view! {
                                        <button type="button" class="relative aspect-square overflow-hidden rounded-field bg-base-200"
                                            class:ring-2=move || chosen.get() class:ring-primary=move || chosen.get()
                                            aria-pressed=move || chosen.get().to_string()
                                            aria-label=item.name.clone()
                                            on:click=move |_| {
                                                if chosen.get_untracked() {
                                                    photos.update(|p| p.retain(|u| u != &url));
                                                } else {
                                                    add(url.clone());
                                                }
                                            }>
                                            <ui::Image src=item.url.clone() alt=""/>
                                            {move || chosen.get().then(|| view! { <span class="badge badge-primary badge-sm absolute right-1 top-1">"✓"</span> })}
                                        </button>
                                    }
                                }).collect_view()}
                            </div>
                        }.into_any(),
                    }}
                </div>
            })}
        </fieldset>
    }
}

/// Las etiquetas: un campo con las elegidas y, al tocarlo o escribir, un
/// menú con las que ya usa la tienda (y en cuántos productos) para no
/// escribirlas distinto, más la opción de crear una nueva.
#[component]
fn TagsField(form: Form, store_tags: Vec<domain::TagCount>) -> impl IntoView {
    use domain::TagOption;
    let text = RwSignal::new(String::new());
    let open = RwSignal::new(false);
    let active = RwSignal::new(0usize);
    let store_tags = StoredValue::new(store_tags);
    let options = Memo::new(move |_| {
        let typed = text.get();
        form.tags
            .with(|mine| store_tags.with_value(|all| domain::tag_options(all, mine, &typed)))
    });
    let pick = move |opt: &TagOption| {
        let tag = match opt {
            TagOption::Existing(t) => t.tag.clone(),
            TagOption::Create(t) => t.clone(),
        };
        form.add_tags(&tag);
        text.set(String::new());
        active.set(0);
    };
    let commit = move || {
        let t = text.get_untracked();
        if !t.trim().is_empty() {
            form.add_tags(&t);
            text.set(String::new());
        }
    };
    let input_ref = NodeRef::<leptos::html::Input>::new();
    view! {
        <fieldset class="fieldset">
            <legend class="fieldset-legend">"Etiquetas " <span class="font-normal text-base-content/60">"· para encontrar y agrupar productos (ej.: regalo, cocina, verano)"</span></legend>
            <div class="relative">
                <div class="flex min-h-10 w-full cursor-text flex-wrap items-center gap-1.5 rounded-field border border-base-content/20 bg-base-100 px-3 py-1.5 focus-within:border-primary focus-within:outline-2 focus-within:outline-offset-1 focus-within:outline-primary"
                    on:click=move |_| { if let Some(i) = input_ref.get() { _ = i.focus(); } }>
                    {move || form.tags.get().into_iter().map(|t| {
                        let t2 = t.clone();
                        view! {
                            <span class="inline-flex items-center gap-1 rounded-full bg-primary/10 py-0.5 pl-2.5 pr-1 text-sm text-primary">
                                {t.clone()}
                                <button type="button" class="cursor-pointer rounded-full px-1 opacity-60 hover:opacity-100" aria-label=format!("Sacar la etiqueta {t}")
                                    on:click=move |ev| { ev.stop_propagation(); form.tags.update(|l| l.retain(|x| x != &t2)); }>"✕"</button>
                            </span>
                        }
                    }).collect_view()}
                    <input node_ref=input_ref class="min-w-32 grow bg-transparent py-0.5 outline-none"
                        placeholder=move || if form.tags.with(Vec::is_empty) { "Tocá para elegir o escribí una nueva" } else { "" }
                        role="combobox" aria-label="Agregar etiqueta" aria-autocomplete="list"
                        aria-controls="etiquetas-menu"
                        aria-expanded=move || (open.get() && !options.with(Vec::is_empty)).to_string()
                        aria-activedescendant=move || (open.get() && !options.with(Vec::is_empty)).then(|| format!("etiqueta-op-{}", active.get()))
                        maxlength=domain::TAG_MAX
                        prop:value=move || text.get()
                        on:focus=move |_| open.set(true)
                        on:input=move |ev| {
                            let v = event_target_value(&ev);
                            open.set(true);
                            active.set(0);
                            if v.contains(',') {
                                form.add_tags(&v);
                                text.set(String::new());
                            } else {
                                text.set(v);
                            }
                        }
                        on:keydown=move |ev| {
                            let n = options.with_untracked(Vec::len);
                            match ev.key().as_str() {
                                "ArrowDown" if n > 0 => {
                                    ev.prevent_default();
                                    open.set(true);
                                    active.update(|a| *a = (*a + 1) % n);
                                }
                                "ArrowUp" if n > 0 => {
                                    ev.prevent_default();
                                    active.update(|a| *a = (*a + n - 1) % n);
                                }
                                "Enter" => {
                                    ev.prevent_default();
                                    let chosen = open.get_untracked().then(|| options.with_untracked(|o| o.get(active.get_untracked()).cloned())).flatten();
                                    match chosen {
                                        Some(opt) => pick(&opt),
                                        None => commit(),
                                    }
                                }
                                "Escape" => open.set(false),
                                "Backspace" if text.with_untracked(String::is_empty) => {
                                    form.tags.update(|l| { l.pop(); });
                                }
                                _ => {}
                            }
                        }
                        on:blur=move |_| { open.set(false); commit(); }/>
                </div>
                {move || (open.get() && !options.with(Vec::is_empty)).then(|| view! {
                    <ul id="etiquetas-menu" role="listbox" aria-label="Etiquetas"
                        class="absolute left-0 top-full z-20 mt-1.5 max-h-72 w-full max-w-sm overflow-y-auto rounded-box border border-base-300 bg-base-100 py-1 shadow-xl">
                        {options.get().into_iter().enumerate().map(|(i, opt)| {
                            let opt2 = opt.clone();
                            let row = match &opt {
                                TagOption::Existing(t) => view! {
                                    <span class="grow">{t.tag.clone()}</span>
                                    <span class="text-xs text-base-content/50">{if t.products == 1 { "1 producto".to_string() } else { format!("{} productos", t.products) }}</span>
                                }.into_any(),
                                TagOption::Create(t) => view! {
                                    <span class="grow font-medium text-primary">{format!("+ Crear la etiqueta «{t}»")}</span>
                                }.into_any(),
                            };
                            let is_create = matches!(opt, TagOption::Create(_));
                            view! {
                                <li id=format!("etiqueta-op-{i}") role="option"
                                    aria-selected=move || (active.get() == i).to_string()
                                    class="mx-1 flex cursor-pointer items-center gap-3 rounded-field px-3 py-2 text-sm"
                                    class:bg-base-200=move || active.get() == i
                                    class:border-t=is_create
                                    class:border-base-200=is_create
                                    on:mouseenter=move |_| active.set(i)
                                    // mousedown, antes de que el campo pierda el foco.
                                    on:mousedown=move |ev| { ev.prevent_default(); pick(&opt2); }>
                                    {row}
                                </li>
                            }
                        }).collect_view()}
                    </ul>
                })}
            </div>
        </fieldset>
    }
}
