//! Asistente de primera página: cinco pasos (encabezado, portada, filtros,
//! productos y pie) que arman o reemplazan esos bloques en Inicio.
//!
//! Todo es lógica pura: las opciones de cada paso, cómo se convierte lo que
//! elige la persona en bloques y qué cambia en una página que ya existe. La
//! ventana con las animaciones vive en el crate del editor.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::{
    font, normalize_hex, safe_bg_image, starter_doc, CardOptions, Field, FieldKind, Look, Node, PageDoc, Size,
    Slide, StoreStyle, Widget, ALIGN_OPTIONS, GRID_COLS, MAX_BG_DIM, ROW_PX, TEXT_ANIMATIONS,
};

/// Cuántos pasos tiene el asistente.
pub const WIZARD_STEPS: usize = 5;

/// Cada bloque que arma el asistente. El orden es el de la página, de arriba
/// hacia abajo, y también el de los pasos.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Section {
    Nav,
    Hero,
    Filters,
    Products,
    Footer,
}

impl Section {
    pub const ALL: [Section; WIZARD_STEPS] = [
        Section::Nav,
        Section::Hero,
        Section::Filters,
        Section::Products,
        Section::Footer,
    ];

    pub fn index(self) -> usize {
        Self::ALL.iter().position(|s| *s == self).unwrap_or(0)
    }

    /// A qué paso pertenece un bloque que ya está en la página.
    pub fn of(widget: &Widget) -> Option<Section> {
        Some(match widget {
            Widget::Navbar(_) => Section::Nav,
            Widget::Banner(_) | Widget::Carousel(_) => Section::Hero,
            Widget::TagFilter(_)
            | Widget::TagList(_)
            | Widget::TagCircles(_)
            | Widget::TagSearch(_)
            | Widget::CategoryTabs(_)
            | Widget::CategorySide(_)
            | Widget::CategoryCards(_)
            | Widget::CategoryDrop(_) => Section::Filters,
            Widget::ProductGrid(_) | Widget::ProductCarousel(_) => Section::Products,
            Widget::Footer(_) => Section::Footer,
            _ => return None,
        })
    }
}

/// Una opción que se puede elegir en un paso.
#[derive(Debug, PartialEq)]
pub struct WizardOption {
    pub name: &'static str,
    pub desc: &'static str,
}

/// Textos de un paso: qué se elige y cómo se lo explicamos.
#[derive(Debug, PartialEq)]
pub struct WizardStep {
    pub section: Section,
    /// Nombre corto, para la barra de pasos y el panel.
    pub name: &'static str,
    /// La tarea, en una frase.
    pub title: &'static str,
    /// Qué es ese bloque, en una línea.
    pub hint: &'static str,
    pub options: &'static [WizardOption],
}

const fn opt(name: &'static str, desc: &'static str) -> WizardOption {
    WizardOption { name, desc }
}

pub const WIZARD: [WizardStep; WIZARD_STEPS] = [
    WizardStep {
        section: Section::Nav,
        name: "Encabezado",
        title: "Elegí el encabezado de tu tienda",
        hint: "Es la barra de arriba, con tu logo y el menú. Mirá las opciones con las flechas.",
        options: &[
            opt("Clásico", "Logo a la izquierda y enlaces a la derecha"),
            opt("Logo al centro", "Enlaces repartidos a los costados"),
            opt("Flotante", "Se apoya sobre la portada"),
        ],
    },
    WizardStep {
        section: Section::Hero,
        name: "Portada",
        title: "Elegí la portada",
        hint: "Es lo primero que ve tu cliente. Cambiá de opción con las flechas.",
        options: &[
            opt("Banner", "Una imagen grande con título"),
            opt("Carrusel", "Varias imágenes que pasan solas"),
            opt("Sin portada", "Empezar directo con los productos"),
        ],
    },
    WizardStep {
        section: Section::Filters,
        name: "Filtros",
        title: "Elegí cómo buscan tus clientes",
        hint: "Los filtros ayudan a encontrar productos. Si no los necesitás, elegí «Sin filtros».",
        options: &[
            opt("Pestañas por categoría", "Una fila con tus categorías"),
            opt("Buscador y filtros", "Búsqueda y menú de etiquetas"),
            opt("Sin filtros", "Solo los productos"),
        ],
    },
    WizardStep {
        section: Section::Products,
        name: "Productos",
        title: "Elegí cómo se muestran tus productos",
        hint: "Se usan los que cargues. Mientras tanto ves productos de ejemplo.",
        options: &[
            opt("Cuadrícula", "Tarjetas simples, una al lado de la otra"),
            opt("Cuadrícula con detalle", "Con precios por transferencia y cuotas"),
            opt("Carrusel", "Pasan de costado con flechas"),
        ],
    },
    WizardStep {
        section: Section::Footer,
        name: "Pie de página",
        title: "Elegí el pie de página",
        hint: "Es la franja del final, con tus enlaces. Después tocá «Usar este y crear mi página».",
        options: &[
            opt("Clásico", "Fondo simple, enlaces claros"),
            opt("Suave", "Fondo con un tono suave"),
            opt("Intenso", "Más contraste, con color"),
        ],
    },
];

/// Colores que se ofrecen en los pasos (los del tema de la tienda).
pub const WIZARD_COLORS: [(&str, &str); 4] = [
    ("", "Principal"),
    ("secondary", "Secundario"),
    ("accent", "Acento"),
    ("neutral", "Neutro"),
];

pub const NAV_TONES: [(&str, &str); 3] = [("", "Claro"), ("glass", "Vidrio"), ("dark", "Oscuro")];
pub const HERO_HEIGHTS: [&str; 3] = ["Baja", "Media", "Alta"];
pub const PRODUCT_COUNTS: [(&str, i64); 3] = [("Pocos", 4), ("Varios", 8), ("Muchos", 12)];

/// Fondo de la web: cómo se arma (índice de [`BG_KINDS`]).
pub const BG_KINDS: [&str; 3] = ["Sin cambios", "Color", "Imagen"];

/// Lo que se eligió en el asistente. Se guarda como borrador mientras se usa.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct WizardPick {
    /// Tipo de fondo de la web (índice de [`BG_KINDS`]).
    pub bg_kind: u8,
    /// Color del fondo (o de respaldo bajo la imagen), `#rrggbb`; vacío: el del tema.
    pub bg_color: String,
    /// Imagen del fondo.
    pub bg_image: String,
    /// Cuánto se oscurece la imagen, en %.
    pub bg_dim: u8,
    /// La imagen queda quieta al desplazar la página.
    pub bg_fixed: bool,
    /// Ajustes de cada bloque: campo → valor (solo los que la persona tocó).
    pub over: [BTreeMap<String, serde_json::Value>; WIZARD_STEPS],
    /// Estilo visual (1 a 5) de cada bloque; 0 deja el que tiene.
    pub design: [u8; WIZARD_STEPS],
    /// Letra, alineación y animación del texto de cada bloque; vacío: la que tiene.
    pub font: [String; WIZARD_STEPS],
    pub align: [String; WIZARD_STEPS],
    pub anim: [String; WIZARD_STEPS],
    /// Opción elegida en cada paso.
    pub choice: [u8; WIZARD_STEPS],
    /// Color de cada bloque (índice de [`WIZARD_COLORS`]).
    pub color: [u8; WIZARD_STEPS],
    /// Fondo del encabezado (índice de [`NAV_TONES`]).
    pub nav_tone: u8,
    /// Buscador de productos en el encabezado.
    pub nav_search: bool,
    /// Altura de la portada (índice de [`HERO_HEIGHTS`]).
    pub hero_height: u8,
    /// Cuántos productos se muestran (índice de [`PRODUCT_COUNTS`]).
    pub products_count: u8,
}

impl Default for WizardPick {
    fn default() -> Self {
        Self {
            bg_kind: 0,
            bg_color: String::new(),
            bg_image: String::new(),
            bg_dim: 30,
            bg_fixed: false,
            over: Default::default(),
            design: [0; WIZARD_STEPS],
            font: Default::default(),
            align: Default::default(),
            anim: Default::default(),
            choice: [0; WIZARD_STEPS],
            color: [0; WIZARD_STEPS],
            nav_tone: 0,
            nav_search: false,
            hero_height: 1,
            products_count: 1,
        }
    }
}

impl WizardPick {
    /// Deja todo dentro de lo que existe (un borrador viejo o a mano no rompe nada).
    pub fn clamped(mut self) -> Self {
        for (i, step) in WIZARD.iter().enumerate() {
            self.choice[i] = self.choice[i].min(step.options.len() as u8 - 1);
            self.color[i] = self.color[i].min(WIZARD_COLORS.len() as u8 - 1);
        }
        self.nav_tone = self.nav_tone.min(NAV_TONES.len() as u8 - 1);
        self.hero_height = self.hero_height.min(HERO_HEIGHTS.len() as u8 - 1);
        self.products_count = self.products_count.min(PRODUCT_COUNTS.len() as u8 - 1);
        self.bg_kind = self.bg_kind.min(BG_KINDS.len() as u8 - 1);
        self.bg_dim = self.bg_dim.min(MAX_BG_DIM);
        if normalize_hex(&self.bg_color).as_deref() != Some(&self.bg_color) {
            self.bg_color.clear();
        }
        if !safe_bg_image(&self.bg_image) {
            self.bg_image.clear();
        }
        for i in 0..WIZARD_STEPS {
            self.design[i] = self.design[i].min(5);
            if !self.font[i].is_empty() && font(&self.font[i]).is_none() {
                self.font[i].clear();
            }
            if !ALIGN_OPTIONS.iter().any(|(k, _)| *k == self.align[i]) {
                self.align[i].clear();
            }
            if !self.anim[i].is_empty() && !TEXT_ANIMATIONS.iter().any(|(k, ..)| *k == self.anim[i]) {
                self.anim[i].clear();
            }
            self.over[i].retain(|k, v| {
                k.len() <= 40 && !matches!(v, serde_json::Value::Array(_) | serde_json::Value::Object(_))
                    && v.as_str().is_none_or(|t| t.chars().count() <= 300)
            });
        }
        self
    }

    /// ¿El paso termina sin bloque («Sin portada», «Sin filtros»)?
    pub fn is_empty_choice(&self, section: Section) -> bool {
        matches!(
            (section, self.choice[section.index()]),
            (Section::Hero, 2) | (Section::Filters, 2)
        )
    }
}

fn existing_of<'a>(doc: &'a PageDoc, section: Section) -> Option<&'a Node> {
    doc.nodes
        .iter()
        .filter(|n| Section::of(&n.widget) == Some(section))
        .min_by_key(|n| (n.pos.y, n.pos.x))
}

/// Lo que se elige en la página que ya existe, para que al reabrir el
/// asistente las opciones arranquen en lo que la persona tiene hoy.
pub fn pick_from_doc(doc: &PageDoc) -> WizardPick {
    let mut p = WizardPick::default();
    if let Some(Widget::Navbar(n)) = existing_of(doc, Section::Nav).map(|n| &n.widget) {
        p.choice[0] = match n.variant.as_str() {
            "c" => 1,
            "b" => 2,
            _ => 0,
        };
        p.nav_tone = NAV_TONES
            .iter()
            .position(|(t, _)| *t == n.tone)
            .unwrap_or(0) as u8;
        p.nav_search = n.search;
    }
    p.bg_color = doc.style.bg_color.clone();
    p.bg_image = doc.style.bg_image.clone();
    p.bg_dim = doc.style.bg_dim;
    p.bg_fixed = doc.style.bg_fixed;
    p.bg_kind = if !p.bg_image.is_empty() {
        2
    } else if !p.bg_color.is_empty() {
        1
    } else {
        0
    };
    p.choice[1] = match existing_of(doc, Section::Hero).map(|n| &n.widget) {
        Some(Widget::Banner(_)) => 0,
        Some(Widget::Carousel(_)) => 1,
        _ => 2,
    };
    p.choice[2] = match existing_of(doc, Section::Filters).map(|n| &n.widget) {
        Some(Widget::CategoryTabs(_)) => 0,
        Some(Widget::TagSearch(_)) => 1,
        Some(_) => 0,
        None => 2,
    };
    if let Some(node) = existing_of(doc, Section::Products) {
        p.choice[3] = match &node.widget {
            Widget::ProductGrid(g) if g.cards.card == "full" => 1,
            Widget::ProductCarousel(_) => 2,
            _ => 0,
        };
        let limit = match &node.widget {
            Widget::ProductGrid(g) => g.limit,
            Widget::ProductCarousel(c) => c.limit,
            _ => 8,
        };
        p.products_count = PRODUCT_COUNTS
            .iter()
            .position(|(_, n)| *n >= limit)
            .unwrap_or(1) as u8;
    }
    for section in Section::ALL {
        if let Some(node) = existing_of(doc, section) {
            if let Some(i) = WIZARD_COLORS.iter().position(|(c, _)| *c == node.look.color) {
                p.color[section.index()] = i as u8;
            }
            if section == Section::Footer {
                p.choice[4] = match node.look.design {
                    3 => 1,
                    4 => 2,
                    _ => 0,
                };
            }
        }
    }
    p.clamped()
}

/// ¿La página es la inicial, tal como se crea una tienda nueva (sin plantilla
/// y sin bloques agregados ni quitados)? Solo entonces el asistente se abre solo.
pub fn is_untouched(doc: &PageDoc) -> bool {
    let kinds = |d: &PageDoc| {
        d.nodes
            .iter()
            .map(|n| std::mem::discriminant(&n.widget))
            .collect::<Vec<_>>()
    };
    doc.pages.is_empty() && kinds(doc) == kinds(&starter_doc(""))
}

/// El bloque que sale de lo elegido en un paso, aprovechando los textos del
/// bloque que ya está en la página (si lo hay). `None`: sin bloque.
pub fn section_widget(section: Section, pick: &WizardPick, existing: Option<&Widget>) -> Option<Widget> {
    let widget = base_widget(section, pick, existing)?;
    Some(with_overrides(widget, &pick.over[section.index()]))
}

/// Los ajustes que tocó la persona, sobre campos que el bloque ya tiene.
/// Si el valor no le sirve al bloque, se deja como estaba.
fn with_overrides(widget: Widget, over: &BTreeMap<String, serde_json::Value>) -> Widget {
    if over.is_empty() {
        return widget;
    }
    let Ok(mut v) = serde_json::to_value(&widget) else {
        return widget;
    };
    let fields = widget.fields();
    if let Some(props) = v.get_mut("props").and_then(|p| p.as_object_mut()) {
        for (k, val) in over {
            if !props.contains_key(k) {
                continue;
            }
            let val = match (fields.iter().find(|f| f.key == k.as_str()), val) {
                // Un texto más largo que el límite del campo haría que la
                // página no se pudiera guardar: se recorta.
                (Some(Field { kind: FieldKind::Text { max } | FieldKind::LongText { max }, .. }), serde_json::Value::String(t)) => {
                    serde_json::Value::String(t.chars().take(*max).collect())
                }
                (Some(Field { kind: FieldKind::Number { min, max }, .. }), serde_json::Value::Number(n)) => {
                    match n.as_i64() {
                        Some(i) => serde_json::Value::from(i.clamp(*min, *max)),
                        None => val.clone(),
                    }
                }
                _ => val.clone(),
            };
            props.insert(k.clone(), val);
        }
    }
    serde_json::from_value::<Widget>(v).unwrap_or(widget)
}

/// Largo máximo de un campo de texto del bloque (`None`: sin límite conocido).
pub fn text_max(widget: &Widget, key: &str) -> Option<usize> {
    widget.fields().iter().find(|f| f.key == key).and_then(|f| match f.kind {
        FieldKind::Text { max } | FieldKind::LongText { max } => Some(max),
        _ => None,
    })
}

fn base_widget(section: Section, pick: &WizardPick, existing: Option<&Widget>) -> Option<Widget> {
    let choice = pick.choice[section.index()];
    match section {
        Section::Nav => {
            let Some(Widget::Navbar(mut n)) = existing
                .cloned()
                .filter(|w| matches!(w, Widget::Navbar(_)))
                .or_else(|| Widget::defaults("Navbar"))
            else {
                return None;
            };
            n.variant = ["a", "c", "b"][usize::from(choice)].to_string();
            n.tone = NAV_TONES[usize::from(pick.nav_tone)].0.to_string();
            n.search = pick.nav_search;
            Some(Widget::Navbar(n))
        }
        Section::Hero => match choice {
            0 => {
                let banner = match existing {
                    Some(Widget::Banner(b)) => Some(b.clone()),
                    Some(Widget::Carousel(c)) => c.slides.first().map(|s| crate::BannerProps {
                        title: s.title.clone(),
                        subtitle: s.subtitle.clone(),
                        image_url: s.image_url.clone(),
                        button_label: s.button_label.clone(),
                        button_href: s.button_href.clone(),
                    }),
                    _ => None,
                };
                match banner {
                    Some(b) => Some(Widget::Banner(b)),
                    None => Widget::defaults("Banner"),
                }
            }
            1 => match existing {
                Some(Widget::Carousel(c)) => Some(Widget::Carousel(c.clone())),
                other => {
                    let mut w = Widget::defaults("Carousel")?;
                    if let (Widget::Carousel(c), Some(Widget::Banner(b))) = (&mut w, other) {
                        if let Some(first) = c.slides.first_mut() {
                            *first = Slide {
                                image_url: b.image_url.clone(),
                                title: b.title.clone(),
                                subtitle: b.subtitle.clone(),
                                button_label: b.button_label.clone(),
                                button_href: b.button_href.clone(),
                            };
                        }
                    }
                    Some(w)
                }
            },
            _ => None,
        },
        Section::Filters => {
            let name = match choice {
                0 => "CategoryTabs",
                1 => "TagSearch",
                _ => return None,
            };
            let fresh = Widget::defaults(name)?;
            match existing {
                Some(w) if std::mem::discriminant(w) == std::mem::discriminant(&fresh) => {
                    Some(w.clone())
                }
                _ => Some(fresh),
            }
        }
        Section::Products => {
            let limit = PRODUCT_COUNTS[usize::from(pick.products_count)].1;
            let (title, cards) = match existing {
                Some(Widget::ProductGrid(g)) => (Some(g.title.clone()), Some(g.cards.clone())),
                Some(Widget::ProductCarousel(c)) => (Some(c.title.clone()), Some(c.cards.clone())),
                _ => (None, None),
            };
            let card = if choice == 1 { "full" } else { "" };
            match choice {
                0 | 1 => {
                    let Widget::ProductGrid(mut g) = Widget::defaults("ProductGrid")? else {
                        return None;
                    };
                    if let Some(t) = title {
                        g.title = t;
                    }
                    g.cards = cards.unwrap_or_else(CardOptions::default);
                    g.cards.card = card.to_string();
                    g.limit = limit;
                    Some(Widget::ProductGrid(g))
                }
                _ => {
                    let Widget::ProductCarousel(mut c) = Widget::defaults("ProductCarousel")?
                    else {
                        return None;
                    };
                    if let Some(t) = title {
                        c.title = t;
                    }
                    if let Some(cards) = cards {
                        c.cards = cards;
                    }
                    c.limit = limit;
                    Some(Widget::ProductCarousel(c))
                }
            }
        }
        Section::Footer => existing
            .cloned()
            .filter(|w| matches!(w, Widget::Footer(_)))
            .or_else(|| Widget::defaults("Footer")),
    }
}

/// Diseño y color del bloque según lo elegido. Los diseños que la persona
/// ya cambió a mano se respetan, salvo el del pie, que es una opción del paso.
pub fn section_look(section: Section, pick: &WizardPick, existing: Option<&Look>) -> Look {
    let i = section.index();
    let mut look = existing.cloned().unwrap_or_default();
    if section == Section::Footer {
        look.design = [1, 3, 4][usize::from(pick.choice[4])];
    } else if pick.design[i] > 0 {
        look.design = pick.design[i];
    }
    let custom = look.color.starts_with('#');
    if !(custom && pick.color[i] == 0) {
        look.color = WIZARD_COLORS[usize::from(pick.color[i])].0.to_string();
    }
    look
}

/// Alto (en filas de la grilla) que necesita una cuadrícula para mostrar
/// `limit` productos por página: las columnas son las de `fill_cols` en una
/// página ancha, y a cada fila de tarjetas se le suma el título y los
/// números de página.
fn products_rows(limit: i64) -> u8 {
    let n = limit.clamp(1, 48) as usize;
    let cols = match n {
        1 | 2 => n,
        3 | 9 => 3,
        5 | 10 => 5,
        6 | 12 => 6,
        _ => 4,
    };
    let card_rows = n.div_ceil(cols);
    // Una tarjeta mide algo menos de las tres cuartas partes de su ancho más
    // el texto de abajo; 1200 px de ancho útil.
    let card_px = 1200.0 / cols as f64 * 0.75 + 95.0 + 12.0;
    let px = 24.0 + 52.0 + card_rows as f64 * card_px + 56.0;
    (px / f64::from(ROW_PX)).ceil().min(255.0) as u8
}

/// Tamaño del bloque nuevo: el de siempre, con la altura elegida en la portada.
pub fn section_size(section: Section, widget: &Widget, pick: &WizardPick) -> Size {
    let limits = widget.limits();
    let mut size = limits.default;
    size.w = GRID_COLS.clamp(limits.min_w, limits.max_w);
    if let Widget::ProductGrid(g) = widget {
        size.h = products_rows(g.limit).clamp(limits.min_h, limits.max_h);
    }
    if section == Section::Hero {
        let factor = [0.75, 1.0, 1.25][usize::from(pick.hero_height)];
        let h = (f64::from(size.h) * factor).round() as u8;
        size.h = h.clamp(limits.min_h, limits.max_h);
    }
    size
}

/// El bloque de un paso tal como quedaría, con su tamaño, para dibujar la
/// vista previa. `existing` es la página sobre la que se trabaja.
pub fn section_node(section: Section, pick: &WizardPick, doc: &PageDoc) -> Option<Node> {
    let old = existing_of(doc, section);
    let widget = section_widget(section, pick, old.map(|n| &n.widget))?;
    let mut node = Node::new(format!("wz-{}", section.index()), widget);
    node.look = section_look(section, pick, old.map(|n| &n.look));
    if let Some(old) = old {
        node.style = old.style.clone();
    }
    let i = section.index();
    if !pick.font[i].is_empty() {
        node.style.font = pick.font[i].clone();
    }
    if !pick.align[i].is_empty() {
        node.style.align = pick.align[i].clone();
    }
    if !pick.anim[i].is_empty() {
        node.style.animation = pick.anim[i].clone();
    }
    node.size = section_size(section, &node.widget, pick);
    // Un bloque del mismo tipo conserva su alto, salvo la portada, cuya
    // altura es una opción del asistente, y la cuadrícula de productos
    // cuando se cambia cuántos se muestran (si no, con más productos las
    // tarjetas se apretaban y los números de página quedaban tapados).
    if let Some(old) = old {
        let same = std::mem::discriminant(&old.widget) == std::mem::discriminant(&node.widget);
        let limit_changed = matches!(
            (&old.widget, &node.widget),
            (Widget::ProductGrid(a), Widget::ProductGrid(b)) if a.limit != b.limit
        );
        if section != Section::Hero && same && !limit_changed {
            node.size.h = old.size.h;
        }
    }
    Some(node)
}

/// Qué hizo el asistente con un bloque de la página.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Change {
    /// El bloque no existía y se agrega.
    Added,
    /// El bloque existía y se reemplaza por el elegido.
    Replaced,
    /// El bloque existía y se quita («Sin portada»).
    Removed,
    /// Ya estaba como se eligió.
    Kept,
    /// No estaba ni se pidió.
    Absent,
}

/// Aplica lo elegido sobre la página de Inicio: reemplaza cada bloque en su
/// lugar (con sus textos), agrega los que faltan y quita los que se
/// descartaron. Devuelve qué pasó con cada paso. Después hay que compactar
/// la grilla.
pub fn apply(doc: &mut PageDoc, pick: &WizardPick) -> [Change; WIZARD_STEPS] {
    apply_background(&mut doc.style, pick);
    let mut out = [Change::Absent; WIZARD_STEPS];
    for section in Section::ALL {
        let i = section.index();
        let old_idx = doc
            .nodes
            .iter()
            .enumerate()
            .filter(|(_, n)| Section::of(&n.widget) == Some(section))
            .min_by_key(|(_, n)| (n.pos.y, n.pos.x))
            .map(|(k, _)| k);
        let wanted = section_node(section, pick, doc);
        out[i] = match (old_idx, wanted) {
            (Some(k), Some(new)) => {
                let old = doc.nodes[k].clone();
                let mut next = old.clone();
                next.widget = new.widget;
                next.look = new.look;
                next.style = new.style;
                next.size.h = new.size.h;
                if next == old {
                    Change::Kept
                } else {
                    doc.nodes[k] = next;
                    Change::Replaced
                }
            }
            (Some(k), None) => {
                doc.nodes.remove(k);
                Change::Removed
            }
            (None, Some(mut new)) => {
                new.id = doc.fresh_id(&[]);
                new.size.w = GRID_COLS;
                // Va antes del primer bloque que corresponde más abajo; si no
                // hay ninguno, al final de la página.
                let below = doc
                    .nodes
                    .iter()
                    .filter(|n| Section::of(&n.widget).is_some_and(|s| s.index() > i))
                    .map(|n| n.pos.y)
                    .min();
                let end = doc
                    .nodes
                    .iter()
                    .map(|n| n.pos.y + u16::from(n.size.h))
                    .max()
                    .unwrap_or(0);
                let y = below.unwrap_or(end);
                for n in doc.nodes.iter_mut().filter(|n| n.pos.y >= y) {
                    n.pos.y += u16::from(new.size.h);
                }
                new.pos.x = 0;
                new.pos.y = y;
                doc.nodes.push(new);
                Change::Added
            }
            (None, None) => Change::Absent,
        };
    }
    out
}

/// El fondo de la web según lo elegido. «Sin cambios» no toca nada.
pub fn apply_background(style: &mut StoreStyle, pick: &WizardPick) {
    match pick.bg_kind {
        1 => {
            style.bg_color = pick.bg_color.clone();
            style.bg_image.clear();
            style.bg_dim = 0;
            style.bg_fixed = false;
        }
        2 if !pick.bg_image.is_empty() => {
            style.bg_color = pick.bg_color.clone();
            style.bg_image = pick.bg_image.clone();
            style.bg_dim = pick.bg_dim.min(MAX_BG_DIM);
            style.bg_fixed = pick.bg_fixed;
        }
        _ => {}
    }
}

/// ¿El fondo elegido cambia el que tiene la página?
pub fn background_changes(style: &StoreStyle, pick: &WizardPick) -> bool {
    let mut copy = style.clone();
    apply_background(&mut copy, pick);
    copy != *style
}

/// Qué cambiaría aplicar lo elegido sobre esta página, sin tocarla.
pub fn plan(doc: &PageDoc, pick: &WizardPick) -> [Change; WIZARD_STEPS] {
    let mut copy = doc.clone();
    apply(&mut copy, pick)
}

impl Change {
    /// Frase para el aviso previo.
    pub fn text(self) -> Option<&'static str> {
        match self {
            Change::Added => Some("se agrega"),
            Change::Replaced => Some("se reemplaza"),
            Change::Removed => Some("se quita"),
            Change::Kept | Change::Absent => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mas_productos_dan_un_bloque_mas_alto() {
        let doc = starter_doc("Tienda");
        let h = |count: u8| {
            let mut pick = pick_from_doc(&doc);
            pick.products_count = count;
            section_node(Section::Products, &pick, &doc).expect("productos").size.h
        };
        assert!(h(1) > h(0), "8 productos no entran en el alto de 4");
        assert!(products_rows(24) > products_rows(8), "24 productos no entran en el alto de 8");
        // Cambiar la cantidad en una página ya armada también reajusta el alto.
        let mut pick = pick_from_doc(&doc);
        pick.over[Section::Products.index()].insert("limit".into(), 24.into());
        let mut doc2 = doc.clone();
        apply(&mut doc2, &pick);
        let mut again = pick_from_doc(&doc2);
        again.products_count = 0;
        let node = section_node(Section::Products, &again, &doc2).unwrap();
        assert!(node.size.h < products_rows(24));
    }

    #[test]
    fn un_texto_largo_se_recorta_al_limite_del_campo() {
        let mut pick = WizardPick::default();
        pick.over[Section::Hero.index()]
            .insert("title".into(), serde_json::Value::String("x".repeat(300)));
        let w = section_widget(Section::Hero, &pick, None).expect("portada");
        let max = text_max(&w, "title").expect("límite del título");
        let json = serde_json::to_value(&w).unwrap();
        let len = json["props"]["title"].as_str().unwrap().chars().count();
        assert_eq!(len, max);
        // Y la página que sale se puede guardar.
        let mut doc = starter_doc("Tienda");
        apply(&mut doc, &pick);
        assert_eq!(crate::validate(&doc), Ok(()));
    }

    #[test]
    fn la_pagina_inicial_no_tiene_cambios_de_forma() {
        let doc = starter_doc("Tienda");
        assert!(is_untouched(&doc));
        let mut edited = doc.clone();
        edited.nodes.pop();
        assert!(!is_untouched(&edited));
    }

    #[test]
    fn reabrir_arranca_en_lo_que_ya_hay() {
        let doc = starter_doc("Tienda");
        let pick = pick_from_doc(&doc);
        // La plantilla inicial tiene banner, cuadrícula y no tiene filtros.
        assert_eq!(pick.choice[1], 0);
        assert_eq!(pick.choice[2], 2);
        assert_eq!(pick.choice[3], 0);
    }

    #[test]
    fn aplicar_lo_mismo_no_cambia_nada() {
        let mut doc = starter_doc("Tienda");
        let pick = pick_from_doc(&doc);
        let before = doc.clone();
        let changes = apply(&mut doc, &pick);
        assert!(changes
            .iter()
            .all(|c| matches!(c, Change::Kept | Change::Absent)));
        assert_eq!(doc, before);
    }

    #[test]
    fn reemplaza_en_su_lugar_y_conserva_los_textos() {
        let mut doc = starter_doc("Mi tienda");
        let title_before = doc
            .nodes
            .iter()
            .find_map(|n| match &n.widget {
                Widget::Banner(b) => Some(b.title.clone()),
                _ => None,
            })
            .unwrap();
        let mut pick = pick_from_doc(&doc);
        pick.choice[1] = 1; // carrusel
        pick.choice[0] = 1; // logo al centro
        let changes = apply(&mut doc, &pick);
        assert_eq!(changes[0], Change::Replaced);
        assert_eq!(changes[1], Change::Replaced);
        let carousel = doc
            .nodes
            .iter()
            .find_map(|n| match &n.widget {
                Widget::Carousel(c) => Some(c.clone()),
                _ => None,
            })
            .unwrap();
        assert_eq!(carousel.slides[0].title, title_before);
        let nav = doc
            .nodes
            .iter()
            .find_map(|n| match &n.widget {
                Widget::Navbar(n) => Some(n.variant.clone()),
                _ => None,
            })
            .unwrap();
        assert_eq!(nav, "c");
    }

    #[test]
    fn agrega_filtros_entre_la_portada_y_los_productos() {
        let mut doc = starter_doc("Tienda");
        let mut pick = pick_from_doc(&doc);
        pick.choice[2] = 0; // pestañas
        let changes = apply(&mut doc, &pick);
        assert_eq!(changes[2], Change::Added);
        let mut nodes = doc.nodes.clone();
        nodes.sort_by_key(|n| n.pos.y);
        let order: Vec<_> = nodes
            .iter()
            .filter_map(|n| Section::of(&n.widget))
            .collect();
        let mut sorted = order.clone();
        sorted.sort_by_key(|s| s.index());
        assert_eq!(order, sorted);
        // Ningún bloque queda encima de otro.
        for (a, b) in nodes.iter().zip(nodes.iter().skip(1)) {
            assert!(b.pos.y >= a.pos.y + u16::from(a.size.h), "{a:?} / {b:?}");
        }
    }

    #[test]
    fn sin_portada_quita_el_bloque() {
        let mut doc = starter_doc("Tienda");
        let mut pick = pick_from_doc(&doc);
        pick.choice[1] = 2;
        let changes = apply(&mut doc, &pick);
        assert_eq!(changes[1], Change::Removed);
        assert!(doc
            .nodes
            .iter()
            .all(|n| !matches!(n.widget, Widget::Banner(_) | Widget::Carousel(_))));
    }

    #[test]
    fn el_plan_no_toca_la_pagina() {
        let doc = starter_doc("Tienda");
        let mut pick = pick_from_doc(&doc);
        pick.choice[3] = 2;
        let before = doc.clone();
        let plan = plan(&doc, &pick);
        assert_eq!(plan[3], Change::Replaced);
        assert_eq!(doc, before);
    }

    #[test]
    fn un_borrador_viejo_no_se_sale_de_las_opciones() {
        let mut pick = WizardPick::default();
        pick.choice = [9; WIZARD_STEPS];
        pick.color = [9; WIZARD_STEPS];
        pick.nav_tone = 9;
        let pick = pick.clamped();
        for (i, step) in WIZARD.iter().enumerate() {
            assert!(usize::from(pick.choice[i]) < step.options.len());
        }
        // Y arma bloques sin fallar con cualquier combinación.
        let doc = starter_doc("Tienda");
        for section in Section::ALL {
            let _ = section_node(section, &pick, &doc);
        }
    }

    #[test]
    fn todas_las_combinaciones_arman_bloques() {
        let doc = starter_doc("Tienda");
        for a in 0..3u8 {
            for b in 0..3u8 {
                for c in 0..3u8 {
                    for d in 0..3u8 {
                        let mut pick = WizardPick::default();
                        pick.choice = [a, b, c, d, a];
                        let mut copy = doc.clone();
                        apply(&mut copy, &pick);
                        for n in &copy.nodes {
                            assert!(n.size.w >= 1 && n.size.h >= 1);
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn ajustes_y_fondo_se_aplican_sin_romper_nada() {
        let mut doc = starter_doc("Tienda");
        let mut pick = pick_from_doc(&doc);
        pick.over[0].insert("brand".into(), "Mi marca".into());
        pick.over[0].insert("no_existe".into(), "x".into());
        pick.over[3].insert("limit".into(), 16.into());
        pick.over[3].insert("card_favorite".into(), false.into());
        pick.design[1] = 3;
        pick.align[1] = "center".into();
        pick.bg_kind = 2;
        pick.bg_image = "/img/banner.webp".into();
        pick.bg_dim = 99;
        let pick = pick.clamped();
        assert_eq!(pick.bg_dim, MAX_BG_DIM);
        apply(&mut doc, &pick);
        assert_eq!(doc.style.bg_image, "/img/banner.webp");
        let nav = doc.nodes.iter().find_map(|n| match &n.widget {
            Widget::Navbar(n) => Some(n.brand.clone()),
            _ => None,
        });
        assert_eq!(nav.as_deref(), Some("Mi marca"));
        let grid = doc.nodes.iter().find_map(|n| match &n.widget {
            Widget::ProductGrid(g) => Some(g.clone()),
            _ => None,
        });
        let grid = grid.unwrap();
        assert_eq!(grid.limit, 16);
        assert!(!grid.cards.card_favorite);
        let hero = doc
            .nodes
            .iter()
            .find(|n| matches!(n.widget, Widget::Banner(_)))
            .unwrap();
        assert_eq!(hero.look.design, 3);
        assert_eq!(hero.style.align, "center");
    }

    #[test]
    fn un_valor_malo_o_una_direccion_peligrosa_se_descartan() {
        let mut pick = WizardPick::default();
        pick.bg_image = "https://x.com/a.jpg);color:red".into();
        pick.bg_color = "rojo".into();
        pick.font[0] = "inventada".into();
        let pick = pick.clamped();
        assert!(pick.bg_image.is_empty() && pick.bg_color.is_empty() && pick.font[0].is_empty());
    }
}
