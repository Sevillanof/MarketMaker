//! Matemática de la grilla fina ([`GRID_COLS`] columnas, filas de
//! [`domain::ROW_PX`] px). Funciones puras, sin Leptos.
//!
//! Después de cualquier operación se cumple siempre: ningún nodo se
//! superpone, todos caben en las columnas y, con [`COMPACT`] activo,
//! ninguno puede subir más.

use domain::{Node, GRID_COLS};

/// Alto de una fila en píxeles.
pub const ROW_PX: f64 = domain::ROW_PX as f64;
/// Separación visible entre bloques. No es un hueco de la grilla: cada bloque
/// deja la mitad como margen alrededor (ver `.mk-node` en el CSS), así la
/// grilla no tiene huecos y el paso de columnas y filas es parejo.
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

/// Rectángulo que ocupa un nodo.
pub fn rect_of(node: &Node) -> Rect {
    Rect {
        x: node.pos.x,
        y: node.pos.y,
        w: node.size.w.clamp(1, GRID_COLS),
        h: node.size.h.max(1),
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

/// Cambia el ancho (columnas) y el alto (filas) de un nodo, limitados a los
/// mínimos y máximos de su widget. Si el nuevo ancho no entra desde su `x`, el
/// nodo se corre a la izquierda. Empuja a los que pise.
pub fn resize_node(nodes: &mut Vec<Node>, id: &str, w: i32, h: i32) {
    let Some(node) = nodes.iter_mut().find(|n| n.id == id) else {
        return;
    };
    let size = node.widget.limits().clamp(w, h);
    node.size = size;
    node.pos.x = node.pos.x.min(GRID_COLS - size.w);
    settle(nodes, id);
}

/// Sube el nodo un lugar en el orden de lectura: pasa a ocupar la fila del
/// nodo anterior, que queda debajo. Es la alternativa a arrastrar.
pub fn move_up(nodes: &mut Vec<Node>, id: &str) {
    let mut order = nodes.clone();
    sort_by_position(&mut order);
    let Some(i) = order.iter().position(|n| n.id == id) else {
        return;
    };
    if i == 0 {
        return;
    }
    let (x, y) = (i32::from(order[i].pos.x), i32::from(order[i - 1].pos.y));
    move_node(nodes, id, x, y);
}

/// Baja el nodo un lugar en el orden de lectura: pasa debajo del siguiente.
pub fn move_down(nodes: &mut Vec<Node>, id: &str) {
    let mut order = nodes.clone();
    sort_by_position(&mut order);
    let Some(i) = order.iter().position(|n| n.id == id) else {
        return;
    };
    let Some(next) = order.get(i + 1) else {
        return;
    };
    let next_id = next.id.clone();
    // El siguiente toma el lugar del nodo, que queda debajo de él.
    let (x, y) = (i32::from(next.pos.x), i32::from(order[i].pos.y));
    move_node(nodes, &next_id, x, y);
}

/// Inserta `node` con su esquina superior izquierda en la celda `(x, y)`,
/// limitada a la grilla, y empuja a los que pise. Es lo que pasa al soltar un
/// bloque de la paleta sobre el lienzo.
pub fn insert_at(nodes: &mut Vec<Node>, node: Node, x: i32, y: i32) {
    let id = node.id.clone();
    nodes.push(node);
    move_node(nodes, &id, x, y);
}

/// Dónde soltar un bloque nuevo de `w` × `h` que se arrastra con la esquina
/// superior izquierda en `(x, y)`. Si cae sobre otro bloque, se decide por
/// la mitad: en la de arriba el nuevo toma su lugar (y el otro baja); en la
/// de abajo, el nuevo va debajo de él. Así no salta arriba de todo un bloque
/// soltado sobre la parte de abajo de uno ancho.
pub fn drop_target(nodes: &[Node], w: u8, h: u8, x: i32, y: i32) -> (i32, i32) {
    let w = w.clamp(1, GRID_COLS);
    let x = x.clamp(0, i32::from(GRID_COLS - w));
    let mut y = y.clamp(0, i32::from(u16::MAX));
    for _ in 0..=nodes.len() {
        let r = Rect {
            x: x as u8,
            y: y as u16,
            w,
            h: h.max(1),
        };
        let Some(hit) = nodes
            .iter()
            .map(rect_of)
            .filter(|n| collides(&r, n))
            .min_by_key(|n| n.y)
        else {
            break;
        };
        if y >= i32::from(hit.y) + i32::from(hit.h) / 2 {
            y = i32::from(hit.bottom());
        } else {
            y = i32::from(hit.y);
            break;
        }
    }
    (x, y)
}

/// Inserta `node` justo debajo del nodo `after_id` y empuja a los demás.
pub fn insert_below(nodes: &mut Vec<Node>, after_id: &str, mut node: Node) {
    let Some(r) = nodes.iter().find(|n| n.id == after_id).map(rect_of) else {
        add_node(nodes, node);
        return;
    };
    node.pos.x = r.x.min(GRID_COLS - rect_of(&node).w);
    node.pos.y = r.bottom();
    let id = node.id.clone();
    nodes.push(node);
    settle(nodes, &id);
}

pub fn remove_node(nodes: &mut Vec<Node>, id: &str) {
    nodes.retain(|n| n.id != id);
    if COMPACT {
        compact(nodes);
    }
}

/// Sube cada nodo, en orden `(y, x)`, mientras no choque con los ya ubicados.
// La firma con `&mut Vec` es la del plan, igual que las demás operaciones.
#[allow(clippy::ptr_arg)]
pub fn compact(nodes: &mut Vec<Node>) {
    sort_by_position(nodes);
    let mut placed: Vec<Rect> = Vec::with_capacity(nodes.len());
    for node in nodes.iter_mut() {
        let mut r = rect_of(node);
        // Sube hasta apoyarse en el más bajo de los que tiene encima. Si ya
        // pisa a alguno (no debería), se queda donde está.
        if !placed.iter().any(|p| collides(&r, p)) {
            r.y = placed
                .iter()
                .filter(|p| p.x < r.x + r.w && r.x < p.x + p.w && p.bottom() <= r.y)
                .map(Rect::bottom)
                .max()
                .unwrap_or(0);
        }
        node.pos.y = r.y;
        placed.push(r);
    }
}

/// Ancho de una columna en píxeles para una grilla de `container_width` px.
pub fn col_pitch(container_width: f64) -> f64 {
    (container_width / f64::from(GRID_COLS)).max(1.0)
}

/// Celda más cercana a un punto en píxeles relativo a la esquina superior
/// izquierda de la grilla: el imán redondea a la línea de la grilla más
/// próxima en vez de truncar.
pub fn cell_from_point(px: f64, py: f64, container_width: f64) -> (i32, i32) {
    let x = (px / col_pitch(container_width)).round() as i32;
    let y = (py / ROW_PX).round() as i32;
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
    use domain::{Pos, Size, Widget, WIDGET_TYPES};

    /// Columnas y filas de la grilla fina por cada celda de las de antes. Los
    /// casos se escriben en celdas de antes, que se leen mejor.
    const SX: u8 = 4;
    const SY: u16 = 12;

    /// Nodo con el tamaño de una de las opciones fijas de antes, en `(x, y)`
    /// celdas de antes.
    fn node(id: &str, type_name: &str, size: &str, x: u8, y: u16) -> Node {
        let (w, h) = match (type_name, size) {
            ("Text", "small") => (4, 2),
            ("Text", "medium") => (6, 3),
            ("Text", "full") => (12, 2),
            ("Banner", "full") => (12, 4),
            ("FeaturedProduct", "small") => (3, 4),
            _ => panic!("tamaño de prueba desconocido"),
        };
        let mut n = Node::new(id, Widget::defaults(type_name).unwrap());
        n.pos = Pos {
            x: x * SX,
            y: y * SY,
        };
        n.size = Size {
            w: w * SX,
            h: h * SY as u8,
        };
        n
    }

    /// Posición en celdas de antes.
    fn pos(nodes: &[Node], id: &str) -> (u8, u16) {
        let n = nodes.iter().find(|n| n.id == id).unwrap();
        (n.pos.x / SX, n.pos.y / SY)
    }

    /// `(x, y)` en celdas de antes, para llamar a las operaciones.
    fn at(x: i32, y: i32) -> (i32, i32) {
        (x * i32::from(SX), y * i32::from(SY))
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
        let (x, y) = at(4, 0);
        move_node(&mut nodes, "a", x, y);
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
        move_node(&mut nodes, "a", 47, 0);
        assert_eq!(nodes[0].pos.x, GRID_COLS - 16);
        move_node(&mut nodes, "a", 13, -5);
        assert_eq!(nodes[0].pos, Pos { x: 13, y: 0 });
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
        let (w, h) = at(6, 3);
        resize_node(&mut nodes, "a", w, h);
        assert_eq!(pos(&nodes, "a"), (6, 0));
        let (w, h) = at(12, 2);
        resize_node(&mut nodes, "a", w, h);
        assert_eq!(pos(&nodes, "a"), (0, 0));
        assert_eq!(nodes[0].size, Size { w: 48, h: 24 });
        assert_invariants(&nodes);
    }

    #[test]
    fn resize_is_clamped_to_widget_limits() {
        let mut nodes = vec![node("a", "FeaturedProduct", "small", 0, 0)];
        resize_node(&mut nodes, "a", 1, 200);
        assert_eq!(nodes[0].size, Size { w: 8, h: 96 });
        resize_node(&mut nodes, "a", 48, 0);
        assert_eq!(nodes[0].size, Size { w: 24, h: 36 });
        // La grilla fina deja tamaños intermedios.
        resize_node(&mut nodes, "a", 13, 41);
        assert_eq!(nodes[0].size, Size { w: 13, h: 41 });
    }

    #[test]
    fn growing_taller_pushes_the_node_below() {
        let mut nodes = vec![
            node("a", "Text", "full", 0, 0),
            node("b", "Text", "full", 0, 2),
        ];
        let (w, h) = at(12, 5);
        resize_node(&mut nodes, "a", w, h);
        assert_eq!(pos(&nodes, "b"), (0, 5));
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
        let types: Vec<&str> = WIDGET_TYPES.iter().map(|(t, _)| *t).collect();
        let mut nodes: Vec<Node> = Vec::new();
        let mut counter = 0;
        for _ in 0..200 {
            let op = if nodes.is_empty() { 0 } else { next(4) };
            match op {
                0 => {
                    let widget =
                        Widget::defaults(types[next(types.len() as u64) as usize]).unwrap();
                    counter += 1;
                    add_node(&mut nodes, Node::new(format!("n{counter}"), widget));
                }
                1 => {
                    let id = nodes[next(nodes.len() as u64) as usize].id.clone();
                    let x = next(56) as i32 - 4;
                    let y = next(300) as i32 - 4;
                    move_node(&mut nodes, &id, x, y);
                }
                2 => {
                    let i = next(nodes.len() as u64) as usize;
                    let id = nodes[i].id.clone();
                    let w = next(56) as i32 - 4;
                    let h = next(160) as i32 - 4;
                    resize_node(&mut nodes, &id, w, h);
                    let l = nodes.iter().find(|n| n.id == id).unwrap();
                    assert!(l.widget.limits().contains(l.size));
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
    fn cell_from_point_snaps_to_the_nearest_line() {
        // 48 columnas de 20 px = 960 px; filas de 8 px.
        assert_eq!(cell_from_point(0.0, 0.0, 960.0), (0, 0));
        assert_eq!(cell_from_point(29.0, 11.0, 960.0), (1, 1));
        assert_eq!(cell_from_point(31.0, 13.0, 960.0), (2, 2));
        assert_eq!(cell_from_point(-12.0, -6.0, 960.0), (-1, -1));
    }

    #[test]
    fn drop_target_decides_by_the_half_of_the_block_below() {
        let nodes = vec![
            node("a", "Banner", "full", 0, 0),
            node("b", "Text", "full", 0, 4),
        ];
        let (w, h) = (16, 24);
        // Mitad de arriba del banner (filas 0 a 47): toma su lugar.
        assert_eq!(drop_target(&nodes, w, h, 8, 10), (8, 0));
        // Mitad de abajo del banner: queda debajo; y como ahí cae en la mitad
        // de arriba de "b", toma el lugar de "b".
        assert_eq!(drop_target(&nodes, w, h, 8, 30), (8, 48));
        // Mitad de abajo de "b": después de todo.
        assert_eq!(drop_target(&nodes, w, h, 8, 65), (8, 72));
        // Contra el borde derecho y por encima de la grilla.
        assert_eq!(drop_target(&nodes, w, h, 60, -9), (32, 0));
    }

    #[test]
    fn insert_at_drops_the_node_where_it_is_released() {
        let mut nodes = vec![
            node("a", "Text", "full", 0, 0),
            node("b", "Text", "full", 0, 2),
        ];
        // Sobre la mitad derecha de "b": "b" baja y el nuevo toma su lugar.
        let (x, y) = at(6, 2);
        insert_at(&mut nodes, node("c", "Text", "small", 0, 0), x, y);
        assert_eq!(pos(&nodes, "c"), (6, 2));
        assert_eq!(pos(&nodes, "b"), (0, 4));
        assert_eq!(order(&nodes), ["a", "c", "b"]);
        // Muy abajo, sin nada en medio: sube hasta apoyarse en el último.
        insert_at(&mut nodes, node("d", "Text", "small", 0, 0), 0, 5000);
        assert_eq!(pos(&nodes, "d"), (0, 6));
        // Contra el borde derecho: se corre para que entre.
        insert_at(&mut nodes, node("e", "Text", "small", 0, 0), 47, 0);
        assert_eq!(nodes.iter().find(|n| n.id == "e").unwrap().pos.x, 32);
        assert_invariants(&nodes);
    }

    #[test]
    fn move_up_and_down_swap_reading_order() {
        let mut nodes = vec![
            node("a", "Text", "full", 0, 0),
            node("b", "Banner", "full", 0, 2),
            node("c", "Text", "full", 0, 6),
        ];
        move_up(&mut nodes, "b");
        assert_eq!(order(&nodes), ["b", "a", "c"]);
        move_down(&mut nodes, "b");
        assert_eq!(order(&nodes), ["a", "b", "c"]);
        move_down(&mut nodes, "c");
        move_up(&mut nodes, "a");
        assert_eq!(order(&nodes), ["a", "b", "c"]);
    }

    #[test]
    fn insert_below_places_copy_after_original() {
        let mut nodes = vec![
            node("a", "Text", "full", 0, 0),
            node("b", "Banner", "full", 0, 2),
        ];
        insert_below(&mut nodes, "a", node("a2", "Text", "full", 0, 0));
        assert_eq!(order(&nodes), ["a", "a2", "b"]);
    }

    fn order(nodes: &[Node]) -> Vec<String> {
        let mut v = nodes.to_vec();
        sort_by_position(&mut v);
        v.into_iter().map(|n| n.id).collect()
    }
}
