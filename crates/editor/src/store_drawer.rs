//! Cajón "Mi tienda": los ajustes de toda la tienda en la columna de la
//! izquierda (en lugar de la lista de bloques).
//!
//! Todo lo que se ve se puede tocar: cada opción es un dibujito de cómo
//! queda, y el cambio se ve al instante en el lienzo. Sin textos de ayuda ni
//! resúmenes; las opciones nuevas se suman como una sección más.

use domain::{
    font_css, Side, StoreLayout, FONT_PAIRS, GRID_COLS, MAX_PAD, PAD_STEP, ROW_PX, STORE_SIZES,
};
use leptos::prelude::*;

use crate::icons::Icon;
use crate::theme_panel::ThemePanel;
use crate::{EditorState, Ui};

/// Id del botón "Mi tienda" de la barra: vuelve el foco ahí al cerrar.
pub(crate) const STORE_BUTTON_ID: &str = "mk-store-button";

/// Clase de una opción con dibujito.
pub(crate) fn tile(active: bool) -> &'static str {
    if active {
        "flex flex-col items-center gap-1 rounded-field border-2 border-primary bg-primary/10 p-1.5 text-primary"
    } else {
        "flex flex-col items-center gap-1 rounded-field border-2 border-base-300 p-1.5 text-base-content/80 transition-colors hover:border-primary"
    }
}

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
        <section class="mt-3" aria-labelledby=id>
            <h3 id=id class="relative mb-1.5 flex items-center gap-1.5 text-[11px] font-semibold uppercase tracking-wide text-base-content/70">
                {title}
                {(!help.is_empty()).then(|| view! { <ui::HelpTip id=format!("tip-{id}") label=title text=help/> })}
            </h3>
            {children()}
        </section>
    }
}

/// Cierra el cajón y vuelve el foco al botón "Mi tienda".
pub(crate) fn close_drawer(ui: Ui) {
    ui.store_open.set(false);
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
    view! {
        <aside
            class="flex w-80 shrink-0 flex-col border-r border-base-300 bg-base-100"
            aria-labelledby="mk-drawer-title"
            on:keydown=move |ev| {
                if ev.key() == "Escape" {
                    ev.prevent_default();
                    close_drawer(ui);
                }
            }
        >
            <div class="flex h-12 shrink-0 items-center gap-2 border-b border-base-300 px-4">
                <h2 id="mk-drawer-title" tabindex="-1" class="font-bold focus:outline-none">"Mi tienda"</h2>
                <button
                    type="button"
                    class="btn btn-ghost btn-sm btn-square ml-auto"
                    aria-label="Cerrar Mi tienda"
                    title="Cerrar (Esc)"
                    on:click=move |_| close_drawer(ui)
                >
                    <Icon name="close"/>
                </button>
            </div>
            <div class="flex-1 overflow-y-auto px-3 pb-4">
                <ThemePanel/>
                <div class="mt-5 border-t border-base-300"></div>
            </div>
        </aside>
    }
}

/// Letras de la tienda: un desplegable con las tipografías (cada opción se
/// ve en su letra) y el tamaño del texto.
#[component]
pub(crate) fn Letters() -> impl IntoView {
    let state = expect_context::<EditorState>();
    let typo = Memo::new(move |_| state.doc.with(|d| d.typography.clone()));
    let current = move || {
        typo.with(|t| {
            FONT_PAIRS
                .iter()
                .position(|p| p.heading == t.heading && p.body == t.body)
        })
    };
    view! {
        <Section title="Letras" id="mk-dr-letters">
            <select
                class="mk-fontselect"
                aria-labelledby="mk-dr-letters"
                style=move || current().map(|i| format!("font-family:{}", font_css(FONT_PAIRS[i].heading))).unwrap_or_default()
                on:change=move |ev| {
                    if let Ok(i) = event_target_value(&ev).parse::<usize>() {
                        if let Some(p) = FONT_PAIRS.get(i) {
                            state.apply(|d| {
                                d.typography.heading = p.heading.into();
                                d.typography.body = p.body.into();
                            });
                        }
                    }
                }
            >
                {FONT_PAIRS.iter().enumerate().map(|(i, p)| view! {
                    <option
                        value=i.to_string()
                        selected=move || current() == Some(i)
                        style=format!("font-family:{}", font_css(p.heading))
                    >{p.label}</option>
                }).collect_view()}
            </select>
            <div class="mk-seg mt-2" role="group" aria-label="Tamaño del texto">
                {STORE_SIZES.iter().map(|(id, name, _)| {
                    let active = move || typo.with(|t| t.size == *id);
                    view! {
                        <button
                            type="button"
                            class:is-active=active
                            aria-pressed=move || active().to_string()
                            on:click=move |_| state.apply(|d| d.typography.size = (*id).into())
                        >{*name}</button>
                    }
                }).collect_view()}
            </div>
        </Section>
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
                               let content = if l.is_full() { avail } else { f64::from(l.max_width).min(avail) };
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
           </div>
       }
}
