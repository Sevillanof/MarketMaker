//! «Colores y talles» en el formulario de producto (diseño B aprobado):
//! una tarjeta por color, con su foto y el stock de cada talle adentro.
//! Los talles se cargan una vez y valen para todos los colores.
//!
//! Cada color tiene un número propio (`uid`) mientras se edita, así al
//! cambiarle el nombre no pierde su stock ni el foco del campo.

use domain::{VariantColor, VariantStock, Variants};
use leptos::prelude::*;
use std::collections::BTreeMap;

/// Colores de muestra para el punto de color de un color nuevo.
const SWATCHES: [&str; 8] = [
    "#1f2937", "#2563eb", "#dc2626", "#16a34a", "#f59e0b", "#db2777", "#7c3aed", "#f5f5f4",
];

#[derive(Clone, Copy)]
pub struct ColorRow {
    uid: u32,
    name: RwSignal<String>,
    hex: RwSignal<String>,
    photo: RwSignal<String>,
}

/// Lo que se está cargando en «Colores y talles».
#[derive(Clone, Copy)]
pub struct VariantsForm {
    /// Se abrió la sección (o el producto ya tenía variantes).
    pub on: RwSignal<bool>,
    colors: RwSignal<Vec<ColorRow>>,
    sizes: RwSignal<Vec<String>>,
    /// Unidades por (color, talle), como se escribieron. El color 0 es «sin
    /// color» (productos que tienen solo talles).
    stock: RwSignal<BTreeMap<(u32, String), String>>,
    next: StoredValue<u32>,
}

impl VariantsForm {
    pub fn new(v: &Variants) -> Self {
        let colors: Vec<ColorRow> = v
            .colors
            .iter()
            .enumerate()
            .map(|(i, c)| ColorRow {
                uid: i as u32 + 1,
                name: RwSignal::new(c.name.clone()),
                hex: RwSignal::new(c.hex.clone()),
                photo: RwSignal::new(c.photo.clone()),
            })
            .collect();
        let uid_of = |name: &str| {
            v.colors
                .iter()
                .position(|c| c.name == name)
                .map_or(0, |i| i as u32 + 1)
        };
        let stock = v
            .stock
            .iter()
            .map(|s| ((uid_of(&s.color), s.size.clone()), s.qty.to_string()))
            .collect();
        Self {
            on: RwSignal::new(!v.is_empty()),
            next: StoredValue::new(colors.len() as u32 + 1),
            colors: RwSignal::new(colors),
            sizes: RwSignal::new(v.sizes.clone()),
            stock: RwSignal::new(stock),
        }
    }

    /// Para el "hay cambios sin guardar".
    pub fn track(&self) {
        self.on.track();
        self.sizes.track();
        self.stock.track();
        self.colors.with(|cs| {
            for c in cs {
                c.name.track();
                c.hex.track();
                c.photo.track();
            }
        });
    }

    /// Hay al menos un color o un talle.
    pub fn active(&self) -> bool {
        self.colors.with(|c| !c.is_empty()) || self.sizes.with(|s| !s.is_empty())
    }

    /// Las variantes tal como quedarían, o el problema con alguna cantidad.
    pub fn variants(&self) -> Result<Variants, String> {
        let colors = self.colors.get();
        let sizes = self.sizes.get();
        let stock = self.stock.get();
        let rows: Vec<(u32, String)> = if colors.is_empty() {
            vec![(0, String::new())]
        } else {
            colors.iter().map(|c| (c.uid, c.name.get())).collect()
        };
        let size_list: Vec<String> = if sizes.is_empty() { vec![String::new()] } else { sizes.clone() };
        let mut out = Vec::new();
        for (uid, color) in &rows {
            for size in &size_list {
                let text = stock.get(&(*uid, size.clone())).cloned().unwrap_or_default();
                let text = text.trim().replace('.', "");
                let qty = if text.is_empty() {
                    0
                } else {
                    text.parse::<u32>().map_err(|_| {
                        "La cantidad de cada color y talle tiene que ser un número entero, como 5."
                            .to_string()
                    })?
                };
                out.push(VariantStock {
                    color: color.trim().to_string(),
                    size: size.clone(),
                    qty,
                });
            }
        }
        if colors.is_empty() && sizes.is_empty() {
            return Ok(Variants::default());
        }
        Ok(Variants {
            colors: colors
                .iter()
                .map(|c| VariantColor {
                    name: c.name.get(),
                    hex: c.hex.get(),
                    photo: c.photo.get(),
                })
                .collect(),
            sizes,
            stock: out,
        })
    }

    fn add_color(&self, photos: &[String]) {
        let uid = self.next.get_value();
        self.next.set_value(uid + 1);
        let n = self.colors.with(Vec::len);
        // Cada color nuevo arranca con la foto siguiente, que suele ser la suya.
        let photo = photos.get(n).cloned().unwrap_or_default();
        let had_colors = n > 0;
        self.colors.update(|cs| {
            cs.push(ColorRow {
                uid,
                name: RwSignal::new(String::new()),
                hex: RwSignal::new(SWATCHES[n % SWATCHES.len()].to_string()),
                photo: RwSignal::new(photo),
            })
        });
        // El primer color se queda con el stock que ya tenían los talles.
        if !had_colors {
            self.stock.update(|m| {
                let moved: Vec<_> = m
                    .iter()
                    .filter(|((u, _), _)| *u == 0)
                    .map(|((_, s), q)| ((uid, s.clone()), q.clone()))
                    .collect();
                m.retain(|(u, _), _| *u != 0);
                m.extend(moved);
            });
        }
    }

    fn remove_color(&self, uid: u32) {
        self.colors.update(|cs| cs.retain(|c| c.uid != uid));
        self.stock.update(|m| m.retain(|(u, _), _| *u != uid));
    }

    fn add_sizes(&self, text: &str) {
        self.sizes.update(|ss| {
            for s in text.split(',') {
                let s = s.split_whitespace().collect::<Vec<_>>().join(" ");
                if !s.is_empty()
                    && ss.len() < domain::VARIANT_SIZES_MAX
                    && !ss.iter().any(|x| x.eq_ignore_ascii_case(&s))
                {
                    ss.push(s);
                }
            }
        });
    }

    /// Total de unidades cargadas (para mostrar en «Stock»).
    pub fn total(&self) -> u32 {
        self.variants().map(|v| v.total()).unwrap_or_default()
    }
}

/// Un campo de cantidad de un color y un talle.
#[component]
fn QtyInput(form: VariantsForm, uid: u32, size: String, #[prop(into)] label: Signal<String>) -> impl IntoView {
    let key = (uid, size);
    let k2 = key.clone();
    let k3 = key.clone();
    let out = move || {
        form.stock.with(|m| {
            m.get(&k3)
                .map(|t| t.trim().is_empty() || t.trim() == "0")
                .unwrap_or(true)
        })
    };
    view! {
        <input class="input input-sm mk-vqty w-16 text-center" class:mk-vqty-out=out
            inputmode="numeric" placeholder="0" aria-label=move || label.get()
            prop:value=move || form.stock.with(|m| m.get(&key).cloned().unwrap_or_default())
            on:input=move |ev| {
                let v = event_target_value(&ev);
                form.stock.update(|m| { m.insert(k2.clone(), v); });
            }/>
    }
}

/// La sección «Colores y talles».
#[component]
pub fn VariantsField(form: VariantsForm, photos: RwSignal<Vec<String>>) -> impl IntoView {
    let size_text = RwSignal::new(String::new());
    let commit_sizes = move || {
        let t = size_text.get_untracked();
        if !t.trim().is_empty() {
            form.add_sizes(&t);
            size_text.set(String::new());
        }
    };
    let cards = move || {
        let sizes = form.sizes.get();
        let colors = form.colors.get();
        // Sin colores pero con talles: una sola fila de stock por talle.
        let rows: Vec<Option<ColorRow>> = if colors.is_empty() {
            if sizes.is_empty() { Vec::new() } else { vec![None] }
        } else {
            colors.into_iter().map(Some).collect()
        };
        rows.into_iter()
            .map(|row| {
                let uid = row.map_or(0, |r| r.uid);
                // El nombre se lee en un `Signal` aparte: escribirlo no vuelve a
                // dibujar la tarjeta (el campo no pierde el foco).
                let color_name = move || row.map(|r| r.name.get()).filter(|n| !n.trim().is_empty()).unwrap_or_else(|| "este color".into());
                let qty_cells = if sizes.is_empty() {
                    view! {
                        <label class="flex items-center gap-2 text-sm">"Unidades"
                            <QtyInput form uid size=String::new() label=Signal::derive(move || format!("Unidades de {}", color_name()))/>
                        </label>
                    }
                    .into_any()
                } else {
                    sizes
                        .iter()
                        .map(|s| {
                            let s2 = s.clone();
                            let label = Signal::derive(move || format!("Unidades talle {s2} de {}", color_name()));
                            view! {
                                <label class="flex flex-col items-center gap-1 text-xs font-medium">
                                    {s.clone()}
                                    <QtyInput form uid size=s.clone() label/>
                                </label>
                            }
                        })
                        .collect_view()
                        .into_any()
                };
                match row {
                    None => view! {
                        <div class="rounded-box border border-base-300 p-3">
                            <p class="mb-2 text-sm font-medium">"Unidades por talle"</p>
                            <div class="flex flex-wrap gap-2">{qty_cells}</div>
                        </div>
                    }
                    .into_any(),
                    Some(r) => {
                        let photo_now = move || r.photo.get();
                        let pick = move |url: String| r.photo.set(url);
                        view! {
                            <div class="grid grid-cols-[4.5rem_1fr] gap-3 rounded-box border border-base-300 p-3">
                                <div class="size-18 overflow-hidden rounded-field bg-base-200">
                                    {move || {
                                        let url = photo_now();
                                        let url = if url.is_empty() { photos.with(|p| p.first().cloned().unwrap_or_default()) } else { url };
                                        (!url.is_empty()).then(|| view! { <ui::Image src=url alt=""/> })
                                    }}
                                </div>
                                <div class="flex min-w-0 flex-col gap-2">
                                    <div class="flex flex-wrap items-center gap-2">
                                        <input type="color" class="mk-vswatch" aria-label="Color del punto"
                                            prop:value=move || { let h = r.hex.get(); if h.is_empty() { "#999999".to_string() } else { h } }
                                            on:input=move |ev| r.hex.set(event_target_value(&ev))/>
                                        <input class="input input-sm w-44 font-semibold" placeholder="Nombre, ej. Azul"
                                            maxlength=domain::VARIANT_NAME_MAX aria-label="Nombre del color"
                                            prop:value=move || r.name.get()
                                            on:input=move |ev| r.name.set(event_target_value(&ev))/>
                                        <button type="button" class="btn btn-ghost btn-xs ml-auto text-base-content/70"
                                            on:click=move |_| form.remove_color(r.uid)>"✕ Quitar color"</button>
                                    </div>
                                    {move || photos.with(|ps| (ps.len() > 1).then(|| {
                                        let ps = ps.clone();
                                        view! {
                                            <div class="flex flex-wrap items-center gap-1.5" role="group" aria-label="Foto de este color">
                                                <span class="text-xs text-base-content/60">"Foto:"</span>
                                                {ps.into_iter().enumerate().map(|(i, url)| {
                                                    let u = url.clone();
                                                    let u2 = url.clone();
                                                    let on = move || { let p = r.photo.get(); p == u || (p.is_empty() && i == 0) };
                                                    view! {
                                                        <button type="button" class="mk-vphoto" aria-pressed=move || on().to_string()
                                                            aria-label=format!("Foto {}", i + 1)
                                                            on:click=move |_| pick(u2.clone())>
                                                            <ui::Image src=url alt=""/>
                                                        </button>
                                                    }
                                                }).collect_view()}
                                            </div>
                                        }
                                    }))}
                                    <div class="flex flex-wrap gap-2">{qty_cells}</div>
                                </div>
                            </div>
                        }
                        .into_any()
                    }
                }
            })
            .collect_view()
    };
    view! {
        <fieldset class="fieldset">
            <legend class="fieldset-legend">"Colores y talles " <span class="font-normal text-base-content/60">"(opcional)"</span></legend>
            {move || if !form.on.get() {
                view! {
                    <div class="flex flex-wrap items-center gap-3">
                        <button type="button" class="btn btn-sm" on:click=move |_| form.on.set(true)>"+ Agregar colores o talles"</button>
                        <span class="label whitespace-normal">"Para vender un mismo producto en varios colores o talles, cada uno con su stock."</span>
                    </div>
                }.into_any()
            } else {
                view! {
                    <div class="flex flex-col gap-3">
                        <div class="flex flex-col gap-1">
                            <span class="text-sm font-medium">"Talles " <span class="font-normal text-base-content/60">"(valen para todos los colores)"</span></span>
                            <div class="flex min-h-10 flex-wrap items-center gap-1.5 rounded-field border border-base-300 px-2 py-1.5">
                                {move || form.sizes.get().into_iter().map(|s| {
                                    let s2 = s.clone();
                                    view! {
                                        <span class="mk-pill-soft inline-flex items-center gap-1 rounded-full py-0.5 pr-1 pl-2.5 text-sm">
                                            {s.clone()}
                                            <button type="button" class="btn btn-ghost btn-circle btn-xs" aria-label=format!("Quitar el talle {s}")
                                                on:click=move |_| {
                                                    let s = s2.clone();
                                                    form.sizes.update(|ss| ss.retain(|x| *x != s));
                                                    form.stock.update(|m| m.retain(|(_, z), _| *z != s));
                                                }>"✕"</button>
                                        </span>
                                    }
                                }).collect_view()}
                                <input class="min-w-32 flex-1 bg-transparent text-sm outline-none" placeholder="Escribí un talle y apretá Enter (ej. S, M, L o 38, 39)"
                                    aria-label="Agregar talles"
                                    prop:value=move || size_text.get()
                                    on:input=move |ev| {
                                        let v = event_target_value(&ev);
                                        if v.contains(',') { size_text.set(v); commit_sizes(); } else { size_text.set(v); }
                                    }
                                    on:keydown=move |ev| {
                                        if ev.key() == "Enter" { ev.prevent_default(); commit_sizes(); }
                                        else if ev.key() == "Backspace" && size_text.with_untracked(String::is_empty) {
                                            form.sizes.update(|ss| { ss.pop(); });
                                        }
                                    }
                                    on:blur=move |_| commit_sizes()/>
                            </div>
                        </div>
                        {cards}
                        <div class="flex flex-wrap items-center gap-3">
                            <button type="button" class="btn btn-sm" on:click=move |_| { let ps = photos.get_untracked(); form.add_color(&ps); }>"+ Agregar color"</button>
                            {move || (!form.active()).then(|| view! {
                                <button type="button" class="btn btn-ghost btn-sm" on:click=move |_| form.on.set(false)>"Cancelar"</button>
                            })}
                            <span class="label whitespace-normal">"En 0 se ve tachado en la tienda y no se puede elegir."</span>
                        </div>
                    </div>
                }.into_any()
            }}
        </fieldset>
    }
}
