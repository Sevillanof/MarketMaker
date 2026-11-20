//! Ayuda con películas: el botón "?" del panel y su reproductor.
//!
//! Cada película es un guion (`Script`) que se repite solo: un cursor va a
//! cada campo, escribe y toca, con una frase corta debajo. Las pantallas que
//! se ven son copias del marcado real (mismas clases de daisyUI), no dibujos:
//! si cambia una pantalla, hay que actualizar su película acá.
//!
//! Primera tanda: cargar un producto, vincular Mercado Pago y conectar Andreani.

use crate::panel::Section;
use leptos::prelude::*;
use std::{sync::Arc, time::Duration};

/// Ancho con el que se dibuja cada película; después se achica al de la ventana.
const PANEL_FILM: (f64, f64) = (960.0, 600.0);
/// Las del editor son más grandes: es una pantalla entera.
pub(crate) const EDITOR_FILM: (f64, f64) = (1360.0, 800.0);

/// Primera película del editor (las anteriores son del panel).
const FIRST_EDITOR: usize = 3;

fn film_size(n: usize) -> (f64, f64) {
    if n >= FIRST_EDITOR {
        EDITOR_FILM
    } else {
        PANEL_FILM
    }
}

/// Un instante de la película: qué se ve y dónde está el cursor.
#[derive(Clone, Default, PartialEq)]
pub(crate) struct Frame {
    /// Cuánto dura antes del siguiente (en milisegundos).
    pub ms: u32,
    pub caption: &'static str,
    /// Elemento (`data-t`) al que apunta el cursor.
    pub target: &'static str,
    pub click: bool,
    pub flags: u64,
    /// Textos que se van escribiendo, por campo.
    pub t: [String; 3],
}

/// Armado de una película, paso a paso.
pub(crate) struct Script {
    frames: Vec<Frame>,
    cur: Frame,
}

impl Script {
    pub fn new() -> Self {
        Self::with_flags(0)
    }
    /// Película que arranca con `flags` ya puestos.
    pub fn with_flags(flags: u64) -> Self {
        let mut s = Self {
            frames: Vec::new(),
            cur: Frame {
                flags,
                ..Frame::default()
            },
        };
        s.push(900);
        s
    }
    pub fn push(&mut self, ms: u32) {
        let mut f = self.cur.clone();
        f.ms = ms;
        self.frames.push(f);
        self.cur.click = false;
    }
    /// El cursor va a `target` y se muestra `caption`.
    pub fn go(&mut self, target: &'static str, caption: &'static str, ms: u32) -> &mut Self {
        self.cur.target = target;
        self.cur.caption = caption;
        self.push(ms);
        self
    }
    pub fn click(&mut self) -> &mut Self {
        self.cur.click = true;
        self.push(450);
        self
    }
    pub fn set(&mut self, flag: u64) -> &mut Self {
        self.cur.flags |= flag;
        self
    }
    pub fn clear(&mut self, flag: u64) -> &mut Self {
        self.cur.flags &= !flag;
        self
    }
    pub fn typing(&mut self, slot: usize, text: &str) -> &mut Self {
        for n in 1..=text.chars().count() {
            self.cur.t[slot] = text.chars().take(n).collect();
            self.push(65);
        }
        self.push(500);
        self
    }
    pub fn say(&mut self, caption: &'static str, ms: u32) -> &mut Self {
        self.cur.caption = caption;
        self.push(ms);
        self
    }
    pub fn finish(self) -> Vec<Frame> {
        self.frames
    }
}

// Qué hay en pantalla en cada película (bits de `Frame::flags`).
const P_PHOTO: u64 = 1;
const P_PREVIEW: u64 = 2;
const P_SAVED: u64 = 4;
const M_WINDOW: u64 = 1;
const M_LINKED: u64 = 2;
const A_CONNECTED: u64 = 1;
const A_ENABLED: u64 = 2;

/// Películas del panel (las primeras) y los pasos del recorrido del editor.
const PANEL_FILMS: [(&str, &str); FIRST_EDITOR] = [
    ("Cargá un producto", "Productos"),
    ("Vinculá Mercado Pago", "Cobros"),
    ("Conectá Andreani", "Entregas"),
];
const FILM_COUNT: usize = FIRST_EDITOR + crate::help_editor::STEPS;

fn film_title(n: usize) -> &'static str {
    if n >= FIRST_EDITOR {
        crate::help_editor::TITLES[n - FIRST_EDITOR]
    } else {
        PANEL_FILMS[n].0
    }
}

fn film_where(n: usize) -> &'static str {
    if n >= FIRST_EDITOR {
        "Editor"
    } else {
        PANEL_FILMS[n].1
    }
}

/// Primera y última película de la ventana en la que está `n`.
fn film_range(n: usize) -> (usize, usize) {
    if n >= FIRST_EDITOR {
        (FIRST_EDITOR, FILM_COUNT - 1)
    } else {
        (0, FIRST_EDITOR - 1)
    }
}

fn script(film: usize) -> Vec<Frame> {
    if film >= FIRST_EDITOR {
        return crate::help_editor::script(film - FIRST_EDITOR);
    }
    let mut s = Script::new();
    match film {
        0 => {
            s.go(
                "photo-add",
                "Elegí una foto de Mis fotos o subí una nueva. La primera es la principal.",
                1800,
            )
            .click()
            .set(P_PHOTO)
            .push(1000);
            s.go(
                "name",
                "Escribí el nombre, tal como lo van a ver tus clientes.",
                1400,
            )
            .click()
            .typing(0, "Taza de cerámica");
            s.go(
                "price",
                "Poné el precio en pesos. A la derecha ves cómo queda en tu tienda.",
                1600,
            )
            .click()
            .typing(1, "18.500")
            .set(P_PREVIEW)
            .push(1400);
            s.go(
                "stock",
                "Si querés, indicá cuántas unidades tenés. Cada venta las descuenta.",
                1600,
            )
            .click()
            .typing(2, "12");
            s.go(
                "save",
                "Tocá \"Crear producto\". Ya aparece en tu tienda.",
                1600,
            )
            .click()
            .set(P_SAVED)
            .push(3200);
        }
        1 => {
            s.go("mp-btn", "Tocá \"Vincular cuenta\" en Mercado Pago.", 1800)
                .click()
                .set(M_WINDOW)
                .push(1200);
            s.go(
                "mp-allow",
                "Mercado Pago te pide permiso. Entrá a tu cuenta y tocá \"Permitir\".",
                2200,
            )
            .click()
            .clear(M_WINDOW)
            .set(M_LINKED)
            .push(800);
            s.say(
                "Listo: tu tienda ya cobra con Mercado Pago y la plata va directo a tu cuenta.",
                3600,
            );
        }
        _ => {
            s.go("a-signup", "Si todavía no tenés cuenta, creala gratis en Andreani PyMEs. Te pide tu CUIL o CUIT.", 2600);
            s.go(
                "a-integ",
                "En Andreani, entrá a Integraciones, elegí WooCommerce y copiá el código.",
                2600,
            );
            s.go("a-code", "Pegá el código acá.", 1400)
                .click()
                .typing(0, "AND-7F3K-92XQ-41LM");
            s.go("a-connect", "Tocá \"Conectar\".", 1400)
                .click()
                .set(A_CONNECTED)
                .push(1400);
            s.go("a-enabled", "Activá Andreani en \"Finalizar compra\". Tus clientes ven el costo real del envío.", 2000)
                .click()
                .set(A_ENABLED)
                .push(3400);
        }
    }
    s.finish()
}

/// Corre los instantes uno tras otro, en bucle, hasta que `gen` cambie.
fn tick(
    frames: Arc<Vec<Frame>>,
    i: usize,
    g: u32,
    gen: RwSignal<u32>,
    frame: RwSignal<Frame>,
    paused: RwSignal<bool>,
) {
    if gen.get_untracked() != g {
        return;
    }
    if paused.get_untracked() {
        set_timeout(
            move || tick(frames, i, g, gen, frame, paused),
            Duration::from_millis(250),
        );
        return;
    }
    let f = frames[i % frames.len()].clone();
    let ms = u64::from(f.ms);
    frame.set(f);
    set_timeout(
        move || tick(frames, i + 1, g, gen, frame, paused),
        Duration::from_millis(ms),
    );
}

/// Pedido para abrir la Ayuda en una película (por ejemplo, el "Ver cómo" de
/// los Primeros pasos). Lo provee el panel; `HelpButton` lo atiende.
#[derive(Clone, Copy)]
pub struct HelpRequest(pub RwSignal<Option<usize>>);

/// Películas: la 0 es cargar un producto, la 1 Cobros, la 2 Andreani y la 3
/// el recorrido del editor.
pub const FILM_PRODUCT: usize = 0;
pub const FILM_PAYMENTS: usize = 1;
#[allow(dead_code)]
pub const FILM_ANDREANI: usize = 2;
pub const FILM_EDITOR: usize = FIRST_EDITOR;

/// El botón "?" (se agranda y dice "Ayuda" al pasar el mouse) y su ventana.
#[component]
pub fn HelpButton(
    section: Signal<Section>,
    /// En el editor: se abre en las películas del editor.
    #[prop(optional)]
    editor: bool,
) -> impl IntoView {
    let dialog = NodeRef::<leptos::html::Dialog>::new();
    let stage = NodeRef::<leptos::html::Div>::new();
    let open = RwSignal::new(false);
    let film = RwSignal::new(0usize);
    let frame = RwSignal::new(Frame::default());
    let gen = RwSignal::new(0u32);
    let paused = RwSignal::new(false);
    let scale = RwSignal::new(0.7f64);
    let cursor = RwSignal::new((24.0f64, 24.0f64));

    let play = move |n: usize| {
        film.set(n);
        frame.set(Frame::default());
        gen.update(|g| *g += 1);
        let g = gen.get_untracked();
        tick(Arc::new(script(n)), 0, g, gen, frame, paused);
        // Con la ventana ya dibujada: se mide para achicar la película a su ancho.
        request_animation_frame(move || {
            if let Some(st) = stage.get_untracked() {
                scale.set((f64::from(st.client_width()) / film_size(n).0).clamp(0.3, 1.0));
            }
        });
    };
    let open_at = move |n: usize| {
        paused.set(false);
        open.set(true);
        if let Some(d) = dialog.get() {
            _ = d.show_modal();
        }
        play(n);
    };
    let start = move |_| {
        let n = match section.get_untracked() {
            _ if editor => FIRST_EDITOR,
            Section::Payments => 1,
            Section::Coupons => 0,
            Section::Delivery => 2,
            _ => 0,
        };
        open_at(n);
    };
    // "Ver cómo" de los Primeros pasos.
    if let Some(req) = use_context::<HelpRequest>() {
        Effect::new(move |_| {
            if let Some(n) = req.0.get() {
                req.0.set(None);
                open_at(n);
            }
        });
    }

    // El cursor sigue al elemento al que apunta el instante actual.
    Effect::new(move |_| {
        let target = frame.with(|f| f.target);
        if target.is_empty() {
            cursor.set((24.0, 24.0));
            return;
        }
        request_animation_frame(move || {
            let Some(st) = stage.get_untracked() else {
                return;
            };
            let Ok(Some(el)) = st.query_selector(&format!("[data-t=\"{target}\"]")) else {
                return;
            };
            let (s, r) = (st.get_bounding_client_rect(), el.get_bounding_client_rect());
            cursor.set((
                r.left() - s.left() + (r.width() * 0.5).min(70.0),
                r.top() - s.top() + r.height() * 0.55,
            ));
        });
    });

    view! {
        <button type="button" aria-label="Ayuda" on:click=start
            class="mk-help group btn max-sm:right-auto max-sm:left-4 btn-primary fixed right-4 bottom-[max(1rem,env(safe-area-inset-bottom))] z-40 h-11 min-h-0 w-11 gap-0 overflow-hidden rounded-full p-0 shadow-lg transition-all duration-300 hover:h-12 hover:w-28 focus-visible:h-12 focus-visible:w-28 motion-reduce:transition-none">
            <span aria-hidden="true" class="text-xl font-bold transition-opacity group-hover:opacity-0 group-focus-visible:opacity-0">"?"</span>
            <span aria-hidden="true" class="absolute inset-0 grid place-items-center text-base font-semibold opacity-0 transition-opacity group-hover:opacity-100 group-focus-visible:opacity-100">"Ayuda"</span>
        </button>
        <dialog node_ref=dialog class="modal" aria-labelledby="help-title"
            on:close=move |_| { gen.update(|g| *g += 1); open.set(false); }>
            <div class=move || { if film.get() >= FIRST_EDITOR { "modal-box w-11/12 max-w-5xl" } else { "modal-box w-11/12 max-w-3xl" } } data-theme="light">
                <form method="dialog">
                    <button class="btn btn-sm btn-circle btn-ghost absolute right-3 top-3" aria-label="Cerrar">"✕"</button>
                </form>
                {move || open.get().then(|| view! {
                    <div class="flex flex-col gap-3">
                        <div>
                            <h2 id="help-title" class="text-xl font-bold">{move || film_title(film.get())}</h2>
                            <p class="text-sm text-base-content/60">{move || film_where(film.get())}</p>
                        </div>
                        // Es una animación decorativa: el texto accesible está debajo.
                        <div node_ref=stage aria-hidden="true" inert=""
                            style=move || { let (w, h) = film_size(film.get()); format!("aspect-ratio:{w}/{h}") }
                            class="relative w-full overflow-hidden rounded-box border border-base-300 bg-base-200">
                            <div class="absolute left-0 top-0 origin-top-left bg-base-200"
                                style=move || format!("width:{}px;transform:scale({})", film_size(film.get()).0, scale.get())>
                                {move || match film.get() {
                                    0 => view! { <ProductFilm f=frame/> }.into_any(),
                                    1 => view! { <PaymentsFilm f=frame/> }.into_any(),
                                    2 => view! { <AndreaniFilm f=frame/> }.into_any(),
                                    _ => view! { <crate::help_editor::EditorFilm f=frame/> }.into_any(),
                                }}
                            </div>
                            <div class="pointer-events-none absolute z-20 size-0 transition-[left,top] duration-700 ease-out motion-reduce:transition-none"
                                style=move || format!("left:{}px;top:{}px", cursor.get().0, cursor.get().1)>
                                {move || frame.with(|f| f.click).then(|| view! {
                                    <span class="absolute -left-3 -top-3 size-6 rounded-full border-2 border-primary motion-safe:animate-ping"></span>
                                })}
                                <svg class="absolute left-0 top-0 size-7 drop-shadow-md" viewBox="0 0 24 24">
                                    <path d="M4 2l15 9-6.5 1.5L9 19z" fill="#fff" stroke="#111" stroke-width="1.6" stroke-linejoin="round"/>
                                </svg>
                            </div>
                        </div>
                        <p class="min-h-14 text-center text-lg font-semibold" role="status" aria-live="off">
                            {move || frame.with(|f| f.caption)}
                        </p>
                        <ol class="sr-only">
                            {move || {
                                let mut seen: Vec<&'static str> = Vec::new();
                                for f in script(film.get()) {
                                    if !f.caption.is_empty() && !seen.contains(&f.caption) {
                                        seen.push(f.caption);
                                    }
                                }
                                seen.into_iter().map(|c| view! { <li>{c}</li> }).collect_view()
                            }}
                        </ol>
                        <button type="button" class="btn btn-outline btn-sm gap-1.5 self-center" aria-pressed=move || paused.get().to_string()
                            on:click=move |_| paused.update(|p| *p = !*p)>
                            <span aria-hidden="true" class="text-xs">{move || if paused.get() { "▶" } else { "❚❚" }}</span>
                            {move || if paused.get() { "Reanudar animación" } else { "Pausar animación" }}
                        </button>
                        <div class="flex items-center justify-center gap-4">
                            <button type="button" class="btn btn-primary" disabled=move || film.get() == film_range(film.get()).0
                                on:click=move |_| play(film.get_untracked().saturating_sub(1))>{move || if film.get() >= FIRST_EDITOR { "Anterior" } else { "Guía anterior" }}</button>
                            <span class="min-w-16 text-center text-lg font-bold tabular-nums">
                                {move || { let (a, _) = film_range(film.get()); film.get() - a + 1 }}
                                <span class="font-normal text-base-content/60">{move || { let (a, b) = film_range(film.get()); format!(" / {}", b - a + 1) }}</span>
                            </span>
                            <button type="button" class="btn btn-primary" disabled=move || film.get() == film_range(film.get()).1
                                on:click=move |_| play((film.get_untracked() + 1).min(film_range(film.get_untracked()).1))>{move || if film.get() >= FIRST_EDITOR { "Siguiente" } else { "Guía siguiente" }}</button>
                        </div>
                    </div>
                })}
            </div>
            <form method="dialog" class="modal-backdrop"><button aria-label="Cerrar">"cerrar"</button></form>
        </dialog>
    }
}

pub(crate) fn has(f: RwSignal<Frame>, bit: u64) -> bool {
    f.with(|f| f.flags & bit != 0)
}

/// Productos → Nuevo producto (el formulario real, recortado).
#[component]
fn ProductFilm(f: RwSignal<Frame>) -> impl IntoView {
    let name = move || f.with(|f| f.t[0].clone());
    let preview = move || {
        let name = name();
        let mut p = domain::Product {
            id: "demo".into(),
            name: if name.is_empty() {
                "Taza de cerámica".into()
            } else {
                name
            },
            price_cents: 1_850_000,
            ..Default::default()
        };
        if has(f, P_PHOTO) {
            p.image_url = "/img/product-1.webp".into();
            p.photos = vec![p.image_url.clone()];
        }
        p
    };
    view! {
        <div class="flex flex-col gap-5 p-6">
            <div class="flex flex-wrap items-center gap-3">
                <span class="btn btn-ghost btn-sm">"← Productos"</span>
                <h1 class="min-w-0 grow truncate text-3xl font-bold">
                    {move || { let n = name(); if n.trim().is_empty() { "Nuevo producto".to_string() } else { n } }}
                </h1>
                <span class="text-sm text-base-content/60">{move || if has(f, P_SAVED) { "Guardado ✓" } else { "" }}</span>
                <span class="btn btn-primary" data-t="save">"Crear producto"</span>
            </div>
            <div class="grid grid-cols-[1fr_340px] items-start gap-6">
                <div class="card bg-base-100 shadow-sm"><div class="card-body gap-5">
                    <fieldset class="fieldset">
                        <legend class="fieldset-legend">"Nombre"</legend>
                        <input data-t="name" class="input w-full" readonly tabindex="-1" placeholder="Por ejemplo: Taza de cerámica"
                            prop:value=name/>
                    </fieldset>
                    <div class="grid grid-cols-2 gap-4">
                        <fieldset class="fieldset">
                            <legend class="fieldset-legend">"Precio"</legend>
                            <label class="input w-full"><span>"$"</span>
                                <input data-t="price" readonly tabindex="-1" placeholder="45.999,90" prop:value=move || f.with(|f| f.t[1].clone())/>
                            </label>
                        </fieldset>
                        <fieldset class="fieldset">
                            <legend class="fieldset-legend">"Precio anterior " <span class="font-normal text-base-content/60">"(opcional, se muestra tachado)"</span></legend>
                            <label class="input w-full"><span>"$"</span>
                                <input readonly tabindex="-1" placeholder="Vacío si no está en oferta"/>
                            </label>
                        </fieldset>
                    </div>
                    <fieldset class="fieldset">
                        <legend class="fieldset-legend">"Fotos " <span class="font-normal text-base-content/60">"· la primera es la principal; tocá una foto para encuadrarla"</span></legend>
                        <ul class="flex flex-wrap gap-3">
                            {move || has(f, P_PHOTO).then(|| view! {
                                <li class="relative h-24 w-32 overflow-hidden rounded-box bg-base-200">
                                    <ui::Image src="/img/product-1.webp" alt="Foto del producto"/>
                                    <span class="badge badge-primary badge-sm absolute left-1 top-1">"Principal"</span>
                                </li>
                            })}
                            <li>
                                <span data-t="photo-add" class="flex h-24 w-32 flex-col items-center justify-center gap-1 rounded-box border-2 border-dashed border-base-300 text-sm text-base-content/70">
                                    <span class="text-xl" aria-hidden="true">"+"</span>"Elegir de Mis fotos"
                                </span>
                            </li>
                            <li>
                                <span class="flex h-24 w-32 flex-col items-center justify-center gap-1 rounded-box border-2 border-dashed border-base-300 text-sm text-base-content/70">
                                    <span class="text-xl" aria-hidden="true">"⇪"</span>"Subir fotos"
                                </span>
                            </li>
                        </ul>
                    </fieldset>
                    <fieldset class="fieldset">
                        <legend class="fieldset-legend">"Stock"</legend>
                        <span class="fieldset-label">"Cantidad disponible " <span class="text-base-content/60">"(opcional)"</span></span>
                        <input data-t="stock" class="input w-40" readonly tabindex="-1" placeholder="Sin contar" prop:value=move || f.with(|f| f.t[2].clone())/>
                        <p class="label whitespace-normal">"Cada venta descuenta. En 0 se ve \"Agotado\"; con 3 o menos, \"Quedan N\"."</p>
                    </fieldset>
                </div></div>
                <aside class="flex flex-col gap-3">
                    <span class="text-sm font-medium text-base-content/70">"Así se ve en tu tienda"</span>
                    <div class="h-80">
                        {move || if has(f, P_PREVIEW) {
                            view! { <widgets::ProductCard product=preview() show_price=true/> }.into_any()
                        } else {
                            view! {
                                <div class="flex h-full items-center justify-center rounded-box border border-dashed border-base-300 p-6 text-center text-sm text-base-content/60">
                                    "Poné un precio para ver cómo queda."
                                </div>
                            }.into_any()
                        }}
                    </div>
                </aside>
            </div>
        </div>
    }
}

/// Cobros (la pantalla real) con la ventana de permiso de Mercado Pago.
#[component]
fn PaymentsFilm(f: RwSignal<Frame>) -> impl IntoView {
    view! {
        <div class="relative flex min-h-[600px] flex-col gap-5 p-6">
            <div>
                <h1 class="text-3xl font-bold">"Cobros"</h1>
                <p class="text-base-content/70">"Elegí cómo te pueden pagar. Podés tener más de una."</p>
            </div>
            <div class="card divide-y divide-base-200 bg-base-100 shadow-sm">
                <div class="flex items-start gap-4 px-6 py-5">
                    {crate::payments::method_icon("mp")}
                    <div class="min-w-0 flex-1">
                        <h2 class="font-bold">"Mercado Pago"</h2>
                        {move || if has(f, M_LINKED) {
                            view! {
                                <p class="text-sm text-base-content/70">"Cuenta n.º 1234567890 · vinculada el 18/11/2026"</p>
                                <p class="mt-1 text-sm text-success">"Tu tienda ya cobra con Mercado Pago."</p>
                            }.into_any()
                        } else {
                            view! { <p class="text-sm text-base-content/70">"Tarjeta, débito o dinero en cuenta. La plata va directo a tu cuenta."</p> }.into_any()
                        }}
                    </div>
                    {move || if has(f, M_LINKED) {
                        view! {
                            <div class="flex shrink-0 items-center gap-2">
                                <span class="badge badge-success badge-soft">"Vinculada"</span>
                                <span class="btn btn-ghost btn-sm">"Desvincular"</span>
                            </div>
                        }.into_any()
                    } else {
                        view! {
                            <span data-t="mp-btn" class="btn btn-sm shrink-0 border-0 bg-[#009ee3] text-white">"Vincular cuenta"</span>
                        }.into_any()
                    }}
                </div>
                <div class="flex items-start gap-4 px-6 py-5">
                    {crate::payments::method_icon("transfer")}
                    <div class="min-w-0 flex-1">
                        <h2 class="font-bold">"Transferencia"</h2>
                        <p class="text-sm text-base-content/70">"Te transfieren a tu cuenta bancaria o virtual."</p>
                    </div>
                    <input type="checkbox" class="toggle toggle-primary mt-1 shrink-0" tabindex="-1" disabled/>
                </div>
                <div class="flex items-start gap-4 px-6 py-5">
                    {crate::payments::method_icon("coordinate")}
                    <div class="min-w-0 flex-1">
                        <h2 class="font-bold">"Acordar con el comprador"</h2>
                        <p class="text-sm text-base-content/70">"Le escribís por WhatsApp y arreglan cómo paga, por ejemplo efectivo al retirar."</p>
                    </div>
                    <input type="checkbox" class="toggle toggle-primary mt-1 shrink-0" tabindex="-1" disabled/>
                </div>
            </div>
            {move || has(f, M_WINDOW).then(|| view! {
                // Representa la página de Mercado Pago, que se abre aparte.
                <div class="absolute inset-0 z-10 grid place-items-center bg-black/40">
                    <div class="w-[440px] overflow-hidden rounded-box bg-white text-neutral shadow-2xl">
                        <div class="bg-[#009ee3] px-5 py-3 font-bold text-white">"Mercado Pago"</div>
                        <div class="flex flex-col gap-3 p-5">
                            <p class="text-lg font-semibold">"MarketMaker quiere conectarse a tu cuenta"</p>
                            <p class="text-sm text-neutral/70">"Para cobrar las ventas de tu tienda. La plata va directo a tu cuenta."</p>
                            <span data-t="mp-allow" class="btn btn-sm w-max border-0 bg-[#009ee3] text-white">"Permitir"</span>
                        </div>
                    </div>
                </div>
            })}
        </div>
    }
}

/// Entregas → Envíos con Andreani (la tarjeta real con sus tres pasos).
#[component]
fn AndreaniFilm(f: RwSignal<Frame>) -> impl IntoView {
    let step = "flex size-7 shrink-0 items-center justify-center rounded-full bg-primary text-sm font-bold text-primary-content";
    view! {
        <div class="flex flex-col gap-5 p-6">
            <section class="card bg-base-100 shadow-sm">
                <div class="card-body gap-4">
                    <div>
                        <h2 class="text-xl font-bold">"Envíos con Andreani"</h2>
                        <p class="text-sm text-base-content/70">
                            "Tus compradores ven el costo real del envío y eligen domicilio o sucursal. Vos imprimís la etiqueta y despachás."
                        </p>
                    </div>
                    {move || if has(f, A_CONNECTED) {
                        view! {
                            <div class="flex flex-wrap items-center gap-3 rounded-box bg-success/10 px-4 py-3">
                                <span class="grow text-sm">
                                    <span class="font-semibold text-success">"✓ Conectado"</span>
                                    " como " <span class="font-semibold">"Taller Norte"</span>
                                    <span class="text-base-content/70">" · CUIT 30-71234567-8"</span>
                                </span>
                                <span class="btn btn-sm btn-ghost">"Desconectar"</span>
                            </div>
                            <label class="flex items-center gap-3">
                                <input data-t="a-enabled" type="checkbox" class="toggle toggle-primary" tabindex="-1"
                                    prop:checked=move || has(f, A_ENABLED)/>
                                <span class="font-semibold">"Ofrecer Andreani en \"Finalizar compra\""</span>
                            </label>
                        }.into_any()
                    } else {
                        view! {
                            <ol class="flex flex-col gap-4">
                                <li class="flex gap-3">
                                    <span class=step>"1"</span>
                                    <div>
                                        <p class="font-semibold">"Creá tu cuenta PyME en Andreani"</p>
                                        <p class="text-sm text-base-content/70">"Es gratis. Te pide tu CUIL o CUIT. Si ya tenés cuenta, seguí al paso 2."</p>
                                        <span data-t="a-signup" class="link link-primary text-sm">"Ir a Andreani PyMEs ↗"</span>
                                    </div>
                                </li>
                                <li class="flex gap-3">
                                    <span class=step>"2"</span>
                                    <div>
                                        <p class="font-semibold">"Copiá tu código de integración"</p>
                                        <p class="text-sm text-base-content/70">"En Andreani, entrá a Integraciones, elegí WooCommerce y copiá el código que aparece."</p>
                                        <span data-t="a-integ" class="link link-primary text-sm">"Abrir Integraciones ↗"</span>
                                    </div>
                                </li>
                                <li class="flex gap-3">
                                    <span class=step>"3"</span>
                                    <div class="grow">
                                        <span class="font-semibold">"Pegalo acá"</span>
                                        <div class="mt-1 flex flex-wrap gap-2">
                                            <input data-t="a-code" class="input grow font-mono" readonly tabindex="-1" placeholder="Código de Andreani"
                                                prop:value=move || f.with(|f| f.t[0].clone())/>
                                            <span data-t="a-connect" class="btn btn-primary">"Conectar"</span>
                                        </div>
                                    </div>
                                </li>
                            </ol>
                        }.into_any()
                    }}
                </div>
            </section>
        </div>
    }
}
