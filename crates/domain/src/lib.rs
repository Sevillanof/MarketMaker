//! Tipos del panel y su validación. Sin Leptos y sin IO: compila a nativo y a WASM.

use serde::{Deserialize, Serialize};

/// Producto de ejemplo del tenant, tal como lo usan los widgets.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Product {
    pub id: String,
    pub name: String,
    pub price_cents: i64,
    pub image_url: String,
}
