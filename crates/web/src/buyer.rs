//! Cuentas de compradores (una por tienda): entrar con un enlace que llega
//! por email, ver los pedidos, cambiar los datos y salir.
//!
//! Son independientes de las cuentas de dueño: usan otra cookie
//! (`mm_buyer`) y otra tabla, y una sesión de comprador nunca da acceso al
//! panel ni al editor. La tienda sale siempre de la dirección del pedido,
//! nunca de lo que manda el navegador, así que una cuenta de una tienda no
//! sirve en otra.

use domain::{BuyerMe, BuyerProfile, CodeRequest, Order};
use leptos::prelude::*;
use leptos::server_fn::codec::Json;

#[cfg(feature = "ssr")]
pub use ssr::*;

#[cfg(feature = "ssr")]
mod ssr {
    use super::*;
    use crate::server::{internal, pool};
    use http::{header, HeaderValue};

    /// Nombre de la cookie de sesión del comprador.
    pub const COOKIE: &str = "mm_buyer";

    /// El token de la cookie del comprador en este pedido.
    pub fn token() -> Option<String> {
        let parts = use_context::<http::request::Parts>()?;
        parts
            .headers
            .get_all(header::COOKIE)
            .iter()
            .filter_map(|v| v.to_str().ok())
            .flat_map(|v| v.split(';'))
            .filter_map(|c| c.trim().split_once('='))
            .find(|(name, _)| *name == COOKIE)
            .map(|(_, value)| value.to_string())
            .filter(|v| !v.is_empty() && v.len() <= 128)
    }

    fn set_cookie(value: &str, max_age_secs: i64) {
        let Some(res) = use_context::<leptos_axum::ResponseOptions>() else {
            return;
        };
        let secure = if crate::auth::config().secure_cookie {
            "; Secure"
        } else {
            ""
        };
        let cookie = format!(
            "{COOKIE}={value}; Path=/; HttpOnly; SameSite=Lax; Max-Age={max_age_secs}{secure}"
        );
        if let Ok(v) = HeaderValue::from_str(&cookie) {
            res.append_header(header::SET_COOKIE, v);
        }
    }

    /// El comprador que entró a la tienda de esta dirección, si su sesión
    /// sigue vigente.
    pub async fn current_buyer() -> Result<Option<(db::Tenant, db::buyers::Buyer)>, ServerFnError> {
        let Some(token) = token() else {
            return Ok(None);
        };
        let Ok(tenant) = crate::auth::current_tenant().await else {
            return Ok(None);
        };
        let buyer = db::buyers::session_buyer(&pool()?, tenant.id, &token)
            .await
            .map_err(internal)?;
        Ok(buyer.map(|b| (tenant, b)))
    }

    pub fn to_me(b: db::buyers::Buyer) -> BuyerMe {
        BuyerMe {
            email: b.email,
            name: b.name,
            phone: b.phone,
            address: b.address,
            postal_code: b.postal_code,
            city: b.city,
            avatar: b.avatar,
        }
    }

    /// El pedido como lo ve el comprador: sin el motivo técnico de un envío
    /// que falló, que es para el dueño.
    pub fn for_buyer(mut o: Order) -> Order {
        if let Some(s) = o.shipment.as_mut() {
            s.error.clear();
        }
        o
    }

    pub fn start_session_cookie(token: &str) {
        set_cookie(token, i64::from(db::buyers::SESSION_DAYS) * 24 * 60 * 60);
    }

    pub fn clear_cookie() {
        set_cookie("", 0);
    }
}

/// Quién entró a la cuenta de esta tienda. `None` si nadie.
#[server]
pub async fn buyer_account() -> Result<Option<BuyerMe>, ServerFnError> {
    Ok(current_buyer().await?.map(|(_, b)| to_me(b)))
}

/// Manda al email un enlace para entrar a la cuenta de esta tienda. La
/// respuesta es la misma tenga o no pedidos: así nadie averigua quién compró.
#[server]
pub async fn buyer_request_link(
    email: String,
    /// Producto que tocó con el corazón antes de entrar (`""` si ninguno): se
    /// guarda en favoritos al canjear el enlace.
    favorite: String,
) -> Result<CodeRequest, ServerFnError> {
    let Some(email) = domain::normalize_email(&email) else {
        return Ok(CodeRequest::BadEmail);
    };
    if !crate::auth::may_ask_for_email() {
        return Ok(CodeRequest::TooMany);
    }
    let tenant = crate::auth::current_tenant().await?;
    let pool = pool()?;
    let favorite = favorite.trim().parse::<uuid::Uuid>().ok();
    match db::buyers::create_link_for(&pool, tenant.id, &email, favorite)
        .await
        .map_err(internal)?
    {
        Err(db::buyers::TooMany) => Ok(CodeRequest::TooMany),
        Ok(key) => {
            let url = format!(
                "{}/mi-cuenta/entrar?k={key}",
                crate::email::store_url(&tenant.slug)
            );
            if !crate::auth::emails_go_out() {
                leptos::logging::log!("\n  Enlace de {email} para {}: {url}\n", tenant.name);
            }
            let mail = crate::email::buyer_link(&email, &tenant.name, &url);
            if let Err(e) = crate::email::deliver(&mail).await {
                leptos::logging::log!("No pudimos mandar el enlace a {email}: {e}");
            }
            Ok(CodeRequest::Sent)
        }
    }
}

/// Canjea el enlace del email: si sirve, abre la sesión. `false` si venció o
/// ya se usó.
#[server]
pub async fn buyer_redeem(key: String) -> Result<bool, ServerFnError> {
    let key = key.trim();
    if key.len() != 64 || !key.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Ok(false);
    }
    if !crate::auth::may_try_code() {
        return Ok(false);
    }
    let tenant = crate::auth::current_tenant().await?;
    let pool = pool()?;
    let Some(buyer) = db::buyers::redeem_link(&pool, tenant.id, &key.to_lowercase())
        .await
        .map_err(internal)?
    else {
        return Ok(false);
    };
    let token = db::buyers::create_session(&pool, buyer)
        .await
        .map_err(internal)?;
    start_session_cookie(&token);
    Ok(true)
}

/// Cierra la sesión del comprador en este navegador.
#[server]
pub async fn buyer_logout() -> Result<(), ServerFnError> {
    if let Some(token) = token() {
        db::buyers::delete_session(&pool()?, &token)
            .await
            .map_err(internal)?;
    }
    clear_cookie();
    Ok(())
}

#[cfg(feature = "ssr")]
use crate::server::{internal, pool};

/// Los pedidos del comprador en esta tienda, el más nuevo primero. `None` si
/// no entró.
#[server]
pub async fn buyer_orders() -> Result<Option<Vec<Order>>, ServerFnError> {
    let Some((tenant, buyer)) = current_buyer().await? else {
        return Ok(None);
    };
    let orders = db::orders::list_for_email(&pool()?, tenant.id, &buyer.email)
        .await
        .map_err(internal)?;
    Ok(Some(orders.into_iter().map(for_buyer).collect()))
}

/// Un pedido del comprador. `None` si no entró o ese pedido no es suyo.
#[server]
pub async fn buyer_order(id: String) -> Result<Option<Order>, ServerFnError> {
    let Some((tenant, buyer)) = current_buyer().await? else {
        return Ok(None);
    };
    let Ok(id) = uuid::Uuid::parse_str(&id) else {
        return Ok(None);
    };
    let order = db::orders::get_for_email(&pool()?, tenant.id, &buyer.email, id)
        .await
        .map_err(internal)?;
    Ok(order.map(for_buyer))
}

/// Guarda los datos del comprador.
#[server(input = Json)]
pub async fn buyer_save_profile(profile: BuyerProfile) -> Result<(), ServerFnError> {
    let Some((_, buyer)) = current_buyer().await? else {
        return Err(ServerFnError::new("Tu sesión venció. Volvé a entrar."));
    };
    let clean = |s: &str, max: usize| -> String {
        s.chars()
            .filter(|c| !c.is_control())
            .collect::<String>()
            .trim()
            .chars()
            .take(max)
            .collect()
    };
    db::buyers::save_profile(
        &pool()?,
        buyer.id,
        &clean(&profile.name, 80),
        &clean(&profile.phone, 30),
        &clean(&profile.address, 160),
        &clean(&profile.postal_code, 12),
        &clean(&profile.city, 80),
    )
    .await
    .map_err(internal)
}

/// Pone o quita la foto de perfil. La foto llega ya reducida desde el
/// navegador, como data URL de una imagen JPEG, PNG o WebP.
#[server]
pub async fn buyer_save_avatar(avatar: Option<String>) -> Result<(), ServerFnError> {
    let Some((_, buyer)) = current_buyer().await? else {
        return Err(ServerFnError::new("Tu sesión venció. Volvé a entrar."));
    };
    if let Some(a) = &avatar {
        if !domain::valid_avatar(a, db::buyers::AVATAR_MAX_BYTES) {
            return Err(ServerFnError::new(
                "Esa foto no se puede usar. Probá con una imagen JPG, PNG o WebP más chica.",
            ));
        }
    }
    db::buyers::set_avatar(&pool()?, buyer.id, avatar.as_deref())
        .await
        .map_err(internal)
}

/// Borra la cuenta del comprador en esta tienda (los pedidos quedan en la
/// tienda) y cierra la sesión.
#[server]
pub async fn buyer_delete_account() -> Result<(), ServerFnError> {
    let Some((_, buyer)) = current_buyer().await? else {
        return Ok(());
    };
    db::buyers::delete_account(&pool()?, buyer.id)
        .await
        .map_err(internal)?;
    clear_cookie();
    Ok(())
}

/// Los productos que el comprador guardó en favoritos en esta tienda (solo
/// sus ids, para pintar los corazones). Vacío si no entró.
#[server]
pub async fn buyer_favorite_ids() -> Result<Vec<String>, ServerFnError> {
    let Some((tenant, buyer)) = current_buyer().await? else {
        return Ok(Vec::new());
    };
    let ids = db::buyers::favorite_ids(&pool()?, tenant.id, buyer.id)
        .await
        .map_err(internal)?;
    Ok(ids.into_iter().map(|i| i.to_string()).collect())
}

/// Marca o desmarca un favorito del comprador. `false` si no entró o no se
/// pudo guardar (producto de otra tienda, o ya llegó al máximo).
#[server]
pub async fn buyer_set_favorite(product_id: String, on: bool) -> Result<bool, ServerFnError> {
    let Some((tenant, buyer)) = current_buyer().await? else {
        return Ok(false);
    };
    let Ok(product) = product_id.trim().parse::<uuid::Uuid>() else {
        return Ok(false);
    };
    db::buyers::set_favorite(&pool()?, tenant.id, buyer.id, product, on)
        .await
        .map_err(internal)
}

/// Los favoritos del comprador, el último guardado primero. `None` si no entró.
#[server]
pub async fn buyer_favorites() -> Result<Option<Vec<domain::Product>>, ServerFnError> {
    let Some((tenant, buyer)) = current_buyer().await? else {
        return Ok(None);
    };
    let list = db::products::list_favorites(&pool()?, tenant.id, buyer.id)
        .await
        .map_err(internal)?;
    Ok(Some(list))
}
