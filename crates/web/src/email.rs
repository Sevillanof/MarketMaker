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

/// Un archivo adjunto. Con `inline`, el HTML lo muestra con `<img src="cid:…">`.
pub struct Attachment {
    pub filename: String,
    pub content_id: Option<String>,
    pub bytes: Vec<u8>,
}

/// Base64 para los adjuntos (Resend los pide así).
fn base64(bytes: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for c in bytes.chunks(3) {
        let n = (u32::from(c[0]) << 16)
            | (u32::from(*c.get(1).unwrap_or(&0)) << 8)
            | u32::from(*c.get(2).unwrap_or(&0));
        out.push(T[(n >> 18) as usize & 63] as char);
        out.push(T[(n >> 12) as usize & 63] as char);
        out.push(if c.len() > 1 {
            T[(n >> 6) as usize & 63] as char
        } else {
            '='
        });
        out.push(if c.len() > 2 {
            T[n as usize & 63] as char
        } else {
            '='
        });
    }
    out
}

/// Lo manda y espera (para los códigos de entrar: hay que saber si salió).
pub async fn deliver(email: &Email) -> Result<(), String> {
    deliver_attached(email, &[]).await
}

/// Como [`send`], con adjuntos.
pub fn send_attached(email: Email, files: Vec<Attachment>) {
    tokio::spawn(async move {
        if let Err(e) = deliver_attached(&email, &files).await {
            leptos::logging::log!(
                "Email \"{}\" a {}: no se pudo mandar: {e}",
                email.subject,
                email.to.join(", ")
            );
        }
    });
}

pub async fn deliver_attached(email: &Email, files: &[Attachment]) -> Result<(), String> {
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
    let mut body = serde_json::json!({
        "from": from,
        "to": email.to,
        "subject": email.subject,
        "html": email.html,
        "text": email.text,
    });
    if !files.is_empty() {
        body["attachments"] = files
            .iter()
            .map(|f| {
                let mut a = serde_json::json!({
                    "filename": f.filename,
                    "content": base64(&f.bytes),
                });
                if let Some(id) = &f.content_id {
                    a["content_id"] = id.clone().into();
                }
                a
            })
            .collect::<Vec<_>>()
            .into();
    }
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

/// Aviso al administrador de un reporte nuevo, con la captura adentro.
pub fn bug_report(to: Vec<String>, r: &db::reports::Report, has_image: bool) -> Email {
    let who = if r.quien.is_empty() {
        "Visitante sin cuenta"
    } else {
        r.quien.as_str()
    };
    let link = app_url();
    let img = if has_image {
        r#"<p style="margin:16px 0 0"><img src="cid:captura" alt="Captura del problema" style="max-width:100%;border:1px solid #e4e4e7;border-radius:8px"></p>"#
    } else {
        ""
    };
    let body = format!(
        r#"<p style="margin:0 0 12px;font-size:16px;line-height:1.5;white-space:pre-wrap">{}</p>
<p style="margin:0;color:#52525b;font-size:13px;line-height:1.6">Página: {}<br>De: {}<br>Pantalla: {} · {}</p>{img}<p style="margin:16px 0 0;color:#52525b;font-size:13px">Lo ves y cambiás su estado en Reportes, desde el menú de tu cuenta.</p>{}"#,
        esc(&r.descripcion),
        esc(&r.pagina),
        esc(who),
        esc(&r.pantalla),
        esc(&r.navegador),
        button(&link, "Abrir MarketMaker"),
    );
    Email {
        to,
        subject: format!("Reporte #{} · {}", r.num, r.descripcion.chars().take(60).collect::<String>().replace('\n', " ")),
        html: frame(&format!("Reporte #{}", r.num), &body),
        text: format!(
            "Reporte #{}\n\n{}\n\nPágina: {}\nDe: {}\nPantalla: {} · {}\n\nLo ves en Reportes, desde el menú de tu cuenta: {link}\n",
            r.num, r.descripcion, r.pagina, who, r.pantalla, r.navegador
        ),
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
    if o.coupon_cents > 0 {
        let d = format!("−{}", ui::format_price(o.coupon_cents));
        let name = format!("Cupón {}", o.coupon_code);
        html.push_str(&format!(
            r#"<tr><td style="padding:6px 0;color:#15803d">{}</td><td style="padding:6px 0;text-align:right;color:#15803d">{d}</td></tr>"#,
            esc(&name)
        ));
        text.push_str(&format!("{name}  {d}\n"));
    }
    if o.discount_cents > 0 {
        let d = format!("−{}", ui::format_price(o.discount_cents));
        html.push_str(&format!(
            r#"<tr><td style="padding:6px 0;color:#15803d">Descuento por transferencia</td><td style="padding:6px 0;text-align:right;color:#15803d">{d}</td></tr>"#
        ));
        text.push_str(&format!("Descuento por transferencia  {d}\n"));
    }
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
    // Enlace de WhatsApp de la tienda, para mandar el comprobante.
    whatsapp_url: Option<&str>,
) -> Email {
    let first = o.customer_name.split_whitespace().next().unwrap_or("");
    let url = format!("{}/pedido/{}", store_url(slug), o.id);
    // Con transferencia todavía falta pagar: lo dice el título, como la
    // página del pedido, y suma el enlace para mandar el comprobante.
    let owes_transfer = !paid && o.pay == domain::PayMethod::Transfer;
    let title = if owes_transfer {
        format!("{first}, ya casi está: falta la transferencia")
    } else {
        format!("¡Gracias por tu compra, {first}!")
    };
    let wa = whatsapp_url.filter(|_| owes_transfer).map(|u| {
        format!(
            "{u}?text={}",
            domain::encode_query(&format!(
                "Hola, te mando el comprobante de la transferencia del pedido #{}.",
                o.number
            ))
        )
    });
    let wa_html = wa.as_ref().map_or_else(String::new, |u| {
        format!(
            r#"<p style="margin:4px 0 0"><a href="{}" style="display:inline-block;color:#4f46e5;text-decoration:none;font-weight:600;padding:10px 0">Mandar comprobante por WhatsApp</a></p>"#,
            esc(u)
        )
    });
    let wa_text = wa.as_ref().map_or_else(String::new, |u| {
        format!("Mandar comprobante por WhatsApp: {u}\n")
    });
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
    // Su nota para la tienda, para que vea que llegó.
    let (note_html, note_text) = if o.notes.is_empty() {
        (String::new(), String::new())
    } else {
        (
            format!(
                r#"<p style="margin:8px 0 0;color:#52525b">Tu nota para la tienda: {}</p>"#,
                esc(&o.notes)
            ),
            format!("Tu nota para la tienda: {}\n", o.notes),
        )
    };
    let html = frame(
        store_name,
        &format!(
            r#"<h1 style="font-size:20px;margin:0 0 8px">{}</h1>
<p style="margin:0;color:#52525b">Pedido #{} · {}</p>{lines_html}
<p style="margin:16px 0 0;color:#52525b">{}</p>{note_html}{}{wa_html}"#,
            esc(&title),
            o.number,
            esc(&next),
            esc(&place),
            button(&url, "Ver mi pedido")
        ),
    );
    let text = format!(
        "{title}\nPedido #{} · {next}\n\n{lines_text}\n{place}\n{note_text}\nTu pedido: {url}\n{wa_text}",
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
        // Con la calle: es lo que la dueña necesita para despachar.
        Delivery::Shipping => format!(
            "Envío a domicilio · {}",
            o.destination().unwrap_or_else(|| o.city.clone())
        ),
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
            r#"<h1 style="font-size:20px;margin:0 0 8px">{} en {}: {}</h1>
<p style="margin:0;color:#52525b">{}</p>{lines_html}
<p style="margin:16px 0 0;color:#52525b">{}<br>{} · {}</p>{}{}"#,
            if paid { "Nueva venta" } else { "Nuevo pedido" },
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
        "{} en {store_name}: {}\n{what}\n\n{lines_text}\n{place}\n{} · {}\n{notes}\n\nPedidos: {url}\n",
        if paid { "Nueva venta" } else { "Nuevo pedido" },
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
        let whatsapp = db::auth::store_info(pool, tenant)
            .await
            .ok()
            .and_then(|i| i.whatsapp_url());
        send(to_buyer(
            &name,
            &slug,
            &o,
            &d.pickup_address,
            &transfer,
            paid,
            whatsapp.as_deref(),
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

/// El enlace del editor de una tienda, para abrirlo desde una computadora.
pub fn editor_link(to: &str, store: &str, url: &str) -> Email {
    let title = format!("El editor de {store}");
    let html = frame(
        "MarketMaker",
        &format!(
            r#"<h1 style="font-size:20px;margin:0 0 8px">{}</h1>
<p style="margin:0 0 16px;color:#52525b">Pediste el enlace para diseñar tu tienda desde una computadora. Abrilo allá e ingresá con tu cuenta.</p>
{}"#,
            esc(&title),
            button(url, "Abrir el editor")
        ),
    );
    Email {
        to: vec![to.to_string()],
        subject: title.clone(),
        html,
        text: format!(
            "{title}\n\nAbrí este enlace desde una computadora para diseñar tu tienda:\n{url}\n"
        ),
    }
}

/// El enlace para entrar a la cuenta de comprador de una tienda. Si lo pidió
/// al guardar un favorito (`favorite`, el nombre del producto), el email
/// habla de eso y no de pedidos.
pub fn buyer_link(to: &str, store: &str, url: &str, favorite: Option<&str>) -> Email {
    let (title, what, label, plain) = match favorite {
        Some(name) => (
            format!("Guardá «{name}» en tus favoritos"),
            format!("Tocá el botón para entrar a tu cuenta de {store}: el producto queda guardado en tus favoritos."),
            "Guardar en mis favoritos",
            "Abrí este enlace para guardarlo en tus favoritos",
        ),
        None => (
            format!("Entrá a tu cuenta de {store}"),
            "Tocá el botón para ver tus pedidos y el seguimiento de tus compras.".to_string(),
            "Entrar a mi cuenta",
            "Abrí este enlace para ver tus pedidos",
        ),
    };
    let html = frame(
        store,
        &format!(
            r#"<h1 style="font-size:20px;margin:0 0 8px">{}</h1>
<p style="margin:0 0 16px;color:#52525b">{} El enlace vence en {} minutos y sirve una sola vez.</p>
{}
<p style="margin:16px 0 0;color:#52525b">Si no lo pediste, ignorá este email: nadie puede entrar a tu cuenta sin este enlace.</p>"#,
            esc(&title),
            esc(&what),
            db::buyers::LINK_MINUTES,
            button(url, label)
        ),
    );
    Email {
        to: vec![to.to_string()],
        subject: title.clone(),
        html,
        text: format!(
            "{title}\n\n{plain} (vence en {} minutos y sirve una sola vez):\n{url}\n\nSi no lo pediste, ignorá este email.\n",
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

/// Al dueño: entró un pago de Mercado Pago por productos que ya no tenían
/// stock suficiente (otra persona pagó antes las últimas unidades).
pub fn sold_without_stock_to_owner(
    owners: Vec<String>,
    store_name: &str,
    slug: &str,
    o: &Order,
    short: &[db::orders::ShortItem],
) -> Email {
    let url = format!("{}/tienda/{slug}/pedidos", app_url());
    let line = |i: &db::orders::ShortItem| {
        let had = match i.had {
            0 => "no quedaba ninguna".to_string(),
            1 => "quedaba 1".to_string(),
            n => format!("quedaban {n}"),
        };
        format!("{} × {} ({had})", i.sold, i.name)
    };
    let list_html: String = short
        .iter()
        .map(|i| format!("<li>{}</li>", esc(&line(i))))
        .collect();
    let list_text: String = short.iter().map(|i| format!("- {}\n", line(i))).collect();
    let html = frame(
        "MarketMaker",
        &format!(
            r#"<h1 style="font-size:20px;margin:0 0 8px">El pedido #{} trae productos sin stock</h1>
<p style="margin:0;color:#52525b">{} pagó su pedido en {} con Mercado Pago, pero otra compra se llevó antes las últimas unidades:</p>
<ul style="margin:12px 0 0;background:#f4f4f5;padding:10px 10px 10px 28px;border-radius:8px">{}</ul>
<p style="margin:12px 0 0;color:#52525b">El pago ya entró. Escribile para avisarle: podés reponer el producto, ofrecerle otro o devolverle la plata desde Mercado Pago.</p>{}"#,
            o.number,
            esc(&o.customer_name),
            esc(store_name),
            list_html,
            button(&url, "Ver pedido")
        ),
    );
    let text = format!(
        "El pedido #{} de {store_name} trae productos sin stock.\n{} pagó con Mercado Pago, pero otra compra se llevó antes las últimas unidades:\n{list_text}\nEl pago ya entró. Escribile para avisarle: podés reponer el producto, ofrecerle otro o devolverle la plata desde Mercado Pago.\nVer pedido: {url}\n",
        o.number, o.customer_name
    );
    Email {
        to: owners,
        subject: format!("El pedido #{} trae productos sin stock", o.number),
        html,
        text,
    }
}

/// Busca lo que hace falta y manda [`sold_without_stock_to_owner`].
#[cfg(feature = "ssr")]
pub async fn tell_sold_without_stock(
    pool: &db::Pool,
    tenant: uuid::Uuid,
    order: uuid::Uuid,
    short: &[db::orders::ShortItem],
) {
    let Ok(Some(o)) = db::orders::get(pool, tenant, order).await else {
        return;
    };
    let Ok(Some((name, slug))) = db::mp::name_and_slug(pool, tenant).await else {
        return;
    };
    let owners = db::mp::owner_emails(pool, tenant).await.unwrap_or_default();
    if owners.is_empty() {
        return;
    }
    send(sold_without_stock_to_owner(owners, &name, &slug, &o, short));
}

/// Al dueño: un pedido pagado con Mercado Pago se devolvió desde Mercado
/// Pago (o el comprador desconoció el pago) y lo cancelamos.
pub async fn tell_refunded(
    pool: &db::Pool,
    tenant: uuid::Uuid,
    order: uuid::Uuid,
    chargeback: bool,
) {
    let Ok(Some(o)) = db::orders::get(pool, tenant, order).await else {
        return;
    };
    let Ok(Some((name, slug))) = db::mp::name_and_slug(pool, tenant).await else {
        return;
    };
    let owners = db::mp::owner_emails(pool, tenant).await.unwrap_or_default();
    if owners.is_empty() {
        return;
    }
    send(refunded_to_owner(owners, &name, &slug, &o, chargeback));
}

pub fn refunded_to_owner(
    owners: Vec<String>,
    store_name: &str,
    slug: &str,
    o: &Order,
    chargeback: bool,
) -> Email {
    let url = format!("{}/tienda/{slug}/pedidos", app_url());
    let (title, why) = if chargeback {
        (
            format!("El pago del pedido #{} se desconoció", o.number),
            format!(
                "{} desconoció el pago con su tarjeta y Mercado Pago le devolvió la plata. Cancelamos el pedido en {store_name} y las unidades volvieron al stock. Si ya lo enviaste, revisá el reclamo en tu cuenta de Mercado Pago.",
                o.customer_name
            ),
        )
    } else {
        (
            format!("El pedido #{} se devolvió", o.number),
            format!(
                "El pago de {} se devolvió desde Mercado Pago. Cancelamos el pedido en {store_name} y las unidades volvieron al stock.",
                o.customer_name
            ),
        )
    };
    let html = frame(
        "MarketMaker",
        &format!(
            r#"<h1 style="font-size:20px;margin:0 0 8px">{}</h1>
<p style="margin:0;color:#52525b">{}</p>{}"#,
            esc(&title),
            esc(&why),
            button(&url, "Ver pedidos")
        ),
    );
    Email {
        to: owners,
        subject: title.clone(),
        html,
        text: format!("{title}\n{why}\nVer pedidos: {url}\n"),
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

/// Qué cambió en un pedido que el dueño movió a mano en Pedidos.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BuyerUpdate {
    /// Marcó "Ya me pagó" (transferencia o pago coordinado).
    Paid,
    /// Lo despachó (envío propio, no Andreani: ese manda su propio email).
    Shipped,
    /// Retiro en el local: ya está listo para que lo busquen.
    ReadyForPickup,
}

impl BuyerUpdate {
    /// Cómo queda anotado en el pedido, para no mandarlo dos veces.
    pub fn key(self) -> &'static str {
        match self {
            BuyerUpdate::Paid => "pagado",
            BuyerUpdate::Shipped => "enviado",
            BuyerUpdate::ReadyForPickup => "para_retirar",
        }
    }
}

/// Rótulo chico en mayúsculas que separa las partes del email.
fn label(text: &str) -> String {
    format!(
        r#"<p style="margin:28px 0 8px;font-size:12px;font-weight:600;letter-spacing:.06em;text-transform:uppercase;color:#71717a">{}</p>"#,
        esc(text)
    )
}

/// Los productos con la variante en una segunda línea, y el total.
fn order_summary(o: &Order) -> (String, String) {
    let cell = "padding:12px 0;border-bottom:1px solid #e4e4e7";
    let mut html = format!(
        r#"<table style="width:100%;border-collapse:collapse;font-size:14px;line-height:1.5;border-top:1px solid #e4e4e7">"#
    );
    let mut text = String::new();
    for i in &o.items {
        let variant = domain::variant_label(&i.color, &i.size);
        let name = domain::base_name(&i.name, &i.color, &i.size);
        let second = if variant.is_empty() {
            String::new()
        } else {
            format!(
                r#"<br><span style="color:#71717a">{}</span>"#,
                esc(&variant)
            )
        };
        html.push_str(&format!(
            r#"<tr><td style="{cell}">{} × {}{second}</td><td style="{cell};text-align:right;vertical-align:top;white-space:nowrap">{}</td></tr>"#,
            i.qty,
            esc(name),
            ui::format_price(i.total_cents())
        ));
        text.push_str(&format!(
            "{} × {}  {}\n",
            i.qty,
            i.name,
            ui::format_price(i.total_cents())
        ));
    }
    // Descuento y envío, para que la suma cierre con el total.
    let mut extra = vec![];
    if o.coupon_cents > 0 {
        extra.push((
            format!("Cupón {}", o.coupon_code),
            format!("−{}", ui::format_price(o.coupon_cents)),
        ));
    }
    if o.discount_cents > 0 {
        extra.push((
            "Descuento por transferencia".to_string(),
            format!("−{}", ui::format_price(o.discount_cents)),
        ));
    }
    if !matches!(o.delivery, Delivery::Pickup) {
        extra.push(("Envío".to_string(), o.shipping_text(ui::format_price)));
    }
    for (what, amount) in &extra {
        html.push_str(&format!(
            r#"<tr><td style="padding:10px 0 0;color:#52525b">{}</td><td style="padding:10px 0 0;text-align:right;color:#52525b">{}</td></tr>"#,
            esc(what),
            esc(amount)
        ));
        text.push_str(&format!("{what}  {amount}\n"));
    }
    html.push_str(&format!(
        r#"<tr><td style="padding:12px 0 0;font-weight:700">Total</td><td style="padding:12px 0 0;text-align:right;font-weight:700">{}</td></tr></table>"#,
        ui::format_price(o.total_cents)
    ));
    text.push_str(&format!("Total  {}\n", ui::format_price(o.total_cents)));
    (html, text)
}

/// Al comprador, cuando el dueño mueve su pedido: pago confirmado, enviado o
/// listo para retirar. Con más aire que los demás emails: un rótulo por parte.
pub fn update_to_buyer(
    what: BuyerUpdate,
    store_name: &str,
    slug: &str,
    o: &Order,
    d: &domain::DeliverySettings,
    whatsapp: &str,
) -> Email {
    let first = o.customer_name.split_whitespace().next().unwrap_or("");
    let url = format!("{}/pedido/{}", store_url(slug), o.id);
    let store = store_name;
    let ask = if whatsapp.trim().is_empty() {
        String::new()
    } else {
        " Si tenés alguna duda, escribile por WhatsApp.".to_string()
    };
    let pickup = matches!(o.delivery, Delivery::Pickup);
    let (subject, title, intro) = match what {
        BuyerUpdate::Paid => (
            format!("Recibimos tu pago · pedido #{}", o.number),
            if first.is_empty() {
                "¡Listo! Recibimos tu pago".to_string()
            } else {
                format!("¡Listo, {first}! Recibimos tu pago")
            },
            if pickup {
                format!("{store} confirmó tu pago y ya empieza a preparar tu pedido. Te avisamos por email cuando esté listo para retirar.")
            } else {
                format!("{store} confirmó tu pago y ya empieza a preparar tu pedido. Te avisamos por email cuando salga.")
            },
        ),
        BuyerUpdate::Shipped => (
            format!("Tu pedido #{} está en camino", o.number),
            "Tu pedido está en camino".to_string(),
            format!("{store} ya envió tu pedido.{ask}"),
        ),
        BuyerUpdate::ReadyForPickup => (
            format!("Tu pedido #{} está listo para retirar", o.number),
            "Tu pedido está listo para retirar".to_string(),
            "Ya podés pasar a buscarlo. Llevá el número de pedido.".to_string(),
        ),
    };
    // Dónde: la dirección del envío o la del local, con su horario.
    let (place_label, mut place) = if pickup {
        let mut lines = vec![];
        if !d.pickup_address.trim().is_empty() {
            lines.push(d.pickup_address.trim().to_string());
        }
        if !d.pickup_hours.trim().is_empty() {
            lines.push(d.pickup_hours.trim().to_string());
        }
        ("Dónde retirarlo", lines)
    } else {
        let to = o.destination().unwrap_or_else(|| o.city.clone());
        let l = if what == BuyerUpdate::Shipped {
            "Lo enviamos a"
        } else if matches!(o.delivery, Delivery::Andreani(s) if s.needs_branch()) {
            "Lo retirás en"
        } else {
            "Envío a domicilio"
        };
        (
            l,
            if to.trim().is_empty() {
                vec![]
            } else {
                vec![to]
            },
        )
    };
    if what == BuyerUpdate::Shipped && !whatsapp.trim().is_empty() {
        place.push(format!("WhatsApp de la tienda: {}", whatsapp.trim()));
    }
    let (lines_html, lines_text) = order_summary(o);
    let place_html = if place.is_empty() {
        String::new()
    } else {
        format!(
            r#"{}<p style="margin:0;color:#3f3f46;line-height:1.6;font-size:14px">{}</p>"#,
            label(place_label),
            place
                .iter()
                .map(|l| esc(l))
                .collect::<Vec<_>>()
                .join("<br>")
        )
    };
    let html = format!(
        r#"<!doctype html><html lang="es"><body style="margin:0;background:#f4f4f5;font-family:-apple-system,Segoe UI,Roboto,Helvetica,Arial,sans-serif;color:#18181b">
<div style="max-width:560px;margin:0 auto;padding:32px 16px">
<p style="font-weight:700;font-size:18px;margin:0 0 16px">{}</p>
<div style="background:#fff;border-radius:12px;padding:32px 28px">
<h1 style="font-size:22px;line-height:1.3;margin:0 0 12px">{}</h1>
<p style="margin:0;color:#52525b;line-height:1.6;font-size:15px">{}</p>
{}{lines_html}{place_html}
<p style="margin:32px 0 0"><a href="{}" style="display:inline-block;background:#4f46e5;color:#fff;text-decoration:none;font-weight:600;padding:12px 22px;border-radius:8px">Ver mi pedido</a></p></div>
<p style="color:#71717a;font-size:12px;margin:20px 0 0;text-align:center">Hecho con MarketMaker</p>
</div></body></html>"#,
        esc(store),
        esc(&title),
        esc(&intro),
        label(&format!("Tu pedido #{}", o.number)),
        esc(&url)
    );
    let text = format!(
        "{title}\n{intro}\n\nTu pedido #{}\n{lines_text}\n{}{}\n\nTu pedido: {url}\n",
        o.number,
        if place.is_empty() {
            String::new()
        } else {
            format!("{place_label}:\n")
        },
        place.join("\n")
    );
    Email {
        to: vec![o.email.clone()],
        subject,
        html,
        text,
    }
}

/// Avisos al comprador por un pedido que no avanza.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BuyerNotice {
    /// Pasó un día y falta la transferencia: todavía está a tiempo.
    Reminder,
    /// Pasaron 48 horas sin el pago: se canceló solo.
    Expired,
    /// Lo canceló la tienda.
    CancelledByStore,
}

impl BuyerNotice {
    /// Cómo queda anotado en el pedido, para no mandarlo dos veces.
    pub fn key(self) -> &'static str {
        match self {
            BuyerNotice::Reminder => "recordatorio",
            BuyerNotice::Expired | BuyerNotice::CancelledByStore => "cancelado",
        }
    }
}

/// Al comprador: «te falta la transferencia» o «tu pedido se canceló», con
/// qué pasó, qué puede hacer y cómo escribirle a la tienda. `transfer` son
/// los datos para transferir (solo en el recordatorio).
pub fn notice_to_buyer(
    what: BuyerNotice,
    store_name: &str,
    slug: &str,
    o: &Order,
    whatsapp_url: Option<&str>,
    transfer: &str,
) -> Email {
    let first = o.customer_name.split_whitespace().next().unwrap_or("");
    let store = store_name;
    let order_url = format!("{}/pedido/{}", store_url(slug), o.id);
    let shop_url = store_url(slug);
    let (subject, title, intro, side_label, side, button, url) = match what {
        BuyerNotice::Reminder => (
            format!("Te falta transferir · pedido #{}", o.number),
            if first.is_empty() {
                "Te falta la transferencia".to_string()
            } else {
                format!("{first}, te falta la transferencia")
            },
            format!("Tu pedido en {store} está reservado. Para que no se cancele, hacé la transferencia en las próximas 24 horas y mandá el comprobante."),
            "Adónde transferir",
            if transfer.trim().is_empty() {
                format!("Importe: {}", ui::format_price(o.total_cents))
            } else {
                format!("Alias o CBU: {}\nImporte: {}", transfer.trim(), ui::format_price(o.total_cents))
            },
            "Ver mi pedido",
            order_url,
        ),
        BuyerNotice::Expired => (
            format!("Tu pedido #{} se canceló", o.number),
            "Tu pedido se canceló".to_string(),
            "Pasaron 48 horas y no recibimos el pago, así que el pedido se canceló y los productos volvieron a estar disponibles. No se te cobró nada.".to_string(),
            "¿Ya habías pagado?",
            format!("Escribile a {store} con el comprobante y lo resuelven."),
            "Volver a la tienda",
            shop_url,
        ),
        BuyerNotice::CancelledByStore => (
            format!("Tu pedido #{} se canceló", o.number),
            "Tu pedido se canceló".to_string(),
            format!("{store} canceló tu pedido. Si ya habías pagado, escribile a la tienda para coordinar la devolución."),
            "¿Tenés dudas?",
            format!("Escribile a {store} y te cuentan qué pasó."),
            "Volver a la tienda",
            shop_url,
        ),
    };
    let (lines_html, lines_text) = order_summary(o);
    let side_html = side.lines().map(esc).collect::<Vec<_>>().join("<br>");
    let wa_label = if what == BuyerNotice::Reminder {
        "Mandar comprobante por WhatsApp"
    } else {
        "Escribir por WhatsApp"
    };
    let wa_html = whatsapp_url.map_or_else(String::new, |u| {
        format!(
            r#" <a href="{}" style="display:inline-block;color:#4f46e5;text-decoration:none;font-weight:600;padding:12px 14px">{wa_label}</a>"#,
            esc(u)
        )
    });
    let html = format!(
        r#"<!doctype html><html lang="es"><body style="margin:0;background:#f4f4f5;font-family:-apple-system,Segoe UI,Roboto,Helvetica,Arial,sans-serif;color:#18181b">
<div style="max-width:560px;margin:0 auto;padding:32px 16px">
<p style="font-weight:700;font-size:18px;margin:0 0 16px">{}</p>
<div style="background:#fff;border-radius:12px;padding:32px 28px">
<h1 style="font-size:22px;line-height:1.3;margin:0 0 12px">{}</h1>
<p style="margin:0;color:#52525b;line-height:1.6;font-size:15px">{}</p>
{}<p style="margin:0;color:#3f3f46;line-height:1.6;font-size:14px">{side_html}</p>
{}{lines_html}
<p style="margin:32px 0 0"><a href="{}" style="display:inline-block;background:#4f46e5;color:#fff;text-decoration:none;font-weight:600;padding:12px 22px;border-radius:8px">{button}</a>{wa_html}</p></div>
<p style="color:#71717a;font-size:12px;margin:20px 0 0;text-align:center">Hecho con MarketMaker</p>
</div></body></html>"#,
        esc(store),
        esc(&title),
        esc(&intro),
        label(side_label),
        label(&format!("Tu pedido #{}", o.number)),
        esc(&url)
    );
    let text = format!(
        "{title}\n{intro}\n\n{side_label}\n{side}\n\nTu pedido #{}\n{lines_text}\n{button}: {url}\n{}",
        o.number,
        whatsapp_url.map_or_else(String::new, |u| format!("{wa_label}: {u}\n"))
    );
    Email {
        to: vec![o.email.clone()],
        subject,
        html,
        text,
    }
}

/// Manda un aviso de [`BuyerNotice`] por el pedido `o` (una sola vez).
pub async fn tell_cancelled(pool: &db::Pool, tenant: uuid::Uuid, o: Order, what: BuyerNotice) {
    if o.email.trim().is_empty() || o.number <= 0 {
        return;
    }
    if !db::orders::note_buyer_update(pool, tenant, &o.id, what.key())
        .await
        .unwrap_or(false)
    {
        return;
    }
    let Ok(Some((name, slug))) = db::mp::name_and_slug(pool, tenant).await else {
        return;
    };
    let whatsapp = db::auth::store_info(pool, tenant)
        .await
        .ok()
        .and_then(|i| i.whatsapp_url());
    let transfer = if what == BuyerNotice::Reminder {
        db::orders::payment_options(pool, tenant)
            .await
            .map(|p| p.transfer_text())
            .unwrap_or_default()
    } else {
        String::new()
    };
    send(notice_to_buyer(
        what,
        &name,
        &slug,
        &o,
        whatsapp.as_deref(),
        &transfer,
    ));
}

/// Les recuerda la transferencia a quienes compraron hace más de `hours`
/// horas y todavía no pagaron (una vez por pedido).
pub async fn remind_unpaid(pool: &db::Pool, hours: i32) {
    match db::orders::unpaid_transfers(pool, hours).await {
        Ok(list) => {
            for (tenant, o) in list {
                tell_cancelled(pool, tenant, o, BuyerNotice::Reminder).await;
            }
        }
        Err(e) => {
            leptos::logging::log!("Pedidos: no pudimos buscar a quién recordarle el pago: {e}")
        }
    }
}

/// Cancela los pedidos sin pagar hace más de `hours` horas (ver
/// `db::orders::expire_unpaid`) y le avisa a cada comprador. Devuelve cuántos
/// sacó en total.
pub async fn expire_unpaid(pool: &db::Pool, hours: i32) -> Result<usize, String> {
    let (dropped, cancelled) = db::orders::expire_unpaid_listed(pool, hours)
        .await
        .map_err(|e| e.to_string())?;
    let n = dropped + cancelled.len();
    for (tenant, o) in cancelled {
        tell_cancelled(pool, tenant, o, BuyerNotice::Expired).await;
    }
    Ok(n)
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
