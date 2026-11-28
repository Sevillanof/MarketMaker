//! Campos del inspector: un control por tipo de campo, con ayuda, contador
//! de caracteres y el error explicado junto al campo.

use domain::{Field, FieldKind, MediaItem, PageDoc, Product, Widget};
use leptos::prelude::*;

use crate::icons::Icon;
use crate::photos::{take_files, LinkImport, MyPhotos};
use crate::{EditorState, Ui};

/// Cambia una prop de un nodo pasando por `serde_json::Value`.
pub(crate) fn set_field(doc: &mut PageDoc, id: &str, key: &str, value: serde_json::Value) {
    let Some(node) = doc.node_mut(id) else {
        return;
    };
    let Ok(mut v) = serde_json::to_value(&node.widget) else {
        return;
    };
    if let Some(props) = v.get_mut("props").and_then(|p| p.as_object_mut()) {
        props.insert(key.to_string(), value);
    }
    if let Ok(widget) = serde_json::from_value::<Widget>(v) {
        node.widget = widget;
    }
}

pub(crate) fn field_value(doc: &PageDoc, id: &str, key: &str) -> serde_json::Value {
    doc.node(id)
        .and_then(|n| serde_json::to_value(&n.widget).ok())
        .and_then(|v| v.get("props").and_then(|p| p.get(key)).cloned())
        .unwrap_or(serde_json::Value::Null)
}

/// Valor inicial de un campo, para elementos nuevos de una lista.
fn empty_value(kind: &FieldKind) -> serde_json::Value {
    use serde_json::Value;
    match kind {
        FieldKind::Number { min, .. } | FieldKind::Range { min, .. } => Value::from(*min),
        FieldKind::Bool => Value::Bool(false),
        FieldKind::Select { options } => {
            Value::String(options.first().map(|(k, _)| *k).unwrap_or_default().into())
        }
        FieldKind::List { .. } => Value::Array(Vec::new()),
        _ => Value::String(String::new()),
    }
}

/// Elemento nuevo para una lista: hereda del último la imagen y las opciones
/// (estilo, estrellas, íconos), pero no sus textos, enlaces ni sublistas.
/// Antes era una copia exacta: en el menú quedaban ocho «Inicio» iguales,
/// cada uno con el desplegable del anterior. Si la lista está vacía, uno con
/// los valores iniciales de cada campo.
pub(crate) fn new_list_item(items: &[serde_json::Value], item: &[Field]) -> serde_json::Value {
    let Some(mut next) = items.last().cloned() else {
        return serde_json::Value::Object(
            item.iter()
                .map(|f| (f.key.to_string(), empty_value(&f.kind)))
                .collect(),
        );
    };
    if let Some(obj) = next.as_object_mut() {
        for f in item {
            let keep_image = matches!(f.kind, FieldKind::Url) && f.key.ends_with("image_url");
            let clear = match f.kind {
                FieldKind::Text { .. } | FieldKind::LongText { .. } => true,
                FieldKind::Url => !keep_image,
                FieldKind::List { min, .. } => min == 0,
                _ => false,
            };
            if clear {
                obj.insert(f.key.to_string(), empty_value(&f.kind));
            }
        }
    }
    next
}

/// Explicación breve de los campos que no se entienden solos.
fn help(key: &str) -> Option<&'static str> {
    Some(match key {
        "button_href" | "href" | "link_href" => "Elegí adónde lleva al tocarlo. Para un sitio de afuera, tocá «Otra» y pegá la dirección (www.… se completa sola).",
        "button_label" => "Si lo dejás vacío, no se muestra el botón.",
        "alt" => "No se ve en la tienda: la leen los buscadores y los lectores de pantalla. Describí la imagen en pocas palabras.",
        "autoplay" => "Con 0 las diapositivas solo cambian cuando el cliente las pasa.",
        "ends_at" => "Cuando llega esta fecha, la cuenta regresiva muestra el texto de fin.",
        "ended_text" => "Se muestra cuando termina la oferta.",
        "limit" => "Cuántos productos de tu catálogo se muestran.",
        "show_price" => "Mostrar u ocultar el precio debajo del nombre.",
        "legal" => "Si lo dejás vacío, se muestra © y el nombre de tu tienda.",
        "brand" => "Si lo dejás vacío, se usa el nombre de tu tienda (lo cambiás en Datos de la tienda).",
        "logo_url" => "Si lo dejás vacío, se usa el logo de tu tienda (lo cambiás en Datos de la tienda).",
        _ => return None,
    })
}

/// ¿Es un campo de imagen? Esos usan el selector visual.
fn is_image(field: &Field) -> bool {
    matches!(field.kind, FieldKind::Url) && field.key.ends_with("image_url")
        || field.key == "logo_url"
}

/// Ajusta el alto de una cuadrícula de productos a su cantidad por página.
pub(crate) fn fit_product_grid(nodes: &mut Vec<domain::Node>, id: &str, total: usize) {
    let Some(node) = nodes.iter().find(|n| n.id == id) else {
        return;
    };
    let Widget::ProductGrid(g) = &node.widget else {
        return;
    };
    let per_page = usize::try_from(g.limit).unwrap_or(1);
    let h = domain::product_grid_rows(per_page, node.size.w, total, !g.title.is_empty());
    let w = i32::from(node.size.w);
    grid::resize_node(nodes, id, w, i32::from(h));
}

/// Campo de primer nivel de las props de un nodo.
#[component]
pub(crate) fn PropField(
    products: StoredValue<Vec<Product>>,
    id: String,
    field: &'static Field,
) -> impl IntoView {
    let state = expect_context::<EditorState>();
    let key = field.key;
    let uid = format!("f-{id}-{key}");
    let id = StoredValue::new(id);
    let group = id.with_value(|id| format!("{id}/{key}"));
    let value = Signal::derive(move || {
        state
            .doc
            .with(|d| id.with_value(|rid| field_value(d, rid, key)))
    });
    let set = Callback::new(move |v: serde_json::Value| {
        id.with_value(|rid| {
            let layer = state.layer.get_untracked();
            state.apply_grouped(Some(&group), |d| {
                set_field(d, rid, key, v);
                // Más o menos productos por página: el bloque toma el alto que
                // necesitan, así las fotos no quedan aplastadas ni gigantes.
                if key == "limit" {
                    let total = products.with_value(|ps| ps.iter().filter(|p| p.visible).count());
                    fit_product_grid(d.layer_mut(layer), rid, total);
                }
            });
        });
    });
    // Al elegir una foto de Mis fotos, su descripción pasa al bloque si el
    // bloque tiene "Descripción" y está vacía.
    let on_photo = (key == "image_url").then(|| {
        Callback::new(move |item: MediaItem| {
            if item.alt.is_empty() {
                return;
            }
            id.with_value(|rid| {
                let empty = state
                    .doc
                    .with_untracked(|d| field_value(d, rid, "alt").as_str() == Some(""));
                if empty {
                    state.apply(|d| {
                        set_field(d, rid, "alt", serde_json::Value::String(item.alt.clone()))
                    });
                }
            });
        })
    });
    view! { <FieldInput products field uid value set on_photo/> }
}

/// Control de un campo según su tipo. `value` es el valor actual y `set`
/// reemplaza el valor entero (en una lista, la lista entera). Los cambios
/// seguidos de un mismo campo son un solo paso de deshacer; al salir del
/// campo o en controles de un solo clic se cierra el paso.
#[component]
fn FieldInput(
    products: StoredValue<Vec<Product>>,
    field: &'static Field,
    /// Prefijo único para los `id` del campo.
    uid: String,
    value: Signal<serde_json::Value>,
    set: Callback<serde_json::Value>,
    /// Se eligió una foto de Mis fotos (para copiar su descripción).
    #[prop(default = None)]
    on_photo: Option<Callback<MediaItem>>,
) -> impl IntoView {
    let state = expect_context::<EditorState>();
    let ui = expect_context::<Ui>();
    // El error se muestra recién cuando la persona terminó con el campo
    // (salió de él o lo cambió de un clic) o intentó guardar, no mientras
    // escribe. Si lo corrige, se va enseguida.
    let touched = RwSignal::new(false);
    let found = Signal::derive(move || value.with(|v| domain::field_error(&field.kind, Some(v))));
    let error = Signal::derive(move || {
        (touched.get() || ui.show_errors.get())
            .then(|| found.get())
            .flatten()
    });
    // Al salir del campo: cierra el paso de deshacer y, si quedó un error que
    // no se veía, lo anuncia un momento después para no pisar el campo que
    // recibe el foco.
    let end = move || {
        state.end_group();
        // Si el campo ya se quitó (se cambió de página o de bloque con el
        // foco adentro), el `blur` llega tarde: no hay nada que marcar.
        let Some(was_shown) = error.try_with_untracked(Option::is_some) else {
            return;
        };
        touched.set(true);
        if !was_shown {
            if let Some(msg) = found.try_get_untracked().flatten() {
                let text = format!("{}: {msg}", field.label);
                set_timeout(
                    move || ui.announce(text),
                    std::time::Duration::from_millis(600),
                );
            }
        }
    };
    // Un clic, un paso de deshacer.
    let set_once = move |v: serde_json::Value| {
        state.end_group();
        set.run(v);
        state.end_group();
        touched.set(true);
    };
    let text = move || value.get().as_str().unwrap_or_default().to_string();
    let set_str = move |s: String| set.run(serde_json::Value::String(s));
    let (help_id, error_id) = (format!("{uid}-help"), format!("{uid}-error"));
    let help_text = help(field.key);
    let described = {
        let (help_id, error_id) = (help_id.clone(), error_id.clone());
        move || {
            let mut ids = Vec::new();
            if help_text.is_some() {
                ids.push(help_id.clone());
            }
            if error.with(Option::is_some) {
                ids.push(error_id.clone());
            }
            (!ids.is_empty()).then(|| ids.join(" "))
        }
    };
    let invalid = move || {
        if error.with(Option::is_some) {
            "true"
        } else {
            "false"
        }
    };

    if let FieldKind::List { item, min, max } = &field.kind {
        if field.label == "Beneficios" || field.label == "Categorías" {
            return view! { <RowsList products field item min=*min max=*max uid value set/> }
                .into_any();
        }
        return view! { <ListInput products field item min=*min max=*max uid value set/> }
            .into_any();
    }
    if field.key == "code" && field.label == "Cupón" {
        return view! { <CouponPicker uid=uid.clone() value set=Callback::new(set_once)/> }
            .into_any();
    }
    if is_image(field) {
        return view! {
            <div class="flex flex-col gap-1">
                <span class="text-sm font-medium" id=format!("{uid}-label")>{field.label}</span>
                <ImagePicker uid=uid.clone() value pick=Callback::new(set_once) on_photo/>
                {move || error.get().map(|e| view! { <FieldError id=format!("{uid}-error") text=e/> })}
            </div>
        }
        .into_any();
    }

    if let FieldKind::Select { options } = &field.kind {
        if let Some(kind) = crate::pictures::Pictures::for_options(options) {
            return view! {
                <crate::pictures::PicturePicker kind uid=uid.clone() legend=field.label options
                    value=Signal::derive(text) products
                    pick=Callback::new(move |v: String| set_once(serde_json::Value::String(v)))/>
            }
            .into_any();
        }
    }

    let counter = match &field.kind {
        FieldKind::Text { max } | FieldKind::LongText { max } => Some(*max),
        _ => None,
    };
    let input = match &field.kind {
        FieldKind::Text { max } => view! {
            <input
                type="text"
                id=uid.clone()
                class="input input-sm w-full"
                class:input-error=move || error.with(Option::is_some)
                maxlength=*max
                aria-invalid=invalid
                aria-describedby=described.clone()
                prop:value=text
                on:input=move |ev| set_str(event_target_value(&ev))
                on:blur=move |_| end()
            />
        }
        .into_any(),
        FieldKind::LongText { max } => view! {
            <textarea
                id=uid.clone()
                class="textarea textarea-sm h-24 w-full"
                class:textarea-error=move || error.with(Option::is_some)
                maxlength=*max
                aria-invalid=invalid
                aria-describedby=described.clone()
                prop:value=text
                on:input=move |ev| set_str(event_target_value(&ev))
                on:blur=move |_| end()
            ></textarea>
        }
        .into_any(),
        FieldKind::Url => {
            use domain::LinkKind;
            let tab = RwSignal::new(value.with_untracked(|v| LinkKind::of(v.as_str().unwrap_or_default())));
            let visible: Vec<(String, String)> = products.with_value(|ps| {
                ps.iter().filter(|p| p.visible).map(|p| (p.id.clone(), p.name.clone())).collect()
            });
            let mut cats: Vec<String> = products.with_value(|ps| {
                ps.iter().filter(|p| p.visible && !p.category.trim().is_empty()).map(|p| p.category.trim().to_string()).collect()
            });
            cats.sort();
            cats.dedup();
            let products_opts = StoredValue::new(
                visible.into_iter().map(|(id, name)| (domain::product_href(&id), name)).collect::<Vec<_>>(),
            );
            let cat_opts = StoredValue::new(
                cats.into_iter().map(|c| (domain::category_href(&c), c)).collect::<Vec<_>>(),
            );
            let pages = move || {
                state.doc.with(|d| {
                    (0..d.page_count())
                        .map(|i| (d.page_path(i).to_string(), format!("{} ({})", d.page_name(i), d.page_path(i))))
                        .collect::<Vec<_>>()
                })
            };
            // Otros bloques de la misma página (solo el destino del anuncio).
            let allow_widget = field.label == "Destino del enlace";
            let widget_opts = move || {
                state.doc.with(|d| {
                    let mut nodes: Vec<_> = d.layer(state.layer.get()).iter().collect();
                    nodes.sort_by_key(|n| (n.pos.y, n.pos.x));
                    nodes
                        .into_iter()
                        .map(|n| (domain::widget_href(&n.id), crate::palette::label_of(n.widget.type_name()).to_string()))
                        .collect::<Vec<_>>()
                })
            };
            let uid = StoredValue::new(uid.clone());
            let described = StoredValue::new(described.clone());
            // Un desplegable con las opciones; `empty` si no hay ninguna.
            let picker = move |opts: Vec<(String, String)>, prompt: &'static str, empty: &'static str| {
                if opts.is_empty() {
                    return view! { <p class="rounded-lg bg-base-200 px-3 py-2 text-xs text-base-content/75">{empty}</p> }.into_any();
                }
                let current = text();
                let known = opts.iter().any(|(h, _)| *h == current);
                view! {
                    <select
                        id=uid.get_value()
                        class="select select-sm w-full"
                        aria-describedby=move || described.with_value(|d| d())
                        on:change=move |ev| set_once(serde_json::Value::String(event_target_value(&ev)))
                    >
                        <option value="" disabled selected=!known>{prompt}</option>
                        {opts.into_iter().map(|(href, label)| {
                            let sel = href == current;
                            view! { <option value=href selected=sel>{label}</option> }
                        }).collect_view()}
                    </select>
                }.into_any()
            };
            view! {
                <div class="flex flex-col gap-1.5">
                    <div class="join w-full" role="group" aria-label="Adónde lleva">
                        {[(LinkKind::Page, "Página"), (LinkKind::Product, "Producto"), (LinkKind::Category, "Categoría"), (LinkKind::Widget, "Widget"), (LinkKind::Other, "Otra")]
                            .into_iter()
                            .filter(|(k, _)| allow_widget || *k != LinkKind::Widget)
                            .map(|(k, label)| view! {
                                <button
                                    type="button"
                                    class="btn btn-xs join-item flex-1"
                                    class:btn-primary=move || tab.get() == k
                                    aria-pressed=move || if tab.get() == k { "true" } else { "false" }
                                    on:click=move |_| tab.set(k)
                                >{label}</button>
                            })
                            .collect_view()}
                    </div>
                    {move || match tab.get() {
                        LinkKind::Page => picker(pages(), "Elegí una página…", ""),
                        LinkKind::Product => picker(
                            products_opts.get_value(),
                            "Elegí un producto…",
                            "Todavía no tenés productos a la vista. Cargalos en Productos.",
                        ),
                        LinkKind::Category => picker(
                            cat_opts.get_value(),
                            "Elegí una categoría…",
                            "Tus productos todavía no tienen categorías. Las asignás en Productos.",
                        ),
                        LinkKind::Widget => picker(
                            widget_opts(),
                            "Elegí un widget de esta página…",
                            "Esta página no tiene otros widgets.",
                        ),
                        LinkKind::Other => view! {
                            <input
                                type="text"
                                inputmode="url"
                                id=uid.get_value()
                                class="input input-sm w-full"
                                class:input-error=move || error.with(Option::is_some)
                                placeholder="www.… , hola@… o #contacto"
                                aria-invalid=invalid
                                aria-describedby=move || described.with_value(|d| d())
                                prop:value=text
                                on:input=move |ev| set_str(event_target_value(&ev))
                                on:blur=move |_| {
                                    // `www.algo.com` → `https://www.algo.com`, `ofertas` → `/ofertas`…
                                    let typed = text();
                                    let tidy = domain::tidy_link(&typed);
                                    if tidy != typed {
                                        set_str(tidy);
                                    }
                                    end();
                                }
                            />
                        }.into_any(),
                    }}
                </div>
            }
            .into_any()
        }
        FieldKind::DateTime => view! {
            <input
                type="datetime-local"
                id=uid.clone()
                class="input input-sm w-full"
                aria-invalid=invalid
                aria-describedby=described.clone()
                prop:value=text
                on:change=move |ev| set_once(serde_json::Value::String(event_target_value(&ev)))
            />
        }
        .into_any(),
        FieldKind::Number { min: 1, max: 5 } if field.key == "rating" => {
            return view! { <Stars label=field.label value set=Callback::new(set_once)/> }.into_any();
        }
        FieldKind::Number { min, max } => {
            let (min, max) = (*min, *max);
            // Lo que se escribió fuera de rango (o con decimales) se corrige,
            // y se avisa con un texto chico en vez de cambiarlo callado.
            let fixed = RwSignal::new(None::<String>);
            let range_id = format!("{uid}-range");
            let described = {
                let range_id = range_id.clone();
                move || Some(described().map_or(range_id.clone(), |d| format!("{d} {range_id}")))
            };
            view! {
                <input
                    type="number"
                    id=uid.clone()
                    class="input input-sm w-full"
                    min=min
                    max=max
                    step="1"
                    inputmode="numeric"
                    aria-describedby=described
                    prop:value=move || value.get().as_i64().map(|n| n.to_string()).unwrap_or_default()
                    on:input=move |ev| {
                        let typed = event_target_value(&ev);
                        let Ok(x) = typed.trim().replace(',', ".").parse::<f64>() else {
                            return;
                        };
                        let n = (x.round() as i64).clamp(min, max);
                        fixed.set((n as f64 != x).then(|| format!("Quedó en {n}.")));
                        set.run(serde_json::Value::from(n));
                    }
                    on:blur=move |ev| {
                        // Vacío o con letras: vuelve al número que tenía.
                        let input = event_target::<web_sys::HtmlInputElement>(&ev);
                        if input.value().trim().is_empty() {
                            if let Some(n) = value.get_untracked().as_i64() {
                                input.set_value(&n.to_string());
                                fixed.set(Some(format!("Quedó en {n}.")));
                            }
                        }
                        end();
                    }
                />
                <p id=range_id class="text-xs text-base-content/75">
                    {format!("Entre {min} y {max}.")}
                    {move || fixed.get().map(|t| view! { " " <span class="font-medium text-amber-800">{t}</span> })}
                </p>
            }
            .into_any()
        }
        FieldKind::Range { min, max, unit } => {
            let (min, max, unit) = (*min, *max, *unit);
            return view! {
                <div class="flex flex-col gap-1">
                    <div class="flex items-baseline justify-between gap-2">
                        <label class="text-sm font-medium" for=uid.clone()>{field.label}</label>
                        <output class="text-xs tabular-nums text-base-content/75" for=uid.clone()>
                            {move || format!("{}{unit}", value.get().as_i64().unwrap_or(min))}
                        </output>
                    </div>
                    <input
                        type="range"
                        id=uid.clone()
                        class="range range-primary range-xs"
                        min=min
                        max=max
                        step="1"
                        aria-describedby=described.clone()
                        prop:value=move || value.get().as_i64().unwrap_or(min).to_string()
                        on:input=move |ev| {
                            if let Ok(n) = event_target_value(&ev).parse::<i64>() {
                                set.run(serde_json::Value::from(n.clamp(min, max)));
                            }
                        }
                        on:change=move |_| end()
                    />
                </div>
            }
            .into_any();
        }
        FieldKind::Color => {
            let theme = move || value.get().as_str().is_none_or(str::is_empty);
            return view! {
                <div class="flex flex-col gap-1">
                    <span class="text-sm font-medium" id=format!("{uid}-label")>{field.label}</span>
                    <div class="flex items-center gap-3">
                        <input
                            type="color"
                            id=uid.clone()
                            class="h-8 w-12 cursor-pointer rounded-field border border-base-300 bg-base-100 p-0.5"
                            aria-labelledby=format!("{uid}-label")
                            prop:value=move || value.get().as_str().filter(|s| !s.is_empty()).unwrap_or("#888888").to_string()
                            on:input=move |ev| set.run(serde_json::Value::String(event_target_value(&ev)))
                            on:change=move |_| end()
                        />
                        <label class="flex items-center gap-1.5 text-xs">
                            <input
                                type="checkbox"
                                class="checkbox checkbox-xs"
                                prop:checked=theme
                                on:change=move |ev| {
                                    if event_target_checked(&ev) {
                                        set_once(serde_json::Value::String(String::new()));
                                    } else {
                                        set_once(serde_json::Value::String("#888888".into()));
                                    }
                                }
                            />
                            "Del tema"
                        </label>
                    </div>
                </div>
            }
            .into_any();
        }
        FieldKind::Bool => {
            return view! {
                <label class="flex items-center justify-between gap-3">
                    <span class="text-sm font-medium">{field.label}</span>
                    <input
                        type="checkbox"
                        class="toggle toggle-sm toggle-primary"
                        aria-describedby=described.clone()
                        prop:checked=move || value.get().as_bool().unwrap_or(false)
                        on:change=move |ev| set_once(serde_json::Value::Bool(event_target_checked(&ev)))
                    />
                </label>
                {help_text.map(|h| view! { <p id=help_id.clone() class="-mt-3 text-xs text-base-content/75">{h}</p> })}
            }
            .into_any();
        }
        FieldKind::Select { options } => view! {
            <select
                id=uid.clone()
                class="select select-sm w-full"
                aria-describedby=described.clone()
                prop:value=text
                on:change=move |ev| set_once(serde_json::Value::String(event_target_value(&ev)))
            >
                {options.iter().map(|(v, l)| view! { <option value=*v>{*l}</option> }).collect_view()}
            </select>
        }
        .into_any(),
        FieldKind::ProductRef => view! {
            <ProductPicker uid=uid.clone() products value=Signal::derive(text)
                pick=Callback::new(move |v: String| set_once(serde_json::Value::String(v)))/>
        }
        .into_any(),
        FieldKind::List { .. } => unreachable!("las listas se devuelven antes"),
    };

    view! {
        <div class="flex flex-col gap-1">
            <div class="flex items-baseline justify-between gap-2">
                <label for=uid.clone() class="text-sm font-medium">{field.label}</label>
                {counter.map(|max| {
                    let count = move || value.with(|v| v.as_str().map_or(0, |s| s.chars().count()));
                    // Solo cuando se acerca al límite: antes es ruido.
                    move || near_limit(count(), max).then(|| view! {
                        <span class="text-xs tabular-nums text-base-content/75" aria-hidden="true">
                            {move || format!("{}/{max}", count())}
                        </span>
                    })
                })}
            </div>
            {input}
            {help_text.map(|h| view! { <p id=help_id.clone() class="text-xs text-base-content/75">{h}</p> })}
            {move || error.get().map(|e| view! { <FieldError id=error_id.clone() text=e/> })}
        </div>
    }
    .into_any()
}

/// ¿Ya se usó el 80 % de los caracteres?
fn near_limit(count: usize, max: usize) -> bool {
    count * 5 >= max * 4
}

/// Texto por defecto al prender el botón de un bloque.
const DEFAULT_BUTTON_LABEL: &str = "Ver más";

/// Qué le falta a un botón prendido, en palabras simples.
fn button_hint(label: &str, href: &str) -> Option<&'static str> {
    match (label.trim().is_empty(), href.trim().is_empty()) {
        (true, _) => Some("Escribí qué dice el botón. Si queda vacío, no se muestra."),
        (false, true) => Some("Falta adónde lleva. Sin eso, el botón no se muestra en tu tienda."),
        _ => None,
    }
}

/// Grupo "Botón" de Contenido: un interruptor para mostrarlo y, prendido,
/// su texto y adónde lleva, con un aviso si falta alguno de los dos.
#[component]
pub(crate) fn ButtonGroup(
    products: StoredValue<Vec<Product>>,
    id: String,
    label_field: &'static Field,
    href_field: &'static Field,
) -> impl IntoView {
    let state = expect_context::<EditorState>();
    let sid = StoredValue::new(id.clone());
    let read = move |key: &'static str| {
        move || {
            state.doc.with(|d| {
                sid.with_value(|id| field_value(d, id, key))
                    .as_str()
                    .unwrap_or_default()
                    .to_string()
            })
        }
    };
    let (label, href) = (read(label_field.key), read(href_field.key));
    // Arranca prendido si el bloque ya tiene botón (o un enlace a medias).
    let on = RwSignal::new(!label().is_empty() || !href().is_empty());
    // Al apagarlo se guarda el texto, para devolverlo si se vuelve a prender.
    let saved = StoredValue::new(String::new());
    let toggle = move |show: bool| {
        on.set(show);
        let text = if show && label().trim().is_empty() {
            let s = saved.get_value();
            if s.trim().is_empty() {
                DEFAULT_BUTTON_LABEL.to_string()
            } else {
                s
            }
        } else if show {
            label()
        } else {
            saved.set_value(label());
            String::new()
        };
        state.end_group();
        sid.with_value(|id| {
            state.apply(|d| set_field(d, id, label_field.key, serde_json::Value::String(text)));
        });
        state.end_group();
    };
    let switch_id = format!("f-{id}-button-on");
    let switch_for = switch_id.clone();
    let hint_id = format!("f-{id}-button-hint");
    let (label_id, href_id) = (id.clone(), id);

    view! {
        <fieldset class="fieldset gap-3 rounded-box border border-base-300 p-3">
            <legend class="fieldset-legend px-1 text-sm font-semibold">"Botón"</legend>
            <label for=switch_for class="flex items-center justify-between gap-3">
                <span class="text-sm">"Mostrar un botón"</span>
                <input
                    id=switch_id
                    type="checkbox"
                    class="toggle toggle-sm toggle-primary"
                    prop:checked=move || on.get()
                    on:change=move |ev| toggle(event_target_checked(&ev))
                />
            </label>
            {move || on.get().then(|| {
                let hint = move || button_hint(&label(), &href());
                let hint_id = hint_id.clone();
                view! {
                    <PropField products id=label_id.clone() field=label_field/>
                    <PropField products id=href_id.clone() field=href_field/>
                    {move || hint().map(|h| view! {
                        <p id=hint_id.clone() class="flex items-start gap-1 text-xs font-medium text-amber-800" role="status">
                            <Icon name="alert" class="mt-px h-3.5 w-3.5 shrink-0"/>
                            {h}
                        </p>
                    })}
                }
            })}
        </fieldset>
    }
}

#[component]
fn FieldError(id: String, text: String) -> impl IntoView {
    view! {
        <p id=id class="flex items-start gap-1 text-xs font-medium text-red-700">
            <Icon name="alert" class="mt-px h-3.5 w-3.5"/>
            {text}
        </p>
    }
}

/// Estrellas de 1 a 5 como botones.
#[component]
fn Stars(
    label: &'static str,
    value: Signal<serde_json::Value>,
    set: Callback<serde_json::Value>,
) -> impl IntoView {
    let current = move || value.get().as_i64().unwrap_or(0);
    view! {
        <fieldset class="flex flex-col gap-1">
            <legend class="mb-1 text-sm font-medium">{label}</legend>
            <div class="flex gap-1">
                {(1..=5i64).map(|n| view! {
                    <button
                        type="button"
                        class=move || if n <= current() { "btn btn-ghost btn-sm btn-square text-amber-600" } else { "btn btn-ghost btn-sm btn-square text-base-content" }
                        aria-label=format!("{n} de 5")
                        aria-pressed=move || (n == current()).to_string()
                        on:click=move |_| set.run(serde_json::Value::from(n))
                    >
                        <svg class="h-5 w-5" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.5" aria-hidden="true"
                            fill=move || if n <= current() { "currentColor" } else { "none" }>
                            <path d="M12 3l2.8 5.7 6.2.9-4.5 4.4 1 6.2L12 17.3 6.5 20.2l1-6.2L3 9.6l6.2-.9z"/>
                        </svg>
                    </button>
                }).collect_view()}
            </div>
        </fieldset>
    }
}

/// Selector de imagen: la actual, Mis fotos (las que subió el dueño, con el
/// botón para subir más), las fotos de ejemplo y, para quien la necesite,
/// una dirección propia.
#[component]
pub(crate) fn ImagePicker(
    uid: String,
    value: Signal<serde_json::Value>,
    /// Cambio de un solo clic.
    pick: Callback<serde_json::Value>,
    /// Se eligió (o subió) una foto de Mis fotos.
    on_photo: Option<Callback<MediaItem>>,
) -> impl IntoView {
    let photos = use_context::<MyPhotos>().unwrap_or_else(MyPhotos::new);
    let open = RwSignal::new(false);
    let src = move || value.get().as_str().unwrap_or_default().to_string();
    let name = move || {
        let s = src();
        if let Some(p) = photos
            .items
            .get()
            .and_then(|l| l.into_iter().find(|p| p.url == s))
        {
            return p.name;
        }
        ui::LIBRARY_IMAGES
            .iter()
            .find(|(p, _)| *p == s)
            .map(|(_, d)| (*d).to_string())
            .unwrap_or_else(|| {
                if s.is_empty() {
                    "Sin imagen".into()
                } else if domain::media_base(&s).is_some() {
                    "Foto de Mis fotos".into()
                } else {
                    "Imagen propia".into()
                }
            })
    };
    let panel_id = format!("{uid}-panel");
    let url_id = format!("{uid}-url");
    let file_id = format!("{uid}-file");
    let mine_id = format!("{uid}-mine");
    let done = Callback::new(move |item: MediaItem| {
        pick.run(serde_json::Value::String(item.url.clone()));
        if let Some(cb) = on_photo {
            cb.run(item);
        }
        open.set(false);
    });
    view! {
        <div class="flex items-center gap-3 rounded-field border border-base-300 p-2">
            <div class="flex h-12 w-16 shrink-0 items-center justify-center overflow-hidden rounded bg-base-200">
                {move || {
                    let s = src();
                    if s.is_empty() {
                        view! { <Icon name="image" class="h-5 w-5 text-base-content/60"/> }.into_any()
                    } else {
                        view! { <img src=s alt="" class="h-full w-full object-cover"/> }.into_any()
                    }
                }}
            </div>
            <span class="min-w-0 flex-1 truncate text-sm">{name}</span>
            <button
                type="button"
                class="btn btn-sm"
                aria-expanded=move || open.get().to_string()
                aria-controls=panel_id.clone()
                on:click=move |_| {
                    open.update(|o| *o = !*o);
                    if open.get_untracked() {
                        photos.ensure_loaded();
                    }
                }
            >
                {move || if open.get() { "Cerrar" } else { "Cambiar" }}
            </button>
        </div>
        <div id=panel_id class="flex flex-col gap-2 rounded-field border border-base-300 p-2" class:hidden=move || !open.get()>
            <div class="flex items-center justify-between gap-2">
                <span class="text-xs font-semibold" id=mine_id.clone()>"Mis fotos"</span>
                // El `<label>` abre el selector de archivos; el input queda
                // oculto pero sigue siendo operable con teclado desde el label.
                <label
                    for=file_id.clone()
                    class="btn btn-primary btn-xs"
                    class:btn-disabled=move || photos.progress.get().is_some()
                    tabindex="0"
                    role="button"
                    on:keydown=move |ev| {
                        if ev.key() == "Enter" || ev.key() == " " {
                            ev.prevent_default();
                            if let Some(t) = ev.current_target() {
                                use wasm_bindgen::JsCast;
                                if let Some(el) = t.dyn_ref::<web_sys::HtmlElement>() {
                                    el.click();
                                }
                            }
                        }
                    }
                >
                    <Icon name="plus" class="h-3.5 w-3.5"/>
                    "Subir fotos"
                </label>
                <input
                    id=file_id
                    type="file"
                    class="sr-only"
                    tabindex="-1"
                    accept="image/jpeg,image/png,image/webp,image/gif,image/svg+xml"
                    multiple
                    on:change=move |ev| photos.upload(take_files(&ev), done)
                />
            </div>
            {move || {
                let errors = photos.upload_errors.get();
                (!errors.is_empty()).then(|| view! {
                    <ul class="flex flex-col gap-1 text-xs text-error" role="alert">
                        {errors.into_iter().map(|e| view! { <li>{e}</li> }).collect_view()}
                    </ul>
                })
            }}
            {move || match (photos.items.get(), photos.load_error.get()) {
                (_, Some(err)) => view! { <p class="text-xs text-error">{err}</p> }.into_any(),
                (None, None) => view! {
                    <div class="grid grid-cols-3 gap-1.5" aria-busy="true" aria-label="Cargando tus fotos">
                        <div class="skeleton aspect-[4/3] w-full"></div>
                        <div class="skeleton aspect-[4/3] w-full"></div>
                        <div class="skeleton aspect-[4/3] w-full"></div>
                    </div>
                }.into_any(),
                (Some(list), None) if list.is_empty() => view! {
                    <p class="text-xs text-base-content/70">
                        "Todavía no subiste fotos. Tocá Subir fotos para agregar las de tus productos."
                    </p>
                }.into_any(),
                (Some(list), None) => view! {
                    <ul class="grid max-h-48 grid-cols-3 gap-1.5 overflow-y-auto" aria-labelledby=mine_id.clone()>
                        {list.into_iter().map(|p| {
                            let url = p.url.clone();
                            let url2 = p.url.clone();
                            let url3 = p.url.clone();
                            let thumb = domain::media_base(&p.url)
                                .map(|b| format!("{b}/480.webp"))
                                .unwrap_or_else(|| p.url.clone());
                            view! {
                                <li>
                                    <button
                                        type="button"
                                        class="block aspect-[4/3] w-full overflow-hidden rounded border-2 bg-base-200"
                                        class:border-primary=move || src() == url
                                        class:border-transparent=move || src() != url2
                                        aria-pressed=move || (src() == url3).to_string()
                                        title=p.name.clone()
                                        on:click=move |_| {
                                            pick.run(serde_json::Value::String(p.url.clone()));
                                            if let Some(cb) = on_photo {
                                                cb.run(p.clone());
                                            }
                                            open.set(false);
                                        }
                                    >
                                        <img src=thumb alt=p.name.clone() loading="lazy" class="h-full w-full object-cover"/>
                                    </button>
                                </li>
                            }
                        }).collect_view()}
                    </ul>
                }.into_any(),
            }}
            <span class="mt-1 text-xs font-semibold">"Fotos de ejemplo"</span>
            <ul class="grid grid-cols-3 gap-1.5" aria-label="Fotos de ejemplo">
                {ui::LIBRARY_IMAGES.iter().map(|(path, desc)| view! {
                    <li>
                        <button
                            type="button"
                            class="block aspect-[4/3] w-full overflow-hidden rounded border-2 bg-base-200"
                            class:border-primary=move || src() == *path
                            class:border-transparent=move || src() != *path
                            aria-pressed=move || (src() == *path).to_string()
                            title=*desc
                            on:click=move |_| {
                                pick.run(serde_json::Value::String((*path).to_string()));
                                open.set(false);
                            }
                        >
                            <img src=*path alt=*desc loading="lazy" class="h-full w-full object-cover"/>
                        </button>
                    </li>
                }).collect_view()}
            </ul>
            <button type="button" class="btn btn-ghost btn-xs self-start" on:click=move |_| pick.run(serde_json::Value::String(String::new()))>
                "Quitar imagen"
            </button>
            <div class="flex flex-wrap items-center gap-2">
                <span class="text-xs font-medium">"¿La foto está en otro sitio?"</span>
                <LinkImport photos done uid=url_id small=true/>
            </div>
        </div>
    }
}

/// Campos de un enlace del Menú superior que usa un solo estilo de
/// desplegable: la foto, «Panel ancho con foto»; los enlaces de cada columna,
/// «En columnas». Al cambiar de estilo no se borra nada.
fn nav_sub_condition(sub: &Field) -> Option<&'static str> {
    if sub.key == "image_url" && sub.label == "Foto del panel ancho" {
        Some("panel")
    } else if sub.key == "children" && sub.label.starts_with("Enlaces de la columna") {
        Some("columns")
    } else {
        None
    }
}

/// Lista editable en acordeón: cada elemento con su título, sus campos y
/// botones para subir, bajar y quitar. Solo se redibuja entera cuando cambia
/// la cantidad.
#[component]
fn ListInput(
    products: StoredValue<Vec<Product>>,
    field: &'static Field,
    item: &'static [Field],
    min: usize,
    max: usize,
    uid: String,
    value: Signal<serde_json::Value>,
    set: Callback<serde_json::Value>,
) -> impl IntoView {
    let state = expect_context::<EditorState>();
    let items = move || {
        value
            .get_untracked()
            .as_array()
            .cloned()
            .unwrap_or_default()
    };
    let len = Memo::new(move |_| value.with(|v| v.as_array().map_or(0, Vec::len)));
    // Elemento a mostrar abierto la próxima vez que se dibuje la lista.
    let open_idx = StoredValue::new(None::<usize>);
    let update = move |f: &dyn Fn(&mut Vec<serde_json::Value>)| {
        let mut list = value
            .get_untracked()
            .as_array()
            .cloned()
            .unwrap_or_default();
        f(&mut list);
        set.run(serde_json::Value::Array(list));
    };
    let update_once = move |f: &dyn Fn(&mut Vec<serde_json::Value>)| {
        state.end_group();
        update(f);
        state.end_group();
    };
    // Estilo de desplegable del Menú superior elegido: cada enlace muestra
    // solo los campos que ese estilo usa (ver `nav_sub_condition`).
    let menu_style = Memo::new(move |_| {
        state.selected.get().map_or_else(
            || "list".to_string(),
            |id| {
                state.doc.with(|d| {
                    field_value(d, &id, "menu")
                        .as_str()
                        .filter(|s| !s.is_empty())
                        .unwrap_or("list")
                        .to_string()
                })
            },
        )
    });
    // Primer campo de texto del elemento: se usa como su título.
    let title_key = item
        .iter()
        .find(|f| matches!(f.kind, FieldKind::Text { .. }))
        .map(|f| f.key);

    view! {
        <fieldset class="flex flex-col gap-2">
            <legend class="mb-1 text-sm font-medium">{field.label}</legend>
            {move || {
                let n = len.get();
                let open_now = open_idx.get_value();
                open_idx.set_value(None);
                (0..n)
                    .map(|i| {
                        let uid = format!("{uid}-{i}");
                        let fields = item
                            .iter()
                            .map(|sub| {
                                let key = sub.key;
                                let sub_value = Signal::derive(move || {
                                    value.with(|v| v.get(i).and_then(|it| it.get(key)).cloned())
                                        .unwrap_or(serde_json::Value::Null)
                                });
                                let sub_set = Callback::new(move |v: serde_json::Value| {
                                    update(&|list| {
                                        if let Some(obj) = list.get_mut(i).and_then(|it| it.as_object_mut()) {
                                            obj.insert(key.to_string(), v.clone());
                                        }
                                    });
                                });
                                let want = nav_sub_condition(sub);
                                let shown = Memo::new(move |_| match want {
                                    None => true,
                                    Some(want) => menu_style.get() == want,
                                });
                                let sub_uid = format!("{uid}-{key}");
                                move || {
                                    shown.get().then(|| view! {
                                        <FieldInput products field=sub uid=sub_uid.clone() value=sub_value set=sub_set/>
                                    })
                                }
                            })
                            .collect_view();
                        let title = move || {
                            let t = title_key
                                .and_then(|k| value.with(|v| v.get(i).and_then(|it| it.get(k)).and_then(|s| s.as_str()).map(str::to_string)))
                                .filter(|s| !s.trim().is_empty());
                            match t {
                                Some(t) => format!("{}. {t}", i + 1),
                                None => format!("{} {}", item_noun(field.label), i + 1),
                            }
                        };
                        view! {
                            <details class="collapse collapse-arrow rounded-box border border-base-300" open=open_now == Some(i) || n == 1>
                                <summary class="collapse-title min-h-0 py-2.5 text-sm font-medium">{title}</summary>
                                <div class="collapse-content flex flex-col gap-3">
                                    <div class="flex gap-1">
                                        <button
                                            type="button"
                                            class="btn btn-ghost btn-xs"
                                            disabled={i == 0}
                                            on:click=move |_| {
                                                open_idx.set_value(Some(i - 1));
                                                update_once(&|list| list.swap(i, i - 1));
                                            }
                                        ><Icon name="up" class="h-3.5 w-3.5"/>"Subir"</button>
                                        <button
                                            type="button"
                                            class="btn btn-ghost btn-xs"
                                            disabled={i + 1 >= n}
                                            on:click=move |_| {
                                                open_idx.set_value(Some(i + 1));
                                                update_once(&|list| list.swap(i, i + 1));
                                            }
                                        ><Icon name="down" class="h-3.5 w-3.5"/>"Bajar"</button>
                                        <button
                                            type="button"
                                            class="btn btn-ghost btn-xs ml-auto text-red-700"
                                            disabled={n <= min}
                                            title=(n <= min).then(|| format!("Tiene que quedar al menos {min}"))
                                            on:click=move |_| update_once(&|list| { list.remove(i); })
                                        ><Icon name="trash" class="h-3.5 w-3.5"/>"Quitar"</button>
                                    </div>
                                    {fields}
                                </div>
                            </details>
                        }
                    })
                    .collect_view()
            }}
            <button
                type="button"
                class="btn btn-sm btn-outline"
                disabled=move || { len.get() >= max }
                on:click=move |_| {
                    let next = new_list_item(&items(), item);
                    open_idx.set_value(Some(len.get_untracked()));
                    update_once(&|list| list.push(next.clone()));
                }
            >
                <Icon name="plus"/>
                {move || format!("Agregar {} ({} de {max})", item_noun(field.label).to_lowercase(), len.get())}
            </button>
        </fieldset>
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::Node;

    #[test]
    fn set_field_changes_one_prop() {
        let mut doc = PageDoc::default();
        doc.nodes
            .push(Node::new("n1", Widget::defaults("Banner").unwrap()));
        set_field(&mut doc, "n1", "title", serde_json::json!("Hola"));
        assert_eq!(field_value(&doc, "n1", "title"), serde_json::json!("Hola"));
        assert!(matches!(&doc.nodes[0].widget, Widget::Banner(b) if b.title == "Hola"));
    }

    #[test]
    fn set_field_replaces_a_whole_list() {
        let mut doc = PageDoc::default();
        doc.nodes
            .push(Node::new("n1", Widget::defaults("Faq").unwrap()));
        let mut items = field_value(&doc, "n1", "items");
        let list = items.as_array_mut().unwrap();
        list.push(new_list_item(list, &[]));
        list[3]["question"] = serde_json::json!("¿Nueva?");
        set_field(&mut doc, "n1", "items", items);
        assert!(
            matches!(&doc.nodes[0].widget, Widget::Faq(f) if f.items.len() == 4 && f.items[3].question == "¿Nueva?")
        );
        assert_eq!(domain::validate(&doc), Ok(()));
    }

    #[test]
    fn new_list_item_keeps_the_photo_but_not_the_texts() {
        let fields = Widget::defaults("Categories").unwrap().fields();
        let list = fields.iter().find(|f| f.key == "items").unwrap();
        let FieldKind::List { item, .. } = &list.kind else {
            panic!("se esperaba una lista");
        };
        let last = serde_json::json!({
            "image_url": "/media/a.avif", "label": "Ofertas", "href": "/ofertas"
        });
        let v = new_list_item(std::slice::from_ref(&last), item);
        assert_eq!(v["image_url"], "/media/a.avif");
        assert_eq!(v["label"], "");
        assert_eq!(v["href"], "");
    }

    #[test]
    fn new_list_item_has_every_field() {
        let fields = Widget::defaults("Testimonials").unwrap().fields();
        let list = fields.iter().find(|f| f.key == "items").unwrap();
        let FieldKind::List { item, .. } = &list.kind else {
            panic!("se esperaba una lista");
        };
        let v = new_list_item(&[], item);
        assert_eq!(v["text"], "");
        assert_eq!(v["name"], "");
        assert_eq!(v["rating"], 1);
    }

    #[test]
    fn counter_shows_near_the_limit() {
        assert!(!near_limit(0, 160));
        assert!(!near_limit(127, 160));
        assert!(near_limit(128, 160));
        assert!(near_limit(30, 30));
    }

    #[test]
    fn button_hint_says_what_is_missing() {
        assert!(button_hint("", "/ofertas").unwrap().starts_with("Escribí"));
        assert!(button_hint("Ver ofertas", " ")
            .unwrap()
            .starts_with("Falta adónde"));
        assert_eq!(button_hint("Ver ofertas", "/ofertas"), None);
    }

    #[test]
    fn image_fields_use_the_picker() {
        let banner = Widget::defaults("Banner").unwrap();
        let image = banner
            .fields()
            .iter()
            .find(|f| f.key == "image_url")
            .unwrap();
        let link = banner
            .fields()
            .iter()
            .find(|f| f.key == "button_href")
            .unwrap();
        assert!(is_image(image));
        assert!(!is_image(link));
    }
}

/// Cómo se llama un elemento de la lista `label` ("Enlaces" → "Enlace"),
/// para "Enlace 3" y "Agregar enlace": los nombres largos de las listas no
/// sirven para eso.
fn item_noun(label: &str) -> &'static str {
    if label.starts_with("Enlaces de la columna") {
        return "Enlace de la columna";
    }
    let first = label.split_whitespace().next().unwrap_or_default();
    match first {
        "Enlaces" => "Enlace",
        "Opciones" => "Opción",
        "Diapositivas" => "Diapositiva",
        "Categorías" => "Categoría",
        "Beneficios" => "Beneficio",
        "Pestañas" => "Pestaña",
        "Preguntas" => "Pregunta",
        "Imágenes" | "Fotos" => "Imagen",
        "Testimonios" => "Testimonio",
        "Marcas" => "Marca",
        "Mensajes" => "Mensaje",
        "Pasos" => "Paso",
        "Cifras" | "Números" => "Cifra",
        "Redes" => "Red",
        _ => "Elemento",
    }
}

/// El código del bloque Cupón: se elige entre los cupones de la tienda que
/// sirven hoy, con un enlace para crear uno nuevo en el panel.
#[component]
fn CouponPicker(
    uid: String,
    value: Signal<serde_json::Value>,
    set: Callback<serde_json::Value>,
) -> impl IntoView {
    let list = crate::coupons();
    let current = move || value.get().as_str().unwrap_or_default().to_string();
    let inactive = {
        let list = list.clone();
        move || {
            let c = current();
            !c.is_empty() && !list.iter().any(|b| b.code.eq_ignore_ascii_case(&c))
        }
    };
    let new_href = use_context::<crate::PanelLink>()
        .map(|crate::PanelLink(h)| format!("{}/cupones", h.trim_end_matches('/')));
    let help_id = format!("{uid}-help");
    let opts = StoredValue::new(list.clone());
    view! {
        <div class="flex flex-col gap-1">
            <label for=uid.clone() class="text-sm font-medium">"Cupón"</label>
            {if list.is_empty() {
                view! {
                    <p id=uid.clone() class="rounded-lg bg-base-200 px-3 py-2 text-xs text-base-content/75">
                        "Todavía no tenés cupones activos. Creá uno y volvé a abrir el editor para elegirlo."
                    </p>
                }.into_any()
            } else {
                view! {
                    <select
                        id=uid.clone()
                        class="select select-sm w-full font-mono"
                        class:select-error=inactive.clone()
                        aria-describedby=help_id.clone()
                        on:change=move |ev| set.run(serde_json::Value::String(event_target_value(&ev)))
                    >
                        {move || {
                            let c = current();
                            let known = opts.with_value(|l| l.iter().any(|b| b.code.eq_ignore_ascii_case(&c)));
                            view! {
                                {(!known).then(|| {
                                    let label = if c.is_empty() {
                                        "Elegí un cupón…".to_string()
                                    } else {
                                        format!("{c} (pausado o vencido)")
                                    };
                                    view! { <option value=c.clone() disabled selected>{label}</option> }
                                })}
                                {opts.get_value().into_iter().map(|b| {
                                    let sel = b.code.eq_ignore_ascii_case(&c);
                                    view! { <option value=b.code.clone() selected=sel>{format!("{} · {}", b.code, b.detail)}</option> }
                                }).collect_view()}
                            }
                        }}
                    </select>
                }.into_any()
            }}
            {move || inactive().then(|| view! {
                <p class="text-xs text-error">"Este cupón está pausado o venció: el bloque no se ve en tu tienda."</p>
            })}
            <p id=help_id class="text-xs text-base-content/75">
                "Solo aparecen tus cupones activos. Si lo pausás o vence, el bloque deja de verse en la tienda."
            </p>
            {new_href.map(|href| view! {
                <a class="link link-primary self-start text-sm" href=href target="_blank" rel="noopener">
                    "+ Crear un cupón nuevo"
                </a>
            })}
        </div>
    }
}

/// Elegir un producto: buscador que achica la lista al escribir, cada fila con
/// foto y precio, y los agotados al final con su marca. Son radios nativos:
/// las flechas pasan de uno a otro.
#[component]
fn ProductPicker(
    uid: String,
    products: StoredValue<Vec<Product>>,
    value: Signal<String>,
    pick: Callback<String>,
) -> impl IntoView {
    let query = RwSignal::new(String::new());
    let total = products.with_value(Vec::len);
    let fold = |s: &str| {
        s.to_lowercase()
            .chars()
            .map(|c| match c {
                'á' | 'à' | 'ä' => 'a',
                'é' | 'è' | 'ë' => 'e',
                'í' | 'ì' | 'ï' => 'i',
                'ó' | 'ò' | 'ö' => 'o',
                'ú' | 'ù' | 'ü' => 'u',
                c => c,
            })
            .collect::<String>()
    };
    let shown = Memo::new(move |_| {
        let q = fold(query.get().trim());
        let mut list: Vec<Product> = products.with_value(|ps| {
            ps.iter()
                .filter(|p| q.is_empty() || fold(&p.name).contains(&q))
                .cloned()
                .collect()
        });
        // Los agotados al final; el resto queda en el orden del catálogo.
        list.sort_by_key(|p| !p.in_stock);
        list
    });
    let list_id = format!("{uid}-list");
    let count_id = format!("{uid}-count");
    view! {
        <div class="flex flex-col gap-2">
            <label class="input input-sm w-full">
                <span class="sr-only">"Buscar producto"</span>
                <crate::icons::Icon name="search" class="h-4 w-4 opacity-70"/>
                <input id=uid.clone() type="search" placeholder="Buscar por nombre…"
                    aria-controls=list_id.clone() aria-describedby=count_id.clone()
                    prop:value=move || query.get()
                    on:input=move |ev| query.set(event_target_value(&ev))/>
            </label>
            <div id=list_id role="radiogroup" aria-label="Productos" class="mk-ppick flex max-h-72 flex-col gap-1 overflow-y-auto">
                <For each=move || shown.get() key=|p| p.id.clone() let:p>
                    {
                        let id = p.id.clone();
                        let pid = StoredValue::new(p.id.clone());
                        let checked = move || pid.with_value(|id| value.get() == *id);
                        view! {
                            <label class="mk-ppick-row" class:is-on=checked class:is-out=!p.in_stock>
                                <input type="radio" class="sr-only" name=format!("{uid}-radio") value=id
                                    prop:checked=checked
                                    on:change=move |_| pick.run(pid.get_value())/>
                                <img src=p.image_url.clone() alt="" loading="lazy" class="h-10 w-10 shrink-0 rounded-md bg-base-200 object-cover"/>
                                <span class="flex min-w-0 flex-1 flex-col">
                                    <span class="truncate text-sm font-medium">{p.name.clone()}</span>
                                    <span class="text-xs text-base-content/70">{ui::format_price(p.price_cents)}</span>
                                </span>
                                {(!p.in_stock).then(|| view! { <span class="badge badge-ghost badge-sm">"Agotado"</span> })}
                                <span class="mk-ppick-check" aria-hidden="true">"✓"</span>
                            </label>
                        }
                    }
                </For>
            </div>
            <p id=count_id class="text-xs text-base-content/70" aria-live="polite">
                {move || {
                    let n = shown.with(Vec::len);
                    if n == 0 {
                        "No hay productos con ese nombre.".to_string()
                    } else if n == total {
                        format!("{total} productos. Los agotados van al final.")
                    } else {
                        format!("{n} de {total} productos. Los agotados van al final.")
                    }
                }}
            </p>
        </div>
    }
}

/// Foto de un acceso: selector flotante con Mis fotos, subir desde la
/// computadora y pegar un enlace. Se cierra con Esc o tocando afuera.
#[component]
fn PhotoPopover(
    uid: String,
    current: Signal<String>,
    pick: Callback<String>,
    open: RwSignal<bool>,
) -> impl IntoView {
    let photos = use_context::<MyPhotos>().unwrap_or_else(MyPhotos::new);
    photos.ensure_loaded();
    let file_id = StoredValue::new(format!("{uid}-file"));
    let link_open = RwSignal::new(false);
    let done = Callback::new(move |item: MediaItem| {
        pick.run(item.url);
        open.set(false);
    });
    view! {
        <div class="fixed inset-0 z-20" on:click=move |_| open.set(false)></div>
        <div
            class="absolute left-0 top-full z-30 mt-2 w-full rounded-box bg-base-100 p-4 shadow-xl ring-1 ring-base-300"
            role="dialog"
            aria-label="Elegir la foto"
            on:keydown=move |ev| if ev.key() == "Escape" { open.set(false) }
        >
            <p class="mb-3 text-xs text-base-content/70">"Elegí una de tus fotos"</p>
            {move || match (photos.items.get(), photos.load_error.get()) {
                (_, Some(err)) => view! { <p class="text-xs text-error">{err}</p> }.into_any(),
                (None, None) => view! {
                    <div class="grid grid-cols-3 gap-2" aria-busy="true">
                        <div class="skeleton aspect-square w-full"></div>
                        <div class="skeleton aspect-square w-full"></div>
                        <div class="skeleton aspect-square w-full"></div>
                    </div>
                }.into_any(),
                (Some(list), None) => view! {
                    <ul class="grid max-h-56 grid-cols-3 gap-2 overflow-y-auto p-1">
                        {list.into_iter().map(|p| {
                            let url = p.url.clone();
                            let url2 = p.url.clone();
                            let url3 = p.url.clone();
                            let thumb = domain::media_base(&p.url)
                                .map(|b| format!("{b}/480.webp"))
                                .unwrap_or_else(|| p.url.clone());
                            view! {
                                <li>
                                    <button
                                        type="button"
                                        class="block aspect-square w-full overflow-hidden rounded-box bg-base-200"
                                        class:ring-2=move || current.get() == url
                                        class:ring-primary=move || current.get() == url2
                                        class:ring-offset-2=move || current.get() == url3
                                        aria-label=p.name.clone()
                                        on:click={
                                            let u = p.url.clone();
                                            move |_| { pick.run(u.clone()); open.set(false); }
                                        }
                                    >
                                        <img src=thumb alt="" loading="lazy" class="h-full w-full object-cover"/>
                                    </button>
                                </li>
                            }
                        }).collect_view()}
                        <li>
                            <label
                                for=file_id.get_value()
                                class="grid aspect-square w-full cursor-pointer place-items-center rounded-box border border-dashed border-base-content/30 text-base-content/70"
                                tabindex="0"
                                role="button"
                                aria-label="Subir una foto nueva"
                            >
                                <Icon name="plus" class="h-5 w-5"/>
                            </label>
                        </li>
                    </ul>
                }.into_any(),
            }}
            {move || {
                let errors = photos.upload_errors.get();
                (!errors.is_empty()).then(|| view! {
                    <ul class="mt-2 text-xs text-error" role="alert">
                        {errors.into_iter().map(|e| view! { <li>{e}</li> }).collect_view()}
                    </ul>
                })
            }}
            <input
                id=file_id.get_value()
                type="file"
                class="sr-only"
                tabindex="-1"
                accept="image/jpeg,image/png,image/webp,image/gif,image/svg+xml"
                on:change=move |ev| photos.upload(take_files(&ev), done)
            />
            <div class="mt-4 flex items-center justify-between text-sm font-semibold text-primary">
                <label for=file_id.get_value() class="cursor-pointer">"Subir desde la compu"</label>
                <button type="button" class="font-semibold" on:click=move |_| link_open.update(|o| *o = !*o)>
                    "Pegar enlace"
                </button>
            </div>
            <div class="mt-3" class:hidden=move || !link_open.get()>
                <LinkImport photos done uid=format!("{uid}-link") small=true/>
            </div>
        </div>
    }
}

/// Lista en filas limpias, sin acordeón: Beneficios (ícono a la vista) y
/// Accesos (foto, nombre y destino).
#[component]
fn RowsList(
    products: StoredValue<Vec<Product>>,
    field: &'static Field,
    item: &'static [Field],
    min: usize,
    max: usize,
    uid: String,
    value: Signal<serde_json::Value>,
    set: Callback<serde_json::Value>,
) -> impl IntoView {
    let state = expect_context::<EditorState>();
    let benefits = field.label == "Beneficios";
    let len = Memo::new(move |_| value.with(|v| v.as_array().map_or(0, Vec::len)));
    let update = move |f: &dyn Fn(&mut Vec<serde_json::Value>)| {
        let mut list = value.get_untracked().as_array().cloned().unwrap_or_default();
        f(&mut list);
        set.run(serde_json::Value::Array(list));
    };
    let update_once = move |f: &dyn Fn(&mut Vec<serde_json::Value>)| {
        state.end_group();
        update(f);
        state.end_group();
    };
    let set_key = move |i: usize, key: &'static str, v: serde_json::Value| {
        update(&|list| {
            if let Some(obj) = list.get_mut(i).and_then(|it| it.as_object_mut()) {
                obj.insert(key.to_string(), v.clone());
            }
        });
    };
    let get_str = move |i: usize, key: &'static str| {
        value.with(|v| {
            v.get(i)
                .and_then(|it| it.get(key))
                .and_then(|s| s.as_str())
                .unwrap_or_default()
                .to_string()
        })
    };
    let href_field = item.iter().find(|f| f.key == "href");

    view! {
        <fieldset class="flex flex-col">
            <legend class="mb-1 text-xs font-semibold uppercase tracking-wide text-base-content/70">{field.label}</legend>
            {move || {
                let n = len.get();
                (0..n).map(|i| {
                    let row_uid = format!("{uid}-{i}");
                    let photo_open = RwSignal::new(false);
                    let dest_open = RwSignal::new(false);
                    let controls = view! {
                        <div class="flex shrink-0 flex-col items-center">
                            <button type="button" class="btn btn-ghost btn-xs btn-square" aria-label="Subir"
                                disabled={i == 0}
                                on:click=move |_| update_once(&|l| l.swap(i, i - 1))>
                                <Icon name="up" class="h-3.5 w-3.5"/>
                            </button>
                            <button type="button" class="btn btn-ghost btn-xs btn-square" aria-label="Bajar"
                                disabled={i + 1 >= n}
                                on:click=move |_| update_once(&|l| l.swap(i, i + 1))>
                                <Icon name="down" class="h-3.5 w-3.5"/>
                            </button>
                            <button type="button" class="btn btn-ghost btn-xs btn-square text-base-content/60" aria-label="Quitar"
                                disabled={n <= min}
                                title=(n <= min).then(|| format!("Tiene que quedar al menos {min}"))
                                on:click=move |_| update_once(&|l| { l.remove(i); })>
                                <Icon name="trash" class="h-3.5 w-3.5"/>
                            </button>
                        </div>
                    };
                    if benefits {
                        let icon = move || get_str(i, "icon");
                        view! {
                            <div class="flex items-start gap-4 border-t border-base-200 py-4 first:border-t-0">
                                <div class="grid h-11 w-11 shrink-0 place-items-center rounded-box bg-base-200">
                                    {move || {
                                        let cur = icon();
                                        let name = domain::BENEFIT_ICON_OPTIONS.iter().find(|(k, _)| *k == cur).map_or("star", |(k, _)| *k);
                                        view! { <Icon name=name class="h-5 w-5"/> }
                                    }}
                                </div>
                                <div class="flex min-w-0 flex-1 flex-col gap-2">
                                    <input type="text" maxlength="40" aria-label="Título del beneficio"
                                        class="input input-sm w-full font-semibold"
                                        prop:value=move || get_str(i, "title")
                                        on:input=move |ev| set_key(i, "title", event_target_value(&ev).into())/>
                                    <input type="text" maxlength="100" aria-label="Detalle del beneficio"
                                        class="input input-sm w-full"
                                        prop:value=move || get_str(i, "text")
                                        on:input=move |ev| set_key(i, "text", event_target_value(&ev).into())/>
                                    <div class="flex flex-wrap gap-1.5" role="group" aria-label="Ícono">
                                        {domain::BENEFIT_ICON_OPTIONS.iter().map(|(k, label)| view! {
                                            <button type="button"
                                                class="grid h-8 w-8 place-items-center rounded-field bg-base-200 text-base-content"
                                                class:ring-2=move || icon() == *k
                                                class:ring-primary=move || icon() == *k
                                                aria-label=*label
                                                aria-pressed=move || (icon() == *k).to_string()
                                                title=*label
                                                on:click=move |_| set_key(i, "icon", (*k).into())>
                                                <Icon name=*k class="h-4 w-4"/>
                                            </button>
                                        }).collect_view()}
                                    </div>
                                </div>
                                {controls}
                            </div>
                        }.into_any()
                    } else {
                        let img = Signal::derive(move || get_str(i, "image_url"));
                        let href_value = Signal::derive(move || {
                            value.with(|v| v.get(i).and_then(|it| it.get("href")).cloned())
                                .unwrap_or(serde_json::Value::Null)
                        });
                        let href_set = Callback::new(move |v: serde_json::Value| set_key(i, "href", v));
                        view! {
                            <div class="relative border-t border-base-200 py-4 first:border-t-0">
                                <div class="flex items-center gap-4">
                                    <button type="button"
                                        class="h-14 w-14 shrink-0 overflow-hidden rounded-box bg-base-200"
                                        class:ring-2=move || photo_open.get()
                                        class:ring-primary=move || photo_open.get()
                                        class:ring-offset-2=move || photo_open.get()
                                        aria-label="Cambiar la foto"
                                        aria-expanded=move || photo_open.get().to_string()
                                        on:click=move |_| photo_open.update(|o| *o = !*o)>
                                        {move || {
                                            let s = img.get();
                                            if s.is_empty() {
                                                view! { <Icon name="image" class="mx-auto h-5 w-5 text-base-content/60"/> }.into_any()
                                            } else {
                                                view! { <img src=s alt="" class="h-full w-full object-cover"/> }.into_any()
                                            }
                                        }}
                                    </button>
                                    <div class="min-w-0 flex-1">
                                        <input type="text" maxlength="40" aria-label="Nombre del acceso"
                                            class="w-full border-0 border-b border-base-300 bg-transparent py-1 text-sm font-semibold focus:border-primary focus:outline-none"
                                            prop:value=move || get_str(i, "label")
                                            on:input=move |ev| set_key(i, "label", event_target_value(&ev).into())/>
                                        <div class="mt-1.5 flex items-center justify-between gap-2 text-xs text-base-content/70">
                                            <span class="truncate">{move || {
                                                let h = get_str(i, "href");
                                                if h.is_empty() { "Sin destino".to_string() } else { h }
                                            }}</span>
                                            <button type="button" class="shrink-0 font-semibold text-primary"
                                                aria-expanded=move || dest_open.get().to_string()
                                                on:click=move |_| dest_open.update(|o| *o = !*o)>
                                                {move || if dest_open.get() { "Listo" } else { "Cambiar" }}
                                            </button>
                                        </div>
                                    </div>
                                    {controls}
                                </div>
                                <div class="mt-3" class:hidden=move || !dest_open.get()>
                                    {href_field.map(|hf| view! {
                                        <FieldInput products field=hf uid=format!("{row_uid}-href") value=href_value set=href_set/>
                                    })}
                                </div>
                                {move || photo_open.get().then(|| view! {
                                    <PhotoPopover uid=row_uid.clone() current=img
                                        pick=Callback::new(move |u: String| set_key(i, "image_url", u.into()))
                                        open=photo_open/>
                                })}
                            </div>
                        }.into_any()
                    }
                }).collect_view()
            }}
            <button type="button" class="btn btn-sm btn-outline mt-2"
                disabled=move || { len.get() >= max }
                on:click=move |_| {
                    let next = new_list_item(&value.get_untracked().as_array().cloned().unwrap_or_default(), item);
                    update_once(&|list| list.push(next.clone()));
                }>
                <Icon name="plus"/>
                {move || format!("Agregar {} ({} de {max})", item_noun(field.label).to_lowercase(), len.get())}
            </button>
        </fieldset>
    }
}
