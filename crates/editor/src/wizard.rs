//! Asistente de primera página: una ventana sobre el editor donde se arma la
//! página en cinco pasos (encabezado, portada, filtros, productos y pie).
//!
//! En el centro está la página de verdad, que se va armando; las flechas
//! cambian de opción con un deslizamiento. A la derecha están los ajustes
//! simples del paso. Lo que se elige no toca la página hasta el final, y se
//! aplica como un solo paso de deshacer. La lógica de qué bloque sale de cada
//! opción está en `domain::wizard`.

use domain::wizard::{
    self, Change, Section, WizardPick, BG_KINDS, HERO_HEIGHTS, WIZARD, WIZARD_COLORS, WIZARD_STEPS,
};
use domain::{
    Layer, Node, Product, Widget, ALIGN_OPTIONS, CARD_STYLE_OPTIONS, DESIGNS, FONTS, MAX_BG_DIM,
    NAV_MENU_OPTIONS, NAV_TONE_OPTIONS, ROW_PX, TEXT_ANIMATIONS,
};
use leptos::prelude::*;
use serde_json::Value;
use widgets::{render_widget, sample_product, RenderCtx};

use crate::fields::ImagePicker;
use crate::icons::Icon;
use crate::{storage, Actions, EditorState, Ui};

/// Pasos de la ventana: el fondo de la web y, después, uno por bloque.
const UI_STEPS: usize = WIZARD_STEPS + 1;

fn step_name(i: usize) -> &'static str {
    if i == 0 {
        "Fondo"
    } else {
        WIZARD[i - 1].name
    }
}

/// Qué control usa cada ajuste.
#[derive(Clone, Copy)]
enum Ctl {
    Text,
    Toggle,
    Select(&'static [(&'static str, &'static str)]),
    /// Mínimo, máximo y unidad.
    Slider(i64, i64, &'static str),
    Photo,
}

type RowSpec = (&'static str, &'static str, Ctl);

/// Ajustes de contenido de cada bloque: campo, nombre y control. Un ajuste
/// que el bloque elegido no tiene (el título en un carrusel, por ejemplo)
/// no se muestra.
fn content_rows(section: Section) -> &'static [RowSpec] {
    match section {
        Section::Nav => &[
            ("brand", "Nombre de la marca", Ctl::Text),
            ("logo_url", "Logo", Ctl::Photo),
            ("button_label", "Texto del botón", Ctl::Text),
            ("search", "Buscador de productos", Ctl::Toggle),
            ("menu", "Menú desplegable", Ctl::Select(NAV_MENU_OPTIONS)),
        ],
        Section::Hero => &[
            ("title", "Título", Ctl::Text),
            ("subtitle", "Subtítulo", Ctl::Text),
            ("image_url", "Imagen", Ctl::Photo),
            ("button_label", "Texto del botón", Ctl::Text),
            ("autoplay", "Cambia cada", Ctl::Slider(0, 30, " s")),
        ],
        Section::Filters => &[
            ("title", "Título", Ctl::Text),
            ("all_label", "Texto de «Todas»", Ctl::Text),
            ("placeholder", "Texto del buscador", Ctl::Text),
            ("show_counts", "Cantidad por categoría", Ctl::Toggle),
        ],
        Section::Products => &[
            ("title", "Título", Ctl::Text),
            ("limit", "Cuántos productos", Ctl::Slider(4, 24, "")),
            ("card", "Tarjeta", Ctl::Select(CARD_STYLE_OPTIONS)),
            ("card_favorite", "Corazón de favoritos", Ctl::Toggle),
            ("card_colors", "Puntos de color y talle", Ctl::Toggle),
            ("card_transfer", "Precio por transferencia", Ctl::Toggle),
            ("card_installments", "Cuotas sin interés", Ctl::Toggle),
        ],
        Section::Footer => &[
            ("brand", "Nombre de la tienda", Ctl::Text),
            ("description", "Descripción", Ctl::Text),
            ("legal", "Texto legal", Ctl::Text),
        ],
    }
}

/// Colores de la barra de color, en el mismo orden que [`WIZARD_COLORS`].
const COLOR_VARS: [&str; 4] = ["primary", "secondary", "accent", "neutral"];

/// Colores sugeridos para el fondo de la web.
const BG_PRESETS: [&str; 6] = [
    "#f3f4f6", "#ffffff", "#e0e7ff", "#fce7f3", "#d1fae5", "#1f2937",
];

/// Ancho de la página en el escenario, en píxeles de pantalla.
const PAGE_W: f64 = 640.0;
/// Espacio entre bloques en el escenario.
const GAP: f64 = 6.0;
/// Alto de «Sin portada» y «Sin filtros».
const NONE_H: f64 = 44.0;

fn scale() -> f64 {
    PAGE_W / f64::from(Layer::Page(0).design_width())
}

fn node_h(node: &Node) -> f64 {
    f64::from(node.size.h) * f64::from(ROW_PX) * scale()
}

/// Alto del esqueleto de un bloque que todavía no se eligió.
fn skeleton_h(section: Section) -> f64 {
    let name = match section {
        Section::Nav => "Navbar",
        Section::Hero => "Banner",
        Section::Filters => "CategoryTabs",
        Section::Products => "ProductGrid",
        Section::Footer => "Footer",
    };
    Widget::defaults(name)
        .map(|w| f64::from(w.limits().default.h) * f64::from(ROW_PX) * scale())
        .unwrap_or(60.0)
}

fn none_label(section: Section) -> &'static str {
    match section {
        Section::Hero => "Sin portada",
        _ => "Sin filtros",
    }
}

/// El bloque dibujado de verdad y achicado al ancho del escenario.
fn scaled(node: &Node, ctx: &RenderCtx) -> AnyView {
    let s = scale();
    let design_w = f64::from(Layer::Page(0).design_width());
    let h = f64::from(node.size.h) * f64::from(ROW_PX);
    view! {
        <div class="mk-wiz-fit" style=format!("width:{PAGE_W}px;height:{}px", h * s)>
            <div
                class="pointer-events-none select-none"
                style=format!("width:{design_w}px;height:{h}px;transform:scale({s});transform-origin:0 0")
                aria-hidden="true"
            >
                {render_widget(node, ctx)}
            </div>
        </div>
    }
    .into_any()
}

/// Esqueleto con la forma de cada bloque, mientras no se eligió.
fn skeleton(section: Section) -> AnyView {
    match section {
        Section::Nav => view! {
            <i style="left:12px;top:30%;width:64px;height:40%"></i>
            <i style="right:12px;top:30%;width:34%;height:40%"></i>
        }
        .into_any(),
        Section::Hero => view! {
            <i style="left:7%;top:38%;width:34%;height:12%"></i>
            <i style="left:7%;top:56%;width:24%;height:7%"></i>
            <i style="right:6%;top:14%;width:40%;height:72%"></i>
        }
        .into_any(),
        Section::Filters => view! {
            <i style="left:10px;top:34%;width:14%;height:30%"></i>
            <i style="left:18%;top:34%;width:14%;height:30%"></i>
            <i style="left:34%;top:34%;width:14%;height:30%"></i>
        }
        .into_any(),
        Section::Products => (0..4)
            .map(|j| {
                view! {
                    <i style=format!("left:{}%;top:4%;width:23%;height:70%", j * 25 + 1)></i>
                    <i style=format!("left:{}%;top:80%;width:14%;height:6%", j * 25 + 1)></i>
                }
            })
            .collect_view()
            .into_any(),
        Section::Footer => view! {
            <i style="left:12px;top:24%;width:90px;height:16%"></i>
            <i style="left:12px;top:56%;width:36%;height:10%"></i>
        }
        .into_any(),
    }
}

/// Una fila de ajustes: nombre a la izquierda y el control a la derecha.
#[component]
fn Row(label: &'static str, id: String, children: Children) -> impl IntoView {
    view! {
        <div class="mk-wiz-row">
            <label class="mk-wiz-rl" for=id>{label}</label>
            {children()}
        </div>
    }
}

#[component]
fn TxtRow(
    label: &'static str,
    id: String,
    value: Signal<String>,
    max: Signal<usize>,
    on_pick: Callback<String>,
) -> impl IntoView {
    view! {
        <Row label=label id=id.clone()>
            <input
                id=id
                type="text"
                class="mk-wiz-inp"
                maxlength=move || max.get().to_string()
                autocomplete="off"
                prop:value=move || value.get()
                on:input=move |ev| on_pick.run(event_target_value(&ev))
            />
        </Row>
    }
}

#[component]
fn SelRow(
    label: &'static str,
    id: String,
    options: Vec<(String, String)>,
    value: Signal<String>,
    on_pick: Callback<String>,
) -> impl IntoView {
    view! {
        <Row label=label id=id.clone()>
            <select
                id=id
                class="mk-wiz-selv"
                prop:value=move || value.get()
                on:change=move |ev| on_pick.run(event_target_value(&ev))
            >
                {options.into_iter().map(|(v, l)| view! { <option value=v>{l}</option> }).collect_view()}
            </select>
        </Row>
    }
}

#[component]
fn TglRow(
    label: &'static str,
    id: String,
    value: Signal<bool>,
    on_pick: Callback<bool>,
) -> impl IntoView {
    let label_id = format!("{id}-l");
    view! {
        <div class="mk-wiz-row">
            <span class="mk-wiz-rl" id=label_id.clone()>{label}</span>
            <button
                type="button"
                id=id
                class="mk-wiz-sw"
                role="switch"
                aria-labelledby=label_id
                aria-checked=move || value.get().to_string()
                on:click=move |_| on_pick.run(!value.get_untracked())
            ></button>
        </div>
    }
}

/// Barra deslizante para una cantidad, con el valor a la vista y las marcas
/// de los extremos (o de cada paso, si `ticks` las trae).
#[component]
fn SldRow(
    label: &'static str,
    id: String,
    min: i64,
    max: i64,
    unit: &'static str,
    ticks: Vec<&'static str>,
    value: Signal<i64>,
    on_pick: Callback<i64>,
) -> impl IntoView {
    let shown = {
        let ticks = ticks.clone();
        Signal::derive(move || {
            let v = value.get();
            ticks
                .get((v - min) as usize)
                .map_or_else(|| format!("{v}{unit}"), |t| (*t).to_string())
        })
    };
    let marks = if ticks.is_empty() {
        vec![format!("{min}{unit}"), format!("{max}{unit}")]
    } else {
        ticks.iter().map(|t| (*t).to_string()).collect()
    };
    view! {
        <div class="mk-wiz-row is-col">
            <label class="mk-wiz-rl is-split" for=id.clone()><span>{label}</span><output>{move || shown.get()}</output></label>
            <input
                id=id
                type="range"
                class="range range-primary range-xs w-full"
                min=min
                max=max
                step="1"
                aria-valuetext=move || shown.get()
                prop:value=move || value.get().to_string()
                on:input=move |ev| {
                    if let Ok(v) = event_target_value(&ev).parse::<i64>() {
                        on_pick.run(v.clamp(min, max));
                    }
                }
            />
            <div class="mk-wiz-ticks" aria-hidden="true">
                {marks.into_iter().map(|m| view! { <span>{m}</span> }).collect_view()}
            </div>
        </div>
    }
}

/// Un grupo de ajustes: título chico y una tarjeta con las filas.
#[component]
fn Group(name: &'static str, d: u8, children: Children) -> impl IntoView {
    view! {
        <section class="mk-wiz-grp" style=format!("--d:{d}")>
            <h4 class="mk-wiz-gh">{name}</h4>
            <div class="mk-wiz-box">{children()}</div>
        </section>
    }
}

/// Panel del primer paso: el fondo de la web, que queda debajo de los bloques.
fn background_panel(
    pick: RwSignal<WizardPick>,
    style: String,
    theme: impl Fn() -> String + Copy + Send + Sync + 'static,
) -> AnyView {
    let kind = move || pick.with(|p| p.bg_kind);
    let set_color = move |c: String| {
        pick.update(|p| {
            p.bg_color = c;
            if p.bg_kind == 0 {
                p.bg_kind = 1;
            }
        })
    };
    let custom_value = move || {
        let c = pick.with(|p| p.bg_color.clone());
        if c.is_empty() {
            "#ffffff".to_string()
        } else {
            c
        }
    };
    view! {
        <div class="mk-wiz-pin" style=style>
            <div style="--d:0">
                <h3 class="mk-wiz-h">"Fondo de la web"</h3>
                <p class="mk-wiz-sub">"Tus bloques van encima del fondo."</p>
            </div>
            <Group name="Fondo" d=1>
                <SelRow label="Tipo" id="wz-bg-kind".to_string()
                    options=BG_KINDS.iter().enumerate().map(|(i, n)| (i.to_string(), n.to_string())).collect()
                    value=Signal::derive(move || kind().to_string())
                    on_pick=Callback::new(move |v: String| pick.update(|p| {
                        p.bg_kind = v.parse().unwrap_or(0);
                        if p.bg_kind == 2 && p.bg_dim == 0 {
                            p.bg_dim = 30;
                        }
                        if p.bg_kind == 1 && p.bg_color.is_empty() {
                            p.bg_color = BG_PRESETS[0].to_string();
                        }
                    }))/>
                <div class="mk-wiz-rw" hidden=move || kind() != 2>
                    <div class="mk-wiz-row is-col">
                        <span class="mk-wiz-rl">"Imagen"</span>
                        <ImagePicker uid="wz-bg-img".to_string()
                            value=Signal::derive(move || Value::String(pick.with(|p| p.bg_image.clone())))
                            pick=Callback::new(move |v: Value| pick.update(|p| p.bg_image = v.as_str().unwrap_or_default().to_string()))
                            on_photo=None/>
                    </div>
                </div>
                <div class="mk-wiz-rw" hidden=move || kind() != 2>
                    <SldRow label="Oscurecer la imagen" id="wz-bg-dim".to_string() min=0 max=i64::from(MAX_BG_DIM) unit=" %" ticks=Vec::new()
                        value=Signal::derive(move || pick.with(|p| i64::from(p.bg_dim)))
                        on_pick=Callback::new(move |v: i64| pick.update(|p| p.bg_dim = v as u8))/>
                </div>
                <div class="mk-wiz-rw" hidden=move || kind() != 2>
                    <TglRow label="Fija al desplazar" id="wz-bg-fixed".to_string()
                        value=Signal::derive(move || pick.with(|p| p.bg_fixed))
                        on_pick=Callback::new(move |v: bool| pick.update(|p| p.bg_fixed = v))/>
                </div>
                <div class="mk-wiz-rw" hidden=move || kind() == 0>
                    <div class="mk-wiz-row is-col">
                        <span class="mk-wiz-rl" id="wz-bg-color-l">{move || if kind() == 2 { "Color de respaldo" } else { "Color" }}</span>
                        <div class="mk-wiz-sws" role="radiogroup" aria-labelledby="wz-bg-color-l" data-theme=theme>
                            {BG_PRESETS.iter().map(|c| {
                                let c = *c;
                                view! {
                                    <label title=c>
                                        <input type="radio" name="wz-bg-color" aria-label=c
                                            prop:checked=move || pick.with(|p| p.bg_color == c)
                                            on:change=move |_| set_color(c.to_string())/>
                                        <i style=format!("background:{c}")></i>
                                    </label>
                                }
                            }).collect_view()}
                            <label title="Otro color" class="is-custom">
                                <input type="color" aria-label="Otro color"
                                    prop:value=custom_value
                                    on:input=move |ev| set_color(event_target_value(&ev))/>
                                <i></i>
                            </label>
                        </div>
                    </div>
                </div>
                <p class="mk-wiz-note" hidden=move || kind() != 0>"Se mantiene el fondo actual de tu tienda."</p>
            </Group>
        </div>
    }
    .into_any()
}

#[component]
pub(crate) fn Wizard(products: StoredValue<Vec<Product>>, store_key: String) -> impl IntoView {
    let state = expect_context::<EditorState>();
    let ui = expect_context::<Ui>();
    let actions = expect_context::<Actions>();
    let dialog = NodeRef::<leptos::html::Dialog>::new();
    let draft_key = StoredValue::new(format!("mk-asistente:{store_key}"));
    let seen_key = StoredValue::new(format!("mk-asistente-visto:{store_key}"));

    let step = RwSignal::new(0usize);
    let intro = RwSignal::new(false);
    let done = RwSignal::new(false);
    let confirm = RwSignal::new(false);
    let pick = RwSignal::new(WizardPick::default());
    let reached = RwSignal::new([true, false, false, false, false, false]);
    // Hacia dónde se va (+1 adelante, -1 atrás): decide de qué lado entra lo nuevo.
    let dir = RwSignal::new(0i32);
    // Las flechas laten hasta que se usan por primera vez.
    let coach = RwSignal::new(true);
    let live = RwSignal::new(String::new());
    let base = StoredValue::new(state.doc.get_untracked());
    let untouched = StoredValue::new(false);
    let auto = StoredValue::new(false);
    // Lo elegido al abrir: en cuanto algo cambia, la ayuda de las flechas sobra.
    let opened_pick = StoredValue::new(WizardPick::default());

    // Al abrir: parte de lo que ya hay en la página o retoma el borrador.
    let start = move || {
        let doc = state.doc.get_untracked();
        let is_new = wizard::is_untouched(&doc);
        untouched.set_value(is_new);
        base.set_value(doc.clone());
        let draft = storage()
            .and_then(|st| st.get_item(&draft_key.get_value()).ok().flatten())
            .and_then(|s| serde_json::from_str::<(WizardPick, usize)>(&s).ok());
        let (p, s) = match draft {
            Some((p, s)) => (p.clamped(), s.min(UI_STEPS - 1)),
            None => (wizard::pick_from_doc(&doc), 0),
        };
        let mut p = p;
        p.products_total = products.with_value(Vec::len);
        opened_pick.set_value(p.clone());
        pick.set(p);
        step.set(s);
        reached.set(if is_new {
            let mut r = [false; UI_STEPS];
            r.iter_mut().take(s + 1).for_each(|x| *x = true);
            r
        } else {
            [true; UI_STEPS]
        });
        intro.set(auto.get_value());
        auto.set_value(false);
        done.set(false);
        confirm.set(false);
        coach.set(true);
        dir.set(0);
        if let Some(st) = storage() {
            _ = st.set_item(&seen_key.get_value(), "1");
        }
        live.set(format!(
            "Asistente abierto. Paso {} de {}.",
            s + 1,
            UI_STEPS
        ));
    };

    Effect::new(move |_| {
        if pick.with(|p| *p != opened_pick.get_value()) {
            coach.set(false);
        }
    });

    // Se abre solo, una vez, en una tienda nueva con la página inicial.
    Effect::new(move |ran: Option<()>| {
        if ran.is_some() {
            return;
        }
        let Some(st) = storage() else {
            return;
        };
        if st.get_item(&seen_key.get_value()).ok().flatten().is_some() {
            return;
        }
        if wizard::is_untouched(&state.doc.get_untracked()) {
            auto.set_value(true);
            ui.wizard.set(true);
        }
    });

    // La ventana nativa sigue a la señal.
    Effect::new(move |prev: Option<bool>| {
        let open = ui.wizard.get();
        if open && prev != Some(true) {
            start();
        }
        if let Some(d) = dialog.get() {
            if open && !d.open() {
                _ = d.show_modal();
            } else if !open && d.open() {
                d.close();
            }
        }
        open
    });

    // Lo elegido queda como borrador: cerrar no pierde nada.
    Effect::new(move |_| {
        let (p, s) = (pick.get(), step.get());
        if !ui.wizard.get_untracked() || done.get_untracked() {
            return;
        }
        if let (Some(st), Ok(json)) = (storage(), serde_json::to_string(&(p, s))) {
            _ = st.set_item(&draft_key.get_value(), &json);
        }
    });

    let nodes = Memo::new(move |_| {
        let p = pick.get();
        let b = base.get_value();
        Section::ALL.map(|s| wizard::section_node(s, &p, &b))
    });
    // Las tres opciones del paso actual, para el deslizamiento.
    let slides = Memo::new(move |_| {
        let Some(st) = step.get().checked_sub(1) else {
            return Vec::new();
        };
        let p = pick.get();
        let b = base.get_value();
        (0..3u8)
            .map(|j| {
                let mut q = p.clone();
                q.choice[st] = j;
                wizard::section_node(Section::ALL[st], &q, &b)
            })
            .collect::<Vec<_>>()
    });
    let ctx = move || {
        let mut ps = products.get_value();
        // Sin productos cargados se muestran de ejemplo.
        if ps.is_empty() {
            ps = (0..8)
                .map(|i| {
                    let mut p = sample_product();
                    p.id = format!("{}-{i}", p.id);
                    p
                })
                .collect();
        }
        RenderCtx {
            products: ps,
            editing: true,
            product: None,
        }
    };
    // Dónde queda cada bloque en el escenario.
    let geo = Memo::new(move |_| {
        let nodes = nodes.get();
        let r = reached.get();
        let mut tops = [0f64; WIZARD_STEPS];
        let mut hs = [0f64; WIZARD_STEPS];
        let mut y = 10.0;
        for (i, section) in Section::ALL.into_iter().enumerate() {
            let h = if r[i + 1] || step.get() == 0 {
                nodes[i].as_ref().map_or(NONE_H, node_h)
            } else {
                skeleton_h(section)
            };
            tops[i] = y;
            hs[i] = h;
            y += h + GAP;
        }
        (tops, hs, y + 10.0)
    });
    let camera = move || {
        let (tops, hs, total) = geo.get();
        if done.get() || confirm.get() || step.get() == 0 {
            let k = (520.0 / total).min(1.0);
            format!("width:{PAGE_W}px; transform: translate(-50%, 14px) scale({k})")
        } else {
            let st = step.get() - 1;
            let c = tops[st] + hs[st] / 2.0;
            format!(
                "width:{PAGE_W}px; transform: translate(-50%, min(40px, calc(50cqh - {c}px - 24px)))"
            )
        }
    };
    let theme = move || state.doc.with(|d| d.theme.clone());
    // El fondo elegido, detrás de la página, mientras se arma.
    let bg_style = move || {
        let p = pick.get();
        let mut st = state.doc.with(|d| d.style.clone());
        wizard::apply_background(&mut st, &p);
        st.css_vars()
    };
    let store_vars = move || state.doc.with(|d| d.typography.css_vars());

    // --- Acciones ---
    let pick_choice = move |j: usize| {
        let Some(st) = step.get_untracked().checked_sub(1) else {
            return;
        };
        let cur = usize::from(pick.get_untracked().choice[st]);
        if j == cur {
            return;
        }
        dir.set(if (j + 3 - cur) % 3 == 1 { 1 } else { -1 });
        pick.update(|p| p.choice[st] = j as u8);
        coach.set(false);
        live.set(format!(
            "{}: {}, opción {} de 3",
            WIZARD[st].name,
            WIZARD[st].options[j].name,
            j + 1
        ));
    };
    let shift_choice = move |d: i32| {
        if intro.get_untracked() || done.get_untracked() || confirm.get_untracked() {
            return;
        }
        let Some(st) = step.get_untracked().checked_sub(1) else {
            return;
        };
        let n = WIZARD[st].options.len() as i32;
        let cur = i32::from(pick.get_untracked().choice[st]);
        pick_choice(((cur + d).rem_euclid(n)) as usize);
    };
    let go_to = move |n: usize, d: i32| {
        dir.set(d);
        step.set(n);
        reached.update(|r| r[n] = true);
        confirm.set(false);
        live.set(format!("Paso {} de {}: {}", n + 1, UI_STEPS, step_name(n)));
    };
    let clear_draft = move || {
        if let Some(st) = storage() {
            _ = st.remove_item(&draft_key.get_value());
        }
    };
    let apply_now = move || {
        let p = pick.get_untracked();
        // Un solo paso de deshacer para todo lo que arma el asistente.
        state.apply(|d| {
            wizard::apply(d, &p);
            grid::compact(&mut d.nodes);
        });
        done.set(true);
        confirm.set(false);
        clear_draft();
        ui.wizard_applied.update(|n| *n += 1);
        actions
            .ui
            .announce("Armamos tu página. Podés deshacerlo con Ctrl y Z.");
    };
    let next = move |_| {
        if confirm.get_untracked() {
            apply_now();
        } else if step.get_untracked() + 1 < UI_STEPS {
            go_to(step.get_untracked() + 1, 1);
        } else if untouched.get_value() {
            apply_now();
        } else {
            reached.set([true; UI_STEPS]);
            confirm.set(true);
            live.set("Revisá los cambios antes de aplicarlos.".to_string());
        }
    };
    let back = move |_| {
        if confirm.get_untracked() {
            confirm.set(false);
        } else if step.get_untracked() > 0 {
            go_to(step.get_untracked() - 1, -1);
        }
    };
    let close = move |_| ui.wizard.set(false);

    // --- Piezas de la ventana ---
    let steps_bar = move || {
        (0..UI_STEPS)
            .map(|i| {
                let name = step_name(i);
                let state_of = move || {
                    if done.get() || confirm.get() || i < step.get() {
                        "done"
                    } else if i == step.get() {
                        "on"
                    } else {
                        ""
                    }
                };
                let can_go = move || reached.get()[i] && !done.get() && i != step.get();
                view! {
                    <li class=move || format!("mk-wiz-st {}", state_of())>
                        <button
                            type="button"
                            class="mk-wiz-sb"
                            aria-current=move || (state_of() == "on").then_some("step")
                            aria-label=name
                            title=name
                            prop:disabled=move || !can_go()
                            on:click=move |_| go_to(i, if i > step.get_untracked() { 1 } else { -1 })
                        >
                            <b aria-hidden="true">
                                {move || if state_of() == "done" { view! { <Icon name="check" class="h-3 w-3"/> }.into_any() } else { (i + 1).to_string().into_any() }}
                            </b>
                            <span class="mk-wiz-sn">{name}</span>
                        </button>
                    </li>
                }
            })
            .collect_view()
    };

    let section_row = move |i: usize| {
        let section = Section::ALL[i];
        let style = move || {
            let (tops, hs, _) = geo.get();
            format!("top:{}px;height:{}px", tops[i], hs[i])
        };
        let content = move || -> AnyView {
            if !(reached.get()[i + 1] || step.get() == 0) {
                return ().into_any();
            }
            if step.get() == i + 1 && !done.get() && !confirm.get() {
                let list = slides.get();
                let c = ctx();
                list.into_iter()
                    .enumerate()
                    .map(|(j, node)| {
                        let style = move || {
                            let cur = i32::from(pick.with(|p| p.choice[i]));
                            let off = (j as i32 - cur + 1).rem_euclid(3) - 1;
                            let d = dir.get();
                            // La que da la vuelta no cruza el escenario a la vista.
                            let jump = (d > 0 && off == 1) || (d < 0 && off == -1);
                            format!(
                                "transform:translateX({}%);opacity:{};{}",
                                off * 100,
                                if off == 0 { 1 } else { 0 },
                                if jump { "transition:none;" } else { "" }
                            )
                        };
                        let label = format!("{}, {} de 3", WIZARD[i].options[j].name, j + 1);
                        view! {
                            <div
                                class="mk-wiz-slide"
                                role="group"
                                aria-roledescription="slide"
                                aria-label=label
                                aria-hidden=move || (pick.with(|p| usize::from(p.choice[i])) != j).to_string()
                                style=style
                            >
                                {match node {
                                    Some(n) => scaled(&n, &c),
                                    None => view! { <div class="mk-wiz-none">{none_label(section)}</div> }.into_any(),
                                }}
                            </div>
                        }
                    })
                    .collect_view()
                    .into_any()
            } else {
                match nodes.get()[i].clone() {
                    Some(n) => scaled(&n, &ctx()),
                    None => {
                        view! { <div class="mk-wiz-none">{none_label(section)}</div> }.into_any()
                    }
                }
            }
        };
        view! {
            <div
                class="mk-wiz-sec"
                class:is-act=move || !done.get() && !confirm.get() && step.get() == i + 1
                style=format!("--i:{i}")
            >
                <div class="mk-wiz-clip" style=style>
                    {content}
                    <div class="mk-wiz-sk" class:is-off=move || reached.get()[i + 1] || step.get() == 0 aria-hidden="true">
                        {skeleton(section)}
                    </div>
                </div>
            </div>
        }
    };

    let panel = move || -> AnyView {
        let d = dir.get_untracked();
        let style = format!("--dir:{}", if d == 0 { 1 } else { d });
        if done.get() {
            let p = pick.get_untracked();
            return view! {
                <div class="mk-wiz-pin" style=style>
                    <div style="--d:0">
                        <h3 class="mk-wiz-h">"Tu página está lista"</h3>
                        <p class="mk-wiz-sub">"Ya la podés seguir editando bloque por bloque."</p>
                    </div>
                    <ul class="mk-wiz-sum" style="--d:1">
                        {WIZARD.iter().enumerate().map(|(i, s)| {
                            let value = if p.is_empty_choice(s.section) {
                                s.options[usize::from(p.choice[i])].name
                            } else {
                                s.options[usize::from(p.choice[i])].name
                            };
                            view! { <li><span>{s.name}</span><b>{value}</b></li> }
                        }).collect_view()}
                    </ul>
                    <div class="mk-wiz-btns" style="--d:2">
                        <button type="button" class="btn btn-primary" autofocus=true on:click=close>"Abrir en el editor"</button>
                        <button type="button" class="btn" on:click=move |_| {
                            // Vuelve a como estaba y deja elegir de nuevo.
                            state.undo();
                            done.set(false);
                            go_to(0, -1);
                        }>"Cambiar algo"</button>
                    </div>
                    <p class="mk-wiz-more" style="--d:3">"Podés volver desde Mi tienda → Asistente de página. Para deshacer todo: Ctrl+Z."</p>
                </div>
            }
            .into_any();
        }
        if confirm.get() {
            let plan = wizard::plan(&state.doc.get_untracked(), &pick.get_untracked());
            let lines = plan
                .iter()
                .enumerate()
                .filter_map(|(i, c)| c.text().map(|t| (i, *c, t)))
                .collect::<Vec<_>>();
            let kept = plan
                .iter()
                .enumerate()
                .filter(|(_, c)| matches!(c, Change::Kept))
                .map(|(i, _)| WIZARD[i].name)
                .collect::<Vec<_>>()
                .join(", ");
            let p = pick.get_untracked();
            let bg_changes = state
                .doc
                .with_untracked(|d| wizard::background_changes(&d.style, &p));
            return view! {
                <div class="mk-wiz-pin" style=style>
                    <div style="--d:0">
                        <p class="mk-wiz-eb">"Antes de aplicar"</p>
                        <h3 class="mk-wiz-h">"Esto es lo que va a cambiar"</h3>
                    </div>
                    <ul class="mk-wiz-sum" style="--d:1">
                        {lines.into_iter().map(|(i, _, t)| view! {
                            <li><span>{WIZARD[i].name}</span><b>{format!("{t}: {}", WIZARD[i].options[usize::from(p.choice[i])].name)}</b></li>
                        }).collect_view()}
                    </ul>
                    {bg_changes.then(|| view! { <p class="mk-wiz-sub" style="--d:2">"Fondo de la web: se cambia."</p> })}
                    {(!kept.is_empty()).then(|| view! { <p class="mk-wiz-sub" style="--d:2">{format!("Sin cambios: {kept}.")}</p> })}
                    <p class="mk-wiz-more" style="--d:3">"Se conservan tus textos, fotos y enlaces. Después podés deshacerlo con Ctrl+Z."</p>
                </div>
            }
            .into_any();
        }
        let cur = step.get();
        if cur == 0 {
            return background_panel(pick, style, theme);
        }
        let st = cur - 1;
        let s = &WIZARD[st];
        // El valor que tiene hoy un campo del bloque (con lo que ya se tocó).
        let prop = move |key: &'static str| -> Option<Value> {
            nodes.with(|n| {
                n[st]
                    .as_ref()
                    .and_then(|n| serde_json::to_value(&n.widget).ok())
                    .and_then(|v| v.get("props").and_then(|p| p.get(key)).cloned())
            })
        };
        let set_over = move |key: &'static str, v: Value| {
            pick.update(|p| {
                p.over[st].insert(key.to_string(), v);
            });
        };
        let content = content_rows(s.section)
            .iter()
            .map(|(key, label, ctl)| {
                let (key, label, ctl) = (*key, *label, *ctl);
                let id = format!("wz-{st}-{key}");
                let text = Signal::derive(move || {
                    prop(key).and_then(|v| v.as_str().map(str::to_string)).unwrap_or_default()
                });
                let text_max = Signal::derive(move || {
                    nodes.with(|n| {
                        n[st].as_ref().and_then(|n| domain::wizard::text_max(&n.widget, key))
                    })
                    .unwrap_or(200)
                });
                let body = match ctl {
                    Ctl::Text => view! {
                        <TxtRow label=label id=id value=text max=text_max
                            on_pick=Callback::new(move |v: String| set_over(key, Value::String(v)))/>
                    }
                    .into_any(),
                    Ctl::Toggle => view! {
                        <TglRow label=label id=id
                            value=Signal::derive(move || prop(key).and_then(|v| v.as_bool()).unwrap_or(false))
                            on_pick=Callback::new(move |v: bool| set_over(key, Value::Bool(v)))/>
                    }
                    .into_any(),
                    Ctl::Select(opts) => view! {
                        <SelRow label=label id=id
                            options=opts.iter().map(|(v, l)| (v.to_string(), l.to_string())).collect()
                            value=text
                            on_pick=Callback::new(move |v: String| set_over(key, Value::String(v)))/>
                    }
                    .into_any(),
                    Ctl::Slider(min, max, unit) => view! {
                        <SldRow label=label id=id min=min max=max unit=unit ticks=Vec::new()
                            value=Signal::derive(move || prop(key).and_then(|v| v.as_i64()).unwrap_or(min))
                            on_pick=Callback::new(move |v: i64| set_over(key, Value::from(v)))/>
                    }
                    .into_any(),
                    Ctl::Photo => view! {
                        <div class="mk-wiz-row is-col">
                            <span class="mk-wiz-rl">{label}</span>
                            <ImagePicker uid=id value=Signal::derive(move || prop(key).unwrap_or(Value::Null))
                                pick=Callback::new(move |v: Value| set_over(key, v)) on_photo=None/>
                        </div>
                    }
                    .into_any(),
                };
                view! { <div class="mk-wiz-rw" hidden=move || prop(key).is_none()>{body}</div> }
            })
            .collect_view();
        // Altura de la portada: solo con un banner.
        let hero_h = (s.section == Section::Hero).then(|| view! {
            <div class="mk-wiz-rw" hidden=move || prop("title").is_none()>
                <SldRow label="Altura" id="wz-hero-h".to_string() min=0 max=2 unit="" ticks=HERO_HEIGHTS.to_vec()
                    value=Signal::derive(move || pick.with(|p| i64::from(p.hero_height)))
                    on_pick=Callback::new(move |v: i64| pick.update(|p| p.hero_height = v as u8))/>
            </div>
        });
        let tone = (s.section == Section::Nav).then(|| view! {
            <div class="mk-wiz-rw" hidden=move || prop("tone").is_none()>
                <SelRow label="Fondo" id="wz-nav-tone".to_string()
                    options=NAV_TONE_OPTIONS.iter().map(|(v, l)| (v.to_string(), l.to_string())).collect()
                    value=Signal::derive(move || prop("tone").and_then(|v| v.as_str().map(str::to_string)).unwrap_or_default())
                    on_pick=Callback::new(move |v: String| set_over("tone", Value::String(v)))/>
            </div>
        });
        let node_look =
            move || nodes.with(|n| n[st].as_ref().map(|n| (n.look.design, n.style.clone())));
        let design = (s.section != Section::Footer).then(|| view! {
            <SelRow label="Estilo visual" id=format!("wz-{st}-design")
                options=DESIGNS.iter().map(|(n, name, _)| (n.to_string(), name.to_string())).collect()
                value=Signal::derive(move || node_look().map(|(d, _)| d).unwrap_or(1).to_string())
                on_pick=Callback::new(move |v: String| pick.update(|p| p.design[st] = v.parse().unwrap_or(0)))/>
        });
        let swatches = view! {
            <div class="mk-wiz-row">
                <span class="mk-wiz-rl" id=format!("wz-{st}-color-l")>"Color"</span>
                <div class="mk-wiz-sws" data-theme=theme role="radiogroup" aria-labelledby=format!("wz-{st}-color-l")>
                    {WIZARD_COLORS.iter().enumerate().map(|(k, (_, name))| view! {
                        <label title=*name>
                            <input type="radio" name=format!("wz-{st}-color") aria-label=*name
                                prop:checked=move || pick.with(|p| usize::from(p.color[st]) == k)
                                on:change=move |_| pick.update(|p| p.color[st] = k as u8)/>
                            <i style=format!("background:var(--color-{})", COLOR_VARS[k])></i>
                        </label>
                    }).collect_view()}
                </div>
            </div>
        };
        let font = matches!(s.section, Section::Nav | Section::Filters | Section::Footer).then(|| {
            let mut opts = vec![(String::new(), "La de tu tienda".to_string())];
            opts.extend(FONTS.iter().map(|f| (f.id.to_string(), f.label.to_string())));
            view! {
                <SelRow label="Letra" id=format!("wz-{st}-font") options=opts
                    value=Signal::derive(move || {
                        let own = pick.with(|p| p.font[st].clone());
                        if own.is_empty() { node_look().map(|(_, s)| s.font).unwrap_or_default() } else { own }
                    })
                    on_pick=Callback::new(move |v: String| pick.update(|p| p.font[st] = v))/>
            }
        });
        let align = matches!(s.section, Section::Hero | Section::Filters).then(|| view! {
            <SelRow label="Alineación" id=format!("wz-{st}-align")
                options=ALIGN_OPTIONS.iter().map(|(v, l)| (v.to_string(), l.to_string())).collect()
                value=Signal::derive(move || {
                    let own = pick.with(|p| p.align[st].clone());
                    if own.is_empty() { node_look().map(|(_, s)| s.align).unwrap_or_default() } else { own }
                })
                on_pick=Callback::new(move |v: String| pick.update(|p| p.align[st] = v))/>
        });
        let anim = (s.section == Section::Hero).then(|| {
            let mut opts = vec![(String::new(), "Ninguna".to_string())];
            opts.extend(TEXT_ANIMATIONS.iter().map(|(v, l, _)| (v.to_string(), l.to_string())));
            view! {
                <SelRow label="Animación del texto" id=format!("wz-{st}-anim") options=opts
                    value=Signal::derive(move || {
                        let own = pick.with(|p| p.anim[st].clone());
                        if own.is_empty() { node_look().map(|(_, s)| s.animation).unwrap_or_default() } else { own }
                    })
                    on_pick=Callback::new(move |v: String| pick.update(|p| p.anim[st] = v))/>
            }
        });
        view! {
            <div class="mk-wiz-pin" style=style>
                <div style="--d:0">
                    <h3 class="mk-wiz-h">{s.name}</h3>
                    <p class="mk-wiz-sub">{move || WIZARD[st].options[usize::from(pick.with(|p| p.choice[st]))].desc}</p>
                </div>
                <Group name="Contenido" d=1>{content}{hero_h}</Group>
                <Group name="Aspecto" d=2>{tone}{design}{swatches}{font}{align}{anim}</Group>
            </div>
        }
        .into_any()
    };

    let go_label = move || {
        if confirm.get() {
            "Aplicar cambios"
        } else if step.get() + 1 == UI_STEPS {
            "Usar este y crear mi página"
        } else {
            "Usar este y seguir"
        }
    };
    let coach_show =
        move || coach.get() && step.get() > 0 && !intro.get() && !done.get() && !confirm.get();
    let option_name = move || {
        let st = step.get().saturating_sub(1);
        let c = usize::from(pick.with(|p| p.choice[st]));
        (WIZARD[st].options[c].name, c + 1)
    };

    view! {
        <dialog
            node_ref=dialog
            class="mk-wiz"
            aria-labelledby="mk-wiz-title"
            on:close=move |_| ui.wizard.set(false)
            on:keydown=move |ev| {
                let typing = ev.target()
                    .and_then(|t| wasm_bindgen::JsCast::dyn_into::<web_sys::HtmlElement>(t).ok())
                    .is_some_and(|el| matches!(el.tag_name().as_str(), "INPUT" | "TEXTAREA" | "SELECT"));
                if typing {
                    return;
                }
                match ev.key().as_str() {
                    "ArrowLeft" => { ev.prevent_default(); shift_choice(-1); }
                    "ArrowRight" => { ev.prevent_default(); shift_choice(1); }
                    _ => {}
                }
            }
        >
            <header class="mk-wiz-hd">
                <h2 id="mk-wiz-title">"Armá tu primera página"</h2>
                <ol class="mk-wiz-steps" aria-label="Pasos">{steps_bar}</ol>
                <button type="button" class="mk-wiz-x" aria-label="Cerrar asistente" title="Cerrar (Esc). Lo elegido queda guardado." on:click=close>
                    <Icon name="close" class="h-4 w-4"/>
                </button>
            </header>
            <div class="mk-wiz-main">
                <div
                    class="mk-wiz-stagewrap"
                    class:is-done=move || done.get() || confirm.get() || step.get() == 0
                    role="group"
                    aria-roledescription="carousel"
                    aria-labelledby="mk-wiz-title"
                >
                    <div class="mk-wiz-bg" data-theme=theme hidden=move || pick.with(|p| p.bg_kind == 0) style=bg_style></div>
                    <div class="mk-wiz-stage">
                        <div class="mk-wiz-camera" style=camera>
                            <div data-theme=theme class="mk-store mk-wiz-page bg-base-100 text-base-content" style=store_vars>
                                {(0..WIZARD_STEPS).map(section_row).collect_view()}
                                <div style=move || format!("height:{}px", geo.get().2)></div>
                            </div>
                        </div>
                    </div>
                    <button type="button" class="mk-wiz-arrow is-l" hidden=move || step.get() == 0 class:is-hint=coach_show on:click=move |_| shift_choice(-1)
                        aria-label=move || "Opción anterior"
                    ><Icon name="chevron-left" class="h-5 w-5"/></button>
                    <button type="button" class="mk-wiz-arrow is-r" hidden=move || step.get() == 0 class:is-hint=coach_show on:click=move |_| shift_choice(1)
                        aria-label=move || "Opción siguiente"
                    ><Icon name="chevron-right" class="h-5 w-5"/></button>
                    <div class="mk-wiz-coach" hidden=move || !coach_show() role="note">"Probá las flechas para ver más opciones de este bloque."</div>
                    <div class="mk-wiz-dots" hidden=move || step.get() == 0>
                        <p class="mk-wiz-nm">{move || option_name().0}<small>{move || format!("Opción {} de 3", option_name().1)}</small></p>
                        <div class="mk-wiz-dotrow">
                            {(0..3usize).map(|j| view! {
                                <button
                                    type="button"
                                    class="mk-wiz-dot"
                                    aria-label=move || WIZARD[step.get().saturating_sub(1)].options[j].name
                                    aria-current=move || (option_name().1 == j + 1).then_some("true")
                                    on:click=move |_| pick_choice(j)
                                ></button>
                            }).collect_view()}
                        </div>
                    </div>
                </div>
                <div class="mk-wiz-panel">{move || panel()}</div>
            </div>
            <footer class="mk-wiz-ft" hidden=move || done.get() || intro.get()>
                <div class="mk-wiz-btns">
                    <button type="button" class="btn btn-ghost" hidden=move || done.get() || intro.get() || (step.get() == 0 && !confirm.get()) on:click=back>
                        {move || if confirm.get() { "Volver" } else { "Atrás" }}
                    </button>
                    <button type="button" class="btn btn-primary" hidden=move || done.get() || intro.get() on:click=next>{go_label}</button>
                </div>
            </footer>
            <div class="mk-wiz-intro" class:is-off=move || !intro.get() role="group" aria-labelledby="mk-wiz-ih">
                <div class="mk-wiz-card">
                    <h3 id="mk-wiz-ih">"Vamos a armar tu página en 6 pasos"</h3>
                    <p>"Es rápido. Después podés cambiar todo desde el editor."</p>
                    <ol>
                        <li style="--k:0"><b>"1"</b><span>"En cada paso mirá las opciones con las flechas ‹ ›."</span></li>
                        <li style="--k:1"><b>"2"</b><span>"Elegí la que más te guste y ajustala a la derecha si querés."</span></li>
                        <li style="--k:2"><b>"3"</b><span>"Tocá "<strong>"Usar este y seguir"</strong>" para pasar al paso siguiente."</span></li>
                    </ol>
                    <button type="button" class="btn btn-primary btn-lg" on:click=move |_| intro.set(false)>"Empezar"</button>
                </div>
            </div>
            <p class="sr-only" role="status">{move || live.get()}</p>
        </dialog>
    }
}
