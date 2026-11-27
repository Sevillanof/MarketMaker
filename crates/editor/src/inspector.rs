//! Inspector: el panel de la derecha con los campos del bloque elegido.
//!
//! Una sola pestaña con secciones plegables: "Contenido" (textos, imágenes,
//! listas y el botón), "Estilo y color", "Tamaño", "Letras" y, si el bloque
//! las tiene, "Más opciones". Se abre una a la vez y cada una cerrada muestra
//! su valor actual. Al elegir un bloque se abre Contenido, que es lo que la
//! persona viene a cambiar casi siempre.

use domain::{Field, Product, Widget, DESIGNS, GRID_COLS, ROW_PX};
use leptos::prelude::*;

use crate::fields::{ButtonGroup, PropField};
use crate::icons::Icon;
use crate::look::{swatch_class, BlockLook};
use crate::page_map::{PageMap, Shortcuts};
use crate::palette::block_info;
use crate::typography::BlockTypography;
use crate::{size_badge, Actions, EditorState, Ui, WIDTHS};


/// Alto en píxeles de `h` filas.
fn rows_to_px(h: u8) -> u32 {
    u32::from(h) * u32::from(ROW_PX)
}

/// Filas más cercanas a `px` píxeles.
fn px_to_rows(px: f64) -> i32 {
    (px / f64::from(ROW_PX)).round() as i32
}

/// Campos de un bloque repartidos para el panel.
pub(crate) struct FieldGroups {
    /// A la vista en Contenido.
    pub(crate) basic: Vec<&'static Field>,
    /// Texto y enlace del botón, si el bloque tiene uno.
    button: Option<(&'static Field, &'static Field)>,
    /// Plegados en "Más opciones".
    advanced: Vec<&'static Field>,
}

pub(crate) fn group_fields(fields: &'static [Field]) -> FieldGroups {
    let find = |key: &str| fields.iter().find(|f| f.key == key);
    let button = find("button_label").zip(find("button_href"));
    let is_button = |f: &Field| button.is_some() && matches!(f.key, "button_label" | "button_href");
    FieldGroups {
        basic: fields
            .iter()
            .filter(|f| !f.advanced && !is_button(f))
            .collect(),
        button,
        advanced: fields
            .iter()
            .filter(|f| f.advanced && !is_button(f))
            .collect(),
    }
}

#[component]
pub(crate) fn Inspector(products: StoredValue<Vec<Product>>) -> impl IntoView {
    let state = expect_context::<EditorState>();
    let ui = expect_context::<Ui>();
    // Se redibuja solo cuando cambia el nodo seleccionado (o deja de existir),
    // no con cada tecla, para no perder el foco de los campos.
    let current = Memo::new(move |_| {
        state.selected.get().filter(|id| {
            let layer = state.layer.get();
            state.doc.with(|d| d.layer_of(id)) == Some(layer)
        })
    });
    // Cada bloque nuevo que se elige empieza en el mapa, con sus ajustes
    // rápidos; los avanzados se piden aparte.
    Effect::new(move |prev: Option<Option<String>>| {
        let now = state.selected.get();
        if prev.is_some_and(|p| p != now) {
            ui.advanced.set(false);
        }
        now
    });
    // Al guardar con errores, los campos a corregir están en los avanzados.
    Effect::new(move |_| {
        if ui.show_errors.get() && current.get_untracked().is_some() {
            ui.advanced.set(true);
        }
    });
    // Los avanzados se ven con un bloque elegido; en la ficha no hay mapa.
    let detail = move || current.get().is_some() && (state.in_modal() || ui.advanced.get());

    view! {
        <aside class="w-[23rem] shrink-0 overflow-y-auto border-l border-base-300 bg-base-100 p-4 pb-24" aria-label="Propiedades del bloque">
            // El mapa queda armado aunque se vean los avanzados: así no pierde
            // las páginas desplegadas ni el lugar donde estabas.
            <div class:hidden=move || state.in_modal() || detail()>
                <PageMap products/>
            </div>
            {move || match current.get() {
                _ if !detail() => state.in_modal().then(|| view! { <EmptyInspector/> }).into_any(),
                None => ().into_any(),
                Some(id) => match state.node(&id) {
                    Some(node) => view! {
                        {(!state.in_modal()).then(|| view! {
                            <button type="button" class="btn btn-ghost btn-sm mb-3 -ml-2 gap-1.5" on:click=move |_| {
                                ui.advanced.set(false);
                                request_animation_frame(|| crate::focus_selector("#mk-inspector-title"));
                            }>
                                <Icon name="arrow-left" class="h-4 w-4"/>"Tu tienda"
                            </button>
                        })}
                        <NodeInspector products id widget=node.widget/>
                    }.into_any(),
                    None => ().into_any(),
                },
            }}
        </aside>
    }
}

#[component]
fn EmptyInspector() -> impl IntoView {
    let state = expect_context::<EditorState>();
    let actions = expect_context::<Actions>();
    view! {
        <div class="flex flex-col gap-4">
            <h2 id="mk-inspector-title" tabindex="-1" class="text-lg font-bold">"Elegí un bloque para cambiarlo"</h2>
            {move || if state.in_modal() {
                view! {
                    <p class="text-sm">"Estás en la ficha del producto. Hacé clic en uno de sus bloques para cambiarlo aquí, o sumá bloques desde la izquierda."</p>
                    <ModalSettings/>
                    <button type="button" class="btn btn-sm self-start" on:click=move |_| actions.close_modal()>"Volver a la página"</button>
                }.into_any()
            } else {
                view! {
                    <p class="text-sm">"Hacé clic en un bloque de tu tienda y acá aparecen sus textos, imágenes y colores."</p>
                }.into_any()
            }}
            <p class="rounded-box bg-base-200 p-3 text-sm">"Consejo: arrastrá un bloque para moverlo, y tirá de su borde para cambiar su tamaño."</p>
            <Shortcuts/>
        </div>
    }
}

/// Ajustes de toda la ficha: cómo se abre y la barra de compra al bajar.
#[component]
fn ModalSettings() -> impl IntoView {
    let state = expect_context::<EditorState>();
    let select = move |id: &'static str,
                       label: &'static str,
                       help: &'static str,
                       options: &'static [(&'static str, &'static str)],
                       get: fn(&domain::ProductModal) -> &String,
                       set: fn(&mut domain::ProductModal, String)| {
        view! {
            <fieldset class="fieldset py-0">
                <label class="fieldset-legend" for=id>{label}</label>
                <select id=id class="select select-sm w-full"
                    on:change=move |ev| {
                        let v = event_target_value(&ev);
                        state.apply(|d| set(&mut d.product_modal, v));
                    }>
                    {options.iter().map(|(v, l)| view! {
                        <option value=*v selected=move || state.doc.with(|d| get(&d.product_modal) == v)>{*l}</option>
                    }).collect_view()}
                </select>
                <p class="label whitespace-normal">{help}</p>
            </fieldset>
        }
    };
    view! {
        <section class="flex flex-col gap-3 rounded-box border border-base-300 p-3" aria-labelledby="mk-modal-settings">
            <h3 id="mk-modal-settings" class="text-sm font-bold">"Toda la ficha"</h3>
            <crate::pictures::PicturePicker kind=crate::pictures::Pictures::ModalOpen
                uid="mk-modal-open".to_string() legend="Cómo se abre" options=domain::MODAL_OPEN_OPTIONS
                help="Así se abre la ficha cuando tu cliente toca un producto."
                value=Signal::derive(move || state.doc.with(|d| d.product_modal.open.clone()))
                pick=Callback::new(move |v: String| state.apply(|d| d.product_modal.open = v.clone()))/>
            {select("mk-modal-share", "Botón Compartir", "Para que tus clientes manden el producto por WhatsApp o copien su enlace.",
                domain::MODAL_SHARE_OPTIONS, |m| &m.share, |m, v| m.share = v)}
        </section>
    }
}

/// Secciones del panel del bloque.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Section {
    Content,
    Look,
    Size,
    Letters,
    More,
}

impl Section {
    fn key(self) -> &'static str {
        match self {
            Section::Content => "content",
            Section::Look => "look",
            Section::Size => "size",
            Section::Letters => "letters",
            Section::More => "more",
        }
    }
}

/// Sección plegable del panel: título con ícono y ayuda (ⓘ), a la derecha el
/// valor actual cuando está cerrada. Solo hay una abierta a la vez. El
/// contenido queda armado aunque esté cerrado, para no perder lo escrito.
#[component]
fn PanelSection(
    which: Section,
    open: RwSignal<Option<Section>>,
    title: &'static str,
    icon: &'static str,
    /// Ayuda en un ⓘ junto al título.
    #[prop(optional)]
    help: Option<&'static str>,
    /// Valor actual, a la derecha del título.
    #[prop(optional, into)]
    summary: ViewFn,
    children: Children,
) -> impl IntoView {
    let is_open = move || open.get() == Some(which);
    let key = which.key();
    let (btn_id, body_id) = (format!("mk-sec-{key}"), format!("mk-sec-body-{key}"));
    let toggle = move |_| open.set(if is_open() { None } else { Some(which) });
    view! {
        <section
            class="rounded-box border border-base-300 bg-base-100"
            class=("border-primary", is_open)
        >
            <div class="flex items-center gap-1 pr-3">
                <button
                    type="button"
                    id=btn_id.clone()
                    class="flex min-w-0 items-center gap-2 rounded-box py-2.5 pl-3 pr-1 text-left text-sm font-semibold"
                    aria-expanded=move || is_open().to_string()
                    aria-controls=body_id.clone()
                    on:click=toggle
                >
                    <Icon name=icon class="h-4 w-4 shrink-0 text-base-content/70"/>
                    {title}
                </button>
                {help.map(|text| view! { <ui::HelpTip id=format!("tip-sec-{key}") label=title text=text/> })}
                <button
                    type="button"
                    tabindex="-1"
                    aria-hidden="true"
                    class="flex min-w-0 flex-1 items-center justify-end gap-2 py-2.5 text-xs font-normal text-base-content/75"
                    on:click=toggle
                >
                    <span class="flex min-w-0 items-center gap-1.5 truncate" class:hidden=is_open>{move || summary.run()}</span>
                    <span class="shrink-0 transition-transform" class=("rotate-180", is_open)><Icon name="chevron-down" class="h-4 w-4"/></span>
                </button>
            </div>
            <div
                id=body_id
                role="region"
                aria-labelledby=btn_id
                class="flex flex-col gap-5 border-t border-base-300 p-3"
                class:hidden=move || !is_open()
            >
                {children()}
            </div>
        </section>
    }
}

#[component]
fn NodeInspector(products: StoredValue<Vec<Product>>, id: String, widget: Widget) -> impl IntoView {
    let ui = expect_context::<Ui>();
    let actions = expect_context::<Actions>();
    let label = widget.label();
    let (icon, description) = block_info(widget.type_name());
    let sid = StoredValue::new(id.clone());
    let (size_sid, look_id, type_id) = (id.clone(), id.clone(), id.clone());
    let widget_size = widget.clone();
    let FieldGroups {
        basic,
        button,
        advanced,
    } = group_fields(widget.fields());
    let has_advanced = !advanced.is_empty();
    let no_fields = basic.is_empty() && button.is_none() && !has_advanced;
    let opens_modal = matches!(
        widget,
        Widget::ProductGrid(_) | Widget::TagTabs(_) | Widget::FeaturedProduct(_)
    );
    let (basic_id, advanced_id, button_id) = (id.clone(), id.clone(), id.clone());
    // Filtros que mueven la cuadrícula de la página (no la llevan adentro).
    let filters_grid = matches!(
        widget.type_name(),
        "TagFilter"
            | "TagList"
            | "TagCircles"
            | "TagSearch"
            | "CategoryTabs"
            | "CategorySide"
            | "CategoryCards"
            | "CategoryDrop"
            | "CategoryPage"
    );

    let state = expect_context::<EditorState>();
    // Con bloques sin nada para escribir se abre directo el estilo.
    let open = RwSignal::new(Some(if no_fields { Section::Look } else { Section::Content }));
    // Al intentar guardar con errores, los campos están en Contenido.
    Effect::new(move |_| {
        if ui.show_errors.get() {
            open.set(Some(Section::Content));
        }
    });
    let node_look = Memo::new(move |_| {
        state.doc.with(|d| sid.with_value(|id| d.node(id).map(|n| (n.look.clone(), n.size, n.style.clone()))))
    });
    let look_summary = move || {
        node_look.with(|n| {
            let look = n.as_ref().map(|(l, _, _)| l.clone()).unwrap_or_default();
            let name = DESIGNS.iter().find(|(d, _, _)| *d == look.design).map_or("Clásico", |(_, n, _)| *n);
            let dot = if look.color.starts_with('#') {
                format!("background:{}", look.color)
            } else {
                String::new()
            };
            let class = if look.color.starts_with('#') { String::new() } else { swatch_class(&look.color).to_string() };
            view! {
                <span class="truncate">{name}</span>
                <span data-theme=move || state.doc.with(|d| d.theme.clone()) class=format!("{class} h-3.5 w-3.5 shrink-0 rounded-full ring-1 ring-base-content/25") style=dot aria-hidden="true"></span>
            }
        })
    };
    let size_summary = move || {
        node_look.with(|n| {
            n.as_ref().map_or_else(String::new, |(_, size, _)| size_badge(*size))
        })
    };
    let letters_summary = move || {
        node_look.with(|n| match n {
            Some((_, _, style)) if !style.is_default() => "Personalizadas",
            _ => "Igual que la tienda",
        })
    };

    view! {
        <div class="flex flex-col gap-4">
            <div class="flex items-start gap-3">
                <span class="rounded-field bg-primary/10 p-2 text-primary" aria-hidden="true"><Icon name=icon class="h-5 w-5"/></span>
                <div class="min-w-0 flex-1">
                    <h2 id="mk-inspector-title" tabindex="-1" class="text-lg font-bold leading-tight">{label}</h2>
                    <p class="text-sm text-base-content/75">{description}</p>
                </div>
            </div>
            <div class="-mt-1 flex gap-2" role="group" aria-label="Acciones del bloque">
                <button
                    type="button"
                    class="btn btn-outline btn-sm gap-1.5 border-base-300"
                    aria-label=format!("Duplicar {}", label.to_lowercase())
                    on:click=move |_| sid.with_value(|id| actions.duplicate(id))
                ><Icon name="copy"/>"Duplicar"</button>
                <button
                    type="button"
                    class="btn btn-outline btn-sm gap-1.5 border-base-300 text-red-700 hover:border-red-700 hover:bg-red-700 hover:text-white"
                    aria-label=format!("Borrar {}", label.to_lowercase())
                    title="Borrar (Suprimir)"
                    on:click=move |_| sid.with_value(|id| actions.delete(id))
                ><Icon name="trash"/>"Borrar"</button>
            </div>

            {opens_modal.then(|| view! {
                <section class="flex flex-col gap-2 rounded-box border border-base-300 bg-primary/5 p-3" aria-labelledby="mk-modal-help-title">
                    <h3 id="mk-modal-help-title" class="text-sm font-semibold">"Ficha del producto"</h3>
                    <p class="text-sm text-base-content/75">"Es la ventana que ven tus clientes al tocar un producto. Es la misma para todos tus productos."</p>
                    <button type="button" class="btn btn-outline btn-sm self-start" on:click=move |_| actions.open_modal(None)>
                        <Icon name="featured"/>
                        "Diseñar la ficha"
                    </button>
                </section>
            })}

            {filters_grid.then(|| view! {
                <p class="flex items-start gap-2 rounded-box bg-base-200 px-3 py-2 text-xs text-base-content/80">
                    <Icon name="grid" class="mt-px h-4 w-4 shrink-0"/>
                    "Este filtro actúa sobre la Cuadrícula de productos de esta página. Si está lejos, la pantalla baja sola hasta los productos."
                </p>
            })}

            <PanelSection which=Section::Content open title="Contenido" icon="pencil">
                {basic
                    .into_iter()
                    .map(|f| view! { <ShownField products id=basic_id.clone() field=f/> })
                    .collect_view()}
                {button.map(|(label_field, href_field)| view! {
                    <ButtonGroup products id=button_id.clone() label_field href_field/>
                })}
                {no_fields.then(|| view! {
                    <p class="text-sm text-base-content/75">"Este bloque muestra el dato del producto que se abre, así que no hay nada para escribir. Su estilo, color, tamaño y letras se cambian en las otras secciones."</p>
                })}
                {move || {
                    let pages = state.doc.with(|d| {
                        d.node(&sid.get_value())
                            .map_or(0, |n| d.shared_pages(&n.shared))
                    });
                    (pages > 1).then(|| view! {
                        <section class="flex flex-col gap-2 rounded-box border border-primary/30 bg-primary/5 p-3" aria-labelledby="mk-shared-title">
                            <h3 id="mk-shared-title" class="flex items-center gap-2 text-sm font-semibold">
                                <Icon name="page" class="h-4 w-4 text-primary"/>
                                {format!("Compartido en {pages} páginas")}
                            </h3>
                            <p class="text-sm text-base-content/75">"Lo que cambies acá (textos, enlaces, letras) se ve igual en todas tus páginas. El lugar y el tamaño son de cada página."</p>
                            <button type="button" class="btn btn-ghost btn-sm self-start" on:click=move |_| sid.with_value(|id| actions.unshare(id))>
                                "Cambiarlo solo en esta página"
                            </button>
                        </section>
                    })
                }}
            </PanelSection>

            <PanelSection which=Section::Look open title="Estilo y color" icon="star" summary=look_summary>
                <BlockLook id=look_id visible=Signal::derive(move || open.get() == Some(Section::Look)) products/>
            </PanelSection>

            <PanelSection which=Section::Size open title="Tamaño" icon="desktop" summary=size_summary>
                <BlockSize id=size_sid widget=widget_size/>
            </PanelSection>

            <PanelSection
                which=Section::Letters
                open
                title="Letras"
                icon="type"
                help="Lo que no cambies sigue a las letras de toda la tienda."
                summary=letters_summary
            >
                <BlockTypography id=type_id/>
            </PanelSection>

            {has_advanced.then(|| view! {
                <PanelSection which=Section::More open title="Más opciones" icon="plus">
                    {advanced
                        .into_iter()
                        .map(|f| view! { <ShownField products id=advanced_id.clone() field=f/> })
                        .collect_view()}
                </PanelSection>
            })}
        </div>
    }
}

/// Campos que solo valen con cierta opción elegida: (campo, opción, valor).
/// Barra de anuncio: los del cupón, con el estilo Cupón; cuadrículas: los de
/// la tarjeta Completa, con esa tarjeta.
fn field_condition(key: &str) -> Option<(&'static str, &'static str)> {
    match key {
        "code" | "note" | "coupon" => Some(("variant", "d")),
        "card_transfer" | "card_installments" => Some(("card", "full")),
        _ => None,
    }
}

/// Un campo del panel que se muestra solo si su opción está elegida (ver
/// [`field_condition`]).
#[component]
pub(crate) fn ShownField(
    products: StoredValue<Vec<Product>>,
    id: String,
    field: &'static Field,
) -> impl IntoView {
    let state = expect_context::<EditorState>();
    let sid = StoredValue::new(id.clone());
    let cond = field_condition(field.key);
    // Un Memo: el campo se redibuja solo si cambia la opción, no con cada
    // tecla (perdería el foco).
    let shown = Memo::new(move |_| {
        let Some((key, want)) = cond else {
            return true;
        };
        state.doc.with(|d| {
            sid.with_value(|id| {
                let v = crate::fields::field_value(d, id, key);
                // Si el bloque no tiene esa opción (el bloque Cupón), se muestra.
                v.is_null() || v.as_str() == Some(want)
            })
        })
    });
    move || {
        shown
            .get()
            .then(|| view! { <PropField products id=id.clone() field=field/> })
    }
}

/// Contenido de la sección "Tamaño": ancho dibujado con su nombre, alto con
/// deslizador y los números exactos plegados para quien los quiera.
#[component]
fn BlockSize(id: String, widget: Widget) -> impl IntoView {
    let state = expect_context::<EditorState>();
    let limits = widget.limits();
    let sid = StoredValue::new(id);
    let size = move || {
        state
            .doc
            .with(|d| sid.with_value(|id| d.node(id).map(|n| n.size).unwrap_or(limits.default)))
    };
    let resize = move |w: i32, h: i32| {
        sid.with_value(|id| state.apply_nodes(|nodes| grid::resize_node(nodes, id, w, h)));
    };
    let widths: Vec<(u8, &str)> = WIDTHS
        .iter()
        .copied()
        .filter(|(w, _)| (limits.min_w..=limits.max_w).contains(w))
        .collect();
    let cols_n = widths.len().max(1);
    let custom = move || !WIDTHS.iter().any(|(w, _)| *w == size().w);
    // "Ajustar a mano": en píxeles del ancho de la tienda, y si lo escrito
    // no entra, se corrige y se avisa.
    let page_w = state.doc.with_untracked(|d| match d.layout.max_width {
        0 => domain::Layer::Page(0).design_width(),
        w => w,
    });
    let page_w = if state.in_modal_untracked() {
        domain::Layer::ProductModal.design_width()
    } else {
        page_w
    };
    let cols_px =
        move |c: u8| (f64::from(c) * f64::from(page_w) / f64::from(GRID_COLS)).round() as u32;
    let px_to_cols = move |px: f64| (px * f64::from(GRID_COLS) / f64::from(page_w)).round() as i32;
    let (fixed_w, fixed_h) = (RwSignal::new(None::<String>), RwSignal::new(None::<String>));
    // Alto mientras se arrastra el deslizador; se aplica al soltar.
    let live_h = RwSignal::new(None::<u8>);

    view! {
        <section class="flex flex-col gap-4">
            <fieldset class="fieldset gap-1 p-0">
                <legend class="fieldset-legend pt-0 text-sm font-medium">"Ancho"</legend>
                <div class="grid gap-1.5" style=format!("grid-template-columns:repeat({}, minmax(0, 1fr))", cols_n)>
                    {widths.into_iter().map(|(w, name)| view! {
                        <button
                            type="button"
                            class=move || format!(
                                "flex flex-col items-stretch gap-1 rounded-field border p-1.5 text-[11px] leading-tight hover:bg-base-200 {}",
                                if size().w == w { "border-primary bg-primary/10 font-semibold text-primary" } else { "border-base-300" },
                            )
                            aria-pressed=move || (size().w == w).to_string()
                            on:click=move |_| resize(i32::from(w), i32::from(size().h))
                        >
                            <span class="h-3 rounded-sm bg-base-300" aria-hidden="true">
                                <span
                                    class=move || format!("block h-full rounded-sm {}", if size().w == w { "bg-primary" } else { "bg-base-content/40" })
                                    style=format!("width:{}%", u32::from(w) * 100 / u32::from(GRID_COLS))
                                ></span>
                            </span>
                            <span class="text-center">{name}</span>
                        </button>
                    }).collect_view()}
                </div>
                {move || custom().then(|| view! {
                    <p class="label text-xs text-base-content/75">"Este ancho lo ajustaste a mano, en «Ajustar a mano»."</p>
                })}
            </fieldset>
            <fieldset class="fieldset gap-1 p-0">
                <legend class="fieldset-legend pt-0 text-sm font-medium">"Alto"</legend>
                <div class="flex items-center gap-3">
                    <input
                        type="range"
                        class="range range-primary range-xs flex-1"
                        aria-label="Alto del bloque"
                        min=limits.min_h
                        max=limits.max_h
                        prop:value=move || size().h.to_string()
                        on:input=move |ev| {
                            let rows = event_target_value(&ev).parse::<i32>().unwrap_or(i32::from(size().h));
                            live_h.set(Some(rows as u8));
                        }
                        on:change=move |ev| {
                            let rows = event_target_value(&ev).parse::<i32>().unwrap_or(i32::from(size().h));
                            live_h.set(None);
                            resize(i32::from(size().w), rows);
                        }
                    />
                    <span class="min-w-14 text-right text-sm tabular-nums" aria-live="polite">
                        {move || format!("{} px", rows_to_px(live_h.get().unwrap_or(size().h)))}
                    </span>
                </div>
            </fieldset>
            <details class="collapse collapse-arrow border border-base-300">
                <summary class="collapse-title text-sm font-medium">"Ajustar a mano"</summary>
                <div class="collapse-content grid grid-cols-2 gap-2">
                    <label class="flex flex-col gap-1">
                        <span class="text-sm font-medium">"Ancho (px)"</span>
                        <input
                            type="number"
                            class="input input-sm w-full"
                            min=cols_px(limits.min_w)
                            max=cols_px(limits.max_w)
                            inputmode="numeric"
                            aria-describedby="mk-size-w-help"
                            prop:value=move || cols_px(size().w).to_string()
                            on:change=move |ev| {
                                let input = event_target::<web_sys::HtmlInputElement>(&ev);
                                let typed = input.value().trim().replace(',', ".").parse::<f64>().ok();
                                let w = typed.map_or(i32::from(size().w), px_to_cols);
                                resize(w, i32::from(size().h));
                                let got = cols_px(size().w);
                                input.set_value(&got.to_string());
                                fixed_w.set((typed.map(|t| t.round() as u32) != Some(got)).then(|| format!("Quedó en {got} px.")));
                            }
                        />
                        <span id="mk-size-w-help" class="text-xs text-base-content/75">
                            {format!("Entre {} y {} px.", cols_px(limits.min_w), cols_px(limits.max_w))}
                            {move || fixed_w.get().map(|t| view! { " " <span class="font-medium text-amber-800">{t}</span> })}
                        </span>
                    </label>
                    <label class="flex flex-col gap-1">
                        <span class="text-sm font-medium">"Alto (px)"</span>
                        <input
                            type="number"
                            class="input input-sm w-full"
                            min=rows_to_px(limits.min_h)
                            max=rows_to_px(limits.max_h)
                            step=ROW_PX
                            inputmode="numeric"
                            aria-describedby="mk-size-h-help"
                            prop:value=move || rows_to_px(size().h).to_string()
                            on:change=move |ev| {
                                let input = event_target::<web_sys::HtmlInputElement>(&ev);
                                let typed = input.value().trim().replace(',', ".").parse::<f64>().ok();
                                let h = typed.map_or(i32::from(size().h), px_to_rows);
                                resize(i32::from(size().w), h);
                                let got = rows_to_px(size().h);
                                input.set_value(&got.to_string());
                                fixed_h.set((typed.map(|t| t.round() as u32) != Some(got)).then(|| format!("Quedó en {got} px.")));
                            }
                        />
                        <span id="mk-size-h-help" class="text-xs text-base-content/75">
                            {format!("Entre {} y {} px.", rows_to_px(limits.min_h), rows_to_px(limits.max_h))}
                            {move || fixed_h.get().map(|t| view! { " " <span class="font-medium text-amber-800">{t}</span> })}
                        </span>
                    </label>
                </div>
            </details>
        </section>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn button_fields_get_their_own_group() {
        let g = group_fields(Widget::defaults("Banner").unwrap().fields());
        let keys = |v: &[&Field]| v.iter().map(|f| f.key).collect::<Vec<_>>();
        assert_eq!(keys(&g.basic), ["title", "subtitle", "image_url"]);
        let (label, href) = g.button.expect("el banner tiene botón");
        assert_eq!((label.key, href.key), ("button_label", "button_href"));
        assert!(g.advanced.is_empty());
    }

    #[test]
    fn other_advanced_fields_stay_folded() {
        let g = group_fields(Widget::defaults("Countdown").unwrap().fields());
        assert!(g.button.is_some());
        assert_eq!(
            g.advanced.iter().map(|f| f.key).collect::<Vec<_>>(),
            ["ended_text"]
        );

        let g = group_fields(Widget::defaults("Image").unwrap().fields());
        assert!(g.button.is_none());
        assert_eq!(
            g.advanced.iter().map(|f| f.key).collect::<Vec<_>>(),
            ["href", "fit"]
        );
    }
}
