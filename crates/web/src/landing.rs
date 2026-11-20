//! La página de inicio de la plataforma (`mkr.com.ar/` sin cuenta): qué es
//! MarketMaker, cómo se ve el editor, tres diseños de tienda, qué incluye y
//! preguntas. Diseño "Vitrina oscura" (opción B, elegida el 2026-11-16).

use crate::account::Logo;
use leptos::prelude::*;

/// La tienda de ejemplo (`demo`) en esta misma plataforma.
fn demo_url() -> String {
    format!(
        "{}://{}",
        crate::site::scheme(),
        domain::store_host(&crate::site::current_host(), "demo")
    )
}

/// Foto de producto de muestra: AVIF con WebP de respaldo.
#[component]
fn Pic(n: u8, #[prop(into)] class: String) -> impl IntoView {
    view! {
        <picture>
            <source srcset=format!("/img/product-{n}.avif") type="image/avif"/>
            <img src=format!("/img/product-{n}.webp") alt="" loading="lazy" class=class/>
        </picture>
    }
}

#[component]
pub fn Landing() -> impl IntoView {
    let demo = demo_url();
    view! {
        <div data-theme="light" class="min-h-screen bg-[#faf9f7] text-[#16131f]">
            <Hero demo/>
            <Stores/>
            <Included/>
            <Faq/>
            <footer class="bg-[#110e1c] py-14 text-sm text-white/60">
                <div class="mx-auto flex max-w-6xl flex-wrap items-center justify-between gap-4 px-6 lg:px-10">
                    <Logo class="text-white"/>
                    <nav class="flex flex-wrap gap-x-6 gap-y-2" aria-label="Legales">
                        <a href="/terminos" class="hover:text-white">"Términos"</a>
                        <a href="/privacidad" class="hover:text-white">"Privacidad"</a>
                        <a href="mailto:hola@mkr.com.ar" class="hover:text-white">"hola@mkr.com.ar"</a>
                    </nav>
                </div>
            </footer>
        </div>
    }
}

#[component]
fn Hero(demo: String) -> impl IntoView {
    view! {
        <div class="relative overflow-hidden bg-[#110e1c] text-white">
            <div class="pointer-events-none absolute -top-[420px] left-1/2 size-[900px] -translate-x-1/2 rounded-full bg-[radial-gradient(closest-side,rgba(106,85,240,.55),rgba(67,48,214,0)_70%)]" aria-hidden="true"></div>
            <header class="relative">
                <nav class="mx-auto flex h-[76px] max-w-6xl items-center gap-10 px-6 lg:px-10" aria-label="Principal">
                    <a href="/" class="text-lg"><Logo/></a>
                    <div class="hidden gap-7 text-[15px] text-white/70 md:flex">
                        <a href="#tiendas" class="hover:text-white">"Tiendas"</a>
                        <a href="#incluye" class="hover:text-white">"Qué incluye"</a>
                        <a href="#preguntas" class="hover:text-white">"Preguntas"</a>
                    </div>
                    <div class="ml-auto flex gap-2.5">
                        <a href="/entrar" class="btn btn-sm h-9 rounded-lg border-white/20 bg-transparent px-4 text-white hover:bg-white/10">"Entrar"</a>
                        <a href="/entrar?crear" class="btn btn-sm h-9 rounded-lg border-0 bg-white px-4 text-[#110e1c] hover:bg-white/90">"Crear cuenta"</a>
                    </div>
                </nav>
            </header>
            <div class="relative mx-auto max-w-6xl px-6 pt-16 text-center lg:px-10 lg:pt-20">
                <span class="inline-flex items-center gap-2.5 rounded-full border border-white/20 py-1.5 pr-4 pl-1.5 text-sm text-white/85">
                    <b class="rounded-full bg-[#ffb547] px-2.5 py-0.5 text-xs text-[#110e1c]">"Nuevo"</b>
                    "Cobrá con Mercado Pago desde tu tienda"
                </span>
                <h1 class="mx-auto mt-6 mb-5 max-w-4xl text-5xl leading-[1.02] font-extrabold tracking-[-0.045em] sm:text-6xl lg:text-7xl">
                    "Vendé online con una tienda "
                    <span class="bg-gradient-to-r from-[#b9adff] to-[#ffcf8a] bg-clip-text text-transparent">"que se ve como vos."</span>
                </h1>
                <p class="mx-auto max-w-2xl text-lg text-white/70 sm:text-xl">
                    "Armala arrastrando bloques, cargá tus productos y compartí tu dirección. Para emprendedores de toda la Argentina."
                </p>
                <div class="mt-9 mb-16 flex flex-wrap justify-center gap-3">
                    <a href="/entrar?crear" class="btn btn-primary btn-lg h-[52px] rounded-xl px-7">"Crear mi tienda"</a>
                    <a href=demo class="btn btn-lg h-[52px] rounded-xl border-white/20 bg-transparent px-7 text-white hover:bg-white/10">"Ver una tienda de ejemplo"</a>
                    <p class="w-full text-sm text-white/60">"Por ahora, solo con invitación."</p>
                </div>
            </div>
            <EditorPreview/>
        </div>
    }
}

/// Dibujo del editor (no es una captura: liviano y siempre al día con la marca).
#[component]
fn EditorPreview() -> impl IntoView {
    // Cada bloque con su dibujito de color, como en la paleta real.
    let blocks = [
        ("Banner", "bg-gradient-to-r from-[#203a2c] to-[#3e6b52]"),
        ("Productos", "bg-[repeating-linear-gradient(90deg,#c9c3f5_0_8px,transparent_8px_11px)]"),
        ("Carrusel", "bg-gradient-to-r from-[#ff5f6d] to-[#ffc371]"),
        ("Texto", "bg-[repeating-linear-gradient(180deg,#bdb9cc_0_2px,transparent_2px_6px)]"),
        ("Etiquetas", "bg-[radial-gradient(circle,#6a55f0_0_3px,transparent_3.5px)] bg-[length:10px_10px]"),
        ("Preguntas", "bg-[repeating-linear-gradient(180deg,#d8d4e6_0_5px,transparent_5px_8px)]"),
        ("Oferta", "bg-[#ffb547]"),
        ("Contacto", "bg-gradient-to-r from-[#4330d6] to-[#8f7dff]"),
    ];
    view! {
        <div class="relative mx-auto hidden h-[460px] w-[min(1080px,calc(100%-48px))] grid-cols-[200px_1fr_220px] grid-rows-[48px_1fr] overflow-hidden rounded-t-2xl bg-white text-left text-[#16131f] shadow-[0_-10px_80px_rgba(106,85,240,.35)] md:grid"
            role="img" aria-label="Así se ve el editor: bloques a la izquierda, la tienda en el medio y sus opciones a la derecha">
            <div class="col-span-3 flex items-center gap-3 border-b border-[#e7e4ee] px-4 text-[13px]">
                <span class="grid size-6 place-items-center rounded-md bg-gradient-to-br from-[#4330d6] to-[#6a55f0] text-xs font-extrabold text-white">"m"</span>
                <b>"Huerta Nativa"</b><span class="text-[#5d5a6e]">"Inicio ▾"</span>
                <span class="ml-6 text-[#5d5a6e]">"Guardado · sin publicar"</span>
                <span class="ml-auto rounded-lg bg-[#4330d6] px-3.5 py-1.5 font-semibold text-white">"Publicar"</span>
            </div>
            <div class="grid grid-cols-2 content-start gap-2.5 border-r border-[#e7e4ee] p-3.5">
                {blocks.into_iter().map(|(b, art)| view! {
                    <div class="flex h-[60px] flex-col justify-between gap-1.5 rounded-[10px] bg-[#f3f2f8] p-2 text-[11px] font-semibold text-[#5d5a6e]">
                        <i class=format!("block h-5 w-full rounded {art}")></i>
                        {b}
                    </div>
                }).collect_view()}
            </div>
            <div class="bg-[#f1f0f6] p-5">
                <div class="flex h-full flex-col gap-3 rounded-[10px] bg-white p-4">
                    <div class="relative h-[110px] rounded-[10px] bg-gradient-to-r from-[#203a2c] to-[#3e6b52] p-4 text-white outline-2 outline-offset-[3px] outline-[#4330d6]">
                        <i class="absolute -top-6 left-0 rounded bg-[#4330d6] px-2 py-0.5 text-[11px] not-italic">"Banner"</i>
                        <b class="text-[22px] tracking-tight">"Plantas que duran"</b>
                        <div class="mt-1 text-xs opacity-80">"Envíos en CABA y GBA"</div>
                    </div>
                    <div class="grid grid-cols-4 gap-2.5">
                        {[4u8, 2, 3, 6].into_iter().map(|n| view! { <Pic n class="aspect-square w-full rounded-lg object-cover"/> }).collect_view()}
                    </div>
                </div>
            </div>
            <div class="flex flex-col gap-3.5 border-l border-[#e7e4ee] p-4 text-[13px]">
                <b>"Banner"</b>
                <div class="flex rounded-lg bg-[#f2f1f6] p-[3px] text-xs">
                    <span class="flex-1 rounded-md bg-white p-1 text-center font-semibold shadow-sm">"Contenido"</span>
                    <span class="flex-1 p-1 text-center">"Diseño"</span>
                </div>
                <div>"Color"
                    <div class="mt-2 flex gap-2">
                        <i class="size-[26px] rounded-full bg-[#2f553f] outline-2 outline-offset-2 outline-[#4330d6]"></i>
                        <i class="size-[26px] rounded-full bg-[#4330d6]"></i>
                        <i class="size-[26px] rounded-full bg-[#ffb547]"></i>
                        <i class="size-[26px] rounded-full bg-[#16131f]"></i>
                    </div>
                </div>
                <div>"Diseño"
                    <div class="mt-2 flex rounded-lg bg-[#f2f1f6] p-[3px] text-xs">
                        {(1..=5).map(|i| view! {
                            <span class=if i == 2 { "flex-1 rounded-md bg-white p-1 text-center font-semibold shadow-sm" } else { "flex-1 p-1 text-center" }>{i}</span>
                        }).collect_view()}
                    </div>
                </div>
            </div>
        </div>
    }
}

/// Tres diseños de muestra hechos con el mismo editor.
#[component]
fn Stores() -> impl IntoView {
    struct Sample {
        name: &'static str,
        banner: &'static str,
        look: &'static str,
        bg: &'static str,
        banner_class: &'static str,
        pics: [u8; 3],
    }
    let samples = [
        Sample {
            name: "Huerta Nativa",
            banner: "Plantas que duran",
            look: "Natural",
            bg: "bg-[#f3efe6]",
            banner_class: "bg-gradient-to-r from-[#203a2c] to-[#3e6b52] text-white",
            pics: [4, 2, 3],
        },
        Sample {
            name: "Sonido Sur",
            banner: "Auriculares y más",
            look: "Oscuro",
            bg: "bg-[#16131f] text-white",
            banner_class: "bg-gradient-to-r from-[#ff5f6d] to-[#ffc371] text-white",
            pics: [1, 8, 6],
        },
        Sample {
            name: "Papelería Tinta",
            banner: "Volvé a clases",
            look: "Claro",
            bg: "bg-[#eef3ff]",
            banner_class: "bg-gradient-to-r from-[#4330d6] to-[#8f7dff] text-white",
            pics: [7, 2, 5],
        },
    ];
    view! {
        <section id="tiendas" class="py-24">
            <div class="mx-auto max-w-6xl px-6 lg:px-10">
                <div class="mb-11 flex flex-wrap items-end justify-between gap-6">
                    <div>
                        <h2 class="text-4xl leading-tight font-extrabold tracking-[-0.035em] lg:text-[44px]">"Una tienda, mil estilos"</h2>
                        <p class="mt-2.5 text-lg text-[#5d5a6e]">"Mismo editor, tres estilos que no se parecen en nada. Así pueden verse las tiendas."</p>
                    </div>
                </div>
                <div class="grid gap-6 md:grid-cols-3">
                    {samples.into_iter().map(|s| view! {
                        <div class="overflow-hidden rounded-[20px] border border-[#e7e4ee] bg-white">
                            <div class=format!("flex h-[250px] flex-col gap-2.5 p-4 {}", s.bg) aria-hidden="true">
                                <div class="flex justify-between text-sm font-extrabold"><span>{s.name}</span><span>"Carrito"</span></div>
                                <div class=format!("flex flex-1 flex-col justify-end rounded-[10px] p-4 text-xl font-extrabold tracking-tight {}", s.banner_class)>{s.banner}</div>
                                <div class="grid grid-cols-3 gap-2">
                                    {s.pics.into_iter().map(|n| view! { <Pic n class="h-14 w-full rounded-md object-cover"/> }).collect_view()}
                                </div>
                            </div>
                            <div class="flex items-center justify-between border-t border-[#e7e4ee] px-5 py-4">
                                <div><b>{format!("Diseño \"{}\"", s.look)}</b><small class="block text-[13px] text-[#5d5a6e]">"Colores, letras y bloques a tu gusto"</small></div>
                            </div>
                        </div>
                    }).collect_view()}
                </div>
            </div>
        </section>
    }
}

#[component]
fn Included() -> impl IntoView {
    let items = [
        "Editor de arrastrar y soltar",
        "Tu dirección tunombre.mkr.com.ar",
        "Cobros con Mercado Pago",
        "Envíos y retiro en tu local",
        "Stock que se descuenta solo",
        "Un email por cada pedido",
        "Carrito y ficha de producto",
        "Tus fotos, livianas y nítidas",
    ];
    view! {
        <section id="incluye" class="border-t border-[#e7e4ee] bg-white py-24">
            <div class="mx-auto grid max-w-6xl items-start gap-16 px-6 lg:grid-cols-2 lg:gap-20 lg:px-10">
                <div>
                    <h2 class="text-4xl leading-tight font-extrabold tracking-[-0.035em] lg:text-[44px]">"Todo incluido, desde el primer día"</h2>
                    <ul class="mt-7 grid gap-x-7 gap-y-4 sm:grid-cols-2">
                        {items.into_iter().map(|i| view! {
                            <li class="flex gap-3 text-[15px]">
                                <b class="grid size-6 shrink-0 place-items-center rounded-full bg-[#e3f6ec] text-[13px] text-[#127a4a]" aria-hidden="true">"✓"</b>{i}
                            </li>
                        }).collect_view()}
                    </ul>
                </div>
                <div class="rounded-3xl bg-[#110e1c] p-9 text-white">
                    <div class="text-white/70">"Para empezar"</div>
                    <div class="text-5xl font-extrabold tracking-[-0.04em] lg:text-6xl">"Gratis"</div>
                    <p class="mt-2 mb-6 text-white/70">"Sin tarjeta. Por ahora MarketMaker es solo por invitación: si tu email está en la lista, creás tu cuenta y tu tienda en un minuto."</p>
                    <a href="/entrar?crear" class="btn btn-lg h-[52px] w-full rounded-xl border-0 bg-white text-[#110e1c] hover:bg-white/90">"Crear mi cuenta"</a>
                </div>
            </div>
        </section>
    }
}

#[component]
fn Faq() -> impl IntoView {
    let questions = [
        ("¿Necesito saber programar?", "No. Todo se hace con clics y arrastrando bloques. Lo que ves en el editor es lo que ven tus clientes."),
        ("¿Cómo me pagan?", "Vinculás tu cuenta de Mercado Pago una vez y cada compra se cobra ahí, directo a tu cuenta. También podés aceptar pagos al retirar."),
        ("¿Mis clientes necesitan una cuenta para comprar?", "No. Eligen sus productos, dejan sus datos y pagan. La cuenta es solo para quien arma la tienda."),
        ("¿Puedo usar mi propio dominio?", "Por ahora cada tienda vive en su dirección nombre.mkr.com.ar, con candado de seguridad incluido. El dominio propio llega más adelante."),
    ];
    view! {
        <section id="preguntas" class="bg-white pb-24">
            <div class="mx-auto grid max-w-6xl gap-10 px-6 lg:grid-cols-2 lg:gap-20 lg:px-10">
                <h2 class="text-4xl leading-tight font-extrabold tracking-[-0.035em] lg:text-[44px]">"Preguntas"</h2>
                <div class="border-t border-[#e7e4ee]">
                    {questions.into_iter().enumerate().map(|(i, (q, a))| view! {
                        <details class="group border-b border-[#e7e4ee] py-5" open=i == 0>
                            <summary class="flex cursor-pointer list-none items-center justify-between gap-4 text-[17px] font-semibold">
                                {q}
                                <span class="text-xl text-[#5d5a6e] group-open:hidden" aria-hidden="true">"+"</span>
                                <span class="hidden text-xl text-[#5d5a6e] group-open:inline" aria-hidden="true">"−"</span>
                            </summary>
                            <p class="mt-2 text-[15px] text-[#5d5a6e]">{a}</p>
                        </details>
                    }).collect_view()}
                </div>
            </div>
        </section>
    }
}
