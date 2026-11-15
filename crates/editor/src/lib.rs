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
mod inspector;
mod palette;
mod typography;

use domain::{Node, PageDoc, PageView, SaveOutcome, THEMES, THEME_LABELS};
use leptos::prelude::*;
use std::time::Duration;
use wasm_bindgen::JsCast;

use canvas::Canvas;
use icons::Icon;
use inspector::Inspector;
use palette::Palette;
use typography::StoreTypography;

/// Cuántos pasos de deshacer se guardan.
pub const MAX_UNDO: usize = 50;

/// Acción de guardado: recibe el documento y la versión base.
pub type SaveAction = Action<(PageDoc, u32), Result<SaveOutcome, ServerFnError>>;

/// Clave del borrador guardado en el navegador.
const DRAFT_KEY: &str = "mk-editor-borrador:home";
/// Clave que recuerda que ya se cerró la guía de primeros pasos.
pub(crate) const WELCOME_KEY: &str = "mk-editor-bienvenida";

/// Todo el estado del documento en un solo lugar. Toda modificación pasa
/// por [`EditorState::apply`] o [`EditorState::apply_grouped`].
#[derive(Clone, Copy)]
pub(crate) struct EditorState {
    pub doc: RwSignal<PageDoc>,
    pub saved: RwSignal<PageDoc>,
    pub base_version: RwSignal<u32>,
    pub selected: RwSignal<Option<String>>,
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
            selected: RwSignal::new(None),
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

    pub fn dirty(&self) -> bool {
        self.doc.with(|d| self.saved.with(|s| d != s))
    }

    pub fn node(&self, id: &str) -> Option<Node> {
        self.doc
            .with_untracked(|d| d.nodes.iter().find(|n| n.id == id).cloned())
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
        let mut nodes = self.state.doc.with_untracked(|d| d.nodes.clone());
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
            .apply(|d| grid::insert_at(&mut d.nodes, node, x, y));
        self.state.selected.set(Some(id.clone()));
        self.ui
            .announce(format!("Agregaste {label}. {}", self.position_text(&id)));
        reveal_node(&id, true);
    }

    pub fn add(&self, type_name: &str) {
        let Some(widget) = domain::Widget::defaults(type_name) else {
            return;
        };
        let label = widget.label();
        let id = new_node_id(&self.state.doc.get_untracked());
        let node = Node::new(id.clone(), widget);
        self.state.apply(|d| grid::add_node(&mut d.nodes, node));
        self.state.selected.set(Some(id.clone()));
        self.ui
            .announce(format!("Agregaste {label} al final de la página."));
        reveal_node(&id, true);
    }

    pub fn move_up(&self, id: &str) {
        self.state.apply(|d| grid::move_up(&mut d.nodes, id));
        self.after_move(id);
    }

    pub fn move_down(&self, id: &str) {
        self.state.apply(|d| grid::move_down(&mut d.nodes, id));
        self.after_move(id);
    }

    /// Mueve una columna a la izquierda (`-1`) o a la derecha (`1`).
    pub fn shift(&self, id: &str, dx: i32) {
        self.state.apply(|d| {
            if let Some(n) = d.nodes.iter().find(|n| n.id == id) {
                let (x, y) = (i32::from(n.pos.x) + dx, i32::from(n.pos.y));
                grid::move_node(&mut d.nodes, id, x, y);
            }
        });
        self.after_move(id);
    }

    pub fn resize_by(&self, id: &str, dw: i32, dh: i32) {
        self.state.apply(|d| {
            if let Some(n) = d.nodes.iter().find(|n| n.id == id) {
                let (w, h) = (i32::from(n.size.w) + dw, i32::from(n.size.h) + dh);
                grid::resize_node(&mut d.nodes, id, w, h);
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
        let label = copy.widget.label();
        self.state
            .apply(|d| grid::insert_below(&mut d.nodes, id, copy));
        self.state.selected.set(Some(new_id.clone()));
        self.ui
            .announce(format!("Duplicaste {label}. La copia quedó debajo."));
        reveal_node(&new_id, true);
    }

    pub fn delete(&self, id: &str) {
        let label = self.label(id);
        self.state.apply(|d| grid::remove_node(&mut d.nodes, id));
        self.state.selected.set(None);
        self.ui.toast(format!("{label} borrado."), true);
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
/// Lo elegido va en un tono suave con borde, para no competir con Guardar.
/// (No usa `btn-soft`: daisyUI pinta lleno todo botón con
/// `aria-pressed="true"`, y las utilidades de Tailwind le ganan.)
pub(crate) fn choice_class(active: bool) -> &'static str {
    if active {
        "btn btn-sm border-primary bg-primary/10 text-primary hover:bg-primary/15"
    } else {
        "btn btn-sm"
    }
}

pub(crate) fn new_node_id(doc: &PageDoc) -> String {
    (1..)
        .map(|k| format!("n{k}"))
        .find(|id| doc.nodes.iter().all(|n| &n.id != id))
        .expect("siempre hay un id libre")
}

/// Problemas de los campos de todos los bloques, en el orden de la página.
pub(crate) fn problems(doc: &PageDoc) -> Vec<Problem> {
    let mut nodes = doc.nodes.clone();
    grid::sort_by_position(&mut nodes);
    let mut out = Vec::new();
    for node in &nodes {
        let Ok(value) = serde_json::to_value(&node.widget) else {
            continue;
        };
        let mut texts = Vec::new();
        field_problems(
            node.widget.fields(),
            value.get("props"),
            node.widget.label(),
            &mut texts,
        );
        out.extend(texts.into_iter().map(|text| Problem {
            node_id: node.id.clone(),
            text,
        }));
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
pub fn Editor(initial: PageView, save: SaveAction) -> impl IntoView {
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
        new_drag: RwSignal::new(None),
        swallow_click: StoredValue::new(false),
        block_drag: StoredValue::new(false),
        show_errors: RwSignal::new(false),
    };
    let actions = Actions { state, ui };
    provide_context(state);
    provide_context(ui);
    provide_context(actions);
    // Documento enviado en el guardado en curso.
    let sent = RwSignal::new(None::<PageDoc>);
    // Borrador de una sesión anterior que se puede recuperar.
    let recover = RwSignal::new(None::<PageDoc>);
    let welcome = RwSignal::new(false);

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
                ui.toast("Guardado. Tu tienda ya muestra estos cambios.", false);
            }
            Ok(SaveOutcome::Conflict { .. }) => status.set(Status::Conflict),
            Ok(SaveOutcome::Invalid { errors }) => {
                status.set(Status::Invalid(errors));
                focus_problems();
            }
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

    let save_now = move || {
        if save.pending().get_untracked() || !state.dirty() {
            return;
        }
        state.end_group();
        let doc = state.doc.get_untracked();
        let found = problems(&doc);
        if !found.is_empty() {
            // Los campos con error lo muestran aunque no se hayan tocado, y
            // el foco va a la lista de errores: el lector de pantalla la lee
            // y desde ahí se llega a cada bloque.
            ui.show_errors.set(true);
            status.set(Status::Problems(found));
            focus_problems();
            return;
        }
        sent.set(Some(doc.clone()));
        status.set(Status::Idle);
        save.dispatch((doc, state.base_version.get_untracked()));
    };

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
                "Escape" if ui.preview.get_untracked() => ui.preview.set(false),
                "Escape" => state.selected.set(None),
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

    let theme = move || state.doc.with(|d| d.theme.clone());

    view! {
        <div data-theme="light" class="flex h-screen flex-col bg-base-200 text-base-content">
            <header class="navbar min-h-14 gap-3 border-b border-base-300 bg-base-100 px-4">
                <div class="flex flex-1 items-center gap-4">
                    <h1 class="text-lg font-bold">"Editor de tu tienda"</h1>
                    <a class="link link-hover inline-flex items-center gap-1 text-sm" href="/" target="_blank" rel="noopener">
                        "Ver tienda publicada"
                        <Icon name="external" class="h-3.5 w-3.5"/>
                        <span class="sr-only">"(se abre en otra pestaña)"</span>
                    </a>
                </div>
                <label class="flex items-center gap-2 text-sm">
                    "Tema"
                    <span data-theme=theme class="flex gap-0.5 rounded-field border border-base-content/20 bg-base-100 p-1" aria-hidden="true">
                        <span class="h-3 w-3 rounded-full bg-primary"></span>
                        <span class="h-3 w-3 rounded-full bg-secondary"></span>
                        <span class="h-3 w-3 rounded-full bg-accent"></span>
                    </span>
                    <select
                        class="select select-sm w-36"
                        prop:value=theme
                        on:change=move |ev| {
                            let theme = event_target_value(&ev);
                            state.apply(|d| d.theme = theme);
                        }
                    >
                        {THEMES
                            .iter()
                            .zip(THEME_LABELS)
                            .map(|(t, l)| view! { <option value=*t>{*l}</option> })
                            .collect_view()}
                    </select>
                </label>
                <StoreTypography/>
                <button
                    type="button"
                    class="btn btn-sm"
                    aria-pressed=move || ui.preview.get().to_string()
                    on:click=move |_| ui.preview.update(|p| *p = !*p)
                >
                    <Icon name="eye"/>
                    "Vista previa"
                </button>
                <button
                    type="button"
                    class="btn btn-sm"
                    title="Deshacer (Ctrl+Z o ⌘Z)"
                    aria-keyshortcuts="Control+Z Meta+Z"
                    disabled=move || state.undo.with(Vec::is_empty)
                    on:click=move |_| state.undo()
                >
                    <Icon name="undo"/>
                    "Deshacer"
                </button>
                <button
                    type="button"
                    class="btn btn-sm"
                    title="Rehacer (Ctrl+Mayús+Z o ⇧⌘Z)"
                    aria-keyshortcuts="Control+Shift+Z Meta+Shift+Z"
                    disabled=move || state.redo.with(Vec::is_empty)
                    on:click=move |_| state.redo()
                >
                    <Icon name="redo"/>
                    "Rehacer"
                </button>
                <span class="flex items-center gap-1.5 text-sm" title="La tienda publicada muestra lo último que guardaste">
                    {move || if state.dirty() {
                        view! {
                            <span class="h-2 w-2 rounded-full bg-warning" aria-hidden="true"></span>
                            <span>"Cambios sin guardar"</span>
                        }.into_any()
                    } else {
                        view! {
                            <Icon name="check" class="h-4 w-4 text-success"/>
                            <span>"Todo guardado"</span>
                        }.into_any()
                    }}
                </span>
                <button
                    type="button"
                    class="btn btn-sm btn-primary"
                    title="Guardar (Ctrl+S o ⌘S)"
                    aria-keyshortcuts="Control+S Meta+S"
                    disabled=move || !state.dirty() || save.pending().get()
                    on:click=move |_| save_now()
                >
                    {move || if save.pending().get() { "Guardando…" } else { "Guardar" }}
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
                            ui.toast("Recuperaste tus cambios. Guardalos para publicarlos.", false);
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
                {move || (!ui.preview.get()).then(|| view! { <Palette/> })}
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
                                "No se guardó: hay un dato para corregir.".to_string()
                            } else {
                                format!("No se guardó: hay {n} datos para corregir.")
                            }}</p>
                            <ul class="flex flex-col gap-1">
                                {list.into_iter().map(|p| {
                                    let id = p.node_id.clone();
                                    view! {
                                        <li>
                                            <button type="button" class="link text-left" on:click=move |_| {
                                                state.selected.set(Some(id.clone()));
                                                reveal_node(&id, true);
                                            }>{p.text}</button>
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
                        <p id="mk-problems-title" tabindex="-1" class="font-semibold focus:outline-none">"No se pudo guardar:"</p>
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
                        <p class="font-semibold">"No se pudo guardar. Revisá tu conexión y probá de nuevo."</p>
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
