//! Emails (solo servidor): se mandan con Resend si hay `RESEND_API_KEY` en
//! `.env.local`; si no, se escriben en la terminal.
//!
//! - `RESEND_API_KEY`: la clave de resend.com.
//! - `EMAIL_FROM`: el remitente, por ejemplo `MarketMaker <ventas@tudominio.com>`.
//!   Sin dominio verificado en Resend, `onboarding@resend.dev` solo llega al
//!   email de tu propia cuenta de Resend (sirve para probar).
//! - `APP_URL`: la dirección de la plataforma para los enlaces de los emails
//!   (por defecto `http://localhost:3000`).

use domain::{Delivery, Order};
use std::sync::OnceLock;

/// Un email para mandar.
pub struct Email {
    pub to: Vec<String>,
    pub subject: String,
    pub html: String,
    pub text: String,
}

fn env(name: &str) -> Option<String> {
    std::env::var(name).ok().map(|v| v.trim().to_string()).filter(|v| !v.is_empty())
}

/// La plataforma (`http://localhost:3000`), para los enlaces.
pub fn app_url() -> String {
    env("APP_URL")
        .unwrap_or_else(|| "http://localhost:3000".into())
        .trim_end_matches('/')
        .to_string()
}

/// La dirección de una tienda (`http://demo.localhost:3000`).
pub fn store_url(slug: &str) -> String {
    let app = app_url();
    let (scheme, host) = app.split_once("://").unwrap_or(("http", &app));
    format!("{scheme}://{}", domain::store_host(host, slug))
}

fn http() -> &'static reqwest::Client {
    static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
    CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(20))
            .build()
            .expect("cliente HTTP")
    })
}

/// Lo manda de fondo: quien compra no espera al servicio de emails, y si
/// falla queda escrito en la terminal.
pub fn send(email: Email) {
    tokio::spawn(async move {
        if let Err(e) = deliver(&email).await {
            leptos::logging::log!("Email \"{}\" a {}: no se pudo mandar: {e}", email.subject, email.to.join(", "));
        }
    });
}

/// Lo manda y espera (para los códigos de entrar: hay que saber si salió).
pub async fn deliver(email: &Email) -> Result<(), String> {
    if email.to.is_empty() {
        return Ok(());
    }
    let Some(key) = env("RESEND_API_KEY") else {
        leptos::logging::log!(
            "\n  ✉ Email (sin RESEND_API_KEY, no se manda)\n  Para: {}\n  Asunto: {}\n\n{}\n",
            email.to.join(", "),
            email.subject,
            email.text
        );
        return Ok(());
    };
    let from = env("EMAIL_FROM").unwrap_or_else(|| "MarketMaker <onboarding@resend.dev>".into());
    let body = serde_json::json!({
        "from": from,
        "to": email.to,
        "subject": email.subject,
        "html": email.html,
        "text": email.text,
    });
    let url = env("RESEND_API_URL").unwrap_or_else(|| "https://api.resend.com".into());
    let res = http()
        .post(format!("{url}/emails"))
        .bearer_auth(key)
        .header("content-type", "application/json")
        .body(body.to_string())
        .send()
        .await
        .map_err(|e| format!("sin conexión: {e}"))?;
    if res.status().is_success() {
        Ok(())
    } else {
        let status = res.status();
        let text = res.text().await.unwrap_or_default();
        Err(format!("{status} {}", text.chars().take(300).collect::<String>()))
    }
}

/// Escapa texto para meterlo en HTML.
fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// Marco común: una tarjeta blanca con el nombre de quien escribe arriba.
fn frame(from: &str, body: &str) -> String {
    format!(
        r#"<!doctype html><html lang="es"><body style="margin:0;background:#f4f4f5;font-family:-apple-system,Segoe UI,Roboto,Helvetica,Arial,sans-serif;color:#18181b">
<div style="max-width:560px;margin:0 auto;padding:24px 16px">
<p style="font-weight:700;font-size:18px;margin:0 0 12px">{}</p>
<div style="background:#fff;border-radius:12px;padding:24px">{body}</div>
<p style="color:#71717a;font-size:12px;margin:16px 0 0">Hecho con MarketMaker</p>
</div></body></html>"#,
        esc(from)
    )
}

fn button(href: &str, label: &str) -> String {
    format!(
        r#"<p style="margin:20px 0 0"><a href="{}" style="display:inline-block;background:#4f46e5;color:#fff;text-decoration:none;font-weight:600;padding:12px 20px;border-radius:8px">{}</a></p>"#,
        esc(href),
        esc(label)
    )
}

/// Los productos y los totales, en HTML y en texto.
fn order_lines(o: &Order) -> (String, String) {
    let mut html = String::from(r#"<table style="width:100%;border-collapse:collapse;font-size:14px;margin-top:12px">"#);
    let mut text = String::new();
    for i in &o.items {
        html.push_str(&format!(
            r#"<tr><td style="padding:6px 0;border-bottom:1px solid #f4f4f5">{} × {}</td><td style="padding:6px 0;border-bottom:1px solid #f4f4f5;text-align:right">{}</td></tr>"#,
            i.qty,
            esc(&i.name),
            ui::format_price(i.total_cents())
        ));
        text.push_str(&format!("{} × {}  {}\n", i.qty, i.name, ui::format_price(i.total_cents())));
    }
    let shipping = match o.delivery {
        Delivery::Pickup => "Retiro".to_string(),
        Delivery::Shipping if o.shipping_cents == 0 => "Gratis o a coordinar".to_string(),
        Delivery::Shipping => ui::format_price(o.shipping_cents),
    };
    html.push_str(&format!(
        r#"<tr><td style="padding:6px 0;color:#52525b">Envío</td><td style="padding:6px 0;text-align:right;color:#52525b">{}</td></tr>
<tr><td style="padding:6px 0;font-weight:700;font-size:16px">Total</td><td style="padding:6px 0;text-align:right;font-weight:700;font-size:16px">{}</td></tr></table>"#,
        esc(&shipping),
        ui::format_price(o.total_cents)
    ));
    text.push_str(&format!("Envío  {shipping}\nTotal  {}\n", ui::format_price(o.total_cents)));
    (html, text)
}

fn where_(o: &Order, pickup_address: &str) -> String {
    match o.delivery {
        Delivery::Pickup if pickup_address.is_empty() => "Retiro en el local".to_string(),
        Delivery::Pickup => format!("Retiro en {pickup_address}"),
        Delivery::Shipping => format!("Envío a {}, {} ({})", o.address, o.city, o.postal_code),
    }
}

/// Al comprador: su compra, con el enlace a la página del pedido. `paid`:
/// ya pagó en Mercado Pago (si no, la tienda coordina el pago).
pub fn to_buyer(store_name: &str, slug: &str, o: &Order, pickup_address: &str, paid: bool) -> Email {
    let first = o.customer_name.split_whitespace().next().unwrap_or("");
    let url = format!("{}/pedido/{}", store_url(slug), o.id);
    let (lines_html, lines_text) = order_lines(o);
    let next = if paid {
        "Tu pago está aprobado. La tienda ya está preparando tu pedido.".to_string()
    } else {
        format!("La tienda te va a escribir al {} para coordinar el pago y la entrega.", o.phone)
    };
    let place = where_(o, pickup_address);
    let html = frame(
        store_name,
        &format!(
            r#"<h1 style="font-size:20px;margin:0 0 8px">¡Gracias por tu compra, {}!</h1>
<p style="margin:0;color:#52525b">Pedido #{} · {}</p>{lines_html}
<p style="margin:16px 0 0;color:#52525b">{}</p>{}"#,
            esc(first),
            o.number,
            esc(&next),
            esc(&place),
            button(&url, "Ver mi pedido")
        ),
    );
    let text = format!(
        "¡Gracias por tu compra, {first}!\nPedido #{} · {next}\n\n{lines_text}\n{place}\n\nTu pedido: {url}\n",
        o.number
    );
    Email {
        to: vec![o.email.clone()],
        subject: format!("Tu compra en {store_name} · pedido #{}", o.number),
        html,
        text,
    }
}

/// Al dueño: hay una venta nueva, con el enlace a Pedidos.
pub fn to_owner(owners: Vec<String>, store_name: &str, slug: &str, o: &Order, paid: bool) -> Email {
    let url = format!("{}/tienda/{slug}/pedidos", app_url());
    let (lines_html, lines_text) = order_lines(o);
    let what = if paid {
        format!("{} pagó el pedido #{} con Mercado Pago.", o.customer_name, o.number)
    } else {
        format!(
            "{} hizo el pedido #{}. Escribile al {} para coordinar el pago.",
            o.customer_name, o.number, o.phone
        )
    };
    let place = match o.delivery {
        Delivery::Pickup => "Retira en el local".to_string(),
        Delivery::Shipping => format!("Envío a domicilio · {}", o.city),
    };
    let notes = if o.notes.is_empty() {
        String::new()
    } else {
        format!("Nota: {}", o.notes)
    };
    let html = frame(
        "MarketMaker",
        &format!(
            r#"<h1 style="font-size:20px;margin:0 0 8px">Nueva venta en {}: {}</h1>
<p style="margin:0;color:#52525b">{}</p>{lines_html}
<p style="margin:16px 0 0;color:#52525b">{}<br>{} · {}</p>{}{}"#,
            esc(store_name),
            ui::format_price(o.total_cents),
            esc(&what),
            esc(&place),
            esc(&o.email),
            esc(&o.phone),
            if notes.is_empty() { String::new() } else { format!(r#"<p style="margin:12px 0 0;background:#f4f4f5;padding:10px;border-radius:8px">{}</p>"#, esc(&notes)) },
            button(&url, "Ver pedidos")
        ),
    );
    let text = format!(
        "Nueva venta en {store_name}: {}\n{what}\n\n{lines_text}\n{place}\n{} · {}\n{notes}\n\nPedidos: {url}\n",
        ui::format_price(o.total_cents),
        o.email,
        o.phone
    );
    Email {
        to: owners,
        subject: if paid {
            format!("Nueva venta: {} · pedido #{}", ui::format_price(o.total_cents), o.number)
        } else {
            format!(
                "Nuevo pedido: {} · #{} (coordiná el pago)",
                ui::format_price(o.total_cents),
                o.number
            )
        },
        html,
        text,
    }
}

/// Manda los dos emails de una venta: al comprador y a los dueños.
pub async fn order_emails(pool: &db::Pool, tenant: uuid::Uuid, order: uuid::Uuid, paid: bool) {
    let r = async {
        let o = db::orders::get(pool, tenant, order).await.map_err(|e| e.to_string())?.ok_or("no está el pedido")?;
        let (name, slug) = db::mp::name_and_slug(pool, tenant).await.map_err(|e| e.to_string())?.ok_or("no está la tienda")?;
        let d = db::orders::delivery(pool, tenant).await.map_err(|e| e.to_string())?;
        let owners = db::mp::owner_emails(pool, tenant).await.map_err(|e| e.to_string())?;
        send(to_buyer(&name, &slug, &o, &d.pickup_address, paid));
        send(to_owner(owners, &name, &slug, &o, paid));
        Ok::<_, String>(())
    }
    .await;
    if let Err(e) = r {
        leptos::logging::log!("Emails del pedido {order}: {e}");
    }
}


/// El código para entrar.
pub fn login_code(to: &str, code: &str) -> Email {
    let html = frame(
        "MarketMaker",
        &format!(
            r#"<h1 style="font-size:20px;margin:0 0 8px">Tu código para entrar</h1>
<p style="font-size:32px;font-weight:700;letter-spacing:6px;margin:12px 0">{}</p>
<p style="margin:0;color:#52525b">Vence en {} minutos. Si no lo pediste, ignorá este email.</p>"#,
            esc(code),
            db::auth::CODE_MINUTES
        ),
    );
    Email {
        to: vec![to.to_string()],
        subject: format!("{code} es tu código para entrar a MarketMaker"),
        html,
        text: format!(
            "Tu código para entrar a MarketMaker: {code}\nVence en {} minutos. Si no lo pediste, ignorá este email.\n",
            db::auth::CODE_MINUTES
        ),
    }
}
