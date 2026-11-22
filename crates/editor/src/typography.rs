//! Letras: la sección "Letras" del inspector (las de toda la
//! tienda están en el cajón "Mi tienda", ver `store_drawer`).
//!
//! Pocas opciones y con muestra: se elige una combinación de fuentes de un
//! clic, y quien quiera puede elegir cada fuente por separado.

use domain::{
    animation_extra, font, font_css, TextStyle, ALIGN_OPTIONS, BLOCK_SIZES, FONTS, TEXT_ANIMATIONS,
};
use leptos::html::Fieldset;
use leptos::prelude::*;
use wasm_bindgen::JsCast;
use widgets::{AnimText, TextAnim};

use crate::icons::Icon;
use crate::{choice_class, EditorState};

fn font_label(id: &str) -> &'static str {
    font(id).map_or("Simple", |f| f.label)
}

/// Selector de una fuente con una muestra escrita en ella.
#[component]
fn FontSelect(
    label: &'static str,
    #[prop(into)] value: Signal<String>,
    sample: &'static str,
    on_pick: Callback<String>,
    /// Primera opción, que deja el valor vacío (por ejemplo "Igual que la tienda").
    #[prop(optional, into)]
    inherit: Option<Signal<String>>,
) -> impl IntoView {
    let preview_font = Signal::derive(move || {
        let v = value.get();
        if v.is_empty() {
            inherit.map(|i| i.get()).unwrap_or_default()
        } else {
            v
        }
    });
    view! {
        <label class="flex flex-col gap-1">
            <span class="text-sm font-medium">{label}</span>
            <select
                class="select select-sm w-full"
                prop:value=move || value.get()
                on:change=move |ev| on_pick.run(event_target_value(&ev))
            >
                {inherit.map(|i| view! {
                    <option value="">{move || format!("Igual que la tienda ({})", font_label(&i.get()))}</option>
                })}
                {FONTS.iter().map(|f| view! {
                    <option value=f.id>{format!("{} · {}", f.label, f.hint)}</option>
                }).collect_view()}
            </select>
            <span
                class="truncate rounded-field bg-base-200 px-2 py-1 text-base"
                style=move || format!("font-family:{}", font_css(&preview_font.get()))
                aria-hidden="true"
            >{sample}</span>
        </label>
    }
}

/// Todas las opciones de animación; la primera es "Ninguna".
fn anim_items() -> Vec<(&'static str, &'static str, &'static str)> {
    std::iter::once((
        "",
        "Ninguna",
        "El texto se muestra quieto, como hasta ahora.",
    ))
    .chain(TEXT_ANIMATIONS.iter().copied())
    .collect()
}

/// El texto de muestra y el dato extra con que se ve una animación:
/// grande en la vista previa, corto en la lista.
fn anim_sample_text(id: &str, name: &'static str, extra: &str, big: bool) -> (String, String) {
    let ex = |fallback: &str| {
        if extra.trim().is_empty() {
            fallback.to_string()
        } else {
            extra.to_string()
        }
    };
    match (id, big) {
        ("split", true) => ("OFERTA".into(), ex("GRATIS")),
        ("split", false) => (name.into(), "¡HEY!".into()),
        ("strike", true) => ("$29.900".into(), ex("$49.900")),
        ("strike", false) => ("Tachar".into(), "$99".into()),
        ("rotating", true) => ("Tu tienda,".into(), ex("rápida, simple, tuya")),
        ("rotating", false) => ("Palabra que".into(), "gira, rota".into()),
        ("count", true) => ("+12.480 pedidos".into(), String::new()),
        ("count", false) => ("1.280".into(), String::new()),
        (_, true) => ("Hola prueba".into(), String::new()),
        _ => (name.into(), String::new()),
    }
}

/// Un texto con la animación `id` (o quieto si `id` está vacío).
fn anim_sample(id: &'static str, text: String, extra: String) -> AnyView {
    if id.is_empty() {
        return text.into_any();
    }
    let anim = TextAnim {
        id: id.to_string(),
        extra,
    };
    view! {
        <span class=format!("mk-anim-{id}")>
            <leptos::context::Provider value=anim><AnimText text=text/></leptos::context::Provider>
        </span>
    }
    .into_any()
}

/// "Animación" de Letras: una vista previa grande con flechas para recorrer
/// las animaciones y, debajo, un desplegable cerrado con todas. Al pasar el
/// mouse por una opción esa opción se mueve; al elegirla, pasa a la vista
/// previa y al lienzo.
#[component]
fn AnimationPicker(
    #[prop(into)] animation: Signal<String>,
    #[prop(into)] extra: Signal<String>,
    on_animation: Callback<String>,
    on_extra: Callback<String>,
) -> impl IntoView {
    let items = StoredValue::new(anim_items());
    let open = RwSignal::new(false);
    let hot = RwSignal::new(None::<usize>);
    let root = NodeRef::<Fieldset>::new();
    let current = move || {
        let id = animation.get();
        items.with_value(|it| it.iter().position(|(k, _, _)| *k == id).unwrap_or(0))
    };
    let pick = move |i: usize| {
        let id = items.with_value(|it| it[i].0);
        on_animation.run(id.to_string());
        open.set(false);
        hot.set(None);
    };
    let step = move |d: isize| {
        let n = items.with_value(|it| it.len()) as isize;
        let i = (current() as isize + d).rem_euclid(n) as usize;
        on_animation.run(items.with_value(|it| it[i].0).to_string());
    };
    // Tocar fuera del desplegable lo cierra.
    let outside = window_event_listener(leptos::ev::pointerdown, move |ev| {
        if !open.get_untracked() {
            return;
        }
        let inside = ev
            .target()
            .and_then(|t| t.dyn_into::<web_sys::Node>().ok())
            .zip(root.get_untracked())
            .is_some_and(|(t, n)| n.contains(Some(&t)));
        if !inside {
            open.set(false);
            hot.set(None);
        }
    });
    on_cleanup(move || outside.remove());
    let on_key = move |ev: leptos::ev::KeyboardEvent| {
        let n = items.with_value(|it| it.len());
        match ev.key().as_str() {
            "Escape" if open.get_untracked() => {
                ev.prevent_default();
                open.set(false);
                hot.set(None);
            }
            "ArrowDown" | "ArrowUp" => {
                ev.prevent_default();
                let down = ev.key() == "ArrowDown";
                open.set(true);
                let from = hot.get_untracked().unwrap_or_else(current);
                hot.set(Some(if down {
                    (from + 1) % n
                } else {
                    (from + n - 1) % n
                }));
            }
            "Enter" if open.get_untracked() => {
                if let Some(i) = hot.get_untracked() {
                    ev.prevent_default();
                    pick(i);
                }
            }
            _ => {}
        }
    };
    let extra_spec = Signal::derive(move || animation_extra(&animation.get()));
    let hint = Signal::derive(move || match animation.get().as_str() {
        "count" => Some("Cuenta hasta el primer número del título, por ejemplo «+12.480 pedidos»."),
        "scramble" | "typewriter" => None,
        _ => None,
    });

    view! {
        <fieldset class="flex flex-col gap-2" node_ref=root>
            <legend class="mb-1 text-sm font-medium">"Animación"</legend>
            <div class="relative flex min-h-24 items-center justify-center overflow-hidden rounded-box border border-base-300 px-11 py-3 text-center text-2xl font-extrabold">
                <button
                    type="button"
                    class="btn btn-circle btn-ghost btn-sm absolute left-1.5 top-1/2 -translate-y-1/2"
                    aria-label="Animación anterior"
                    on:click=move |_| step(-1)
                ><Icon name="chevron-left"/></button>
                <div role="img" aria-label="Vista previa de la animación" class="min-w-0">
                    {move || {
                        let i = current();
                        let (id, name, _) = items.with_value(|it| it[i]);
                        let (text, ex) = anim_sample_text(id, name, &extra.get(), true);
                        anim_sample(id, text, ex)
                    }}
                </div>
                <button
                    type="button"
                    class="btn btn-circle btn-ghost btn-sm absolute right-1.5 top-1/2 -translate-y-1/2"
                    aria-label="Animación siguiente"
                    on:click=move |_| step(1)
                ><Icon name="chevron-right"/></button>
            </div>
            <div class="text-center">
                <p class="text-sm font-semibold">{move || items.with_value(|it| it[current()].1)}</p>
                <p class="text-xs opacity-70">{move || items.with_value(|it| it[current()].2)}</p>
            </div>
            <div class="relative">
                <button
                    type="button"
                    class="select select-sm w-full items-center justify-between text-left"
                    aria-haspopup="listbox"
                    aria-expanded=move || open.get().to_string()
                    aria-label="Elegir animación de la lista"
                    on:click=move |_| {
                        hot.set(None);
                        open.update(|o| *o = !*o);
                    }
                    on:keydown=on_key
                >
                    {move || items.with_value(|it| it[current()].1)}
                </button>
                <Show when=move || open.get()>
                    <ul
                        role="listbox"
                        class="absolute inset-x-0 top-full z-30 mt-1 max-h-72 overflow-y-auto rounded-box border border-base-300 bg-base-100 p-1 shadow-lg"
                    >
                        {(0..items.with_value(|it| it.len())).map(|i| {
                            let (id, name, _) = items.with_value(|it| it[i]);
                            let selected = move || current() == i;
                            view! {
                                <li
                                    role="option"
                                    aria-selected=move || selected().to_string()
                                    class=move || format!(
                                        "flex min-h-10 cursor-pointer items-center justify-between overflow-hidden rounded-field px-3 {} {}",
                                        if id.is_empty() { "text-sm opacity-70" } else { "text-base font-bold" },
                                        if hot.get() == Some(i) { "bg-primary/10" } else { "" },
                                    )
                                    on:mouseenter=move |_| hot.set(Some(i))
                                    on:mouseleave=move |_| hot.update(|h| if *h == Some(i) { *h = None })
                                    on:click=move |_| pick(i)
                                >
                                    <span class="min-w-0 truncate">
                                        {move || if hot.get() == Some(i) && !id.is_empty() {
                                            let (text, ex) = anim_sample_text(id, name, "", false);
                                            anim_sample(id, text, ex)
                                        } else {
                                            name.into_any()
                                        }}
                                    </span>
                                    {move || selected().then(|| view! { <span class="text-primary"><Icon name="check"/></span> })}
                                </li>
                            }
                        }).collect_view()}
                    </ul>
                </Show>
            </div>
            {move || extra_spec.get().map(|(label, example)| view! {
                <label class="flex flex-col gap-1">
                    <span class="text-sm font-medium">{label}</span>
                    <input
                        type="text"
                        class="input input-sm w-full"
                        maxlength="80"
                        placeholder=example
                        prop:value=move || extra.get()
                        on:input=move |ev| on_extra.run(event_target_value(&ev))
                    />
                </label>
            })}
            {move || hint.get().map(|h| view! { <p class="text-xs opacity-70">{h}</p> })}
        </fieldset>
    }
}

/// Tamaños de un bloque de menor a mayor ("Más chico" antes de "Igual").
fn sizes_small_to_large() -> Vec<&'static (&'static str, &'static str, f32)> {
    let mut sizes: Vec<_> = BLOCK_SIZES.iter().collect();
    sizes.sort_by(|a, b| a.2.total_cmp(&b.2));
    sizes
}

/// Contenido de la sección "Letras" del inspector (el título y la ayuda
/// los pone la sección plegable).
#[component]
pub(crate) fn BlockTypography(id: String) -> impl IntoView {
    let state = expect_context::<EditorState>();
    let sid = StoredValue::new(id);
    let style = Memo::new(move |_| {
        state
            .doc
            .with(|d| sid.with_value(|id| d.node(id).map(|n| n.style.clone()).unwrap_or_default()))
    });
    let store_body = Signal::derive(move || state.doc.with(|d| d.typography.body.clone()));
    let set = move |f: Box<dyn FnOnce(&mut TextStyle)>| {
        sid.with_value(|id| {
            state.apply(|d| {
                if let Some(n) = d.node_mut(id) {
                    f(&mut n.style);
                }
            })
        })
    };
    let changed = move || style.with(|s| !s.is_default());

    view! {
        <section class="flex flex-col gap-4">
            <FontSelect
                label="Fuente"
                value=Signal::derive(move || style.with(|s| s.font.clone()))
                sample="Así se ven las letras"
                inherit=store_body
                on_pick=Callback::new(move |v: String| set(Box::new(move |s| s.font = v)))
            />
            <fieldset>
                <legend class="mb-1 text-sm font-medium">"Tamaño"</legend>
                <div class="join w-full">
                    {sizes_small_to_large().into_iter().map(|(id, name, _)| {
                        let active = move || style.with(|s| s.size == *id);
                        view! {
                            <button
                                type="button"
                                class=move || format!("{} join-item flex-1 px-1", choice_class(active()))
                                aria-pressed=move || active().to_string()
                                on:click=move |_| set(Box::new(move |s| s.size = (*id).into()))
                            >{*name}</button>
                        }
                    }).collect_view()}
                </div>
            </fieldset>
            <div class="flex gap-4">
                <fieldset>
                    <legend class="mb-1 text-sm font-medium">"Estilo"</legend>
                    <div class="join">
                        <button
                            type="button"
                            class=move || format!("{} join-item btn-square", choice_class(style.with(|s| s.bold)))
                            aria-pressed=move || style.with(|s| s.bold).to_string()
                            aria-label="Negrita"
                            title="Negrita"
                            on:click=move |_| set(Box::new(|s| s.bold = !s.bold))
                        ><span class="text-base font-bold" aria-hidden="true">"N"</span></button>
                        <button
                            type="button"
                            class=move || format!("{} join-item btn-square", choice_class(style.with(|s| s.italic)))
                            aria-pressed=move || style.with(|s| s.italic).to_string()
                            aria-label="Cursiva"
                            title="Cursiva"
                            on:click=move |_| set(Box::new(|s| s.italic = !s.italic))
                        ><span class="font-serif text-base italic" aria-hidden="true">"C"</span></button>
                    </div>
                </fieldset>
                <fieldset>
                    <legend class="mb-1 text-sm font-medium">"Alineación"</legend>
                    <div class="join">
                        {ALIGN_OPTIONS.iter().map(|(id, name)| {
                            let active = move || style.with(|s| s.align == *id);
                            let icon = match *id {
                                "left" => "align-left",
                                "center" => "align-center",
                                "right" => "align-right",
                                _ => "align-auto",
                            };
                            view! {
                                <button
                                    type="button"
                                    class=move || format!("{} join-item btn-square", choice_class(active()))
                                    aria-pressed=move || active().to_string()
                                    aria-label=*name
                                    title=*name
                                    on:click=move |_| set(Box::new(move |s| s.align = (*id).into()))
                                ><Icon name=icon/></button>
                            }
                        }).collect_view()}
                    </div>
                </fieldset>
            </div>
            <AnimationPicker
                animation=Signal::derive(move || style.with(|s| s.animation.clone()))
                extra=Signal::derive(move || style.with(|s| s.anim_text.clone()))
                on_animation=Callback::new(move |v: String| set(Box::new(move |s| s.animation = v)))
                on_extra=Callback::new(move |v: String| set(Box::new(move |s| s.anim_text = v)))
            />
            {move || changed().then(|| view! {
                <button
                    type="button"
                    class="btn btn-sm btn-ghost self-start"
                    on:click=move |_| set(Box::new(|s| *s = TextStyle::default()))
                >
                    <Icon name="undo"/>
                    "Volver a las letras de la tienda"
                </button>
            })}
        </section>
    }
}
