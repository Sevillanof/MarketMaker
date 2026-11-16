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
        FieldKind::Number { min, .. } => Value::from(*min),
        FieldKind::Bool => Value::Bool(false),
        FieldKind::Select { options } => {
            Value::String(options.first().map(|(k, _)| *k).unwrap_or_default().into())
        }
        FieldKind::List { .. } => Value::Array(Vec::new()),
        _ => Value::String(String::new()),
    }
}

/// Elemento nuevo para una lista: copia del último (así hereda imagen y
/// estilo) o, si está vacía, uno con los valores iniciales de cada campo.
pub(crate) fn new_list_item(items: &[serde_json::Value], item: &[Field]) -> serde_json::Value {
    items.last().cloned().unwrap_or_else(|| {
        serde_json::Value::Object(
            item.iter()
                .map(|f| (f.key.to_string(), empty_value(&f.kind)))
                .collect(),
        )
    })
}

/// Explicación breve de los campos que no se entienden solos.
fn help(key: &str) -> Option<&'static str> {
    Some(match key {
        "button_href" | "href" => "Adónde lleva al hacer clic. Una página de tu tienda (/ofertas) o una dirección completa (https://…).",
        "button_label" => "Si lo dejás vacío, no se muestra el botón.",
        "alt" => "Describí la imagen para quien no la puede ver. Ayuda también a los buscadores.",
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
            state.apply_grouped(Some(&group), |d| set_field(d, rid, key, v));
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
        return view! { <ListInput products field item min=*min max=*max uid value set/> }
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
        FieldKind::Url => view! {
            <input
                type="text"
                inputmode="url"
                id=uid.clone()
                class="input input-sm w-full"
                class:input-error=move || error.with(Option::is_some)
                placeholder="/ofertas o https://…"
                aria-invalid=invalid
                aria-describedby=described.clone()
                prop:value=text
                on:input=move |ev| set_str(event_target_value(&ev))
                on:blur=move |_| end()
            />
        }
        .into_any(),
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
            view! {
                <input
                    type="number"
                    id=uid.clone()
                    class="input input-sm w-full"
                    min=min
                    max=max
                    aria-describedby=described.clone()
                    prop:value=move || value.get().as_i64().map(|n| n.to_string()).unwrap_or_default()
                    on:input=move |ev| {
                        if let Ok(n) = event_target_value(&ev).parse::<i64>() {
                            set.run(serde_json::Value::from(n.clamp(min, max)));
                        }
                    }
                    on:blur=move |_| end()
                />
            }
            .into_any()
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
            <select
                id=uid.clone()
                class="select select-sm w-full"
                aria-describedby=described.clone()
                prop:value=text
                on:change=move |ev| set_once(serde_json::Value::String(event_target_value(&ev)))
            >
                <option value="">"Elegí un producto"</option>
                {products
                    .get_value()
                    .into_iter()
                    .map(|p| view! {
                        <option value=p.id.clone()>{format!("{} · {}", p.name, ui::format_price(p.price_cents))}</option>
                    })
                    .collect_view()}
            </select>
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
        (false, true) => Some("Falta adónde lleva. Sin eso, el botón no hace nada al tocarlo."),
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
    let toggle = move |show: bool| {
        on.set(show);
        let text = if show && label().trim().is_empty() {
            DEFAULT_BUTTON_LABEL.to_string()
        } else if show {
            label()
        } else {
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
fn ImagePicker(
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
            <p class="text-xs text-base-content/70" role="status" aria-live="polite">
                {move || photos.progress.get().unwrap_or_default()}
            </p>
            {move || {
                let saved = photos.saved.get();
                (!saved.is_empty()).then(|| view! {
                    <ul class="flex flex-col gap-0.5 text-xs text-base-content/80" aria-label="Fotos optimizadas">
                        {saved.into_iter().map(|t| view! { <li>{t}</li> }).collect_view()}
                    </ul>
                })
            }}
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
                                view! { <FieldInput products field=sub uid=format!("{uid}-{key}") value=sub_value set=sub_set/> }
                            })
                            .collect_view();
                        let title = move || {
                            let t = title_key
                                .and_then(|k| value.with(|v| v.get(i).and_then(|it| it.get(k)).and_then(|s| s.as_str()).map(str::to_string)))
                                .filter(|s| !s.trim().is_empty());
                            match t {
                                Some(t) => format!("{}. {t}", i + 1),
                                None => format!("{} {}", field.label, i + 1),
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
                {move || format!("Agregar ({} de {max})", len.get())}
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
    fn new_list_item_has_every_field() {
        let fields = Widget::defaults("Testimonials").unwrap().fields();
        let FieldKind::List { item, .. } = &fields[1].kind else {
            panic!("se esperaba una lista");
        };
        let v = new_list_item(&[], item);
        assert_eq!(
            v,
            serde_json::json!({ "name": "", "text": "", "rating": 1 })
        );
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
