//! Tema de la tienda dentro del cajón "Mi tienda": plantilla, colores,
//! esquinas, efectos, tamaños y bordes.
//!
//! Cada cambio se guarda en `PageDoc.style` (ver `domain::StoreStyle`) y se
//! ve al instante en el lienzo. Lo que no se toca queda como lo trae la
//! plantilla (un tema de daisyUI).

use domain::{
    opposite_colors, theme_base, theme_label, Corner, OFFERED_THEMES, THEMES, THEME_LABELS,
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
        <Appearance/>
        <Spacing/>
    }
}

/// Esquinas de tarjetas y botones (un solo valor para los tres radios del
/// tema), relieve y textura, con una muestra que cambia en vivo.
#[component]
fn Appearance() -> impl IntoView {
    let state = expect_context::<EditorState>();
    let ui = expect_context::<Ui>();
    // Los tres radios ya resueltos con el tema. Si difieren, son
    // "Personalizadas": no se redondean ni se tocan hasta que la persona
    // mueve el deslizador.
    let radii = Memo::new(move |_| {
        state.doc.with(|d| {
            [Corner::Box, Corner::Field, Corner::Selector].map(|c| d.style.radius(&d.theme, c))
        })
    });
    let custom = move || {
        let r = radii.get();
        r[0] != r[1] || r[1] != r[2]
    };
    let px = move || radii.get()[0];
    let shape = move || match px() {
        0 => "Rectas",
        24.. => "Redondeadas",
        _ => "Suaves",
    };
    let label = move || {
        if custom() {
            "Personalizadas".to_string()
        } else {
            format!("{} px · {}", px(), shape())
        }
    };
    let depth = Memo::new(move |_| {
        state
            .doc
            .with(|d| d.style.depth.unwrap_or_else(|| theme_base(&d.theme).depth))
    });
    let noise = Memo::new(move |_| {
        state
            .doc
            .with(|d| d.style.noise.unwrap_or_else(|| theme_base(&d.theme).noise))
    });
    // Colores de la muestra: los de la versión que se está viendo.
    let sample_style = move || {
        let alt = ui.edit_alt.get();
        let [bg, ink, pri, line, on] = state.doc.with(|d| {
            let c = |k: &str| d.style.variant_color(&d.theme, k, alt);
            [
                c("base-100"),
                c("base-content"),
                c("primary"),
                c("accent"),
                c("primary-content"),
            ]
        });
        let r = px();
        let shadow = if depth.get() {
            "0 2px 3px #00000018, 0 8px 18px #00000026"
        } else {
            "none"
        };
        let grain = if noise.get() {
            format!(
                "radial-gradient({ink}1e .5px,transparent .7px),radial-gradient({ink}1e .35px,transparent .6px)"
            )
        } else {
            "none".to_string()
        };
        format!(
            "--s-bg:{bg};--s-ink:{ink};--s-pri:{pri};--s-line:{line};--s-on:{on};--s-r:{r}px;--s-shadow:{shadow};--s-grain:{grain}"
        )
    };
    view! {
        <Section title="Apariencia" id="mk-th-appearance" help="Se refleja en los widgets.">
            <label class="flex items-baseline justify-between text-xs font-semibold" for="mk-th-radius">
                "Esquinas"
                <output class="font-normal text-base-content/70" for="mk-th-radius">{label}</output>
            </label>
            <div class="mk-appsample mt-1.5" style=sample_style>
                <div class="mk-appcard" aria-hidden="true"><i></i><i></i><span></span></div>
                <div class="min-w-0">
                    <b class="block text-[11px]">"Tarjetas y botones"</b>
                    <small class="mt-0.5 block text-[10px] text-base-content/60">"Así se ven tus bordes y efectos."</small>
                </div>
            </div>
            <input
                id="mk-th-radius"
                type="range"
                class="range range-primary range-xs mt-2 w-full"
                min="0"
                max="48"
                step="1"
                prop:value=move || px().to_string()
                aria-valuetext=move || {
                    if custom() {
                        "Esquinas personalizadas".to_string()
                    } else {
                        format!("{} píxeles, esquinas {}", px(), shape().to_lowercase())
                    }
                }
                on:input=move |ev| {
                    if let Ok(v) = event_target_value(&ev).trim().parse::<u8>() {
                        // Una sola operación deshacible para los tres radios.
                        state.apply_grouped(Some("radius"), |d| {
                            for c in [Corner::Box, Corner::Field, Corner::Selector] {
                                d.style.set_radius(c, v);
                            }
                        });
                        ui.flash_changed("field selector box");
                    }
                }
            />
            <div class="flex justify-between text-[10.5px] text-base-content/55" aria-hidden="true">
                <span>"Rectas"</span>
                <span>"Redondeadas"</span>
            </div>
            <div class="mt-3 grid gap-1.5">
                <SwitchRow
                    id="mk-th-depth"
                    title="Relieve sutil"
                    hint="Sombra en tarjetas y botones."
                    checked=Signal::derive(move || depth.get())
                    on_change=Callback::new(move |on| {
                        state.apply(|d| d.style.depth = Some(on));
                        ui.flash_changed("field selector");
                    })
                />
                <SwitchRow
                    id="mk-th-noise"
                    title="Textura suave"
                    hint="Grano fino sobre superficies, no sobre fotos."
                    checked=Signal::derive(move || noise.get())
                    on_change=Callback::new(move |on| {
                        state.apply(|d| d.style.noise = Some(on));
                        ui.flash_changed("field selector");
                    })
                />
            </div>
            <p class="mt-2 text-[10px] leading-snug text-base-content/60">
                "Las transparencias se ajustan en el widget correspondiente, para no afectar la legibilidad del texto."
            </p>
        </Section>
    }
}

/// Márgenes de la página y espacio entre widgets.
#[component]
fn Spacing() -> impl IntoView {
    view! {
        <Section title="Márgenes y espaciado" id="mk-th-spacing">
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
        <div class="flex items-center gap-2.5 rounded-field border border-base-300 bg-base-100 p-2">
            <div class="min-w-0">
                <label for=id class="block text-[11px] font-bold">{title}</label>
                <small class="block text-[10px] text-base-content/60">{hint}</small>
            </div>
            <input
                id=id
                type="checkbox"
                role="switch"
                class="toggle toggle-sm ml-auto shrink-0"
                prop:checked=move || checked.get()
                on:change=move |ev| on_change.run(event_target_checked(&ev))
            />
        </div>
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
