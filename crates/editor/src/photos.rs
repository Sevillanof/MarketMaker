//! Mis fotos en el editor: la lista de fotos del dueño y la subida.
//!
//! Habla con las rutas `/api/media` del servidor (ver `web::media`).

use domain::MediaItem;
use leptos::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;

/// Estado compartido por todos los selectores de imagen del editor.
#[derive(Clone, Copy)]
pub struct MyPhotos {
    /// `None` hasta la primera carga.
    pub items: RwSignal<Option<Vec<MediaItem>>>,
    /// Error al traer la lista, en palabras simples.
    pub load_error: RwSignal<Option<String>>,
    /// "Subiendo 2 de 3…" mientras sube.
    pub progress: RwSignal<Option<String>>,
    /// Problemas de la última subida, uno por foto.
    pub upload_errors: RwSignal<Vec<String>>,
    /// Cuánto se achicó cada foto de la última subida.
    pub saved: RwSignal<Vec<String>>,
    loading: StoredValue<bool>,
}

impl MyPhotos {
    pub fn new() -> Self {
        Self {
            items: RwSignal::new(None),
            load_error: RwSignal::new(None),
            progress: RwSignal::new(None),
            upload_errors: RwSignal::new(Vec::new()),
            saved: RwSignal::new(Vec::new()),
            loading: StoredValue::new(false),
        }
    }

    /// Trae la lista la primera vez que se abre un selector.
    pub fn ensure_loaded(self) {
        if self.items.get_untracked().is_some() || self.loading.get_value() {
            return;
        }
        self.loading.set_value(true);
        leptos::task::spawn_local(async move {
            match request("GET", "/api/media", None).await {
                Ok(text) => match serde_json::from_str::<Vec<MediaItem>>(&text) {
                    Ok(list) => {
                        self.items.set(Some(list));
                        self.load_error.set(None);
                    }
                    Err(_) => self.load_error.set(Some(GENERIC.into())),
                },
                Err(msg) => self.load_error.set(Some(msg)),
            }
            self.loading.set_value(false);
        });
    }

    /// Sube las fotos de a una. Si subió una sola sin problemas, la pasa a
    /// `done` para ponerla en el bloque.
    pub fn upload(self, files: Vec<web_sys::File>, done: Callback<MediaItem>) {
        self.upload_with(files.into_iter().map(Source::File).collect(), done, None);
    }

    /// Trae las fotos de los enlaces pegados (uno o varios, separados por
    /// espacios o renglones) y las guarda en Mis fotos. Como [`upload`]:
    /// si es una sola, la pasa a `done`; cada una que sale bien, a `each`.
    pub fn from_links(
        self,
        text: &str,
        done: Callback<MediaItem>,
        each: Option<Callback<MediaItem>>,
    ) {
        let links: Vec<Source> = text
            .split_whitespace()
            .filter(|l| l.contains('.'))
            .take(20)
            .map(|l| Source::Link(l.to_string()))
            .collect();
        if links.is_empty() {
            self.upload_errors.set(vec![
                "Pegá el enlace de la foto (empieza con https://).".into()
            ]);
            return;
        }
        self.upload_with(links, done, each);
    }

    /// Como [`MyPhotos::upload`], pero pasa a `each` cada foto que subió bien
    /// (para sumar varias a un producto).
    pub fn upload_each(self, files: Vec<web_sys::File>, each: Callback<MediaItem>) {
        self.upload_with(
            files.into_iter().map(Source::File).collect(),
            Callback::new(|_| ()),
            Some(each),
        );
    }

    fn upload_with(
        self,
        files: Vec<Source>,
        done: Callback<MediaItem>,
        each: Option<Callback<MediaItem>>,
    ) {
        if files.is_empty() || self.progress.get_untracked().is_some() {
            return;
        }
        self.upload_errors.set(Vec::new());
        self.saved.set(Vec::new());
        let total = files.len();
        leptos::task::spawn_local(async move {
            let mut uploaded = Vec::new();
            for (i, file) in files.into_iter().enumerate() {
                let verb = match &file {
                    Source::File(_) => "Subiendo",
                    Source::Link(_) => "Trayendo",
                };
                self.progress.set(Some(if total == 1 {
                    format!("{verb} y optimizando la foto…")
                } else {
                    format!("{verb} y optimizando {} de {total}…", i + 1)
                }));
                let (name, result) = match &file {
                    Source::File(f) => (f.name(), upload_one(f).await),
                    Source::Link(l) => (short_link(l), link_one(l).await),
                };
                match result {
                    Ok(item) => {
                        let text = if item.duplicate {
                            Some(format!(
                                "{name}: ya estaba en Mis fotos como «{}», usamos esa.",
                                item.name
                            ))
                        } else {
                            domain::savings_text(&item)
                        };
                        if let Some(text) = text {
                            self.saved.update(|s| s.push(text));
                        }
                        let item = MediaItem {
                            duplicate: false,
                            ..item
                        };
                        self.items.update(|l| {
                            let l = l.get_or_insert_with(Vec::new);
                            l.retain(|p| p.id != item.id);
                            l.insert(0, item.clone());
                        });
                        if let Some(each) = each {
                            each.run(item.clone());
                        }
                        uploaded.push(item);
                    }
                    Err(msg) => self
                        .upload_errors
                        .update(|e| e.push(format!("{name}: {msg}"))),
                }
            }
            self.progress.set(None);
            if total == 1 && uploaded.len() == 1 {
                done.run(uploaded.remove(0));
            }
        });
    }
}

/// De dónde sale una foto nueva.
enum Source {
    File(web_sys::File),
    Link(String),
}

/// `dropbox.com/…/remera.jpg` para los mensajes, sin el `https://` ni lo
/// que va después del `?`.
fn short_link(link: &str) -> String {
    let l = link.split("://").nth(1).unwrap_or(link);
    let l = l
        .split(['?', '#'])
        .next()
        .unwrap_or(l)
        .trim_start_matches("www.");
    if l.chars().count() > 50 {
        let head: String = l.chars().take(24).collect();
        let tail: String = l
            .chars()
            .rev()
            .take(20)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect();
        format!("{head}…{tail}")
    } else {
        l.to_string()
    }
}

async fn link_one(link: &str) -> Result<MediaItem, String> {
    let body = serde_json::json!({ "url": link }).to_string();
    let text = request_json("/api/media/link", &body).await?;
    serde_json::from_str(&text).map_err(|_| GENERIC.to_string())
}

const GENERIC: &str =
    "No pudimos conectarnos con el servidor. Revisá tu conexión y probá de nuevo.";

/// Lado más ancho que guarda el servidor: más que esto no se usa nunca.
const MAX_UPLOAD_WIDTH: f64 = 1600.0;
/// Las fotos que pesan menos que esto se suben tal cual.
const SHRINK_FROM_BYTES: f64 = 800_000.0;

/// Sube una foto. Antes de mandarla: se fija si ya está en Mis fotos (con una
/// huella del archivo, sin subirlo) y, si es una foto grande, la achica a
/// 1600 px en el propio dispositivo (pesa 5 a 10 veces menos).
async fn upload_one(file: &web_sys::File) -> Result<MediaItem, String> {
    let hash = file_hash(file).await;
    if let Some(h) = &hash {
        if let Ok(text) = request("GET", &format!("/api/media/by-hash/{h}"), None).await {
            if let Ok(item) = serde_json::from_str::<MediaItem>(&text) {
                return Ok(item);
            }
        }
    }
    let form = web_sys::FormData::new().map_err(|_| GENERIC.to_string())?;
    match shrink(file).await {
        Some(small) => {
            form.append_with_blob_and_filename("file", &small, &file.name())
                .map_err(|_| GENERIC.to_string())?;
            // El servidor anota la huella y el peso de la foto original.
            if let Some(h) = &hash {
                _ = form.append_with_str("sha256", h);
            }
            _ = form.append_with_str("original_size", &(file.size() as u64).to_string());
        }
        None => {
            form.append_with_blob_and_filename("file", file, &file.name())
                .map_err(|_| GENERIC.to_string())?;
        }
    }
    let text = request("POST", "/api/media", Some(form.into())).await?;
    serde_json::from_str(&text).map_err(|_| GENERIC.to_string())
}

/// Huella SHA-256 (en hexadecimal) del archivo, o `None` si el navegador no
/// puede calcularla.
async fn file_hash(file: &web_sys::File) -> Option<String> {
    let subtle = web_sys::window()?.crypto().ok()?.subtle();
    let data = JsFuture::from(file.array_buffer()).await.ok()?;
    let digest = JsFuture::from(
        subtle
            .digest_with_str_and_buffer_source("SHA-256", &data.unchecked_into())
            .ok()?,
    )
    .await
    .ok()?;
    let bytes = js_sys::Uint8Array::new(&digest).to_vec();
    Some(bytes.iter().map(|b| format!("{b:02x}")).collect())
}

/// La foto achicada a 1600 px de ancho en JPG, o `None` si se sube como está
/// (es chica, es un logo o un PNG con transparencia, o el navegador no la
/// puede abrir: el servidor se encarga igual).
async fn shrink(file: &web_sys::File) -> Option<web_sys::Blob> {
    let kind = file.type_();
    // Un PNG, GIF o SVG puede tener transparencia o animación: no se toca.
    let photo = matches!(
        kind.as_str(),
        "image/jpeg" | "image/webp" | "image/heic" | "image/heif"
    );
    let heic = matches!(kind.as_str(), "image/heic" | "image/heif");
    if !photo || (!heic && file.size() < SHRINK_FROM_BYTES) {
        return None;
    }
    let window = web_sys::window()?;
    // Gira la foto según cómo se sacó, igual que el servidor.
    let bitmap: web_sys::ImageBitmap = JsFuture::from(window.create_image_bitmap_with_blob(file).ok()?)
        .await
        .ok()?
        .dyn_into()
        .ok()?;
    let (w, h) = (f64::from(bitmap.width()), f64::from(bitmap.height()));
    if w < 1.0 || h < 1.0 || (!heic && w <= MAX_UPLOAD_WIDTH) {
        return None;
    }
    let tw = w.min(MAX_UPLOAD_WIDTH);
    let th = (h * tw / w).round().max(1.0);
    let canvas: web_sys::HtmlCanvasElement = window
        .document()?
        .create_element("canvas")
        .ok()?
        .dyn_into()
        .ok()?;
    canvas.set_width(tw as u32);
    canvas.set_height(th as u32);
    let ctx: web_sys::CanvasRenderingContext2d = canvas.get_context("2d").ok()??.dyn_into().ok()?;
    _ = js_sys::Reflect::set(&ctx, &"imageSmoothingQuality".into(), &"high".into());
    ctx.draw_image_with_image_bitmap_and_dw_and_dh(&bitmap, 0.0, 0.0, tw, th)
        .ok()?;
    bitmap.close();
    let promise = js_sys::Promise::new(&mut |resolve, _| {
        let cb = wasm_bindgen::closure::Closure::once_into_js(move |blob: wasm_bindgen::JsValue| {
            _ = resolve.call1(&wasm_bindgen::JsValue::NULL, &blob);
        });
        _ = canvas.to_blob_with_type_and_encoder_options(
            cb.unchecked_ref(),
            "image/jpeg",
            &wasm_bindgen::JsValue::from_f64(0.92),
        );
    });
    let blob: web_sys::Blob = JsFuture::from(promise).await.ok()?.dyn_into().ok()?;
    // Si no quedó más liviana, mejor la original.
    (blob.size() < file.size() || heic).then_some(blob)
}

/// Hace el pedido y devuelve el cuerpo. Si el servidor responde con error,
/// su texto (ya en palabras simples) es el mensaje.
async fn request(
    method: &str,
    url: &str,
    body: Option<wasm_bindgen::JsValue>,
) -> Result<String, String> {
    send(method, url, body, None).await
}

/// `POST` con un cuerpo JSON.
async fn request_json(url: &str, json: &str) -> Result<String, String> {
    send(
        "POST",
        url,
        Some(wasm_bindgen::JsValue::from_str(json)),
        Some("application/json"),
    )
    .await
}

async fn send(
    method: &str,
    url: &str,
    body: Option<wasm_bindgen::JsValue>,
    content_type: Option<&str>,
) -> Result<String, String> {
    let init = web_sys::RequestInit::new();
    init.set_method(method);
    init.set_credentials(web_sys::RequestCredentials::SameOrigin);
    if let Some(b) = body.as_ref() {
        init.set_body(b);
    }
    let req =
        web_sys::Request::new_with_str_and_init(url, &init).map_err(|_| GENERIC.to_string())?;
    if let Some(ct) = content_type {
        req.headers()
            .set("content-type", ct)
            .map_err(|_| GENERIC.to_string())?;
    }
    let window = web_sys::window().ok_or_else(|| GENERIC.to_string())?;
    let res: web_sys::Response = JsFuture::from(window.fetch_with_request(&req))
        .await
        .map_err(|_| GENERIC.to_string())?
        .dyn_into()
        .map_err(|_| GENERIC.to_string())?;
    let text = match res.text() {
        Ok(p) => JsFuture::from(p)
            .await
            .ok()
            .and_then(|v| v.as_string())
            .unwrap_or_default(),
        Err(_) => String::new(),
    };
    if res.ok() {
        Ok(text)
    } else if res.status() == 413 {
        Err(
            "Esta foto pesa más de 10 MB, que es el máximo. Probá exportarla con menos calidad."
                .into(),
        )
    } else if text.is_empty() || text.len() > 400 {
        Err(GENERIC.into())
    } else {
        Err(text)
    }
}

/// Las fotos elegidas en un `<input type="file">`, que queda vacío para poder
/// elegir la misma otra vez.
pub fn take_files(ev: &leptos::ev::Event) -> Vec<web_sys::File> {
    let Some(input) = ev
        .target()
        .and_then(|t| t.dyn_into::<web_sys::HtmlInputElement>().ok())
    else {
        return Vec::new();
    };
    let files = input
        .files()
        .map(|list| (0..list.length()).filter_map(|i| list.get(i)).collect())
        .unwrap_or_default();
    input.set_value("");
    files
}

/// "Pegar enlace": un botón que abre un renglón para pegar el enlace de una
/// foto de otro sitio (Dropbox, Drive, una página de producto…) y traerla a
/// Mis fotos. Cerrado no ocupa lugar.
#[leptos::component]
pub fn LinkImport(
    photos: MyPhotos,
    /// Si trajo una sola foto, se la pasa acá (para ponerla en el bloque).
    done: Callback<MediaItem>,
    /// Cada foto que trajo bien (para sumarlas a un producto).
    #[prop(optional)]
    each: Option<Callback<MediaItem>>,
    /// Prefijo único para los `id`.
    #[prop(into)]
    uid: String,
    /// Botón chico (en el panel del editor).
    #[prop(optional)]
    small: bool,
) -> impl IntoView {
    let open = RwSignal::new(false);
    let text = RwSignal::new(String::new());
    let input_id = format!("{uid}-enlace");
    let help_id = format!("{uid}-enlace-ayuda");
    let busy = move || photos.progress.get().is_some();
    let go = move || {
        let t = text.get_untracked();
        let done = Callback::new(move |item: MediaItem| {
            done.run(item);
            open.set(false);
        });
        photos.from_links(&t, done, each);
        if !t.trim().is_empty() {
            text.set(String::new());
        }
    };
    let btn = if small {
        "btn btn-ghost btn-xs"
    } else {
        "btn btn-outline"
    };
    let input = if small {
        "input input-sm min-w-0 flex-1"
    } else {
        "input min-w-0 flex-1"
    };
    let input_id2 = input_id.clone();
    view! {
        <button type="button" class=btn
            aria-expanded=move || open.get().to_string()
            aria-controls=input_id.clone()
            on:click=move |_| {
                open.update(|o| *o = !*o);
                if open.get_untracked() {
                    if let Some(el) = leptos::prelude::document().get_element_by_id(&input_id2) {
                        use wasm_bindgen::JsCast;
                        if let Ok(el) = el.dyn_into::<web_sys::HtmlElement>() {
                            _ = el.focus();
                        }
                    }
                }
            }>
            "Pegar enlace"
        </button>
        <div class="basis-full flex-col gap-1" class:hidden=move || !open.get() class:flex=move || open.get()>
            <div class="flex w-full items-center gap-2">
                <input id=input_id type="text" inputmode="url" class=input
                    placeholder="Pegá el enlace de la foto (Dropbox, Drive, una página…)"
                    aria-label="Enlace de la foto" aria-describedby=help_id.clone()
                    prop:value=move || text.get()
                    on:input=move |ev| text.set(event_target_value(&ev))
                    on:keydown=move |ev| {
                        if ev.key() == "Enter" {
                            ev.prevent_default();
                            go();
                        } else if ev.key() == "Escape" {
                            open.set(false);
                        }
                    }/>
                <button type="button" class=if small { "btn btn-primary btn-sm" } else { "btn btn-primary" }
                    disabled=move || busy() || text.with(|t| t.trim().is_empty())
                    on:click=move |_| go()>"Traer"</button>
            </div>
            <p id=help_id class="text-xs text-base-content/60">
                "La copiamos a Mis fotos y la optimizamos. Podés pegar varios enlaces juntos. Google Fotos, Instagram y Facebook no dejan: bajá la foto y subila."
            </p>
        </div>
    }
}
