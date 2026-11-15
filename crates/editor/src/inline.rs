//! Edición directa en el lienzo: doble clic sobre un título, subtítulo o
//! descripción de un bloque y se escribe ahí mismo, sin ir al panel.
//!
//! Los widgets no marcan sus textos: el editor busca, desde el elemento que
//! está bajo el puntero hacia afuera, el primero cuyo texto es el de un campo
//! de texto del bloque (de primer nivel o de un elemento de una lista). Así
//! funciona con todos los bloques, también con los que se sumen después.

use domain::{Field, FieldKind, PageDoc, Widget};
use leptos::prelude::*;
use wasm_bindgen::JsCast;

use crate::fields::{field_value, set_field};
use crate::{EditorState, Ui};

/// Dónde vive un texto dentro de las props de un bloque.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum TextPath {
    /// Un campo de primer nivel, como `title`.
    Top(&'static str),
    /// El campo `key` del elemento `index` de la lista `list`.
    Item {
        list: &'static str,
        index: usize,
        key: &'static str,
    },
}

/// Un texto del bloque que se puede cambiar en el lienzo.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct TextSpot {
    pub path: TextPath,
    pub label: &'static str,
    pub value: String,
    pub max: usize,
    /// Texto largo: admite saltos de línea.
    pub long: bool,
}

/// Espacios seguidos (y saltos de línea) como uno solo, sin los de los bordes.
pub(crate) fn normalize(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn text_kind(field: &Field) -> Option<(usize, bool)> {
    match field.kind {
        FieldKind::Text { max } => Some((max, false)),
        FieldKind::LongText { max } => Some((max, true)),
        _ => None,
    }
}

/// Los textos que muestra el bloque y se pueden cambiar en el lienzo.
pub(crate) fn text_spots(widget: &Widget) -> Vec<TextSpot> {
    let props = serde_json::to_value(widget)
        .ok()
        .and_then(|v| v.get("props").cloned())
        .unwrap_or_default();
    let mut spots = Vec::new();
    for field in widget.fields() {
        if let FieldKind::List { item, .. } = &field.kind {
            let items = props
                .get(field.key)
                .and_then(|v| v.as_array())
                .cloned()
                .unwrap_or_default();
            for (index, it) in items.iter().enumerate() {
                for f in item.iter() {
                    let Some((max, long)) = text_kind(f) else {
                        continue;
                    };
                    let value = it.get(f.key).and_then(|v| v.as_str()).unwrap_or_default();
                    spots.push(TextSpot {
                        path: TextPath::Item {
                            list: field.key,
                            index,
                            key: f.key,
                        },
                        label: f.label,
                        value: value.to_string(),
                        max,
                        long,
                    });
                }
            }
        } else if let Some((max, long)) = text_kind(field) {
            let value = props
                .get(field.key)
                .and_then(|v| v.as_str())
                .unwrap_or_default();
            spots.push(TextSpot {
                path: TextPath::Top(field.key),
                label: field.label,
                value: value.to_string(),
                max,
                long,
            });
        }
    }
    spots
}

/// Comillas que algunos bloques ponen alrededor del texto (las opiniones).
const QUOTES: &[char] = &['“', '”', '"', '«', '»'];

/// Para comparar lo que se ve con lo guardado.
fn comparable(s: &str) -> String {
    normalize(normalize(s).trim_matches(QUOTES))
}

/// El texto del bloque que se ve como `shown`, si hay uno.
pub(crate) fn find_spot(spots: &[TextSpot], shown: &str) -> Option<TextSpot> {
    let shown = comparable(shown);
    if shown.is_empty() {
        return None;
    }
    spots
        .iter()
        .find(|s| comparable(&s.value) == shown)
        .cloned()
}

/// Cambia el texto `path` del bloque `id`.
pub(crate) fn set_text(doc: &mut PageDoc, id: &str, path: &TextPath, value: &str) {
    let value = serde_json::Value::String(value.to_string());
    match path {
        TextPath::Top(key) => set_field(doc, id, key, value),
        TextPath::Item { list, index, key } => {
            let mut items = field_value(doc, id, list);
            let Some(item) = items
                .as_array_mut()
                .and_then(|a| a.get_mut(*index))
                .and_then(|i| i.as_object_mut())
            else {
                return;
            };
            item.insert((*key).to_string(), value);
            set_field(doc, id, list, items);
        }
    }
}

/// Lo que se está escribiendo en el lienzo.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct InlineEdit {
    pub node_id: String,
    pub spot: TextSpot,
    /// Posición y letras del cuadro, copiadas del texto del lienzo.
    pub style: String,
}

/// Clase que esconde el texto original mientras se escribe encima.
const HIDDEN: &str = "mk-inline-hidden";

/// Texto del bloque bajo el puntero, listo para editarlo en el lienzo.
/// `stage` es el contenedor donde se dibuja el cuadro.
pub(crate) fn inline_at(
    doc: &PageDoc,
    stage: &web_sys::Element,
    client_x: i32,
    client_y: i32,
) -> Option<InlineEdit> {
    let hit = document().element_from_point(client_x as f32, client_y as f32)?;
    let content = hit.closest(".mk-content").ok()??;
    let node_el = content.closest("[data-node-id]").ok()??;
    let node_id = node_el.get_attribute("data-node-id")?;
    // Los botones llevan a otra página con un clic; se cambian en el panel.
    if hit.closest("[data-href]").ok().flatten().is_some() {
        return None;
    }
    let spots = text_spots(&doc.node(&node_id)?.widget);
    let mut el = Some(hit);
    while let Some(cur) = el {
        if !content.contains(Some(&cur)) {
            break;
        }
        if let Some(spot) = find_spot(&spots, &cur.text_content().unwrap_or_default()) {
            let html = cur.dyn_into::<web_sys::HtmlElement>().ok()?;
            let style = overlay_style(&html, stage)?;
            _ = html.class_list().add_1(HIDDEN);
            return Some(InlineEdit {
                node_id,
                spot,
                style,
            });
        }
        el = cur.parent_element();
    }
    None
}

/// Muestra de nuevo el texto escondido.
pub(crate) fn unhide() {
    if let Ok(list) = document().query_selector_all(&format!(".{HIDDEN}")) {
        for i in 0..list.length() {
            if let Some(el) = list
                .get(i)
                .and_then(|n| n.dyn_into::<web_sys::Element>().ok())
            {
                _ = el.class_list().remove_1(HIDDEN);
            }
        }
    }
}

/// Lugar y letras del cuadro de escritura: encima del texto, del mismo
/// tamaño en pantalla (el lienzo está achicado) y con las mismas letras.
fn overlay_style(el: &web_sys::HtmlElement, stage: &web_sys::Element) -> Option<String> {
    let rect = el.get_bounding_client_rect();
    let stage_rect = stage.get_bounding_client_rect();
    let k = if el.offset_width() > 0 {
        rect.width() / f64::from(el.offset_width())
    } else {
        1.0
    };
    let cs = window().get_computed_style(el).ok()??;
    let get = |p: &str| cs.get_property_value(p).unwrap_or_default();
    let px = |p: &str| -> String {
        let v = get(p);
        match v.strip_suffix("px").and_then(|n| n.parse::<f64>().ok()) {
            Some(n) => format!("{:.2}px", n * k),
            None => v,
        }
    };
    let width = rect.width().max(160.0);
    // Si se ensanchó, se corre para que el texto quede donde estaba.
    let left = rect.left()
        - stage_rect.left()
        - match get("text-align").as_str() {
            "center" => (width - rect.width()) / 2.0,
            "right" | "end" => width - rect.width(),
            _ => 0.0,
        };
    Some(format!(
        "left:{left:.1}px; top:{:.1}px; width:{width:.1}px; min-height:{:.1}px; \
         font-family:{}; font-size:{}; font-weight:{}; font-style:{}; line-height:{}; \
         letter-spacing:{}; text-align:{}; text-transform:{}; color:{};",
        rect.top() - stage_rect.top(),
        rect.height(),
        get("font-family"),
        px("font-size"),
        get("font-weight"),
        get("font-style"),
        px("line-height"),
        px("letter-spacing"),
        get("text-align"),
        get("text-transform"),
        get("color"),
    ))
}

/// Ajusta el alto del cuadro a lo escrito.
fn grow(el: &web_sys::HtmlTextAreaElement) {
    let style = web_sys::HtmlElement::style(el);
    _ = style.set_property("height", "auto");
    _ = style.set_property("height", &format!("{}px", el.scroll_height()));
}

/// El cuadro donde se escribe, encima del texto del lienzo.
#[component]
pub(crate) fn InlineEditor(edit: RwSignal<Option<InlineEdit>>) -> impl IntoView {
    let state = expect_context::<EditorState>();
    let ui = expect_context::<Ui>();
    let area = NodeRef::<leptos::html::Textarea>::new();
    let Some(start) = edit.get_untracked() else {
        return ().into_any();
    };
    let long = start.spot.long;

    // Termina: con `keep` guarda lo escrito (un solo paso de deshacer).
    let finish = move |keep: bool| {
        let Some(e) = edit.get_untracked() else {
            return;
        };
        edit.set(None);
        unhide();
        if !keep {
            return;
        }
        let Some(el) = area.get_untracked() else {
            return;
        };
        let mut text = el.value();
        if !e.spot.long {
            text = normalize(&text);
        }
        if text != e.spot.value {
            state.apply(|d| set_text(d, &e.node_id, &e.spot.path, &text));
            ui.announce(format!("{} cambiado.", e.spot.label));
        }
    };

    // Tocar otra cosa (un bloque, el panel…) también termina: el lienzo
    // evita que el foco cambie al tocar un bloque, así que no llega el blur.
    let outside = window_event_listener(leptos::ev::pointerdown, move |ev| {
        let inside = ev
            .target()
            .and_then(|t| t.dyn_into::<web_sys::Node>().ok())
            .zip(area.get_untracked())
            .is_some_and(|(t, a)| a.contains(Some(&t)));
        if !inside {
            finish(true);
        }
    });
    on_cleanup(move || outside.remove());

    Effect::new(move |_| {
        if let Some(el) = area.get() {
            grow(&el);
            // Después del cuadro que lleva el foco al bloque al soltar el
            // clic (ver `reveal_node`), que si no se lo saca enseguida.
            request_animation_frame(move || {
                _ = el.focus();
                el.select();
            });
        }
    });

    view! {
        <textarea
            node_ref=area
            class="mk-inline-edit"
            style=start.style.clone()
            rows="1"
            maxlength=start.spot.max
            aria-label=format!("{} del bloque", start.spot.label)
            prop:value=start.spot.value.clone()
            on:input=move |_| {
                if let Some(el) = area.get_untracked() {
                    grow(&el);
                }
            }
            on:pointerdown=|ev| ev.stop_propagation()
            on:dblclick=|ev| ev.stop_propagation()
            on:keydown=move |ev: web_sys::KeyboardEvent| {
                // Lo que se escribe acá no son atajos del editor (Ctrl+Z
                // deshace dentro del cuadro, Suprimir borra letras).
                ev.stop_propagation();
                match ev.key().as_str() {
                    "Escape" => {
                        ev.prevent_default();
                        finish(false);
                    }
                    "Enter" if !long || ev.ctrl_key() || ev.meta_key() => {
                        ev.prevent_default();
                        finish(true);
                    }
                    _ => {}
                }
            }
            on:blur=move |_| finish(true)
        ></textarea>
    }
    .into_any()
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::Node;

    fn doc_with(widget: Widget) -> PageDoc {
        let mut doc = PageDoc::default();
        doc.nodes.push(Node::new("n1", widget));
        doc
    }

    #[test]
    fn spots_cover_top_level_and_list_texts() {
        let w = Widget::defaults("Benefits").unwrap();
        let spots = text_spots(&w);
        assert!(spots.iter().any(|s| matches!(
            s.path,
            TextPath::Item {
                list: "items",
                index: 0,
                key: "title"
            }
        )));
        let w = Widget::defaults("Banner").unwrap();
        let spots = text_spots(&w);
        assert!(spots.iter().any(|s| s.path == TextPath::Top("title")));
        // Las direcciones no son textos que se vean.
        assert!(!spots.iter().any(|s| s.path == TextPath::Top("button_href")));
    }

    #[test]
    fn find_spot_ignores_spacing_and_empty_text() {
        let spots = vec![TextSpot {
            path: TextPath::Top("body"),
            label: "Texto",
            value: "Hola\n  mundo".into(),
            max: 100,
            long: true,
        }];
        assert_eq!(
            find_spot(&spots, " Hola mundo ").unwrap().path,
            TextPath::Top("body")
        );
        assert!(find_spot(&spots, "   ").is_none());
        // Las opiniones se muestran entre comillas.
        assert!(find_spot(&spots, "“Hola mundo”").is_some());
        assert!(find_spot(&spots, "Chau").is_none());
    }

    #[test]
    fn set_text_changes_one_text() {
        let mut doc = doc_with(Widget::defaults("Benefits").unwrap());
        set_text(
            &mut doc,
            "n1",
            &TextPath::Item {
                list: "items",
                index: 1,
                key: "title",
            },
            "Pagá en cuotas",
        );
        let spots = text_spots(&doc.node("n1").unwrap().widget);
        let get = |i: usize| {
            spots
                .iter()
                .find(|s| {
                    s.path
                        == TextPath::Item {
                            list: "items",
                            index: i,
                            key: "title",
                        }
                })
                .unwrap()
                .value
                .clone()
        };
        assert_eq!(get(1), "Pagá en cuotas");
        assert_eq!(get(0), "Envío gratis");

        let mut doc = doc_with(Widget::defaults("Banner").unwrap());
        set_text(&mut doc, "n1", &TextPath::Top("title"), "Nuevo título");
        let spots = text_spots(&doc.node("n1").unwrap().widget);
        assert_eq!(spots[0].value, "Nuevo título");
    }
}
