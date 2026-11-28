//! Cajón "Mi tienda": los ajustes de toda la tienda en la columna de la
//! izquierda (en lugar de la lista de bloques).
//!
//! Todo lo que se ve se puede tocar: cada opción es un dibujito de cómo
//! queda, y el cambio se ve al instante en el lienzo. Sin textos de ayuda ni
//! resúmenes; las opciones nuevas se suman como una sección más.

use domain::{
    font_css, Side, StoreLayout, FONT_PAIRS, GAP_STEP, GRID_COLS, MAX_GAP, MAX_PAD, PAD_STEP,
    ROLE_MAX, ROLE_MIN, ROW_PX, SCALE_MAX, SCALE_MIN, SCALE_STEP, TEXT_ROLES,
};
use leptos::prelude::*;

use crate::icons::Icon;
use crate::theme_panel::ThemePanel;
use crate::{EditorState, Ui};

/// Id del botón "Mi tienda" de la barra: vuelve el foco ahí al cerrar.
pub(crate) const STORE_BUTTON_ID: &str = "mk-rail-style";

/// Título de una sección.
#[component]
pub(crate) fn Section(
    title: &'static str,
    id: &'static str,
    /// Ayuda en un ⓘ al lado del título, si el título no se entiende solo.
    #[prop(optional)]
    help: &'static str,
    children: Children,
) -> impl IntoView {
    view! {
        <details class="mk-sec" open>
            <summary class="mk-sec-head">
                <Icon name="chevron-down" class="mk-sec-chev h-4 w-4"/>
                <h3 id=id class="relative flex items-center gap-1.5 text-[11px] font-semibold uppercase tracking-wide text-base-content/70">
                    {title}
                    {(!help.is_empty()).then(|| view! { <ui::HelpTip id=format!("tip-{id}") label=title text=help/> })}
                </h3>
            </summary>
            <div class="pb-3 pt-2.5">{children()}</div>
        </details>
    }
}

/// Cierra "Estilo" (vuelve a Bloques) y devuelve el foco a su botón del riel.
pub(crate) fn close_drawer(ui: Ui) {
    ui.tool.set(crate::Tool::Blocks);
    if let Some(b) = document().get_element_by_id(STORE_BUTTON_ID) {
        use wasm_bindgen::JsCast;
        if let Ok(b) = b.dyn_into::<web_sys::HtmlElement>() {
            _ = b.focus();
        }
    }
}

#[component]
pub(crate) fn StoreDrawer() -> impl IntoView {
    let ui = expect_context::<Ui>();
    let state = expect_context::<EditorState>();
    view! {
        <aside
            class="flex min-h-0 w-full flex-1 flex-col bg-base-100"
            aria-labelledby="mk-drawer-title"
            on:keydown=move |ev| {
                if ev.key() == "Escape" {
                    // Con un flotante abierto, Esc cierra solo el flotante
                    // (lo hace el navegador), no todo el panel.
                    if ev
                        .target()
                        .and_then(|t| wasm_bindgen::JsCast::dyn_into::<web_sys::Element>(t).ok())
                        .is_some_and(|el| el.closest("[popover]").ok().flatten().is_some())
                    {
                        return;
                    }
                    ev.prevent_default();
                    close_drawer(ui);
                }
            }
        >
            <div class="flex h-12 shrink-0 items-center gap-2 border-b border-base-300 px-4">
                <h2 id="mk-drawer-title" tabindex="-1" class="font-bold focus:outline-none">"Estilo de la tienda"</h2>
                <button
                    type="button"
                    class="btn btn-primary btn-xs ml-auto"
                    title="Guardar el borrador (no publica)"
                    on:click=move |_| ui.save_clicks.update(|n| *n += 1)
                >"Guardar"</button>
                <button
                    type="button"
                    class="btn btn-ghost btn-sm btn-square"
                    aria-label="Cerrar Estilo"
                    title="Cerrar (Esc)"
                    on:click=move |_| close_drawer(ui)
                >
                    <Icon name="close"/>
                </button>
            </div>
            <div class="flex-1 overflow-y-auto px-3 pb-4">
                <button
                    type="button"
                    class="btn btn-outline mt-3 h-auto w-full justify-start gap-3 py-2.5 text-left font-normal"
                    on:click=move |_| ui.wizard.set(true)
                >
                    <Icon name="page" class="h-5 w-5 shrink-0"/>
                    <span class="flex flex-col leading-tight">
                        <span class="font-bold">"Asistente de página"</span>
                        <span class="text-xs text-base-content/70">"Armá o renová tu página paso a paso"</span>
                    </span>
                </button>
                <ThemePanel/>
            </div>
            <div class="flex shrink-0 items-center gap-2 border-t border-base-300 px-3 py-2.5">
                <button
                    type="button"
                    class="text-[11px] font-bold text-base-content/70 hover:text-base-content"
                    on:click=move |_| state.undo()
                >"↶ Deshacer"</button>
                <span class="ml-auto text-[10px] text-base-content/65" role="status" aria-live="polite">{move || ui.save_label.get()}</span>
                <button
                    type="button"
                    class="btn btn-primary btn-xs"
                    title="Guardar el borrador (no publica)"
                    on:click=move |_| ui.save_clicks.update(|n| *n += 1)
                >"Guardar"</button>
            </div>
        </aside>
    }
}

/// Letras de la tienda: una pareja de tipografías a la vez, con flechas e
/// indicadores para recorrerlas; el tamaño general en porcentaje y, en un
/// flotante, el tamaño base de cada tipo de texto.
#[component]
pub(crate) fn Letters() -> impl IntoView {
    let state = expect_context::<EditorState>();
    let typo = Memo::new(move |_| state.doc.with(|d| d.typography.clone()));
    // Posición de la pareja activa; `None` si el documento usa una
    // combinación que no está en la lista ("Personalizada").
    let current = Memo::new(move |_| {
        typo.with(|t| {
            FONT_PAIRS
                .iter()
                .position(|p| p.heading == t.heading && p.body == t.body)
        })
    });
    let n = FONT_PAIRS.len();
    let pick = move |i: usize| {
        if let Some(p) = FONT_PAIRS.get(i) {
            state.apply(|d| {
                d.typography.heading = p.heading.into();
                d.typography.body = p.body.into();
            });
        }
    };
    let step = move |delta: isize| {
        let i = match current.get_untracked() {
            Some(i) => (i as isize + delta).rem_euclid(n as isize) as usize,
            // Primer clic explícito: elige una pareja conocida.
            None if delta > 0 => 0,
            None => n - 1,
        };
        pick(i);
    };
    let heading_css = move || typo.with(|t| font_css(&t.heading));
    let body_css = move || typo.with(|t| font_css(&t.body));
    let name = move || {
        current
            .get()
            .map_or("Personalizada", |i| FONT_PAIRS[i].label)
    };
    let families = move || {
        typo.with(|t| {
            let label = |id: &str| domain::font(id).map_or(id.to_string(), |f| f.label.to_string());
            if t.heading == t.body {
                label(&t.heading)
            } else {
                format!("{} + {}", label(&t.heading), label(&t.body))
            }
        })
    };
    let percent = move || typo.with(|t| t.scale_percent());
    let percent_text = move || format!("{} %", percent().to_string().replace('.', ","));
    view! {
        <Section title="Letras" id="mk-dr-letters" help="Se aplica a toda la tienda.">
            <div role="group" aria-label="Pareja tipográfica">
                <div class="mk-fontpick">
                    <button type="button" class="mk-fontarrow" aria-label="Tipografía anterior" on:click=move |_| step(-1)>
                        <Icon name="chevron-left"/>
                    </button>
                    <div class="min-w-0 flex-1 py-1" aria-live="polite" aria-atomic="true">
                        <span class="flex flex-wrap items-baseline gap-x-1.5 text-[11px] text-base-content/60">
                            <span class="font-bold text-base-content/80">{name}</span>
                            <span class="min-w-0 break-words">{families}</span>
                            <span class="ml-auto shrink-0">
                                {move || current.get().map(|i| format!("{} de {n}", i + 1)).unwrap_or_default()}
                            </span>
                        </span>
                        <span class="mt-1 block text-[1.35rem] leading-tight text-base-content" style=move || format!("font-family:{}", heading_css())>"Pequeñas cosas."</span>
                        <span class="mt-0.5 block text-xs text-base-content/60" style=move || format!("font-family:{}", body_css())>"Una tienda con tu estilo"</span>
                    </div>
                    <button type="button" class="mk-fontarrow" aria-label="Tipografía siguiente" on:click=move |_| step(1)>
                        <Icon name="chevron-right"/>
                    </button>
                </div>
                <div class="mk-markers" role="group" aria-label="Elegir tipografía">
                    {FONT_PAIRS.iter().enumerate().map(|(i, p)| view! {
                        <button
                            type="button"
                            aria-label=p.label
                            title=p.label
                            aria-pressed=move || (current.get() == Some(i)).to_string()
                            on:click=move |_| pick(i)
                        ></button>
                    }).collect_view()}
                </div>
            </div>
            <div class="mt-3">
                <label class="flex items-baseline justify-between text-xs font-semibold" for="mk-dr-tscale">
                    "Tamaño general del texto"
                    <output class="font-normal text-base-content/70" for="mk-dr-tscale">{percent_text}</output>
                </label>
                <input
                    id="mk-dr-tscale"
                    type="range"
                    class="range range-primary range-xs mt-1 w-full"
                    min=SCALE_MIN
                    max=SCALE_MAX
                    step=SCALE_STEP
                    prop:value=move || percent().round().to_string()
                    aria-valuetext=move || format!("{} por ciento", percent_text().trim_end_matches(" %"))
                    on:input=move |ev| {
                        if let Ok(v) = event_target_value(&ev).trim().parse::<u16>() {
                            state.apply_grouped(Some("tscale"), |d| d.typography.scale = v);
                        }
                    }
                />
                <div class="flex justify-between text-[10.5px] text-base-content/55" aria-hidden="true">
                    <span>"Más chico"</span>
                    <span>"Más grande"</span>
                </div>
            </div>
            <button
                type="button"
                class="mt-2.5 flex w-full items-center justify-between rounded-field border border-base-300 bg-base-200/60 px-2.5 py-2 text-left text-xs font-bold text-primary"
                popovertarget=TYPE_POP_ID
            >
                "Tamaños avanzados"
                <Icon name="chevron-down"/>
            </button>
            <TypeSizes/>
        </Section>
    }
}

const TYPE_POP_ID: &str = "mk-typepop";

/// Flotante con el tamaño base (en px, a 100 %) de cada tipo de texto. Lo
/// que se deja vacío queda como siempre.
#[component]
fn TypeSizes() -> impl IntoView {
    let state = expect_context::<EditorState>();
    let typo = Memo::new(move |_| state.doc.with(|d| d.typography.clone()));
    view! {
        <div
            id=TYPE_POP_ID
            popover="auto"
            class="mk-colorpop mk-typepop"
            role="group"
            aria-labelledby="mk-typepop-title"
            on:keydown=move |ev| {
                // Esc cierra solo el flotante, no todo el cajón.
                if ev.key() == "Escape" {
                    ev.stop_propagation();
                }
            }
        >
            <div class="mb-2 flex items-start gap-2">
                <div>
                    <h3 id="mk-typepop-title" class="text-sm font-bold">"Tamaños avanzados"</h3>
                    <p class="text-[11px] text-base-content/60">"Personalizá cada tipo de texto."</p>
                </div>
                <button
                    type="button"
                    class="btn btn-ghost btn-xs btn-square ml-auto"
                    aria-label="Cerrar tamaños avanzados"
                    popovertarget=TYPE_POP_ID
                    popovertargetaction="hide"
                >
                    <Icon name="close"/>
                </button>
            </div>
            <p class="mb-2 text-[10.5px] leading-snug text-base-content/60">"Valores base en px al 100 %. El tamaño general se aplica sobre cada uno."</p>
            <div class="grid gap-1.5">
                {TEXT_ROLES.iter().map(|(id, label, reference)| {
                    let input_id = format!("mk-type-{id}");
                    view! {
                        <div class="flex items-center gap-2 text-xs">
                            <label class="flex-1" for=input_id.clone()>{*label}</label>
                            <input
                                id=input_id
                                type="number"
                                class="input input-xs w-16 text-right"
                                min=ROLE_MIN
                                max=ROLE_MAX
                                step="1"
                                placeholder=reference.to_string()
                                prop:value=move || typo.with(|t| t.sizes.get(*id).map(|v| v.to_string()).unwrap_or_default())
                                on:change=move |ev| {
                                    let raw = event_target_value(&ev);
                                    state.apply_grouped(Some(&format!("tsize-{id}")), |d| {
                                        match raw.trim().parse::<u16>() {
                                            Ok(v) => {
                                                d.typography.sizes.insert((*id).to_string(), v.clamp(ROLE_MIN, ROLE_MAX));
                                            }
                                            Err(_) => {
                                                d.typography.sizes.remove(*id);
                                            }
                                        }
                                    });
                                }
                            />
                            <span class="text-[10px] text-base-content/55">"px"</span>
                        </div>
                    }
                }).collect_view()}
            </div>
            <div class="mt-3 flex items-center justify-between">
                <button
                    type="button"
                    class="text-[11px] font-semibold text-primary hover:underline"
                    prop:disabled=move || typo.with(|t| t.sizes.is_empty())
                    on:click=move |_| state.apply(|d| d.typography.sizes.clear())
                >"Volver a los de siempre"</button>
                <button type="button" class="btn btn-primary btn-xs" popovertarget=TYPE_POP_ID popovertargetaction="hide">"Listo"</button>
            </div>
        </div>
    }
}

/// Márgenes de la tienda: una cruz con un campo por lado alrededor de una
/// mini pantalla. La mini pantalla tiene la forma real de la ventana de quien
/// edita y el recuadro azul muestra el contenido a esa escala.
#[component]
pub(crate) fn Margins() -> impl IntoView {
    let state = expect_context::<EditorState>();
    let ui = expect_context::<Ui>();
    let layout = Memo::new(move |_| state.doc.with(|d| d.layout.clone()));
    // Tamaño real de la ventana (ancho y alto), para dibujar a escala.
    let screen = RwSignal::new((1440.0_f64, 900.0_f64));
    let measure = move || {
        let w = document()
            .document_element()
            .map_or(0.0, |r| f64::from(r.client_width()));
        let h = window()
            .inner_height()
            .ok()
            .and_then(|h| h.as_f64())
            .unwrap_or(0.0);
        if w > 0.0 && h > 0.0 {
            screen.set((w, h));
        }
    };
    Effect::new(move |_| measure());
    let _ = window_event_listener(leptos::ev::resize, move |_| measure());
    let field = move |s: Side| {
        view! {
                   <div class="mk-step">
                       <button
                           type="button"
                           aria-label=format!("Menos margen {}", s.label().to_lowercase())
                           on:click=move |_| {
                               let px = i32::from(layout.with_untracked(|l| l.pad(s))) - i32::from(PAD_STEP) / 2;
                               {
        state.apply_grouped(Some("pad"), |d| d.layout.set_pad(s, px));
        ui.flash_changed("margins");
        }
                           }
                       >"\u{2212}"</button>
                       <input
                           type="number"
                           min="0"
                           max=MAX_PAD
                           step=PAD_STEP / 2
                           aria-label=format!("Margen {} en píxeles", s.label().to_lowercase())
                           prop:value=move || layout.with(|l| l.pad(s)).to_string()
                           on:change=move |ev| {
                               if let Ok(px) = event_target_value(&ev).trim().parse::<i32>() {
                                   {
        state.apply_grouped(Some("pad"), |d| d.layout.set_pad(s, px));
        ui.flash_changed("margins");
        }
                               }
                           }
                       />
                       <button
                           type="button"
                           aria-label=format!("Más margen {}", s.label().to_lowercase())
                           on:click=move |_| {
                               let px = i32::from(layout.with_untracked(|l| l.pad(s))) + i32::from(PAD_STEP) / 2;
                               {
        state.apply_grouped(Some("pad"), |d| d.layout.set_pad(s, px));
        ui.flash_changed("margins");
        }
                           }
                       >"+"</button>
                   </div>
               }
    };
    view! {
           <div class="mt-2.5">
               <p class="mb-1 flex items-baseline gap-1.5">
                   <span class="text-xs font-semibold" id="mk-dr-margins">"Márgenes"</span>
                   <span class="text-[10.5px] text-base-content/55">"borde de la pantalla"</span>
                   <button
                       type="button"
                       class="ml-auto text-[11px] font-semibold text-primary hover:underline"
                       prop:disabled=move || layout.with(|l| *l == StoreLayout::fresh())
                       on:click=move |_| {
    state.apply(|d| d.layout = StoreLayout::fresh());
    ui.flash_changed("margins");
    }
                   >"Valores iniciales"</button>
               </p>
               <div class="mk-cross" role="group" aria-labelledby="mk-dr-margins">
                   <div class="mk-cross-t">{field(Side::Top)}</div>
                   <div class="mk-cross-l">{field(Side::Left)}</div>
                   <div
                       class="mk-cross-pg"
                       aria-hidden="true"
                       style=move || {
                           let (w, h) = screen.get();
                           let l = layout.get();
                           let pct = |px: u16, of: f64| (f64::from(px) / of * 100.0).min(45.0);
                           format!(
                               "aspect-ratio:{w:.0}/{h:.0}; --t:{:.2}%; --r:{:.2}%; --b:{:.2}%; --l:{:.2}%",
                               pct(l.pad_top, h),
                               pct(l.pad_right, w),
                               pct(l.pad_bottom, h),
                               pct(l.pad_left, w),
                           )
                       }
                   >
                       <div class="mk-cross-in">
                           {move || {
                               // Ancho real del contenido, para que alto y ancho
                               // de cada bloque guarden la proporción verdadera.
                               let (w, _) = screen.get();
                               let l = layout.get();
                               let avail = (w - f64::from(l.pad_left) - f64::from(l.pad_right)).max(1.0);
                               let content = avail;
                               let cols = f64::from(GRID_COLS);
                               state
                                   .nodes()
                                   .into_iter()
                                   .map(|n| {
                                       let px = |rows: f64| rows * f64::from(ROW_PX) / content * 100.0;
                                       view! {
                                           <i
                                               class="mk-cross-bk"
                                               style=format!(
                                                   "left:{:.2}%; width:{:.2}%; top:{:.2}cqw; height:{:.2}cqw",
                                                   f64::from(n.pos.x) / cols * 100.0,
                                                   f64::from(n.size.w) / cols * 100.0,
                                                   px(f64::from(n.pos.y)),
                                                   px(f64::from(n.size.h)),
                                               )
                                           ></i>
                                       }
                                   })
                                   .collect_view()
                           }}
                       </div>
                   </div>
                   <div class="mk-cross-r">{field(Side::Right)}</div>
                   <div class="mk-cross-b">{field(Side::Bottom)}</div>
               </div>
               <p class="mt-1 text-center text-[10.5px] text-base-content/55">
                   {move || {
                       let (w, h) = screen.get();
                       format!("Tu pantalla: {w:.0} \u{d7} {h:.0} px")
                   }}
               </p>
               <div class="mt-3">
                   <label class="flex items-baseline justify-between text-xs font-semibold" for="mk-dr-gap">
                       "Espacio entre widgets"
                       <output class="font-normal text-base-content/70" for="mk-dr-gap">
                           {move || format!("{} px", layout.with(|l| l.gap))}
                       </output>
                   </label>
                   <input
                       id="mk-dr-gap"
                       type="range"
                       class="range range-primary range-xs mt-1 w-full"
                       min="0"
                       max=MAX_GAP
                       step=GAP_STEP
                       prop:value=move || layout.with(|l| l.gap).to_string()
                       aria-valuetext=move || format!("{} píxeles entre widgets", layout.with(|l| l.gap))
                       on:input=move |ev| {
                           if let Ok(px) = event_target_value(&ev).trim().parse::<i32>() {
                               state.apply_grouped(Some("gap"), |d| d.layout.set_gap(px));
                               ui.flash_changed("margins");
                           }
                       }
                   />
                   <div class="flex justify-between text-[10.5px] text-base-content/55" aria-hidden="true">
                       <span>"Sin espacio"</span>
                       <span>"Más aire"</span>
                   </div>
               </div>
           </div>
       }
}
