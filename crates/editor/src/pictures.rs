//! Opciones que se eligen viendo un dibujo de cómo quedan, en vez de una
//! lista de nombres: el estilo de «Foto del producto» y cómo se abre la ficha.
//! Son radios nativos (las flechas del teclado pasan de una a otra); el
//! dibujo es decorativo y el nombre va debajo.

use domain::Product;
use leptos::prelude::*;
use widgets::sample_product;

use crate::Ui;

/// Qué dibujos usa un grupo de opciones.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Pictures {
    /// Estilos de «Foto del producto» (`VARIANT_PRODUCT_IMAGE`).
    ProductPhoto,
    /// Cómo se abre la ficha (`MODAL_OPEN_OPTIONS`).
    ModalOpen,
}

impl Pictures {
    /// Las opciones de un campo de lista que se eligen con dibujos.
    pub(crate) fn for_options(options: &'static [(&'static str, &'static str)]) -> Option<Self> {
        std::ptr::eq(options, domain::VARIANT_PRODUCT_IMAGE).then_some(Pictures::ProductPhoto)
    }

    /// Nombre corto que va debajo del dibujo.
    fn name(self, value: &str, label: &'static str) -> &'static str {
        match (self, value) {
            (Pictures::ProductPhoto, "a") => "Clásico",
            (Pictures::ProductPhoto, "b") => "Galería grande",
            (Pictures::ProductPhoto, "c") => "Mosaico",
            (Pictures::ProductPhoto, "d") => "Miniaturas al costado",
            (Pictures::ProductPhoto, "e") => "Visor con flechas",
            (Pictures::ModalOpen, "") => "En el centro",
            (Pictures::ModalOpen, "drawer") => "Cajón al costado",
            (Pictures::ModalOpen, "sheet") => "Desde abajo",
            (Pictures::ModalOpen, "auto") => "Cajón en computadora, hoja en celular",
            _ => label,
        }
    }
}

/// El dibujo de una opción (decorativo).
fn drawing(kind: Pictures, value: &str) -> AnyView {
    let ph = || view! { <i class="mk-pic-ph"></i> };
    match (kind, value) {
        (Pictures::ProductPhoto, "a") => view! {
            <span class="mk-pic-d grid-cols-4 grid-rows-[1fr_18%]">
                <i class="mk-pic-ph col-span-4"></i>{ph()}{ph()}{ph()}{ph()}
            </span>
        }
        .into_any(),
        (Pictures::ProductPhoto, "b") => view! {
            <span class="mk-pic-d grid-cols-2 grid-rows-2">{ph()}{ph()}{ph()}{ph()}</span>
        }
        .into_any(),
        (Pictures::ProductPhoto, "c") => view! {
            <span class="mk-pic-d grid-cols-[2fr_1fr] grid-rows-2">
                <i class="mk-pic-ph row-span-2"></i>{ph()}{ph()}
            </span>
        }
        .into_any(),
        (Pictures::ProductPhoto, "d") => view! {
            <span class="mk-pic-d grid-cols-[18%_1fr] grid-rows-4">
                <i class="mk-pic-ph col-start-2 row-span-4 row-start-1"></i>{ph()}{ph()}{ph()}{ph()}
            </span>
        }
        .into_any(),
        (Pictures::ProductPhoto, _) => view! {
            <span class="mk-pic-d relative grid-cols-1">
                {ph()}
                <b class="mk-pic-arrow left-1">"‹"</b>
                <b class="mk-pic-arrow right-1">"›"</b>
            </span>
        }
        .into_any(),
        (Pictures::ModalOpen, "drawer") => view! {
            <span class="mk-pic-screen"><i class="mk-pic-panel inset-y-0 right-0 w-1/2"></i></span>
        }
        .into_any(),
        (Pictures::ModalOpen, "sheet") => view! {
            <span class="mk-pic-screen"><i class="mk-pic-panel inset-x-0 bottom-0 h-2/3 rounded-t-md"></i></span>
        }
        .into_any(),
        (Pictures::ModalOpen, "auto") => view! {
            <span class="flex items-end gap-1">
                <span class="mk-pic-screen" style="width:70%"><i class="mk-pic-panel inset-y-0 right-0 w-1/2"></i></span>
                <span class="mk-pic-screen" style="aspect-ratio:1/2;width:26%"><i class="mk-pic-panel inset-x-0 bottom-0 h-2/3 rounded-t-sm"></i></span>
            </span>
        }
        .into_any(),
        (Pictures::ModalOpen, _) => view! {
            <span class="mk-pic-screen"><i class="mk-pic-panel inset-x-[22%] inset-y-[16%] rounded-sm"></i></span>
        }
        .into_any(),
    }
}

/// Grupo de opciones con dibujo. `value` es la elegida; `pick` la cambia.
#[component]
pub(crate) fn PicturePicker(
    kind: Pictures,
    uid: String,
    legend: &'static str,
    options: &'static [(&'static str, &'static str)],
    #[prop(into)] value: Signal<String>,
    pick: Callback<String>,
    #[prop(optional)] help: Option<&'static str>,
    #[prop(optional)] products: Option<StoredValue<Vec<Product>>>,
) -> impl IntoView {
    let name = uid.clone();
    view! {
        <fieldset class="flex flex-col gap-1.5">
            <legend class="mb-1.5 text-sm font-medium">{legend}</legend>
            {products.filter(|_| kind == Pictures::ProductPhoto).map(|products| view! { <OnePhotoNotice products/> })}
            <div class="mk-pics">
                {options
                    .iter()
                    .map(|(v, l)| {
                        let v: &'static str = v;
                        let checked = move || value.get() == v;
                        view! {
                            <label class="mk-pic" class:is-on=checked>
                                <input
                                    type="radio"
                                    class="sr-only"
                                    name=name.clone()
                                    value=v
                                    prop:checked=checked
                                    on:change=move |_| pick.run(v.to_string())
                                />
                                <span class="mk-pic-art" aria-hidden="true">{drawing(kind, v)}</span>
                                <span class="mk-pic-name">{kind.name(v, l)}</span>
                            </label>
                        }
                    })
                    .collect_view()}
            </div>
            {help.map(|h| view! { <p class="text-xs text-base-content/75">{h}</p> })}
        </fieldset>
    }
}

/// Si la ficha se está mostrando con un producto de una sola foto, todos los
/// estilos se ven iguales: ofrece verla con uno que tenga varias.
#[component]
fn OnePhotoNotice(products: StoredValue<Vec<Product>>) -> impl IntoView {
    let ui = expect_context::<Ui>();
    let offer = Memo::new(move |_| {
        let chosen = ui.modal_product.get();
        products.with_value(|ps| {
            let current = chosen
                .and_then(|id| ps.iter().find(|p| p.id == id).cloned())
                .or_else(|| ps.first().cloned())
                .unwrap_or_else(sample_product);
            if current.all_photos().len() > 1 {
                return None;
            }
            let other = ps
                .iter()
                .filter(|p| p.visible)
                .max_by_key(|p| p.all_photos().len())
                .filter(|p| p.all_photos().len() > 1)?;
            Some((
                current.name,
                other.id.clone(),
                other.name.clone(),
                other.all_photos().len(),
            ))
        })
    });
    move || {
        offer.get().map(|(current, id, name, n)| {
            view! {
                <p class="rounded-box border border-primary/30 bg-primary/5 px-3 py-2 text-xs">
                    {format!("Estás viendo «{current}», que tiene una sola foto. ")}
                    <button type="button" class="link link-primary"
                        on:click=move |_| ui.modal_product.set(Some(id.clone()))>
                        {format!("Ver con «{name}» ({n} fotos)")}
                    </button>
                </p>
            }
        })
    }
}
