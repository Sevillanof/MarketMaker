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
    std::env::var(name)
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
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
            leptos::logging::log!(
                "Email \"{}\" a {}: no se pudo mandar: {e}",
                email.subject,
                email.to.join(", ")
            );
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
        Err(format!(
            "{status} {}",
            text.chars().take(300).collect::<String>()
        ))
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
    let mut html = String::from(
        r#"<table style="width:100%;border-collapse:collapse;font-size:14px;margin-top:12px">"#,
    );
    let mut text = String::new();
    for i in &o.items {
        html.push_str(&format!(
            r#"<tr><td style="padding:6px 0;border-bottom:1px solid #f4f4f5">{} × {}</td><td style="padding:6px 0;border-bottom:1px solid #f4f4f5;text-align:right">{}</td></tr>"#,
            i.qty,
            esc(&i.name),
            ui::format_price(i.total_cents())
        ));
        text.push_str(&format!(
            "{} × {}  {}\n",
            i.qty,
            i.name,
            ui::format_price(i.total_cents())
        ));
    }
    let shipping = match o.delivery {
        Delivery::Pickup => "Retiro".to_string(),
        _ => o.shipping_text(ui::format_price),
    };
    html.push_str(&format!(
        r#"<tr><td style="padding:6px 0;color:#52525b">Envío</td><td style="padding:6px 0;text-align:right;color:#52525b">{}</td></tr>
<tr><td style="padding:6px 0;font-weight:700;font-size:16px">Total</td><td style="padding:6px 0;text-align:right;font-weight:700;font-size:16px">{}</td></tr></table>"#,
        esc(&shipping),
        ui::format_price(o.total_cents)
    ));
    text.push_str(&format!(
        "Envío  {shipping}\nTotal  {}\n",
        ui::format_price(o.total_cents)
    ));
    (html, text)
}

fn where_(o: &Order, pickup_address: &str) -> String {
    match o.delivery {
        Delivery::Pickup if pickup_address.is_empty() => "Retiro en el local".to_string(),
        Delivery::Pickup => format!("Retiro en {pickup_address}"),
        Delivery::Shipping => format!(
            "Envío a {}",
            domain::ship_address(&o.address, &o.city, &o.postal_code)
        ),
        Delivery::Andreani(s) => format!("{} · {}", s.label(), o.destination().unwrap_or_default()),
    }
}

/// Al comprador: su compra, con el enlace a la página del pedido. `paid`:
/// ya pagó en Mercado Pago (si no, la tienda coordina el pago).
pub fn to_buyer(
    store_name: &str,
    slug: &str,
    o: &Order,
    pickup_address: &str,
    transfer_info: &str,
    paid: bool,
) -> Email {
    let first = o.customer_name.split_whitespace().next().unwrap_or("");
    let url = format!("{}/pedido/{}", store_url(slug), o.id);
    let (lines_html, lines_text) = order_lines(o);
    let next = if paid {
        "Tu pago está aprobado. La tienda ya está preparando tu pedido.".to_string()
    } else if o.pay == domain::PayMethod::Transfer {
        format!(
            "Transferí {} a: {}. Cuando la tienda reciba la transferencia, empieza a preparar tu pedido. Si no la recibe en 48 horas, el pedido se cancela.",
            ui::format_price(o.total_cents),
            transfer_info.replace('\n', ", ")
        )
    } else {
        format!(
            "La tienda te va a escribir al {} para coordinar el pago. Cuando lo reciba, empieza a preparar tu pedido.",
            o.phone
        )
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
        format!(
            "{} pagó el pedido #{} con Mercado Pago.",
            o.customer_name, o.number
        )
    } else if o.pay == domain::PayMethod::Transfer {
        format!(
            "{} hizo el pedido #{} y va a pagar con transferencia. Cuando la recibas, tocá \"Ya me pagó\" en Pedidos. Si no lo marcás en 48 horas, se cancela solo.",
            o.customer_name, o.number
        )
    } else {
        format!(
            "{} hizo el pedido #{}. Escribile al {} para coordinar el pago y, cuando cobres, tocá \"Ya me pagó\" en Pedidos. Si no lo marcás en 48 horas, se cancela solo.",
            o.customer_name, o.number, o.phone
        )
    };
    let place = match o.delivery {
        Delivery::Pickup => "Retira en el local".to_string(),
        Delivery::Shipping => format!("Envío a domicilio · {}", o.city),
        Delivery::Andreani(s) => format!("{} · {}", s.label(), o.destination().unwrap_or_default()),
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
            if notes.is_empty() {
                String::new()
            } else {
                format!(
                    r#"<p style="margin:12px 0 0;background:#f4f4f5;padding:10px;border-radius:8px">{}</p>"#,
                    esc(&notes)
                )
            },
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
            format!(
                "Nueva venta: {} · pedido #{}",
                ui::format_price(o.total_cents),
                o.number
            )
        } else {
            format!(
                "Nuevo pedido: {} · #{} (esperando el pago)",
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
        let o = db::orders::get(pool, tenant, order)
            .await
            .map_err(|e| e.to_string())?
            .ok_or("no está el pedido")?;
        let (name, slug) = db::mp::name_and_slug(pool, tenant)
            .await
            .map_err(|e| e.to_string())?
            .ok_or("no está la tienda")?;
        let d = db::orders::delivery(pool, tenant)
            .await
            .map_err(|e| e.to_string())?;
        let owners = db::mp::owner_emails(pool, tenant)
            .await
            .map_err(|e| e.to_string())?;
        let transfer = db::orders::payment_options(pool, tenant)
            .await
            .map_err(|e| e.to_string())?
            .transfer_text();
        send(to_buyer(
            &name,
            &slug,
            &o,
            &d.pickup_address,
            &transfer,
            paid,
        ));
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
            r#"<h1 style="font-size:20px;margin:0 0 8px">Tu código de MarketMaker</h1>
<p style="font-size:32px;font-weight:700;letter-spacing:6px;margin:12px 0">{}</p>
<p style="margin:0;color:#52525b">Vence en {} minutos. Si no lo pediste, ignorá este email.</p>"#,
            esc(code),
            db::auth::CODE_MINUTES
        ),
    );
    Email {
        to: vec![to.to_string()],
        subject: format!("{code} es tu código de MarketMaker"),
        html,
        text: format!(
            "Tu código de MarketMaker: {code}\nVence en {} minutos. Si no lo pediste, ignorá este email.\n",
            db::auth::CODE_MINUTES
        ),
    }
}

/// El enlace para entrar a la cuenta de comprador de una tienda.
pub fn buyer_link(to: &str, store: &str, url: &str) -> Email {
    let title = format!("Entrá a tu cuenta de {store}");
    let html = frame(
        store,
        &format!(
            r#"<h1 style="font-size:20px;margin:0 0 8px">{}</h1>
<p style="margin:0 0 16px;color:#52525b">Tocá el botón para ver tus pedidos y el seguimiento de tus compras. El enlace vence en {} minutos y sirve una sola vez.</p>
{}
<p style="margin:16px 0 0;color:#52525b">Si no lo pediste, ignorá este email: nadie puede entrar a tu cuenta sin este enlace.</p>"#,
            esc(&title),
            db::buyers::LINK_MINUTES,
            button(url, "Entrar a mi cuenta")
        ),
    );
    Email {
        to: vec![to.to_string()],
        subject: title.clone(),
        html,
        text: format!(
            "{title}\n\nAbrí este enlace para ver tus pedidos (vence en {} minutos y sirve una sola vez):\n{url}\n\nSi no lo pediste, ignorá este email.\n",
            db::buyers::LINK_MINUTES
        ),
    }
}

/// El enlace para elegir la contraseña (se la olvidó, o confirma una cuenta
/// nueva del registro abierto).
pub fn password_link(to: &str, url: &str, new_account: bool) -> Email {
    let (title, label) = if new_account {
        ("Confirmá tu cuenta de MarketMaker", "Elegir mi contraseña")
    } else {
        ("Elegí una contraseña nueva", "Elegir contraseña nueva")
    };
    let html = frame(
        "MarketMaker",
        &format!(
            r#"<h1 style="font-size:20px;margin:0 0 8px">{title}</h1>
<p style="margin:0 0 16px;color:#52525b">Tocá el botón para elegir tu contraseña. El enlace vence en {} minutos.</p>
{}
<p style="margin:16px 0 0;color:#52525b">Si no lo pediste, ignorá este email: tu cuenta sigue igual.</p>"#,
            db::auth::CODE_MINUTES,
            button(url, label)
        ),
    );
    Email {
        to: vec![to.to_string()],
        subject: title.to_string(),
        html,
        text: format!(
            "{title}\n\nAbrí este enlace para elegir tu contraseña (vence en {} minutos):\n{url}\n\nSi no lo pediste, ignorá este email.\n",
            db::auth::CODE_MINUTES
        ),
    }
}

/// La invitación a crear la cuenta, con el enlace personal. La manda el
/// administrador desde la Whitelist, con el asunto, el título y el mensaje
/// que haya dejado (ya pasados por [`domain::InviteText::clean`]).
pub fn invitation(to: &str, url: &str, words: &domain::InviteText) -> Email {
    let days = db::auth::INVITE_DAYS;
    let step = |n: u8, title: &str, text: &str| {
        format!(
            r#"<tr><td width="44" valign="top" style="padding:0 0 18px"><div style="width:30px;height:30px;line-height:30px;border-radius:15px;background:#eeebff;color:#4330d6;font-weight:700;font-size:14px;text-align:center">{n}</div></td><td valign="top" style="padding:0 0 18px"><div style="font-weight:700;font-size:15px;color:#18181b">{title}</div><div style="font-size:14px;line-height:21px;color:#52525b">{text}</div></td></tr>"#
        )
    };
    let steps = [
        step(1, "Creá tu cuenta", "Elegís tu nombre y una contraseña."),
        step(2, "Armá tu tienda", "Diseño, productos y formas de cobro."),
        step(3, "Publicala", "Tu dirección propia: tutienda.mkr.com.ar"),
    ]
    .concat();
    let html = format!(
        r#"<!doctype html><html lang="es"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><meta name="color-scheme" content="light"></head><body style="margin:0;background:#f4f4f7;font-family:-apple-system,Segoe UI,Roboto,Helvetica,Arial,sans-serif;color:#18181b">
<div style="display:none;max-height:0;overflow:hidden">Tu enlace personal para crear la cuenta. Vence en {days} días.</div>
<div style="max-width:520px;margin:0 auto;padding:32px 16px">
<div style="background:#ffffff;border:1px solid #e4e4e7;border-radius:20px;padding:36px">
<table role="presentation" cellpadding="0" cellspacing="0" style="border-collapse:collapse"><tr><td style="width:36px;height:36px;border-radius:10px;background:#4330d6;background-image:linear-gradient(135deg,#4330d6,#6a55f0);color:#ffffff;font-weight:800;font-size:20px;text-align:center">m</td><td style="padding-left:10px;font-weight:800;font-size:18px;color:#18181b">MarketMaker</td></tr></table>
<h1 style="font-size:28px;line-height:34px;font-weight:800;letter-spacing:-0.5px;margin:28px 0 10px;color:#18181b">{title}</h1>
<p style="font-size:16px;line-height:24px;margin:0 0 26px;color:#52525b">{message}</p>
<table role="presentation" cellpadding="0" cellspacing="0" width="100%" style="border-collapse:collapse">{steps}</table>
<a href="{href}" style="display:block;margin:8px 0 0;background:#4330d6;color:#ffffff;text-decoration:none;font-weight:700;font-size:16px;text-align:center;padding:15px 20px;border-radius:12px">Crear mi cuenta</a>
<p style="font-size:13px;line-height:20px;margin:14px 0 0;color:#71717a;text-align:center">El enlace es personal y vence en {days} días.</p>
</div>
<p style="font-size:12px;line-height:19px;margin:18px 0 0;color:#71717a;text-align:center">Si no esperabas esta invitación, podés ignorar este email.<br>MarketMaker · mkr.com.ar</p>
</div></body></html>"#,
        href = esc(url),
        title = esc(&words.title),
        message = esc(&words.message).replace('\n', "<br>")
    );
    Email {
        to: vec![to.to_string()],
        subject: words.subject.clone(),
        html,
        text: format!(
            "{title}\n\n{message}\n\n1. Creá tu cuenta: elegís tu nombre y una contraseña.\n2. Armá tu tienda: diseño, productos y formas de cobro.\n3. Publicala: tu dirección propia, tutienda.mkr.com.ar\n\nCrear mi cuenta:\n{url}\n\nEl enlace es personal y vence en {days} días.\n\nSi no esperabas esta invitación, podés ignorar este email.\nMarketMaker · mkr.com.ar\n",
            title = words.title,
            message = words.message
        ),
    }
}

/// Botón de arrepentimiento, al comprador: recibimos tu pedido, con el código.
pub fn regret_to_buyer(to: &str, store_name: &str, number: i32, code: &str) -> Email {
    let what = format!(
        "Recibimos tu pedido para arrepentirte de la compra #{number} en {store_name}. Tu código es {code}. La tienda se va a comunicar con vos para coordinar la devolución y el reintegro."
    );
    let html = frame(
        store_name,
        &format!(
            r#"<h1 style="font-size:20px;margin:0 0 8px">Pedido de arrepentimiento recibido</h1>
<p style="margin:0;color:#52525b">{}</p>
<p style="margin:16px 0 0;font-size:22px;font-weight:700;letter-spacing:1px">{}</p>"#,
            esc(&what),
            esc(code)
        ),
    );
    Email {
        to: vec![to.to_string()],
        subject: format!("Arrepentimiento de la compra #{number}: código {code}"),
        html,
        text: format!("{what}\n"),
    }
}

/// Botón de arrepentimiento, a los dueños: un comprador quiere revocar la compra.
pub fn regret_to_owner(
    owners: Vec<String>,
    store_name: &str,
    slug: &str,
    o: &Order,
    code: &str,
    reason: &str,
) -> Email {
    let url = format!("{}/tienda/{slug}/pedidos", app_url());
    let what = format!(
        "{} usó el botón de arrepentimiento para el pedido #{} ({}). Tiene derecho a revocar la compra dentro de los 10 días de recibirla: comunicate con {} · {} para coordinar la devolución y el reintegro. Código: {code}.",
        o.customer_name,
        o.number,
        ui::format_price(o.total_cents),
        o.email,
        o.phone
    );
    let reason_line = if reason.is_empty() {
        String::new()
    } else {
        format!("Motivo: {reason}")
    };
    let html = frame(
        "MarketMaker",
        &format!(
            r#"<h1 style="font-size:20px;margin:0 0 8px">Arrepentimiento en {}: pedido #{}</h1>
<p style="margin:0;color:#52525b">{}</p>{}{}"#,
            esc(store_name),
            o.number,
            esc(&what),
            if reason_line.is_empty() {
                String::new()
            } else {
                format!(
                    r#"<p style="margin:12px 0 0;background:#f4f4f5;padding:10px;border-radius:8px">{}</p>"#,
                    esc(&reason_line)
                )
            },
            button(&url, "Ver pedidos")
        ),
    );
    Email {
        to: owners,
        subject: format!("Arrepentimiento: pedido #{} · {store_name}", o.number),
        html,
        text: format!("{what}\n{reason_line}\n\nPedidos: {url}\n"),
    }
}

/// Al dueño: no se pudo crear el envío con Andreani de un pedido pagado.
pub fn shipment_failed_to_owner(
    owners: Vec<String>,
    store_name: &str,
    slug: &str,
    o: &Order,
    reason: &str,
) -> Email {
    let url = format!("{}/tienda/{slug}/pedidos", app_url());
    let html = frame(
        "MarketMaker",
        &format!(
            r#"<h1 style="font-size:20px;margin:0 0 8px">No pudimos crear el envío del pedido #{}</h1>
<p style="margin:0;color:#52525b">{} pagó su pedido en {}, pero Andreani no aceptó crear el envío.</p>
<p style="margin:12px 0 0;background:#f4f4f5;padding:10px;border-radius:8px">{}</p>
<p style="margin:12px 0 0;color:#52525b">Abrí el pedido y tocá "Reintentar". Si el motivo dice que Andreani no respondió, revisá primero en Andreani si el envío se creó igual.</p>{}"#,
            o.number,
            esc(&o.customer_name),
            esc(store_name),
            esc(reason),
            button(&url, "Ver pedido")
        ),
    );
    let text = format!(
        "No pudimos crear el envío del pedido #{} de {store_name}.\n{reason}\n\nAbrí el pedido y tocá \"Reintentar\": {url}\n",
        o.number
    );
    Email {
        to: owners,
        subject: format!("No pudimos crear el envío del pedido #{}", o.number),
        html,
        text,
    }
}

/// Al comprador: su pedido salió con Andreani, con el número de seguimiento.
pub fn shipped_to_buyer(store_name: &str, slug: &str, o: &Order, tracking: &str) -> Email {
    let first = o.customer_name.split_whitespace().next().unwrap_or("");
    let track = domain::tracking_url(tracking);
    let url = format!("{}/pedido/{}", store_url(slug), o.id);
    let what = match &o.shipment {
        Some(s) if s.service.needs_branch() => format!(
            "Tu pedido #{} ya está en manos de Andreani. Te avisan cuando esté listo para retirar en la sucursal {} ({}). Llevá tu DNI.",
            o.number, s.branch_name, s.branch_address
        ),
        _ => format!(
            "Tu pedido #{} ya está en manos de Andreani y va camino a {}.",
            o.number,
            o.destination().unwrap_or_default()
        ),
    };
    let html = frame(
        store_name,
        &format!(
            r#"<h1 style="font-size:20px;margin:0 0 8px">¡{}, tu pedido está en camino!</h1>
<p style="margin:0;color:#52525b">{}</p>
<p style="margin:16px 0 0">Número de seguimiento: <b>{}</b></p>{}
<p style="margin:16px 0 0;font-size:14px"><a href="{}" style="color:#4f46e5">Ver mi pedido</a></p>"#,
            esc(first),
            esc(&what),
            esc(tracking),
            button(&track, "Seguir mi envío"),
            esc(&url)
        ),
    );
    Email {
        to: vec![o.email.clone()],
        subject: format!("Tu pedido #{} está en camino", o.number),
        html,
        text: format!(
            "¡{first}, tu pedido está en camino!\n{what}\nNúmero de seguimiento: {tracking}\nSeguilo en {track}\nTu pedido: {url}\n"
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_invitation_carries_its_link_safely() {
        let url = "https://mkr.com.ar/entrar?recuperar=ana%40correo.com&codigo=abc&nuevo";
        let mail = invitation("ana@correo.com", url, &domain::InviteText::default());
        assert_eq!(mail.to, ["ana@correo.com"]);
        assert!(mail.html.contains(&esc(url)));
        assert!(!mail.html.contains("codigo=abc&nuevo"));
        assert!(mail.text.contains(url));
        assert!(mail.html.contains(">Ya podés crear tu cuenta</h1>"));
        // Lo que escribe el administrador va como texto, nunca como HTML.
        let own = domain::InviteText::clean("Hola", "<b>Hola</b>", "Uno\n<a href=x>dos</a>");
        let mail = invitation("ana@correo.com", url, &own);
        assert_eq!(mail.subject, "Hola");
        assert!(mail.html.contains("&lt;b&gt;Hola&lt;/b&gt;</h1>"));
        assert!(mail.html.contains("Uno<br>&lt;a href=x&gt;dos&lt;/a&gt;"));
        assert!(mail.html.contains(&esc(url)));
        if let Ok(path) = std::env::var("INVITATION_PREVIEW") {
            std::fs::write(path, &mail.html).unwrap();
        }
    }
}
