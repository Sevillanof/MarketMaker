//! Asistente de primera página: una ventana sobre el editor donde se arma la
//! página en cinco pasos (encabezado, portada, filtros, productos y pie).
//!
//! En el centro está la página de verdad, que se va armando; las flechas
//! cambian de opción con un deslizamiento. A la derecha están los ajustes
//! simples del paso. Lo que se elige no toca la página hasta el final, y se
//! aplica como un solo paso de deshacer. La lógica de qué bloque sale de cada
//! opción está en `domain::wizard`.

use domain::wizard::{
    self, Change, Section, WizardPick, HERO_HEIGHTS, NAV_TONES, PRODUCT_COUNTS, WIZARD,
    WIZARD_COLORS, WIZARD_STEPS,
};
use domain::{Layer, Node, Product, Widget, ROW_PX};
use leptos::prelude::*;
use widgets::{render_widget, sample_product, RenderCtx};

use crate::icons::Icon;
use crate::{storage, Actions, EditorState, Ui};

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

/// Selector de dos o más opciones en una fila (radios de verdad, con flechas).
#[component]
fn Seg(
    name: &'static str,
    label: &'static str,
    options: Vec<String>,
    value: Signal<usize>,
    on_pick: Callback<usize>,
) -> impl IntoView {
    let label_id = format!("mk-wiz-l-{name}");
    view! {
        <div>
            <p class="mk-wiz-lb" id=label_id.clone()>{label}</p>
            <div class="mk-wiz-seg" role="radiogroup" aria-labelledby=label_id>
                {options.into_iter().enumerate().map(|(i, text)| view! {
                    <label>
                        <input
                            type="radio"
                            name=name
                            prop:checked=move || value.get() == i
                            on:change=move |_| on_pick.run(i)
                        />
                        <span>{text}</span>
                    </label>
                }).collect_view()}
            </div>
        </div>
    }
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
    let reached = RwSignal::new([true, false, false, false, false]);
    // Hacia dónde se va (+1 adelante, -1 atrás): decide de qué lado entra lo nuevo.
    let dir = RwSignal::new(0i32);
    // Las flechas laten hasta que se usan por primera vez.
    let coach = RwSignal::new(true);
    let live = RwSignal::new(String::new());
    let base = StoredValue::new(state.doc.get_untracked());
    let untouched = StoredValue::new(false);
    let auto = StoredValue::new(false);

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
            Some((p, s)) => (p.clamped(), s.min(WIZARD_STEPS - 1)),
            None => (wizard::pick_from_doc(&doc), 0),
        };
        pick.set(p);
        step.set(s);
        reached.set(if is_new {
            let mut r = [false; WIZARD_STEPS];
            r.iter_mut().take(s + 1).for_each(|x| *x = true);
            r
        } else {
            [true; WIZARD_STEPS]
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
        live.set(format!("Asistente abierto. Paso {} de {}.", s + 1, WIZARD_STEPS));
    };

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
        let st = step.get();
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
            let h = if r[i] {
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
        if done.get() || confirm.get() {
            let k = (520.0 / total).min(1.0);
            format!("width:{PAGE_W}px; transform: translate(-50%, 14px) scale({k})")
        } else {
            let st = step.get();
            let c = tops[st] + hs[st] / 2.0;
            format!(
                "width:{PAGE_W}px; transform: translate(-50%, min(96px, calc(50cqh - {c}px + 18px)))"
            )
        }
    };
    let theme = move || state.doc.with(|d| d.theme.clone());
    let store_vars = move || state.doc.with(|d| d.typography.css_vars());

    // --- Acciones ---
    let pick_choice = move |j: usize| {
        let st = step.get_untracked();
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
        let st = step.get_untracked();
        let n = WIZARD[st].options.len() as i32;
        let cur = i32::from(pick.get_untracked().choice[st]);
        pick_choice(((cur + d).rem_euclid(n)) as usize);
    };
    let go_to = move |n: usize, d: i32| {
        dir.set(d);
        step.set(n);
        reached.update(|r| r[n] = true);
        confirm.set(false);
        live.set(format!("Paso {} de {}: {}", n + 1, WIZARD_STEPS, WIZARD[n].name));
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
        actions.ui.announce("Armamos tu página. Podés deshacerlo con Ctrl y Z.");
    };
    let next = move |_| {
        if confirm.get_untracked() {
            apply_now();
        } else if step.get_untracked() + 1 < WIZARD_STEPS {
            go_to(step.get_untracked() + 1, 1);
        } else if untouched.get_value() {
            apply_now();
        } else {
            reached.set([true; WIZARD_STEPS]);
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
        WIZARD
            .iter()
            .enumerate()
            .map(|(i, s)| {
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
                            prop:disabled=move || !can_go()
                            on:click=move |_| go_to(i, if i > step.get_untracked() { 1 } else { -1 })
                        >
                            <b aria-hidden="true">
                                {move || if state_of() == "done" { view! { <Icon name="check" class="h-3 w-3"/> }.into_any() } else { (i + 1).to_string().into_any() }}
                            </b>
                            <span class="mk-wiz-sn">{s.name}</span>
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
            if !reached.get()[i] {
                return ().into_any();
            }
            if step.get() == i && !done.get() && !confirm.get() {
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
                    None => view! { <div class="mk-wiz-none">{none_label(section)}</div> }.into_any(),
                }
            }
        };
        view! {
            <div
                class="mk-wiz-sec"
                class:is-act=move || !done.get() && !confirm.get() && step.get() == i
                style=format!("--i:{i}")
            >
                <div class="mk-wiz-clip" style=style>
                    {content}
                    <div class="mk-wiz-sk" class:is-off=move || reached.get()[i] aria-hidden="true">
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
                    {(!kept.is_empty()).then(|| view! { <p class="mk-wiz-sub" style="--d:2">{format!("Sin cambios: {kept}.")}</p> })}
                    <p class="mk-wiz-more" style="--d:3">"Se conservan tus textos, fotos y enlaces. Después podés deshacerlo con Ctrl+Z."</p>
                </div>
            }
            .into_any();
        }
        let st = step.get();
        let s = &WIZARD[st];
        let radios = s
            .options
            .iter()
            .enumerate()
            .map(|(j, o)| {
                view! {
                    <label class="mk-wiz-opt">
                        <input
                            type="radio"
                            name="mk-wiz-est"
                            prop:checked=move || pick.with(|p| usize::from(p.choice[st])) == j
                            on:change=move |_| pick_choice(j)
                        />
                        <span class="mk-wiz-rd" aria-hidden="true">
                            <Icon name="check" class="h-3 w-3"/>
                        </span>
                        <span><b>{o.name}</b><small>{o.desc}</small></span>
                    </label>
                }
            })
            .collect_view();
        let colors = Seg(SegProps {
            name: "color",
            label: "Color del bloque",
            options: WIZARD_COLORS.iter().map(|(_, n)| n.to_string()).collect(),
            value: Signal::derive(move || pick.with(|p| usize::from(p.color[st]))),
            on_pick: Callback::new(move |i| pick.update(|p| p.color[st] = i as u8)),
        });
        let extra = match s.section {
            Section::Nav => view! {
                <div style="--d:3">
                    {Seg(SegProps {
                        name: "tone",
                        label: "Fondo",
                        options: NAV_TONES.iter().map(|(_, n)| n.to_string()).collect(),
                        value: Signal::derive(move || pick.with(|p| usize::from(p.nav_tone))),
                        on_pick: Callback::new(move |i| pick.update(|p| p.nav_tone = i as u8)),
                    })}
                </div>
                <div class="mk-wiz-tg" style="--d:4">
                    <span id="mk-wiz-l-search">"Buscador de productos"</span>
                    <button
                        type="button"
                        class="mk-wiz-sw"
                        role="switch"
                        aria-labelledby="mk-wiz-l-search"
                        aria-checked=move || pick.with(|p| p.nav_search).to_string()
                        on:click=move |_| pick.update(|p| p.nav_search = !p.nav_search)
                    ></button>
                </div>
            }
            .into_any(),
            Section::Hero => view! {
                <div style="--d:3">
                    {Seg(SegProps {
                        name: "height",
                        label: "Altura",
                        options: HERO_HEIGHTS.iter().map(|n| n.to_string()).collect(),
                        value: Signal::derive(move || pick.with(|p| usize::from(p.hero_height))),
                        on_pick: Callback::new(move |i| pick.update(|p| p.hero_height = i as u8)),
                    })}
                </div>
            }
            .into_any(),
            Section::Products => view! {
                <div style="--d:3">
                    {Seg(SegProps {
                        name: "count",
                        label: "Cuántos productos",
                        options: PRODUCT_COUNTS.iter().map(|(n, _)| n.to_string()).collect(),
                        value: Signal::derive(move || pick.with(|p| usize::from(p.products_count))),
                        on_pick: Callback::new(move |i| pick.update(|p| p.products_count = i as u8)),
                    })}
                </div>
            }
            .into_any(),
            _ => ().into_any(),
        };
        view! {
            <div class="mk-wiz-pin" style=style>
                <div style="--d:0">
                    <p class="mk-wiz-eb">{format!("Paso {} de {}", st + 1, WIZARD_STEPS)}</p>
                    <h3 class="mk-wiz-h">{s.name}</h3>
                </div>
                <div style="--d:1">
                    <p class="mk-wiz-lb" id="mk-wiz-l-est">"Elegí una opción o usá las flechas"</p>
                    <fieldset class="mk-wiz-opts" aria-labelledby="mk-wiz-l-est">{radios}</fieldset>
                </div>
                <div class="mk-wiz-sep" style="--d:2">
                    <p class="mk-wiz-h2">"Personalizalo"</p>
                    <p class="mk-wiz-sub">"Es opcional. Los cambios se ven al instante."</p>
                </div>
                {extra}
                <div style="--d:5">{colors}</div>
                <p class="mk-wiz-more" style="--d:6">"Hay más ajustes de cada bloque en el editor."</p>
            </div>
        }
        .into_any()
    };

    let go_label = move || {
        if confirm.get() {
            "Aplicar cambios"
        } else if step.get() + 1 == WIZARD_STEPS {
            "Usar este y crear mi página"
        } else {
            "Usar este y seguir"
        }
    };
    let coach_show = move || coach.get() && !intro.get() && !done.get() && !confirm.get();
    let option_name = move || {
        let st = step.get();
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
                    class:is-done=move || done.get() || confirm.get()
                    role="group"
                    aria-roledescription="carousel"
                    aria-labelledby="mk-wiz-title"
                >
                    <div class="mk-wiz-say" aria-hidden=move || (done.get() || confirm.get()).to_string()>
                        <h3>{move || WIZARD[step.get()].title}</h3>
                        <p>{move || WIZARD[step.get()].hint}</p>
                    </div>
                    <div class="mk-wiz-stage">
                        <div class="mk-wiz-camera" style=camera>
                            <div data-theme=theme class="mk-store mk-wiz-page bg-base-100 text-base-content" style=store_vars>
                                {(0..WIZARD_STEPS).map(section_row).collect_view()}
                                <div style=move || format!("height:{}px", geo.get().2)></div>
                            </div>
                        </div>
                    </div>
                    <button type="button" class="mk-wiz-arrow is-l" class:is-hint=coach_show on:click=move |_| shift_choice(-1)
                        aria-label=move || "Opción anterior"
                    ><Icon name="chevron-left" class="h-5 w-5"/></button>
                    <button type="button" class="mk-wiz-arrow is-r" class:is-hint=coach_show on:click=move |_| shift_choice(1)
                        aria-label=move || "Opción siguiente"
                    ><Icon name="chevron-right" class="h-5 w-5"/></button>
                    <span class="mk-wiz-alab is-l" aria-hidden="true">"Anterior"</span>
                    <span class="mk-wiz-alab is-r" aria-hidden="true">"Otra opción"</span>
                    <div class="mk-wiz-coach" hidden=move || !coach_show() role="note">"Probá las flechas para ver más opciones de este bloque."</div>
                    <div class="mk-wiz-dots">
                        <p class="mk-wiz-nm">{move || option_name().0}<small>{move || format!("Opción {} de 3", option_name().1)}</small></p>
                        <div class="mk-wiz-dotrow">
                            {(0..3usize).map(|j| view! {
                                <button
                                    type="button"
                                    class="mk-wiz-dot"
                                    aria-label=move || WIZARD[step.get()].options[j].name
                                    aria-current=move || (option_name().1 == j + 1).then_some("true")
                                    on:click=move |_| pick_choice(j)
                                ></button>
                            }).collect_view()}
                        </div>
                    </div>
                </div>
                <div class="mk-wiz-panel">{move || panel()}</div>
            </div>
            <footer class="mk-wiz-ft">
                <span>{move || if done.get() { "Podés deshacer todo con Ctrl+Z.".to_string() } else if intro.get() { String::new() } else { format!("Paso {} de {} · Todo se puede cambiar después en el editor.", step.get() + 1, WIZARD_STEPS) }}</span>
                <div class="mk-wiz-btns">
                    <button type="button" class="btn" hidden=move || done.get() || intro.get() || (step.get() == 0 && !confirm.get()) on:click=back>
                        {move || if confirm.get() { "Volver" } else { "Atrás" }}
                    </button>
                    <button type="button" class="btn btn-primary" hidden=move || done.get() || intro.get() on:click=next>{go_label}</button>
                </div>
            </footer>
            <div class="mk-wiz-intro" class:is-off=move || !intro.get() role="group" aria-labelledby="mk-wiz-ih">
                <div class="mk-wiz-card">
                    <h3 id="mk-wiz-ih">"Vamos a armar tu página en 5 pasos"</h3>
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
