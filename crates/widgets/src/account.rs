//! El botón de la cuenta del comprador en el menú de la tienda.
//!
//! El cajón y las páginas de la cuenta viven en `web` (necesitan al servidor);
//! acá solo está el estado que comparten ([`Account`]) y el botón, que abre
//! el cajón. Sin contexto (en el editor) el botón se dibuja igual pero no
//! hace nada, como el del carrito.

use domain::BuyerMe;
use leptos::prelude::*;
use std::collections::HashSet;
use ui::Icon;

/// Qué se sabe de quien mira la tienda.
#[derive(Clone, Debug, PartialEq)]
pub enum AccountState {
    /// Todavía no respondió el servidor.
    Loading,
    /// No entró a su cuenta.
    Out,
    /// Entró.
    In(BuyerMe),
}

/// Alguien tocó el corazón de un producto. El cajón de la cuenta (en `web`,
/// que habla con el servidor) lo atiende: guarda el favorito o, si no entró,
/// le ofrece entrar.
#[derive(Clone, Debug, PartialEq)]
pub struct FavRequest {
    pub product_id: String,
    pub name: String,
    /// Dónde está el corazón en la pantalla (borde derecho y borde de abajo),
    /// para poner el globito pegado a él.
    pub right: f64,
    pub bottom: f64,
}

/// El estado de la cuenta del comprador, en el contexto de la página.
#[derive(Clone, Copy)]
pub struct Account {
    /// El cajón de la cuenta está abierto.
    pub open: RwSignal<bool>,
    pub state: RwSignal<AccountState>,
    /// Ids de los productos que guardó en favoritos (de esta tienda).
    pub favs: RwSignal<HashSet<String>>,
    /// El último corazón que tocó (lo atiende `web`).
    pub fav_req: RwSignal<Option<FavRequest>>,
}

impl Account {
    pub fn new() -> Self {
        Self {
            open: RwSignal::new(false),
            state: RwSignal::new(AccountState::Loading),
            favs: RwSignal::new(HashSet::new()),
            fav_req: RwSignal::new(None),
        }
    }
}

impl Default for Account {
    fn default() -> Self {
        Self::new()
    }
}

/// El avatar: la foto si hay, o la inicial sobre el color de la tienda.
#[component]
pub fn BuyerAvatar(
    me: BuyerMe,
    /// Clases de tamaño (`h-9 w-9 text-sm`).
    #[prop(into, default = "h-9 w-9 text-sm".to_string())]
    size: String,
) -> impl IntoView {
    let class = format!(
        "inline-grid shrink-0 place-items-center overflow-hidden rounded-full bg-primary font-bold text-primary-content {size}"
    );
    match me.avatar.clone() {
        Some(src) => view! {
            <span class=class><img class="h-full w-full object-cover" src=src alt="" draggable="false"/></span>
        }
        .into_any(),
        None => view! { <span class=class aria-hidden="true">{me.initial()}</span> }.into_any(),
    }
}

/// El botón de la cuenta, junto al del carrito: "Ingresar" si no entró y su
/// avatar si entró. Abre el cajón de la cuenta.
#[component]
pub fn AccountNavButton() -> impl IntoView {
    let Some(account) = use_context::<Account>() else {
        return view! {
            <span class="btn btn-ghost btn-square shrink-0" aria-hidden="true">
                <Icon name="user" class="h-6 w-6"/>
            </span>
        }
        .into_any();
    };
    view! {
        <button
            type="button"
            class=move || match account.state.get() {
                AccountState::In(_) => "btn btn-ghost btn-circle mk-account-nav shrink-0 p-0",
                // Mientras carga se ve como "Ingresar": casi todos los
                // visitantes no tienen sesión, y así el botón no cambia de
                // tamaño al hidratar.
                AccountState::Out | AccountState::Loading => {
                    "btn btn-ghost mk-account-nav shrink-0 max-sm:btn-square"
                }
            }
            aria-haspopup="dialog"
            aria-label=move || match account.state.get() {
                AccountState::In(me) => format!("Mi cuenta, {}", me.display_name()),
                _ => "Ingresar a mi cuenta".to_string(),
            }
            on:click=move |_| account.open.set(true)
        >
            {move || match account.state.get() {
                AccountState::In(me) => view! { <BuyerAvatar me/> }.into_any(),
                AccountState::Out | AccountState::Loading => view! {
                    <Icon name="user" class="h-6 w-6"/>
                    <span class="max-sm:hidden">"Ingresar"</span>
                }
                .into_any(),
            }}
        </button>
    }
    .into_any()
}

/// "Mi cuenta" en el pie de la tienda: una segunda puerta a la cuenta, para
/// las tiendas cuyo menú no la muestra o ya no está a la vista.
#[component]
pub fn AccountFooterLink() -> impl IntoView {
    let Some(account) = use_context::<Account>() else {
        return ().into_any();
    };
    view! {
        <button type="button" class="link link-hover" aria-haspopup="dialog"
            on:click=move |_| account.open.set(true)>
            {move || match account.state.get() {
                AccountState::In(_) => "Mi cuenta",
                _ => "Ingresar a mi cuenta",
            }}
        </button>
    }
    .into_any()
}

/// El icono del corazón (contorno o relleno).
#[component]
pub fn HeartIcon(#[prop(into)] filled: Signal<bool>, #[prop(into, default = "size-5".to_string())] class: String) -> impl IntoView {
    view! {
        <svg class=class viewBox="0 0 24 24" fill=move || if filled.get() { "currentColor" } else { "none" }
            stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
            <path d="M19 14c1.49-1.46 3-3.21 3-5.5A5.5 5.5 0 0 0 16.5 3c-1.76 0-3 .5-4.5 2-1.5-1.5-2.74-2-4.5-2A5.5 5.5 0 0 0 2 8.5c0 2.3 1.5 4.05 3 5.5l7 7Z"/>
        </svg>
    }
}

/// El corazón para guardar un producto en favoritos. Va arriba a la derecha
/// de la foto de la tarjeta (dentro de una caja `relative`) y por encima del
/// botón que cubre la tarjeta. En el editor se ve pero no hace nada.
#[component]
pub fn FavoriteHeart(product_id: String, name: String) -> impl IntoView {
    let Some(account) = use_context::<Account>() else {
        return view! {
            <span class="mk-fav absolute right-2 top-2 z-10" aria-hidden="true"><HeartIcon filled=false/></span>
        }
        .into_any();
    };
    let id = StoredValue::new(product_id);
    let name = StoredValue::new(name);
    let on = Memo::new(move |_| id.with_value(|id| account.favs.with(|f| f.contains(id))));
    view! {
        <button type="button" class="mk-fav absolute right-2 top-2 z-10"
            class:is-on=move || on.get()
            aria-pressed=move || on.get().to_string()
            aria-label=move || name.with_value(|n| if on.get() { format!("Quitar {n} de favoritos") } else { format!("Agregar {n} a favoritos") })
            on:click=move |ev| {
                use wasm_bindgen::JsCast;
                ev.stop_propagation();
                let rect = ev
                    .current_target()
                    .and_then(|t| t.dyn_into::<web_sys::Element>().ok())
                    .map(|el| el.get_bounding_client_rect());
                account.fav_req.set(Some(FavRequest {
                    product_id: id.get_value(),
                    name: name.get_value(),
                    right: rect.as_ref().map_or(0.0, |r| r.right()),
                    bottom: rect.as_ref().map_or(0.0, |r| r.bottom()),
                }));
            }>
            <HeartIcon filled=on/>
        </button>
    }
    .into_any()
}
