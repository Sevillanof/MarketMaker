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

    /// La tienda `slug`, si quien entró está en su equipo (cualquier rol).
    /// `Ok(None)` si no entró.
    pub async fn owned_store(slug: &str) -> Result<Option<db::Tenant>, ServerFnError> {
        owned_store_min(slug, "editor").await
    }

    /// Como [`owned_store`], pero pide al menos el rol `min` (`editor`, `admin`
    /// u `owner`). Pedidos, entregas, cobros y cupones piden `admin`.
    pub async fn owned_store_min(
        slug: &str,
        min: &str,
    ) -> Result<Option<db::Tenant>, ServerFnError> {
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
            Some(role) if domain::role_rank(&role) >= domain::role_rank(min) => Ok(Some(tenant)),
            Some(_) => Err(ServerFnError::new(
                "Tu rol en esta tienda no alcanza para hacer esto.",
            )),
            None => Err(ServerFnError::new("Tu cuenta no puede editar esta tienda.")),
        }
    }

    pub async fn require_store(slug: &str) -> Result<db::Tenant, ServerFnError> {
        require_store_min(slug, "editor").await
    }

    pub async fn require_store_min(slug: &str, min: &str) -> Result<db::Tenant, ServerFnError> {
        owned_store_min(slug, min)
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
        category_order: db::products::category_order(&pool, tenant.id)
            .await
            .map_err(internal)?,
        store_url: crate::email::store_url(&slug),
    }))
}

/// Cómo se ve la ficha del producto en la tienda `slug`: su tema, letras,
/// estilo y diseño de la ficha (lo publicado; si nunca publicó, el borrador).
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct SheetLook {
    pub theme: String,
    pub style: String,
    pub modal: domain::ProductModal,
    pub store: domain::StoreInfo,
}

#[server]
pub async fn store_sheet_look(slug: String) -> Result<SheetLook, ServerFnError> {
    use crate::server::{internal, pool};
    let tenant = server::require_store(&slug).await?;
    let pool = pool()?;
    let published = db::get_published_page(&pool, tenant.id, "home")
        .await
        .map_err(internal)?
        .and_then(|p| p.doc);
    let doc = match published {
        Some(d) => Some(d),
        None => db::get_current_page(&pool, tenant.id, "home")
            .await
            .map_err(internal)?
            .and_then(|p| p.doc),
    };
    let doc = match doc {
        Some(v) => domain::parse_doc(v).map_err(internal)?,
        None => domain::PageDoc::default(),
    };
    let store = db::auth::store_info(&pool, tenant.id)
        .await
        .map_err(internal)?;
    Ok(SheetLook {
        style: format!("{}; {}", doc.typography.css_vars(), doc.style.css_vars()),
        theme: doc.theme,
        modal: doc.product_modal,
        store,
    })
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

/// Cambia el nombre de una categoría (o de una subcategoría, si `sub` no
/// está vacío) en todos los productos. Devuelve cómo quedaron.
#[server]
pub async fn rename_category(
    slug: String,
    cat: String,
    sub: String,
    new: String,
) -> Result<domain::TagSave, ServerFnError> {
    use crate::server::{internal, pool};
    let tenant = server::require_store(&slug).await?;
    let new = domain::clean_category(&new);
    if new.is_empty() {
        return Ok(domain::TagSave::Invalid {
            error: "Escribí un nombre.".into(),
        });
    }
    let pool = pool()?;
    db::products::rename_category(&pool, tenant.id, &cat, &sub, &new)
        .await
        .map_err(internal)?;
    if sub.is_empty() {
        let mut order = db::products::category_order(&pool, tenant.id)
            .await
            .map_err(internal)?;
        for o in order.iter_mut().filter(|o| **o == cat) {
            o.clone_from(&new);
        }
        order.dedup();
        db::products::set_category_order(&pool, tenant.id, &order)
            .await
            .map_err(internal)?;
    }
    let products = db::products::list_all(&pool, tenant.id)
        .await
        .map_err(internal)?;
    Ok(domain::TagSave::Saved { products })
}

/// Saca una categoría (o una subcategoría) de todos los productos. Los
/// productos no se borran.
#[server]
pub async fn delete_category(
    slug: String,
    cat: String,
    sub: String,
) -> Result<domain::TagSave, ServerFnError> {
    use crate::server::{internal, pool};
    let tenant = server::require_store(&slug).await?;
    let pool = pool()?;
    db::products::delete_category(&pool, tenant.id, &cat, &sub)
        .await
        .map_err(internal)?;
    if sub.is_empty() {
        let mut order = db::products::category_order(&pool, tenant.id)
            .await
            .map_err(internal)?;
        order.retain(|o| *o != cat);
        db::products::set_category_order(&pool, tenant.id, &order)
            .await
            .map_err(internal)?;
    }
    let products = db::products::list_all(&pool, tenant.id)
        .await
        .map_err(internal)?;
    Ok(domain::TagSave::Saved { products })
}

/// Guarda el orden de las categorías (las de arriba van primero en la
/// tienda). Una categoría nueva, sin productos todavía, se crea sumándola acá.
#[server(input = leptos::server_fn::codec::Json)]
pub async fn save_category_order(
    slug: String,
    order: Vec<String>,
) -> Result<Result<Vec<String>, String>, ServerFnError> {
    use crate::server::{internal, pool};
    let tenant = server::require_store(&slug).await?;
    let mut clean: Vec<String> = Vec::new();
    for name in order.iter().map(|n| domain::clean_category(n)) {
        if name.is_empty() {
            return Ok(Err("Escribí un nombre.".into()));
        }
        if !clean
            .iter()
            .any(|c| c.to_lowercase() == name.to_lowercase())
        {
            clean.push(name);
        }
    }
    if clean.len() > 200 {
        return Ok(Err("Son demasiadas categorías.".into()));
    }
    db::products::set_category_order(&pool()?, tenant.id, &clean)
        .await
        .map_err(internal)?;
    Ok(Ok(clean))
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
        store_categories: domain::order_categories(
            domain::categories_from_pairs(
                &db::products::category_pairs(&pool, tenant.id)
                    .await
                    .map_err(internal)?,
            ),
            &db::products::category_order(&pool, tenant.id)
                .await
                .map_err(internal)?,
            true,
        ),
        andreani: db::andreani::account(&pool, tenant.id)
            .await
            .map_err(internal)?
            .is_some(),
        store_url: crate::email::store_url(&slug),
        published: db::get_current_page(&pool, tenant.id, "home")
            .await
            .map_err(internal)?
            .is_some_and(|p| p.published > 0),
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

pub(crate) fn slug_param() -> Memo<String> {
    let params = use_params_map();
    Memo::new(move |_| params.read().get("slug").unwrap_or_default())
}

/// `/tienda/:slug/productos` y `/tienda/:slug/productos/:id`: la lista con
/// el panel rápido y, con `:id` (o `nuevo`), la ficha completa abierta.
#[component]
pub fn ProductsPage() -> impl IntoView {
    if matches!(crate::site::current_site(), domain::Site::Store(_)) {
        return view! { <crate::Home/> }.into_any();
    }
    let slug = slug_param();
    let params = use_params_map();
    let first_ficha = params.read_untracked().get("id");
    let data = Resource::new(move || slug.get(), list_store_products);
    view! {
        <leptos_meta::Title text="Productos · MarketMaker"/>
            <div class="flex w-full max-w-[1440px] flex-col gap-5 px-8 max-sm:px-4 py-8 max-sm:py-5 max-sm:pb-24">
                <Suspense fallback=|| view! { <ui::PanelSkeleton kind=ui::PanelSkeletonKind::List/> }>
                    {move || {
                        let first_ficha = first_ficha.clone();
                        Suspend::new(async move {
                            match data.await {
                                Ok(Some(list)) => view! { <ProductList slug=slug.get_untracked() data=list first_ficha/> }.into_any(),
                                Ok(None) => view! { <SignInFirst back=format!("/tienda/{}/productos", slug.get_untracked())/> }.into_any(),
                                Err(e) => view! { <div role="alert" class="alert alert-error">{error_text(&e)}</div> }.into_any(),
                            }
                        })
                    }}
                </Suspense>
            </div>
    }
    .into_any()
}

/// Los filtros de la lista.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Filter {
    All,
    Out,
    Few,
    Hidden,
}

impl Filter {
    const ALL: [(Filter, &'static str); 4] = [
        (Filter::All, "Todos"),
        (Filter::Out, "Agotados"),
        (Filter::Few, "Pocas"),
        (Filter::Hidden, "Ocultos"),
    ];

    fn keeps(self, p: &Product) -> bool {
        match self {
            Filter::All => true,
            Filter::Out => !p.in_stock,
            Filter::Few => p.in_stock && p.stock.is_some_and(|n| (1..=3).contains(&n)),
            Filter::Hidden => !p.visible,
        }
    }
}

/// Lo que comparten la lista, el panel rápido y la ficha completa.
#[derive(Clone, Copy)]
struct ListCtx {
    slug: StoredValue<String>,
    products: RwSignal<Vec<Product>>,
    order: RwSignal<Vec<String>>,
    /// Lo que se ve en la lista, con la búsqueda y el filtro.
    shown: Memo<Vec<Product>>,
    /// El producto del panel rápido.
    selected: RwSignal<Option<String>>,
    /// En el teléfono, el panel rápido abierto a pantalla completa.
    phone_open: RwSignal<bool>,
    /// La ficha completa abierta: `Some("")` para uno nuevo.
    ficha: RwSignal<Option<String>>,
    /// Lo que la ficha cargó (no cambia cuando se crea el producto nuevo,
    /// para no volver a armar el formulario).
    ficha_src: RwSignal<Option<String>>,
    /// El panel rápido tiene cambios sin guardar.
    panel_dirty: RwSignal<bool>,
    /// La ficha completa tiene cambios sin guardar.
    ficha_dirty: RwSignal<bool>,
    /// Lo que se hace si la persona elige descartar los cambios.
    pending: RwSignal<Option<Callback<()>>>,
    /// Sube para volver a armar el panel o la ficha (Descartar).
    nonce: RwSignal<u32>,
    ask_delete: Callback<(String, String)>,
    store_url: StoredValue<String>,
}

impl ListCtx {
    /// Hace `then`, pero si hay cambios sin guardar pregunta antes.
    fn guard(self, dirty: RwSignal<bool>, then: impl Fn() + Send + Sync + 'static) {
        if dirty.get_untracked() {
            self.pending.set(Some(Callback::new(move |()| then())));
        } else {
            then();
        }
    }

    fn select(self, id: String) {
        self.guard(self.panel_dirty, move || {
            self.panel_dirty.set(false);
            self.selected.set(Some(id.clone()));
            self.phone_open.set(true);
        });
    }

    /// Abre la ficha completa de `id` (vacío: un producto nuevo).
    fn open_ficha(self, id: String) {
        self.guard(self.panel_dirty, move || {
            self.panel_dirty.set(false);
            self.show_ficha(Some(id.clone()));
        });
    }

    fn show_ficha(self, id: Option<String>) {
        self.ficha_src.set(id.clone());
        self.ficha.set(id);
    }

    /// El vecino de `id` en la lista (`step` -1 o 1).
    fn neighbor(self, id: &str, step: i32) -> Option<String> {
        self.shown.with_untracked(|ps| {
            let i = ps.iter().position(|p| p.id == id)? as i32 + step;
            usize::try_from(i)
                .ok()
                .and_then(|i| ps.get(i))
                .map(|p| p.id.clone())
        })
    }

    /// Cambia la dirección sin recargar (para que el enlace abra lo mismo).
    fn set_url(self, ficha: Option<&str>) {
        let base = format!("/tienda/{}/productos", self.slug.get_value());
        let url = match ficha {
            Some("") => format!("{base}/nuevo"),
            Some(id) => format!("{base}/{id}"),
            None => base,
        };
        if let Ok(history) = window().history() {
            _ = history.replace_state_with_url(
                &leptos::wasm_bindgen::JsValue::NULL,
                "",
                Some(&url),
            );
        }
    }
}

/// Lo que dice la pastilla de stock de un producto.
fn stock_pill(p: &Product) -> (&'static str, String) {
    if !p.in_stock {
        ("mk-stock-out", "Agotado".into())
    } else {
        match p.stock {
            Some(n) => ("", format!("{n} u.")),
            None => ("", "Hay stock".into()),
        }
    }
}

#[component]
fn ProductList(slug: String, data: StoreProducts, first_ficha: Option<String>) -> impl IntoView {
    let first_ficha = first_ficha.map(|id| if id == "nuevo" { String::new() } else { id });
    let products = RwSignal::new(data.products);
    let order = RwSignal::new(data.category_order);
    // "Etiquetas" filtra con esto la lista de su ventana.
    let tag = RwSignal::new(None::<String>);
    let _ = data.tags;
    let manage = NodeRef::<leptos::html::Dialog>::new();
    let manage_cats = NodeRef::<leptos::html::Dialog>::new();
    let query = RwSignal::new(String::new());
    let filter = RwSignal::new(Filter::All);
    let problem = RwSignal::new(None::<String>);
    let slug = StoredValue::new(slug);
    // "Importar": sube para abrir la ventana; al terminar se vuelve a leer la lista.
    let import_open = RwSignal::new(0u32);
    let notice = RwSignal::new(None::<String>);
    let import_done = Callback::new(move |done: crate::import::ImportDone| {
        leptos::task::spawn_local(async move {
            if let Ok(Some(fresh)) = list_store_products(slug.get_value()).await {
                products.set(fresh.products);
                order.set(fresh.category_order);
            }
            problem.set(done.note.clone());
            notice.set(Some(match done.created {
                1 => "Importamos 1 producto.".to_string(),
                n => format!("Importamos {n} productos."),
            }));
        });
    });

    let summary = move || {
        products.with(|ps| {
            let visible = ps.iter().filter(|p| p.visible).count();
            match ps.len() {
                1 => format!(
                    "1 producto · {}",
                    if visible == 1 {
                        "en la tienda"
                    } else {
                        "oculto"
                    }
                ),
                n => format!("{n} productos · {visible} en la tienda"),
            }
        })
    };
    let shown = Memo::new(move |_| {
        let q = query.get();
        let f = filter.get();
        products.with(|ps| {
            ps.iter()
                .filter(|p| domain::product_matches(p, &q) && f.keeps(p))
                .cloned()
                .collect::<Vec<_>>()
        })
    });

    // Borrar, preguntando antes.
    let confirm = NodeRef::<leptos::html::Dialog>::new();
    let doomed = RwSignal::new(None::<(String, String)>);
    let ask_delete = Callback::new(move |(id, name): (String, String)| {
        doomed.set(Some((id, name)));
        if let Some(d) = confirm.get() {
            _ = d.show_modal();
        }
    });
    let ctx = ListCtx {
        slug,
        products,
        order,
        shown,
        selected: RwSignal::new(None),
        phone_open: RwSignal::new(false),
        ficha: RwSignal::new(first_ficha.clone()),
        ficha_src: RwSignal::new(first_ficha),
        panel_dirty: RwSignal::new(false),
        ficha_dirty: RwSignal::new(false),
        pending: RwSignal::new(None),
        nonce: RwSignal::new(0),
        ask_delete,
        store_url: StoredValue::new(data.store_url),
    };
    let remove = Action::new(move |id: &String| {
        let id = id.clone();
        async move {
            match delete_product(slug.get_value(), id.clone()).await {
                Ok(()) => {
                    problem.set(None);
                    let next = ctx.neighbor(&id, 1).or_else(|| ctx.neighbor(&id, -1));
                    products.update(|ps| ps.retain(|p| p.id != id));
                    if ctx.selected.get_untracked().as_deref() == Some(id.as_str()) {
                        ctx.panel_dirty.set(false);
                        ctx.selected.set(next);
                        ctx.phone_open.set(false);
                    }
                    if ctx.ficha.get_untracked().as_deref() == Some(id.as_str()) {
                        ctx.ficha_dirty.set(false);
                        ctx.show_ficha(None);
                    }
                    if let Some(d) = confirm.get_untracked() {
                        d.close();
                    }
                }
                Err(e) => problem.set(Some(error_text(&e))),
            }
        }
    });
    // El panel muestra el elegido o, si no hay, el primero de la lista.
    let current = Memo::new(move |_| {
        let sel = ctx.selected.get();
        sel.filter(|id| products.with(|all| all.iter().any(|p| &p.id == id)))
            .or_else(|| shown.with(|ps| ps.first().map(|p| p.id.clone())))
    });
    // Los productos de ejemplo de la tienda nueva, con el botón para borrarlos.
    let examples = Memo::new(move |_| products.with(|ps| ps.iter().filter(|p| p.example).count()));
    let clear_examples = Action::new(move |_: &()| async move {
        match crate::panel::delete_examples(slug.get_value()).await {
            Ok(()) => {
                problem.set(None);
                products.update(|ps| ps.retain(|p| !p.example));
            }
            Err(e) => problem.set(Some(error_text(&e))),
        }
    });

    // "¿Descartar los cambios?"
    let leave = NodeRef::<leptos::html::Dialog>::new();
    Effect::new(move |_| {
        let open = ctx.pending.with(Option::is_some);
        if let Some(d) = leave.get() {
            if open && !d.open() {
                _ = d.show_modal();
            } else if !open && d.open() {
                d.close();
            }
        }
    });

    let new_product = move |_| ctx.open_ficha(String::new());
    let empty = move || products.with(Vec::is_empty);
    view! {
        <div class="flex flex-wrap items-center gap-3">
            <div class="grow max-sm:basis-full">
                <h1 class="text-3xl font-bold max-sm:sr-only">"Productos"</h1>
                <p class="text-sm text-base-content/70">{summary}</p>
            </div>
            <button type="button" class="btn btn-ghost max-sm:hidden" on:click=move |_| import_open.update(|n| *n += 1)>"Importar"</button>
            <button type="button" class="btn max-sm:min-h-11 max-sm:flex-1" on:click=move |_| {
                if let Some(d) = manage.get() { _ = d.show_modal(); }
            }>"Etiquetas"</button>
            <button type="button" class="btn max-sm:min-h-11 max-sm:flex-1" on:click=move |_| {
                if let Some(d) = manage_cats.get() { _ = d.show_modal(); }
            }>"Categorías"</button>
            // En el teléfono va a todo el ancho, debajo del título (ver `.mk-fab`).
            <button type="button" class="btn btn-primary mk-fab max-sm:order-first" on:click=new_product>"+ Nuevo producto"</button>
        </div>
        <dialog node_ref=manage class="modal" aria-labelledby="etiquetas-titulo">
            <div class="modal-box max-w-3xl">
                <TagManager slug=slug.get_value() products tag dialog=manage/>
            </div>
            <form method="dialog" class="modal-backdrop"><button>"Cerrar"</button></form>
        </dialog>
        <dialog node_ref=manage_cats class="modal" aria-labelledby="categorias-titulo">
            <div class="modal-box max-w-xl">
                <CategoryManager slug=slug.get_value() products order dialog=manage_cats/>
            </div>
            <form method="dialog" class="modal-backdrop"><button>"Cerrar"</button></form>
        </dialog>
        <dialog node_ref=confirm class="modal" aria-labelledby="borrar-producto-titulo">
            <div class="modal-box max-w-sm">
                <h2 id="borrar-producto-titulo" class="text-lg font-semibold">
                    {move || doomed.get().map(|(_, n)| format!("¿Borrar «{n}»?")).unwrap_or_default()}
                </h2>
                <p class="mt-1 text-sm text-base-content/70">"Deja de verse en tu tienda y no se puede deshacer. Los pedidos que ya tenga no cambian y sus fotos siguen en Mis fotos."</p>
                <div class="modal-action mt-4">
                    <form method="dialog"><button class="btn btn-ghost" autofocus>"No, dejarlo"</button></form>
                    <button type="button" class="btn btn-error" disabled=move || remove.pending().get()
                        on:click=move |_| { if let Some((id, _)) = doomed.get_untracked() { remove.dispatch(id); } }>
                        {move || if remove.pending().get() { "Borrando…" } else { "Sí, borrar" }}
                    </button>
                </div>
            </div>
            <form method="dialog" class="modal-backdrop"><button>"Cerrar"</button></form>
        </dialog>
        <dialog node_ref=leave class="modal" aria-labelledby="descartar-titulo"
            on:close=move |_| ctx.pending.set(None)>
            <div class="modal-box max-w-sm">
                <h2 id="descartar-titulo" class="text-lg font-semibold">"¿Descartar los cambios?"</h2>
                <p class="mt-1 text-sm text-base-content/70">"Hay cambios sin guardar en este producto."</p>
                <div class="modal-action mt-4">
                    <form method="dialog"><button class="btn btn-ghost" autofocus>"Seguir editando"</button></form>
                    <button type="button" class="btn btn-error"
                        on:click=move |_| {
                            if let Some(then) = ctx.pending.get_untracked() {
                                ctx.pending.set(None);
                                then.run(());
                            }
                        }>"Descartar cambios"</button>
                </div>
            </div>
            <form method="dialog" class="modal-backdrop"><button>"Cerrar"</button></form>
        </dialog>
        {move || (examples.get() > 0).then(|| view! {
            <div role="region" aria-label="Productos de ejemplo" class="flex flex-wrap items-center gap-3 rounded-box border border-amber-300 bg-amber-50 px-4 py-3 text-amber-900">
                <p class="grow text-sm">
                    <b>{move || match examples.get() { 1 => "1 producto es de ejemplo.".to_string(), n => format!("{n} productos son de ejemplo.") }}</b>
                    " Tus clientes no los ven. Borralos cuando cargues los tuyos."
                </p>
                <button type="button" class="btn btn-sm" disabled=move || clear_examples.pending().get()
                    on:click=move |_| { clear_examples.dispatch(()); }>
                    {move || if clear_examples.pending().get() { "Borrando…" } else { "Borrar los de ejemplo" }}
                </button>
            </div>
        })}
        {move || notice.get().map(|e| view! { <p role="status" class="text-sm text-success">{e}</p> })}
        {move || problem.get().map(|e| view! { <p role="alert" class="text-sm text-error">{e}</p> })}
        <crate::import::ImportDialog slug open=import_open on_done=import_done/>
        <Show when=move || !empty() fallback=move || view! {
            <div class="card bg-base-100 shadow-sm"><div class="card-body items-start">
                <h2 class="card-title">"Todavía no cargaste productos"</h2>
                <p>"Cargá el primero con su nombre, precio y fotos. Las grillas de tu tienda lo muestran solas."</p>
                <button type="button" class="btn btn-primary" on:click=new_product>"+ Nuevo producto"</button>
            </div></div>
        }>
            <div class="grid items-start gap-4 md:grid-cols-[320px_1fr] lg:grid-cols-[400px_1fr]">
                <section class="card overflow-hidden bg-base-100 shadow-sm md:sticky md:top-4 md:max-h-[calc(100dvh-2rem)]" aria-label="Lista de productos">
                    <div class="flex flex-col gap-2 border-b border-base-200 p-3">
                        <label class="input w-full max-sm:h-12">
                            <span class="text-base-content/50" aria-hidden="true">"⌕"</span>
                            <input type="search" placeholder="Buscar producto"
                                aria-label="Buscar por nombre, descripción o etiqueta"
                                prop:value=move || query.get()
                                on:input=move |ev| query.set(event_target_value(&ev))/>
                        </label>
                        <div class="tabs tabs-box tabs-sm grid grid-cols-4 max-sm:tabs-md" role="group" aria-label="Filtrar">
                            {Filter::ALL.into_iter().map(|(f, label)| view! {
                                <button type="button" class="tab px-1" class:tab-active=move || filter.get() == f
                                    aria-pressed=move || (filter.get() == f).to_string()
                                    on:click=move |_| filter.set(f)>{label}</button>
                            }).collect_view()}
                        </div>
                    </div>
                    <ul class="overflow-y-auto md:max-h-[calc(100dvh-10rem)]" aria-label="Productos">
                        <For each=move || shown.get() key=|p| (p.id.clone(), p.name.clone(), p.price_cents, p.stock, p.in_stock, p.visible, p.image_url.clone(), p.category.clone()) let:p>
                            {
                                let id = p.id.clone();
                                let id_sel = p.id.clone();
                                let (pill_class, pill) = stock_pill(&p);
                                let sub = [
                                    (!p.category.is_empty()).then(|| p.category.clone()),
                                    p.example.then(|| "De ejemplo".to_string()),
                                    (!p.visible).then(|| "Oculto".to_string()),
                                ].into_iter().flatten().collect::<Vec<_>>().join(" · ");
                                let on = Memo::new(move |_| current.with(|c| c.as_deref() == Some(id_sel.as_str())));
                                view! {
                                    <li class="border-b border-base-200 last:border-0">
                                        <button type="button" class="mk-prow" class:mk-prow-on=move || on.get()
                                            aria-current=move || on.get().then_some("true")
                                            on:click=move |_| ctx.select(id.clone())>
                                            <span class="size-11 shrink-0 overflow-hidden rounded-field bg-base-200 max-sm:size-12" class:opacity-60=!p.visible>
                                                <ui::Image src=p.image_url.clone() alt="" fit=p.fit(&p.image_url)/>
                                            </span>
                                            <span class="min-w-0 grow text-left">
                                                <span class="block truncate font-semibold">{p.name.clone()}</span>
                                                <span class="block truncate text-xs text-base-content/60">{sub}</span>
                                            </span>
                                            <span class="shrink-0 text-right">
                                                <span class="block text-sm font-semibold">{ui::format_price(p.price_cents)}</span>
                                                <span class=format!("mk-stock {pill_class}")>{pill}</span>
                                            </span>
                                        </button>
                                    </li>
                                }
                            }
                        </For>
                        {move || shown.with(Vec::is_empty).then(|| view! {
                            <li class="p-6 text-center text-sm text-base-content/70">"Ningún producto coincide."</li>
                        })}
                    </ul>
                </section>
                // El panel rápido no se arma mientras la ficha completa está
                // abierta: los dos usan los mismos campos.
                {move || {
                    ctx.nonce.track();
                    let open = ctx.ficha.with(Option::is_some);
                    let p = (!open).then(|| current.get()).flatten().and_then(|id| {
                        products.with_untracked(|ps| ps.iter().find(|p| p.id == id).cloned())
                    });
                    p.map(|p| view! { <QuickPanel ctx product=p/> })
                }}
            </div>
        </Show>
        <FichaDialog ctx/>
    }
}

/// Panel rápido: lo que más se cambia del producto, sin salir de la lista.
#[component]
fn QuickPanel(ctx: ListCtx, product: Product) -> impl IntoView {
    let form = Form::new(&product);
    let id = StoredValue::new(product.id.clone());
    let name_now = StoredValue::new(product.name.clone());
    let errors = RwSignal::new(Vec::<String>::new());
    let saved = RwSignal::new(false);
    let store_tags = {
        let own: Vec<Product> = ctx
            .products
            .with_untracked(|ps| ps.iter().filter(|p| !p.example).cloned().collect());
        domain::count_tags(&own)
    };
    let store_categories = domain::order_categories(
        ctx.products
            .with_untracked(|ps| domain::count_categories(ps)),
        &ctx.order.get_untracked(),
        true,
    );
    ctx.panel_dirty.set(false);
    Effect::new(move |first: Option<()>| {
        form.name.track();
        form.price.track();
        form.free.track();
        form.was.track();
        form.description.track();
        form.tags.track();
        form.category.track();
        form.subcategory.track();
        form.in_stock.track();
        form.stock.track();
        form.visible.track();
        if first.is_some() {
            ctx.panel_dirty.set(true);
            saved.set(false);
        }
    });
    let save = Action::new(move |p: &Product| {
        let p = p.clone();
        async move {
            (
                save_product(ctx.slug.get_value(), id.get_value(), p.clone()).await,
                p,
            )
        }
    });
    Effect::new(move |_| match save.value().get() {
        Some((
            Ok(SaveResult {
                outcome: ProductSave::Saved,
                ..
            }),
            mut p,
        )) => {
            _ = domain::tidy_product(&mut p);
            errors.set(Vec::new());
            ctx.panel_dirty.set(false);
            saved.set(true);
            ctx.products.update(|ps| {
                if let Some(old) = ps.iter_mut().find(|x| x.id == p.id) {
                    *old = p;
                }
            });
        }
        Some((
            Ok(SaveResult {
                outcome: ProductSave::Invalid { errors: e },
                ..
            }),
            _,
        )) => errors.set(e),
        Some((Err(e), _)) => errors.set(vec![error_text(&e)]),
        None => {}
    });
    let on_save = move |_| match form.product(&id.get_value()) {
        Ok(p) => {
            errors.set(Vec::new());
            save.dispatch(p);
        }
        Err(e) => errors.set(e),
    };
    let discard = move |_| {
        ctx.panel_dirty.set(false);
        ctx.nonce.update(|n| *n += 1);
    };
    // «Duplicar»: copia lo ya guardado, oculta, y la deja abierta.
    let duplicate = Action::new(move |_: &()| {
        let source = ctx
            .products
            .with_untracked(|ps| ps.iter().find(|x| x.id == id.get_value()).cloned());
        async move {
            let Some(mut copy) = source else {
                return Err(ServerFnError::new("Ese producto ya no está en la lista."));
            };
            let suffix = " (copia)";
            let room = domain::PRODUCT_NAME_MAX - suffix.chars().count();
            copy.name = format!("{}{suffix}", copy.name.chars().take(room).collect::<String>());
            copy.visible = false;
            copy.example = false;
            let saved = save_product(ctx.slug.get_value(), String::new(), copy.clone()).await?;
            Ok((saved, copy))
        }
    });
    Effect::new(move |_| match duplicate.value().get() {
        Some(Ok((
            SaveResult {
                id: new_id,
                outcome: ProductSave::Saved,
            },
            mut copy,
        ))) => {
            copy.id = new_id.clone();
            ctx.products.update(|ps| ps.insert(0, copy));
            ctx.panel_dirty.set(false);
            ctx.selected.set(Some(new_id));
        }
        Some(Ok((
            SaveResult {
                outcome: ProductSave::Invalid { errors: e },
                ..
            },
            _,
        ))) => errors.set(e),
        Some(Err(e)) => errors.set(vec![error_text(&e)]),
        None => {}
    });
    let on_duplicate = move |_| {
        ctx.guard(ctx.panel_dirty, move || {
            duplicate.dispatch(());
        });
    };
    let position = move || {
        ctx.shown.with(|ps| {
            ps.iter()
                .position(|p| p.id == id.get_value())
                .map(|i| format!("{} de {}", i + 1, ps.len()))
                .unwrap_or_default()
        })
    };
    let prev = move || ctx.neighbor(&id.get_value(), -1);
    let next = move || ctx.neighbor(&id.get_value(), 1);
    let go = move |to: Option<String>| {
        if let Some(to) = to {
            ctx.select(to);
        }
    };
    let close_phone = move |_| {
        ctx.guard(ctx.panel_dirty, move || {
            ctx.panel_dirty.set(false);
            ctx.nonce.update(|n| *n += 1);
            ctx.phone_open.set(false);
        })
    };
    let ficha = move |_| ctx.open_ficha(id.get_value());
    let ask_delete = move |_| ctx.ask_delete.run((id.get_value(), name_now.get_value()));
    let price_error = move |field: &'static str| {
        move || {
            errors.with(|es| {
                es.iter()
                    .filter(|e| error_field(e) == field)
                    .map(|e| view! { <p class="mt-1 text-xs text-error">{e.clone()}</p> })
                    .collect_view()
            })
        }
    };
    let other_errors = move || {
        errors.with(|es| {
            es.iter()
                .filter(|e| !matches!(error_field(e), "price" | "was" | "stock"))
                .map(|e| view! { <p>{e.clone()}</p> })
                .collect_view()
        })
    };
    let visible_toggle = move || view! {
        <label class="flex cursor-pointer items-center justify-between gap-3 rounded-box border border-base-300 px-4 py-3">
            <span><span class="block font-semibold">"En la tienda"</span>
                <span class="block text-xs text-base-content/60">{move || if form.visible.get() { "Visible para los compradores" } else { "Oculto: no aparece en la tienda" }}</span></span>
            <input type="checkbox" class="toggle toggle-primary"
                prop:checked=move || form.visible.get()
                on:change=move |ev| form.visible.set(event_target_checked(&ev))/>
        </label>
    };
    let photos = product.all_photos();
    let fit_of = StoredValue::new(product.clone());
    let menu_id = format!("mk-pmenu-{}", product.id);
    let menu_id_dup = menu_id.clone();
    let store_href = format!("{}/?ver={}", ctx.store_url.get_value(), product.id);
    view! {
        <section class="mk-qpanel card bg-base-100 shadow-sm" class=("max-sm:hidden", move || !ctx.phone_open.get())
            aria-label=format!("Editar {}", product.name)>
            // Teléfono: volver, posición, anterior/siguiente y ⋯ (Borrar, lejos de Guardar).
            <div class="sticky top-0 z-10 flex items-center gap-2 border-b border-base-200 bg-base-100 px-3 py-2 sm:hidden">
                <button type="button" class="btn btn-ghost btn-sm min-h-11 px-2" on:click=close_phone>"‹ Productos"</button>
                <button type="button" class="btn btn-sm btn-square ml-auto min-h-11 min-w-11" aria-label="Anterior" disabled=move || prev().is_none() on:click=move |_| go(prev())>"‹"</button>
                <button type="button" class="btn btn-sm btn-square min-h-11 min-w-11" aria-label="Siguiente" disabled=move || next().is_none() on:click=move |_| go(next())>"›"</button>
                <button type="button" class="btn btn-sm btn-square min-h-11 min-w-11" aria-label="Más acciones" popovertarget=menu_id.clone()>"⋯"</button>
                <div id=menu_id popover="auto" class="mk-pmenu">
                    <a class="btn btn-ghost w-full justify-start" href=store_href.clone() target="_blank" rel="noopener">"Ver en la tienda"</a>
                    <button type="button" class="btn btn-ghost w-full justify-start border-t border-base-200" popovertarget=menu_id_dup popovertargetaction="hide" on:click=on_duplicate>"Duplicar producto"</button>
                    <button type="button" class="btn btn-ghost w-full justify-start border-t border-base-200 text-error" on:click=ask_delete>"Borrar producto…"</button>
                </div>
            </div>
            <div class="card-body gap-4 p-5 max-sm:p-4 max-sm:pb-40">
                <div class="flex items-center gap-2 max-sm:hidden">
                    <span class="text-sm text-base-content/60">{position}</span>
                    <span class="ml-auto text-sm text-base-content/60" role="status" aria-live="polite">
                        {move || if save.pending().get() { "Guardando…" } else if saved.get() { "Guardado ✓" } else { "" }}
                    </span>
                    <button type="button" class="btn btn-sm" disabled=move || prev().is_none() on:click=move |_| go(prev())>"‹ Anterior"</button>
                    <button type="button" class="btn btn-sm" disabled=move || next().is_none() on:click=move |_| go(next())>"Siguiente ›"</button>
                </div>
                <div class="grid gap-5 sm:grid-cols-[minmax(0,15rem)_1fr]">
                    <div class="flex min-w-0 flex-col gap-4">
                    // Fotos: tocarlas abre la ficha completa (ahí se suben y encuadran).
                    <div class="flex flex-col gap-2 max-sm:flex-row max-sm:overflow-x-auto">
                        <button type="button" class="aspect-square w-full overflow-hidden rounded-box bg-base-200 max-sm:w-32 max-sm:shrink-0"
                            aria-label="Cambiar las fotos en la ficha completa" on:click=ficha>
                            {photos.first().cloned().map(|u| view! { <ui::Image src=u.clone() alt="" fit=fit_of.with_value(|p| p.fit(&u))/> })}
                        </button>
                        <div class="grid grid-cols-4 gap-2 max-sm:flex">
                            {photos.iter().skip(1).take(3).cloned().map(|u| view! {
                                <button type="button" class="aspect-square overflow-hidden rounded-field bg-base-200 max-sm:size-32 max-sm:shrink-0" tabindex="-1" aria-hidden="true" on:click=ficha>
                                    <ui::Image src=u.clone() alt="" fit=fit_of.with_value(|p| p.fit(&u))/>
                                </button>
                            }).collect_view()}
                            <button type="button" class="grid aspect-square place-items-center rounded-field border border-dashed border-base-300 text-base-content/50 max-sm:size-32 max-sm:shrink-0"
                                aria-label="Agregar fotos en la ficha completa" on:click=ficha>"+"</button>
                        </div>
                    </div>
                    // Compu: debajo de las fotos. Teléfono: al final, antes de los botones.
                    <div class="max-sm:hidden">{visible_toggle()}</div>
                    </div>
                    <div class="flex min-w-0 flex-col gap-3">
                        <input class="input input-ghost w-full px-1 text-xl font-bold" aria-label="Nombre"
                            maxlength=domain::PRODUCT_NAME_MAX
                            prop:value=move || form.name.get()
                            on:input=move |ev| form.name.set(event_target_value(&ev))/>
                        <TagsField form store_tags/>
                        <div class="grid grid-cols-2 gap-3">
                            <label class="flex flex-col gap-1 text-xs text-base-content/70">"Precio"
                                <span class="input w-full"><span>"$"</span>
                                    <input inputmode="decimal" aria-label="Precio en pesos"
                                        prop:value=move || form.price.get()
                                        on:input=move |ev| form.price.set(event_target_value(&ev))/>
                                </span>
                                {price_error("price")}
                            </label>
                            <label class="flex flex-col gap-1 text-xs text-base-content/70"><span class="mk-tip-legend flex items-center">"Precio anterior"
                                <ui::HelpTip id="tip-q-was" label="Precio anterior" text="Opcional. Si está en oferta, se muestra tachado al lado del precio."/></span>
                                <span class="input w-full"><span>"$"</span>
                                    <input inputmode="decimal" aria-label="Precio anterior en pesos" placeholder="Opcional"
                                        prop:value=move || form.was.get()
                                        on:input=move |ev| form.was.set(event_target_value(&ev))/>
                                </span>
                                {price_error("was")}
                            </label>
                        </div>
                        <FreeSwitch form/>
                        <CategoryField form store_categories/>
                        <StockStepper form/>
                        {price_error("stock")}
                    </div>
                </div>
                <label class="flex flex-col gap-1 text-xs font-semibold uppercase tracking-wide text-base-content/70">"Descripción"
                    <textarea class="textarea h-32 w-full text-sm font-normal normal-case tracking-normal text-base-content" maxlength=domain::PRODUCT_DESC_MAX
                        prop:value=move || form.description.get()
                        on:input=move |ev| form.description.set(event_target_value(&ev))></textarea>
                </label>
                <div class="sm:hidden">{visible_toggle()}</div>
                <div role="alert" class="text-sm text-error empty:hidden">{other_errors}</div>
                // Compu: Ficha completa, Borrar al centro, Descartar y Guardar.
                // Teléfono: barra fija abajo, sin Borrar (está en ⋯).
                <div class="mk-qfoot">
                    <span class="text-right text-xs text-base-content/60 max-sm:hidden" class:invisible=move || !ctx.panel_dirty.get()>"Cambios sin guardar"</span>
                    <div class="mk-qfoot-row">
                        <span class="flex gap-1"><button type="button" class="btn max-sm:btn-ghost max-sm:px-1" on:click=ficha>"Ficha completa ›"</button>
                            <button type="button" class="btn btn-ghost max-sm:hidden" disabled=move || duplicate.pending().get() on:click=on_duplicate>"Duplicar"</button></span>
                        <span class="max-sm:hidden"><button type="button" class="btn btn-ghost btn-sm text-error" on:click=ask_delete>"Borrar"</button></span>
                        <span class="text-xs text-base-content/60 sm:hidden" class:invisible=move || !ctx.panel_dirty.get()>"Cambios sin guardar"</span>
                        <span class="flex justify-end gap-2">
                            <button type="button" class="btn max-sm:btn-ghost" disabled=move || !ctx.panel_dirty.get() on:click=discard>"Descartar"</button>
                            <button type="button" class="btn btn-primary max-sm:hidden" disabled=move || save.pending().get() on:click=on_save>"Guardar"</button>
                        </span>
                    </div>
                    <button type="button" class="btn btn-primary w-full sm:hidden" disabled=move || save.pending().get() on:click=on_save>
                        {move || if save.pending().get() { "Guardando…" } else { "Guardar" }}
                    </button>
                </div>
            </div>
        </section>
    }
}

/// Stock del panel rápido: − número + (con «unidades» debajo). Con colores o
/// talles, el total, que se cambia en la ficha completa.
#[component]
fn StockStepper(form: Form) -> impl IntoView {
    let count = move || {
        form.stock
            .with(|s| s.trim().replace('.', "").parse::<u32>().ok())
    };
    let set = move |n: u32| {
        form.stock.set(n.to_string());
        form.in_stock.set(n > 0);
    };
    view! {
        <div class="flex flex-col gap-1 text-xs text-base-content/70">
            <span class="mk-tip-legend flex items-center">"Stock"
                {move || form.variants.active().then(|| view! {
                    <ui::HelpTip id="tip-q-stock" label="Stock" text="Se cuenta por color y talle. Se cambia en Ficha completa."/>
                })}</span>
            {move || if form.variants.active() {
                view! {
                    <p class="rounded-box border border-base-300 px-4 py-3 text-sm text-base-content">
                        <b>{move || format!("{} unidades", form.variants.total())}</b>
                        <span class="text-base-content/60">" · por color y talle"</span>
                    </p>
                }.into_any()
            } else {
                view! {
                    <div class="flex items-center justify-center gap-4 rounded-box border border-base-300 px-3 py-2" role="group" aria-label="Stock">
                        <button type="button" class="btn btn-circle btn-soft" aria-label="Una unidad menos"
                            disabled=move || count().is_none_or(|n| n == 0)
                            on:click=move |_| if let Some(n) = count() { set(n.saturating_sub(1)); }>"−"</button>
                        <span class="flex min-w-16 flex-col items-center text-base-content">
                            <input class="w-16 bg-transparent text-center text-xl font-bold outline-none" inputmode="numeric"
                                aria-label="Unidades" placeholder="—"
                                prop:value=move || form.stock.get()
                                on:input=move |ev| {
                                    let v = event_target_value(&ev);
                                    if let Ok(n) = v.trim().parse::<u32>() { form.in_stock.set(n > 0); }
                                    form.stock.set(v);
                                }/>
                            <span class="text-xs text-base-content/60">
                                {move || match count() {
                                    Some(1) => "unidad",
                                    Some(_) => "unidades",
                                    None if form.in_stock.get() => "sin contar",
                                    None => "agotado",
                                }}
                            </span>
                        </span>
                        <button type="button" class="btn btn-circle btn-soft" aria-label="Una unidad más"
                            on:click=move |_| set(count().map_or(1, |n| n + 1))>"+"</button>
                    </div>
                }.into_any()
            }}
        </div>
    }
}

/// La ficha completa: todo el producto, en una ventana sobre la lista.
#[component]
fn FichaDialog(ctx: ListCtx) -> impl IntoView {
    let dialog = NodeRef::<leptos::html::Dialog>::new();
    // La ficha se arma recién en el navegador, ya hidratada la página: el
    // servidor no la manda (así no hay diferencias al hidratar).
    let mounted = RwSignal::new(false);
    Effect::new(move |_| mounted.set(true));
    // Solo en el navegador: la ficha se arma dentro de la lista, que ya
    // está dentro de un Suspense.
    let data = LocalResource::new(move || {
        let id = ctx.ficha_src.get();
        ctx.nonce.track();
        async move {
            match id {
                Some(id) => {
                    let key = if id.is_empty() {
                        "nuevo".to_string()
                    } else {
                        id
                    };
                    product_to_edit(ctx.slug.get_value(), key).await.map(Some)
                }
                None => Ok(None),
            }
        }
    });
    Effect::new(move |_| {
        let open = ctx.ficha.get();
        ctx.set_url(open.as_deref());
        if let Some(d) = dialog.get() {
            if open.is_some() && !d.open() {
                _ = d.show_modal();
            } else if open.is_none() && d.open() {
                d.close();
            }
        }
    });
    let close = move || {
        ctx.guard(ctx.ficha_dirty, move || {
            ctx.ficha_dirty.set(false);
            ctx.show_ficha(None);
        });
    };
    let step = move |dir: i32| {
        let Some(id) = ctx.ficha.get_untracked().filter(|id| !id.is_empty()) else {
            return;
        };
        if let Some(to) = ctx.neighbor(&id, dir) {
            ctx.guard(ctx.ficha_dirty, move || {
                ctx.ficha_dirty.set(false);
                ctx.selected.set(Some(to.clone()));
                ctx.show_ficha(Some(to.clone()));
            });
        }
    };
    let host = FichaHost {
        dirty: ctx.ficha_dirty,
        close: Callback::new(move |()| close()),
        prev: Callback::new(move |()| step(-1)),
        next: Callback::new(move |()| step(1)),
        has_prev: Signal::derive(move || {
            ctx.ficha
                .get()
                .filter(|i| !i.is_empty())
                .and_then(|i| ctx.neighbor(&i, -1))
                .is_some()
        }),
        has_next: Signal::derive(move || {
            ctx.ficha
                .get()
                .filter(|i| !i.is_empty())
                .and_then(|i| ctx.neighbor(&i, 1))
                .is_some()
        }),
        position: Signal::derive(move || {
            ctx.ficha
                .get()
                .filter(|i| !i.is_empty())
                .and_then(|id| {
                    ctx.shown.with(|ps| {
                        ps.iter()
                            .position(|p| p.id == id)
                            .map(|i| format!("{} de {}", i + 1, ps.len()))
                    })
                })
                .unwrap_or_default()
        }),
        saved: Callback::new(move |p: Product| {
            let id = p.id.clone();
            ctx.products
                .update(|ps| match ps.iter_mut().find(|x| x.id == id) {
                    Some(old) => *old = p,
                    None => ps.insert(0, p),
                });
            ctx.selected.set(Some(id.clone()));
            // El nuevo pasa a ser este (‹ ›, Borrar y la dirección), sin
            // volver a cargar la ficha.
            ctx.ficha.update(|f| {
                if f.as_deref() == Some("") {
                    *f = Some(id);
                }
            });
        }),
        delete: ctx.ask_delete,
        again: Callback::new(move |()| {
            ctx.ficha_dirty.set(false);
            ctx.show_ficha(Some(String::new()));
            ctx.nonce.update(|n| *n += 1);
        }),
        discard: Callback::new(move |()| {
            ctx.ficha_dirty.set(false);
            ctx.nonce.update(|n| *n += 1);
        }),
    };
    view! {
        <dialog node_ref=dialog class="modal mk-ficha" aria-label="Ficha completa del producto"
            on:cancel=move |ev: leptos::ev::Event| { ev.prevent_default(); close(); }>
            <div class="modal-box flex h-[calc(100dvh-2rem)] w-[min(72rem,calc(100vw-2rem))] max-w-none flex-col gap-0 p-0 max-sm:h-dvh max-sm:max-h-none max-sm:w-full max-sm:rounded-none">
                {move || (mounted.get() && ctx.ficha_src.with(Option::is_some)).then(|| view! {
                <Transition fallback=|| view! { <ui::PanelSkeleton kind=ui::PanelSkeletonKind::Product/> }>
                    {move || Suspend::new(async move {
                        match data.await {
                            Ok(Some(Some(edit))) => {
                                let id = ctx.ficha_src.get_untracked().unwrap_or_default();
                                view! { <ProductForm slug=ctx.slug.get_value() id edit host/> }.into_any()
                            }
                            Ok(Some(None)) => view! { <SignInFirst back=format!("/tienda/{}/productos", ctx.slug.get_value())/> }.into_any(),
                            Ok(None) => ().into_any(),
                            Err(e) => view! {
                                <div class="flex flex-col items-start gap-3 p-6">
                                    <p role="alert" class="text-sm text-error">{error_text(&e)}</p>
                                    <button type="button" class="btn" on:click=move |_| { ctx.ficha_dirty.set(false); ctx.show_ficha(None); }>"Cerrar"</button>
                                </div>
                            }.into_any(),
                        }
                    })}
                </Transition>
                })}
            </div>
        </dialog>
    }
}

/// Lo que la ficha completa le avisa a la lista (o le pide) cuando se abre
/// en ventana.
#[derive(Clone, Copy)]
struct FichaHost {
    dirty: RwSignal<bool>,
    close: Callback<()>,
    prev: Callback<()>,
    next: Callback<()>,
    has_prev: Signal<bool>,
    has_next: Signal<bool>,
    position: Signal<String>,
    saved: Callback<Product>,
    /// Borrar (id, nombre): pregunta la lista.
    delete: Callback<(String, String)>,
    /// «+ Cargar otro producto»: la ficha vacía.
    again: Callback<()>,
    discard: Callback<()>,
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

/// «Categorías»: las de la tienda con sus subcategorías, para cambiarles el
/// nombre o sacarlas. Se crean al ponerlas en un producto.
#[component]
fn CategoryManager(
    slug: String,
    products: RwSignal<Vec<Product>>,
    /// Orden elegido por el dueño (y las categorías nuevas aún sin productos).
    order: RwSignal<Vec<String>>,
    dialog: NodeRef<leptos::html::Dialog>,
) -> impl IntoView {
    let slug = StoredValue::new(slug);
    let cats = Memo::new(move |_| {
        let counted = products.with(|ps| domain::count_categories(ps));
        order.with(|o| domain::order_categories(counted, o, true))
    });
    // La que se está renombrando o por borrar: (categoría, subcategoría).
    let editing = RwSignal::new(None::<(String, String)>);
    let doomed = RwSignal::new(None::<(String, String)>);
    let name = RwSignal::new(String::new());
    let fresh = RwSignal::new(String::new());
    let problem = RwSignal::new(None::<String>);
    let add_problem = RwSignal::new(None::<String>);
    // Índice de la categoría que se está arrastrando.
    let dragging = RwSignal::new(None::<usize>);
    let done = move |r: Result<domain::TagSave, ServerFnError>| match r {
        Ok(domain::TagSave::Saved { products: ps }) => {
            // El servidor ya cambió el orden guardado; acá se hace lo mismo.
            if let Some((c, _)) = editing.get_untracked().filter(|(_, s)| s.is_empty()) {
                let n = domain::clean_category(&name.get_untracked());
                order.update(|o| {
                    o.iter_mut()
                        .filter(|x| **x == c)
                        .for_each(|x| x.clone_from(&n))
                });
            }
            if let Some((c, _)) = doomed.get_untracked().filter(|(_, s)| s.is_empty()) {
                order.update(|o| o.retain(|x| *x != c));
            }
            products.set(ps);
            problem.set(None);
            editing.set(None);
            doomed.set(None);
        }
        Ok(domain::TagSave::Invalid { error }) => problem.set(Some(error)),
        Err(e) => problem.set(Some(error_text(&e))),
    };
    let rename = Action::new(move |_: &()| {
        let (c, s) = editing.get_untracked().unwrap_or_default();
        let n = name.get_untracked();
        async move { done(rename_category(slug.get_value(), c, s, n).await) }
    });
    let remove = Action::new(move |_: &()| {
        let (c, s) = doomed.get_untracked().unwrap_or_default();
        async move { done(delete_category(slug.get_value(), c, s).await) }
    });
    // Guarda el orden; `adding` dice si el error va bajo el campo de nueva.
    let save = Action::new(move |(names, adding): &(Vec<String>, bool)| {
        let (names, adding) = (names.clone(), *adding);
        let before = order.get_untracked();
        order.set(names.clone());
        async move {
            let shown = if adding { add_problem } else { problem };
            match save_category_order(slug.get_value(), names).await {
                Ok(Ok(clean)) => {
                    order.set(clean);
                    shown.set(None);
                    if adding {
                        fresh.set(String::new());
                    }
                }
                Ok(Err(e)) => {
                    order.set(before);
                    shown.set(Some(e));
                }
                Err(e) => {
                    order.set(before);
                    shown.set(Some(error_text(&e)));
                }
            }
        }
    });
    let busy = move || rename.pending().get() || remove.pending().get() || save.pending().get();
    let names_now =
        move || cats.with_untracked(|cs| cs.iter().map(|c| c.name.clone()).collect::<Vec<_>>());
    let move_to = move |from: usize, to: usize| {
        let mut names = names_now();
        if from >= names.len() || to >= names.len() || from == to {
            return;
        }
        let n = names.remove(from);
        names.insert(to, n);
        save.dispatch((names, false));
    };
    let add = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        let n = domain::clean_category(&fresh.get_untracked());
        if n.is_empty() {
            add_problem.set(Some("Escribí un nombre.".into()));
            return;
        }
        let mut names = names_now();
        if names.iter().any(|c| c.to_lowercase() == n.to_lowercase()) {
            add_problem.set(Some("Ya hay una categoría con ese nombre.".into()));
            return;
        }
        names.push(n);
        save.dispatch((names, true));
    };
    let close = move |_| {
        if let Some(d) = dialog.get() {
            d.close();
        }
    };
    let row = move |label: String, cat: String, sub: String, n: usize, at: Option<usize>| {
        let key = (cat, sub);
        let (k_edit, k_doom, k_start, k_ask) = (key.clone(), key.clone(), key.clone(), key.clone());
        let is_sub = !key.1.is_empty();
        let label2 = label.clone();
        let label3 = label.clone();
        view! {
            <li class=if is_sub { "ml-12 border-b border-base-200 py-1.5 last:border-0" } else { "border-b border-base-200 py-2 last:border-0" }>
                {move || if editing.get().as_ref() == Some(&k_edit) {
                    view! {
                        <form class="flex flex-wrap items-center gap-2" on:submit=move |ev| { ev.prevent_default(); rename.dispatch(()); }>
                            <input class="input input-sm grow" maxlength=domain::CATEGORY_MAX required=true
                                aria-label="Nombre nuevo"
                                prop:value=move || name.get()
                                on:input=move |ev| name.set(event_target_value(&ev))/>
                            <button type="submit" class="btn btn-primary btn-sm" disabled=busy>"Guardar"</button>
                            <button type="button" class="btn btn-ghost btn-sm" on:click=move |_| editing.set(None)>"Cancelar"</button>
                        </form>
                    }.into_any()
                } else if doomed.get().as_ref() == Some(&k_doom) {
                    view! {
                        <div role="alert" class="alert alert-warning flex flex-wrap text-sm">
                            <span class="grow">{if n == 0 {
                                format!("¿Sacar «{label2}»?")
                            } else {
                                format!("¿Sacar «{}»? Los productos no se borran: quedan {}.", label2, if is_sub { "sin subcategoría" } else { "sin categoría" })
                            }}</span>
                            <button type="button" class="btn btn-sm" on:click=move |_| doomed.set(None)>"Cancelar"</button>
                            <button type="button" class="btn btn-error btn-sm" disabled=busy on:click=move |_| { remove.dispatch(()); }>"Sacar"</button>
                        </div>
                    }.into_any()
                } else {
                    let (ks, ka) = (k_start.clone(), k_ask.clone());
                    let l = label3.clone();
                    let l2 = label3.clone();
                    view! {
                        <div class="flex items-center gap-2">
                            {at.map(|i| view! {
                                <button type="button" draggable="true"
                                    class="btn btn-ghost btn-xs btn-square cursor-grab text-base-content/50 active:cursor-grabbing"
                                    aria-label=format!("Mover {l2}. Usá las flechas arriba y abajo")
                                    title="Arrastrá para ordenar"
                                    disabled=busy
                                    on:keydown=move |ev| match ev.key().as_str() {
                                        "ArrowUp" if i > 0 => { ev.prevent_default(); move_to(i, i - 1); }
                                        "ArrowDown" => { ev.prevent_default(); move_to(i, i + 1); }
                                        _ => {}
                                    }
                                    on:dragstart=move |ev| {
                                        use leptos::wasm_bindgen::JsCast;
                                        if let Some(dt) = ev.data_transfer() {
                                            _ = dt.set_data("text/plain", &i.to_string());
                                            dt.set_effect_allowed("move");
                                            if let Some(li) = ev.target()
                                                .and_then(|t| t.dyn_into::<leptos::web_sys::Element>().ok())
                                                .and_then(|e| e.closest("li").ok().flatten())
                                            {
                                                dt.set_drag_image(&li, 16, 16);
                                            }
                                        }
                                        dragging.set(Some(i));
                                    }
                                    on:dragend=move |_| dragging.set(None)>
                                    "⠿"
                                </button>
                            })}
                            <span class="grow truncate">{label3.clone()}</span>
                            <span class="text-sm text-base-content/60">{n}</span>
                            <button type="button" class="btn btn-ghost btn-xs" aria-label=format!("Cambiar el nombre de {l}")
                                on:click=move |_| { name.set(if ks.1.is_empty() { ks.0.clone() } else { ks.1.clone() }); problem.set(None); doomed.set(None); editing.set(Some(ks.clone())); }>"Renombrar"</button>
                            <button type="button" class="btn btn-ghost btn-xs text-error" aria-label=format!("Sacar {l}")
                                on:click=move |_| { problem.set(None); editing.set(None); doomed.set(Some(ka.clone())); }>"Sacar"</button>
                        </div>
                    }.into_any()
                }}
            </li>
        }
    };
    view! {
        <div class="flex items-start gap-3">
            <div class="grow">
                <h2 id="categorias-titulo" class="text-xl font-bold">"Categorías"</h2>
                <p class="text-sm text-base-content/70">"Arrastrá ⠿ para elegir en qué orden las ven tus clientes. Cada producto tiene una categoría y, si querés, una subcategoría."</p>
            </div>
            <button type="button" class="btn btn-ghost btn-sm btn-square" aria-label="Cerrar" on:click=close>"✕"</button>
        </div>
        <form class="mt-4 flex items-start gap-2" on:submit=add>
            <div class="grow">
                <input class="input input-sm w-full" maxlength=domain::CATEGORY_MAX
                    placeholder="Nueva categoría" aria-label="Nombre de la categoría nueva"
                    aria-invalid=move || add_problem.with(Option::is_some).then_some("true")
                    prop:value=move || fresh.get()
                    on:input=move |ev| { fresh.set(event_target_value(&ev)); add_problem.set(None); }/>
                {move || add_problem.get().map(|e| view! { <p class="mt-1 text-sm text-error">{e}</p> })}
            </div>
            <button type="submit" class="btn btn-sm" disabled=busy>"+ Agregar"</button>
        </form>
        <ul class="mt-3 max-h-96 overflow-y-auto" aria-label="Categorías de la tienda">
            {move || cats.get().into_iter().enumerate().map(|(i, c)| {
                let over = RwSignal::new(false);
                view! {
                    <li class="rounded-field transition-colors"
                        class:bg-base-200=move || over.get()
                        class:opacity-40=move || dragging.get() == Some(i)
                        on:dragover=move |ev| {
                            if dragging.get_untracked().is_some() {
                                ev.prevent_default();
                                over.set(true);
                            }
                        }
                        on:dragleave=move |_| over.set(false)
                        on:drop=move |ev| {
                            ev.prevent_default();
                            over.set(false);
                            if let Some(from) = dragging.get_untracked() {
                                dragging.set(None);
                                move_to(from, i);
                            }
                        }>
                        <ul>
                            {row(c.name.clone(), c.name.clone(), String::new(), c.products, Some(i))}
                            {c.subs.iter().map(|(s, n)| row(s.clone(), c.name.clone(), s.clone(), *n, None)).collect_view()}
                        </ul>
                    </li>
                }
            }).collect_view()}
        </ul>
        {move || cats.with(Vec::is_empty).then(|| view! {
            <p class="mt-2 text-sm text-base-content/60">"Todavía no hay categorías. Agregá una arriba o elegila al cargar un producto."</p>
        })}
        {move || problem.get().map(|e| view! { <p role="alert" class="mt-3 text-sm text-error">{e}</p> })}
    }
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
    category: RwSignal<String>,
    subcategory: RwSignal<String>,
    in_stock: RwSignal<bool>,
    /// Cantidad disponible; vacío si no se cuenta.
    stock: RwSignal<String>,
    visible: RwSignal<bool>,
    /// «Producto gratis»: permite el precio $ 0.
    free: RwSignal<bool>,
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
            free: RwSignal::new(p.free),
            price: RwSignal::new(if p.id.is_empty() && p.price_cents == 0 && !p.free {
                String::new()
            } else {
                domain::price_input(p.price_cents)
            }),
            was: RwSignal::new(money(p.compare_at_cents)),
            description: RwSignal::new(p.description.clone()),
            photos: RwSignal::new(p.all_photos()),
            fits: RwSignal::new(p.photo_fits.clone()),
            tags: RwSignal::new(p.tags.clone()),
            category: RwSignal::new(p.category.clone()),
            subcategory: RwSignal::new(p.subcategory.clone()),
            in_stock: RwSignal::new(p.in_stock || p.stock.is_some()),
            stock: RwSignal::new(p.stock.map(|s| s.to_string()).unwrap_or_default()),
            visible: RwSignal::new(p.visible),
            weight: RwSignal::new(
                p.parcel
                    .map(|x| domain::kg_input(x.weight_g))
                    .unwrap_or_default(),
            ),
            length: RwSignal::new(
                p.parcel
                    .map(|x| domain::cm_input(x.length_mm))
                    .unwrap_or_default(),
            ),
            width: RwSignal::new(
                p.parcel
                    .map(|x| domain::cm_input(x.width_mm))
                    .unwrap_or_default(),
            ),
            height: RwSignal::new(
                p.parcel
                    .map(|x| domain::cm_input(x.height_mm))
                    .unwrap_or_default(),
            ),
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
        let price_cents = if price.trim().is_empty() && self.free.get() {
            0
        } else if price.trim().is_empty() {
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
        let stock = if variants.is_empty() {
            stock
        } else {
            Some(variants.total())
        };
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
            category: self.category.get(),
            subcategory: self.subcategory.get(),
            photo_fits: self.fits.get(),
            example: false,
            parcel,
            variants,
            free: self.free.get(),
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
fn ProductForm(slug: String, id: String, edit: ProductEdit, host: FichaHost) -> impl IntoView {
    let form = Form::new(&edit.product);
    let id = RwSignal::new(id);
    let slug = StoredValue::new(slug);
    let store_tags = StoredValue::new(edit.store_tags);
    let store_categories = StoredValue::new(edit.store_categories);
    // Peso y medidas solo sirven con Andreani (o si ya estaban cargados).
    let show_parcel = edit.andreani || edit.product.parcel.is_some();
    let errors = RwSignal::new(Vec::<String>::new());
    let saved = RwSignal::new(false);
    let dirty = host.dirty;
    dirty.set(false);
    // El nombre del producto recién creado: muestra qué hacer después.
    let created = RwSignal::new(None::<(String, bool)>);
    let store_url = edit.store_url.clone();
    let published = edit.published;
    // Cualquier cambio en el formulario saca el "Guardado".
    Effect::new(move |first: Option<()>| {
        form.name.track();
        form.price.track();
        form.free.track();
        form.was.track();
        form.description.track();
        form.photos.track();
        form.fits.track();
        form.tags.track();
        form.category.track();
        form.subcategory.track();
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

    // Cerrar la pestaña o recargar con cambios sin guardar: el navegador avisa.
    #[cfg(feature = "hydrate")]
    {
        let handle = window_event_listener(leptos::ev::beforeunload, move |ev| {
            if dirty.get_untracked() {
                ev.prevent_default();
                ev.set_return_value("");
            }
        });
        on_cleanup(move || handle.remove());
    }

    let save = Action::new(move |p: &Product| {
        let p = p.clone();
        async move {
            (
                save_product(slug.get_value(), id.get_untracked(), p.clone()).await,
                p,
            )
        }
    });
    Effect::new(move |_| match save.value().get() {
        Some((
            Ok(SaveResult {
                id: new_id,
                outcome: ProductSave::Saved,
            }),
            mut p,
        )) => {
            errors.set(Vec::new());
            dirty.set(false);
            saved.set(true);
            if id.get_untracked().is_empty() {
                created.set(Some((
                    form.name.get_untracked().trim().to_string(),
                    form.visible.get_untracked(),
                )));
                id.set(new_id.clone());
            }
            _ = domain::tidy_product(&mut p);
            p.id = new_id;
            host.saved.run(p);
        }
        Some((
            Ok(SaveResult {
                outcome: ProductSave::Invalid { errors: e },
                ..
            }),
            _,
        )) => {
            focus_first_error(&e);
            errors.set(e);
        }
        Some((Err(e), _)) => errors.set(vec![error_text(&e)]),
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
                    .map(
                        |e| view! { <p class="label whitespace-normal text-error">{e.clone()}</p> },
                    )
                    .collect_view()
            })
        }
    };
    let has_error =
        move |field: &'static str| errors.with(|es| es.iter().any(|e| error_field(e) == field));
    // Al salir de un precio: si se entiende, se reescribe como se va a leer
    // (1500.50 → 1.500,50) para que no haya sorpresas; si no, el error va
    // debajo del campo sin esperar a "Guardar".
    let check_price = move |value: RwSignal<String>, campo: &'static str, field: &'static str| {
        let text = value.get_untracked();
        let problem = if text.trim().is_empty() {
            None
        } else if let Some(cents) = domain::parse_price(&text) {
            value.set(domain::price_input(cents));
            None
        } else {
            domain::price_problem(&text, campo)
        };
        errors.update(|es| {
            es.retain(|e| error_field(e) != field);
            es.extend(problem);
        });
    };
    let save_label = move || {
        if save.pending().get() {
            "Guardando…"
        } else if id.get().is_empty() {
            "Crear producto"
        } else {
            "Guardar"
        }
    };
    let ask_delete = move |_| {
        host.delete
            .run((id.get_untracked(), form.name.get_untracked()))
    };
    let menu_id = format!("mk-fmenu-{}", id.get_untracked());

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
        // Arriba: el nombre, la posición en la lista, ‹ › y cerrar.
        <header class="flex items-center gap-2 border-b border-base-200 px-5 py-3 max-sm:px-3">
            <button type="button" class="btn btn-ghost btn-sm btn-square min-h-11 min-w-11 sm:hidden" aria-label="Cerrar la ficha" on:click=move |_| host.close.run(())>"✕"</button>
            <div class="min-w-0 grow">
                <p class="text-xs font-semibold uppercase tracking-wide text-base-content/60 max-sm:hidden">"Ficha completa"</p>
                <h2 class="truncate text-xl font-bold max-sm:text-base">{title}</h2>
            </div>
            <span class="text-sm text-base-content/60 max-sm:hidden" role="status" aria-live="polite">
                {move || if save.pending().get() { "Guardando…" } else if saved.get() && !dirty.get() { "Guardado ✓" } else { "" }}
            </span>
            <span class="text-xs text-base-content/60 max-sm:hidden">{move || host.position.get()}</span>
            <button type="button" class="btn btn-sm btn-square min-h-11 min-w-11 sm:min-h-8 sm:min-w-8" aria-label="Producto anterior" disabled=move || !host.has_prev.get() on:click=move |_| host.prev.run(())>"‹"</button>
            <button type="button" class="btn btn-sm btn-square min-h-11 min-w-11 sm:min-h-8 sm:min-w-8" aria-label="Producto siguiente" disabled=move || !host.has_next.get() on:click=move |_| host.next.run(())>"›"</button>
            // En el teléfono Borrar va en ⋯, lejos de Guardar.
            {move || (!id.get().is_empty()).then(|| {
                let menu_id = menu_id.clone();
                view! {
                    <button type="button" class="btn btn-sm btn-square min-h-11 min-w-11 sm:hidden" aria-label="Más acciones" popovertarget=menu_id.clone()>"⋯"</button>
                    <div id=menu_id popover="auto" class="mk-pmenu">
                        <button type="button" class="btn btn-ghost w-full justify-start text-error" on:click=ask_delete>"Borrar producto…"</button>
                    </div>
                }
            })}
            <button type="button" class="btn btn-ghost btn-sm btn-square max-sm:hidden" aria-label="Cerrar la ficha" on:click=move |_| host.close.run(())>"✕"</button>
        </header>
        <div class="flex min-h-0 grow flex-col gap-5 overflow-y-auto overscroll-contain px-5 py-4 max-sm:px-4">
        {move || created.get().filter(|_| !dirty.get()).map(|(name, shown)| {
            let store = store_url.clone();
            view! {
                <div role="status" class="flex flex-wrap items-center gap-3 rounded-box border border-emerald-300 bg-emerald-50 px-4 py-3 text-emerald-900">
                    <span class="grid size-7 shrink-0 place-items-center rounded-full bg-success text-success-content"><ui::Icon class="size-[18px]" name="check"/></span>
                    <p class="min-w-0 grow"><b>{format!("Creaste «{name}».")}</b>{if !shown { " Está oculto: tus clientes no lo ven hasta que lo muestres." } else if published { " Ya está en tu tienda." } else { " Se va a ver cuando publiques la tienda." }}</p>
                    <button type="button" class="btn btn-primary btn-sm" on:click=move |_| host.again.run(())>"+ Cargar otro producto"</button>
                    <a class="btn btn-outline btn-sm" href=store target="_blank" rel="noopener">
                        "Ver en mi tienda"<ui::Icon class="size-4" name="external"/><span class="sr-only">" (se abre en otra pestaña)"</span>
                    </a>
                    <button type="button" class="btn btn-ghost btn-sm" on:click=move |_| host.close.run(())>"Volver a la lista"</button>
                </div>
            }
        })}
        // Solo lo que no es de un campo (la conexión, por ejemplo); lo demás
        // va debajo de cada campo.
        <div role="alert" class="text-sm text-error empty:hidden">
            {move || errors.with(|es| es.iter().filter(|e| error_field(e) == "general").map(|e| view! { <p>{e.clone()}</p> }).collect_view())}
        </div>
        <div class="grid items-start gap-6 lg:grid-cols-[1fr_340px]">
            <div><div class="flex flex-col gap-5">
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
                                on:input=move |ev| form.price.set(event_target_value(&ev))
                                on:blur=move |_| check_price(form.price, "El precio", "price")/>
                        </label>
                        {field_error("price")}
                        <FreeSwitch form/>
                    </fieldset>
                    <fieldset class="fieldset">
                        <legend class="fieldset-legend mk-tip-legend">"Precio anterior " <span class="font-normal text-base-content/60">"(opcional)"</span>
                            <ui::HelpTip id="tip-was" label="Precio anterior" text="Si está en oferta, se muestra tachado al lado del precio. Vacío si no."/></legend>
                        <label class="input w-full" class:input-error=move || has_error("was")><span>"$"</span>
                            <input id="p-was" inputmode="decimal" placeholder="Vacío si no está en oferta" aria-label="Precio anterior en pesos"
                                aria-invalid=move || has_error("was").to_string()
                                prop:value=move || form.was.get()
                                on:input=move |ev| form.was.set(event_target_value(&ev))
                                on:blur=move |_| check_price(form.was, "El precio anterior", "was")/>
                        </label>
                        {field_error("was")}
                    </fieldset>
                </div>
                <PhotosField photos=form.photos fits=form.fits adjusting/>
                {field_error("photos")}
                <fieldset class="fieldset">
                    <legend class="fieldset-legend w-full">"Descripción"
                        <SheetPreview slug=slug.get_value() product=Signal::derive(preview)/></legend>
                    <textarea id="p-description" class="textarea h-28 w-full" maxlength=domain::PRODUCT_DESC_MAX
                        placeholder="Qué es, de qué está hecho, medidas, cómo se usa…"
                        prop:value=move || form.description.get()
                        on:input=move |ev| form.description.set(event_target_value(&ev))></textarea>
                    <p class="label">{move || format!("{} de {} letras", form.description.with(|d| d.chars().count()), domain::PRODUCT_DESC_MAX)}</p>
                    {field_error("description")}
                </fieldset>
                <CategoryField form store_categories=store_categories.get_value()/>
                <TagsField form store_tags=store_tags.get_value()/>
                <crate::variants_field::VariantsField form=form.variants photos=form.photos/>
                <div class="grid gap-4 sm:grid-cols-2">
                    {move || form.variants.active().then(|| view! {
                        <fieldset class="fieldset">
                            <legend class="fieldset-legend mk-tip-legend">"Stock"
                                <ui::HelpTip id="tip-stock-variants" label="Stock" text="Se cuenta por color y talle. Cada venta descuenta de la combinación elegida."/></legend>
                            <p class="text-sm">{move || format!("{} unidades en total", form.variants.total())}</p>
                            {field_error("stock")}
                        </fieldset>
                    })}
                    <fieldset class="fieldset" class:hidden=move || form.variants.active()>
                        <legend class="fieldset-legend mk-tip-legend">"Stock"
                            <ui::HelpTip id="p-stock-hint" label="Stock" text=Signal::derive(move || if form.stock.with(|s| s.trim().is_empty()) {
                                "Dejala vacía si no querés llevar la cuenta. Sin stock se ve \"Agotado\".".to_string()
                            } else {
                                "Cada venta descuenta. En 0 se ve \"Agotado\"; con 3 o menos, \"Quedan N\".".to_string()
                            })/></legend>
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
                        {field_error("stock")}
                    </fieldset>
                    <fieldset class="fieldset">
                        <legend class="fieldset-legend mk-tip-legend">"En la tienda"
                            <ui::HelpTip id="tip-visible" label="En la tienda" text="Apagalo para prepararlo sin que lo vean."/></legend>
                        <label class="flex items-center gap-3">
                            <input type="checkbox" class="toggle toggle-primary"
                                prop:checked=move || form.visible.get()
                                on:change=move |ev| form.visible.set(event_target_checked(&ev))/>
                            {move || if form.visible.get() { "Visible" } else { "Oculto" }}
                        </label>
                    </fieldset>
                </div>
                <fieldset class="fieldset" aria-describedby="p-parcel-hint" class:hidden=!show_parcel>
                    <legend class="fieldset-legend mk-tip-legend">"Envío " <span class="font-normal text-base-content/60">"(opcional)"</span>
                        <ui::HelpTip id="p-parcel-hint" label="Envío" text="Peso y medidas del producto ya empaquetado, para cotizar con Andreani. Vacío: se usa el paquete de tu tienda (en Entregas)."/></legend>
                    <div class="grid grid-cols-2 gap-2 sm:grid-cols-4">
                        <crate::sales::ParcelInput id="p-weight" label="Peso" unit="kg" value=form.weight invalid=Signal::derive(move || has_error("parcel"))/>
                        <crate::sales::ParcelInput id="p-length" label="Largo" unit="cm" value=form.length invalid=Signal::derive(move || has_error("parcel"))/>
                        <crate::sales::ParcelInput id="p-width" label="Ancho" unit="cm" value=form.width invalid=Signal::derive(move || has_error("parcel"))/>
                        <crate::sales::ParcelInput id="p-height" label="Alto" unit="cm" value=form.height invalid=Signal::derive(move || has_error("parcel"))/>
                    </div>
                    {field_error("parcel")}
                </fieldset>
            </div></div>
            <aside class="flex flex-col gap-3 lg:sticky lg:top-0">
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
        </div>
        // Abajo: Cambios sin guardar | Borrar | Descartar y Guardar. En el
        // teléfono, Guardar a todo el ancho y Borrar en ⋯ (arriba).
        <footer class="mk-qfoot border-t border-base-200 px-5 py-3 max-sm:px-4">
            <div class="mk-qfoot-row">
                <span class="flex items-center gap-3 text-sm">
                    <span class="text-base-content/60" class:invisible=move || !dirty.get()>"Cambios sin guardar"</span>
                    {move || {
                        let n = errors.with(Vec::len);
                        (n > 0).then(|| view! {
                            <span class="text-error max-sm:hidden">{if n == 1 { "Revisá el campo marcado.".to_string() } else { format!("Revisá los {n} campos marcados.") }}</span>
                        })
                    }}
                </span>
                <span class="max-sm:hidden">
                    {move || (!id.get().is_empty()).then(|| view! {
                        <button type="button" class="btn btn-ghost btn-sm text-error" on:click=ask_delete>"Borrar"</button>
                    })}
                </span>
                <span class="flex justify-end gap-2">
                    <button type="button" class="btn max-sm:btn-ghost" disabled=move || !dirty.get() on:click=move |_| host.discard.run(())>"Descartar"</button>
                    <button type="button" class="btn btn-primary max-sm:hidden" disabled=move || save.pending().get() on:click=move |_| on_save()>{save_label}</button>
                </span>
            </div>
            <button type="button" class="btn btn-primary w-full sm:hidden" disabled=move || save.pending().get() on:click=move |_| on_save()>{save_label}</button>
        </footer>
    }
}

/// «Ver en la tienda» junto a Descripción: abre, sobre la ficha, la ficha
/// del producto como la ven los compradores (con el diseño y los colores de
/// la tienda), al día con lo que se está escribiendo. Se cierra tocando
/// afuera o con Esc.
#[component]
fn SheetPreview(slug: String, product: Signal<Option<Product>>) -> impl IntoView {
    let wanted = RwSignal::new(false);
    let pos = RwSignal::new(String::new());
    let btn = NodeRef::<leptos::html::Button>::new();
    let slug = StoredValue::new(slug);
    let look = LocalResource::new(move || {
        let go = wanted.get();
        async move {
            if go {
                Some(store_sheet_look(slug.get_value()).await)
            } else {
                None
            }
        }
    });
    // Debajo del botón, alineada a su derecha y sin salirse de la pantalla.
    let place = move |_| {
        wanted.set(true);
        let Some(b) = btn.get_untracked() else { return };
        let r = b.get_bounding_client_rect();
        let w = leptos::web_sys::window();
        let vw = w.as_ref().and_then(|w| w.inner_width().ok()).and_then(|v| v.as_f64()).unwrap_or(1024.0);
        let vh = w.as_ref().and_then(|w| w.inner_height().ok()).and_then(|v| v.as_f64()).unwrap_or(768.0);
        if vw < 640.0 {
            pos.set(String::new());
            return;
        }
        let right = (vw - r.right()).max(16.0);
        let below = vh - r.bottom() - 16.0;
        pos.set(if below >= 360.0 {
            format!("top:{}px;right:{right}px;max-height:{}px", r.bottom() + 8.0, below - 8.0)
        } else {
            format!("bottom:{}px;right:{right}px;max-height:{}px", vh - r.top() + 8.0, r.top() - 24.0)
        });
    };
    view! {
        <button type="button" node_ref=btn class="btn btn-ghost btn-xs ml-auto gap-1 font-normal text-primary"
            popovertarget="mk-sheetpop" on:click=place>
            <ui::Icon name="eye" class="size-4"/>"Ver en la tienda"
        </button>
        <div id="mk-sheetpop" popover="auto" class="mk-sheetpop" style=move || pos.get()
            role="dialog" aria-label="Así se ve la ficha en tu tienda">
            <div class="flex items-center gap-2 border-b border-base-200 px-4 py-2 text-sm">
                <span class="grow font-medium">"Así se ve en tu tienda"</span>
                <button type="button" class="btn btn-ghost btn-sm btn-circle" aria-label="Cerrar"
                    popovertarget="mk-sheetpop" popovertargetaction="hide">"✕"</button>
            </div>
            {move || match look.get() {
                Some(Some(Ok(l))) => {
                    let SheetLook { theme, style, modal, store } = l;
                    provide_context(store);
                    view! {
                        <div data-theme=theme class="mk-store bg-base-100 text-base-content" style=style>
                            <div class="mk-dialog-box mk-sheet">
                                {move || match product.get() {
                                    Some(_) => view! { <widgets::ProductSheetPreview modal=modal.clone() product/> }.into_any(),
                                    None => view! { <p class="p-6 text-center text-sm text-base-content/60">"Poné un precio para ver cómo queda."</p> }.into_any(),
                                }}
                            </div>
                        </div>
                    }.into_any()
                }
                Some(Some(Err(e))) => view! { <p class="p-4 text-sm text-error">{error_text(&e)}</p> }.into_any(),
                _ => view! {
                    <div class="flex flex-col gap-3 p-4" aria-busy="true">
                        <div class="grid grid-cols-2 gap-4">
                            <ui::Skeleton class="aspect-square w-full"/>
                            <div class="flex flex-col gap-3"><ui::Skeleton class="h-6 w-3/4"/><ui::Skeleton class="h-5 w-1/3"/><ui::Skeleton class="h-20 w-full"/><ui::Skeleton class="h-10 w-full"/></div>
                        </div>
                    </div>
                }.into_any(),
            }}
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
            <legend class="fieldset-legend mk-tip-legend">"Fotos"
                <ui::HelpTip id="tip-photos" label="Fotos" text="La primera es la principal. Tocá una foto para encuadrarla."/></legend>
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
            {move || {
                let e = library.upload_errors.get();
                (!e.is_empty()).then(|| view! {
                    <ul role="alert" class="mt-1 flex flex-col gap-0.5 text-sm text-error">
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

/// La categoría y la subcategoría: dos campos de texto con sugerencias de
/// las que ya usa la tienda (para no escribirlas distinto) y la opción de
/// escribir una nueva. Cada producto tiene una sola de cada una.
#[component]
fn CategoryField(form: Form, store_categories: Vec<domain::CategoryCount>) -> impl IntoView {
    let cats = StoredValue::new(store_categories);
    let subs = move || {
        let typed = domain::clean_category(&form.category.get());
        cats.with_value(|all| {
            all.iter()
                .find(|c| c.name.to_lowercase() == typed.to_lowercase())
                .map(|c| c.subs.iter().map(|s| s.0.clone()).collect::<Vec<_>>())
                .unwrap_or_default()
        })
    };
    view! {
        <div class="grid gap-4 sm:grid-cols-2">
            <fieldset class="fieldset">
                <legend class="fieldset-legend mk-tip-legend">"Categoría"
                    <ui::HelpTip id="tip-category" label="Categoría" text="Elegí una de tu tienda o escribí una nueva. Es una sola por producto."/></legend>
                <input id="p-category" type="text" class="input w-full" list="p-category-list"
                    maxlength=domain::CATEGORY_MAX autocomplete="off"
                    placeholder="Ej.: Remeras"
                    prop:value=move || form.category.get()
                    on:input=move |ev| form.category.set(event_target_value(&ev))/>
                <datalist id="p-category-list">
                    {cats.with_value(|all| all.iter().map(|c| view! { <option value=c.name.clone()></option> }).collect_view())}
                </datalist>
            </fieldset>
            <fieldset class="fieldset">
                <legend class="fieldset-legend mk-tip-legend">"Subcategoría " <span class="font-normal text-base-content/60">"(opcional)"</span>
                    <ui::HelpTip id="tip-subcategory" label="Subcategoría" text="Va dentro de la categoría."/></legend>
                <input id="p-subcategory" type="text" class="input w-full" list="p-subcategory-list"
                    maxlength=domain::CATEGORY_MAX autocomplete="off"
                    placeholder="Ej.: Manga larga"
                    disabled=move || form.category.with(|c| c.trim().is_empty())
                    prop:value=move || form.subcategory.get()
                    on:input=move |ev| form.subcategory.set(event_target_value(&ev))/>
                <datalist id="p-subcategory-list">
                    {move || subs().into_iter().map(|s| view! { <option value=s></option> }).collect_view()}
                </datalist>
            </fieldset>
        </div>
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
        // Elegida la etiqueta, la lista se cierra para no tapar lo que sigue.
        // Vuelve a abrirse al escribir o al tocar el campo.
        open.set(false);
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
            <legend class="fieldset-legend mk-tip-legend">"Etiquetas"
                <ui::HelpTip id="tip-tags" label="Etiquetas" text="Para encontrar y agrupar productos (por ejemplo: regalo, cocina, verano)."/></legend>
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
                        on:click=move |_| open.set(true)
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
                                    open.set(false);
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
            t.push((
                ev.pointer_id(),
                f64::from(ev.client_x()),
                f64::from(ev.client_y()),
            ));
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

/// «Producto gratis»: sin él, el precio $ 0 no se puede guardar. Al prenderlo
/// el precio queda en 0; al apagarlo, vacío para escribir uno.
#[component]
fn FreeSwitch(form: Form) -> impl IntoView {
    view! {
        <label class="mt-1 flex cursor-pointer items-center gap-2 text-xs text-base-content/70">
            <input type="checkbox" class="toggle toggle-xs toggle-primary"
                prop:checked=move || form.free.get()
                on:change=move |ev| {
                    let on = event_target_checked(&ev);
                    form.free.set(on);
                    if on {
                        form.price.set("0".to_string());
                    } else if form.price.with_untracked(|p| p.trim() == "0") {
                        form.price.set(String::new());
                    }
                }/>
            "Producto gratis ($ 0)"
        </label>
    }
}
