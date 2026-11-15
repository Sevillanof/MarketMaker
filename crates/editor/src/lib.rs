//! Editor del panel: lienzo, paleta e inspector.
//!
//! No llama al servidor: recibe la acción de guardado como parámetro.
//! Está pensado para alguien sin experiencia: todo se puede hacer con mouse
//! o con teclado, los errores se explican junto al campo y cada cambio se
//! puede deshacer.

#![recursion_limit = "512"]

mod canvas;
mod fields;
mod icons;
mod inline;
mod inspector;
mod look;
mod palette;
mod photos;
mod store_drawer;
mod topbar;
mod typography;

use domain::{Layer, Node, PageDoc, PageView, SaveOutcome, HOME};
use leptos::prelude::*;
use std::time::Duration;
use wasm_bindgen::JsCast;

use canvas::Canvas;
use icons::Icon;
use inspector::Inspector;
use palette::Palette;
use topbar::{PageSwitcher, StoreMenu};

/// Cuántos pasos de deshacer se guardan.
pub const MAX_UNDO: usize = 50;

/// Acción de guardado del borrador: recibe el documento y la versión base.
pub type SaveAction = Action<(PageDoc, u32), Result<SaveOutcome, ServerFnError>>;
/// Acción de publicar: recibe la versión guardada a publicar y devuelve la
/// versión que quedó publicada.
pub type PublishAction = Action<u32, Result<u32, ServerFnError>>;

/// Cuánto espera el editor sin cambios nuevos antes de guardar solo.
const AUTOSAVE_DELAY: Duration = Duration::from_millis(1500);

/// Clave del borrador guardado en el navegador.
const DRAFT_KEY: &str = "mk-editor-borrador:home";
/// Clave que recuerda que ya se cerró la guía de primeros pasos.
pub(crate) const WELCOME_KEY: &str = "mk-editor-bienvenida";

/// Todo el estado del documento en un solo lugar. Toda modificación pasa
/// por [`EditorState::apply`] o [`EditorState::apply_grouped`].
#[derive(Clone, Copy)]
pub(crate) struct EditorState {
    pub doc: RwSignal<PageDoc>,
    /// Lo último guardado en el servidor (el borrador).
    pub saved: RwSignal<PageDoc>,
    /// Versión de `saved`.
    pub base_version: RwSignal<u32>,
    /// Versión que ven los visitantes.
    pub published: RwSignal<u32>,
    pub selected: RwSignal<Option<String>>,
    /// Qué se está editando: una página o la ficha del producto.
    pub layer: RwSignal<Layer>,
    /// Página que se está editando (o la que queda detrás de la ficha):
    /// `0` es Inicio.
    pub page: RwSignal<usize>,
    pub undo: RwSignal<Vec<PageDoc>>,
    pub redo: RwSignal<Vec<PageDoc>>,
    /// Grupo de la última edición. Las ediciones seguidas del mismo grupo
    /// (las teclas de un mismo campo) son un solo paso de deshacer.
    group: RwSignal<Option<String>>,
}

impl EditorState {
    fn new(view: &PageView) -> Self {
        Self {
            doc: RwSignal::new(view.doc.clone()),
            saved: RwSignal::new(view.doc.clone()),
            base_version: RwSignal::new(view.version),
            published: RwSignal::new(view.published),
            selected: RwSignal::new(None),
            layer: RwSignal::new(HOME),
            page: RwSignal::new(0),
            undo: RwSignal::new(Vec::new()),
            redo: RwSignal::new(Vec::new()),
            group: RwSignal::new(None),
        }
    }

    /// Aplica un cambio como un paso de deshacer propio.
    pub fn apply(&self, f: impl FnOnce(&mut PageDoc)) {
        self.end_group();
        self.apply_grouped(None, f);
    }

    /// Aplica un cambio. Si `group` es el mismo que el de la edición anterior,
    /// se suma a ese paso de deshacer en vez de crear uno nuevo. Si el cambio
    /// no modifica nada, no hace nada.
    pub fn apply_grouped(&self, group: Option<&str>, f: impl FnOnce(&mut PageDoc)) {
        let before = self.doc.get_untracked();
        let mut next = before.clone();
        f(&mut next);
        // El menú y el pie compartidos cambian en todas las páginas.
        domain::sync_shared(&before, &mut next);
        if next == before {
            return;
        }
        let same_group = group.is_some() && self.group.with_untracked(|g| g.as_deref() == group);
        if !same_group {
            push_capped(self.undo, before);
        }
        self.group.set(group.map(str::to_string));
        self.redo.update(Vec::clear);
        self.doc.set(next);
    }

    /// Cierra el paso de deshacer en curso: la próxima edición abre uno nuevo.
    pub fn end_group(&self) {
        if self.group.with_untracked(Option::is_some) {
            self.group.set(None);
        }
    }

    pub fn undo(&self) {
        self.end_group();
        if let Some(prev) = self.undo.try_update(Vec::pop).flatten() {
            let current = self.doc.get_untracked();
            self.redo.update(|r| r.push(current));
            self.doc.set(prev);
        }
    }

    pub fn redo(&self) {
        self.end_group();
        if let Some(next) = self.redo.try_update(Vec::pop).flatten() {
            let current = self.doc.get_untracked();
            push_capped(self.undo, current);
            self.doc.set(next);
        }
    }

    /// Hay cambios que todavía no se guardaron en el servidor.
    pub fn dirty(&self) -> bool {
        self.doc.with(|d| self.saved.with(|s| d != s))
    }

    /// Hay algo que los visitantes todavía no ven: cambios sin guardar o un
    /// borrador guardado sin publicar.
    pub fn unpublished(&self) -> bool {
        self.dirty() || self.base_version.get() != self.published.get()
    }

    /// Bloque con ese id, en la página o en la ficha.
    pub fn node(&self, id: &str) -> Option<Node> {
        self.doc.with_untracked(|d| d.node(id).cloned())
    }

    /// Bloques de la capa que se está editando (sigue los cambios).
    pub fn nodes(&self) -> Vec<Node> {
        let layer = self.layer.get();
        self.doc.with(|d| d.layer(layer).clone())
    }

    pub fn nodes_untracked(&self) -> Vec<Node> {
        let layer = self.layer.get_untracked();
        self.doc.with_untracked(|d| d.layer(layer).clone())
    }

    /// Aplica un cambio a los bloques de la capa que se está editando.
    pub fn apply_nodes(&self, f: impl FnOnce(&mut Vec<Node>)) {
        let layer = self.layer.get_untracked();
        self.apply(|d| f(d.layer_mut(layer)));
    }

    pub fn in_modal(&self) -> bool {
        self.layer.get() == Layer::ProductModal
    }

    pub fn in_modal_untracked(&self) -> bool {
        self.layer.get_untracked() == Layer::ProductModal
    }

    /// La página que se está editando (o la que queda detrás de la ficha).
    pub fn page_layer(&self) -> Layer {
        Layer::Page(self.page.get())
    }
}

fn push_capped(stack: RwSignal<Vec<PageDoc>>, doc: PageDoc) {
    stack.update(|u| {
        u.push(doc);
        if u.len() > MAX_UNDO {
            u.remove(0);
        }
    });
}

/// Un problema que impide guardar, con el bloque donde está.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Problem {
    pub node_id: String,
    pub layer: Layer,
    pub text: String,
}

/// Resultado del último guardado que necesita atención.
#[derive(Clone, Debug, PartialEq)]
enum Status {
    Idle,
    Conflict,
    /// Problemas encontrados en el navegador antes de enviar.
    Problems(Vec<Problem>),
    /// Problemas que devolvió el servidor.
    Invalid(Vec<String>),
    Error(String),
}

/// Aviso breve que se va solo, con un "Deshacer" opcional.
#[derive(Clone, Debug, PartialEq)]
struct Toast {
    text: String,
    undo: bool,
}

/// Estado de la interfaz compartido por todo el editor (por contexto).
#[derive(Clone, Copy)]
pub(crate) struct Ui {
    toast: RwSignal<Option<Toast>>,
    toast_seq: StoredValue<u64>,
    /// El mouse o el foco están sobre el aviso: no se va solo mientras tanto.
    toast_hover: StoredValue<bool>,
    toast_focus: StoredValue<bool>,
    /// Texto para lectores de pantalla (región `role=status` invisible).
    announce: RwSignal<String>,
    pub preview: RwSignal<bool>,
    /// El cajón "Mi tienda" está abierto en lugar de la lista de bloques.
    pub store_open: RwSignal<bool>,
    /// Bloque nuevo que se está arrastrando desde la paleta.
    pub new_drag: RwSignal<Option<NewDrag>>,
    /// El último apretar y soltar fue un arrastre: el clic que lo sigue no
    /// tiene que agregar el bloque al final.
    pub swallow_click: StoredValue<bool>,
    /// Se está moviendo o cambiando de tamaño un bloque del lienzo con el
    /// mouse: Esc cancela eso y no suelta el bloque elegido.
    pub block_drag: StoredValue<bool>,
    /// Se intentó guardar con datos para corregir: los campos muestran su
    /// error aunque todavía no se hayan tocado.
    pub show_errors: RwSignal<bool>,
    /// Producto con el que se muestra la ficha mientras se edita. Sin
    /// elegir, el primero de la tienda.
    pub modal_product: RwSignal<Option<String>>,
    /// Bloque de la página desde el que se abrió la ficha: se vuelve a él
    /// al cerrarla.
    pub return_to: StoredValue<Option<String>>,
}

/// Arrastre de un bloque nuevo desde la paleta hacia el lienzo.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct NewDrag {
    pub type_name: &'static str,
    /// Dónde se apretó el botón, para distinguir un clic de un arrastre.
    pub start_x: f64,
    pub start_y: f64,
    /// Posición actual del puntero.
    pub x: f64,
    pub y: f64,
    /// Ya se movió lo suficiente para ser un arrastre y no un clic.
    pub active: bool,
}

impl NewDrag {
    /// Píxeles que hay que mover el puntero para que deje de ser un clic.
    pub const THRESHOLD_PX: f64 = 5.0;
}

impl Ui {
    fn toast(&self, text: impl Into<String>, undo: bool) {
        let seq = self.toast_seq.get_value() + 1;
        self.toast_seq.set_value(seq);
        self.toast.set(Some(Toast {
            text: text.into(),
            undo,
        }));
        self.hide_toast_later(Duration::from_secs(6));
    }

    /// Cierra el aviso actual dentro de `delay`, salvo que para entonces haya
    /// otro aviso o el mouse o el foco estén sobre él. En ese caso se vuelve a
    /// programar cuando salen (ver [`Ui::release_toast`]).
    fn hide_toast_later(&self, delay: Duration) {
        let seq = self.toast_seq.get_value();
        let ui = *self;
        set_timeout(
            move || {
                if ui.toast_seq.get_value() == seq
                    && !ui.toast_hover.get_value()
                    && !ui.toast_focus.get_value()
                {
                    ui.toast.set(None);
                }
            },
            delay,
        );
    }

    /// El mouse (`hover`) o el foco dejaron el aviso: si ya nada lo retiene,
    /// se va a los pocos segundos.
    fn release_toast(&self, hover: bool) {
        if hover {
            self.toast_hover.set_value(false);
        } else {
            self.toast_focus.set_value(false);
        }
        if self.toast.with_untracked(Option::is_some) {
            self.hide_toast_later(Duration::from_secs(3));
        }
    }

    /// Cierra el aviso a mano.
    fn close_toast(&self) {
        self.toast_hover.set_value(false);
        self.toast_focus.set_value(false);
        self.toast.set(None);
    }

    pub fn announce(&self, text: impl Into<String>) {
        // Vaciar primero para que se repita el anuncio aunque el texto sea igual.
        self.announce.set(String::new());
        let (announce, text) = (self.announce, text.into());
        request_animation_frame(move || announce.set(text));
    }
}

/// Acciones sobre un bloque, compartidas por el lienzo, el inspector y el
/// teclado.
#[derive(Clone, Copy)]
pub(crate) struct Actions {
    state: EditorState,
    ui: Ui,
}

impl Actions {
    pub fn label(&self, id: &str) -> &'static str {
        self.state.node(id).map_or("Bloque", |n| n.widget.label())
    }

    /// Texto que describe dónde quedó un bloque.
    pub fn position_text(&self, id: &str) -> String {
        let mut nodes = self.state.nodes_untracked();
        grid::sort_by_position(&mut nodes);
        let total = nodes.len();
        nodes
            .iter()
            .position(|n| n.id == id)
            .map(|i| {
                let n = &nodes[i];
                format!(
                    "{}: bloque {} de {total}, {}",
                    n.widget.label(),
                    i + 1,
                    size_text(n.size)
                )
            })
            .unwrap_or_default()
    }

    /// Agrega un bloque nuevo con su esquina superior izquierda en la celda
    /// `(x, y)`: es lo que pasa al soltarlo desde la paleta.
    pub fn add_at(&self, node: Node, x: i32, y: i32) {
        let (id, label) = (node.id.clone(), node.widget.label());
        self.state
            .apply_nodes(|nodes| grid::insert_at(nodes, node, x, y));
        self.state.selected.set(Some(id.clone()));
        self.ui
            .announce(format!("Agregaste {label}. {}", self.position_text(&id)));
        reveal_node(&id, true);
    }

    pub fn add(&self, type_name: &str) {
        let Some(widget) = domain::Widget::defaults(type_name) else {
            return;
        };
        if !domain::allowed_in(type_name, self.state.layer.get_untracked()) {
            return;
        }
        let label = widget.label();
        let id = new_node_id(&self.state.doc.get_untracked());
        let node = Node::new(id.clone(), widget);
        self.state.apply_nodes(|nodes| grid::add_node(nodes, node));
        self.state.selected.set(Some(id.clone()));
        let place = if self.state.layer.get_untracked() == Layer::ProductModal {
            "de la ficha"
        } else {
            "de la página"
        };
        self.ui
            .announce(format!("Agregaste {label} al final {place}."));
        reveal_node(&id, true);
    }

    pub fn move_up(&self, id: &str) {
        self.state.apply_nodes(|nodes| grid::move_up(nodes, id));
        self.after_move(id);
    }

    pub fn move_down(&self, id: &str) {
        self.state.apply_nodes(|nodes| grid::move_down(nodes, id));
        self.after_move(id);
    }

    /// Mueve una columna a la izquierda (`-1`) o a la derecha (`1`).
    pub fn shift(&self, id: &str, dx: i32) {
        self.state.apply_nodes(|nodes| {
            if let Some(n) = nodes.iter().find(|n| n.id == id) {
                let (x, y) = (i32::from(n.pos.x) + dx, i32::from(n.pos.y));
                grid::move_node(nodes, id, x, y);
            }
        });
        self.after_move(id);
    }

    pub fn resize_by(&self, id: &str, dw: i32, dh: i32) {
        self.state.apply_nodes(|nodes| {
            if let Some(n) = nodes.iter().find(|n| n.id == id) {
                let (w, h) = (i32::from(n.size.w) + dw, i32::from(n.size.h) + dh);
                grid::resize_node(nodes, id, w, h);
            }
        });
        self.after_move(id);
    }

    fn after_move(&self, id: &str) {
        self.ui.announce(self.position_text(id));
        reveal_node(id, false);
    }

    pub fn duplicate(&self, id: &str) {
        let Some(mut copy) = self.state.node(id) else {
            return;
        };
        let new_id = new_node_id(&self.state.doc.get_untracked());
        copy.id = new_id.clone();
        // La copia es un bloque aparte: no se comparte con las otras páginas.
        copy.shared.clear();
        let label = copy.widget.label();
        self.state
            .apply_nodes(|nodes| grid::insert_below(nodes, id, copy));
        self.state.selected.set(Some(new_id.clone()));
        self.ui
            .announce(format!("Duplicaste {label}. La copia quedó debajo."));
        reveal_node(&new_id, true);
    }

    pub fn delete(&self, id: &str) {
        let label = self.label(id);
        self.state.apply_nodes(|nodes| grid::remove_node(nodes, id));
        self.state.selected.set(None);
        self.ui.toast(format!("{label} borrado."), true);
    }

    /// Abre la ficha del producto para editarla, mostrando `product_id` (o
    /// el producto elegido antes). El fondo se oscurece y la paleta pasa a
    /// mostrar solo bloques para la ficha.
    pub fn open_modal(&self, product_id: Option<String>) {
        if product_id.is_some() {
            self.ui.modal_product.set(product_id);
        }
        if self.state.layer.get_untracked() == Layer::ProductModal {
            return;
        }
        self.state.end_group();
        self.ui
            .return_to
            .set_value(self.state.selected.get_untracked());
        self.state.selected.set(None);
        self.state.layer.set(Layer::ProductModal);
        self.ui.announce(
            "Estás editando la ficha del producto. Es la misma para todos tus productos. Esc para volver a la página.",
        );
        request_animation_frame(|| {
            if let Some(el) = document().get_element_by_id("mk-canvas") {
                el.set_scroll_top(0);
            }
            focus_selector("#mk-modal-title");
        });
    }

    /// Cierra la ficha y vuelve a la página, al bloque desde el que se abrió.
    pub fn close_modal(&self) {
        if self.state.layer.get_untracked() != Layer::ProductModal {
            return;
        }
        self.state.end_group();
        let page = Layer::Page(self.state.page.get_untracked());
        self.state.layer.set(page);
        let back = self
            .ui
            .return_to
            .try_update_value(Option::take)
            .flatten()
            .filter(|id| self.state.doc.with_untracked(|d| d.layer_of(id)) == Some(page));
        self.state.selected.set(back.clone());
        self.ui.announce("Volviste a la página.");
        match back {
            Some(id) => reveal_node(&id, false),
            None => request_animation_frame(|| focus_selector("#mk-inspector-title")),
        }
    }

    /// Lleva a un bloque, esté en la página que esté o en la ficha, y lo elige.
    pub fn go_to(&self, id: &str) {
        let target = self.state.doc.with_untracked(|d| d.layer_of(id));
        match target {
            Some(Layer::ProductModal) => self.open_modal(None),
            Some(Layer::Page(i)) if i != self.state.page.get_untracked() => {
                self.open_page(i, false);
            }
            Some(Layer::Page(_)) => {
                self.ui.return_to.set_value(None);
                self.close_modal();
            }
            None => return,
        }
        self.state.selected.set(Some(id.to_string()));
        reveal_node(id, true);
    }

    /// Pasa a editar la página `i` (0 es Inicio). Si estaba abierta la
    /// ficha, la cierra. Con `announce`, avisa a qué página se fue.
    pub fn open_page(&self, i: usize, announce: bool) {
        let count = self.state.doc.with_untracked(PageDoc::page_count);
        let i = if i < count { i } else { 0 };
        self.state.end_group();
        self.ui.return_to.set_value(None);
        self.state.selected.set(None);
        self.state.page.set(i);
        self.state.layer.set(Layer::Page(i));
        if announce {
            let name = self
                .state
                .doc
                .with_untracked(|d| d.page_name(i).to_string());
            self.ui
                .announce(format!("Estás editando la página {name}."));
        }
        request_animation_frame(|| {
            if let Some(el) = document().get_element_by_id("mk-canvas") {
                el.set_scroll_top(0);
            }
        });
    }

    /// Lo que pasa al tocar un enlace en el lienzo: va a la página a la que
    /// lleva. Si esa página todavía no existe, la crea (con el menú y el pie
    /// de la página actual) y avisa. `label` es el texto del enlace, para
    /// nombrar la página nueva.
    pub fn follow_link(&self, href: &str, label: &str) {
        let Some(path) = domain::normalize_path(href) else {
            self.ui.toast(
                "Este enlace lleva a otro sitio. En tu tienda se abre al tocarlo.",
                false,
            );
            return;
        };
        if let Some(i) = self.state.doc.with_untracked(|d| d.page_index(&path)) {
            if i == self.state.page.get_untracked() && !self.state.in_modal_untracked() {
                let name = self
                    .state
                    .doc
                    .with_untracked(|d| d.page_name(i).to_string());
                self.ui
                    .toast(format!("Ya estás en la página {name}."), false);
                return;
            }
            self.open_page(i, true);
            return;
        }
        if domain::is_reserved_path(&path) {
            self.ui.toast(
                format!("La dirección {path} la usa la aplicación: elegí otra para este enlace."),
                false,
            );
            return;
        }
        if self.state.doc.with_untracked(|d| d.pages.len()) >= domain::MAX_PAGES {
            self.ui.toast(
                format!(
                    "Tu tienda ya tiene {} páginas, el máximo. Borrá una para crear otra.",
                    domain::MAX_PAGES + 1
                ),
                false,
            );
            return;
        }
        let label = label.split_whitespace().collect::<Vec<_>>().join(" ");
        let name: String = if label.is_empty() {
            domain::name_from_path(&path)
        } else {
            label.chars().take(domain::MAX_PAGE_NAME).collect()
        };
        let from = self.state.page.get_untracked();
        let mut created = 0;
        self.state
            .apply(|d| created = d.add_page(path, name.clone(), from));
        self.open_page(created, false);
        let has = |t: &str| {
            self.state.doc.with_untracked(|d| {
                d.layer(Layer::Page(created))
                    .iter()
                    .any(|n| n.widget.type_name() == t)
            })
        };
        let start = match (has("Navbar"), has("Footer")) {
            (true, true) => {
                " Ya tiene tu menú superior y tu pie de página, compartidos con las demás páginas."
            }
            (true, false) => " Ya tiene tu menú superior, compartido con las demás páginas.",
            (false, true) => " Ya tiene tu pie de página, compartido con las demás páginas.",
            (false, false) => "",
        };
        self.ui.toast(
            format!("Creaste la página {name}.{start} Sumale bloques desde la izquierda."),
            true,
        );
    }

    /// Borra la página `i` (nunca Inicio) y vuelve a Inicio.
    pub fn delete_page(&self, i: usize) {
        if i == 0 || i >= self.state.doc.with_untracked(PageDoc::page_count) {
            return;
        }
        let name = self
            .state
            .doc
            .with_untracked(|d| d.page_name(i).to_string());
        self.open_page(0, false);
        self.state.apply(|d| {
            d.pages.remove(i - 1);
        });
        self.ui
            .toast(format!("Página {name} borrada. Volviste a Inicio."), true);
    }

    /// Deja de compartir el bloque `id` con las otras páginas: desde ahora
    /// se cambia solo en esta.
    pub fn unshare(&self, id: &str) {
        let label = self.label(id);
        self.state.apply(|d| {
            if let Some(n) = d.node_mut(id) {
                n.shared.clear();
            }
        });
        self.ui.toast(
            format!("{label} separado: los cambios ya no se copian a las otras páginas."),
            true,
        );
    }

    /// Elige un bloque y lleva el foco al inspector.
    pub fn edit(&self, id: &str) {
        self.state.selected.set(Some(id.to_string()));
        request_animation_frame(|| focus_selector("#mk-inspector-title"));
    }
}

/// Tamaño de un bloque dicho en palabras: ancho en columnas y alto en píxeles.
pub(crate) fn size_text(size: domain::Size) -> String {
    format!(
        "ancho {} de {} columnas, alto {} píxeles",
        size.w,
        domain::GRID_COLS,
        u32::from(size.h) * u32::from(domain::ROW_PX)
    )
}

/// Anchos con nombre, en columnas de 48.
pub(crate) const WIDTHS: &[(u8, &str)] = &[
    (48, "Completo"),
    (32, "2/3"),
    (24, "Mitad"),
    (16, "1/3"),
    (12, "1/4"),
];

/// Nombre del ancho si es uno de [`WIDTHS`]; si no, "36/48".
pub(crate) fn width_name(w: u8) -> String {
    WIDTHS.iter().find(|(cols, _)| *cols == w).map_or_else(
        || format!("{w}/{}", domain::GRID_COLS),
        |(_, name)| (*name).to_string(),
    )
}

/// Tamaño de un bloque en corto, para la etiqueta sobre el lienzo.
pub(crate) fn size_badge(size: domain::Size) -> String {
    format!(
        "{} · {} px",
        width_name(size.w),
        u32::from(size.h) * u32::from(domain::ROW_PX)
    )
}

/// Clases de un botón de una elección (ancho, tamaño de letra, alineación…).
/// Lo elegido va en un tono suave con borde, para no competir con Publicar.
/// (No usa `btn-soft`: daisyUI pinta lleno todo botón con
/// `aria-pressed="true"`, y las utilidades de Tailwind le ganan.)
/// Clase de un botón del interruptor Editar / Vista previa.
fn mode_class(active: bool) -> &'static str {
    if active {
        "btn btn-sm border-0 bg-base-100 shadow-sm"
    } else {
        "btn btn-sm btn-ghost border-0 font-normal text-base-content/80"
    }
}

pub(crate) fn choice_class(active: bool) -> &'static str {
    if active {
        "btn btn-sm border-primary bg-primary/10 text-primary hover:bg-primary/15"
    } else {
        "btn btn-sm"
    }
}

pub(crate) fn new_node_id(doc: &PageDoc) -> String {
    doc.fresh_id(&[])
}

/// Problemas de los campos de todos los bloques, en el orden de la página.
/// Primero las páginas (Inicio, después las demás) y al final la ficha del
/// producto.
pub(crate) fn problems(doc: &PageDoc) -> Vec<Problem> {
    let mut out = Vec::new();
    let layers = (0..doc.page_count())
        .map(Layer::Page)
        .chain([Layer::ProductModal]);
    for layer in layers {
        let mut nodes = doc.layer(layer).clone();
        grid::sort_by_position(&mut nodes);
        for node in &nodes {
            let Ok(value) = serde_json::to_value(&node.widget) else {
                continue;
            };
            let path = match layer {
                Layer::Page(0) => node.widget.label().to_string(),
                Layer::Page(i) => format!("Página {} › {}", doc.page_name(i), node.widget.label()),
                Layer::ProductModal => format!("Ficha del producto › {}", node.widget.label()),
            };
            let mut texts = Vec::new();
            field_problems(node.widget.fields(), value.get("props"), &path, &mut texts);
            out.extend(texts.into_iter().map(|text| Problem {
                node_id: node.id.clone(),
                layer,
                text,
            }));
        }
    }
    out
}

fn field_problems(
    fields: &[domain::Field],
    props: Option<&serde_json::Value>,
    path: &str,
    out: &mut Vec<String>,
) {
    for field in fields {
        let v = props.and_then(|p| p.get(field.key));
        if let Some(msg) = domain::field_error(&field.kind, v) {
            out.push(format!("{path} › {}: {msg}", field.label));
        }
        if let domain::FieldKind::List { item, .. } = &field.kind {
            for (i, it) in v
                .and_then(|v| v.as_array())
                .into_iter()
                .flatten()
                .enumerate()
            {
                let sub = format!("{path} › {} {}", field.label, i + 1);
                field_problems(item, Some(it), &sub, out);
            }
        }
    }
}

/// Enfoca el primer elemento que coincide con `selector`.
pub(crate) fn focus_selector(selector: &str) {
    if let Some(el) = document()
        .query_selector(selector)
        .ok()
        .flatten()
        .and_then(|e| e.dyn_into::<web_sys::HtmlElement>().ok())
    {
        _ = el.focus();
    }
}

/// Después de redibujar, lleva el foco al título de la lista de errores del
/// guardado.
fn focus_problems() {
    request_animation_frame(|| focus_selector("#mk-problems-title"));
}

/// Después de redibujar: lleva la vista al bloque, lo enfoca y, si `flash`,
/// lo resalta un instante.
pub(crate) fn reveal_node(id: &str, flash: bool) {
    let id = id.to_string();
    request_animation_frame(move || {
        let selector = format!("[data-node-id='{id}']");
        let Some(el) = document().query_selector(&selector).ok().flatten() else {
            return;
        };
        let opts = web_sys::ScrollIntoViewOptions::new();
        opts.set_block(web_sys::ScrollLogicalPosition::Nearest);
        opts.set_behavior(if flash {
            web_sys::ScrollBehavior::Smooth
        } else {
            web_sys::ScrollBehavior::Auto
        });
        el.scroll_into_view_with_scroll_into_view_options(&opts);
        if let Ok(html) = el.clone().dyn_into::<web_sys::HtmlElement>() {
            _ = html.focus_with_options(&{
                let o = web_sys::FocusOptions::new();
                o.set_prevent_scroll(true);
                o
            });
        }
        if flash {
            // El resalte va sobre la caja del bloque, no sobre su celda.
            let el = el
                .query_selector(".mk-anode-box")
                .ok()
                .flatten()
                .unwrap_or(el);
            _ = el.class_list().add_1("mk-flash");
            set_timeout(
                move || _ = el.class_list().remove_1("mk-flash"),
                Duration::from_millis(1600),
            );
        }
    });
}

pub(crate) fn storage() -> Option<web_sys::Storage> {
    window().local_storage().ok().flatten()
}

/// ¿Hay un menú de bloque abierto?
fn menu_open() -> bool {
    document()
        .query_selector(".mk-menu:popover-open")
        .ok()
        .flatten()
        .is_some()
}

/// ¿El foco está en un campo donde se escribe?
fn typing_in_field(ev: &web_sys::KeyboardEvent) -> bool {
    ev.target()
        .and_then(|t| t.dyn_into::<web_sys::HtmlElement>().ok())
        .is_some_and(|el| {
            matches!(el.tag_name().as_str(), "INPUT" | "TEXTAREA" | "SELECT")
                || el.is_content_editable()
        })
}

#[component]
pub fn Editor(initial: PageView, save: SaveAction, publish: PublishAction) -> impl IntoView {
    let state = EditorState::new(&initial);
    let products = StoredValue::new(initial.products.clone());
    let status = RwSignal::new(Status::Idle);
    let ui = Ui {
        toast: RwSignal::new(None),
        toast_seq: StoredValue::new(0),
        toast_hover: StoredValue::new(false),
        toast_focus: StoredValue::new(false),
        announce: RwSignal::new(String::new()),
        preview: RwSignal::new(false),
        store_open: RwSignal::new(false),
        new_drag: RwSignal::new(None),
        swallow_click: StoredValue::new(false),
        block_drag: StoredValue::new(false),
        show_errors: RwSignal::new(false),
        modal_product: RwSignal::new(None),
        return_to: StoredValue::new(None),
    };
    let actions = Actions { state, ui };
    provide_context(state);
    provide_context(ui);
    provide_context(actions);
    provide_context(photos::MyPhotos::new());
    // Documento enviado en el guardado en curso.
    let sent = RwSignal::new(None::<PageDoc>);
    // Borrador de una sesión anterior que se puede recuperar.
    let recover = RwSignal::new(None::<PageDoc>);
    let welcome = RwSignal::new(false);
    // Se tocó Publicar: en cuanto termine de guardarse, se publica.
    let publish_after = RwSignal::new(false);
    // Turno del guardado automático: solo guarda el último cambio.
    let autosave_seq = StoredValue::new(0u64);

    // Guarda el borrador en el servidor. Con `loud`, si hay errores los
    // muestra; sin `loud` (guardado automático) espera a que se corrijan.
    // Devuelve si se mandó a guardar.
    let save_draft = move |loud: bool| -> bool {
        if save.pending().get_untracked() || !state.dirty() {
            return false;
        }
        let doc = state.doc.get_untracked();
        let found = problems(&doc);
        if !found.is_empty() {
            if loud {
                // Los campos con error lo muestran aunque no se hayan tocado, y
                // el foco va a la lista de errores: el lector de pantalla la lee
                // y desde ahí se llega a cada bloque.
                ui.show_errors.set(true);
                status.set(Status::Problems(found));
                focus_problems();
            }
            return false;
        }
        if loud {
            state.end_group();
        }
        sent.set(Some(doc.clone()));
        if loud {
            status.set(Status::Idle);
        }
        save.dispatch((doc, state.base_version.get_untracked()));
        true
    };
    let save_now = move || {
        save_draft(true);
    };

    // Publicar: guarda lo que falte y después publica esa versión.
    let publish_now = move || {
        if publish.pending().get_untracked() || publish_after.get_untracked() {
            return;
        }
        if save.pending().get_untracked() {
            publish_after.set(true);
        } else if state.dirty() {
            publish_after.set(save_draft(true));
        } else if state.base_version.get_untracked() != state.published.get_untracked() {
            publish.dispatch(state.base_version.get_untracked());
        }
    };

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
                status.set(Status::Idle);
                ui.show_errors.set(false);
                if publish_after.get_untracked() {
                    // Si se siguió editando mientras se guardaba, primero se
                    // guarda eso también.
                    if !state.dirty() || !save_draft(true) {
                        publish_after.set(false);
                        if !state.dirty() {
                            publish.dispatch(version);
                        }
                    }
                }
            }
            Ok(SaveOutcome::Conflict { .. }) => {
                publish_after.set(false);
                status.set(Status::Conflict);
            }
            Ok(SaveOutcome::Invalid { errors }) => {
                publish_after.set(false);
                status.set(Status::Invalid(errors));
                focus_problems();
            }
            Err(e) => {
                publish_after.set(false);
                status.set(Status::Error(e.to_string()));
            }
        }
    });

    // Resultado de publicar.
    Effect::new(move |_| {
        let Some(result) = publish.value().get() else {
            return;
        };
        match result {
            Ok(version) => {
                state.published.set(version);
                ui.toast("Publicado. Tu tienda ya muestra estos cambios.", false);
            }
            Err(e) => status.set(Status::Error(e.to_string())),
        }
    });

    // Guardado automático: un rato después del último cambio, el borrador se
    // guarda solo. Los visitantes no lo ven hasta que se toca Publicar.
    Effect::new(move |prev: Option<()>| {
        let dirty = state.dirty();
        state.doc.track();
        if prev.is_none() || !dirty {
            return;
        }
        let seq = autosave_seq.get_value() + 1;
        autosave_seq.set_value(seq);
        set_timeout(
            move || {
                if autosave_seq.get_value() == seq {
                    save_draft(false);
                }
            },
            AUTOSAVE_DELAY,
        );
    });

    // Si se deshace la creación de una página (o se borra), la que se
    // estaba editando puede dejar de existir: se vuelve a Inicio.
    Effect::new(move |_| {
        let count = state.doc.with(PageDoc::page_count);
        if state.page.get() >= count {
            actions.open_page(0, true);
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

    // Al abrir: ¿hay un borrador sin guardar de otra sesión? ¿es la primera vez?
    Effect::new(move |_| {
        let Some(st) = storage() else {
            return;
        };
        if st.get_item(WELCOME_KEY).ok().flatten().is_none() {
            welcome.set(true);
        }
        let draft = st
            .get_item(DRAFT_KEY)
            .ok()
            .flatten()
            .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
            .and_then(|v| domain::parse_doc(v).ok());
        match draft {
            Some(doc) if state.saved.with_untracked(|s| s != &doc) => recover.set(Some(doc)),
            _ => _ = st.remove_item(DRAFT_KEY),
        }
    });

    // Cada cambio sin guardar queda también en el navegador.
    Effect::new(move |prev: Option<()>| {
        let dirty = state.dirty();
        let doc = state.doc.get();
        // La primera vez no se toca: puede haber un borrador por recuperar.
        if prev.is_none() {
            return;
        }
        let Some(st) = storage() else {
            return;
        };
        if dirty {
            if let Ok(json) = serde_json::to_string(&doc) {
                _ = st.set_item(DRAFT_KEY, &json);
            }
        } else if recover.with_untracked(Option::is_none) {
            _ = st.remove_item(DRAFT_KEY);
        }
    });

    // Atajos de teclado de todo el editor.
    Effect::new(move |_| {
        let handle = window_event_listener(leptos::ev::keydown, move |ev| {
            let key = ev.key();
            let cmd = ev.ctrl_key() || ev.meta_key();
            if cmd && !ev.alt_key() {
                match key.to_lowercase().as_str() {
                    "z" if ev.shift_key() => {
                        ev.prevent_default();
                        state.redo();
                    }
                    "z" => {
                        ev.prevent_default();
                        state.undo();
                    }
                    "y" => {
                        ev.prevent_default();
                        state.redo();
                    }
                    "s" => {
                        ev.prevent_default();
                        save_now();
                    }
                    _ => {}
                }
                return;
            }
            if typing_in_field(&ev) {
                return;
            }
            match key.as_str() {
                // Cancelar el arrastre lo resuelve el lienzo.
                "Escape"
                    if ui.new_drag.with_untracked(Option::is_some) || ui.block_drag.get_value() => {
                }
                // Un menú abierto se cierra solo con Esc; no suelta el bloque.
                "Escape" if menu_open() => {}
                "Escape" if ui.preview.get_untracked() => ui.preview.set(false),
                // Primero se suelta el bloque elegido; después se cierra la ficha.
                "Escape" if state.selected.with_untracked(Option::is_some) => {
                    state.selected.set(None)
                }
                "Escape" => actions.close_modal(),
                "Delete" | "Backspace" => {
                    if let Some(id) = state.selected.get_untracked() {
                        ev.prevent_default();
                        actions.delete(&id);
                    }
                }
                _ => {}
            }
        });
        on_cleanup(move || handle.remove());
    });

    view! {
        <div data-theme="light" class="flex h-screen flex-col bg-base-200 text-base-content">
            <header class="flex min-h-16 items-center gap-3 border-b border-base-300 bg-base-100 px-4">
                <h1 class="sr-only">"Editor de tu tienda"</h1>
                <StoreMenu/>
                <span class="h-7 w-px bg-base-300" aria-hidden="true"></span>
                <PageSwitcher/>
                <div class="flex flex-1 justify-center">
                    <div class="flex gap-0.5 rounded-field bg-base-200 p-0.5" role="group" aria-label="Modo">
                        <button
                            type="button"
                            class=move || mode_class(!ui.preview.get())
                            aria-pressed=move || (!ui.preview.get()).to_string()
                            on:click=move |_| ui.preview.set(false)
                        >
                            <Icon name="pencil"/>
                            "Editar"
                        </button>
                        <button
                            type="button"
                            class=move || mode_class(ui.preview.get())
                            aria-pressed=move || ui.preview.get().to_string()
                            on:click=move |_| {
                                // La vista previa muestra la tienda: la ficha se abre tocando un producto.
                                actions.close_modal();
                                ui.preview.set(true);
                            }
                        >
                            <Icon name="eye"/>
                            "Vista previa"
                        </button>
                    </div>
                </div>
                <div class="flex gap-0.5" role="group" aria-label="Historial">
                    <button
                        type="button"
                        class="btn btn-ghost btn-sm btn-square"
                        aria-label="Deshacer"
                        title="Deshacer (Ctrl+Z o ⌘Z)"
                        aria-keyshortcuts="Control+Z Meta+Z"
                        disabled=move || state.undo.with(Vec::is_empty)
                        on:click=move |_| state.undo()
                    >
                        <Icon name="undo" class="h-5 w-5"/>
                    </button>
                    <button
                        type="button"
                        class="btn btn-ghost btn-sm btn-square"
                        aria-label="Rehacer"
                        title="Rehacer (Ctrl+Mayús+Z o ⇧⌘Z)"
                        aria-keyshortcuts="Control+Shift+Z Meta+Shift+Z"
                        disabled=move || state.redo.with(Vec::is_empty)
                        on:click=move |_| state.redo()
                    >
                        <Icon name="redo" class="h-5 w-5"/>
                    </button>
                </div>
                <span class="h-7 w-px bg-base-300" aria-hidden="true"></span>
                <span
                    class="flex items-center gap-1.5 whitespace-nowrap text-sm"
                    role="status"
                    title=move || if state.unpublished() {
                        "Tus cambios se guardan solos. Tus clientes ven la última versión que publicaste."
                    } else {
                        "Tu tienda muestra exactamente esto."
                    }
                >
                    {move || match (state.dirty(), save.pending().get(), state.unpublished()) {
                        (true, true, _) => view! {
                            <span class="loading loading-spinner loading-xs" aria-hidden="true"></span>
                            <span>"Guardando…"</span>
                        }.into_any(),
                        (true, false, _) => view! {
                            <span class="h-2 w-2 rounded-full bg-base-content/40" aria-hidden="true"></span>
                            <span>"Sin guardar"</span>
                        }.into_any(),
                        (false, _, true) => view! {
                            <span class="h-2 w-2 rounded-full bg-warning" aria-hidden="true"></span>
                            <span>"Guardado, sin publicar"</span>
                        }.into_any(),
                        (false, _, false) => view! {
                            <Icon name="check" class="h-4 w-4 text-success"/>
                            <span>"Publicado"</span>
                        }.into_any(),
                    }}
                </span>
                <a
                    class="btn btn-ghost btn-sm btn-square"
                    href=move || state.doc.with(|d| d.page_path(state.page.get()).to_string())
                    target="_blank"
                    rel="noopener"
                    aria-label="Ver tienda publicada (se abre en otra pestaña)"
                    title="Ver tienda publicada"
                >
                    <Icon name="external" class="h-5 w-5"/>
                </a>
                <button
                    type="button"
                    class="btn btn-sm btn-primary"
                    title="Publicar: tus clientes ven estos cambios"
                    disabled=move || !state.unpublished() || publish.pending().get() || publish_after.get()
                    on:click=move |_| publish_now()
                >
                    <Icon name="up"/>
                    {move || if publish.pending().get() || publish_after.get() { "Publicando…" } else { "Publicar" }}
                </button>
            </header>
            {move || recover.get().map(|doc| {
                let doc = StoredValue::new(doc);
                view! {
                    <div class="flex items-center gap-3 border-b border-warning/40 bg-warning/15 px-4 py-2 text-sm" role="region" aria-label="Cambios recuperados">
                        <Icon name="info" class="h-5 w-5"/>
                        <span class="flex-1">"Encontramos cambios que no guardaste la última vez que usaste el editor."</span>
                        <button type="button" class="btn btn-sm btn-primary" on:click=move |_| {
                            let draft = doc.get_value();
                            state.apply(|d| *d = draft);
                            recover.set(None);
                            ui.toast("Recuperaste tus cambios. Se guardan solos; tocá Publicar cuando quieras que se vean en tu tienda.", false);
                        }>"Recuperar cambios"</button>
                        <button type="button" class="btn btn-sm" on:click=move |_| {
                            recover.set(None);
                            if let Some(st) = storage() {
                                _ = st.remove_item(DRAFT_KEY);
                            }
                        }>"Descartar"</button>
                    </div>
                }
            })}
            <div class="flex min-h-0 flex-1">
                {move || match (ui.preview.get(), ui.store_open.get()) {
                    (true, _) => ().into_any(),
                    (false, true) => view! { <store_drawer::StoreDrawer/> }.into_any(),
                    (false, false) => view! { <Palette products/> }.into_any(),
                }}
                {view! { <Canvas products welcome/> }.into_any()}
                {move || (!ui.preview.get()).then(|| view! { <Inspector products/> })}
            </div>
            {view! { <Notices status save_now/> }.into_any()}
            <div class="sr-only" role="status" aria-live="polite">{move || ui.announce.get()}</div>
        </div>
    }
}

/// Avisos flotantes abajo a la izquierda: el resultado del guardado y los
/// avisos breves. No mueven el resto de la pantalla.
#[component]
fn Notices(
    status: RwSignal<Status>,
    save_now: impl Fn() + Copy + Send + Sync + 'static,
) -> impl IntoView {
    let state = expect_context::<EditorState>();
    let ui = expect_context::<Ui>();
    let actions = expect_context::<Actions>();
    let close_alert = move |_| status.set(Status::Idle);
    // Cierra el aviso breve. Si el foco estaba en él, vuelve adonde estaba.
    let close = move || {
        let had_focus = ui.toast_focus.get_value();
        ui.close_toast();
        if had_focus {
            // El elemento de donde vino el foco quedó marcado al entrar.
            let back = document()
                .query_selector("[data-mk-focus-return]")
                .ok()
                .flatten();
            if let Some(el) = back.as_ref() {
                _ = el.remove_attribute("data-mk-focus-return");
            }
            match back.and_then(|e| e.dyn_into::<web_sys::HtmlElement>().ok()) {
                Some(el) => _ = el.focus(),
                None => focus_selector("#mk-inspector-title"),
            }
        }
    };
    let alert = move || {
        let close_btn = move || {
            view! {
                <button type="button" class="btn btn-ghost btn-sm btn-square" aria-label="Cerrar aviso" on:click=close_alert>
                    <Icon name="close"/>
                </button>
            }
        };
        match status.get() {
            Status::Idle => ().into_any(),
            Status::Conflict => view! {
                <div class="alert alert-warning shadow-lg">
                    <Icon name="alert" class="h-5 w-5"/>
                    <span>"Alguien guardó otra versión mientras editabas. Tus cambios quedaron guardados en este navegador: recargá y vas a poder recuperarlos."</span>
                    <button type="button" class="btn btn-sm" on:click=move |_| {
                        if let Some(w) = web_sys::window() {
                            _ = w.location().reload();
                        }
                    }>"Recargar"</button>
                </div>
            }.into_any(),
            Status::Problems(list) => {
                let n = list.len();
                view! {
                    <div class="alert items-start border-error bg-base-100 text-base-content shadow-lg">
                        <Icon name="alert" class="h-5 w-5 text-error"/>
                        <div class="flex flex-col gap-1">
                            <p id="mk-problems-title" tabindex="-1" class="font-semibold focus:outline-none">{if n == 1 {
                                "No se publicó: hay un dato para corregir.".to_string()
                            } else {
                                format!("No se publicó: hay {n} datos para corregir.")
                            }}</p>
                            <ul class="flex flex-col gap-1">
                                {list.into_iter().map(|p| {
                                    let id = p.node_id.clone();
                                    view! {
                                        <li>
                                            <button type="button" class="link text-left" on:click=move |_| actions.go_to(&id)>{p.text}</button>
                                        </li>
                                    }
                                }).collect_view()}
                            </ul>
                            <button type="button" class="btn btn-sm btn-primary mt-1 self-start" on:click=move |_| save_now()>"Intentar de nuevo"</button>
                        </div>
                        {close_btn()}
                    </div>
                }.into_any()
            }
            Status::Invalid(errors) => view! {
                <div class="alert items-start border-error bg-base-100 text-base-content shadow-lg">
                    <Icon name="alert" class="h-5 w-5 text-error"/>
                    <div>
                        <p id="mk-problems-title" tabindex="-1" class="font-semibold focus:outline-none">"No se pudo publicar:"</p>
                        <ul class="list-disc pl-5">
                            {errors.into_iter().map(|e| view! { <li>{e}</li> }).collect_view()}
                        </ul>
                    </div>
                    {close_btn()}
                </div>
            }.into_any(),
            Status::Error(e) => view! {
                <div class="alert items-start border-error bg-base-100 text-base-content shadow-lg">
                    <Icon name="alert" class="h-5 w-5 text-error"/>
                    <div>
                        <p class="font-semibold">"No se pudo publicar. Revisá tu conexión y probá de nuevo."</p>
                        <p class="text-sm text-base-content/75">{format!("Detalle: {e}")}</p>
                    </div>
                    {close_btn()}
                </div>
            }.into_any(),
        }
    };
    view! {
        <div class="pointer-events-none fixed bottom-4 left-4 z-50 flex max-w-lg flex-col gap-2 [&>*]:pointer-events-auto" role="region" aria-label="Avisos">
            <div role="alert">{alert}</div>
            <div role="status" aria-live="polite">
                {move || ui.toast.get().map(|t| view! {
                    <div
                        class="alert bg-neutral text-neutral-content shadow-lg"
                        on:pointerenter=move |_| ui.toast_hover.set_value(true)
                        on:pointerleave=move |_| ui.release_toast(true)
                        on:focusin=move |ev: leptos::ev::FocusEvent| {
                            // Recordar de dónde vino el foco para devolverlo al cerrar.
                            if !ui.toast_focus.get_value() {
                                if let Ok(list) = document().query_selector_all("[data-mk-focus-return]") {
                                    for i in 0..list.length() {
                                        if let Some(el) = list.get(i).and_then(|n| n.dyn_into::<web_sys::Element>().ok()) {
                                            _ = el.remove_attribute("data-mk-focus-return");
                                        }
                                    }
                                }
                                if let Some(from) = ev.related_target().and_then(|t| t.dyn_into::<web_sys::Element>().ok()) {
                                    _ = from.set_attribute("data-mk-focus-return", "");
                                }
                            }
                            ui.toast_focus.set_value(true);
                        }
                        on:focusout=move |ev: leptos::ev::FocusEvent| {
                            let inside = ev
                                .related_target()
                                .and_then(|t| t.dyn_into::<web_sys::Node>().ok())
                                .zip(ev.current_target().and_then(|t| t.dyn_into::<web_sys::Node>().ok()))
                                .is_some_and(|(to, toast)| toast.contains(Some(&to)));
                            if !inside {
                                ui.release_toast(false);
                            }
                        }
                    >
                        <span>{t.text}</span>
                        {t.undo.then(|| view! {
                            <button type="button" class="btn btn-sm" on:click=move |_| {
                                state.undo();
                                close();
                            }>"Deshacer"</button>
                        })}
                        <button type="button" class="btn btn-ghost btn-sm btn-square text-neutral-content hover:bg-white/10" aria-label="Cerrar aviso" on:click=move |_| close()>
                            <Icon name="close"/>
                        </button>
                    </div>
                })}
            </div>
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::Widget;

    #[test]
    fn problems_name_block_and_field() {
        let mut doc = PageDoc::default();
        let mut node = Node::new("n1", Widget::defaults("Banner").unwrap());
        if let Widget::Banner(b) = &mut node.widget {
            b.image_url = "www.ejemplo.com".into();
        }
        doc.nodes.push(node);
        let found = problems(&doc);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].node_id, "n1");
        assert!(found[0]
            .text
            .starts_with("Banner › Imagen: Escribí una dirección"));
        assert_eq!(new_node_id(&doc), "n2");
    }

    #[test]
    fn product_modal_problems_say_they_are_in_the_modal() {
        let mut doc = PageDoc::default();
        if let Widget::ProductButton(b) = &mut doc.product_modal.nodes[4].widget {
            b.href = "www.ejemplo.com".into();
        }
        let found = problems(&doc);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].layer, Layer::ProductModal);
        assert_eq!(found[0].node_id, "m5");
        assert!(found[0]
            .text
            .starts_with("Ficha del producto › Botón de la ficha › Enlace del botón"));
        // Los ids nuevos no chocan con los de la ficha.
        doc.nodes
            .push(Node::new("n1", Widget::defaults("Text").unwrap()));
        doc.product_modal.nodes[0].id = "n2".into();
        assert_eq!(new_node_id(&doc), "n3");
    }

    #[test]
    fn list_item_problems_say_which_item() {
        let mut doc = PageDoc::default();
        let mut node = Node::new("n1", Widget::defaults("Carousel").unwrap());
        if let Widget::Carousel(c) = &mut node.widget {
            c.slides[1].button_href = "javascript:alert(1)".into();
        }
        doc.nodes.push(node);
        assert_eq!(
            problems(&doc)[0].text,
            "Carrusel › Diapositivas 2 › Enlace del botón: Escribí una dirección que empiece con https://, http:// o /."
        );
    }
}
