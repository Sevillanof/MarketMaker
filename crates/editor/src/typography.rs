//! Letras: la sección "Letras de este bloque" del inspector (las de toda la
//! tienda están en el cajón "Mi tienda", ver `store_drawer`).
//!
//! Pocas opciones y con muestra: se elige una combinación de fuentes de un
//! clic, y quien quiera puede elegir cada fuente por separado.

use domain::{font, font_css, TextStyle, ALIGN_OPTIONS, BLOCK_SIZES, FONTS};
use leptos::prelude::*;

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

/// Tamaños de un bloque de menor a mayor ("Más chico" antes de "Igual").
fn sizes_small_to_large() -> Vec<&'static (&'static str, &'static str, f32)> {
    let mut sizes: Vec<_> = BLOCK_SIZES.iter().collect();
    sizes.sort_by(|a, b| a.2.total_cmp(&b.2));
    sizes
}

/// Sección "Letras de este bloque" del inspector.
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
        <section class="flex flex-col gap-4" aria-labelledby="mk-block-type-title">
            <div>
                <h3 id="mk-block-type-title" class="text-sm font-semibold">"Letras de este bloque"</h3>
                <p class="text-xs text-base-content/75">"Lo que no cambies sigue a las letras de toda la tienda."</p>
            </div>
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
