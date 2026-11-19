//! El botón de la cuenta del comprador en el menú de la tienda.
//!
//! El cajón y las páginas de la cuenta viven en `web` (necesitan al servidor);
//! acá solo está el estado que comparten ([`Account`]) y el botón, que abre
//! el cajón. Sin contexto (en el editor) el botón se dibuja igual pero no
//! hace nada, como el del carrito.

use domain::BuyerMe;
use leptos::prelude::*;
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

/// El estado de la cuenta del comprador, en el contexto de la página.
#[derive(Clone, Copy)]
pub struct Account {
    /// El cajón de la cuenta está abierto.
    pub open: RwSignal<bool>,
    pub state: RwSignal<AccountState>,
}

impl Account {
    pub fn new() -> Self {
        Self {
            open: RwSignal::new(false),
            state: RwSignal::new(AccountState::Loading),
        }
    }
}

impl Default for Account {
    fn default() -> Self {
        Self::new()
    }
}

/// El avatar: la foto si hay, o las iniciales sobre el color de la tienda.
#[component]
pub fn BuyerAvatar(
    me: BuyerMe,
    /// Clases de tamaño (`h-9 w-9 text-sm`).
    #[prop(into, default = "h-9 w-9 text-sm".to_string())]
    size: String,
) -> impl IntoView {
    let class = format!(
        "mk-avatar-buyer inline-grid shrink-0 place-items-center overflow-hidden rounded-full bg-primary font-bold text-primary-content {size}"
    );
    match me.avatar.clone() {
        Some(src) => view! {
            <span class=class><img class="h-full w-full object-cover" src=src alt="" draggable="false"/></span>
        }
        .into_any(),
        None => view! { <span class=class aria-hidden="true">{me.initials()}</span> }.into_any(),
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
