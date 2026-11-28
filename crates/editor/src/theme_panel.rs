//! Tema de la tienda dentro del cajón "Mi tienda": plantilla, colores,
//! esquinas, efectos, tamaños y bordes.
//!
//! Cada cambio se guarda en `PageDoc.style` (ver `domain::StoreStyle`) y se
//! ve al instante en el lienzo. Lo que no se toca queda como lo trae la
//! plantilla (un tema de daisyUI).

use domain::{
    opposite_colors, theme_base, theme_label, Corner, BORDER_STEPS, OFFERED_THEMES, RADIUS_STEPS,
    SIZE_RANGE, THEMES, THEME_LABELS,
};
use leptos::prelude::*;

use crate::icons::Icon;
use crate::store_drawer::{Letters, Margins, Section};
use crate::{EditorState, Ui};

#[component]
pub(crate) fn ThemePanel() -> impl IntoView {
    view! {
        <Themes/>
        <Letters/>
        <Corners/>
        <Effects/>
        <Sizes/>
        <Borders/>
        <Options/>
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
            <RadiusRow which=Corner::Box title="Tarjetas" hint="tarjetas, ventanas, avisos"/>
            <RadiusRow which=Corner::Field title="Botones y campos" hint="botones, campos, pestañas"/>
            <RadiusRow which=Corner::Selector title="Casillas e interruptores" hint="casillas, interruptores, etiquetas"/>
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
            <div class="relative flex min-w-0 items-center gap-1.5">
                <label for=id class="text-[13px] font-semibold">{title}</label>
                <ui::HelpTip id=format!("tip-{id}") label=title text=hint/>
            </div>
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
                hint="Sombra suave que hace que los campos y selectores parezcan levantados."
                checked=Signal::derive(move || state.doc.with(|d| d.style.depth.unwrap_or_else(|| theme_base(&d.theme).depth)))
                on_change=Callback::new(move |on| {
                    state.apply(|d| d.style.depth = Some(on));
                    ui.flash_changed("field selector");
                })
            />
            <SwitchRow
                id="mk-th-noise"
                title="Textura"
                hint="Un grano fino sobre los campos y selectores, como papel."
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
                title="Botones y campos"
                hint="botones, campos, pestañas"
                bars=[10, 16, 22, 28, 34]
                value=Signal::derive(move || state.doc.with(|d| d.style.size_field.unwrap_or_else(|| theme_base(&d.theme).size_field)))
                on_change=Callback::new(move |v| {
                    state.apply_grouped(Some("size-field"), |d| d.style.size_field = Some(v));
                    ui.flash_changed("field");
                })
            />
            <SizeCard
                title="Casillas e interruptores"
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
        <Section title="Grosor de bordes" id="mk-th-border" help="Qué tan gruesas son las líneas alrededor de botones, campos y tarjetas.">
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
                hint="Los formularios y las barras del navegador se ven en oscuro. Conviene si tu tienda tiene fondo oscuro."
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

/// Id del flotante "Editar tema".
const THEME_POP_ID: &str = "mk-themepop";

/// Fondo, texto, principal y acento (y el texto sobre el principal) con los
/// que se dibuja la muestra de un tema en la variante pedida.
fn sample_colors(state: EditorState, theme: &str, alt: bool) -> [String; 5] {
    state.doc.with(|d| {
        if d.theme == theme {
            let c = |k: &str| d.style.variant_color(theme, k, alt);
            [
                c("base-100"),
                c("base-content"),
                c("primary"),
                c("accent"),
                c("primary-content"),
            ]
        } else {
            let b = theme_base(theme);
            let c = |k: &str| b.colors[domain::color_index(k).unwrap_or(0)].to_string();
            if alt {
                let o = opposite_colors(
                    &c("base-100"),
                    &c("base-content"),
                    &c("primary"),
                    &c("accent"),
                    !b.dark,
                );
                let i = |k: &str| o[domain::color_index(k).unwrap_or(0)].clone();
                [
                    i("base-100"),
                    i("base-content"),
                    i("primary"),
                    i("accent"),
                    i("primary-content"),
                ]
            } else {
                [
                    c("base-100"),
                    c("base-content"),
                    c("primary"),
                    c("accent"),
                    c("primary-content"),
                ]
            }
        }
    })
}

/// Temas: uno a la vez, con flechas e indicadores; versión clara u oscura
/// que se edita por separado; y un flotante con los cuatro colores.
#[component]
fn Themes() -> impl IntoView {
    let state = expect_context::<EditorState>();
    let ui = expect_context::<Ui>();
    let n = OFFERED_THEMES;
    let theme = Memo::new(move |_| state.doc.with(|d| d.theme.clone()));
    // Posición entre los que se ofrecen; `None` = tema anterior.
    let current = Memo::new(move |_| theme.with(|t| THEMES.iter().take(n).position(|x| x == t)));
    let pick = move |i: usize| {
        if let Some(t) = THEMES.get(i) {
            state.apply(|d| {
                d.theme = (*t).to_string();
                d.style.switch_theme();
            });
        }
    };
    let step = move |delta: isize| {
        let i = match current.get_untracked() {
            Some(i) => (i as isize + delta).rem_euclid(n as isize) as usize,
            None if delta > 0 => 0,
            None => n - 1,
        };
        pick(i);
    };
    let alt = move || ui.edit_alt.get();
    // El modo que se está viendo es oscuro (según el tema y la variante).
    let dark_now = move || theme.with(|t| theme_base(t).dark) != alt();
    let name = move || {
        if current.get().is_some() {
            theme.with(|t| theme_label(t).to_string())
        } else {
            format!(
                "Tema anterior · {}",
                theme.with(|t| theme_label(t).to_string())
            )
        }
    };
    let colors = move || {
        let t = theme.get();
        sample_colors(state, &t, alt())
    };
    let set_dark = move |dark: bool| {
        let native = theme.with_untracked(|t| theme_base(t).dark);
        ui.edit_alt.set(dark != native);
    };
    let rows: [(&'static str, &'static str); 4] = [
        ("base-100", "Fondo"),
        ("base-content", "Texto"),
        ("primary", "Principal"),
        ("accent", "Acento"),
    ];
    view! {
        <Section title="Temas" id="mk-th-themes" help="Cada tema tiene versión clara y oscura, que se editan por separado.">
            <div role="group" aria-label="Temas de la tienda">
                <div class="mk-fontpick">
                    <button type="button" class="mk-fontarrow" aria-label="Tema anterior" on:click=move |_| step(-1)>
                        <Icon name="chevron-left"/>
                    </button>
                    <div class="min-w-0 flex-1" aria-live="polite" aria-atomic="true">
                        <div class="flex items-baseline gap-1.5 px-0.5 pb-1 text-[11px] font-bold text-base-content/70">
                            <span>{name}</span>
                            <span class="ml-auto font-normal text-base-content/55">
                                {move || current.get().map(|i| format!("{} de {n}", i + 1)).unwrap_or_default()}
                            </span>
                        </div>
                        <div
                            class="overflow-hidden rounded-md border px-2.5 py-2"
                            aria-hidden="true"
                            style=move || {
                                let [bg, ink, ..] = colors();
                                format!("background:{bg};color:{ink};border-color:color-mix(in srgb,{ink} 15%,transparent)")
                            }
                        >
                            <div class="flex items-center justify-between gap-2 border-b pb-1" style=move || {
                                let [_, ink, ..] = colors();
                                format!("border-color:color-mix(in srgb,{ink} 16%,transparent)")
                            }>
                                <b class="text-[10px] tracking-tight">"nido."</b>
                                <span class="flex gap-[3px]"><i class="h-[5px] w-[5px] rounded-full bg-current opacity-40"></i><i class="h-[5px] w-[5px] rounded-full bg-current opacity-40"></i><i class="h-[5px] w-[5px] rounded-full bg-current opacity-40"></i></span>
                            </div>
                            <div class="flex items-center justify-between gap-2 pt-1.5">
                                <strong class="min-w-0 text-xs leading-tight">"Tu tienda, tu estilo."</strong>
                                <span class="shrink-0 rounded-[3px] px-1.5 py-1 text-[8px] font-bold" style=move || {
                                    let [_, _, pri, _, on] = colors();
                                    format!("background:{pri};color:{on}")
                                }>"Explorar"</span>
                            </div>
                        </div>
                    </div>
                    <button type="button" class="mk-fontarrow" aria-label="Tema siguiente" on:click=move |_| step(1)>
                        <Icon name="chevron-right"/>
                    </button>
                </div>
                <div class="mk-markers" role="group" aria-label="Elegir tema">
                    {THEMES.iter().zip(THEME_LABELS).take(n).enumerate().map(|(i, (_, l))| view! {
                        <button
                            type="button"
                            aria-label=*l
                            title=*l
                            aria-pressed=move || (current.get() == Some(i)).to_string()
                            on:click=move |_| pick(i)
                        ></button>
                    }).collect_view()}
                </div>
            </div>
            <div class="mt-1.5 flex items-center justify-between gap-2">
                <button
                    type="button"
                    class="inline-flex items-center gap-2 rounded-field border border-base-300 bg-base-200/60 px-2.5 py-1.5 text-xs font-bold text-primary"
                    popovertarget=THEME_POP_ID
                >
                    <span aria-hidden="true">"◈"</span>
                    "Editar tema"
                </button>
                <div class="mk-modepick" role="group" aria-label="Versión del tema que se edita">
                    <button
                        type="button"
                        aria-label="Versión clara"
                        title="Versión clara"
                        aria-pressed=move || (!dark_now()).to_string()
                        on:click=move |_| set_dark(false)
                    >"☀"</button>
                    <button
                        type="button"
                        aria-label="Versión oscura"
                        title="Versión oscura"
                        aria-pressed=move || dark_now().to_string()
                        on:click=move |_| set_dark(true)
                    >"☾"</button>
                </div>
            </div>
            <div
                id=THEME_POP_ID
                popover="auto"
                class="mk-colorpop mk-typepop"
                role="dialog"
                aria-labelledby="mk-themepop-title"
                on:keydown=move |ev| {
                    if ev.key() == "Escape" {
                        ev.stop_propagation();
                    }
                }
            >
                <div class="mb-3 flex items-start gap-2">
                    <div>
                        <h3 id="mk-themepop-title" class="text-sm font-bold">
                            {move || format!("Editar tema · {}", theme.with(|t| theme_label(t).to_string()))}
                        </h3>
                        <p class="text-[11px] text-base-content/60">
                            {move || if dark_now() {
                                "Editando modo oscuro. Los cambios se ven en la tienda."
                            } else {
                                "Editando modo claro. Los cambios se ven en la tienda."
                            }}
                        </p>
                    </div>
                    <button
                        type="button"
                        class="btn btn-ghost btn-xs btn-square ml-auto"
                        aria-label="Cerrar editor de tema"
                        popovertarget=THEME_POP_ID
                        popovertargetaction="hide"
                    >
                        <Icon name="close"/>
                    </button>
                </div>
                <div class="grid grid-cols-2 gap-1.5">
                    {rows.map(|(key, label)| {
                        let hex = move || {
                            let t = theme.get();
                            state.doc.with(|d| d.style.variant_color(&t, key, alt()))
                        };
                        view! {
                            <label class="relative flex min-w-0 cursor-pointer items-center gap-2 rounded-field border border-base-300 bg-base-100 px-2 py-1.5 focus-within:outline focus-within:outline-2 focus-within:outline-offset-2 focus-within:outline-primary">
                                <span class="h-5 w-5 shrink-0 rounded-md border border-black/15" style=move || format!("background:{}", hex())></span>
                                <span class="min-w-0">
                                    <b class="block text-[10px]">{label}</b>
                                    <small class="block text-[9px] uppercase text-base-content/60">{move || hex()}</small>
                                </span>
                                <input
                                    type="color"
                                    class="absolute inset-0 h-full w-full cursor-pointer opacity-0"
                                    aria-label=label
                                    prop:value=hex
                                    on:input=move |ev| {
                                        let v = event_target_value(&ev);
                                        let a = ui.edit_alt.get_untracked();
                                        state.apply_grouped(Some(&format!("vcolor-{key}-{a}")), |d| d.style.set_variant_color(key, Some(&v), a));
                                    }
                                />
                            </label>
                        }
                    })}
                </div>
                <p class="mt-2.5 flex items-start gap-2 rounded-field border border-base-300 bg-base-200/60 p-2 text-[10px] leading-snug text-base-content/70">
                    <span class="text-primary" aria-hidden="true">"ⓘ"</span>
                    <span>"El texto de los botones se adapta al color principal."</span>
                </p>
                <div class="mt-3 flex items-center justify-between gap-2">
                    <button
                        type="button"
                        class="text-[11px] font-bold text-primary"
                        aria-pressed=move || state.doc.with(|d| !d.style.reset_colors_on_theme).to_string()
                        on:click=move |_| state.apply(|d| d.style.reset_colors_on_theme = !d.style.reset_colors_on_theme)
                    >
                        {move || if state.doc.with(|d| d.style.reset_colors_on_theme) { "Conservar colores al cambiar de tema" } else { "✓ Conservar colores al cambiar de tema" }}
                    </button>
                    <button type="button" class="btn btn-primary btn-xs" popovertarget=THEME_POP_ID popovertargetaction="hide">"Listo"</button>
                </div>
            </div>
        </Section>
    }
}
