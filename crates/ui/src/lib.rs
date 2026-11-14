//! Primitivas de presentación sobre clases de daisyUI.
//!
//! Las clases se escriben completas y literales para que Tailwind las encuentre.

use leptos::prelude::*;

/// Botón primario. Con `href` se dibuja como enlace; con `disabled` nunca navega.
#[component]
pub fn Button(
    #[prop(into)] label: String,
    #[prop(optional, into)] href: Option<String>,
    #[prop(optional)] disabled: bool,
) -> impl IntoView {
    match href {
        Some(href) if !disabled && !href.is_empty() => view! {
            <a class="btn btn-primary" href=href>{label}</a>
        }
        .into_any(),
        _ => view! {
            <button type="button" class="btn btn-primary" disabled=disabled>{label}</button>
        }
        .into_any(),
    }
}

/// Tarjeta que ocupa todo el alto disponible y recorta lo que sobra.
#[component]
pub fn Card(children: Children) -> impl IntoView {
    view! {
        <div class="card card-sm bg-base-100 shadow-sm border border-base-300 h-full min-h-0 overflow-hidden">
            {children()}
        </div>
    }
}

/// Imagen que cubre su contenedor sin deformarse. No se puede arrastrar,
/// para no interferir con el arrastre del editor.
#[component]
pub fn Image(#[prop(into)] src: String, #[prop(into)] alt: String) -> impl IntoView {
    view! {
        <img class="block w-full h-full object-cover" src=src alt=alt draggable="false" loading="lazy"/>
    }
}

/// Precio con formato argentino a partir de centavos: `$ 12.345,00`.
#[component]
pub fn Price(cents: i64) -> impl IntoView {
    view! { <span class="font-semibold">{format_price(cents)}</span> }
}

/// Formatea centavos como `$ 12.345,00`.
pub fn format_price(cents: i64) -> String {
    let sign = if cents < 0 { "-" } else { "" };
    let cents = cents.unsigned_abs();
    let units = (cents / 100).to_string();
    let mut grouped = String::new();
    for (i, ch) in units.chars().enumerate() {
        if i > 0 && (units.len() - i) % 3 == 0 {
            grouped.push('.');
        }
        grouped.push(ch);
    }
    format!("{sign}$ {grouped},{:02}", cents % 100)
}

#[cfg(test)]
mod tests {
    use super::format_price;

    #[test]
    fn formats_argentine_prices() {
        assert_eq!(format_price(1_234_500), "$ 12.345,00");
        assert_eq!(format_price(5), "$ 0,05");
        assert_eq!(format_price(99_900), "$ 999,00");
        assert_eq!(format_price(123_456_789), "$ 1.234.567,89");
        assert_eq!(format_price(-100), "-$ 1,00");
    }
}
