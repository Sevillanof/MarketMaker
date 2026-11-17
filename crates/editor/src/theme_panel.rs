//! Tema de la tienda dentro del cajón "Mi tienda": plantilla, colores,
//! esquinas, efectos, tamaños y bordes.
//!
//! Cada cambio se guarda en `PageDoc.style` (ver `domain::StoreStyle`) y se
//! ve al instante en el lienzo. Lo que no se toca queda como lo trae la
//! plantilla (un tema de daisyUI).

use domain::{
    theme_base, Corner, BORDER_STEPS, COLOR_KEYS, RADIUS_STEPS, SIZE_RANGE, THEMES, THEME_LABELS,
};
use leptos::prelude::*;

use crate::store_drawer::{tile, Letters, Margins, Section};
use crate::{EditorState, Ui};

/// Id del selector de color (un solo `popover` para todas las muestras).
const POP_ID: &str = "mk-colorpop";

/// Muestras del selector: cada fila es un tono, de más claro a más oscuro.
const SHADES: [[&str; 5]; 10] = [
    ["#fee2e2", "#fecaca", "#f87171", "#dc2626", "#7f1d1d"],
    ["#ffedd5", "#fed7aa", "#fb923c", "#ea580c", "#7c2d12"],
    ["#fef9c3", "#fde68a", "#facc15", "#ca8a04", "#713f12"],
    ["#dcfce7", "#bbf7d0", "#4ade80", "#16a34a", "#14532d"],
    ["#ccfbf1", "#99f6e4", "#2dd4bf", "#0d9488", "#134e4a"],
    ["#dbeafe", "#bfdbfe", "#60a5fa", "#2563eb", "#1e3a8a"],
    ["#e0e7ff", "#c7d2fe", "#818cf8", "#4f46e5", "#312e81"],
    ["#f3e8ff", "#e9d5ff", "#c084fc", "#9333ea", "#581c87"],
    ["#fce7f3", "#fbcfe8", "#f472b6", "#db2777", "#831843"],
    ["#f5f5f4", "#e7e5e4", "#a8a29e", "#57534e", "#1c1917"],
];

/// Cuál muestra de color se está cambiando.
#[derive(Clone, Copy)]
struct Picking(RwSignal<&'static str>);

fn color_label(key: &str) -> &'static str {
    COLOR_KEYS
        .iter()
        .find(|(k, _)| *k == key)
        .map_or("Color", |(_, l)| l)
}

/// Pasa HSL (`h` 0..360, `s` y `l` 0..1) a `#rrggbb`.
fn hsl_hex(h: f64, s: f64, l: f64) -> String {
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
    let m = l - c / 2.0;
    let (r, g, b) = match (h / 60.0) as u32 % 6 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    let ch = |v: f64| ((v + m) * 255.0).round() as u8;
    format!("#{:02x}{:02x}{:02x}", ch(r), ch(g), ch(b))
}

#[component]
pub(crate) fn ThemePanel() -> impl IntoView {
    provide_context(Picking(RwSignal::new("primary")));
    view! {
        <Name/>
        <Presets/>
        <Letters/>
        <Colors/>
        <Corners/>
        <Effects/>
        <Sizes/>
        <Borders/>
        <Options/>
        <ColorPopover/>
    }
}

#[component]
fn Name() -> impl IntoView {
    let state = expect_context::<EditorState>();
    let css_open = RwSignal::new(false);
    view! {
        <div class="mt-3 flex items-center gap-2">
            <label for="mk-theme-name" class="text-xs font-semibold text-base-content/70">"Nombre"</label>
            <input
                id="mk-theme-name"
                type="text"
                maxlength="40"
                placeholder="Mi tema"
                class="min-w-0 flex-1 border-b border-transparent bg-transparent py-1 text-sm font-semibold hover:border-base-content/30 focus:border-primary focus:outline-none"
                prop:value=move || state.doc.with(|d| d.style.name.clone())
                on:input=move |ev| {
                    let v = event_target_value(&ev);
                    state.apply_grouped(Some("theme-name"), |d| d.style.name = v);
                }
            />
        </div>
        <div class="mt-2 grid grid-cols-2 gap-2">
            <button
                type="button"
                class="btn btn-sm border-0 bg-base-200 font-semibold"
                on:click=move |_| {
                    let r = || js_sys::Math::random();
                    let hue = r() * 360.0;
                    let a = (hue + 150.0 + r() * 60.0) % 360.0;
                    let b = (hue + 30.0 + r() * 60.0) % 360.0;
                    state.apply(|d| {
                        d.style.set_color("primary", Some(&hsl_hex(hue, 0.65, 0.5)));
                        d.style.set_color("secondary", Some(&hsl_hex(a, 0.7, 0.58)));
                        d.style.set_color("accent", Some(&hsl_hex(b, 0.65, 0.55)));
                    });
                }
            >"Al azar"</button>
            <button
                type="button"
                class="btn btn-sm btn-neutral font-semibold"
                aria-expanded=move || css_open.get().to_string()
                aria-controls="mk-theme-css"
                on:click=move |_| css_open.update(|o| *o = !*o)
            >"{ } CSS"</button>
        </div>
        <pre
            id="mk-theme-css"
            tabindex="0"
            hidden=move || !css_open.get()
            class="mt-2 max-h-56 overflow-auto rounded-field bg-neutral p-2 text-[11px] leading-relaxed text-neutral-content"
        >{move || state.doc.with(|d| d.style.theme_css(&d.theme))}</pre>
    }
}

/// Plantillas: un tema de daisyUI cada una. Al elegir una se quitan los
/// cambios propios.
#[component]
fn Presets() -> impl IntoView {
    let state = expect_context::<EditorState>();
    view! {
        <Section title="Plantilla" id="mk-th-presets">
            <div class="grid grid-cols-2 gap-1" role="group" aria-labelledby="mk-th-presets">
                {THEMES.iter().zip(THEME_LABELS).map(|(t, l)| {
                    let active = move || state.doc.with(|d| d.theme == *t);
                    view! {
                        <button
                            type="button"
                            class=move || format!("{} !flex-row !justify-start !gap-1.5", tile(active()))
                            aria-pressed=move || active().to_string()
                            on:click=move |_| state.apply(|d| {
                                d.theme = (*t).to_string();
                                d.style.reset_theme();
                            })
                        >
                            <span data-theme=*t class="flex h-5 w-9 shrink-0 items-center justify-center gap-0.5 rounded-[3px] border border-base-content/10 bg-base-100" aria-hidden="true">
                                <span class="h-2 w-2 rounded-full bg-primary"></span>
                                <span class="h-2 w-2 rounded-full bg-secondary"></span>
                                <span class="h-2 w-2 rounded-full bg-accent"></span>
                            </span>
                            <span class="min-w-0 truncate text-[11px] font-medium leading-tight">{*l}</span>
                        </button>
                    }
                }).collect_view()}
            </div>
        </Section>
    }
}

/// Muestra de un color. `bg` y `fg` son las claves de los colores con los que
/// se pinta; abre el selector.
#[component]
fn Swatch(
    pick: &'static str,
    bg: &'static str,
    fg: &'static str,
    text: &'static str,
    label: String,
) -> impl IntoView {
    let state = expect_context::<EditorState>();
    let picking = expect_context::<Picking>().0;
    let style = move || {
        state.doc.with(|d| {
            format!(
                "background:{};color:{}",
                d.style.color(&d.theme, bg),
                d.style.color(&d.theme, fg)
            )
        })
    };
    view! {
        <button
            type="button"
            popovertarget=POP_ID
            class="mk-swatch"
            style=style
            aria-label=label
            on:click=move |_| picking.set(pick)
        >{text}</button>
    }
}

#[component]
fn Colors() -> impl IntoView {
    // Cada color con su texto: (clave, texto sobre él, nombre).
    const PAIRS: [(&str, &str, &str); 8] = [
        ("primary", "primary-content", "principal"),
        ("secondary", "secondary-content", "secundario"),
        ("accent", "accent-content", "acento"),
        ("neutral", "neutral-content", "neutro"),
        ("info", "info-content", "info"),
        ("success", "success-content", "éxito"),
        ("warning", "warning-content", "aviso"),
        ("error", "error-content", "error"),
    ];
    view! {
        <Section title="Colores" id="mk-th-colors">
            <div class="mk-swatch-strip">
                <Swatch pick="base-100" bg="base-100" fg="base-content" text="100" label="Fondo".to_string()/>
                <Swatch pick="base-200" bg="base-200" fg="base-content" text="200" label="Fondo 2".to_string()/>
                <Swatch pick="base-300" bg="base-300" fg="base-content" text="300" label="Fondo 3".to_string()/>
                <Swatch pick="base-content" bg="base-content" fg="base-100" text="A" label="Texto".to_string()/>
            </div>
            <p class="mt-0.5 text-[10px] text-base-content/60">"base: fondos y texto"</p>
            <div class="mt-2 grid grid-cols-4 gap-x-2 gap-y-2" role="group" aria-labelledby="mk-th-colors">
                {PAIRS.iter().map(|(key, content, name)| view! {
                    <div class="min-w-0">
                        <div class="mk-swatch-strip">
                            <Swatch pick=*key bg=*key fg=*content text="" label=format!("Color {name}")/>
                            <Swatch pick=*content bg=*key fg=*content text="A" label=format!("Texto sobre {name}")/>
                        </div>
                        <p class="mt-0.5 truncate text-[10px] text-base-content/60">{*name}</p>
                    </div>
                }).collect_view()}
            </div>
        </Section>
    }
}

/// El selector de color: muestras, "Otro color" y volver al del tema.
#[component]
fn ColorPopover() -> impl IntoView {
    let state = expect_context::<EditorState>();
    let picking = expect_context::<Picking>().0;
    let current = move || {
        let key = picking.get();
        state.doc.with(|d| d.style.color(&d.theme, key))
    };
    let is_content = move || picking.get().ends_with("-content") && picking.get() != "base-content";
    view! {
        <div
            id=POP_ID
            popover="auto"
            class="mk-colorpop"
            role="group"
            aria-label="Elegir color"
            on:keydown=move |ev| {
                // Esc cierra solo el selector, no todo el cajón.
                if ev.key() == "Escape" {
                    ev.stop_propagation();
                }
            }
        >
            <p class="mb-2 flex items-center gap-2 text-sm font-semibold">
                <span class="h-4 w-4 rounded border border-base-content/20" style=move || format!("background:{}", current())></span>
                {move || color_label(picking.get())}
            </p>
            <div class="grid grid-cols-10 gap-0.5">
                {(0..5).flat_map(|row| SHADES.iter().map(move |col| col[row])).map(|hex| view! {
                    <button
                        type="button"
                        class="aspect-square rounded-[4px] hover:outline hover:outline-2 hover:outline-offset-1 hover:outline-base-content"
                        style=format!("background:{hex}")
                        aria-label=hex
                        on:click=move |_| {
                            let key = picking.get_untracked();
                            state.apply(|d| d.style.set_color(key, Some(hex)));
                        }
                    ></button>
                }).collect_view()}
            </div>
            <div class="mt-2 flex items-center gap-2">
                <label class="flex flex-1 items-center gap-2 rounded-field border border-base-300 px-2 py-1 text-xs font-semibold">
                    <input
                        type="color"
                        class="h-5 w-5 cursor-pointer border-0 bg-transparent p-0"
                        prop:value=current
                        on:input=move |ev| {
                            let v = event_target_value(&ev);
                            let key = picking.get_untracked();
                            state.apply_grouped(Some(&format!("color-{key}")), |d| d.style.set_color(key, Some(&v)));
                        }
                    />
                    "Otro color"
                </label>
                <button
                    type="button"
                    class="btn btn-sm btn-ghost border border-base-300 text-xs"
                    on:click=move |_| {
                        let key = picking.get_untracked();
                        state.apply(|d| d.style.set_color(key, None));
                    }
                >
                    {move || if is_content() { "Automático" } else { "Como la plantilla" }}
                </button>
            </div>
        </div>
    }
}

/// Una fila de cinco esquinas para una familia de componentes.
#[component]
fn RadiusRow(which: Corner, title: &'static str, hint: &'static str) -> impl IntoView {
    let state = expect_context::<EditorState>();
    let ui = expect_context::<Ui>();
    view! {
        <div class="mt-2.5 first:mt-0">
            <p class="mb-1 flex items-baseline gap-1.5">
                <span class="text-xs font-semibold">{title}</span>
                <span class="text-[10.5px] text-base-content/55">{hint}</span>
            </p>
            <div class="grid grid-cols-5 gap-1" role="group" aria-label=title>
                {RADIUS_STEPS.iter().map(|px| {
                    let px = *px;
                    let active = move || state.doc.with(|d| d.style.radius(&d.theme, which) == px);
                    view! {
                        <button
                            type="button"
                            class="mk-rad"
                            style=format!("--r:{}px", f32::from(px) * 0.75)
                            aria-pressed=move || active().to_string()
                            aria-label=format!("{title}: {px} px")
                            on:click=move |_| {
                                state.apply(|d| d.style.set_radius(which, px));
                                ui.flash_changed(match which {
                                    Corner::Box => "box",
                                    Corner::Field => "field",
                                    Corner::Selector => "selector",
                                });
                            }
                        ></button>
                    }
                }).collect_view()}
            </div>
        </div>
    }
}

#[component]
fn Corners() -> impl IntoView {
    view! {
        <Section title="Esquinas" id="mk-th-corners">
            <RadiusRow which=Corner::Box title="Cajas" hint="tarjeta, ventana, aviso"/>
            <RadiusRow which=Corner::Field title="Campos" hint="botón, campo, pestaña"/>
            <RadiusRow which=Corner::Selector title="Selectores" hint="casilla, interruptor, etiqueta"/>
            <Margins/>
        </Section>
    }
}

/// Un interruptor con título y una línea de ayuda.
#[component]
fn SwitchRow(
    id: &'static str,
    title: &'static str,
    hint: &'static str,
    checked: Signal<bool>,
    on_change: Callback<bool>,
) -> impl IntoView {
    view! {
        <div class="flex items-center justify-between gap-3 py-1.5">
            <label for=id class="min-w-0">
                <span class="block text-[13px] font-semibold">{title}</span>
                <span class="block text-[10.5px] text-base-content/55">{hint}</span>
            </label>
            <input
                id=id
                type="checkbox"
                role="switch"
                class="toggle toggle-sm shrink-0"
                prop:checked=move || checked.get()
                on:change=move |ev| on_change.run(event_target_checked(&ev))
            />
        </div>
    }
}

#[component]
fn Effects() -> impl IntoView {
    let state = expect_context::<EditorState>();
    let ui = expect_context::<Ui>();
    view! {
        <Section title="Efectos" id="mk-th-effects">
            <SwitchRow
                id="mk-th-depth"
                title="Relieve"
                hint="Profundidad en campos y selectores"
                checked=Signal::derive(move || state.doc.with(|d| d.style.depth.unwrap_or_else(|| theme_base(&d.theme).depth)))
                on_change=Callback::new(move |on| {
                    state.apply(|d| d.style.depth = Some(on));
                    ui.flash_changed("field selector");
                })
            />
            <SwitchRow
                id="mk-th-noise"
                title="Textura"
                hint="Grano en campos y selectores"
                checked=Signal::derive(move || state.doc.with(|d| d.style.noise.unwrap_or_else(|| theme_base(&d.theme).noise)))
                on_change=Callback::new(move |on| {
                    state.apply(|d| d.style.noise = Some(on));
                    ui.flash_changed("field selector");
                })
            />
        </Section>
    }
}

fn px_text(v: f32) -> String {
    format!("{v:.1}").replace('.', ",")
}

/// Tamaño base (de 3 a 5 px) con barritas de XS a XL y un deslizador.
#[component]
fn SizeCard(
    title: &'static str,
    hint: &'static str,
    value: Signal<f32>,
    on_change: Callback<f32>,
    bars: [u8; 5],
) -> impl IntoView {
    let (lo, _, step) = SIZE_RANGE;
    let idx = move || (((value.get() - lo) / step).round() as i32).clamp(0, 4);
    view! {
        <div class="mt-2.5 first:mt-0">
            <p class="mb-1 flex items-baseline gap-1.5">
                <span class="text-xs font-semibold">{title}</span>
                <span class="text-[10.5px] text-base-content/55">{hint}</span>
            </p>
            <div class="flex items-center gap-3 rounded-box bg-base-200/70 px-3 py-2">
                <div class="flex h-9 items-end gap-1.5" aria-hidden="true">
                    {bars.iter().enumerate().map(|(i, h)| view! {
                        <span class="flex flex-col items-center gap-0.5">
                            <i
                                class="block w-[5px] rounded-sm"
                                class=("bg-primary", move || idx() == i as i32)
                                class=("bg-base-content/35", move || idx() != i as i32)
                                style=format!("height:{h}px")
                            ></i>
                            <span class="text-[8.5px] font-semibold text-base-content/55">{["XS", "SM", "MD", "LG", "XL"][i]}</span>
                        </span>
                    }).collect_view()}
                </div>
                <div class="min-w-0 flex-1">
                    <p class="text-right text-sm font-bold leading-none">{move || px_text(value.get())}<span class="ml-0.5 text-[10px] font-semibold text-base-content/55">"px"</span></p>
                    <input
                        type="range"
                        min="0"
                        max="4"
                        step="1"
                        class="range range-xs w-full"
                        aria-label=title
                        aria-valuetext=move || format!("{} píxeles", px_text(value.get()))
                        prop:value=move || idx().to_string()
                        on:input=move |ev| {
                            if let Ok(i) = event_target_value(&ev).parse::<f32>() {
                                on_change.run(lo + i * step);
                            }
                        }
                    />
                </div>
            </div>
        </div>
    }
}

#[component]
fn Sizes() -> impl IntoView {
    let state = expect_context::<EditorState>();
    let ui = expect_context::<Ui>();
    view! {
        <Section title="Tamaños" id="mk-th-sizes">
            <SizeCard
                title="Campos"
                hint="botón, campo, pestaña"
                bars=[10, 16, 22, 28, 34]
                value=Signal::derive(move || state.doc.with(|d| d.style.size_field.unwrap_or_else(|| theme_base(&d.theme).size_field)))
                on_change=Callback::new(move |v| {
                    state.apply_grouped(Some("size-field"), |d| d.style.size_field = Some(v));
                    ui.flash_changed("field");
                })
            />
            <SizeCard
                title="Selectores"
                hint="casilla, interruptor, etiqueta"
                bars=[8, 12, 16, 20, 26]
                value=Signal::derive(move || state.doc.with(|d| d.style.size_selector.unwrap_or_else(|| theme_base(&d.theme).size_selector)))
                on_change=Callback::new(move |v| {
                    state.apply_grouped(Some("size-selector"), |d| d.style.size_selector = Some(v));
                    ui.flash_changed("selector");
                })
            />
        </Section>
    }
}

#[component]
fn Borders() -> impl IntoView {
    let state = expect_context::<EditorState>();
    let ui = expect_context::<Ui>();
    let value = move || {
        state.doc.with(|d| {
            d.style
                .border
                .unwrap_or_else(|| theme_base(&d.theme).border)
        })
    };
    let idx = move || {
        let v = value();
        BORDER_STEPS
            .iter()
            .position(|s| (*s - v).abs() < 0.01)
            .unwrap_or(1)
    };
    view! {
        <Section title="Grosor de bordes" id="mk-th-border">
            <div class="relative rounded-box bg-base-200/70 px-3 pb-1.5 pt-6">
                <span
                    class="pointer-events-none absolute top-1.5 -translate-x-1/2 rounded-full bg-neutral px-2 py-0.5 text-[10.5px] font-semibold text-neutral-content"
                    style=move || format!("left:calc(0.75rem + (100% - 1.5rem) * {} / 4)", idx())
                    aria-hidden="true"
                >{move || format!("{}px", px_text(value()).trim_end_matches(",0"))}</span>
                <input
                    type="range"
                    min="0"
                    max="4"
                    step="1"
                    class="range range-xs w-full"
                    aria-label="Grosor de bordes"
                    aria-valuetext=move || format!("{} píxeles", px_text(value()))
                    prop:value=move || idx().to_string()
                    on:input=move |ev| {
                        if let Ok(i) = event_target_value(&ev).parse::<usize>() {
                            let v = BORDER_STEPS[i.min(4)];
                            state.apply_grouped(Some("border"), |d| d.style.border = Some(v));
                            ui.flash_changed("box field selector");
                        }
                    }
                />
            </div>
        </Section>
    }
}

#[component]
fn Options() -> impl IntoView {
    let state = expect_context::<EditorState>();
    view! {
        <Section title="Opciones" id="mk-th-options">
            <SwitchRow
                id="mk-th-dark"
                title="Esquema de color oscuro"
                hint="Formularios y barras del navegador en oscuro"
                checked=Signal::derive(move || state.doc.with(|d| d.style.dark))
                on_change=Callback::new(move |on| state.apply(|d| d.style.dark = on))
            />
            <button
                type="button"
                class="btn btn-sm mt-2 w-full border-0 bg-base-200 font-semibold text-error"
                disabled=move || state.doc.with(|d| d.style.theme_is_default() && d.style.name.is_empty())
                on:click=move |_| state.apply(|d| d.style.reset_theme())
            >"↺ Volver a la plantilla"</button>
        </Section>
    }
}
