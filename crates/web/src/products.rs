//! Productos de una tienda, en la plataforma: la lista con búsqueda y filtro
//! por etiqueta (`/tienda/:slug/productos`) y la pantalla para cargar o
//! editar uno (`/tienda/:slug/productos/:id`, `nuevo` para uno nuevo).

use domain::{PhotoFit, Product, ProductEdit, ProductSave, SaveResult, StoreProducts};
use leptos::prelude::*;
use leptos_router::hooks::use_params_map;
use std::collections::BTreeMap;

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

/// Guarda una etiqueta desde "Etiquetas": la crea (`old` vacío) o la
/// renombra, y queda puesta justo en los productos `ids`.
#[server]
pub async fn save_tag(
    slug: String,
    old: String,
    name: String,
    ids: Vec<String>,
) -> Result<domain::TagSave, ServerFnError> {
    use domain::TagSave;
    let name = match domain::check_tag_name(&name) {
        Ok(n) => n,
        Err(error) => return Ok(TagSave::Invalid { error }),
    };
    retag_products(&slug, |all| {
        if name != old && all.iter().any(|p| p.tags.contains(&name)) {
            return Err(format!(
                "Ya hay una etiqueta «{name}». Elegí otro nombre o editá esa."
            ));
        }
        Ok(all
            .iter()
            .map(|p| {
                let tags = domain::retag(&p.tags, &old, &name, ids.contains(&p.id));
                (p.id.clone(), tags)
            })
            .collect())
    })
    .await
}

/// Saca la etiqueta `tag` de todos los productos.
#[server]
pub async fn delete_tag(slug: String, tag: String) -> Result<domain::TagSave, ServerFnError> {
    retag_products(&slug, |all| {
        Ok(all
            .iter()
            .map(|p| (p.id.clone(), domain::retag(&p.tags, &tag, &tag, false)))
            .collect())
    })
    .await
}

/// Aplica `change` (las etiquetas nuevas de cada producto, o un problema)
/// a los productos de la tienda y devuelve cómo quedaron.
#[cfg(feature = "ssr")]
async fn retag_products(
    slug: &str,
    change: impl FnOnce(&[Product]) -> Result<Vec<(String, Vec<String>)>, String>,
) -> Result<domain::TagSave, ServerFnError> {
    use crate::server::{internal, pool};
    use domain::TagSave;
    let tenant = server::require_store(slug).await?;
    let pool = pool()?;
    let mut all = db::products::list_all(&pool, tenant.id)
        .await
        .map_err(internal)?;
    let new = match change(&all) {
        Ok(n) => n,
        Err(error) => return Ok(TagSave::Invalid { error }),
    };
    let mut changes = Vec::new();
    for (p, (id, tags)) in all.iter_mut().zip(new) {
        if p.tags != tags {
            changes.push((server::parse_id(&id)?, tags.clone()));
            p.tags = tags;
        }
    }
    db::products::set_tags(&pool, tenant.id, &changes)
        .await
        .map_err(internal)?;
    Ok(TagSave::Saved { products: all })
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
        // Arranca visible: lo normal es cargarlo para venderlo. Quien quiera
        // prepararlo antes apaga "Mostrar en la tienda".
        Product::default()
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

#[component]
pub(crate) fn SignInFirst(back: String) -> impl IntoView {
    view! {
        <div class="card bg-base-100 shadow-sm"><div class="card-body items-start">
            <p>"Entrá con tu cuenta para ver tu tienda."</p>
            <a class="btn btn-primary" href=format!("/entrar?volver={back}")>"Entrar"</a>
        </div></div>
    }
}

/// Si la pantalla es de teléfono (menos de 640 px, como `sm` de Tailwind).
fn is_phone() -> bool {
    window()
        .inner_width()
        .ok()
        .and_then(|w| w.as_f64())
        .is_some_and(|w| w < 640.0)
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
            <div class="mx-auto flex max-w-5xl flex-col gap-5 px-4 py-8 max-sm:py-5 max-sm:pb-24">
                <Suspense fallback=|| view! { <ui::PanelSkeleton kind=ui::PanelSkeletonKind::List/> }>
                    {move || Suspend::new(async move {
                        match data.await {
                            Ok(Some(list)) => view! { <ProductList slug=slug.get_untracked() data=list/> }.into_any(),
                            Ok(None) => view! { <SignInFirst back=format!("/tienda/{}/productos", slug.get_untracked())/> }.into_any(),
                            Err(e) => view! { <div role="alert" class="alert alert-error">{error_text(&e)}</div> }.into_any(),
                        }
                    })}
                </Suspense>
            </div>
    }
    .into_any()
}

#[component]
fn ProductList(slug: String, data: StoreProducts) -> impl IntoView {
    let products = RwSignal::new(data.products);
    // Las etiquetas salen de los productos, así se actualizan al cambiarlas
    // desde "Etiquetas".
    let tags = Memo::new(move |_| {
        products.with(|ps| {
            let mut t: Vec<String> = domain::count_tags(ps).into_iter().map(|c| c.tag).collect();
            t.sort();
            t
        })
    });
    let _ = data.tags;
    let manage = NodeRef::<leptos::html::Dialog>::new();
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
    let base_phone = base.clone();
    let empty = products.with_untracked(Vec::is_empty);
    view! {
        <div class="flex flex-wrap items-center gap-3">
            <div class="grow">
                <h1 class="text-3xl font-bold max-sm:text-2xl">"Productos"</h1>
                <p class="text-sm text-base-content/70">{data.store_name} " · " {summary}</p>
            </div>
            <button type="button" class="btn max-sm:min-h-11" on:click=move |_| {
                if let Some(d) = manage.get() { _ = d.show_modal(); }
            }>"Etiquetas"</button>
            // En el teléfono es el botón flotante de abajo (ver `.mk-fab`).
            <a class="btn btn-primary mk-fab" href=new_href.clone()>"+ Nuevo producto"</a>
        </div>
        <dialog node_ref=manage class="modal" aria-labelledby="etiquetas-titulo">
            <div class="modal-box max-w-3xl">
                <TagManager slug=slug.get_value() products tag dialog=manage/>
            </div>
            <form method="dialog" class="modal-backdrop"><button>"Cerrar"</button></form>
        </dialog>
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
                    <label class="input w-full max-w-md max-sm:h-12 max-sm:max-w-none">
                        <span class="text-base-content/50" aria-hidden="true">"⌕"</span>
                        <input type="search" placeholder="Buscar por nombre, descripción o etiqueta"
                            aria-label="Buscar productos"
                            prop:value=move || query.get()
                            on:input=move |ev| query.set(event_target_value(&ev))/>
                    </label>
                    {move || (!tags.with(Vec::is_empty)).then(|| view! {
                        <div class="flex flex-wrap items-center gap-2 max-sm:flex-nowrap max-sm:overflow-x-auto max-sm:pb-1 max-sm:[&>*]:shrink-0 max-sm:[&_.badge]:h-10 max-sm:[&_.badge]:px-4" role="group" aria-label="Filtrar por etiqueta">
                            <span class="text-sm text-base-content/70 max-sm:hidden">"Etiquetas:"</span>
                            <button type="button" class="badge badge-lg cursor-pointer"
                                class:badge-primary=move || tag.get().is_none()
                                class:badge-outline=move || tag.get().is_some()
                                aria-pressed=move || tag.get().is_none().to_string()
                                on:click=move |_| tag.set(None)>"Todas"</button>
                            {tags.get().into_iter().map(|t| {
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
                // Teléfono: una tarjeta por producto, con foto grande y el interruptor
                // para mostrarlo u ocultarlo sin abrirlo.
                <ul class="flex flex-col gap-2.5 sm:hidden" aria-label="Productos">
                    <For each=move || shown.get() key=|p| (p.id.clone(), p.visible) let:p>
                        {
                            let href = format!("{base_phone}/{}", p.id);
                            let id = p.id.clone();
                            let visible = p.visible;
                            let label = format!("Mostrar {} en la tienda", p.name);
                            view! {
                                <li class="flex items-center gap-2 rounded-box bg-base-100 p-2.5 shadow-sm" class=("opacity-60", !visible)>
                                    <a href=href class="flex min-w-0 grow items-center gap-3">
                                        <div class="size-16 shrink-0 overflow-hidden rounded-field bg-base-200">
                                            <ui::Image src=p.image_url.clone() alt="" fit=p.fit(&p.image_url)/>
                                        </div>
                                        <div class="min-w-0">
                                            <div class="truncate font-semibold">{p.name.clone()}</div>
                                            <div class="flex flex-wrap items-center gap-x-2 text-sm">
                                                <span class="font-semibold">{ui::format_price(p.price_cents)}</span>
                                                {(p.compare_at_cents > p.price_cents).then(|| view! {
                                                    <s class="text-xs text-base-content/50">{ui::format_price(p.compare_at_cents)}</s>
                                                })}
                                                {if p.in_stock {
                                                    view! { <span class="text-green-800">"· Hay stock"</span> }.into_any()
                                                } else {
                                                    view! { <span class="text-amber-800">"· Agotado"</span> }.into_any()
                                                }}
                                            </div>
                                            <div class="truncate text-xs text-base-content/60">
                                                {if p.example {
                                                    "De ejemplo · tus clientes no lo ven".to_string()
                                                } else if !visible {
                                                    "Oculto · no aparece en la tienda".to_string()
                                                } else {
                                                    photo_count(p.all_photos().len())
                                                }}
                                            </div>
                                        </div>
                                    </a>
                                    <label class="flex size-12 shrink-0 cursor-pointer items-center justify-center">
                                        <input type="checkbox" class="toggle toggle-primary"
                                            aria-label=label
                                            prop:checked=visible
                                            disabled=move || toggle.pending().get()
                                            on:change=move |ev| { toggle.dispatch((id.clone(), event_target_checked(&ev))); }/>
                                    </label>
                                </li>
                            }
                        }
                    </For>
                </ul>
                {move || shown.with(Vec::is_empty).then(|| view! {
                    <p class="p-6 text-center text-base-content/70 sm:hidden">"Ningún producto coincide con la búsqueda."</p>
                })}
                <div class="card overflow-x-auto bg-base-100 shadow-sm max-sm:hidden">
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
                                                        <ui::Image src=p.image_url.clone() alt="" fit=p.fit(&p.image_url)/>
                                                    </div>
                                                    <div class="min-w-0">
                                                        <div class="font-medium">
                                                            {p.name.clone()}
                                                            {p.example.then(|| view! { <span class="badge badge-sm ml-2 border-0 bg-amber-100 font-semibold text-amber-800">"Ejemplo"</span> })}
                                                        </div>
                                                        <div class="text-xs text-base-content/60">
                                                            {if p.example {
                                                                "De ejemplo · tus clientes no lo ven".to_string()
                                                            } else if visible {
                                                                photo_count(p.all_photos().len())
                                                            } else {
                                                                "Oculto · no aparece en la tienda".into()
                                                            }}
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
                                                    view! { <span class="badge badge-soft badge-success text-green-800">"Hay stock"</span> }.into_any()
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

/// "Etiquetas": crear una etiqueta y elegir sus productos, cambiarle el
/// nombre o borrarla. A la izquierda las etiquetas; a la derecha la elegida.
#[component]
fn TagManager(
    slug: String,
    products: RwSignal<Vec<Product>>,
    /// El filtro de la lista: se limpia si se borra o renombra su etiqueta.
    tag: RwSignal<Option<String>>,
    dialog: NodeRef<leptos::html::Dialog>,
) -> impl IntoView {
    let slug = StoredValue::new(slug);
    let counts = Memo::new(move |_| products.with(|ps| domain::count_tags(ps)));
    // La etiqueta que se está editando: `Some("")` es una nueva.
    let editing = RwSignal::new(None::<String>);
    let name = RwSignal::new(String::new());
    let picked = RwSignal::new(Vec::<String>::new());
    let search = RwSignal::new(String::new());
    let problem = RwSignal::new(None::<String>);
    let saved = RwSignal::new(false);
    let confirm = RwSignal::new(false);
    let name_ref = NodeRef::<leptos::html::Input>::new();

    let open = move |t: String| {
        picked.set(products.with_untracked(|ps| {
            ps.iter()
                .filter(|p| !t.is_empty() && p.tags.contains(&t))
                .map(|p| p.id.clone())
                .collect()
        }));
        name.set(t.clone());
        editing.set(Some(t));
        search.set(String::new());
        problem.set(None);
        saved.set(false);
        confirm.set(false);
        if let Some(i) = name_ref.get_untracked() {
            _ = i.focus();
        }
    };
    let done = move |r: Result<domain::TagSave, ServerFnError>, now: Option<String>| match r {
        Ok(domain::TagSave::Saved { products: ps }) => {
            let old = editing.get_untracked().unwrap_or_default();
            if !old.is_empty() && tag.get_untracked().as_deref() == Some(old.as_str()) {
                tag.set(None);
            }
            products.set(ps);
            problem.set(None);
            confirm.set(false);
            saved.set(now.is_some());
            match now {
                Some(n) => {
                    name.set(n.clone());
                    editing.set(Some(n));
                }
                None => editing.set(None),
            }
        }
        Ok(domain::TagSave::Invalid { error }) => problem.set(Some(error)),
        Err(e) => problem.set(Some(error_text(&e))),
    };
    let save = Action::new(move |_: &()| {
        let old = editing.get_untracked().unwrap_or_default();
        let n = name.get_untracked();
        let ids = picked.get_untracked();
        async move {
            let clean = domain::clean_tag(&n);
            let r = save_tag(slug.get_value(), old, n, ids).await;
            done(r, clean);
        }
    });
    let remove = Action::new(move |_: &()| {
        let t = editing.get_untracked().unwrap_or_default();
        async move {
            let r = delete_tag(slug.get_value(), t).await;
            done(r, None);
        }
    });
    let busy = move || save.pending().get() || remove.pending().get();
    let shown = Memo::new(move |_| {
        let q = search.get();
        products.with(|ps| {
            ps.iter()
                .filter(|p| domain::product_matches(p, &q))
                .map(|p| (p.id.clone(), p.name.clone(), p.image_url.clone()))
                .collect::<Vec<_>>()
        })
    });
    let close = move |_| {
        if let Some(d) = dialog.get() {
            d.close();
        }
    };

    view! {
        <div class="flex items-start gap-3">
            <div class="grow">
                <h2 id="etiquetas-titulo" class="text-xl font-bold">"Etiquetas"</h2>
                <p class="text-sm text-base-content/70">"Sirven para agrupar productos. Si sumás a tu tienda uno de los bloques de etiquetas (en el editor, en Productos), tus clientes las tocan para ver solo esos productos."</p>
            </div>
            <button type="button" class="btn btn-ghost btn-sm btn-square" aria-label="Cerrar" on:click=close>"✕"</button>
        </div>
        <div class="mt-4 grid min-h-[26rem] grid-cols-[14rem_1fr] gap-5">
            <div class="flex flex-col gap-1 border-r border-base-300 pr-4">
                <button type="button" class="btn btn-primary btn-sm mb-2 justify-start" on:click=move |_| open(String::new())>"+ Nueva etiqueta"</button>
                <ul class="flex flex-col gap-0.5 overflow-y-auto" aria-label="Etiquetas de la tienda">
                    {move || counts.get().into_iter().map(|c| {
                        let t = c.tag.clone();
                        let t2 = c.tag.clone();
                        let on = move || editing.get().as_deref() == Some(t.as_str());
                        view! {
                            <li>
                                <button type="button" class="flex w-full items-center justify-between gap-2 rounded-field px-3 py-2 text-left text-sm hover:bg-base-200"
                                    class:bg-primary=on.clone()
                                    class:text-primary-content=on.clone()
                                    class:hover:bg-primary=on.clone()
                                    aria-current=move || on().then_some("true")
                                    on:click=move |_| open(t2.clone())>
                                    <span class="truncate">{c.tag.clone()}</span>
                                    <span class="opacity-70">{c.products}</span>
                                </button>
                            </li>
                        }
                    }).collect_view()}
                </ul>
                {move || counts.with(Vec::is_empty).then(|| view! {
                    <p class="px-1 text-sm text-base-content/60">"Todavía no hay etiquetas."</p>
                })}
            </div>
            <div class="flex min-w-0 flex-col gap-3">
                {move || match editing.get() {
                    None => view! {
                        <div class="flex h-full flex-col items-center justify-center gap-2 text-center text-base-content/70">
                            <p>"Elegí una etiqueta de la izquierda para cambiarle el nombre o sus productos, o creá una nueva."</p>
                        </div>
                    }.into_any(),
                    Some(t) => {
                        let is_new = t.is_empty();
                        view! {
                            <form class="flex min-h-0 flex-1 flex-col gap-3" on:submit=move |ev| { ev.prevent_default(); save.dispatch(()); }>
                                <label class="flex flex-col gap-1">
                                    <span class="text-sm font-medium">{if is_new { "Nombre de la etiqueta nueva" } else { "Nombre" }}</span>
                                    <input node_ref=name_ref class="input w-full" maxlength="30" required=true
                                        placeholder="Por ejemplo: ofertas, verano, regalos"
                                        prop:value=move || name.get()
                                        on:input=move |ev| { name.set(event_target_value(&ev)); saved.set(false); }/>
                                </label>
                                <fieldset class="flex min-h-0 flex-1 flex-col gap-2">
                                    <legend class="mb-1 text-sm font-medium">
                                        "Productos con esta etiqueta "
                                        <span class="font-normal text-base-content/60">{move || format!("({} elegidos)", picked.with(Vec::len))}</span>
                                    </legend>
                                    <input type="search" class="input input-sm w-full" placeholder="Buscar producto" aria-label="Buscar producto"
                                        prop:value=move || search.get()
                                        on:input=move |ev| search.set(event_target_value(&ev))/>
                                    <ul class="max-h-64 min-h-0 overflow-y-auto rounded-field border border-base-300">
                                        <For each=move || shown.get() key=|p| p.0.clone() let:p>
                                            {
                                                let (id, pname, img) = p;
                                                let id2 = id.clone();
                                                let checked = move || picked.with(|l| l.contains(&id));
                                                view! {
                                                    <li class="border-b border-base-200 last:border-0">
                                                        <label class="flex cursor-pointer items-center gap-3 px-3 py-1.5 hover:bg-base-200">
                                                            <input type="checkbox" class="checkbox checkbox-sm checkbox-primary"
                                                                prop:checked=checked
                                                                on:change=move |ev| {
                                                                    let on = event_target_checked(&ev);
                                                                    saved.set(false);
                                                                    picked.update(|l| {
                                                                        l.retain(|x| x != &id2);
                                                                        if on { l.push(id2.clone()); }
                                                                    });
                                                                }/>
                                                            <span class="h-8 w-10 shrink-0 overflow-hidden rounded bg-base-200"><ui::Image src=img alt=""/></span>
                                                            <span class="truncate text-sm">{pname}</span>
                                                        </label>
                                                    </li>
                                                }
                                            }
                                        </For>
                                    </ul>
                                </fieldset>
                                {move || problem.get().map(|e| view! { <div role="alert" class="alert alert-error text-sm">{e}</div> })}
                                {move || if confirm.get() {
                                    let n = picked.with(Vec::len);
                                    view! {
                                        <div role="alert" class="alert alert-warning flex flex-wrap text-sm">
                                            <span class="grow">{format!("¿Borrar «{}»? Se va a sacar de {n} producto{}. Los productos no se borran.", editing.get_untracked().unwrap_or_default(), if n == 1 { "" } else { "s" })}</span>
                                            <button type="button" class="btn btn-sm" on:click=move |_| confirm.set(false)>"Cancelar"</button>
                                            <button type="button" class="btn btn-error btn-sm" disabled=busy on:click=move |_| { remove.dispatch(()); }>"Borrar etiqueta"</button>
                                        </div>
                                    }.into_any()
                                } else {
                                    view! {
                                        <div class="flex items-center gap-2">
                                            <button type="submit" class="btn btn-primary" disabled=busy>
                                                {if is_new { "Crear etiqueta" } else { "Guardar cambios" }}
                                            </button>
                                            {move || saved.get().then(|| view! { <span role="status" class="text-sm text-success">"✓ Guardado"</span> })}
                                            <span class="grow"></span>
                                            {(!is_new).then(|| view! {
                                                <button type="button" class="btn btn-ghost btn-sm text-error" on:click=move |_| confirm.set(true)>"Borrar etiqueta"</button>
                                            })}
                                        </div>
                                    }.into_any()
                                }}
                            </form>
                        }.into_any()
                    }
                }}
            </div>
        </div>
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
            <div class="mx-auto flex max-w-6xl flex-col gap-5 px-4 py-8 max-sm:py-4 max-sm:pb-28">
                <Suspense fallback=|| view! { <ui::PanelSkeleton kind=ui::PanelSkeletonKind::Form/> }>
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
    /// Encuadre de cada foto en este producto.
    fits: RwSignal<BTreeMap<String, PhotoFit>>,
    tags: RwSignal<Vec<String>>,
    in_stock: RwSignal<bool>,
    /// Cantidad disponible; vacío si no se cuenta.
    stock: RwSignal<String>,
    visible: RwSignal<bool>,
    /// Peso (kg) y medidas (cm) para el envío; todo vacío: el paquete de la tienda.
    weight: RwSignal<String>,
    length: RwSignal<String>,
    width: RwSignal<String>,
    height: RwSignal<String>,
    /// Colores y talles, con el stock de cada combinación.
    variants: crate::variants_field::VariantsForm,
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
            fits: RwSignal::new(p.photo_fits.clone()),
            tags: RwSignal::new(p.tags.clone()),
            in_stock: RwSignal::new(p.in_stock || p.stock.is_some()),
            stock: RwSignal::new(p.stock.map(|s| s.to_string()).unwrap_or_default()),
            visible: RwSignal::new(p.visible),
            weight: RwSignal::new(p.parcel.map(|x| domain::kg_input(x.weight_g)).unwrap_or_default()),
            length: RwSignal::new(p.parcel.map(|x| domain::cm_input(x.length_mm)).unwrap_or_default()),
            width: RwSignal::new(p.parcel.map(|x| domain::cm_input(x.width_mm)).unwrap_or_default()),
            height: RwSignal::new(p.parcel.map(|x| domain::cm_input(x.height_mm)).unwrap_or_default()),
            variants: crate::variants_field::VariantsForm::new(&p.variants),
        }
    }

    /// El producto tal como quedaría, o los problemas con los precios.
    fn product(&self, id: &str) -> Result<Product, Vec<String>> {
        let mut errors = Vec::new();
        if self.name.with(|n| n.trim().is_empty()) {
            errors.push("Poné un nombre al producto.".to_string());
        }
        let price = self.price.get();
        let price_cents = if price.trim().is_empty() {
            errors.push("Poné el precio.".to_string());
            0
        } else {
            domain::parse_price(&price).unwrap_or_else(|| {
                errors.extend(domain::price_problem(&price, "El precio"));
                0
            })
        };
        let was = self.was.get();
        let compare_at_cents = if was.trim().is_empty() {
            0
        } else {
            domain::parse_price(&was).unwrap_or_else(|| {
                errors.extend(domain::price_problem(&was, "El precio anterior"));
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
                    errors.push(
                        "La cantidad tiene que ser un número entero, como 5, sin signo menos, o quedar vacía."
                            .to_string(),
                    );
                    None
                }
            }
        };
        let variants = self.variants.variants().unwrap_or_else(|e| {
            errors.push(e);
            domain::Variants::default()
        });
        // Con variantes, el stock es la suma de las combinaciones.
        let stock = if variants.is_empty() { stock } else { Some(variants.total()) };
        let parcel = domain::parse_parcel(
            &self.weight.get(),
            &self.length.get(),
            &self.width.get(),
            &self.height.get(),
        )
        .unwrap_or_else(|e| {
            errors.push(e);
            None
        });
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
            photo_fits: self.fits.get(),
            example: false,
            parcel,
            variants,
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

/// A qué campo del formulario de producto va un error (para mostrarlo
/// debajo): `name`, `price`, `was`, `stock`, `description`, `photos` o
/// `general`.
fn error_field(e: &str) -> &'static str {
    let e = e.to_lowercase();
    if e.contains("el peso") || e.contains("medidas en") {
        "parcel"
    } else if e.contains("precio anterior") {
        "was"
    } else if e.contains("precio") {
        "price"
    } else if e.contains("nombre") {
        "name"
    } else if e.contains("cantidad") || e.contains("stock") {
        "stock"
    } else if e.contains("descripción") {
        "description"
    } else if e.contains("foto") {
        "photos"
    } else {
        "general"
    }
}

/// Lleva la vista y el foco al campo del primer error.
fn focus_first_error(errors: &[String]) {
    let Some(first) = errors.first() else {
        return;
    };
    let id = match error_field(first) {
        "name" => "p-name",
        "price" => "p-price",
        "was" => "p-was",
        "stock" => "p-stock",
        "parcel" => "p-weight",
        "description" => "p-description",
        _ => return,
    };
    request_animation_frame(move || {
        use leptos::wasm_bindgen::JsCast;
        if let Some(el) = document()
            .get_element_by_id(id)
            .and_then(|e| e.dyn_into::<leptos::web_sys::HtmlElement>().ok())
        {
            _ = el.focus();
        }
    });
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
        form.fits.track();
        form.tags.track();
        form.in_stock.track();
        form.stock.track();
        form.visible.track();
        form.weight.track();
        form.length.track();
        form.width.track();
        form.height.track();
        form.variants.track();
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
        })) => {
            focus_first_error(&e);
            errors.set(e);
        }
        Some(Err(e)) => errors.set(vec![error_text(&e)]),
        None => {}
    });
    let on_save = move || match form.product(&id.get_untracked()) {
        Ok(p) => {
            errors.set(Vec::new());
            save.dispatch(p);
        }
        Err(e) => {
            focus_first_error(&e);
            errors.set(e);
        }
    };
    // Los errores de un campo, para mostrarlos debajo de él.
    let field_error = move |field: &'static str| {
        move || {
            errors.with(|es| {
                es.iter()
                    .filter(|e| error_field(e) == field)
                    .map(|e| view! { <p class="label whitespace-normal text-error">{e.clone()}</p> })
                    .collect_view()
            })
        }
    };
    let has_error = move |field: &'static str| errors.with(|es| es.iter().any(|e| error_field(e) == field));
    let save_button = move || view! {
        <button type="button" class="btn btn-primary" disabled=move || save.pending().get()
            on:click=move |_| on_save()>
            {move || if id.get().is_empty() { "Crear producto" } else { "Guardar" }}
        </button>
    };

    let confirm_delete = RwSignal::new(false);
    let delete = Action::new(move |_: &()| async move {
        delete_product(slug.get_value(), id.get_untracked()).await
    });
    let list_href2 = list_href.clone();
    let navigate = leptos_router::hooks::use_navigate();
    Effect::new(move |_| match delete.value().get() {
        Some(Ok(())) => navigate(&list_href2, Default::default()),
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
    // La foto que se está encuadrando sobre la vista previa (si sigue estando).
    let adjusting = RwSignal::new(None::<String>);
    let adjusting_now = Memo::new(move |_| {
        adjusting
            .get()
            .filter(|url| form.photos.with(|p| p.contains(url)))
    });

    view! {
        <div class="flex flex-wrap items-center gap-3">
            <a class="btn btn-ghost btn-sm max-sm:size-11 max-sm:p-0" href=list_href>
                <span aria-hidden="true">"←"</span><span class="max-sm:sr-only">"Productos"</span>
            </a>
            <h1 class="min-w-0 grow truncate text-3xl font-bold max-sm:text-xl">{title}</h1>
            <span class="text-sm text-base-content/60 max-sm:text-xs" role="status" aria-live="polite">
                {move || if save.pending().get() { "Guardando…" } else if saved.get() && !dirty.get() { "Guardado ✓" } else { "" }}
            </span>
            // En el teléfono, Borrar y Guardar están en la barra fija de abajo.
            {move || (!id.get().is_empty()).then(|| view! {
                <button type="button" class="btn btn-ghost max-sm:hidden" on:click=move |_| confirm_delete.set(true)>"Borrar"</button>
            })}
            <span class="contents max-sm:hidden">{save_button()}</span>
        </div>
        {move || confirm_delete.get().then(|| view! {
            <div role="alert" class="alert alert-warning flex flex-wrap max-sm:fixed max-sm:inset-x-3 max-sm:bottom-24 max-sm:z-40 max-sm:shadow-xl">
                <span class="grow">"¿Borrar este producto? Deja de verse en la tienda y no se puede deshacer. Sus fotos siguen en Mis fotos."</span>
                <button type="button" class="btn btn-sm" on:click=move |_| confirm_delete.set(false)>"No, dejarlo"</button>
                <button type="button" class="btn btn-sm btn-error" disabled=move || delete.pending().get()
                    on:click=move |_| { delete.dispatch(()); }>"Sí, borrar"</button>
            </div>
        })}
        // Solo lo que no es de un campo (la conexión, por ejemplo); lo demás
        // va debajo de cada campo.
        <div role="alert" class="text-sm text-error empty:hidden">
            {move || errors.with(|es| es.iter().filter(|e| error_field(e) == "general").map(|e| view! { <p>{e.clone()}</p> }).collect_view())}
        </div>
        <div class="grid items-start gap-6 lg:grid-cols-[1fr_340px]">
            <div class="card bg-base-100 shadow-sm"><div class="card-body gap-5">
                <fieldset class="fieldset">
                    <legend class="fieldset-legend">"Nombre"</legend>
                    <input id="p-name" class="input w-full" class:input-error=move || has_error("name")
                        aria-invalid=move || has_error("name").to_string()
                        maxlength=domain::PRODUCT_NAME_MAX placeholder="Por ejemplo: Taza de cerámica"
                        prop:value=move || form.name.get()
                        on:input=move |ev| form.name.set(event_target_value(&ev))/>
                    {field_error("name")}
                </fieldset>
                <div class="grid gap-4 sm:grid-cols-2">
                    <fieldset class="fieldset">
                        <legend class="fieldset-legend">"Precio"</legend>
                        <label class="input w-full" class:input-error=move || has_error("price")><span>"$"</span>
                            <input id="p-price" inputmode="decimal" placeholder="45.999,90" aria-label="Precio en pesos"
                                aria-invalid=move || has_error("price").to_string()
                                prop:value=move || form.price.get()
                                on:input=move |ev| form.price.set(event_target_value(&ev))/>
                        </label>
                        {field_error("price")}
                    </fieldset>
                    <fieldset class="fieldset">
                        <legend class="fieldset-legend">"Precio anterior " <span class="font-normal text-base-content/60">"(opcional, se muestra tachado)"</span></legend>
                        <label class="input w-full" class:input-error=move || has_error("was")><span>"$"</span>
                            <input id="p-was" inputmode="decimal" placeholder="Vacío si no está en oferta" aria-label="Precio anterior en pesos"
                                aria-invalid=move || has_error("was").to_string()
                                prop:value=move || form.was.get()
                                on:input=move |ev| form.was.set(event_target_value(&ev))/>
                        </label>
                        {field_error("was")}
                    </fieldset>
                </div>
                <PhotosField photos=form.photos fits=form.fits adjusting/>
                {field_error("photos")}
                <fieldset class="fieldset">
                    <legend class="fieldset-legend">"Descripción"</legend>
                    <textarea id="p-description" class="textarea h-28 w-full" maxlength=domain::PRODUCT_DESC_MAX
                        placeholder="Qué es, de qué está hecho, medidas, cómo se usa…"
                        prop:value=move || form.description.get()
                        on:input=move |ev| form.description.set(event_target_value(&ev))></textarea>
                    <p class="label">{move || format!("{} de {} letras", form.description.with(|d| d.chars().count()), domain::PRODUCT_DESC_MAX)}</p>
                    {field_error("description")}
                </fieldset>
                <TagsField form store_tags=store_tags.get_value()/>
                <crate::variants_field::VariantsField form=form.variants photos=form.photos/>
                <div class="grid gap-4 sm:grid-cols-2">
                    {move || form.variants.active().then(|| view! {
                        <fieldset class="fieldset">
                            <legend class="fieldset-legend">"Stock"</legend>
                            <p class="text-sm">{move || format!("{} unidades en total", form.variants.total())}</p>
                            <p class="label whitespace-normal">"Se cuenta por color y talle. Cada venta descuenta de la combinación elegida."</p>
                            {field_error("stock")}
                        </fieldset>
                    })}
                    <fieldset class="fieldset" class:hidden=move || form.variants.active()>
                        <legend class="fieldset-legend">"Stock"</legend>
                        <label class="fieldset-label" for="p-stock">"Cantidad disponible " <span class="text-base-content/60">"(opcional)"</span></label>
                        <input id="p-stock" class="input w-40" class:input-error=move || has_error("stock") inputmode="numeric" placeholder="Sin contar"
                            aria-invalid=move || has_error("stock").to_string()
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
                        {field_error("stock")}
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
                <fieldset class="fieldset" aria-describedby="p-parcel-hint">
                    <legend class="fieldset-legend">"Envío " <span class="font-normal text-base-content/60">"(opcional)"</span></legend>
                    <p id="p-parcel-hint" class="label whitespace-normal">
                        "Peso y medidas del producto ya empaquetado, para cotizar con Andreani. Vacío: se usa el paquete de tu tienda (en Entregas)."
                    </p>
                    <div class="grid grid-cols-2 gap-2 sm:grid-cols-4">
                        <crate::sales::ParcelInput id="p-weight" label="Peso" unit="kg" value=form.weight invalid=Signal::derive(move || has_error("parcel"))/>
                        <crate::sales::ParcelInput id="p-length" label="Largo" unit="cm" value=form.length invalid=Signal::derive(move || has_error("parcel"))/>
                        <crate::sales::ParcelInput id="p-width" label="Ancho" unit="cm" value=form.width invalid=Signal::derive(move || has_error("parcel"))/>
                        <crate::sales::ParcelInput id="p-height" label="Alto" unit="cm" value=form.height invalid=Signal::derive(move || has_error("parcel"))/>
                    </div>
                    {field_error("parcel")}
                </fieldset>
                <div class="mk-savebar flex flex-wrap items-center justify-end gap-3 border-t border-base-200 pt-4">
                    {move || {
                        let n = errors.with(Vec::len);
                        (n > 0).then(|| view! {
                            <p class="text-sm text-error max-sm:hidden">{if n == 1 { "Revisá el campo marcado.".to_string() } else { format!("Revisá los {n} campos marcados.") }}</p>
                        })
                    }}
                    {move || (!id.get().is_empty()).then(|| view! {
                        <button type="button" class="btn btn-outline size-12 shrink-0 p-0 sm:hidden" aria-label="Borrar este producto"
                            on:click=move |_| confirm_delete.set(true)>
                            <ui::Icon class="size-5" name="trash"/>
                        </button>
                    })}
                    {save_button()}
                </div>
            </div></div>
            <aside class="flex flex-col gap-3 lg:sticky lg:top-4">
                <span class="text-sm font-medium text-base-content/70">"Así se ve en tu tienda"</span>
                {move || match adjusting_now.get() {
                    // En el teléfono el ajuste ocupa toda la pantalla; en las demás, `contents`
                    // lo deja como siempre, sobre la vista previa.
                    Some(url) => view! {
                        <div class="contents max-sm:fixed max-sm:inset-0 max-sm:z-[70] max-sm:flex max-sm:flex-col max-sm:gap-3 max-sm:overflow-y-auto max-sm:overscroll-contain max-sm:bg-base-200 max-sm:p-4" role="group" aria-label="Ajustar foto">
                            <div class="flex items-center gap-3 sm:hidden">
                                <h2 class="grow text-lg font-semibold">"Ajustar foto"</h2>
                                <button type="button" class="btn btn-primary" on:click=move |_| adjusting.set(None)>"Listo"</button>
                            </div>
                            <PhotoAdjust url fits=form.fits name=form.name
                                price=Signal::derive(move || preview().map(|p| (p.price_cents, p.compare_at_cents)))
                                on_done=Callback::new(move |()| adjusting.set(None))/>
                        </div>
                    }.into_any(),
                    None => view! {
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
                    }.into_any(),
                }}
            </aside>
        </div>
    }
}

/// Las fotos del producto: elegir de Mis fotos o subir, ordenar y sacar.
#[component]
fn PhotosField(
    photos: RwSignal<Vec<String>>,
    fits: RwSignal<BTreeMap<String, PhotoFit>>,
    /// La foto que se está encuadrando sobre la vista previa.
    adjusting: RwSignal<Option<String>>,
) -> impl IntoView {
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
    // Teléfono: al tocar una foto se abre una hoja con sus acciones (encuadrar,
    // hacerla principal, moverla, sacarla), con botones grandes en vez de
    // flechitas.
    let sheet = RwSignal::new(None::<usize>);
    let sheet_el = NodeRef::<leptos::html::Dialog>::new();
    Effect::new(move |_| {
        let Some(d) = sheet_el.get() else { return };
        if sheet.get().is_some() {
            if !d.open() {
                _ = d.show_modal();
            }
        } else if d.open() {
            d.close();
        }
    });
    view! {
        <fieldset class="fieldset">
            <legend class="fieldset-legend">"Fotos " <span class="font-normal text-base-content/60">"· la primera es la principal; tocá una foto para encuadrarla"</span></legend>
            <ul class="flex flex-wrap gap-3 max-sm:grid max-sm:grid-cols-3 max-sm:gap-2" aria-label="Fotos del producto">
                {move || {
                    let list = photos.get();
                    let last = list.len().saturating_sub(1);
                    list.into_iter().enumerate().map(|(i, url)| {
                        let n = i + 1;
                        let fit = fits.with(|f| f.get(&url).copied());
                        let mine = url.clone();
                        let here = Memo::new(move |_| adjusting.with(|a| a.as_deref() == Some(mine.as_str())));
                        let pick = url.clone();
                        view! {
                            <li class="group relative h-24 w-32 overflow-hidden rounded-box bg-base-200 max-sm:aspect-square max-sm:h-auto max-sm:w-auto"
                                class:ring-2=move || i == 0 || here.get()
                                class:ring-primary=move || i == 0 || here.get()
                                class:ring-4=move || here.get()>
                                <ui::Image src=url alt=format!("Foto {n}") fit/>
                                <button type="button" class="absolute inset-0 cursor-pointer rounded-box focus-visible:outline-2 focus-visible:-outline-offset-2 focus-visible:outline-primary"
                                    aria-label=format!("Encuadrar la foto {n}")
                                    aria-pressed=move || here.get().to_string()
                                    on:click=move |_| {
                                        if is_phone() {
                                            sheet.set(Some(i));
                                        } else {
                                            adjusting.set(Some(pick.clone()));
                                        }
                                    }></button>
                                <span aria-hidden="true" class="pointer-events-none absolute right-1 top-1 grid size-7 place-items-center rounded-full bg-black/70 text-white sm:hidden">"⋯"</span>
                                {move || if here.get() {
                                    view! { <span class="badge badge-primary badge-sm absolute left-1 top-1">"Encuadrando"</span> }.into_any()
                                } else if i == 0 {
                                    view! { <span class="badge badge-primary badge-sm absolute left-1 top-1">"Principal"</span> }.into_any()
                                } else {
                                    ().into_any()
                                }}
                                <div class="absolute inset-x-1 bottom-1 flex justify-between gap-1 max-sm:hidden">
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
                    <button type="button" class="flex h-24 w-32 flex-col items-center justify-center gap-1 rounded-box border-2 border-dashed border-base-300 text-sm text-base-content/70 hover:border-primary max-sm:aspect-square max-sm:h-auto max-sm:w-full max-sm:text-xs"
                        aria-expanded=move || picking.get().to_string()
                        disabled=full
                        on:click=move |_| { library.ensure_loaded(); picking.update(|v| *v = !*v); }>
                        <span class="text-xl" aria-hidden="true">"+"</span>"Elegir de Mis fotos"
                    </button>
                </li>
                <li>
                    <label for="producto-subir" class="flex h-24 w-32 cursor-pointer flex-col items-center justify-center gap-1 rounded-box border-2 border-dashed border-base-300 text-sm text-base-content/70 hover:border-primary max-sm:aspect-square max-sm:h-auto max-sm:w-full max-sm:text-xs"
                        class:pointer-events-none=move || library.progress.get().is_some() || full()>
                        <span class="text-xl" aria-hidden="true">"⇪"</span>"Subir fotos"
                    </label>
                    <input id="producto-subir" type="file" class="sr-only" multiple
                        accept="image/jpeg,image/png,image/webp,image/gif"
                        on:change=move |ev| library.upload_each(editor::take_files(&ev), Callback::new(move |item: domain::MediaItem| add(item.url)))/>
                </li>
            </ul>
            <dialog node_ref=sheet_el class="mk-sheet" aria-labelledby="foto-hoja-titulo"
                on:close=move |_| sheet.set(None)
                on:click=move |ev| {
                    // Tocar el fondo oscuro cierra.
                    use leptos::wasm_bindgen::JsCast;
                    if ev.target().and_then(|t| t.dyn_into::<leptos::web_sys::HtmlDialogElement>().ok()).is_some() {
                        sheet.set(None);
                    }
                }>
                {move || sheet.get().and_then(|i| photos.with(|p| p.get(i).cloned()).map(|url| (i, url))).map(|(i, url)| {
                    let last = photos.with(Vec::len).saturating_sub(1);
                    let fit = fits.with(|f| f.get(&url).copied());
                    let pick = url.clone();
                    view! {
                        <div class="mx-auto mb-3 mt-1 h-1.5 w-10 rounded-full bg-base-300" aria-hidden="true"></div>
                        <div class="flex items-center gap-3 pb-2">
                            <div class="size-14 shrink-0 overflow-hidden rounded-field bg-base-200"><ui::Image src=url alt="" fit/></div>
                            <h2 id="foto-hoja-titulo" class="font-semibold">{if i == 0 { "Foto principal".to_string() } else { format!("Foto {}", i + 1) }}</h2>
                        </div>
                        <ul class="menu w-full p-0 [&_button]:min-h-12 [&_button]:text-base">
                            <li><button type="button" on:click=move |_| { sheet.set(None); adjusting.set(Some(pick.clone())); }>"Ajustar encuadre"</button></li>
                            {(i > 0).then(|| view! {
                                <li><button type="button" on:click=move |_| {
                                    photos.update(|p| { let x = p.remove(i); p.insert(0, x); });
                                    sheet.set(None);
                                }>"Hacer principal"</button></li>
                                <li><button type="button" on:click=move |_| { shift(i, i - 1); sheet.set(None); }>"Mover a la izquierda"</button></li>
                            })}
                            {(i < last).then(|| view! {
                                <li><button type="button" on:click=move |_| { shift(i, i + 1); sheet.set(None); }>"Mover a la derecha"</button></li>
                            })}
                            <li><button type="button" class="text-error" on:click=move |_| {
                                photos.update(|p| { p.remove(i); });
                                sheet.set(None);
                            }>"Quitar de este producto"</button></li>
                        </ul>
                    }
                })}
            </dialog>
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

/// Forma con la que se prueba el encuadre. La forma de verdad la decide cada
/// bloque de la tienda; el encuadre vale para todas.
#[derive(Clone, Copy, PartialEq)]
enum Shape {
    Square,
    Tall,
    Wide,
}

impl Shape {
    const ALL: [Shape; 3] = [Shape::Square, Shape::Tall, Shape::Wide];

    fn class(self) -> &'static str {
        match self {
            Shape::Square => "aspect-square",
            Shape::Tall => "aspect-[4/5]",
            Shape::Wide => "aspect-[4/3]",
        }
    }

    fn label(self) -> &'static str {
        match self {
            Shape::Square => "Cuadrada",
            Shape::Tall => "Vertical",
            Shape::Wide => "Horizontal",
        }
    }

    /// Íconito de la forma.
    fn icon(self) -> &'static str {
        match self {
            Shape::Square => "h-3.5 w-3.5",
            Shape::Tall => "h-4 w-3",
            Shape::Wide => "h-3 w-4",
        }
    }
}

/// Cuánto % se mueve el punto a la vista al arrastrar `drag` px, en un lado
/// de `boxed` px donde la foto (ya cubriendo el lugar) mide `shown` px.
fn drag_to_pct(drag: f64, boxed: f64, shown: f64, zoom: f64) -> f64 {
    // Al subir 1 % el punto, la foto se corre (sobrante × zoom + lugar ×
    // (zoom − 1)) / 100 px: lo que sobra por object-position más lo que
    // agrega el zoom alrededor del punto.
    let travel = (shown - boxed).max(0.0) * zoom + boxed * (zoom - 1.0);
    if travel < 1.0 {
        0.0
    } else {
        -drag * 100.0 / travel
    }
}

/// Encuadrar una foto sobre la vista previa: se arrastra para moverla, zoom
/// con la barrita (o la rueda) y se prueba en cada forma. No toca la foto
/// guardada: queda en `fits` y se guarda con el producto.
#[component]
fn PhotoAdjust(
    url: String,
    fits: RwSignal<BTreeMap<String, PhotoFit>>,
    name: RwSignal<String>,
    /// Precio y precio anterior, si se pueden leer.
    #[prop(into)]
    price: Signal<Option<(i64, i64)>>,
    on_done: Callback<()>,
) -> impl IntoView {
    let key = StoredValue::new(url.clone());
    let opened = fits.with_untracked(|f| f.get(&url).copied().unwrap_or_default());
    // Mientras se arrastra se mueve esta; al soltar se guarda en `fits`.
    let fit = RwSignal::new(opened);
    let commit = move || {
        let f = fit.get_untracked().clamped();
        fits.update(|all| {
            if f.is_default() {
                all.remove(&key.get_value());
            } else {
                all.insert(key.get_value(), f);
            }
        });
    };
    let shape = RwSignal::new(Shape::Square);
    let moved = RwSignal::new(false);
    let img = NodeRef::<leptos::html::Img>::new();
    // Dónde empezó el arrastre: puntero, encuadre y medidas.
    let start = StoredValue::new(None::<(f64, f64, PhotoFit)>);
    // Dedos apoyados (id, x, y) y, con dos, la distancia inicial y el encuadre
    // de partida: el pellizco acerca o aleja la foto.
    let touches = StoredValue::new(Vec::<(i32, f64, f64)>::new());
    let pinch = StoredValue::new(None::<(f64, PhotoFit)>);
    let gap = |t: &[(i32, f64, f64)]| {
        let (a, b) = (t[0], t[1]);
        ((a.1 - b.1).powi(2) + (a.2 - b.2).powi(2)).sqrt()
    };

    let measure = move || {
        let el = img.get_untracked()?;
        let (bw, bh) = (f64::from(el.client_width()), f64::from(el.client_height()));
        let (nw, nh) = (
            f64::from(el.natural_width()),
            f64::from(el.natural_height()),
        );
        if bw < 1.0 || bh < 1.0 || nw < 1.0 || nh < 1.0 {
            return None;
        }
        let cover = (bw / nw).max(bh / nh);
        Some((bw, bh, nw * cover, nh * cover))
    };
    let nudge = move |dx: f64, dy: f64, base: PhotoFit| {
        let Some((bw, bh, sw, sh)) = measure() else {
            return;
        };
        let z = f64::from(base.zoom) / 100.0;
        let x = f64::from(base.x) + drag_to_pct(dx, bw, sw, z);
        let y = f64::from(base.y) + drag_to_pct(dy, bh, sh, z);
        fit.set(PhotoFit {
            x: x.round().clamp(0.0, 100.0) as u8,
            y: y.round().clamp(0.0, 100.0) as u8,
            ..base
        });
    };
    let set_zoom = move |z: f64| {
        let z = z
            .round()
            .clamp(f64::from(domain::ZOOM_MIN), f64::from(domain::ZOOM_MAX));
        fit.update(|f| f.zoom = z as u16);
    };

    let on_down = move |ev: leptos::ev::PointerEvent| {
        if ev.button() != 0 {
            return;
        }
        ev.prevent_default();
        if let Some(t) = ev.current_target().and_then(|t| {
            leptos::wasm_bindgen::JsCast::dyn_into::<leptos::web_sys::Element>(t).ok()
        }) {
            _ = t.set_pointer_capture(ev.pointer_id());
        }
        touches.update_value(|t| {
            t.retain(|p| p.0 != ev.pointer_id());
            t.push((ev.pointer_id(), f64::from(ev.client_x()), f64::from(ev.client_y())));
        });
        let two = touches.with_value(|t| t.len() >= 2);
        if two {
            // Con el segundo dedo deja de arrastrar y empieza el pellizco.
            start.set_value(None);
            let d = touches.with_value(|t| gap(t));
            pinch.set_value((d > 1.0).then(|| (d, fit.get_untracked())));
        } else {
            start.set_value(Some((
                f64::from(ev.client_x()),
                f64::from(ev.client_y()),
                fit.get_untracked(),
            )));
        }
    };
    let on_move = move |ev: leptos::ev::PointerEvent| {
        touches.update_value(|t| {
            if let Some(p) = t.iter_mut().find(|p| p.0 == ev.pointer_id()) {
                p.1 = f64::from(ev.client_x());
                p.2 = f64::from(ev.client_y());
            }
        });
        if let Some((d0, base)) = pinch.get_value() {
            if touches.with_value(|t| t.len() >= 2) {
                let d = touches.with_value(|t| gap(t));
                moved.set(true);
                set_zoom(f64::from(base.zoom) * d / d0);
            }
            return;
        }
        if let Some((x0, y0, base)) = start.get_value() {
            moved.set(true);
            nudge(
                f64::from(ev.client_x()) - x0,
                f64::from(ev.client_y()) - y0,
                base,
            );
        }
    };
    let on_up = move |ev: leptos::ev::PointerEvent| {
        touches.update_value(|t| t.retain(|p| p.0 != ev.pointer_id()));
        if pinch.get_value().is_some() {
            // Terminó el pellizco; si queda un dedo, sigue arrastrando desde ahí.
            pinch.set_value(None);
            commit();
            if let Some((_, x, y)) = touches.with_value(|t| t.first().copied()) {
                start.set_value(Some((x, y, fit.get_untracked())));
            }
            return;
        }
        if start.get_value().is_some() {
            start.set_value(None);
            commit();
        }
    };
    let on_wheel = move |ev: leptos::ev::WheelEvent| {
        ev.prevent_default();
        set_zoom(f64::from(fit.get_untracked().zoom) - ev.delta_y() / 4.0);
        commit();
    };
    let on_key = move |ev: leptos::ev::KeyboardEvent| {
        let base = fit.get_untracked();
        let step = 8.0;
        match ev.key().as_str() {
            "ArrowLeft" => nudge(-step, 0.0, base),
            "ArrowRight" => nudge(step, 0.0, base),
            "ArrowUp" => nudge(0.0, -step, base),
            "ArrowDown" => nudge(0.0, step, base),
            "+" | "=" => set_zoom(f64::from(base.zoom) + 10.0),
            "-" => set_zoom(f64::from(base.zoom) - 10.0),
            _ => return,
        }
        ev.prevent_default();
        moved.set(true);
        commit();
    };

    view! {
        <div class="card overflow-hidden border border-base-300 bg-base-100 shadow-sm outline-3 outline-offset-2 outline-primary">
            <div class=move || format!("relative w-full cursor-grab touch-none select-none overflow-hidden bg-base-200 active:cursor-grabbing focus-visible:outline-2 focus-visible:-outline-offset-2 focus-visible:outline-primary {}", shape.get().class())
                tabindex="0" role="application"
                aria-label="Encuadre de la foto. Arrastrala o usá las flechas para moverla, y + o − para el zoom. En pantalla táctil, pellizcá para acercar."
                on:pointerdown=on_down on:pointermove=on_move on:pointerup=on_up on:pointercancel=on_up
                on:wheel=on_wheel on:keydown=on_key>
                <img node_ref=img src=url class="pointer-events-none block h-full w-full object-cover"
                    style=move || fit.get().css() alt="" draggable="false"/>
                {move || (!moved.get()).then(|| view! {
                    <span class="pointer-events-none absolute left-1/2 top-1/2 -translate-x-1/2 -translate-y-1/2 rounded-full bg-black/60 px-3 py-1.5 text-sm whitespace-nowrap text-white max-sm:w-max max-sm:max-w-[85%] max-sm:whitespace-normal max-sm:text-center">
                        "✥ Arrastrá para mover"<span class="sm:hidden">" · pellizcá para acercar"</span>
                    </span>
                })}
            </div>
            <div class="card-body gap-0 p-4">
                <h3 class="truncate text-sm font-semibold">{move || name.get()}</h3>
                {move || price.get().map(|(cents, was)| view! { <ui::PriceWas cents was/> })}
            </div>
        </div>
        <div class="flex flex-col gap-3 rounded-box bg-base-100 p-4 shadow-md max-sm:[&_.btn]:min-h-11">
            <label class="flex items-center gap-3 text-sm">
                <span>"Zoom"</span>
                <input type="range" class="range range-primary range-xs grow"
                    min=domain::ZOOM_MIN max=domain::ZOOM_MAX step="5"
                    aria-valuetext=move || format!("{} %", fit.get().zoom)
                    prop:value=move || fit.get().zoom.to_string()
                    on:input=move |ev| { if let Ok(z) = event_target_value(&ev).parse::<f64>() { set_zoom(z) } }
                    on:change=move |_| commit()/>
                <span class="w-14 text-right whitespace-nowrap text-base-content/60 tabular-nums">{move || format!("{} %", fit.get().zoom)}</span>
            </label>
            <div class="flex items-center gap-3 text-sm">
                <span>"Ver en forma"</span>
                <div class="join" role="group" aria-label="Probar en otra forma">
                    {Shape::ALL.into_iter().map(|s| view! {
                        <button type="button" class="btn btn-sm join-item" title=s.label() aria-label=s.label()
                            class:btn-active=move || shape.get() == s
                            class:text-primary=move || shape.get() == s
                            aria-pressed=move || (shape.get() == s).to_string()
                            on:click=move |_| shape.set(s)>
                            <span class=format!("inline-block rounded-[2px] border-[1.5px] border-current {}", s.icon())></span>
                        </button>
                    }).collect_view()}
                </div>
            </div>
            <p class="text-xs text-base-content/60">"Cada bloque de tu tienda tiene su forma; lo que encuadres vale para todas."</p>
            <div class="flex items-center gap-2">
                <button type="button" class="btn btn-ghost btn-sm -ml-2"
                    on:click=move |_| { fit.set(opened); commit(); }>"↺ Como estaba"</button>
                <button type="button" class="btn btn-ghost btn-sm"
                    on:click=move |_| { fit.set(PhotoFit::default()); commit(); }>"Centrar"</button>
                <span class="grow"></span>
                <button type="button" class="btn btn-primary btn-sm" on:click=move |_| on_done.run(())>"Listo"</button>
            </div>
        </div>
        <p class="text-xs text-base-content/60">"La foto original no cambia. Al tocar Guardar, tu tienda la muestra así."</p>
    }
}
