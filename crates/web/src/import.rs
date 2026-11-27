//! Importar productos desde una planilla (CSV), en Productos: se sube el
//! archivo, se revisa y completa en una tabla (fila por fila, con sus fotos)
//! y se importa. Lo que se hizo queda guardado en este navegador, así que
//! cerrar la ventana sin querer no pierde nada.

use domain::{ImportRow, Product};
use leptos::prelude::*;
use serde::{Deserialize, Serialize};

/// Cuántas filas se ven por página.
const PER_PAGE: usize = 8;

/// Lo que devuelve la importación.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ImportDone {
    pub created: usize,
    /// Aviso en palabras simples si no entraron todos.
    pub note: Option<String>,
}

/// Crea los productos de la planilla. La primera fila queda arriba de la
/// lista. Los que tengan algún problema se dejan afuera.
#[server(input = leptos::server_fn::codec::Json)]
pub async fn import_products(
    slug: String,
    products: Vec<Product>,
) -> Result<ImportDone, ServerFnError> {
    use crate::server::{internal, pool};
    let tenant = crate::products::server::require_store(&slug).await?;
    let pool = pool()?;
    let count = db::products::count(&pool, tenant.id)
        .await
        .map_err(internal)?;
    let room = usize::try_from((db::products::MAX_PER_TENANT - count).max(0)).unwrap_or(0);
    let mut valid = Vec::new();
    let mut skipped = 0usize;
    for mut p in products.into_iter().take(domain::IMPORT_MAX_ROWS) {
        p.id = String::new();
        p.example = false;
        if domain::tidy_product(&mut p).is_empty() {
            valid.push(p);
        } else {
            skipped += 1;
        }
    }
    let over = valid.len().saturating_sub(room);
    valid.truncate(room);
    let mut created = 0usize;
    // Del último al primero: así la primera fila de la planilla queda arriba.
    for p in valid.iter().rev() {
        db::products::create(&pool, tenant.id, p)
            .await
            .map_err(internal)?;
        created += 1;
    }
    let note = match (skipped, over) {
        (0, 0) => None,
        (s, 0) => Some(format!(
            "{s} no se importaron porque tenían algo para corregir."
        )),
        (_, o) => Some(format!(
            "{o} no entraron: la tienda llegó al máximo de {} productos.",
            db::products::MAX_PER_TENANT
        )),
    };
    Ok(ImportDone { created, note })
}

/// Lo que se guarda en el navegador mientras se trabaja.
#[derive(Serialize, Deserialize, Clone, Debug)]
struct Draft {
    file: String,
    rows: Vec<ImportRow>,
}

fn storage_key(slug: &str) -> String {
    format!("mk-import:{slug}")
}

fn storage() -> Option<leptos::web_sys::Storage> {
    window().local_storage().ok().flatten()
}

fn load_draft(slug: &str) -> Option<Draft> {
    let text = storage()?.get_item(&storage_key(slug)).ok().flatten()?;
    serde_json::from_str::<Draft>(&text)
        .ok()
        .filter(|d| !d.rows.is_empty())
}

fn save_draft(slug: &str, draft: &Draft) {
    if let (Some(s), Ok(text)) = (storage(), serde_json::to_string(draft)) {
        _ = s.set_item(&storage_key(slug), &text);
    }
}

fn clear_draft(slug: &str) {
    if let Some(s) = storage() {
        _ = s.remove_item(&storage_key(slug));
    }
}

/// El texto como dirección `data:` para bajarlo como archivo.
fn data_url(text: &str) -> String {
    let mut out = String::from("data:text/csv;charset=utf-8,");
    for b in text.bytes() {
        if b.is_ascii_alphanumeric() || b"-_.~".contains(&b) {
            out.push(char::from(b));
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

async fn read_bytes(file: &leptos::web_sys::File) -> Option<Vec<u8>> {
    let buffer = wasm_bindgen_futures::JsFuture::from(file.array_buffer())
        .await
        .ok()?;
    Some(js_sys::Uint8Array::new(&buffer).to_vec())
}

/// Qué filas se ven.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Show {
    All,
    Ready,
    NoPhoto,
    Review,
}

fn field(rows: RwSignal<Vec<ImportRow>>, id: u32, get: fn(&ImportRow) -> String) -> String {
    rows.with(|rs| rs.iter().find(|r| r.id == id).map(get).unwrap_or_default())
}

fn set_field(rows: RwSignal<Vec<ImportRow>>, id: u32, set: fn(&mut ImportRow, String), v: String) {
    rows.update(|rs| {
        if let Some(r) = rs.iter_mut().find(|r| r.id == id) {
            set(r, v);
        }
    });
}

fn problem_of(rows: RwSignal<Vec<ImportRow>>, id: u32) -> Option<String> {
    rows.with(|rs| rs.iter().find(|r| r.id == id).and_then(ImportRow::problem))
}

/// Una celda de la tabla: se escribe encima, sin abrir nada.
#[component]
fn Cell(
    rows: RwSignal<Vec<ImportRow>>,
    id: u32,
    label: &'static str,
    get: fn(&ImportRow) -> String,
    set: fn(&mut ImportRow, String),
    /// Palabra que, si está en el problema de la fila, marca esta celda.
    #[prop(optional)]
    flag: &'static str,
    #[prop(optional)] numeric: bool,
    #[prop(optional)] bold: bool,
) -> impl IntoView {
    let bad = move || !flag.is_empty() && problem_of(rows, id).is_some_and(|p| fold_has(&p, flag));
    view! {
        <td class:num=numeric>
            <input class="mk-cell" class:font-semibold=bold class:text-right=numeric
                aria-label=label
                inputmode=if numeric { "decimal" } else { "text" }
                aria-invalid=move || bad().then_some("true")
                prop:value=move || field(rows, id, get)
                on:input=move |ev| set_field(rows, id, set, event_target_value(&ev))/>
        </td>
    }
}

fn fold_has(text: &str, word: &str) -> bool {
    domain::fold(text).contains(word)
}

/// «Importar»: la ventana con la planilla.
#[component]
pub fn ImportDialog(
    slug: StoredValue<String>,
    /// Sube para abrir la ventana.
    open: RwSignal<u32>,
    /// Se llama con cuántos productos se importaron.
    on_done: Callback<ImportDone>,
) -> impl IntoView {
    let dialog = NodeRef::<leptos::html::Dialog>::new();
    let rows = RwSignal::new(Vec::<ImportRow>::new());
    let file_name = RwSignal::new(String::new());
    let draft = RwSignal::new(None::<Draft>);
    let show = RwSignal::new(Show::All);
    let page = RwSignal::new(0usize);
    let message = RwSignal::new(None::<String>);
    let reading = RwSignal::new(false);
    let target = RwSignal::new(None::<u32>);
    let menu = RwSignal::new((0u32, 0.0f64, 0.0f64));

    // Abrir: busca si quedó algo sin terminar y muestra la ventana.
    Effect::new(move |first: Option<()>| {
        open.track();
        if first.is_none() {
            return;
        }
        if rows.with_untracked(Vec::is_empty) {
            draft.set(load_draft(&slug.get_value()));
        }
        message.set(None);
        if let Some(d) = dialog.get_untracked() {
            if !d.open() {
                _ = d.show_modal();
            }
        }
    });

    // Cada cambio queda guardado.
    Effect::new(move |_| {
        let rs = rows.get();
        if !rs.is_empty() {
            save_draft(
                &slug.get_value(),
                &Draft {
                    file: file_name.get_untracked(),
                    rows: rs,
                },
            );
        }
    });
    Effect::new(move |_| {
        show.track();
        page.set(0);
    });

    let pick_file = move |ev: leptos::ev::Event| {
        let Some(file) = editor::take_files(&ev).into_iter().next() else {
            return;
        };
        reading.set(true);
        message.set(None);
        leptos::task::spawn_local(async move {
            let parsed = match read_bytes(&file).await {
                Some(bytes) => domain::parse_import(&domain::decode_text(&bytes)),
                None => Err("No pudimos leer el archivo. Probá de nuevo.".to_string()),
            };
            reading.set(false);
            match parsed {
                Ok(sheet) => {
                    file_name.set(file.name());
                    show.set(Show::All);
                    page.set(0);
                    if sheet.ignored > 0 {
                        message.set(Some(format!(
                            "Dejamos afuera {} filas vacías o de más (el máximo es {}).",
                            sheet.ignored,
                            domain::IMPORT_MAX_ROWS
                        )));
                    }
                    rows.set(sheet.rows);
                }
                Err(e) => message.set(Some(e)),
            }
        });
    };

    let counts = Memo::new(move |_| {
        rows.with(|rs| {
            let review = rs.iter().filter(|r| r.problem().is_some()).count();
            let nophoto = rs.iter().filter(|r| !r.has_photos()).count();
            (rs.len() - review, nophoto, review)
        })
    });
    let ids = Memo::new(move |_| {
        let s = show.get();
        rows.with(|rs| {
            rs.iter()
                .filter(|r| match s {
                    Show::All => true,
                    Show::Ready => r.problem().is_none(),
                    Show::NoPhoto => !r.has_photos(),
                    Show::Review => r.problem().is_some(),
                })
                .map(|r| r.id)
                .collect::<Vec<_>>()
        })
    });
    let pages = Memo::new(move |_| domain::page_count(ids.with(Vec::len), PER_PAGE));
    let current = Memo::new(move |_| page.get().min(pages.get() - 1));
    let on_page = Memo::new(move |_| {
        let from = current.get() * PER_PAGE;
        ids.with(|v| {
            v.iter()
                .skip(from)
                .take(PER_PAGE)
                .copied()
                .collect::<Vec<_>>()
        })
    });
    let first_problem = Memo::new(move |_| {
        on_page.with(|v| {
            v.iter().find_map(|id| {
                problem_of(rows, *id).map(|p| {
                    let name = field(rows, *id, |r| r.name.clone());
                    let name = if name.trim().is_empty() {
                        "Sin nombre".to_string()
                    } else {
                        name
                    };
                    format!("{name}: {p}")
                })
            })
        })
    });

    let discard = move |_| {
        clear_draft(&slug.get_value());
        draft.set(None);
        rows.set(Vec::new());
        file_name.set(String::new());
        message.set(None);
    };
    let resume = move |_| {
        if let Some(d) = draft.get_untracked() {
            file_name.set(d.file);
            rows.set(d.rows);
            show.set(Show::All);
            page.set(0);
        }
        draft.set(None);
    };

    let run = Action::new(move |products: &Vec<Product>| {
        let products = products.clone();
        async move { import_products(slug.get_value(), products).await }
    });
    Effect::new(move |_| match run.value().get() {
        Some(Ok(done)) => {
            clear_draft(&slug.get_value());
            rows.set(Vec::new());
            draft.set(None);
            file_name.set(String::new());
            if let Some(d) = dialog.get_untracked() {
                d.close();
            }
            on_done.run(done);
        }
        Some(Err(e)) => message.set(Some(crate::products::error_text(&e))),
        None => {}
    });
    let ready_products = move || {
        rows.with(|rs| {
            rs.iter()
                .filter_map(|r| r.to_product().ok())
                .collect::<Vec<_>>()
        })
    };
    let do_import = move |_| {
        let products = ready_products();
        if !products.is_empty() {
            message.set(None);
            run.dispatch(products);
        }
    };

    // Menú ⋯ de una fila.
    let duplicate = move |_| {
        let id = menu.get_untracked().0;
        rows.update(|rs| {
            let next = rs.iter().map(|r| r.id).max().unwrap_or(0) + 1;
            if let Some(i) = rs.iter().position(|r| r.id == id) {
                let mut copy = rs[i].clone();
                copy.id = next;
                rs.insert(i + 1, copy);
            }
        });
    };
    let remove = move |_| {
        let id = menu.get_untracked().0;
        rows.update(|rs| rs.retain(|r| r.id != id));
        if rows.with_untracked(Vec::is_empty) {
            clear_draft(&slug.get_value());
        }
    };

    // Fotos de una fila.
    let library = editor::MyPhotos::new();
    let picker = NodeRef::<leptos::html::Dialog>::new();
    Effect::new(move |_| {
        let Some(d) = picker.get() else { return };
        if target.get().is_some() {
            if !d.open() {
                _ = d.show_modal();
            }
        } else if d.open() {
            d.close();
        }
    });
    let add_photo = move |url: String| {
        let Some(id) = target.get_untracked() else {
            return;
        };
        rows.update(|rs| {
            if let Some(r) = rs.iter_mut().find(|r| r.id == id) {
                if !r.photos.contains(&url) && r.photos.len() < domain::PRODUCT_PHOTOS_MAX {
                    r.photos.push(url);
                }
            }
        });
    };
    let row_photos = move || {
        target
            .get()
            .map(|id| rows.with(|rs| rs.iter().find(|r| r.id == id).map(|r| r.photos.clone())))
            .flatten()
            .unwrap_or_default()
    };

    let stat = move |label_one: &'static str,
                     label_many: &'static str,
                     n: Signal<usize>,
                     which: Show,
                     tone: &'static str| {
        view! {
            <button type="button" class="mk-imp-stat"
                aria-pressed=move || (show.get() == which).to_string()
                on:click=move |_| show.update(|s| *s = if *s == which { Show::All } else { which })>
                <b class=tone>{move || n.get()}</b>
                {move || if n.get() == 1 { label_one } else { label_many }}
            </button>
        }
    };
    let n_ready: Signal<usize> = Signal::derive(move || counts.get().0);
    let n_nophoto: Signal<usize> = Signal::derive(move || counts.get().1);
    let n_review: Signal<usize> = Signal::derive(move || counts.get().2);
    let template = data_url(domain::IMPORT_TEMPLATE);

    view! {
        <dialog node_ref=dialog class="modal mk-imp" aria-labelledby="importar-titulo">
            <div class="modal-box flex max-h-[92dvh] w-full max-w-[min(1240px,96vw)] flex-col gap-0 overflow-hidden p-0">
                <header class="flex items-center gap-3 border-b border-base-200 px-6 py-4">
                    <h2 id="importar-titulo" class="text-lg font-semibold">"Importar productos"</h2>
                    <span class="truncate text-sm text-base-content/60">{move || file_name.get()}</span>
                    <form method="dialog" class="ml-auto">
                        <button class="btn btn-ghost btn-sm btn-square" aria-label="Cerrar">"✕"</button>
                    </form>
                </header>
                {move || {
                    if !rows.with(Vec::is_empty) {
                        view! {
                            <div class="flex flex-wrap items-center gap-2 px-5 py-2.5" role="group" aria-label="Filtrar filas">
                                {stat("listo", "listos", n_ready, Show::Ready, "")}
                                {stat("sin foto", "sin foto", n_nophoto, Show::NoPhoto, "text-warning")}
                                {stat("para revisar", "para revisar", n_review, Show::Review, "text-error")}
                            </div>
                            <div class="min-h-0 grow overflow-auto">
                                <table class="mk-imp-table">
                                    <caption class="sr-only">"Productos de la planilla"</caption>
                                    <colgroup>
                                        <col style="width:15%"/><col style="width:8%"/><col style="width:9%"/><col style="width:6%"/>
                                        <col style="width:9%"/><col style="width:9%"/><col style="width:10%"/><col style="width:12%"/>
                                        <col style="width:6%"/><col style="width:11%"/><col style="width:3rem"/>
                                    </colgroup>
                                    <thead><tr>
                                        <th scope="col" class="!pl-6">"Nombre"</th>
                                        <th scope="col" class="num">"Precio"</th>
                                        <th scope="col" class="num">"Precio anterior"</th>
                                        <th scope="col" class="num">"Stock"</th>
                                        <th scope="col">"Categoría"</th>
                                        <th scope="col">"Subcategoría"</th>
                                        <th scope="col">"Etiquetas"</th>
                                        <th scope="col">"Descripción"</th>
                                        <th scope="col">"Visible"</th>
                                        <th scope="col">"Fotos"</th>
                                        <th scope="col"><span class="sr-only">"Más acciones"</span></th>
                                    </tr></thead>
                                    <tbody>
                                        <For each=move || on_page.get() key=|id| *id let:id>
                                            <tr class:mk-imp-bad=move || problem_of(rows, id).is_some()>
                                                <Cell rows id label="Nombre" bold=true flag="nombre"
                                                    get=|r| r.name.clone() set=|r, v| r.name = v/>
                                                <Cell rows id label="Precio" numeric=true flag="precio"
                                                    get=|r| r.price.clone() set=|r, v| r.price = v/>
                                                <Cell rows id label="Precio anterior" numeric=true flag="anterior"
                                                    get=|r| r.was.clone() set=|r, v| r.was = v/>
                                                <Cell rows id label="Stock" numeric=true flag="cantidad"
                                                    get=|r| r.stock.clone() set=|r, v| r.stock = v/>
                                                <Cell rows id label="Categoría"
                                                    get=|r| r.category.clone() set=|r, v| r.category = v/>
                                                <Cell rows id label="Subcategoría"
                                                    get=|r| r.subcategory.clone() set=|r, v| r.subcategory = v/>
                                                <Cell rows id label="Etiquetas"
                                                    get=|r| r.tags.clone() set=|r, v| r.tags = v/>
                                                <Cell rows id label="Descripción"
                                                    get=|r| r.description.clone() set=|r, v| r.description = v/>
                                                <Cell rows id label="Visible"
                                                    get=|r| r.visible.clone() set=|r, v| r.visible = v/>
                                                <td>
                                                    <div class="mk-imp-ph">
                                                        <button type="button" class="mk-imp-add"
                                                            aria-label=move || format!("Agregar fotos a {}", field(rows, id, |r| r.name.clone()))
                                                            on:click=move |_| { library.ensure_loaded(); target.set(Some(id)); }>"+"</button>
                                                        {move || {
                                                            let photos = rows.with(|rs| rs.iter().find(|r| r.id == id).map(|r| r.photos.clone()).unwrap_or_default());
                                                            let extra = photos.len().saturating_sub(2);
                                                            if photos.is_empty() {
                                                                view! { <span class="text-xs text-base-content/50">"Sin foto"</span> }.into_any()
                                                            } else {
                                                                view! {
                                                                    {photos.into_iter().take(2).map(|u| view! {
                                                                        <span class="size-8 shrink-0 overflow-hidden rounded-field bg-base-200"><ui::Image src=u alt=""/></span>
                                                                    }).collect_view()}
                                                                    {(extra > 0).then(|| view! { <span class="text-xs text-base-content/60">{format!("+{extra}")}</span> })}
                                                                }.into_any()
                                                            }
                                                        }}
                                                    </div>
                                                </td>
                                                <td class="text-center">
                                                    <button type="button" class="btn btn-ghost btn-sm btn-square"
                                                        aria-label=move || format!("Más acciones de {}", field(rows, id, |r| r.name.clone()))
                                                        popovertarget="mk-imp-menu"
                                                        on:click=move |ev| {
                                                            use leptos::wasm_bindgen::JsCast;
                                                            let rect = ev.current_target()
                                                                .and_then(|t| t.dyn_into::<leptos::web_sys::Element>().ok())
                                                                .map(|e| e.get_bounding_client_rect());
                                                            if let Some(r) = rect {
                                                                menu.set((id, r.bottom() + 4.0, (r.right() - 208.0).max(8.0)));
                                                            }
                                                        }>"⋯"</button>
                                                </td>
                                            </tr>
                                        </For>
                                    </tbody>
                                </table>
                                {move || ids.with(Vec::is_empty).then(|| view! {
                                    <p class="p-8 text-center text-sm text-base-content/70">"Ninguna fila coincide."</p>
                                })}
                            </div>
                            <p role="alert" class="px-6 py-2 text-sm text-error empty:hidden">{move || first_problem.get()}</p>
                        }.into_any()
                    } else if let Some(d) = draft.get() {
                        view! {
                            <div class="flex flex-col items-start gap-4 p-8">
                                <div>
                                    <h3 class="text-base font-semibold">"Tenés una importación sin terminar"</h3>
                                    <p class="mt-1 text-sm text-base-content/70">
                                        {format!("{} productos de «{}».", d.rows.len(), d.file)}
                                    </p>
                                </div>
                                <div class="flex gap-2">
                                    <button type="button" class="btn btn-primary" on:click=resume>"Continuar"</button>
                                    <button type="button" class="btn" on:click=discard>"Empezar de nuevo"</button>
                                </div>
                            </div>
                        }.into_any()
                    } else {
                        view! {
                            <div class="flex flex-col items-start gap-4 p-8">
                                <div>
                                    <h3 class="text-base font-semibold">"Subí tu planilla"</h3>
                                    <p class="mt-1 max-w-xl text-sm text-base-content/70">
                                        "Elegí un archivo CSV con tus productos. Después revisás cada fila y sumás las fotos antes de importar."
                                    </p>
                                </div>
                                <div class="flex flex-wrap items-center gap-3">
                                    <label for="importar-archivo" class="btn btn-primary" class:btn-disabled=move || reading.get()>
                                        {move || if reading.get() { "Leyendo…" } else { "Elegir planilla (.csv)" }}
                                    </label>
                                    <input id="importar-archivo" type="file" class="sr-only" accept=".csv,text/csv"
                                        on:change=pick_file/>
                                    <a class="link text-sm" download="planilla-productos.csv" href=template.clone()>"Bajar la planilla modelo"</a>
                                </div>
                                <p role="alert" class="text-sm text-error empty:hidden">{move || message.get()}</p>
                            </div>
                        }.into_any()
                    }
                }}
                <footer class="grid grid-cols-[1fr_auto_1fr] items-center gap-3 border-t border-base-200 bg-base-200/40 px-6 py-3 max-md:grid-cols-1">
                    <span class="text-xs text-base-content/60" role="status" aria-live="polite">
                        {move || if rows.with(Vec::is_empty) { "" } else { "Guardamos tu avance" }}
                    </span>
                    <nav aria-label="Páginas" class="flex items-center gap-1 justify-self-center"
                        class:invisible=move || rows.with(Vec::is_empty) || pages.get() < 2>
                        <button type="button" class="mk-imp-pg" aria-label="Página anterior"
                            disabled=move || current.get() == 0
                            on:click=move |_| page.set(current.get_untracked().saturating_sub(1))>"‹"</button>
                        {move || domain::page_items(current.get(), pages.get()).into_iter().map(|p| match p {
                            Some(n) => view! {
                                <button type="button" class="mk-imp-pg" class:mk-imp-pg-on=move || current.get() == n
                                    aria-current=move || (current.get() == n).then_some("page")
                                    aria-label=format!("Página {}", n + 1)
                                    on:click=move |_| page.set(n)>{n + 1}</button>
                            }.into_any(),
                            None => view! { <span class="px-1 text-base-content/50" aria-hidden="true">"…"</span> }.into_any(),
                        }).collect_view()}
                        <button type="button" class="mk-imp-pg" aria-label="Página siguiente"
                            disabled=move || current.get() + 1 >= pages.get()
                            on:click=move |_| page.set((current.get_untracked() + 1).min(pages.get_untracked() - 1))>"›"</button>
                    </nav>
                    <div class="flex items-center justify-end gap-2">
                        <form method="dialog"><button class="btn">"Seguir después"</button></form>
                        <button type="button" class="btn btn-primary"
                            disabled=move || n_ready.get() == 0 || run.pending().get() || rows.with(Vec::is_empty)
                            on:click=do_import>
                            {move || if run.pending().get() {
                                "Importando…".to_string()
                            } else if n_ready.get() == 1 {
                                "Importar 1 producto".to_string()
                            } else {
                                format!("Importar {} productos", n_ready.get())
                            }}
                        </button>
                    </div>
                </footer>
                <p role="alert" class="border-t border-base-200 px-6 py-2 text-sm text-error empty:hidden">
                    {move || if rows.with(Vec::is_empty) { None } else { message.get() }}
                </p>
                // Menú ⋯: va en la capa de arriba para no quedar cortado por la tabla.
                <div id="mk-imp-menu" popover="auto" class="mk-imp-menu"
                    style=move || { let (_, top, left) = menu.get(); format!("top:{top}px;left:{left}px") }>
                    <button type="button" class="btn btn-ghost w-full justify-start" popovertarget="mk-imp-menu" popovertargetaction="hide"
                        on:click=duplicate>"Duplicar fila"</button>
                    <button type="button" class="btn btn-ghost w-full justify-start text-error" popovertarget="mk-imp-menu" popovertargetaction="hide"
                        on:click=remove>"Quitar de la importación"</button>
                </div>
            </div>
            <form method="dialog" class="modal-backdrop"><button>"Cerrar"</button></form>
        </dialog>
        // Fotos de la fila elegida.
        <dialog node_ref=picker class="modal" aria-labelledby="importar-fotos-titulo"
            on:close=move |_| target.set(None)>
            <div class="modal-box max-w-xl">
                <h2 id="importar-fotos-titulo" class="text-base font-semibold">
                    {move || target.get().map(|id| {
                        let n = field(rows, id, |r| r.name.clone());
                        if n.trim().is_empty() { "Fotos del producto".to_string() } else { format!("Fotos de «{n}»") }
                    })}
                </h2>
                <div class="mt-3 flex flex-wrap items-center gap-2">
                    <label for="importar-subir" class="btn btn-primary btn-sm"
                        class:btn-disabled=move || library.progress.get().is_some()>"Subir desde mi computadora"</label>
                    <input id="importar-subir" type="file" class="sr-only" multiple
                        accept="image/jpeg,image/png,image/webp,image/gif"
                        on:change=move |ev| library.upload_each(editor::take_files(&ev), Callback::new(move |item: domain::MediaItem| add_photo(item.url)))/>
                    <span class="text-sm text-base-content/70" role="status">{move || library.progress.get()}</span>
                </div>
                {move || {
                    let e = library.upload_errors.get();
                    (!e.is_empty()).then(|| view! {
                        <ul role="alert" class="mt-2 flex flex-col gap-0.5 text-sm text-error">
                            {e.into_iter().map(|e| view! { <li>{e}</li> }).collect_view()}
                        </ul>
                    })
                }}
                {move || {
                    let list = row_photos();
                    (!list.is_empty()).then(|| view! {
                        <p class="mt-4 text-sm font-medium">"En este producto"</p>
                        <ul class="mt-2 flex flex-wrap gap-2" aria-label="Fotos del producto">
                            {list.into_iter().enumerate().map(|(i, u)| {
                                let drop = u.clone();
                                view! {
                                    <li class="group relative size-16 overflow-hidden rounded-field bg-base-200">
                                        <ui::Image src=u alt=format!("Foto {}", i + 1)/>
                                        <button type="button" class="absolute right-0.5 top-0.5 grid size-5 place-items-center rounded-full bg-black/70 text-xs text-white"
                                            aria-label=format!("Sacar la foto {}", i + 1)
                                            on:click=move |_| {
                                                let Some(id) = target.get_untracked() else { return };
                                                rows.update(|rs| if let Some(r) = rs.iter_mut().find(|r| r.id == id) { r.photos.retain(|p| p != &drop); });
                                            }>"✕"</button>
                                    </li>
                                }
                            }).collect_view()}
                        </ul>
                    })
                }}
                <p class="mt-4 text-sm font-medium">"Elegir de Mis fotos"</p>
                {move || library.load_error.get().map(|e| view! { <p role="alert" class="text-sm text-error">{e}</p> })}
                {move || match library.items.get() {
                    None => view! { <ui::Skeleton class="mt-2 h-24 w-full"/> }.into_any(),
                    Some(items) if items.is_empty() => view! {
                        <p class="mt-2 text-sm text-base-content/70">"Todavía no tenés fotos. Subí las primeras."</p>
                    }.into_any(),
                    Some(items) => view! {
                        <div class="mt-2 grid max-h-64 grid-cols-5 gap-2 overflow-y-auto sm:grid-cols-6">
                            {items.into_iter().map(|item| {
                                let url = item.url.clone();
                                let url2 = item.url.clone();
                                let chosen = Memo::new(move |_| row_photos().contains(&url2));
                                view! {
                                    <button type="button" class="relative aspect-square overflow-hidden rounded-field bg-base-200"
                                        class:ring-2=move || chosen.get() class:ring-primary=move || chosen.get()
                                        aria-pressed=move || chosen.get().to_string()
                                        aria-label=item.name.clone()
                                        on:click=move |_| {
                                            if chosen.get_untracked() {
                                                let Some(id) = target.get_untracked() else { return };
                                                rows.update(|rs| if let Some(r) = rs.iter_mut().find(|r| r.id == id) { r.photos.retain(|p| p != &url); });
                                            } else {
                                                add_photo(url.clone());
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
                <div class="modal-action">
                    <form method="dialog"><button class="btn btn-primary" autofocus>"Listo"</button></form>
                </div>
            </div>
            <form method="dialog" class="modal-backdrop"><button>"Cerrar"</button></form>
        </dialog>
    }
}
