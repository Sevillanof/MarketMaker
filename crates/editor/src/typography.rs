//! Letras: el panel de toda la tienda (botón "Letras" de la barra) y la
//! sección "Letras de este bloque" del inspector.
//!
//! Pocas opciones y con muestra: se elige una combinación de fuentes de un
//! clic, y quien quiera puede elegir cada fuente por separado.

use domain::{
    font, font_css, TextStyle, Typography, ALIGN_OPTIONS, BLOCK_SIZES, FONTS, FONT_PAIRS,
    STORE_SIZES,
};
use leptos::prelude::*;

use crate::icons::Icon;
use crate::{EditorState, Ui};

/// Id del panel emergente de las letras de la tienda.
const PANEL_ID: &str = "mk-type-panel";

fn font_label(id: &str) -> &'static str {
    font(id).map_or("Simple", |f| f.label)
}

/// Botón de la barra y panel emergente con las letras de toda la tienda.
///
/// Usa la API `popover` del navegador: se cierra con Esc o con un clic
/// afuera, y devuelve el foco al botón.
#[component]
pub(crate) fn StoreTypography() -> impl IntoView {
    let state = expect_context::<EditorState>();
    let typo = Memo::new(move |_| state.doc.with(|d| d.typography.clone()));
    let set = move |f: Box<dyn FnOnce(&mut Typography)>| state.apply(|d| f(&mut d.typography));

    view! {
        <button
            type="button"
            class="btn btn-sm"
            popovertarget=PANEL_ID
            aria-haspopup="dialog"
            title="Fuentes y tamaño del texto de toda la tienda"
        >
            <Icon name="type"/>
            "Letras"
        </button>
        <div
            id=PANEL_ID
            popover="auto"
            role="dialog"
            aria-labelledby="mk-type-title"
            class="mk-popover w-[26rem] rounded-box border border-base-300 bg-base-100 p-4 text-base-content shadow-xl"
        >
            <div class="flex items-start justify-between gap-3">
                <div>
                    <h2 id="mk-type-title" class="text-lg font-bold">"Letras de toda la tienda"</h2>
                    <p class="text-sm text-base-content/75">"Se aplican a todos los bloques. Podés cambiar un bloque solo desde el panel de la derecha."</p>
                </div>
                <button
                    type="button"
                    class="btn btn-ghost btn-sm btn-square"
                    popovertarget=PANEL_ID
                    popovertargetaction="hide"
                    aria-label="Cerrar"
                >
                    <Icon name="close"/>
                </button>
            </div>

            <fieldset class="mt-4">
                <legend class="mb-2 text-sm font-medium">"Estilo de las letras"</legend>
                <div class="grid grid-cols-2 gap-2">
                    {FONT_PAIRS.iter().map(|p| {
                        let active = move || typo.with(|t| t.heading == p.heading && t.body == p.body);
                        view! {
                            <button
                                type="button"
                                class=move || if active() {
                                    "flex flex-col items-start gap-0.5 rounded-field border-2 border-primary bg-primary/10 p-2 text-left"
                                } else {
                                    "flex flex-col items-start gap-0.5 rounded-field border-2 border-base-300 p-2 text-left transition-colors hover:border-primary"
                                }
                                aria-pressed=move || active().to_string()
                                on:click=move |_| set(Box::new(move |t| {
                                    t.heading = p.heading.into();
                                    t.body = p.body.into();
                                }))
                            >
                                <span class="text-lg font-bold leading-tight" style=format!("font-family:{}", font_css(p.heading))>"Ofertas"</span>
                                <span class="text-sm" style=format!("font-family:{}", font_css(p.body))>"Envíos a todo el país"</span>
                                <span class="mt-1 flex items-center gap-1 text-xs font-medium text-base-content/75">
                                    {move || active().then(|| view! { <Icon name="check" class="h-3.5 w-3.5 text-primary"/> })}
                                    {p.label}
                                </span>
                            </button>
                        }
                    }).collect_view()}
                </div>
            </fieldset>

            <details class="collapse collapse-arrow mt-3 border border-base-300">
                <summary class="collapse-title text-sm font-medium">"Elegir cada fuente por separado"</summary>
                <div class="collapse-content flex flex-col gap-3">
                    <FontSelect
                        label="Títulos"
                        value=Signal::derive(move || typo.with(|t| t.heading.clone()))
                        sample="Nuestros productos"
                        on_pick=Callback::new(move |id: String| set(Box::new(move |t| t.heading = id)))
                    />
                    <FontSelect
                        label="Texto"
                        value=Signal::derive(move || typo.with(|t| t.body.clone()))
                        sample="Envíos en 24 a 72 horas a todo el país."
                        on_pick=Callback::new(move |id: String| set(Box::new(move |t| t.body = id)))
                    />
                </div>
            </details>

            <fieldset class="mt-4">
                <legend class="mb-2 text-sm font-medium">"Tamaño del texto"</legend>
                <div class="join w-full">
                    {STORE_SIZES.iter().map(|(id, name, _)| {
                        let active = move || typo.with(|t| t.size == *id);
                        view! {
                            <button
                                type="button"
                                class="btn btn-sm join-item flex-1"
                                class:btn-primary=active
                                aria-pressed=move || active().to_string()
                                on:click=move |_| set(Box::new(move |t| t.size = (*id).into()))
                            >{*name}</button>
                        }
                    }).collect_view()}
                </div>
                <p class="mt-1 text-xs text-base-content/75">"Grande ayuda a leer mejor, sobre todo en el celular."</p>
            </fieldset>
        </div>
    }
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

/// Sección "Letras de este bloque" del inspector.
#[component]
pub(crate) fn BlockTypography(id: String) -> impl IntoView {
    let state = expect_context::<EditorState>();
    let ui = expect_context::<Ui>();
    let sid = StoredValue::new(id);
    let style = Memo::new(move |_| {
        state.doc.with(|d| {
            sid.with_value(|id| {
                d.nodes
                    .iter()
                    .find(|n| &n.id == id)
                    .map(|n| n.style.clone())
                    .unwrap_or_default()
            })
        })
    });
    let store_body = Signal::derive(move || state.doc.with(|d| d.typography.body.clone()));
    let set = move |f: Box<dyn FnOnce(&mut TextStyle)>| {
        sid.with_value(|id| {
            state.apply(|d| {
                if let Some(n) = d.nodes.iter_mut().find(|n| &n.id == id) {
                    f(&mut n.style);
                }
            })
        })
    };
    let changed = move || style.with(|s| !s.is_default());

    view! {
        <details
            class="collapse collapse-arrow border border-base-300"
            prop:open=ui.block_type_open.get_untracked()
            on:toggle=move |ev| {
                use wasm_bindgen::JsCast;
                if let Some(el) = ev.target().and_then(|t| t.dyn_into::<web_sys::HtmlDetailsElement>().ok()) {
                    ui.block_type_open.set(el.open());
                }
            }
        >
            <summary class="collapse-title flex items-center gap-2 text-sm font-medium">
                <Icon name="type"/>
                "Letras de este bloque"
                {move || changed().then(|| view! { <span class="badge badge-sm badge-primary">"Cambiadas"</span> })}
            </summary>
            <div class="collapse-content flex flex-col gap-4">
                <FontSelect
                    label="Fuente"
                    value=Signal::derive(move || style.with(|s| s.font.clone()))
                    sample="Así se ven las letras"
                    inherit=store_body
                    on_pick=Callback::new(move |v: String| set(Box::new(move |s| s.font = v)))
                />
                <fieldset>
                    <legend class="mb-1 text-sm font-medium">"Tamaño"</legend>
                    <div class="grid grid-cols-2 gap-1">
                        {BLOCK_SIZES.iter().map(|(id, name, _)| {
                            let active = move || style.with(|s| s.size == *id);
                            view! {
                                <button
                                    type="button"
                                    class="btn btn-sm"
                                    class:btn-primary=active
                                    aria-pressed=move || active().to_string()
                                    on:click=move |_| set(Box::new(move |s| s.size = (*id).into()))
                                >{*name}</button>
                            }
                        }).collect_view()}
                    </div>
                </fieldset>
                <fieldset>
                    <legend class="mb-1 text-sm font-medium">"Estilo"</legend>
                    <div class="grid grid-cols-2 gap-1">
                        <button
                            type="button"
                            class="btn btn-sm"
                            class:btn-primary=move || style.with(|s| s.bold)
                            aria-pressed=move || style.with(|s| s.bold).to_string()
                            on:click=move |_| set(Box::new(|s| s.bold = !s.bold))
                        ><span class="font-bold" aria-hidden="true">"N"</span>"Negrita"</button>
                        <button
                            type="button"
                            class="btn btn-sm"
                            class:btn-primary=move || style.with(|s| s.italic)
                            aria-pressed=move || style.with(|s| s.italic).to_string()
                            on:click=move |_| set(Box::new(|s| s.italic = !s.italic))
                        ><span class="font-serif italic" aria-hidden="true">"C"</span>"Cursiva"</button>
                    </div>
                </fieldset>
                <fieldset>
                    <legend class="mb-1 text-sm font-medium">"Alineación"</legend>
                    <div class="grid grid-cols-2 gap-1">
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
                                    class="btn btn-sm"
                                    class:btn-primary=active
                                    aria-pressed=move || active().to_string()
                                    on:click=move |_| set(Box::new(move |s| s.align = (*id).into()))
                                ><Icon name=icon/>{*name}</button>
                            }
                        }).collect_view()}
                    </div>
                </fieldset>
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
            </div>
        </details>
    }
}
