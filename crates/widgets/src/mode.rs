//! Modo claro u oscuro de la tienda publicada, a elección de cada visitante.
//!
//! El comerciante prepara las dos variantes (ver `domain::StoreStyle`) y la
//! tienda arranca en la del tema. El visitante puede cambiar desde el menú
//! (o desde un botón suelto si la página no tiene menú); su elección se
//! recuerda en su navegador, separada por tienda (por dirección), y no toca
//! el documento ni el borrador del comerciante.
//!
//! Un script chico va como primer hijo de `.mk-store`: aplica la elección
//! antes del primer pintado (así no parpadea) y escucha los botones
//! `[data-mk-mode]`. La variante opuesta es una regla CSS con
//! `data-mode="alt"` (ver `StoreStyle::alt_css`).

use leptos::prelude::*;

/// Script y regla de la variante opuesta. `native_dark`: el tema original es
/// oscuro. Va como primer hijo de `.mk-store`.
#[component]
pub fn StoreMode(native_dark: bool, alt_css: String) -> impl IntoView {
    let script = SCRIPT.replace("__ND__", if native_dark { "true" } else { "false" });
    view! {
        <style inner_html=alt_css></style>
        <script inner_html=script></script>
    }
}

/// Botón del visitante para pasar de claro a oscuro. Su nombre, su estado y
/// su ícono los pone el script de [`StoreMode`]; en el editor no hace nada.
#[component]
pub fn ModeButton(
    #[prop(optional)] editing: bool,
    /// Con el nombre escrito al lado (para el menú del celular).
    #[prop(optional)]
    labeled: bool,
    #[prop(optional, into)] class: String,
) -> impl IntoView {
    view! {
        <button
            type="button"
            class=format!("mk-modebtn {class}")
            data-mk-mode
            disabled=editing
            aria-pressed="false"
            aria-label="Cambiar a modo oscuro"
            title="Cambiar a modo oscuro"
        >
            <span aria-hidden="true" data-mk-mode-icon>"☾"</span>
            {labeled.then(|| view! { <span data-mk-mode-text>"Modo oscuro"</span> })}
        </button>
    }
}

/// Botón suelto para las páginas sin menú: arriba a la derecha, sobre el
/// contenido. No crea ni cambia ningún widget del documento.
#[component]
pub fn ModeFloat() -> impl IntoView {
    view! { <div class="mk-modefloat"><ModeButton/></div> }
}

const SCRIPT: &str = r#"(function(){var s=document.currentScript&&document.currentScript.parentElement;if(!s)return;var nd=__ND__,k='mk-mode:'+location.hostname;
function get(){try{return localStorage.getItem(k)}catch(e){return null}}
function dark(){return s.dataset.mode==='alt'?!nd:nd}
function ui(){var d=dark();document.querySelectorAll('[data-mk-mode]').forEach(function(b){var t=d?'Cambiar a modo claro':'Cambiar a modo oscuro';b.setAttribute('aria-pressed',String(d));b.setAttribute('aria-label',t);b.title=t;var i=b.querySelector('[data-mk-mode-icon]');if(i)i.textContent=d?'☀':'☾';var x=b.querySelector('[data-mk-mode-text]');if(x)x.textContent=d?'Modo claro':'Modo oscuro'})}
function note(){var n=document.createElement('div');n.className='mk-modenote';n.setAttribute('role','status');n.textContent='No pudimos recordar tu elección en este navegador.';s.appendChild(n);setTimeout(function(){n.remove()},4000)}
var c=get();s.dataset.mode=(c==='dark'||c==='light')&&((c==='dark')!==nd)?'alt':'orig';
if(!window.__mkMode){window.__mkMode=1;document.addEventListener('click',function(e){var b=e.target.closest&&e.target.closest('[data-mk-mode]');if(!b||b.disabled)return;var d=!dark();s.dataset.mode=d===nd?'orig':'alt';ui();try{localStorage.setItem(k,d?'dark':'light')}catch(x){note()}})}
document.addEventListener('DOMContentLoaded',ui);ui()})();"#;
