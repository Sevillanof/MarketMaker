//! Animación del texto de un bloque (Letras › Animación en el editor).
//!
//! `render_widget` pone la animación del bloque en el contexto
//! ([`TextAnim`]) y la clase `mk-anim-<id>` en su envoltorio. Cada título
//! se dibuja con [`AnimText`], que arma lo que pide la animación (una
//! letra por `span`, una palabra por `span`, o el texto tal cual) y
//! `style/tailwind.css` pone el movimiento.
//!
//! Regla: el texto que se ve nunca se repite como texto real dentro del
//! elemento (las copias van en atributos `data-*` que se dibujan con
//! `::before`/`::after`), así `textContent` sigue siendo el título y la
//! edición con doble clic del lienzo lo encuentra igual.

use std::time::Duration;

use leptos::prelude::*;

/// Animación del bloque que se está dibujando.
#[derive(Clone, Debug, PartialEq)]
pub struct TextAnim {
    pub id: String,
    /// Dato extra: palabras que rotan, texto tachado o palabra oculta.
    pub extra: String,
}

/// ¿La persona pidió menos movimiento? Entonces el texto queda quieto.
fn reduced_motion() -> bool {
    web_sys::window()
        .and_then(|w| {
            w.match_media("(prefers-reduced-motion: reduce)")
                .ok()
                .flatten()
        })
        .is_some_and(|m| m.matches())
}

fn rnd() -> f64 {
    #[cfg(target_arch = "wasm32")]
    {
        js_sys::Math::random()
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        0.5
    }
}

/// El texto de un título, con la animación del bloque si la tiene.
#[component]
pub fn AnimText(#[prop(into)] text: String) -> impl IntoView {
    let Some(anim) = use_context::<TextAnim>() else {
        return text.into_any();
    };
    match anim.id.as_str() {
        "typewriter" | "bounce" | "wave" | "flip" => letters_view(&text, anim.id == "typewriter"),
        "mask" => words_view(&text, true),
        "stagger" => words_view(&text, false),
        "glitch" => {
            let t = text.clone();
            view! { <span class="mk-a" data-t=t>{text}</span> }.into_any()
        }
        "split" => {
            let t = text.clone();
            let hidden = anim.extra.clone();
            view! { <span class="mk-a" data-t=t><i class="mk-a-h" data-h=hidden></i>{text}</span> }
                .into_any()
        }
        "strike" => {
            let old = anim.extra.clone();
            view! { <span class="mk-a"><span class="mk-a-old" data-o=old></span><span class="mk-a-new">{text}</span></span> }
                .into_any()
        }
        "rotating" => rotating_view(text, &anim.extra),
        "scramble" => scramble_view(text),
        "count" => count_view(text),
        _ => view! { <span class="mk-a">{text}</span> }.into_any(),
    }
}

/// Cada palabra en un `span` que no se parte, y cada letra en el suyo.
fn letters_view(text: &str, typing: bool) -> AnyView {
    let total = text.chars().filter(|c| *c != ' ').count();
    let mut i = 0usize;
    let mut out: Vec<AnyView> = Vec::new();
    for (wi, word) in text.split(' ').enumerate() {
        if wi > 0 {
            out.push(" ".into_any());
        }
        if word.is_empty() {
            continue;
        }
        let spans = word
            .chars()
            .map(|c| {
                i += 1;
                let last = typing && i == total;
                view! {
                    <span class=if last { "mk-a-l mk-a-last" } else { "mk-a-l" } style=format!("--i:{}", i - 1)>{c.to_string()}</span>
                }
            })
            .collect_view();
        out.push(view! { <span class="mk-a-w">{spans}</span> }.into_any());
    }
    let label = text.to_string();
    view! { <span class="mk-a" role="img" aria-label=label>{out}</span> }.into_any()
}

/// Cada palabra en un `span`; con `mask`, adentro de una máscara.
fn words_view(text: &str, mask: bool) -> AnyView {
    let mut out: Vec<AnyView> = Vec::new();
    let mut n = 0usize;
    for (wi, word) in text.split(' ').enumerate() {
        if wi > 0 {
            out.push(" ".into_any());
        }
        if word.is_empty() {
            continue;
        }
        let w = word.to_string();
        let style = format!("--i:{n}");
        n += 1;
        out.push(if mask {
            view! { <span class="mk-a-w"><span style=style>{w}</span></span> }.into_any()
        } else {
            view! { <span class="mk-a-w" style=style>{w}</span> }.into_any()
        });
    }
    let label = text.to_string();
    view! { <span class="mk-a" role="img" aria-label=label>{out}</span> }.into_any()
}

/// Una palabra que gira por otra cada pocos segundos.
fn rotating_view(text: String, extra: &str) -> AnyView {
    let words: Vec<String> = extra
        .split(',')
        .map(|w| w.trim().to_string())
        .filter(|w| !w.is_empty())
        .collect();
    if words.is_empty() {
        return view! { <span class="mk-a">{text}</span> }.into_any();
    }
    let idx = RwSignal::new(0usize);
    if words.len() > 1 {
        Effect::new(move |_| {
            if reduced_motion() {
                return;
            }
            if let Ok(handle) = set_interval_with_handle(
                move || idx.update(|i| *i += 1),
                Duration::from_millis(2200),
            ) {
                on_cleanup(move || handle.clear());
            }
        });
    }
    view! {
        <span class="mk-a">
            {text}
            {move || {
                let w = words[idx.get() % words.len()].clone();
                view! { <span class="mk-a-rw" data-w=w></span> }
            }}
        </span>
    }
    .into_any()
}

/// Letras al azar que se van ordenando de izquierda a derecha.
fn scramble_view(text: String) -> AnyView {
    const POOL: &[char] = &[
        '!', '<', '>', '-', '_', '/', '[', ']', '{', '}', '=', '+', '*', '^', '?', '#', 'A', 'B',
        'C', 'D', 'E', 'K', 'M', 'R', 'S', 'X', 'Z', '0', '1', '3', '7', '9',
    ];
    let shown = RwSignal::new(text.clone());
    let target = text.clone();
    Effect::new(move |_| {
        if reduced_motion() {
            return;
        }
        let chars: Vec<char> = target.chars().collect();
        let locks: Vec<f64> = (0..chars.len())
            .map(|i| 10.0 + i as f64 * 2.5 + rnd() * 12.0)
            .collect();
        let last = locks.iter().cloned().fold(0.0, f64::max) + 2.0;
        let frame = StoredValue::new(0.0f64);
        let handle = StoredValue::new(None::<IntervalHandle>);
        let target = target.clone();
        if let Ok(h) = set_interval_with_handle(
            move || {
                frame.update_value(|f| *f += 1.0);
                let f = frame.get_value();
                let s: String = chars
                    .iter()
                    .enumerate()
                    .map(|(i, c)| {
                        if *c == ' ' || f > locks[i] {
                            *c
                        } else {
                            POOL[(rnd() * POOL.len() as f64) as usize % POOL.len()]
                        }
                    })
                    .collect();
                if f > last {
                    shown.set(target.clone());
                    if let Some(h) = handle.get_value() {
                        h.clear();
                    }
                } else {
                    shown.set(s);
                }
            },
            Duration::from_millis(32),
        ) {
            handle.set_value(Some(h));
            on_cleanup(move || {
                if let Some(h) = handle.get_value() {
                    h.clear();
                }
            });
        }
    });
    view! { <span class="mk-a">{move || shown.get()}</span> }.into_any()
}

/// El primer número del texto: (antes, valor, con puntos de miles, después).
fn split_number(text: &str) -> Option<(String, u64, bool, String)> {
    let start = text.find(|c: char| c.is_ascii_digit())?;
    let rest = &text[start..];
    let end = rest
        .find(|c: char| !(c.is_ascii_digit() || c == '.'))
        .unwrap_or(rest.len());
    let run = rest[..end].trim_end_matches('.');
    let groups: Vec<&str> = run.split('.').collect();
    // «4.5» sería un decimal, no miles: queda quieto.
    if groups.iter().skip(1).any(|g| g.len() != 3) || groups[0].len() > 3 && groups.len() > 1 {
        return None;
    }
    let value: u64 = run.replace('.', "").parse().ok()?;
    Some((
        text[..start].to_string(),
        value,
        groups.len() > 1,
        text[start + run.len()..].to_string(),
    ))
}

fn thousands(n: u64) -> String {
    let s = n.to_string();
    let mut out = String::new();
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i) % 3 == 0 {
            out.push('.');
        }
        out.push(c);
    }
    out
}

/// Cuenta desde 0 hasta el número que tenga el texto.
fn count_view(text: String) -> AnyView {
    let Some((pre, value, dotted, post)) = split_number(&text) else {
        return view! { <span class="mk-a">{text}</span> }.into_any();
    };
    let shown = RwSignal::new(text.clone());
    Effect::new(move |_| {
        if reduced_motion() {
            return;
        }
        let (pre, post) = (pre.clone(), post.clone());
        let frame = StoredValue::new(0u32);
        let handle = StoredValue::new(None::<IntervalHandle>);
        let render = move |n: u64| {
            let num = if dotted { thousands(n) } else { n.to_string() };
            format!("{pre}{num}{post}")
        };
        shown.set(render(0));
        if let Ok(h) = set_interval_with_handle(
            move || {
                frame.update_value(|f| *f += 1);
                let p = (frame.get_value() as f64 / 48.0).min(1.0);
                let eased = 1.0 - (1.0 - p).powi(3);
                shown.set(render((value as f64 * eased).round() as u64));
                if p >= 1.0 {
                    if let Some(h) = handle.get_value() {
                        h.clear();
                    }
                }
            },
            Duration::from_millis(30),
        ) {
            handle.set_value(Some(h));
            on_cleanup(move || {
                if let Some(h) = handle.get_value() {
                    h.clear();
                }
            });
        }
    });
    view! { <span class="mk-a mk-a-num">{move || shown.get()}</span> }.into_any()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_the_number() {
        assert_eq!(
            split_number("+12.480 pedidos"),
            Some(("+".into(), 12480, true, " pedidos".into()))
        );
        assert_eq!(
            split_number("98% clientes"),
            Some(("".into(), 98, false, "% clientes".into()))
        );
        assert_eq!(split_number("Sin número"), None);
        assert_eq!(split_number("4.5 estrellas"), None);
        assert_eq!(thousands(1234567), "1.234.567");
        assert_eq!(thousands(980), "980");
    }
}
