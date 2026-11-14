//! Lienzo: los bloques sobre la grilla. Se mueven y redimensionan con el
//! mouse (arrastrando) o con el teclado, y cada uno tiene una barra de
//! acciones para hacer lo mismo sin arrastrar.

use domain::{Node, Product};
use leptos::ev::{KeyboardEvent, PointerEvent};
use leptos::html::Div;
use leptos::prelude::*;
use wasm_bindgen::JsCast;
use widgets::{node_style, render_widget, RenderCtx};

use crate::icons::Icon;
use crate::{Actions, EditorState, Ui, WELCOME_KEY};

/// Qué borde se arrastra al redimensionar.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Edge {
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
pub(crate) fn resized(
    edge: Edge,
    start_w: u8,
    start_h: u8,
    dx: f64,
    dy: f64,
    col_pitch: f64,
) -> (i32, i32) {
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

/// Teclas sobre un bloque enfocado.
fn on_node_key(ev: KeyboardEvent, id: &str, actions: Actions) {
    // Solo las teclas pulsadas sobre el bloque, no sobre su barra de acciones.
    if ev.target() != ev.current_target() {
        return;
    }
    let shift = ev.shift_key();
    match ev.key().as_str() {
        "Enter" | " " => actions.edit(id),
        "ArrowUp" if shift => actions.resize_by(id, 0, -1),
        "ArrowDown" if shift => actions.resize_by(id, 0, 1),
        "ArrowLeft" if shift => actions.resize_by(id, -1, 0),
        "ArrowRight" if shift => actions.resize_by(id, 1, 0),
        "ArrowUp" => actions.move_up(id),
        "ArrowDown" => actions.move_down(id),
        "ArrowLeft" => actions.shift(id, -1),
        "ArrowRight" => actions.shift(id, 1),
        "Delete" | "Backspace" => actions.delete(id),
        _ => return,
    }
    ev.prevent_default();
    ev.stop_propagation();
}

#[component]
pub(crate) fn Canvas(
    products: StoredValue<Vec<Product>>,
    welcome: RwSignal<bool>,
) -> impl IntoView {
    let state = expect_context::<EditorState>();
    let ui = expect_context::<Ui>();
    let actions = expect_context::<Actions>();
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
        let Some(size) = state.node(&id).map(|n| n.size) else {
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
        let Some(d) = drag.get_untracked() else {
            return;
        };
        if let Some(grid_el) = grid_ref.get_untracked() {
            _ = grid_el.release_pointer_capture(ev.pointer_id());
        }
        if let Some(nodes) = preview.get_untracked() {
            state.apply(|doc| doc.nodes = nodes);
            ui.announce(actions.position_text(&d.id));
        }
        preview.set(None);
        drag.set(None);
        crate::reveal_node(&d.id, false);
    };

    let shown_nodes = move || {
        let mut nodes = preview
            .get()
            .unwrap_or_else(|| state.doc.with(|d| d.nodes.clone()));
        grid::sort_by_position(&mut nodes);
        nodes
    };

    let close_welcome = move |_| {
        welcome.set(false);
        if let Some(st) = crate::storage() {
            _ = st.set_item(WELCOME_KEY, "1");
        }
    };

    view! {
        <section class="min-w-0 flex-1 overflow-auto" aria-label="Lienzo de la página">
            <p id="mk-node-help" class="sr-only">
                "Enter para editar. Flechas arriba y abajo para cambiar el orden, izquierda y derecha para correrlo. Mayúscula más flechas para cambiar el tamaño. Suprimir para borrar."
            </p>
            {move || ui.preview.get().then(|| view! {
                <div class="sticky top-0 z-30 flex items-center gap-3 border-b border-base-300 bg-base-100 px-4 py-2 text-sm">
                    <Icon name="eye" class="h-4 w-4"/>
                    <span class="flex-1">"Vista previa: así se va a ver tu tienda cuando guardes."</span>
                    <button type="button" class="btn btn-sm btn-primary" on:click=move |_| ui.preview.set(false)>"Volver a editar"</button>
                </div>
            })}
            <div data-theme=move || state.doc.with(|d| d.theme.clone()) class="min-h-full bg-base-200 text-base-content">
                <div class="mx-auto max-w-6xl p-4">
                    {move || (welcome.get() && !ui.preview.get()).then(|| view! {
                        <div data-theme="light" class="mb-4 rounded-box border border-primary/30 bg-base-100 p-4 text-base-content shadow-sm" role="region" aria-labelledby="mk-welcome-title">
                            <div class="flex items-start gap-3">
                                <div class="flex-1">
                                    <h2 id="mk-welcome-title" class="font-semibold">"Primeros pasos"</h2>
                                    <ol class="mt-2 list-decimal space-y-1 pl-5 text-sm">
                                        <li>"Elegí un bloque de la izquierda: se agrega al final de la página."</li>
                                        <li>"Hacé clic en un bloque para cambiar sus textos e imágenes a la derecha. Arrastralo para moverlo, o usá los botones Subir y Bajar."</li>
                                        <li>"Tocá Guardar para que tu tienda muestre los cambios. Si te equivocás, Deshacer vuelve atrás."</li>
                                    </ol>
                                </div>
                                <button type="button" class="btn btn-sm btn-primary" on:click=close_welcome>"Entendido"</button>
                            </div>
                        </div>
                    })}
                    <div
                        node_ref=grid_ref
                        class="mk-grid min-h-96 select-none"
                        on:pointermove=on_pointer_move
                        on:pointerup=on_pointer_up
                        on:pointercancel=on_pointer_up
                    >
                        {move || {
                            let previewing = ui.preview.get();
                            let ctx = RenderCtx {
                                products: products.get_value(),
                                editing: true,
                            };
                            if previewing {
                                return shown_nodes()
                                    .into_iter()
                                    .map(|node| view! {
                                        <div class="mk-node" style=node_style(&node)>{render_widget(&node, &ctx)}</div>
                                    })
                                    .collect_view()
                                    .into_any();
                            }
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
                                        "mk-node relative cursor-grab touch-none rounded-box outline outline-1 outline-dashed outline-base-content/50 hover:outline-primary"
                                    };
                                    let content = render_widget(&node, &ctx);
                                    let label = node.widget.label();
                                    let aria_label = format!("{label}, ancho {} de 12, alto {}", node.size.w, node.size.h);
                                    let size_label = format!("{}×{}", node.size.w, node.size.h);
                                    let toolbar = (is_selected && !is_dragged).then(|| {
                                        let (up, down, dup, del) = (id.clone(), id.clone(), id.clone(), id.clone());
                                        view! {
                                            <div
                                                data-theme="light"
                                                class="absolute right-2 top-2 z-20 flex items-center gap-1 rounded-field border border-base-300 bg-base-100 p-1 text-base-content shadow-md"
                                                role="toolbar"
                                                aria-label=format!("Acciones de {label}")
                                                on:pointerdown=|ev: PointerEvent| ev.stop_propagation()
                                            >
                                                <button type="button" class="btn btn-ghost btn-sm btn-square" aria-label="Subir" title="Subir (flecha arriba)" on:click=move |_| actions.move_up(&up)>
                                                    <Icon name="up"/>
                                                </button>
                                                <button type="button" class="btn btn-ghost btn-sm btn-square" aria-label="Bajar" title="Bajar (flecha abajo)" on:click=move |_| actions.move_down(&down)>
                                                    <Icon name="down"/>
                                                </button>
                                                <button type="button" class="btn btn-ghost btn-sm btn-square" aria-label="Duplicar" title="Duplicar" on:click=move |_| actions.duplicate(&dup)>
                                                    <Icon name="copy"/>
                                                </button>
                                                <button type="button" class="btn btn-ghost btn-sm btn-square text-red-700" aria-label="Borrar" title="Borrar (Suprimir)" on:click=move |_| actions.delete(&del)>
                                                    <Icon name="trash"/>
                                                </button>
                                                <span class="badge badge-primary badge-sm ml-1" aria-hidden="true">{size_label.clone()}</span>
                                            </div>
                                        }
                                    });
                                    let size_badge = (is_dragged).then(|| view! {
                                        <span class="badge badge-primary badge-sm pointer-events-none absolute right-2 top-2 z-10" aria-hidden="true">{size_label.clone()}</span>
                                    });
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
                                        }
                                    });
                                    let (key_id, down_id) = (id.clone(), id.clone());
                                    view! {
                                        <div
                                            class=class
                                            style=node_style(&node)
                                            data-node-id=id.clone()
                                            tabindex="0"
                                            role="group"
                                            aria-roledescription="bloque"
                                            aria-label=aria_label
                                            aria-describedby="mk-node-help"
                                            aria-current=is_selected.then_some("true")
                                            on:pointerdown=move |ev| on_pointer_down(ev, down_id.clone())
                                            on:keydown=move |ev| on_node_key(ev, &key_id, actions)
                                        >
                                            <div class="pointer-events-none h-full" inert=true>{content}</div>
                                            {toolbar}
                                            {size_badge}
                                            {handles}
                                        </div>
                                    }
                                })
                                .collect_view()
                                .into_any()
                        }}
                    </div>
                    {move || state.doc.with(|d| d.nodes.is_empty()).then(|| view! {
                        <div data-theme="light" class="mx-auto mt-2 flex max-w-md flex-col items-center gap-2 rounded-box border-2 border-dashed border-base-content/40 bg-base-100 p-8 text-center text-base-content">
                            <Icon name="plus" class="h-8 w-8 text-primary"/>
                            <p class="font-semibold">"Tu página está vacía"</p>
                            <p class="text-sm text-base-content/75">"Elegí un bloque de la columna izquierda para empezar. Te recomendamos un Menú superior y un Banner."</p>
                        </div>
                    })}
                </div>
            </div>
        </section>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resize_snaps_to_cells() {
        // Columnas de 80 px de paso y filas de 96 px.
        assert_eq!(resized(Edge::Right, 4, 2, 130.0, 500.0, 80.0), (6, 2));
        assert_eq!(resized(Edge::Bottom, 4, 2, 500.0, -100.0, 80.0), (4, 1));
        assert_eq!(resized(Edge::Corner, 4, 2, -50.0, 150.0, 80.0), (3, 4));
    }
}
