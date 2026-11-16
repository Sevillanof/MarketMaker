//! Lienzo: los bloques sobre la grilla. Se mueven y redimensionan con el
//! mouse (arrastrando) o con el teclado, y cada uno tiene una barra de
//! acciones para hacer lo mismo sin arrastrar.

use domain::{Layer, Node, Product, Side, StoreLayout, Widget, GRID_COLS, ROW_PX};
use leptos::ev::{KeyboardEvent, PointerEvent};
use leptos::html::{Div, Section};
use leptos::prelude::*;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;
use widgets::{node_style, render_widget, sample_product, OpenProduct, ProductDialog, RenderCtx};

use crate::icons::Icon;
use crate::inline::{inline_at, InlineEdit, InlineEditor};
use crate::{new_node_id, size_badge, size_text, Actions, EditorState, NewDrag, Ui, WELCOME_KEY};

/// La página en el lienzo: ancho del contenido y margen de cada lado, en
/// píxeles de diseño, y la escala con que se dibuja.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Frame {
    pub w: f64,
    pub top: f64,
    pub right: f64,
    pub bottom: f64,
    pub left: f64,
    pub k: f64,
}

impl Frame {
    /// Ancho de toda la página (contenido y márgenes de los costados).
    fn total(&self) -> f64 {
        self.w + self.left + self.right
    }
}

/// La página como la ve una persona en una pantalla de `screen_w` px (la
/// misma que muestra la vista previa), achicada para que entre en el lienzo
/// de `canvas_w` px: así Editar se ve igual que la vista previa, solo que
/// más chico. El contenido mide su ancho máximo o, si no entra, lo que deja
/// la pantalla (como en la tienda); con "Ancho completo", toda la pantalla.
/// `dragging` es el ancho que se está probando al arrastrar los bordes.
pub(crate) fn page_frame(
    layout: &StoreLayout,
    dragging: Option<u16>,
    canvas_w: f64,
    screen_w: f64,
) -> Frame {
    let px = |s: Side| f64::from(layout.pad(s));
    let (left, right) = (px(Side::Left), px(Side::Right));
    let screen_w = screen_w.max(canvas_w);
    let avail = (screen_w - left - right).max(f64::from(GRID_COLS));
    let w = match dragging {
        Some(w) => f64::from(w),
        None if layout.is_full() => avail,
        None => f64::from(layout.max_width).min(avail),
    };
    Frame {
        w,
        top: px(Side::Top),
        right,
        bottom: px(Side::Bottom),
        left,
        k: fit_scale(canvas_w, screen_w.max(w + left + right)),
    }
}

/// Escala para que una página de `total` px de diseño entre en el lienzo.
/// El lienzo dibuja siempre al ancho de diseño y lo achica con
/// `transform: scale` si no entra, así cada bloque se ve como en la tienda.
fn fit_scale(canvas_w: f64, total: f64) -> f64 {
    (canvas_w / total.max(1.0)).clamp(0.3, 1.0)
}

/// Arrastre de un borde de la página para cambiar su ancho.
#[derive(Clone, Copy, Debug, PartialEq)]
struct WidthDrag {
    /// `-1` el borde izquierdo, `1` el derecho.
    side: f64,
    start_x: f64,
    /// Ancho del contenido al empezar, en px de diseño.
    start_w: f64,
    /// Escala del lienzo al empezar: con ella se pasa lo que se mueve el
    /// puntero a píxeles de diseño (la escala del dibujo cambia al arrastrar).
    scale: f64,
    /// Ancho que quedaría al soltar (con imán); `None` hasta que se mueve.
    width: Option<u16>,
    pointer_id: i32,
}

/// Nuevo ancho del contenido al llevar un borde `dx` píxeles de pantalla
/// hacia afuera (`side` = 1 a la derecha, -1 a la izquierda). La página está
/// centrada, así que el ancho cambia el doble de lo que se mueve el borde.
pub(crate) fn dragged_width(start_w: f64, side: f64, dx: f64, scale: f64) -> u16 {
    domain::snap_width(start_w + side * 2.0 * dx / scale.max(0.01))
}

/// Margen de cada bloque dentro de su celda: la mitad de la separación.
const MARGIN_PX: f64 = grid::GAP_PX / 2.0;
/// Alto mínimo del lienzo, en filas.
const MIN_ROWS: u32 = 48;
/// Alto mínimo de la ficha del producto, en filas.
const MIN_MODAL_ROWS: u32 = 24;

/// Distancia al borde del lienzo, en px, en la que arrastrar hace scroll.
const AUTOSCROLL_EDGE_PX: f64 = 48.0;
/// Cuánto se desplaza el lienzo por cada movimiento cerca del borde.
const AUTOSCROLL_STEP_PX: i32 = 16;

/// Si el puntero está cerca del borde de arriba o de abajo del lienzo, lo
/// desplaza para poder llevar el bloque más allá de lo que se ve.
fn autoscroll(section: &web_sys::Element, client_y: f64) {
    let r = section.get_bounding_client_rect();
    let step = if client_y > r.bottom() - AUTOSCROLL_EDGE_PX {
        AUTOSCROLL_STEP_PX
    } else if client_y < r.top() + AUTOSCROLL_EDGE_PX {
        -AUTOSCROLL_STEP_PX
    } else {
        return;
    };
    section.set_scroll_top(section.scroll_top() + step);
}

/// Escala con la que se dibuja la grilla: píxeles de pantalla por píxel de
/// diseño. Sale del tamaño real de la grilla, así sirve con cualquier zoom.
fn scale_of(grid: &web_sys::HtmlElement, rect: &web_sys::DomRect) -> f64 {
    let layout = f64::from(grid.offset_width());
    if layout > 0.0 {
        (rect.width() / layout).max(0.01)
    } else {
        1.0
    }
}

/// Celda donde queda la esquina superior izquierda de un bloque nuevo de
/// `w` columnas soltado con el puntero en `(client_x, client_y)`: centrado
/// a lo ancho bajo el puntero y con el borde de arriba apenas por encima.
pub(crate) fn drop_cell(
    client_x: f64,
    client_y: f64,
    grid: &web_sys::DomRect,
    scale: f64,
    w: u8,
) -> (i32, i32) {
    let width = grid.width() / scale;
    let pitch = grid::col_pitch(width);
    let px = (client_x - grid.left()) / scale - f64::from(w) * pitch / 2.0;
    let py = (client_y - grid.top()) / scale - grid::GAP_PX;
    grid::cell_from_point(px, py, width)
}

/// Producto de la tarjeta que está bajo el puntero dentro del bloque
/// `node_id`, si hay. Se mira la posición de cada tarjeta (así cuenta toda
/// la tarjeta, también su borde).
fn product_under(node_id: &str, client_x: i32, client_y: i32) -> Option<String> {
    let selector = format!("[data-node-id='{node_id}'] [data-product-id]");
    let cards = document().query_selector_all(&selector).ok()?;
    let (x, y) = (f64::from(client_x), f64::from(client_y));
    (0..cards.length()).find_map(|i| {
        let el = cards.get(i)?.dyn_into::<web_sys::Element>().ok()?;
        let r = el.get_bounding_client_rect();
        let inside = x >= r.left() && x <= r.right() && y >= r.top() && y <= r.bottom();
        inside
            .then(|| el.get_attribute("data-product-id"))
            .flatten()
            .filter(|id| !id.is_empty())
    })
}

/// Lo que se puede tocar dentro de un bloque y funciona igual que en la
/// tienda: flechas y puntos del carrusel, pestañas, preguntas frecuentes…
/// Los botones desactivados (los que llevan a otra página mientras se
/// edita) no cuentan: los resuelve [`link_under`].
const CONTROLS: &str = "button:not(:disabled), summary, input:not(:disabled), label, select";

/// Control del contenido del bloque `node_id` que está bajo el puntero, si
/// hay. El arrastre captura el puntero en la grilla, así que el clic del
/// navegador nunca le llega al control: el editor lo busca y lo toca.
fn control_under(node_id: &str, client_x: i32, client_y: i32) -> Option<web_sys::HtmlElement> {
    let hit = document().element_from_point(client_x as f32, client_y as f32)?;
    let control = hit.closest(CONTROLS).ok()??;
    let content = control.closest(".mk-content").ok()??;
    let node = content.closest("[data-node-id]").ok()??;
    (node.get_attribute("data-node-id").as_deref() == Some(node_id))
        .then(|| control.dyn_into::<web_sys::HtmlElement>().ok())
        .flatten()
}

/// Enlace bajo el puntero dentro del bloque `node_id`, si hay: su dirección
/// y su texto. Igual que [`product_under`], mira la posición de cada enlace
/// (los widgets los marcan con `data-href` mientras se edita).
fn link_under(node_id: &str, client_x: i32, client_y: i32) -> Option<(String, String)> {
    let selector = format!("[data-node-id='{node_id}'] [data-href]");
    let links = document().query_selector_all(&selector).ok()?;
    let (x, y) = (f64::from(client_x), f64::from(client_y));
    (0..links.length()).find_map(|i| {
        let el = links.get(i)?.dyn_into::<web_sys::Element>().ok()?;
        let r = el.get_bounding_client_rect();
        let inside = x >= r.left() && x <= r.right() && y >= r.top() && y <= r.bottom();
        if !inside {
            return None;
        }
        let href = el.get_attribute("data-href")?;
        Some((
            href,
            el.text_content().unwrap_or_default().trim().to_string(),
        ))
    })
}

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
    Move {
        offset_x: f64,
        offset_y: f64,
        start_x: f64,
        start_y: f64,
    },
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
    /// El puntero que empezó el arrastre: los demás (un segundo dedo, otro
    /// mouse) no lo mueven ni lo terminan.
    pointer_id: i32,
}

/// Nuevo `(w, h)` al arrastrar `edge` `dx`, `dy` píxeles de pantalla, con
/// imán a la grilla. `col_pitch` y `row_pitch` son lo que miden una columna y
/// una fila en pantalla.
pub(crate) fn resized(
    edge: Edge,
    start_w: u8,
    start_h: u8,
    (dx, dy): (f64, f64),
    (col_pitch, row_pitch): (f64, f64),
) -> (i32, i32) {
    let dw = (dx / col_pitch.max(1.0)).round() as i32;
    let dh = (dy / row_pitch.max(1.0)).round() as i32;
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

/// Flechas, Inicio y Fin dentro de la barra de un bloque: mueven el foco
/// entre sus botones (una sola parada de Tab para toda la barra).
fn on_toolbar_key(ev: KeyboardEvent, active: RwSignal<usize>) {
    let Some(bar) = ev
        .current_target()
        .and_then(|t| t.dyn_into::<web_sys::Element>().ok())
    else {
        return;
    };
    let Ok(buttons) = bar.query_selector_all("button") else {
        return;
    };
    let n = buttons.length() as usize;
    if n == 0 {
        return;
    }
    let i = active.get_untracked().min(n - 1);
    let next = match ev.key().as_str() {
        "ArrowRight" | "ArrowDown" => (i + 1) % n,
        "ArrowLeft" | "ArrowUp" => (i + n - 1) % n,
        "Home" => 0,
        "End" => n - 1,
        _ => return,
    };
    ev.prevent_default();
    ev.stop_propagation();
    active.set(next);
    if let Some(b) = buttons
        .get(next as u32)
        .and_then(|b| b.dyn_into::<web_sys::HtmlElement>().ok())
    {
        _ = b.focus();
    }
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
    let stage_ref = NodeRef::<Div>::new();
    let section_ref = NodeRef::<Section>::new();
    let drag = RwSignal::new(None::<Drag>);
    // Vista previa del arrastre: los nodos como quedarían al soltar.
    let preview = RwSignal::new(None::<Vec<Node>>);
    // Bloque nuevo que se arrastra desde la paleta, con el id que va a tener.
    let incoming = RwSignal::new(None::<Node>);
    // Esquina del bloque que se arrastra con el mouse, en píxeles de diseño:
    // sigue al puntero sin imán, y el lugar donde va a quedar se ve aparte.
    let drag_pos = RwSignal::new(None::<(f64, f64)>);
    // Ancho del lienzo en pantalla, y el de la pantalla que muestra la
    // vista previa (todo el ancho de la ventana, sin las columnas).
    let canvas_w = RwSignal::new(1200.0_f64);
    let screen_w = RwSignal::new(1440.0_f64);
    // Escala de la ficha del producto (que tiene su propio ancho).
    let modal_scale = RwSignal::new(1.0_f64);
    let width_drag = RwSignal::new(None::<WidthDrag>);
    // El texto que se está escribiendo directo en el lienzo.
    let inline_edit = RwSignal::new(None::<InlineEdit>);
    // La página: ancho del contenido y margen (px de diseño) y escala.
    let frame = Memo::new(move |_| {
        let wd = width_drag.get();
        // Siempre entra entera en el lienzo, también mientras se arrastra un
        // borde: la página se achica o se agranda y la etiqueta dice el ancho.
        state.doc.with(|d| {
            page_frame(
                &d.layout,
                wd.and_then(|d| d.width),
                canvas_w.get(),
                screen_w.get(),
            )
        })
    });
    // Píxeles de pantalla por píxel de diseño de la capa que se edita.
    let scale = Memo::new(move |_| {
        if state.in_modal() {
            modal_scale.get()
        } else {
            frame.get().k
        }
    });
    // Ancho de diseño de la capa que se edita.
    let design_w = Memo::new(move |_| {
        if state.in_modal() {
            f64::from(Layer::ProductModal.design_width())
        } else {
            frame.get().w
        }
    });
    // Producto con el que se muestra la ficha mientras se edita.
    let modal_product = Memo::new(move |_| {
        let chosen = ui.modal_product.get();
        products.with_value(|ps| {
            chosen
                .and_then(|id| ps.iter().find(|p| p.id == id).cloned())
                .or_else(|| ps.first().cloned())
                .unwrap_or_else(sample_product)
        })
    });

    // Mide el lienzo (para la página) y el cuadro de la ficha (para la ficha).
    let observer_slot = std::rc::Rc::new(std::cell::RefCell::new(None::<web_sys::ResizeObserver>));
    Effect::new(move |_| {
        let section = section_ref.get();
        let stage = stage_ref.get();
        let measure = move || {
            if let Some(section) = section_ref.get_untracked() {
                let w = f64::from(section.client_width());
                if w > 0.0 && (canvas_w.get_untracked() - w).abs() > 0.5 {
                    canvas_w.set(w);
                }
                // La vista previa ocupa todo el ancho, menos la barra de
                // desplazamiento del lienzo.
                let bar = f64::from(section.offset_width() - section.client_width());
                if let Some(root) = document().document_element() {
                    let sw = f64::from(root.client_width()) - bar;
                    if sw > 0.0 && (screen_w.get_untracked() - sw).abs() > 0.5 {
                        screen_w.set(sw);
                    }
                }
            }
            if state.in_modal_untracked() {
                if let Some(stage) = stage_ref.get_untracked() {
                    let dw = f64::from(Layer::ProductModal.design_width());
                    let s = (f64::from(stage.client_width()) / dw).clamp(0.3, 1.0);
                    if (modal_scale.get_untracked() - s).abs() > 0.001 {
                        modal_scale.set(s);
                    }
                }
            }
        };
        measure();
        let cb = Closure::<dyn FnMut()>::new(measure);
        if let Ok(observer) = web_sys::ResizeObserver::new(cb.as_ref().unchecked_ref()) {
            if let Some(section) = section {
                observer.observe(&section);
            }
            if let Some(stage) = stage {
                observer.observe(&stage);
            }
            // Al volver a medir (cambió el cuadro), el observador anterior sobra.
            if let Some(old) = observer_slot.borrow_mut().replace(observer) {
                old.disconnect();
            }
        }
        cb.forget();
    });

    // Termina el arrastre de un bloque del lienzo sin aplicarlo: el bloque
    // vuelve a su lugar (el CSS lo anima).
    let cancel_drag = move |announce: bool| {
        let Some(d) = drag.get_untracked() else {
            return;
        };
        drag.set(None);
        ui.block_drag.set_value(false);
        if let Some(grid_el) = grid_ref.get_untracked() {
            if grid_el.has_pointer_capture(d.pointer_id) {
                _ = grid_el.release_pointer_capture(d.pointer_id);
            }
        }
        let changed = preview.get_untracked().is_some();
        preview.set(None);
        drag_pos.set(None);
        if announce && changed {
            ui.announce(match d.kind {
                DragKind::Move { .. } => "Movimiento cancelado. El bloque volvió a su lugar.",
                DragKind::Resize { .. } => "Cambio de tamaño cancelado.",
            });
        }
    };

    // Arrastre desde la paleta: el puntero no está capturado por nadie, así
    // que se sigue desde la ventana.
    let cancel_new_drag = move || {
        ui.new_drag.set(None);
        incoming.set(None);
        preview.set(None);
    };
    Effect::new(move |_| {
        let on_move = window_event_listener(leptos::ev::pointermove, move |ev| {
            let Some(mut nd) = ui.new_drag.get_untracked() else {
                return;
            };
            let (x, y) = (f64::from(ev.client_x()), f64::from(ev.client_y()));
            nd.x = x;
            nd.y = y;
            if !nd.active && (x - nd.start_x).hypot(y - nd.start_y) < NewDrag::THRESHOLD_PX {
                return;
            }
            ev.prevent_default();
            if !nd.active {
                nd.active = true;
                let Some(widget) = Widget::defaults(nd.type_name) else {
                    cancel_new_drag();
                    return;
                };
                let id = state.doc.with_untracked(new_node_id);
                incoming.set(Some(Node::new(id, widget)));
                state.selected.set(None);
            }
            ui.new_drag.set(Some(nd));
            let (Some(section), Some(grid_el), Some(node)) = (
                section_ref.get_untracked(),
                grid_ref.get_untracked(),
                incoming.get_untracked(),
            ) else {
                return;
            };
            autoscroll(&section, y);
            // Se puede soltar en cualquier parte del lienzo; fuera de él, no.
            let s = section.get_bounding_client_rect();
            let inside = x >= s.left() && x <= s.right() && y >= s.top() && y <= s.bottom();
            if !inside {
                if preview.with_untracked(Option::is_some) {
                    preview.set(None);
                }
                return;
            }
            let g = grid_el.get_bounding_client_rect();
            let k = scale_of(&grid_el, &g);
            let (cx, cy) = drop_cell(x, y, &g, k, node.size.w);
            let mut nodes = state.nodes_untracked();
            let (cx, cy) = grid::drop_target(&nodes, node.size.w, node.size.h, cx, cy);
            grid::insert_at(&mut nodes, node, cx, cy);
            if preview.with_untracked(|p| p.as_ref() != Some(&nodes)) {
                preview.set(Some(nodes));
            }
        });
        let on_up = window_event_listener(leptos::ev::pointerup, move |_| {
            let Some(nd) = ui.new_drag.get_untracked() else {
                return;
            };
            if !nd.active {
                // Fue un clic: lo resuelve el botón de la paleta.
                ui.new_drag.set(None);
                return;
            }
            ui.swallow_click.set_value(true);
            let target = preview.get_untracked().zip(incoming.get_untracked());
            cancel_new_drag();
            let Some((nodes, node)) = target else {
                return;
            };
            if let Some(n) = nodes.iter().find(|n| n.id == node.id) {
                let (x, y) = (i32::from(n.pos.x), i32::from(n.pos.y));
                actions.add_at(node, x, y);
            }
        });
        let on_cancel = window_event_listener(leptos::ev::pointercancel, move |_| {
            if ui.new_drag.with_untracked(Option::is_some) {
                cancel_new_drag();
            }
        });
        let on_key = window_event_listener(leptos::ev::keydown, move |ev| {
            if ev.key() == "Escape" && drag.with_untracked(Option::is_some) {
                ev.prevent_default();
                cancel_drag(true);
                return;
            }
            if ev.key() == "Escape" && ui.new_drag.with_untracked(Option::is_some) {
                ev.prevent_default();
                ui.swallow_click.set_value(true);
                cancel_new_drag();
                ui.announce("Arrastre cancelado.");
            }
        });
        on_cleanup(move || {
            on_move.remove();
            on_up.remove();
            on_cancel.remove();
            on_key.remove();
        });
    });

    let on_pointer_down = move |ev: PointerEvent, id: String| {
        if ev.button() != 0 || drag.with_untracked(Option::is_some) {
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
        let (x, y) = (f64::from(ev.client_x()), f64::from(ev.client_y()));
        drag.set(Some(Drag {
            id,
            kind: DragKind::Move {
                offset_x: x - rect.left(),
                offset_y: y - rect.top(),
                start_x: x,
                start_y: y,
            },
            moved: false,
            pointer_id: ev.pointer_id(),
        }));
        ui.block_drag.set_value(true);
    };

    let on_resize_down = move |ev: PointerEvent, id: String, edge: Edge| {
        if ev.button() != 0 || drag.with_untracked(Option::is_some) {
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
            pointer_id: ev.pointer_id(),
        }));
        ui.block_drag.set_value(true);
    };

    let on_pointer_move = move |ev: PointerEvent| {
        let Some(d) = drag
            .get_untracked()
            .filter(|d| d.pointer_id == ev.pointer_id())
        else {
            return;
        };
        let Some(grid_el) = grid_ref.get_untracked() else {
            return;
        };
        if let Some(section) = section_ref.get_untracked() {
            autoscroll(&section, f64::from(ev.client_y()));
        }
        let g = grid_el.get_bounding_client_rect();
        let k = scale_of(&grid_el, &g);
        let layout_w = g.width() / k;
        let mut nodes = state.nodes_untracked();
        match d.kind {
            DragKind::Move {
                offset_x,
                offset_y,
                start_x,
                start_y,
            } => {
                let (cx, cy) = (f64::from(ev.client_x()), f64::from(ev.client_y()));
                // Hasta moverse unos píxeles es un clic, no un arrastre.
                if !d.moved && (cx - start_x).hypot(cy - start_y) < NewDrag::THRESHOLD_PX {
                    return;
                }
                let px = (cx - g.left() - offset_x) / k;
                let py = (cy - g.top() - offset_y) / k;
                drag_pos.set(Some((px, py)));
                let (x, y) = grid::cell_from_point(px, py, layout_w);
                grid::move_node(&mut nodes, &d.id, x, y);
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
                let pitch = (grid::col_pitch(layout_w) * k, grid::ROW_PX * k);
                let (w, h) = resized(edge, start_w, start_h, (dx, dy), pitch);
                // Al tirar del borde derecho el nodo no se corre: el ancho topa en la última columna.
                let x = nodes
                    .iter()
                    .find(|n| n.id == d.id)
                    .map_or(0, |n| i32::from(n.pos.x));
                grid::resize_node(&mut nodes, &d.id, w.min(i32::from(GRID_COLS) - x), h);
            }
        }
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
    };

    let on_pointer_up = move |ev: PointerEvent| {
        let Some(d) = drag
            .get_untracked()
            .filter(|d| d.pointer_id == ev.pointer_id())
        else {
            return;
        };
        // Primero se termina el arrastre: soltar la captura dispara
        // `lostpointercapture`, que así ya no encuentra nada que cancelar.
        drag.set(None);
        ui.block_drag.set_value(false);
        if let Some(grid_el) = grid_ref.get_untracked() {
            _ = grid_el.release_pointer_capture(ev.pointer_id());
        }
        if let Some(nodes) = preview.get_untracked() {
            // Los bloques de la capa que se edita (la página o la ficha).
            state.apply_nodes(|layer_nodes| *layer_nodes = nodes);
            ui.announce(actions.position_text(&d.id));
        }
        preview.set(None);
        drag_pos.set(None);
        // Un clic (sin arrastrar) sobre un producto de la página abre su ficha.
        if !d.moved && matches!(d.kind, DragKind::Move { .. }) {
            if let Some(pid) = product_under(&d.id, ev.client_x(), ev.client_y()) {
                actions.open_modal(Some(pid));
                return;
            }
            // Uno sobre un control del bloque (las flechas del carrusel, una
            // pestaña, una pregunta…) lo usa, como en la tienda.
            if let Some(control) = control_under(&d.id, ev.client_x(), ev.client_y()) {
                control.click();
                return;
            }
            // Y uno sobre un enlace (del menú, un botón…) lleva a esa página.
            if let Some((href, label)) = link_under(&d.id, ev.client_x(), ev.client_y()) {
                actions.follow_link(&href, &label);
                return;
            }
        }
        crate::reveal_node(&d.id, d.moved && matches!(d.kind, DragKind::Move { .. }));
    };

    // Lo que se dibuja: la vista previa del arrastre o el documento.
    let shown = Memo::new(move |_| preview.get().unwrap_or_else(|| state.nodes()));
    // Los ids, en orden de lectura del documento. Durante un arrastre no
    // cambian de lugar en el HTML, así no se reinician las animaciones.
    let order_ids = Memo::new(move |_| {
        let mut ids: Vec<String> = {
            let mut nodes = state.nodes();
            grid::sort_by_position(&mut nodes);
            nodes.into_iter().map(|n| n.id).collect()
        };
        if let Some(new) = incoming.with(|n| n.as_ref().map(|n| n.id.clone())) {
            if shown.with(|nodes| nodes.iter().any(|n| n.id == new)) && !ids.contains(&new) {
                ids.push(new);
            }
        }
        ids
    });
    let rows = Memo::new(move |_| {
        shown.with(|nodes| {
            nodes
                .iter()
                .map(|n| u32::from(n.pos.y) + u32::from(n.size.h))
                .max()
                .unwrap_or(0)
                .max(if state.in_modal() {
                    MIN_MODAL_ROWS
                } else {
                    MIN_ROWS
                })
        })
    });
    // Lugar donde va a quedar el bloque que se mueve con el mouse.
    let slot = Memo::new(move |_| {
        let d = drag.get()?;
        if !(d.moved && matches!(d.kind, DragKind::Move { .. })) {
            return None;
        }
        shown.with(|nodes| nodes.iter().find(|n| n.id == d.id).cloned())
    });

    let close_welcome = move |_| {
        welcome.set(false);
        if let Some(st) = crate::storage() {
            _ = st.set_item(WELCOME_KEY, "1");
        }
    };

    // Un bloque del lienzo. El elemento se conserva mientras exista el bloque:
    // al cambiar de lugar o de tamaño solo cambia su estilo, y el CSS lo anima.
    let node_view = move |id: String| {
        let node = Memo::new({
            let id = id.clone();
            move |_| shown.with(|nodes| nodes.iter().find(|n| n.id == id).cloned())
        });
        // Solo lo que cambia el dibujo del bloque: moverlo no lo redibuja.
        let widget = Memo::new(move |_| node.get().map(|n| (n.widget, n.style, n.look)));
        let is_selected = Memo::new({
            let id = id.clone();
            move |_| state.selected.get().as_deref() == Some(id.as_str())
        });
        let is_incoming = Memo::new({
            let id = id.clone();
            move |_| incoming.with(|n| n.as_ref().is_some_and(|n| n.id == id))
        });
        let is_moving = Memo::new({
            let id = id.clone();
            move |_| {
                drag.with(|d| {
                    d.as_ref().is_some_and(|d| {
                        d.id == id && d.moved && matches!(d.kind, DragKind::Move { .. })
                    })
                })
            }
        });
        let is_resizing = Memo::new({
            let id = id.clone();
            move |_| {
                drag.with(|d| {
                    d.as_ref()
                        .is_some_and(|d| d.id == id && matches!(d.kind, DragKind::Resize { .. }))
                })
            }
        });

        let style = move || {
            let Some(n) = node.get() else {
                return String::new();
            };
            let cols = f64::from(GRID_COLS);
            let w = f64::from(n.size.w) * 100.0 / cols;
            let h = u32::from(n.size.h) * u32::from(ROW_PX);
            match drag_pos.get().filter(|_| is_moving.get()) {
                Some((x, y)) => format!("left:{x:.1}px; top:{y:.1}px; width:{w:.4}%; height:{h}px"),
                None => format!(
                    "left:{:.4}%; top:{}px; width:{w:.4}%; height:{h}px",
                    f64::from(n.pos.x) * 100.0 / cols,
                    u32::from(n.pos.y) * u32::from(ROW_PX)
                ),
            }
        };
        let aria_label = move || {
            node.get()
                .map(|n| format!("{}, {}", n.widget.label(), size_text(n.size)))
                .unwrap_or_default()
        };
        let content = {
            let id = id.clone();
            move || {
                widget.get().map(|(w, style, look)| {
                    let ctx = RenderCtx {
                        products: products.get_value(),
                        editing: true,
                        product: (state.layer.get_untracked() == Layer::ProductModal)
                            .then(|| modal_product.get()),
                    };
                    let mut node = Node::new(id.clone(), w);
                    node.style = style;
                    node.look = look;
                    render_widget(&node, &ctx)
                })
            }
        };
        let label = move || node.get().map_or("Bloque", |n| n.widget.label());
        // Menú del bloque: un botón ⋯ en su esquina superior derecha que
        // despliega las acciones. El menú es un `popover` (capa superior), así
        // no lo recorta el bloque ni lo achica la escala del lienzo.
        let toolbar = {
            let id = id.clone();
            move || {
                (is_selected.get() && !is_moving.get() && !is_resizing.get()).then(|| {
                    let (up, down, dup, del) = (id.clone(), id.clone(), id.clone(), id.clone());
                    let has_modal = node.get_untracked().is_some_and(|n| {
                        matches!(n.widget, Widget::ProductGrid(_) | Widget::TagTabs(_) | Widget::FeaturedProduct(_))
                    });
                    let menu_id = format!("mk-menu-{id}");
                    let menu_ref = NodeRef::<Div>::new();
                    // Cierra el menú y después hace la acción.
                    let run = move |f: Box<dyn FnOnce()>| {
                        if let Some(menu) = menu_ref.get_untracked() {
                            _ = menu.hide_popover();
                        }
                        f();
                    };
                    // Flechas, Inicio y Fin pasan de una opción a otra.
                    let active = RwSignal::new(0_usize);
                    let tab = move |i: usize| move || if active.get() == i { "0" } else { "-1" };
                    let item = "flex w-full items-center gap-3 rounded-field px-3 py-2 text-left text-sm hover:bg-base-200 focus-visible:bg-base-200 focus-visible:outline-none";
                    view! {
                        <button
                            type="button"
                            data-theme="light"
                            class="mk-menu-trigger btn btn-sm btn-square absolute right-2 top-2 z-20 border-base-300 bg-base-100 text-base-content shadow-md"
                            popovertarget=menu_id.clone()
                            // Mismo tamaño en pantalla con cualquier escala del lienzo.
                            style=move || format!("transform-origin:top right; transform:scale({:.4})", 1.0 / scale.get().max(0.01))
                            aria-label=move || format!("Opciones de {}", label())
                            title="Opciones del bloque"
                            on:pointerdown=|ev: PointerEvent| ev.stop_propagation()
                            on:click=move |ev: leptos::ev::MouseEvent| {
                                // Lugar del menú para los navegadores sin anchor positioning (ver CSS).
                                if let (Some(btn), Some(menu)) = (
                                    ev.current_target().and_then(|t| t.dyn_into::<web_sys::Element>().ok()),
                                    menu_ref.get_untracked(),
                                ) {
                                    let r = btn.get_bounding_client_rect();
                                    let style = web_sys::HtmlElement::style(&menu);
                                    _ = style.set_property("--mk-menu-top", &format!("{:.0}px", r.bottom() + 4.0));
                                    _ = style.set_property("--mk-menu-right", &format!("{:.0}px", window().inner_width().ok().and_then(|w| w.as_f64()).unwrap_or(0.0) - r.right()));
                                }
                            }
                        >
                            <Icon name="more"/>
                        </button>
                        <div
                            node_ref=menu_ref
                            id=menu_id
                            popover="auto"
                            data-theme="light"
                            class="mk-menu w-56 rounded-box border border-base-300 bg-base-100 p-1 text-base-content shadow-lg"
                            aria-label=move || format!("Opciones de {}", label())
                            on:pointerdown=|ev: PointerEvent| ev.stop_propagation()
                            on:keydown=move |ev: KeyboardEvent| on_toolbar_key(ev, active)
                        >
                            <p class="px-3 pb-1 pt-2 text-xs text-base-content/70">
                                {move || node.get().map(|n| size_badge(n.size))}
                            </p>
                            <button type="button" class=item autofocus=true tabindex=tab(0) on:focus=move |_| active.set(0) aria-keyshortcuts="ArrowUp" on:click=move |_| { let id = up.clone(); run(Box::new(move || actions.move_up(&id))) }>
                                <Icon name="up"/>"Subir"
                            </button>
                            <button type="button" class=item tabindex=tab(1) on:focus=move |_| active.set(1) aria-keyshortcuts="ArrowDown" on:click=move |_| { let id = down.clone(); run(Box::new(move || actions.move_down(&id))) }>
                                <Icon name="down"/>"Bajar"
                            </button>
                            <button type="button" class=item tabindex=tab(2) on:focus=move |_| active.set(2) on:click=move |_| { let id = dup.clone(); run(Box::new(move || actions.duplicate(&id))) }>
                                <Icon name="copy"/>"Duplicar"
                            </button>
                            {has_modal.then(|| view! {
                                <button type="button" class=item tabindex=tab(3) on:focus=move |_| active.set(3) on:click=move |_| run(Box::new(move || actions.open_modal(None)))>
                                    <Icon name="modal"/>"Diseñar la ficha del producto"
                                </button>
                            })}
                            <div class="my-1 border-t border-base-300" role="separator"></div>
                            <button type="button" class=format!("{item} text-red-700") tabindex=tab(if has_modal { 4 } else { 3 }) on:focus=move |_| active.set(if has_modal { 4 } else { 3 }) aria-keyshortcuts="Delete" on:click=move |_| { let id = del.clone(); run(Box::new(move || actions.delete(&id))) }>
                                <Icon name="trash"/>"Borrar"
                            </button>
                        </div>
                    }
                })
            }
        };
        let badge = move || {
            (is_moving.get() || is_resizing.get() || is_incoming.get()).then(|| view! {
                <span class="mk-size-badge badge badge-primary badge-sm pointer-events-none absolute right-2 top-2 z-10" aria-hidden="true">
                    {move || node.get().map(|n| size_badge(n.size))}
                </span>
            })
        };
        let handles = {
            let id = id.clone();
            move || {
                is_selected.get().then(|| {
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
                })
            }
        };
        let (key_id, down_id, attr_id) = (id.clone(), id.clone(), id.clone());
        view! {
            <div
                class="mk-anode"
                class:mk-moving=move || is_moving.get()
                class:mk-resizing=move || is_resizing.get()
                class:mk-incoming=move || is_incoming.get()
                class:mk-selected=move || is_selected.get()
                style=style
                data-node-id=attr_id
                tabindex=move || if is_incoming.get() { "-1" } else { "0" }
                role="group"
                aria-roledescription="bloque"
                aria-label=aria_label
                aria-describedby="mk-node-help"
                aria-current=move || is_selected.get().then_some("true")
                aria-hidden=move || is_incoming.get().then_some("true")
                on:pointerdown=move |ev| {
                    if !is_incoming.get_untracked() {
                        on_pointer_down(ev, down_id.clone());
                    }
                }
                on:keydown=move |ev| on_node_key(ev, &key_id, actions)
            >
                <div class="mk-anode-box">
                    <div class="mk-content h-full">{content}</div>
                    {toolbar}
                    {badge}
                    {handles}
                </div>
            </div>
        }
    };

    // La grilla interactiva de la capa que se edita.
    let stage = move || {
        view! {
            <div
                node_ref=stage_ref
                class="mk-stage"
                class:mk-dropzone=move || incoming.with(Option::is_some)
                style=move || format!("height:{:.1}px", (f64::from(rows.get() * u32::from(ROW_PX)) - 2.0 * MARGIN_PX) * scale.get())
            >
                <div
                    node_ref=grid_ref
                    class="mk-agrid select-none"
                    class:mk-grid-guides=move || drag.with(Option::is_some) || incoming.with(Option::is_some)
                    style=move || {
                        let k = scale.get();
                        format!(
                            "width:{}px; height:{}px; transform:translate({:.2}px,{:.2}px) scale({k:.4})",
                            design_w.get() + 2.0 * MARGIN_PX,
                            rows.get() * u32::from(ROW_PX),
                            -MARGIN_PX * k,
                            -MARGIN_PX * k,
                        )
                    }
                    on:pointermove=on_pointer_move
                    on:pointerup=on_pointer_up
                    on:dblclick=move |ev: leptos::ev::MouseEvent| {
                        // Doble clic sobre un texto del bloque: se escribe ahí mismo.
                        if inline_edit.with_untracked(Option::is_some) {
                            return;
                        }
                        let Some(stage) = stage_ref.get_untracked() else {
                            return;
                        };
                        let found = state
                            .doc
                            .with_untracked(|d| inline_at(d, &stage, ev.client_x(), ev.client_y()));
                        if let Some(e) = found {
                            ev.prevent_default();
                            state.selected.set(Some(e.node_id.clone()));
                            inline_edit.set(Some(e));
                        }
                    }
                    on:pointercancel=move |ev: PointerEvent| {
                        if drag.with_untracked(|d| d.as_ref().is_some_and(|d| d.pointer_id == ev.pointer_id())) {
                            cancel_drag(false);
                        }
                    }
                    on:lostpointercapture=move |ev: PointerEvent| {
                        if drag.with_untracked(|d| d.as_ref().is_some_and(|d| d.pointer_id == ev.pointer_id())) {
                            cancel_drag(false);
                        }
                    }
                >
                    {move || slot.get().map(|_| view! {
                        <div class="mk-slot" style=move || {
                            slot.get()
                                .map(|n| format!(
                                    "left:{:.4}%; top:{}px; width:{:.4}%; height:{}px",
                                    f64::from(n.pos.x) * 100.0 / f64::from(GRID_COLS),
                                    u32::from(n.pos.y) * u32::from(ROW_PX),
                                    f64::from(n.size.w) * 100.0 / f64::from(GRID_COLS),
                                    u32::from(n.size.h) * u32::from(ROW_PX),
                                ))
                                .unwrap_or_default()
                        } aria-hidden="true"><div class="mk-slot-box"></div></div>
                    })}
                    <For each=move || order_ids.get() key=|id| id.clone() children=node_view/>
                </div>
                {move || inline_edit.with(Option::is_some).then(|| view! { <InlineEditor edit=inline_edit/> })}
            </div>
        }
    };
    // La página como la ve la tienda, sin edición: para la vista previa y
    // para el fondo oscurecido mientras se edita la ficha.
    let static_page =
        move || {
            let ctx = RenderCtx {
                products: products.get_value(),
                editing: true,
                product: None,
            };
            let mut nodes = state.doc.with(|d| d.layer(state.page_layer()).clone());
            grid::sort_by_position(&mut nodes);
            nodes
            .into_iter()
            .map(|node| view! {
                <div class="mk-node" style=node_style(&node)>{render_widget(&node, &ctx)}</div>
            })
            .collect_view()
        };
    // Bordes de la página: se arrastran (o se mueven con las flechas) para
    // cambiar el ancho del contenido de toda la tienda.
    let finish_width_drag = move |apply: bool| {
        let Some(d) = width_drag.get_untracked() else {
            return;
        };
        width_drag.set(None);
        let Some(w) = d.width.filter(|_| apply) else {
            return;
        };
        if state.doc.with_untracked(|doc| doc.layout.max_width != w) {
            state.apply(|doc| doc.layout.max_width = w);
            ui.announce(format!("Ancho de la tienda: {w} px."));
        }
    };
    let width_handle = move |side: f64| {
        let place = move || {
            let f = frame.get();
            let (edge, pad) = if side < 0.0 {
                ("left", f.left)
            } else {
                ("right", f.right)
            };
            format!(
                "{edge}:{:.1}px; top:{:.1}px; bottom:{:.1}px",
                pad * f.k - 12.0,
                f.top * f.k,
                f.bottom * f.k
            )
        };
        let dragging = move || width_drag.with(|d| d.is_some_and(|d| d.side == side));
        let now = move || {
            let w = frame.get().w;
            let full = state.doc.with(|d| d.layout.is_full())
                && width_drag.with(|d| d.and_then(|d| d.width).is_none());
            if full {
                "Ancho completo".to_string()
            } else {
                format!("{} px", w.round() as u32)
            }
        };
        view! {
            <div
                class="mk-width-handle"
                class:is-dragging=dragging
                class:is-left=side < 0.0
                style=place
                role="separator"
                aria-orientation="vertical"
                tabindex="0"
                aria-label=if side < 0.0 { "Borde izquierdo de la tienda" } else { "Borde derecho de la tienda" }
                aria-valuemin=domain::MIN_WIDTH
                aria-valuemax=domain::MAX_WIDTH
                aria-valuenow=move || frame.get().w.round() as u32
                aria-valuetext=now
                title="Arrastrá para cambiar el ancho de toda la tienda"
                on:pointerdown=move |ev: PointerEvent| {
                    if ev.button() != 0 || width_drag.with_untracked(Option::is_some) {
                        return;
                    }
                    ev.prevent_default();
                    if let Some(el) = ev.current_target().and_then(|t| t.dyn_into::<web_sys::HtmlElement>().ok()) {
                        _ = el.set_pointer_capture(ev.pointer_id());
                        _ = el.focus();
                    }
                    let Frame { w, k, .. } = frame.get_untracked();
                    width_drag.set(Some(WidthDrag {
                        side,
                        start_x: f64::from(ev.client_x()),
                        start_w: w,
                        scale: k,
                        width: None,
                        pointer_id: ev.pointer_id(),
                    }));
                }
                on:pointermove=move |ev: PointerEvent| {
                    let Some(d) = width_drag.get_untracked().filter(|d| d.pointer_id == ev.pointer_id()) else {
                        return;
                    };
                    let dx = f64::from(ev.client_x()) - d.start_x;
                    if d.width.is_none() && dx.abs() < NewDrag::THRESHOLD_PX {
                        return;
                    }
                    let w = dragged_width(d.start_w, d.side, dx, d.scale);
                    if d.width != Some(w) {
                        width_drag.set(Some(WidthDrag { width: Some(w), ..d }));
                    }
                }
                on:pointerup=move |ev: PointerEvent| {
                    if width_drag.with_untracked(|d| d.is_some_and(|d| d.pointer_id == ev.pointer_id())) {
                        finish_width_drag(true);
                    }
                }
                on:lostpointercapture=move |_| finish_width_drag(false)
                on:keydown=move |ev: KeyboardEvent| {
                    let grow = match ev.key().as_str() {
                        "Escape" if width_drag.with_untracked(Option::is_some) => {
                            ev.prevent_default();
                            finish_width_drag(false);
                            ui.announce("Cambio de ancho cancelado.");
                            return;
                        }
                        "ArrowRight" => side,
                        "ArrowLeft" => -side,
                        _ => return,
                    };
                    ev.prevent_default();
                    let w = frame.get_untracked().w;
                    let w = domain::snap_width(w + grow * f64::from(domain::WIDTH_STEP));
                    state.apply_grouped(Some("layout-width"), |d| d.layout.max_width = w);
                    ui.announce(format!("Ancho de la tienda: {w} px."));
                }
            >
                <span class="mk-width-grip" aria-hidden="true">
                    <span class="mk-width-label">{now}</span>
                </span>
            </div>
        }
    };
    // Lo que mide la página en el lienzo: el ancho, el margen y la escala de
    // la tienda. Centrada, también mientras se arrastra un borde.
    let frame_style = move || {
        let f = frame.get();
        let total = f.total() * f.k;
        format!(
            "width:{total:.1}px; padding:{:.2}px {:.2}px {:.2}px {:.2}px; margin-left:calc(50% - {:.1}px)",
            f.top * f.k,
            f.right * f.k,
            f.bottom * f.k,
            f.left * f.k,
            total / 2.0
        )
    };

    let product_choices = move || products.get_value();
    // En la vista previa, tocar un enlace muestra esa página, como en la tienda.
    let on_preview_click = move |ev: leptos::ev::MouseEvent| {
        let Some(el) = ev
            .target()
            .and_then(|t| t.dyn_into::<web_sys::Element>().ok())
            .and_then(|t| t.closest("[data-href]").ok().flatten())
        else {
            return;
        };
        let href = el.get_attribute("data-href").unwrap_or_default();
        match state.doc.with_untracked(|d| d.page_index(&href)) {
            Some(i) => {
                state.page.set(i);
                state.layer.set(domain::Layer::Page(i));
                state.selected.set(None);
                if let Some(el) = document().get_element_by_id("mk-canvas") {
                    el.set_scroll_top(0);
                }
            }
            None if domain::normalize_path(&href).is_none() => ui.toast(
                "Este enlace lleva a otro sitio. En tu tienda se abre al tocarlo.",
                false,
            ),
            None => ui.toast(
                "Esa página todavía no existe. Volvé a editar y tocá el enlace para crearla.",
                false,
            ),
        }
    };
    view! {
        <section id="mk-canvas" node_ref=section_ref class="min-w-0 flex-1 overflow-auto" aria-label="Lienzo de la página">
            <p id="mk-node-help" class="sr-only">
                "Enter para editar. Doble clic sobre un texto para cambiarlo ahí mismo. Flechas arriba y abajo para cambiar el orden, izquierda y derecha para correrlo. Mayúscula más flechas para cambiar el tamaño. Suprimir para borrar."
            </p>
            {move || ui.preview.get().then(|| view! {
                <div class="sticky top-0 z-30 flex items-center gap-3 border-b border-base-300 bg-base-100 px-4 py-2 text-sm">
                    <Icon name="eye" class="h-4 w-4"/>
                    <span class="flex-1">"Vista previa: así se va a ver tu tienda cuando publiques. Tocá un producto para ver su ficha."</span>
                    <button type="button" class="btn btn-sm btn-primary" on:click=move |_| ui.preview.set(false)>"Volver a editar"</button>
                </div>
            })}
            {move || (state.in_modal() && !ui.preview.get()).then(|| view! {
                <div class="sticky top-0 z-30 flex items-center gap-3 border-b border-primary/30 bg-base-100 px-4 py-2 text-sm shadow-sm" role="region" aria-labelledby="mk-modal-title">
                    <span class="rounded-field bg-primary/10 p-1.5 text-primary" aria-hidden="true"><Icon name="modal" class="h-4 w-4"/></span>
                    <div class="min-w-0 flex-1">
                        <h2 id="mk-modal-title" tabindex="-1" class="font-semibold focus:outline-none">"Ficha del producto"</h2>
                        <p class="text-xs text-base-content/75">"Se abre al tocar un producto en tu tienda. Es la misma para todos tus productos."</p>
                    </div>
                    {(!product_choices().is_empty()).then(|| view! {
                        <label class="flex items-center gap-2">
                            <span class="whitespace-nowrap">"Ver con"</span>
                            <select
                                class="select select-sm w-48"
                                prop:value=move || modal_product.with(|p| p.id.clone())
                                on:change=move |ev| ui.modal_product.set(Some(event_target_value(&ev)))
                            >
                                {product_choices()
                                    .into_iter()
                                    .map(|p| view! { <option value=p.id.clone()>{p.name}</option> })
                                    .collect_view()}
                            </select>
                        </label>
                    })}
                    <button type="button" class="btn btn-sm btn-primary" title="Volver a la página (Esc)" aria-keyshortcuts="Escape" on:click=move |_| actions.close_modal()>"Volver a la página"</button>
                </div>
            })}
            <div
                data-theme=move || state.doc.with(|d| d.theme.clone())
                class="mk-store relative min-h-full bg-base-200 text-base-content"
                style=move || state.doc.with(|d| format!("{}; {}", d.typography.css_vars(), d.style.css_vars()))
            >
                {move || (state.in_modal() && !ui.preview.get()).then(|| view! {
                    // Tocar el fondo oscurecido vuelve a la página, como en la tienda.
                    <div
                        class="absolute inset-0 z-[5] cursor-pointer bg-black/50"
                        title="Volver a la página"
                        aria-hidden="true"
                        on:pointerdown=move |ev: PointerEvent| {
                            if ev.button() == 0 && ui.new_drag.with_untracked(Option::is_none) {
                                actions.close_modal();
                            }
                        }
                    ></div>
                })}
                <div
                    class="mk-page-frame relative"
                    class:is-dragging=move || width_drag.with(|d| d.is_some_and(|d| d.width.is_some()))
                    style=frame_style
                >
                    {move || (!ui.preview.get() && !state.in_modal()).then(|| view! {
                        <div class="mk-frame-outline" aria-hidden="true" style=move || {
                            let f = frame.get();
                            format!(
                                "top:{:.1}px; right:{:.1}px; bottom:{:.1}px; left:{:.1}px",
                                f.top * f.k,
                                f.right * f.k,
                                f.bottom * f.k,
                                f.left * f.k
                            )
                        }></div>
                        {width_handle(-1.0)}
                        {width_handle(1.0)}
                    })}
                    {move || (welcome.get() && !ui.preview.get() && !state.in_modal()).then(|| view! {
                        <div data-theme="light" class="mb-4 rounded-box border border-primary/30 bg-base-100 p-4 text-base-content shadow-sm" role="region" aria-labelledby="mk-welcome-title">
                            <div class="flex items-start gap-3">
                                <div class="flex-1">
                                    <h2 id="mk-welcome-title" class="font-semibold">"Primeros pasos"</h2>
                                    <ol class="mt-2 list-decimal space-y-1 pl-5 text-sm">
                                        <li>"Elegí un bloque de la izquierda: con un clic se agrega al final, o arrastralo hasta donde lo quieras."</li>
                                        <li>"Hacé clic en un bloque para cambiar sus textos e imágenes a la derecha. Arrastralo para moverlo, o usá los botones Subir y Bajar."</li>
                                        <li>"Tocá Publicar para que tu tienda muestre los cambios. Si te equivocás, Deshacer vuelve atrás."</li>
                                    </ol>
                                </div>
                                <button type="button" class="btn btn-sm btn-primary" on:click=close_welcome>"Entendido"</button>
                            </div>
                        </div>
                    })}
                    {move || {
                        if ui.preview.get() {
                            // La vista previa usa la misma grilla que la tienda, al ancho real,
                            // y abre la ficha al tocar un producto, como la tienda.
                            let open = RwSignal::new(None::<String>);
                            provide_context(OpenProduct(open));
                            widgets::ProductFilter::provide();
                            let modal = state.doc.with_untracked(|d| d.product_modal.clone());
                            return view! {
                                <div class="mk-grid min-h-96" on:click=on_preview_click>{static_page()}</div>
                                <ProductDialog modal products=products.get_value() open editing=true/>
                            }
                            .into_any();
                        }
                        if state.layer.get() == Layer::ProductModal {
                            return view! {
                                <div class="grid">
                                    <div class="mk-grid [grid-area:1/1]" inert=true aria-hidden="true">{static_page()}</div>
                                    // El cuadro usa el tema y las letras de la tienda, como en la tienda.
                                    <div
                                        class="mk-modal-panel relative z-10 mx-auto mb-10 mt-6 self-start rounded-box bg-base-100 shadow-2xl [grid-area:1/1]"
                                        role="region"
                                        aria-label="Ficha del producto"
                                    >
                                        <button
                                            type="button"
                                            class="btn btn-sm btn-circle btn-ghost absolute right-2 top-2"
                                            aria-label="Cerrar la ficha y volver a la página"
                                            title="Volver a la página (Esc)"
                                            on:click=move |_| actions.close_modal()
                                        >
                                            <Icon name="close"/>
                                        </button>
                                        {stage()}
                                        {move || state.nodes().is_empty().then(|| view! {
                                            <div class="absolute inset-x-4 top-16 flex flex-col items-center gap-2 rounded-box border-2 border-dashed border-base-content/40 bg-base-100 p-8 text-center">
                                                <Icon name="plus" class="h-8 w-8 text-primary"/>
                                                <p class="font-semibold">"La ficha está vacía"</p>
                                                <p class="text-sm text-base-content/75">"Sumá bloques desde la izquierda: la foto, el nombre y el precio del producto, por ejemplo."</p>
                                            </div>
                                        })}
                                    </div>
                                </div>
                            }
                            .into_any();
                        }
                        stage().into_any()
                    }}
                    {move || (!state.in_modal() && state.doc.with(|d| d.layer(state.page_layer()).is_empty())).then(|| view! {
                        <div data-theme="light" class="mx-auto mt-2 flex max-w-md flex-col items-center gap-2 rounded-box border-2 border-dashed border-base-content/40 bg-base-100 p-8 text-center text-base-content">
                            <Icon name="plus" class="h-8 w-8 text-primary"/>
                            <p class="font-semibold">{move || if state.page.get() == 0 { "Tu página está vacía" } else { "Esta página está vacía" }}</p>
                            <p class="text-sm text-base-content/75">"Elegí un bloque de la columna izquierda, o arrastralo hasta aquí, para empezar. Te recomendamos un Menú superior y un Banner."</p>
                        </div>
                    })}
                </div>
            </div>
            {move || ui.new_drag.get().filter(|d| d.active).map(|d| {
                let (icon, _) = crate::palette::block_info(d.type_name);
                view! {
                    <div
                        data-theme="light"
                        class="mk-ghost pointer-events-none fixed z-50 flex flex-col gap-0.5 rounded-field border border-primary bg-base-100 px-3 py-1.5 text-sm font-medium text-base-content shadow-xl"
                        style=format!("left:{}px; top:{}px", d.x + 14.0, d.y + 14.0)
                        aria-hidden="true"
                    >
                        <span class="flex items-center gap-2">
                            <Icon name=icon class="h-4 w-4 text-primary"/>
                            {crate::palette::label_of(d.type_name)}
                        </span>
                        <span class="text-xs font-normal text-base-content/70">
                            {move || if preview.with(Option::is_some) { "Soltá para agregarlo acá" } else { "Llevalo a la página" }}
                        </span>
                    </div>
                }
            })}
        </section>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resize_snaps_to_cells() {
        // Columnas de 20 px de paso y filas de 8 px.
        let pitch = (20.0, 8.0);
        assert_eq!(
            resized(Edge::Right, 16, 24, (130.0, 500.0), pitch),
            (23, 24)
        );
        assert_eq!(
            resized(Edge::Bottom, 16, 24, (500.0, -37.0), pitch),
            (16, 19)
        );
        assert_eq!(
            resized(Edge::Corner, 16, 24, (-45.0, 150.0), pitch),
            (14, 43)
        );
        // Con la grilla achicada a la mitad, el mismo gesto vale el doble de celdas.
        assert_eq!(
            resized(Edge::Corner, 16, 24, (20.0, 8.0), (10.0, 4.0)),
            (18, 26)
        );
    }

    #[test]
    fn page_frame_follows_the_store_layout() {
        let mut l = StoreLayout::default();
        // Lienzo de 900 px en una pantalla de 1500: se dibuja la pantalla
        // entera achicada, con el contenido de 1120 px en el medio.
        let f = page_frame(&l, None, 900.0, 1500.0);
        assert_eq!((f.w, f.left, f.top), (1120.0, 16.0, 16.0));
        assert!((f.k - 900.0 / 1500.0).abs() < 1e-9);
        // En una pantalla más angosta que el ancho máximo, el contenido se
        // achica como en la tienda.
        assert_eq!(page_frame(&l, None, 900.0, 1000.0).w, 968.0);
        // Mientras se arrastra manda el ancho que se prueba (y si no entra
        // en la pantalla, se achica todo).
        let f = page_frame(&l, Some(1600), 900.0, 1500.0);
        assert_eq!(f.w, 1600.0);
        assert!((f.k - 900.0 / 1632.0).abs() < 1e-9);
        l.set_pad(Side::Left, 48);
        l.set_pad(Side::Right, 8);
        l.max_width = 0;
        // Ancho completo: toda la pantalla.
        let f = page_frame(&l, None, 2000.0, 2000.0);
        assert_eq!((f.w, f.left, f.right, f.k), (1944.0, 48.0, 8.0, 1.0));
        assert!((fit_scale(576.0, 1152.0) - 0.5).abs() < 1e-9);
    }

    #[test]
    fn dragging_an_edge_changes_the_width_twice_and_snaps() {
        // El borde derecho 50 px afuera, sin escala: 100 px más, con imán de 40.
        assert_eq!(dragged_width(1120.0, 1.0, 50.0, 1.0), 1240);
        // El izquierdo hacia la derecha achica.
        assert_eq!(dragged_width(1120.0, -1.0, 80.0, 1.0), 960);
        // Con el lienzo a la mitad, cada px de pantalla vale dos de diseño.
        assert_eq!(dragged_width(1120.0, 1.0, 40.0, 0.5), 1280);
        assert_eq!(dragged_width(1120.0, 1.0, -5000.0, 1.0), domain::MIN_WIDTH);
    }
}
