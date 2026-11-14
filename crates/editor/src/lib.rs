//! Editor del panel: lienzo, paleta e inspector.
//!
//! No llama al servidor: recibe la acción de guardado como parámetro.

use domain::{
    Field, FieldKind, Node, PageDoc, PageView, Product, SaveOutcome, Widget, THEMES, WIDGET_TYPES,
};
use leptos::ev::PointerEvent;
use leptos::html::Div;
use leptos::prelude::*;
use wasm_bindgen::JsCast;
use widgets::{node_style, render_widget, RenderCtx};

/// Cuántos pasos de deshacer se guardan.
pub const MAX_UNDO: usize = 50;

/// Acción de guardado: recibe el documento y la versión base.
pub type SaveAction = Action<(PageDoc, u32), Result<SaveOutcome, ServerFnError>>;

/// Todo el estado del editor en un solo lugar. Toda modificación del
/// documento pasa por [`EditorState::apply`].
#[derive(Clone, Copy)]
struct EditorState {
    doc: RwSignal<PageDoc>,
    saved: RwSignal<PageDoc>,
    base_version: RwSignal<u32>,
    selected: RwSignal<Option<String>>,
    undo: RwSignal<Vec<PageDoc>>,
    redo: RwSignal<Vec<PageDoc>>,
}

impl EditorState {
    fn new(view: &PageView) -> Self {
        Self {
            doc: RwSignal::new(view.doc.clone()),
            saved: RwSignal::new(view.doc.clone()),
            base_version: RwSignal::new(view.version),
            selected: RwSignal::new(None),
            undo: RwSignal::new(Vec::new()),
            redo: RwSignal::new(Vec::new()),
        }
    }

    /// Aplica un cambio: apila el documento anterior en `undo` (máximo
    /// [`MAX_UNDO`]) y vacía `redo`. Si el cambio no modifica nada, no hace nada.
    fn apply(&self, f: impl FnOnce(&mut PageDoc)) {
        let before = self.doc.get_untracked();
        let mut next = before.clone();
        f(&mut next);
        if next == before {
            return;
        }
        self.undo.update(|u| {
            u.push(before);
            if u.len() > MAX_UNDO {
                u.remove(0);
            }
        });
        self.redo.update(Vec::clear);
        self.doc.set(next);
    }

    fn undo(&self) {
        if let Some(prev) = self.undo.try_update(Vec::pop).flatten() {
            let current = self.doc.get_untracked();
            self.redo.update(|r| r.push(current));
            self.doc.set(prev);
        }
    }

    fn redo(&self) {
        if let Some(next) = self.redo.try_update(Vec::pop).flatten() {
            let current = self.doc.get_untracked();
            self.undo.update(|u| u.push(current));
            self.doc.set(next);
        }
    }

    fn dirty(&self) -> bool {
        self.doc.with(|d| self.saved.with(|s| d != s))
    }
}

/// Qué borde se arrastra al redimensionar.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Edge {
    /// Borde derecho: cambia el ancho.
    Right,
    /// Borde inferior: cambia el alto.
    Bottom,
    /// Esquina inferior derecha: cambia los dos.
    Corner,
}

#[derive(Clone, Debug, PartialEq)]
enum DragKind {
    /// Mover. Distancia del puntero a la esquina superior izquierda del nodo.
    Move { offset_x: f64, offset_y: f64 },
    /// Redimensionar desde `edge`. Puntero y tamaño al empezar.
    Resize {
        edge: Edge,
        start_x: f64,
        start_y: f64,
        start_w: u8,
        start_h: u8,
    },
}

/// Arrastre en curso.
#[derive(Clone, Debug)]
struct Drag {
    id: String,
    kind: DragKind,
    moved: bool,
}

/// Nuevo `(w, h)` al arrastrar `edge` `dx`, `dy` píxeles, con imán a la grilla.
fn resized(edge: Edge, start_w: u8, start_h: u8, dx: f64, dy: f64, col_pitch: f64) -> (i32, i32) {
    let row_pitch = grid::ROW_HEIGHT_PX + grid::GAP_PX;
    let dw = (dx / col_pitch.max(1.0)).round() as i32;
    let dh = (dy / row_pitch).round() as i32;
    let (w, h) = (i32::from(start_w), i32::from(start_h));
    match edge {
        Edge::Right => (w + dw, h),
        Edge::Bottom => (w, h + dh),
        Edge::Corner => (w + dw, h + dh),
    }
}

/// Resultado del último guardado, para mostrar al usuario.
#[derive(Clone, Debug, PartialEq)]
enum Status {
    Idle,
    Saved(u32),
    Conflict,
    Invalid(Vec<String>),
    Error(String),
}

fn new_node_id(doc: &PageDoc) -> String {
    (1..)
        .map(|k| format!("n{k}"))
        .find(|id| doc.nodes.iter().all(|n| &n.id != id))
        .expect("siempre hay un id libre")
}

/// Cambia una prop de un nodo pasando por `serde_json::Value`.
fn set_field(doc: &mut PageDoc, id: &str, key: &str, value: serde_json::Value) {
    let Some(node) = doc.nodes.iter_mut().find(|n| n.id == id) else {
        return;
    };
    let Ok(mut v) = serde_json::to_value(&node.widget) else {
        return;
    };
    if let Some(props) = v.get_mut("props").and_then(|p| p.as_object_mut()) {
        props.insert(key.to_string(), value);
    }
    if let Ok(widget) = serde_json::from_value::<Widget>(v) {
        node.widget = widget;
    }
}

fn field_value(doc: &PageDoc, id: &str, key: &str) -> serde_json::Value {
    doc.nodes
        .iter()
        .find(|n| n.id == id)
        .and_then(|n| serde_json::to_value(&n.widget).ok())
        .and_then(|v| v.get("props").and_then(|p| p.get(key)).cloned())
        .unwrap_or(serde_json::Value::Null)
}

#[component]
pub fn Editor(initial: PageView, save: SaveAction) -> impl IntoView {
    let state = EditorState::new(&initial);
    let products = StoredValue::new(initial.products.clone());
    let status = RwSignal::new(Status::Idle);
    // Documento enviado en el guardado en curso.
    let sent = RwSignal::new(None::<PageDoc>);

    // Resultado del guardado.
    Effect::new(move |_| {
        let Some(result) = save.value().get() else {
            return;
        };
        match result {
            Ok(SaveOutcome::Saved { version }) => {
                if let Some(doc) = sent.get_untracked() {
                    state.saved.set(doc);
                }
                state.base_version.set(version);
                status.set(Status::Saved(version));
            }
            Ok(SaveOutcome::Conflict { .. }) => status.set(Status::Conflict),
            Ok(SaveOutcome::Invalid { errors }) => status.set(Status::Invalid(errors)),
            Err(e) => status.set(Status::Error(e.to_string())),
        }
    });

    // Con cambios sin guardar, cerrar o recargar la pestaña pide confirmación.
    Effect::new(move |_| {
        let handle = window_event_listener(leptos::ev::beforeunload, move |ev| {
            if state
                .doc
                .with_untracked(|d| state.saved.with_untracked(|s| d != s))
            {
                ev.prevent_default();
                ev.set_return_value("");
            }
        });
        on_cleanup(move || handle.remove());
    });

    let on_save = move |_| {
        let doc = state.doc.get_untracked();
        sent.set(Some(doc.clone()));
        status.set(Status::Idle);
        save.dispatch((doc, state.base_version.get_untracked()));
    };

    view! {
        <div data-theme="light" class="flex h-screen flex-col bg-base-200 text-base-content">
            <div class="navbar gap-2 border-b border-base-300 bg-base-100 px-4">
                <div class="flex-1">
                    <span class="text-lg font-bold">"Editor"</span>
                    <a class="link link-hover ml-4 text-sm" href="/" target="_blank">"Ver marketplace"</a>
                </div>
                <label class="flex items-center gap-2 text-sm">
                    "Tema"
                    <select
                        class="select select-sm w-36"
                        prop:value=move || state.doc.with(|d| d.theme.clone())
                        on:change=move |ev| {
                            let theme = event_target_value(&ev);
                            state.apply(|d| d.theme = theme);
                        }
                    >
                        {THEMES.iter().map(|t| view! { <option value=*t>{*t}</option> }).collect_view()}
                    </select>
                </label>
                <button
                    type="button"
                    class="btn btn-sm"
                    disabled=move || state.undo.with(Vec::is_empty)
                    on:click=move |_| state.undo()
                >
                    "Deshacer"
                </button>
                <button
                    type="button"
                    class="btn btn-sm"
                    disabled=move || state.redo.with(Vec::is_empty)
                    on:click=move |_| state.redo()
                >
                    "Rehacer"
                </button>
                <button
                    type="button"
                    class="btn btn-sm btn-primary"
                    disabled=move || !state.dirty() || save.pending().get()
                    on:click=on_save
                >
                    {move || if save.pending().get() { "Guardando…" } else { "Guardar" }}
                </button>
            </div>
            {view! { <StatusBar status/> }.into_any()}
            <div class="flex min-h-0 flex-1">
                {view! { <Palette state/> }.into_any()}
                {view! { <Canvas state products/> }.into_any()}
                {view! { <Inspector state products/> }.into_any()}
            </div>
        </div>
    }
}

#[component]
fn StatusBar(status: RwSignal<Status>) -> impl IntoView {
    move || {
        match status.get() {
        Status::Idle => ().into_any(),
        Status::Saved(v) => view! {
            <div class="alert alert-success rounded-none py-2">{format!("Guardado como versión {v}.")}</div>
        }
        .into_any(),
        Status::Conflict => view! {
            <div class="alert alert-warning rounded-none py-2">
                <span>"Alguien guardó otra versión mientras editabas. Recargá para ver la última; tus cambios sin guardar se pierden."</span>
                <button
                    type="button"
                    class="btn btn-sm"
                    on:click=move |_| {
                        if let Some(w) = web_sys::window() {
                            _ = w.location().reload();
                        }
                    }
                >
                    "Recargar"
                </button>
            </div>
        }
        .into_any(),
        Status::Invalid(errors) => view! {
            <div class="alert alert-error rounded-none py-2">
                <div>
                    <p class="font-semibold">"No se pudo guardar:"</p>
                    <ul class="list-disc pl-5">
                        {errors.into_iter().map(|e| view! { <li>{e}</li> }).collect_view()}
                    </ul>
                </div>
            </div>
        }
        .into_any(),
        Status::Error(e) => view! {
            <div class="alert alert-error rounded-none py-2">{format!("Error al guardar: {e}")}</div>
        }
        .into_any(),
    }
    }
}

#[component]
fn Palette(state: EditorState) -> impl IntoView {
    view! {
        <aside class="w-56 shrink-0 overflow-y-auto border-r border-base-300 bg-base-100 p-3">
            <h2 class="mb-2 text-sm font-semibold uppercase opacity-60">"Widgets"</h2>
            <ul class="menu w-full p-0">
                {WIDGET_TYPES
                    .iter()
                    .map(|(type_name, label)| {
                        let type_name = *type_name;
                        view! {
                            <li>
                                <button
                                    type="button"
                                    on:click=move |_| {
                                        let Some(widget) = Widget::defaults(type_name) else {
                                            return;
                                        };
                                        let id = new_node_id(&state.doc.get_untracked());
                                        let node = Node::new(id.clone(), widget);
                                        state.apply(|d| grid::add_node(&mut d.nodes, node));
                                        state.selected.set(Some(id));
                                    }
                                >
                                    {*label}
                                </button>
                            </li>
                        }
                    })
                    .collect_view()}
            </ul>
        </aside>
    }
}

#[component]
fn Canvas(state: EditorState, products: StoredValue<Vec<Product>>) -> impl IntoView {
    let grid_ref = NodeRef::<Div>::new();
    let drag = RwSignal::new(None::<Drag>);
    // Vista previa del arrastre: los nodos como quedarían al soltar.
    let preview = RwSignal::new(None::<Vec<Node>>);

    let on_pointer_down = move |ev: PointerEvent, id: String| {
        if ev.button() != 0 {
            return;
        }
        ev.prevent_default();
        state.selected.set(Some(id.clone()));
        let Some(node_el) = ev
            .current_target()
            .and_then(|t| t.dyn_into::<web_sys::Element>().ok())
        else {
            return;
        };
        let rect = node_el.get_bounding_client_rect();
        // La captura va sobre la grilla, que no se vuelve a crear al redibujar los nodos.
        if let Some(grid_el) = grid_ref.get_untracked() {
            _ = grid_el.set_pointer_capture(ev.pointer_id());
        }
        drag.set(Some(Drag {
            id,
            kind: DragKind::Move {
                offset_x: f64::from(ev.client_x()) - rect.left(),
                offset_y: f64::from(ev.client_y()) - rect.top(),
            },
            moved: false,
        }));
    };

    let on_resize_down = move |ev: PointerEvent, id: String, edge: Edge| {
        if ev.button() != 0 {
            return;
        }
        ev.prevent_default();
        ev.stop_propagation();
        let Some(size) = state
            .doc
            .with_untracked(|d| d.nodes.iter().find(|n| n.id == id).map(|n| n.size))
        else {
            return;
        };
        if let Some(grid_el) = grid_ref.get_untracked() {
            _ = grid_el.set_pointer_capture(ev.pointer_id());
        }
        drag.set(Some(Drag {
            id,
            kind: DragKind::Resize {
                edge,
                start_x: f64::from(ev.client_x()),
                start_y: f64::from(ev.client_y()),
                start_w: size.w,
                start_h: size.h,
            },
            moved: false,
        }));
    };

    let on_pointer_move = move |ev: PointerEvent| {
        let Some(d) = drag.get_untracked() else {
            return;
        };
        let Some(grid_el) = grid_ref.get_untracked() else {
            return;
        };
        let g = grid_el.get_bounding_client_rect();
        let col_pitch = (g.width() - grid::GAP_PX * 11.0) / 12.0 + grid::GAP_PX;
        let row_pitch = grid::ROW_HEIGHT_PX + grid::GAP_PX;
        let mut nodes = state.doc.with_untracked(|doc| doc.nodes.clone());
        match d.kind {
            DragKind::Move { offset_x, offset_y } => {
                let px = f64::from(ev.client_x()) - g.left() - offset_x;
                let py = f64::from(ev.client_y()) - g.top() - offset_y;
                // Medio paso de celda para que el imán redondee en vez de truncar.
                let (cx, cy) =
                    grid::cell_from_point(px + col_pitch / 2.0, py + row_pitch / 2.0, g.width());
                grid::move_node(&mut nodes, &d.id, cx, cy);
            }
            DragKind::Resize {
                edge,
                start_x,
                start_y,
                start_w,
                start_h,
            } => {
                let dx = f64::from(ev.client_x()) - start_x;
                let dy = f64::from(ev.client_y()) - start_y;
                let (w, h) = resized(edge, start_w, start_h, dx, dy, col_pitch);
                // Al tirar del borde derecho el nodo no se corre: el ancho topa en la columna 12.
                let x = nodes
                    .iter()
                    .find(|n| n.id == d.id)
                    .map_or(0, |n| i32::from(n.pos.x));
                grid::resize_node(
                    &mut nodes,
                    &d.id,
                    w.min(i32::from(domain::GRID_COLS) - x),
                    h,
                );
            }
        }
        let changed = state.doc.with_untracked(|doc| doc.nodes != nodes);
        if changed || d.moved {
            if !d.moved {
                drag.update(|d| {
                    if let Some(d) = d {
                        d.moved = true;
                    }
                });
            }
            if preview.with_untracked(|p| p.as_ref() != Some(&nodes)) {
                preview.set(Some(nodes));
            }
        }
    };

    let on_pointer_up = move |ev: PointerEvent| {
        if drag.get_untracked().is_none() {
            return;
        }
        if let Some(grid_el) = grid_ref.get_untracked() {
            _ = grid_el.release_pointer_capture(ev.pointer_id());
        }
        if let Some(nodes) = preview.get_untracked() {
            state.apply(|d| d.nodes = nodes);
        }
        preview.set(None);
        drag.set(None);
    };

    let shown_nodes = move || {
        let mut nodes = preview
            .get()
            .unwrap_or_else(|| state.doc.with(|d| d.nodes.clone()));
        grid::sort_by_position(&mut nodes);
        nodes
    };

    view! {
        <section class="min-w-0 flex-1 overflow-auto">
            <div data-theme=move || state.doc.with(|d| d.theme.clone()) class="min-h-full bg-base-200 text-base-content">
                <div class="mx-auto max-w-6xl p-4">
                    <div
                        node_ref=grid_ref
                        class="mk-grid min-h-96 select-none"
                        on:pointermove=on_pointer_move
                        on:pointerup=on_pointer_up
                        on:pointercancel=on_pointer_up
                    >
                        {move || {
                            let ctx = RenderCtx {
                                products: products.get_value(),
                                editing: true,
                            };
                            let selected = state.selected.get();
                            let dragging = drag.with(|d| d.as_ref().map(|d| d.id.clone()));
                            let resizing = drag.with(|d| {
                                d.as_ref().is_some_and(|d| matches!(d.kind, DragKind::Resize { .. }))
                            });
                            shown_nodes()
                                .into_iter()
                                .map(|node| {
                                    let id = node.id.clone();
                                    let is_selected = selected.as_deref() == Some(id.as_str());
                                    let is_dragged = dragging.as_deref() == Some(id.as_str());
                                    let class = if is_dragged && !resizing {
                                        "mk-node relative cursor-grabbing touch-none rounded-box outline outline-2 outline-offset-2 outline-primary opacity-80"
                                    } else if is_selected || is_dragged {
                                        "mk-node relative cursor-grab touch-none rounded-box outline outline-2 outline-offset-2 outline-primary"
                                    } else {
                                        "mk-node relative cursor-grab touch-none rounded-box outline outline-1 outline-dashed outline-base-300 hover:outline-primary"
                                    };
                                    let content = render_widget(&node, &ctx);
                                    let size_label = format!("{}×{}", node.size.w, node.size.h);
                                    let handles = (is_selected || is_dragged).then(|| {
                                        let (r, b, c) = (id.clone(), id.clone(), id.clone());
                                        view! {
                                            <div
                                                class="mk-handle mk-handle-e"
                                                title="Arrastrá para cambiar el ancho"
                                                on:pointerdown=move |ev| on_resize_down(ev, r.clone(), Edge::Right)
                                            ></div>
                                            <div
                                                class="mk-handle mk-handle-s"
                                                title="Arrastrá para cambiar el alto"
                                                on:pointerdown=move |ev| on_resize_down(ev, b.clone(), Edge::Bottom)
                                            ></div>
                                            <div
                                                class="mk-handle mk-handle-se"
                                                title="Arrastrá para cambiar ancho y alto"
                                                on:pointerdown=move |ev| on_resize_down(ev, c.clone(), Edge::Corner)
                                            ></div>
                                            <span class="badge badge-primary badge-sm pointer-events-none absolute right-2 top-2 z-10">{size_label}</span>
                                        }
                                    });
                                    view! {
                                        <div
                                            class=class
                                            style=node_style(&node)
                                            on:pointerdown=move |ev| on_pointer_down(ev, id.clone())
                                        >
                                            <div class="pointer-events-none h-full">{content}</div>
                                            {handles}
                                        </div>
                                    }
                                })
                                .collect_view()
                        }}
                    </div>
                    {move || state.doc.with(|d| d.nodes.is_empty()).then(|| view! {
                        <p class="py-8 text-center opacity-60">"Agregá widgets desde la paleta de la izquierda."</p>
                    })}
                </div>
            </div>
        </section>
    }
}

#[component]
fn Inspector(state: EditorState, products: StoredValue<Vec<Product>>) -> impl IntoView {
    // Se redibuja solo cuando cambia el nodo seleccionado (o deja de existir),
    // no con cada tecla, para no perder el foco de los campos.
    let current = Memo::new(move |_| {
        state
            .selected
            .get()
            .filter(|id| state.doc.with(|d| d.nodes.iter().any(|n| &n.id == id)))
    });

    view! {
        <aside class="w-80 shrink-0 overflow-y-auto border-l border-base-300 bg-base-100 p-4">
            {move || match current.get() {
                None => view! {
                    <p class="text-sm opacity-60">"Seleccioná un widget del lienzo para editarlo."</p>
                }
                .into_any(),
                Some(id) => {
                    let widget = state.doc.with_untracked(|d| {
                        d.nodes.iter().find(|n| n.id == id).map(|n| n.widget.clone())
                    });
                    match widget {
                        Some(widget) => view! { <NodeInspector state products id widget/> }.into_any(),
                        None => ().into_any(),
                    }
                }
            }}
        </aside>
    }
}

#[component]
fn NodeInspector(
    state: EditorState,
    products: StoredValue<Vec<Product>>,
    id: String,
    widget: Widget,
) -> impl IntoView {
    let label = widget.label();
    let limits = widget.limits();
    let size_id = StoredValue::new(id.clone());
    let size = move || {
        state.doc.with(|d| {
            size_id.with_value(|sid| {
                d.nodes
                    .iter()
                    .find(|n| &n.id == sid)
                    .map(|n| n.size)
                    .unwrap_or(limits.default)
            })
        })
    };
    let resize = move |w: i32, h: i32| {
        size_id.with_value(|sid| state.apply(|d| grid::resize_node(&mut d.nodes, sid, w, h)));
    };
    let delete_id = id.clone();
    let basic: Vec<&'static Field> = widget.fields().iter().filter(|f| !f.advanced).collect();
    let advanced: Vec<&'static Field> = widget.fields().iter().filter(|f| f.advanced).collect();
    let has_advanced = !advanced.is_empty();
    let basic_id = id.clone();
    let advanced_id = id.clone();

    view! {
        <div class="flex flex-col gap-4">
            <h2 class="text-lg font-bold">{label}</h2>
            <div class="grid grid-cols-2 gap-2">
                <label class="flex flex-col gap-1">
                    <span class="text-sm font-medium">"Ancho"</span>
                    <input
                        type="number"
                        class="input input-sm w-full"
                        min=limits.min_w
                        max=limits.max_w
                        prop:value=move || size().w.to_string()
                        on:change=move |ev| {
                            if let Ok(w) = event_target_value(&ev).parse::<i32>() {
                                resize(w, i32::from(size().h));
                            }
                        }
                    />
                    <span class="text-xs opacity-60">{format!("{} a {} columnas", limits.min_w, limits.max_w)}</span>
                </label>
                <label class="flex flex-col gap-1">
                    <span class="text-sm font-medium">"Alto"</span>
                    <input
                        type="number"
                        class="input input-sm w-full"
                        min=limits.min_h
                        max=limits.max_h
                        prop:value=move || size().h.to_string()
                        on:change=move |ev| {
                            if let Ok(h) = event_target_value(&ev).parse::<i32>() {
                                resize(i32::from(size().w), h);
                            }
                        }
                    />
                    <span class="text-xs opacity-60">{format!("{} a {} filas", limits.min_h, limits.max_h)}</span>
                </label>
            </div>
            <p class="text-xs opacity-60">"También podés arrastrar los bordes del widget en el lienzo."</p>
            {basic
                .into_iter()
                .map(|f| view! { <PropField state products id=basic_id.clone() field=f/> })
                .collect_view()}
            {has_advanced.then(|| view! {
                <details class="collapse collapse-arrow border border-base-300">
                    <summary class="collapse-title text-sm font-medium">"Avanzado"</summary>
                    <div class="collapse-content flex flex-col gap-4">
                        {advanced
                            .into_iter()
                            .map(|f| view! { <PropField state products id=advanced_id.clone() field=f/> })
                            .collect_view()}
                    </div>
                </details>
            })}
            <button
                type="button"
                class="btn btn-error btn-outline btn-sm"
                on:click=move |_| {
                    state.apply(|d| grid::remove_node(&mut d.nodes, &delete_id));
                    state.selected.set(None);
                }
            >
                "Borrar"
            </button>
        </div>
    }
}

/// Campo de primer nivel de las props de un nodo.
#[component]
fn PropField(
    state: EditorState,
    products: StoredValue<Vec<Product>>,
    id: String,
    field: &'static Field,
) -> impl IntoView {
    let key = field.key;
    let id = StoredValue::new(id);
    let value = Signal::derive(move || {
        state
            .doc
            .with(|d| id.with_value(|rid| field_value(d, rid, key)))
    });
    let set = Callback::new(move |v: serde_json::Value| {
        id.with_value(|rid| state.apply(|d| set_field(d, rid, key, v)));
    });
    view! { <FieldInput products field value set/> }
}

/// Valor inicial de un campo, para elementos nuevos de una lista.
fn empty_value(kind: &FieldKind) -> serde_json::Value {
    use serde_json::Value;
    match kind {
        FieldKind::Number { min, .. } => Value::from(*min),
        FieldKind::Bool => Value::Bool(false),
        FieldKind::Select { options } => {
            Value::String(options.first().map(|(k, _)| *k).unwrap_or_default().into())
        }
        FieldKind::List { .. } => Value::Array(Vec::new()),
        _ => Value::String(String::new()),
    }
}

/// Elemento nuevo para una lista: copia del último (así hereda imagen y
/// estilo) o, si está vacía, uno con los valores iniciales de cada campo.
fn new_list_item(items: &[serde_json::Value], item: &[Field]) -> serde_json::Value {
    items.last().cloned().unwrap_or_else(|| {
        serde_json::Value::Object(
            item.iter()
                .map(|f| (f.key.to_string(), empty_value(&f.kind)))
                .collect(),
        )
    })
}

/// Control de un campo según su tipo. `value` es el valor actual y `set`
/// reemplaza el valor entero (en una lista, la lista entera).
#[component]
fn FieldInput(
    products: StoredValue<Vec<Product>>,
    field: &'static Field,
    value: Signal<serde_json::Value>,
    set: Callback<serde_json::Value>,
) -> impl IntoView {
    let text = move || value.get().as_str().unwrap_or_default().to_string();
    let set_str = move |s: String| set.run(serde_json::Value::String(s));

    let input = match &field.kind {
        FieldKind::Text { max } => view! {
            <input
                type="text"
                class="input input-sm w-full"
                maxlength=*max
                prop:value=text
                on:input=move |ev| set_str(event_target_value(&ev))
            />
        }
        .into_any(),
        FieldKind::LongText { max } => view! {
            <textarea
                class="textarea textarea-sm h-24 w-full"
                maxlength=*max
                prop:value=text
                on:input=move |ev| set_str(event_target_value(&ev))
            ></textarea>
        }
        .into_any(),
        FieldKind::Url => view! {
            <input
                type="text"
                class="input input-sm w-full"
                placeholder="https:// o /ruta"
                prop:value=text
                on:input=move |ev| set_str(event_target_value(&ev))
            />
        }
        .into_any(),
        FieldKind::DateTime => view! {
            <input
                type="datetime-local"
                class="input input-sm w-full"
                prop:value=text
                on:input=move |ev| set_str(event_target_value(&ev))
            />
        }
        .into_any(),
        FieldKind::Number { min, max } => {
            let (min, max) = (*min, *max);
            view! {
                <input
                    type="number"
                    class="input input-sm w-full"
                    min=min
                    max=max
                    prop:value=move || value.get().as_i64().map(|n| n.to_string()).unwrap_or_default()
                    on:input=move |ev| {
                        if let Ok(n) = event_target_value(&ev).parse::<i64>() {
                            set.run(serde_json::Value::from(n.clamp(min, max)));
                        }
                    }
                />
            }
            .into_any()
        }
        FieldKind::Bool => view! {
            <input
                type="checkbox"
                class="toggle toggle-sm"
                prop:checked=move || value.get().as_bool().unwrap_or(false)
                on:change=move |ev| set.run(serde_json::Value::Bool(event_target_checked(&ev)))
            />
        }
        .into_any(),
        FieldKind::Select { options } => view! {
            <select
                class="select select-sm w-full"
                prop:value=text
                on:change=move |ev| set_str(event_target_value(&ev))
            >
                {options.iter().map(|(v, l)| view! { <option value=*v>{*l}</option> }).collect_view()}
            </select>
        }
        .into_any(),
        FieldKind::ProductRef => view! {
            <select
                class="select select-sm w-full"
                prop:value=text
                on:change=move |ev| set_str(event_target_value(&ev))
            >
                <option value="">"Elegí un producto"</option>
                {products
                    .get_value()
                    .into_iter()
                    .map(|p| view! {
                        <option value=p.id.clone()>{format!("{} · {}", p.name, ui::format_price(p.price_cents))}</option>
                    })
                    .collect_view()}
            </select>
        }
        .into_any(),
        FieldKind::List { item, min, max } => {
            return view! { <ListInput products field item min=*min max=*max value set/> }.into_any();
        }
    };

    view! {
        <label class="flex flex-col gap-1">
            <span class="text-sm font-medium">{field.label}</span>
            {input}
        </label>
    }
    .into_any()
}

/// Lista editable: cada elemento con sus campos, y botones para agregar,
/// subir, bajar y quitar. Solo se redibuja entera cuando cambia la cantidad.
#[component]
fn ListInput(
    products: StoredValue<Vec<Product>>,
    field: &'static Field,
    item: &'static [Field],
    min: usize,
    max: usize,
    value: Signal<serde_json::Value>,
    set: Callback<serde_json::Value>,
) -> impl IntoView {
    let items = move || {
        value
            .get_untracked()
            .as_array()
            .cloned()
            .unwrap_or_default()
    };
    let len = Memo::new(move |_| value.with(|v| v.as_array().map_or(0, Vec::len)));
    let update = move |f: &dyn Fn(&mut Vec<serde_json::Value>)| {
        let mut list = value
            .get_untracked()
            .as_array()
            .cloned()
            .unwrap_or_default();
        f(&mut list);
        set.run(serde_json::Value::Array(list));
    };

    view! {
        <fieldset class="flex flex-col gap-2">
            <legend class="mb-1 text-sm font-medium">{field.label}</legend>
            {move || {
                let n = len.get();
                (0..n)
                    .map(|i| {
                        let fields = item
                            .iter()
                            .map(|sub| {
                                let key = sub.key;
                                let sub_value = Signal::derive(move || {
                                    value.with(|v| v.get(i).and_then(|it| it.get(key)).cloned())
                                        .unwrap_or(serde_json::Value::Null)
                                });
                                let sub_set = Callback::new(move |v: serde_json::Value| {
                                    update(&|list| {
                                        if let Some(obj) = list.get_mut(i).and_then(|it| it.as_object_mut()) {
                                            obj.insert(key.to_string(), v.clone());
                                        }
                                    });
                                });
                                view! { <FieldInput products field=sub value=sub_value set=sub_set/> }
                            })
                            .collect_view();
                        view! {
                            <div class="flex flex-col gap-2 rounded-box border border-base-300 p-3">
                                <div class="flex items-center gap-1">
                                    <span class="flex-1 text-xs font-semibold opacity-60">{format!("#{}", i + 1)}</span>
                                    <button
                                        type="button"
                                        class="btn btn-ghost btn-xs"
                                        aria-label="Subir"
                                        disabled={i == 0}
                                        on:click=move |_| update(&|list| list.swap(i, i - 1))
                                    >"↑"</button>
                                    <button
                                        type="button"
                                        class="btn btn-ghost btn-xs"
                                        aria-label="Bajar"
                                        disabled={i + 1 >= n}
                                        on:click=move |_| update(&|list| list.swap(i, i + 1))
                                    >"↓"</button>
                                    <button
                                        type="button"
                                        class="btn btn-ghost btn-xs text-error"
                                        aria-label="Quitar"
                                        disabled={n <= min}
                                        on:click=move |_| update(&|list| { list.remove(i); })
                                    >"✕"</button>
                                </div>
                                {fields}
                            </div>
                        }
                    })
                    .collect_view()
            }}
            <button
                type="button"
                class="btn btn-sm btn-outline"
                disabled=move || { len.get() >= max }
                on:click=move |_| {
                    let next = new_list_item(&items(), item);
                    update(&|list| list.push(next.clone()));
                }
            >
                {format!("Agregar ({max} como máximo)")}
            </button>
        </fieldset>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_field_changes_one_prop() {
        let mut doc = PageDoc::default();
        doc.nodes
            .push(Node::new("n1", Widget::defaults("Banner").unwrap()));
        set_field(&mut doc, "n1", "title", serde_json::json!("Hola"));
        assert_eq!(field_value(&doc, "n1", "title"), serde_json::json!("Hola"));
        assert!(matches!(&doc.nodes[0].widget, Widget::Banner(b) if b.title == "Hola"));
        assert_eq!(new_node_id(&doc), "n2");
    }

    #[test]
    fn set_field_replaces_a_whole_list() {
        let mut doc = PageDoc::default();
        doc.nodes
            .push(Node::new("n1", Widget::defaults("Faq").unwrap()));
        let mut items = field_value(&doc, "n1", "items");
        let list = items.as_array_mut().unwrap();
        list.push(new_list_item(list, &[]));
        list[3]["question"] = serde_json::json!("¿Nueva?");
        set_field(&mut doc, "n1", "items", items);
        assert!(
            matches!(&doc.nodes[0].widget, Widget::Faq(f) if f.items.len() == 4 && f.items[3].question == "¿Nueva?")
        );
        assert_eq!(domain::validate(&doc), Ok(()));
    }

    #[test]
    fn new_list_item_has_every_field() {
        let fields = Widget::defaults("Testimonials").unwrap().fields();
        let FieldKind::List { item, .. } = &fields[1].kind else {
            panic!("se esperaba una lista");
        };
        let v = new_list_item(&[], item);
        assert_eq!(
            v,
            serde_json::json!({ "name": "", "text": "", "rating": 1 })
        );
    }

    #[test]
    fn resize_snaps_to_cells() {
        // Columnas de 80 px de paso y filas de 96 px.
        assert_eq!(resized(Edge::Right, 4, 2, 130.0, 500.0, 80.0), (6, 2));
        assert_eq!(resized(Edge::Bottom, 4, 2, 500.0, -100.0, 80.0), (4, 1));
        assert_eq!(resized(Edge::Corner, 4, 2, -50.0, 150.0, 80.0), (3, 4));
    }
}
