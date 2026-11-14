//! Matemática de la grilla de 12 columnas. Funciones puras, sin Leptos.
//!
//! Después de cualquier operación se cumple siempre: ningún nodo se
//! superpone, todos caben en las 12 columnas y, con [`COMPACT`] activo,
//! ninguno puede subir más.

use domain::{Node, GRID_COLS};

pub const ROW_HEIGHT_PX: f64 = 80.0;
pub const GAP_PX: f64 = 16.0;
/// Con `false` se desactiva la compactación y se permiten huecos verticales.
pub const COMPACT: bool = true;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rect {
    pub x: u8,
    pub y: u16,
    pub w: u8,
    pub h: u8,
}

impl Rect {
    fn bottom(&self) -> u16 {
        self.y + u16::from(self.h)
    }
}

/// Rectángulo que ocupa un nodo. Si el `size` no es válido se usa el
/// primer tamaño del widget.
pub fn rect_of(node: &Node) -> Rect {
    let preset = node.preset().unwrap_or(&node.widget.sizes()[0]);
    Rect {
        x: node.pos.x,
        y: node.pos.y,
        w: preset.w,
        h: preset.h,
    }
}

pub fn collides(a: &Rect, b: &Rect) -> bool {
    a.x < b.x + b.w && b.x < a.x + a.w && a.y < b.bottom() && b.y < a.bottom()
}

/// Agrega el nodo en `x = 0`, debajo del último.
pub fn add_node(nodes: &mut Vec<Node>, mut node: Node) {
    let y = nodes.iter().map(|n| rect_of(n).bottom()).max().unwrap_or(0);
    node.pos.x = 0;
    node.pos.y = y;
    let id = node.id.clone();
    nodes.push(node);
    settle(nodes, &id);
}

/// Mueve el nodo a la celda `(x, y)`, limitada a la grilla, y empuja a los
/// que pise.
pub fn move_node(nodes: &mut Vec<Node>, id: &str, x: i32, y: i32) {
    let Some(node) = nodes.iter_mut().find(|n| n.id == id) else {
        return;
    };
    let w = i32::from(rect_of(node).w);
    node.pos.x = x.clamp(0, i32::from(GRID_COLS) - w) as u8;
    node.pos.y = y.clamp(0, i32::from(u16::MAX)) as u16;
    settle(nodes, id);
}

/// Aplica una nueva opción de tamaño. Si no es válida para el widget no hace nada.
pub fn change_size(nodes: &mut Vec<Node>, id: &str, size: &str) {
    let Some(node) = nodes.iter_mut().find(|n| n.id == id) else {
        return;
    };
    let Some(preset) = node.widget.size(size) else {
        return;
    };
    node.size = preset.id.to_string();
    node.pos.x = node.pos.x.min(GRID_COLS - preset.w);
    settle(nodes, id);
}

pub fn remove_node(nodes: &mut Vec<Node>, id: &str) {
    nodes.retain(|n| n.id != id);
    if COMPACT {
        compact(nodes);
    }
}

/// Sube cada nodo, en orden `(y, x)`, mientras no choque con los ya ubicados.
pub fn compact(nodes: &mut Vec<Node>) {
    sort_by_position(nodes);
    let mut placed: Vec<Rect> = Vec::with_capacity(nodes.len());
    for node in nodes.iter_mut() {
        let mut r = rect_of(node);
        while r.y > 0 {
            let up = Rect { y: r.y - 1, ..r };
            if placed.iter().any(|p| collides(&up, p)) {
                break;
            }
            r = up;
        }
        node.pos.y = r.y;
        placed.push(r);
    }
}

/// Celda que corresponde a un punto en píxeles relativo a la esquina
/// superior izquierda de la grilla.
pub fn cell_from_point(px: f64, py: f64, container_width: f64) -> (i32, i32) {
    let cols = f64::from(GRID_COLS);
    let col_width = ((container_width - GAP_PX * (cols - 1.0)) / cols).max(1.0);
    let x = (px / (col_width + GAP_PX)).floor() as i32;
    let y = (py / (ROW_HEIGHT_PX + GAP_PX)).floor() as i32;
    (x, y)
}

/// Ordena por `(y, x)`, el orden de lectura que usa también el HTML.
pub fn sort_by_position(nodes: &mut [Node]) {
    nodes.sort_by_key(|n| (n.pos.y, n.pos.x));
}

/// Empuja hacia abajo en cascada todo lo que pise el nodo `fixed_id` y
/// después compacta.
fn settle(nodes: &mut Vec<Node>, fixed_id: &str) {
    push_down(nodes, fixed_id);
    if COMPACT {
        compact(nodes);
    }
}

fn push_down(nodes: &mut [Node], fixed_id: &str) {
    let Some(fixed) = nodes.iter().find(|n| n.id == fixed_id).map(rect_of) else {
        return;
    };
    let mut placed = vec![fixed];
    let mut order: Vec<usize> = (0..nodes.len())
        .filter(|&i| nodes[i].id != fixed_id)
        .collect();
    order.sort_by_key(|&i| (nodes[i].pos.y, nodes[i].pos.x));
    for i in order {
        let mut r = rect_of(&nodes[i]);
        // Mientras pise algo ya ubicado, baja justo debajo de eso.
        while let Some(hit) = placed
            .iter()
            .filter(|p| collides(&r, p))
            .map(Rect::bottom)
            .max()
        {
            r.y = hit;
        }
        nodes[i].pos.y = r.y;
        placed.push(r);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::{Pos, Widget};

    fn node(id: &str, type_name: &str, size: &str, x: u8, y: u16) -> Node {
        let widget = Widget::defaults(type_name).unwrap();
        assert!(widget.size(size).is_some());
        Node {
            id: id.into(),
            widget,
            pos: Pos { x, y },
            size: size.into(),
        }
    }

    fn pos(nodes: &[Node], id: &str) -> (u8, u16) {
        let n = nodes.iter().find(|n| n.id == id).unwrap();
        (n.pos.x, n.pos.y)
    }

    fn assert_invariants(nodes: &[Node]) {
        let rects: Vec<Rect> = nodes.iter().map(rect_of).collect();
        for (i, a) in rects.iter().enumerate() {
            assert!(a.x + a.w <= GRID_COLS, "fuera de la grilla: {a:?}");
            for b in &rects[i + 1..] {
                assert!(!collides(a, b), "superposición: {a:?} {b:?}");
            }
        }
        if COMPACT {
            for (i, a) in rects.iter().enumerate() {
                if a.y == 0 {
                    continue;
                }
                let up = Rect { y: a.y - 1, ..*a };
                let blocked = rects
                    .iter()
                    .enumerate()
                    .any(|(j, b)| j != i && collides(&up, b));
                assert!(blocked, "el nodo {a:?} podía subir");
            }
        }
    }

    #[test]
    fn moving_onto_another_pushes_it_down() {
        let mut nodes = vec![
            node("a", "Text", "small", 0, 0),
            node("b", "Text", "small", 4, 0),
        ];
        move_node(&mut nodes, "a", 4, 0);
        assert_eq!(pos(&nodes, "a"), (4, 0));
        assert_eq!(pos(&nodes, "b"), (4, 2));
        assert_invariants(&nodes);
    }

    #[test]
    fn cascade_push_resolves_three_stacked_nodes() {
        let mut nodes = vec![
            node("a", "Text", "full", 0, 0),
            node("b", "Text", "full", 0, 2),
            node("c", "Text", "full", 0, 4),
            node("d", "Text", "small", 0, 6),
        ];
        move_node(&mut nodes, "d", 0, 0);
        assert_eq!(pos(&nodes, "d"), (0, 0));
        assert_eq!(pos(&nodes, "a"), (0, 2));
        assert_eq!(pos(&nodes, "b"), (0, 4));
        assert_eq!(pos(&nodes, "c"), (0, 6));
        assert_invariants(&nodes);
    }

    #[test]
    fn x_is_clamped_to_the_edges() {
        let mut nodes = vec![node("a", "Text", "small", 4, 0)];
        move_node(&mut nodes, "a", -3, 0);
        assert_eq!(pos(&nodes, "a"), (0, 0));
        move_node(&mut nodes, "a", 11, 0);
        assert_eq!(pos(&nodes, "a"), (12 - 4, 0));
        move_node(&mut nodes, "a", 3, -5);
        assert_eq!(pos(&nodes, "a"), (3, 0));
    }

    #[test]
    fn removing_the_top_node_lifts_the_rest() {
        let mut nodes = vec![
            node("a", "Banner", "full", 0, 0),
            node("b", "Text", "full", 0, 4),
            node("c", "Text", "small", 0, 6),
        ];
        remove_node(&mut nodes, "a");
        assert_eq!(pos(&nodes, "b"), (0, 0));
        assert_eq!(pos(&nodes, "c"), (0, 2));
        assert_invariants(&nodes);
    }

    #[test]
    fn wider_size_on_the_right_edge_fixes_x() {
        let mut nodes = vec![node("a", "Text", "small", 8, 0)];
        change_size(&mut nodes, "a", "medium");
        assert_eq!(pos(&nodes, "a"), (6, 0));
        change_size(&mut nodes, "a", "full");
        assert_eq!(pos(&nodes, "a"), (0, 0));
        assert_eq!(nodes[0].size, "full");
        assert_invariants(&nodes);
    }

    #[test]
    fn compact_is_idempotent() {
        let mut nodes = vec![
            node("a", "Text", "small", 0, 3),
            node("b", "Text", "medium", 4, 7),
            node("c", "FeaturedProduct", "small", 9, 12),
            node("d", "Banner", "full", 0, 20),
        ];
        compact(&mut nodes);
        let once = nodes.clone();
        compact(&mut nodes);
        assert_eq!(nodes, once);
        assert_invariants(&nodes);
    }

    #[test]
    fn invariants_hold_after_200_random_operations() {
        // Generador congruencial simple, para no sumar dependencias.
        let mut seed: u64 = 0x5eed_1234_abcd_0001;
        let mut next = move |n: u64| {
            seed = seed
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            (seed >> 33) % n
        };
        let types = ["Banner", "Text", "ProductGrid", "FeaturedProduct"];
        let mut nodes: Vec<Node> = Vec::new();
        let mut counter = 0;
        for _ in 0..200 {
            let op = if nodes.is_empty() { 0 } else { next(4) };
            match op {
                0 => {
                    let widget = Widget::defaults(types[next(4) as usize]).unwrap();
                    let size = widget.sizes()[0].id.to_string();
                    counter += 1;
                    let n = Node {
                        id: format!("n{counter}"),
                        widget,
                        pos: Pos { x: 0, y: 0 },
                        size,
                    };
                    add_node(&mut nodes, n);
                }
                1 => {
                    let id = nodes[next(nodes.len() as u64) as usize].id.clone();
                    let x = next(16) as i32 - 2;
                    let y = next(30) as i32 - 2;
                    move_node(&mut nodes, &id, x, y);
                }
                2 => {
                    let i = next(nodes.len() as u64) as usize;
                    let id = nodes[i].id.clone();
                    let sizes = nodes[i].widget.sizes();
                    let size = sizes[next(sizes.len() as u64) as usize].id;
                    change_size(&mut nodes, &id, size);
                }
                _ => {
                    let id = nodes[next(nodes.len() as u64) as usize].id.clone();
                    remove_node(&mut nodes, &id);
                }
            }
            assert_invariants(&nodes);
        }
    }

    #[test]
    fn cell_from_point_maps_pixels_to_cells() {
        // 12 columnas de 64 px + 11 huecos de 16 px = 944 px.
        assert_eq!(cell_from_point(0.0, 0.0, 944.0), (0, 0));
        assert_eq!(cell_from_point(80.0, 96.0, 944.0), (1, 1));
        assert_eq!(cell_from_point(-10.0, -10.0, 944.0), (-1, -1));
    }
}
