//! Mis fotos en el editor: la lista de fotos del dueño y la subida.
//!
//! Habla con las rutas `/api/media` del servidor (ver `web::media`).

use domain::MediaItem;
use leptos::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;

/// Cuántas fotos se suben a la vez.
const CONCURRENCY: usize = 2;

/// En qué paso va una foto de la bandeja.
#[derive(Clone, PartialEq)]
pub enum UploadState {
    Waiting,
    /// Calculando la huella y achicando en el dispositivo.
    Preparing,
    /// Mandando la foto: porcentaje enviado.
    Sending(u32),
    /// El servidor la está convirtiendo.
    Optimizing,
    Done(MediaItem),
    /// Ya estaba en Mis fotos.
    Duplicate(MediaItem),
    Failed(String),
}

impl UploadState {
    fn is_active(&self) -> bool {
        matches!(
            self,
            Self::Waiting | Self::Preparing | Self::Sending(_) | Self::Optimizing
        )
    }
}

/// Una fila de la bandeja de subidas.
#[derive(Clone, PartialEq)]
pub struct Upload {
    pub key: u32,
    pub name: String,
    /// Miniatura local (la foto elegida) mientras sube.
    pub preview: Option<String>,
    pub state: UploadState,
}

/// Lo que comparten todas las pantallas: la lista de fotos y la bandeja.
#[derive(Clone, Copy)]
struct Shared {
    items: RwSignal<Option<Vec<MediaItem>>>,
    load_error: RwSignal<Option<String>>,
    loading: StoredValue<bool>,
    queue: RwSignal<Vec<Upload>>,
    next_key: StoredValue<u32>,
    /// Una foto está volando de la bandeja a Mis fotos.
    flying: StoredValue<bool>,
}

impl Shared {
    fn new() -> Self {
        Self {
            items: RwSignal::new(None),
            load_error: RwSignal::new(None),
            loading: StoredValue::new(false),
            queue: RwSignal::new(Vec::new()),
            next_key: StoredValue::new(0),
            flying: StoredValue::new(false),
        }
    }
}

/// Crea el estado compartido de fotos para toda la app. Se llama una vez,
/// arriba de todo: así Mis fotos, productos y el editor ven la misma lista.
pub fn provide_photos() {
    if use_context::<Shared>().is_none() {
        provide_context(Shared::new());
    }
}

/// Estado de los selectores de imagen. La lista y la bandeja son una sola
/// para toda la app; los avisos de la última subida son de cada pantalla.
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
    shared: Shared,
}

impl MyPhotos {
    pub fn new() -> Self {
        let shared = use_context::<Shared>().unwrap_or_else(Shared::new);
        Self {
            items: shared.items,
            load_error: shared.load_error,
            progress: RwSignal::new(None),
            upload_errors: RwSignal::new(Vec::new()),
            saved: RwSignal::new(Vec::new()),
            shared,
        }
    }

    /// Trae la lista la primera vez que se abre un selector.
    pub fn ensure_loaded(self) {
        let sh = self.shared;
        if sh.items.get_untracked().is_some() || sh.loading.get_value() {
            return;
        }
        sh.loading.set_value(true);
        leptos::task::spawn_local(async move {
            match request("GET", "/api/media", None).await {
                Ok(text) => match serde_json::from_str::<Vec<MediaItem>>(&text) {
                    Ok(list) => {
                        sh.items.set(Some(list));
                        sh.load_error.set(None);
                    }
                    Err(_) => sh.load_error.set(Some(GENERIC.into())),
                },
                Err(msg) => sh.load_error.set(Some(msg)),
            }
            sh.loading.set_value(false);
        });
    }

    /// Sube las fotos (de a dos por vez). Si subió una sola sin problemas, la
    /// pasa a `done` para ponerla en el bloque.
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

    fn set_state(self, key: u32, state: UploadState) {
        self.shared.queue.update(|q| {
            if let Some(e) = q.iter_mut().find(|e| e.key == key) {
                e.state = state;
            }
        });
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
        let sh = self.shared;
        self.upload_errors.set(Vec::new());
        self.saved.set(Vec::new());
        let total = files.len();
        self.progress.set(Some(if total == 1 {
            "Subiendo la foto…".into()
        } else {
            format!("Subiendo {total} fotos…")
        }));
        // Una fila por foto, ya con su miniatura.
        let mut jobs = std::collections::VecDeque::new();
        for src in files {
            let key = sh.next_key.get_value();
            sh.next_key.set_value(key + 1);
            let (name, preview) = match &src {
                Source::File(f) => (f.name(), web_sys::Url::create_object_url_with_blob(f).ok()),
                Source::Link(l) => (short_link(l), None),
            };
            sh.queue.update(|q| {
                q.push(Upload {
                    key,
                    name,
                    preview,
                    state: UploadState::Waiting,
                })
            });
            jobs.push_back((key, src));
        }
        let jobs = std::rc::Rc::new(std::cell::RefCell::new(jobs));
        let uploaded = std::rc::Rc::new(std::cell::RefCell::new(Vec::<MediaItem>::new()));
        let remaining = std::rc::Rc::new(std::cell::Cell::new(total));
        for _ in 0..CONCURRENCY.min(total) {
            let (jobs, uploaded, remaining) = (jobs.clone(), uploaded.clone(), remaining.clone());
            leptos::task::spawn_local(async move {
                loop {
                    let Some((key, src)) = jobs.borrow_mut().pop_front() else {
                        break;
                    };
                    if let Some(item) = self.run_one(key, src, each).await {
                        uploaded.borrow_mut().push(item);
                    }
                    remaining.set(remaining.get() - 1);
                    if remaining.get() == 0 {
                        self.progress.set(None);
                        let mut up = uploaded.borrow_mut();
                        if total == 1 && up.len() == 1 {
                            done.run(up.remove(0));
                        }
                    }
                }
            });
        }
    }

    /// Sube una foto y actualiza su fila. Devuelve la foto si salió bien.
    async fn run_one(
        self,
        key: u32,
        src: Source,
        each: Option<Callback<MediaItem>>,
    ) -> Option<MediaItem> {
        let sh = self.shared;
        let name = sh
            .queue
            .with_untracked(|q| q.iter().find(|e| e.key == key).map(|e| e.name.clone()))
            .unwrap_or_default();
        let result = match &src {
            Source::File(f) => {
                self.set_state(key, UploadState::Preparing);
                upload_one(f, move |p| {
                    self.set_state(
                        key,
                        if p >= 100 {
                            UploadState::Optimizing
                        } else {
                            UploadState::Sending(p)
                        },
                    )
                })
                .await
            }
            Source::Link(l) => {
                self.set_state(key, UploadState::Optimizing);
                link_one(l).await
            }
        };
        match result {
            Ok(item) if item.duplicate => {
                self.saved.update(|s| {
                    s.push(format!(
                        "{name}: ya estaba en Mis fotos como «{}», usamos esa.",
                        item.name
                    ))
                });
                let item = MediaItem {
                    duplicate: false,
                    ..item
                };
                self.set_state(key, UploadState::Duplicate(item.clone()));
                self.add_item(&item);
                if let Some(each) = each {
                    each.run(item.clone());
                }
                Some(item)
            }
            Ok(item) => {
                if let Some(text) = domain::savings_text(&item) {
                    self.saved.update(|s| s.push(text));
                }
                self.land(key, &item).await;
                if let Some(each) = each {
                    each.run(item.clone());
                }
                Some(item)
            }
            Err(msg) => {
                self.upload_errors
                    .update(|e| e.push(format!("{name}: {msg}")));
                self.set_state(key, UploadState::Failed(msg));
                None
            }
        }
    }

    /// Anota la foto en la lista compartida (al principio).
    fn add_item(self, item: &MediaItem) {
        let sh = self.shared;
        if sh.items.get_untracked().is_none() {
            // Todavía no se bajó la lista: se baja entera, con esta incluida.
            self.ensure_loaded();
            return;
        }
        sh.items.update(|l| {
            if let Some(l) = l {
                l.retain(|p| p.id != item.id);
                l.insert(0, item.clone());
            }
        });
    }

    /// La foto terminó: sale volando de la bandeja a su lugar en Mis fotos.
    async fn land(self, key: u32, item: &MediaItem) {
        let sh = self.shared;
        // Que la foto ya esté bajada cuando llegue, para que no aparezca vacía.
        if let Some(base) = domain::media_base(&item.url) {
            preload(&format!("{base}/480.webp")).await;
        }
        // Una a la vez: una animación nueva cortaría a la anterior.
        while sh.flying.get_value() {
            sleep_ms(60).await;
        }
        sh.flying.set_value(true);
        let doc = document();
        let thumb = doc.get_element_by_id(&format!("upl-thumb-{key}"));
        set_vt_name(thumb.as_ref(), Some(&format!("foto-{}", item.id)));
        // Las primeras tarjetas de Mis fotos se corren para hacerle lugar.
        let cards = name_cards(12);
        let (this, item2) = (self, item.clone());
        with_view_transition(move || {
            set_vt_name(thumb.as_ref(), None);
            this.add_item(&item2);
            this.set_state(key, UploadState::Done(item2));
        })
        .await;
        for c in cards {
            set_vt_name(Some(&c), None);
        }
        // La miniatura local se libera un rato después, cuando ya no se ve.
        if let Some(url) = sh.queue.with_untracked(|q| {
            q.iter()
                .find(|e| e.key == key)
                .and_then(|e| e.preview.clone())
        }) {
            set_timeout(
                move || _ = web_sys::Url::revoke_object_url(&url),
                std::time::Duration::from_secs(5),
            );
        }
        sh.flying.set_value(false);
    }
}

fn set_vt_name(el: Option<&web_sys::Element>, name: Option<&str>) {
    if let Some(el) = el.and_then(|e| e.dyn_ref::<web_sys::HtmlElement>()) {
        _ = el
            .style()
            .set_property("view-transition-name", name.unwrap_or("none"));
    }
}

/// Les pone nombre a las primeras tarjetas de Mis fotos (`data-photo-card`)
/// para que el navegador las anime al correrse. Devuelve las tarjetas.
fn name_cards(max: u32) -> Vec<web_sys::Element> {
    let mut out = Vec::new();
    if let Ok(list) = document().query_selector_all("[data-photo-card]") {
        for i in 0..list.length().min(max) {
            if let Some(el) = list
                .get(i)
                .and_then(|n| n.dyn_into::<web_sys::Element>().ok())
            {
                let id = el.get_attribute("data-photo-card").unwrap_or_default();
                set_vt_name(Some(&el), Some(&format!("t-{id}")));
                out.push(el);
            }
        }
    }
    out
}

async fn sleep_ms(ms: i32) {
    let promise = js_sys::Promise::new(&mut |resolve, _| {
        if let Some(w) = web_sys::window() {
            _ = w.set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, ms);
        }
    });
    _ = JsFuture::from(promise).await;
}

/// Baja la imagen para que esté lista antes de mostrarla.
async fn preload(url: &str) {
    if let Ok(img) = web_sys::HtmlImageElement::new() {
        img.set_src(url);
        _ = JsFuture::from(img.decode()).await;
    }
}

/// Hace `update` dentro de una transición de vista del navegador (para que
/// las fotos con el mismo `view-transition-name` viajen de un lugar al otro).
/// Si el navegador no la tiene, o la persona pidió menos movimiento, hace el
/// cambio directo.
async fn with_view_transition(update: impl FnOnce() + 'static) {
    let doc = document();
    let reduce = web_sys::window()
        .and_then(|w| {
            w.match_media("(prefers-reduced-motion: reduce)")
                .ok()
                .flatten()
        })
        .is_some_and(|m| m.matches());
    let start = js_sys::Reflect::get(&doc, &"startViewTransition".into())
        .ok()
        .filter(|f| f.is_function());
    let Some(start) = start.filter(|_| !reduce) else {
        update();
        return;
    };
    let cb = wasm_bindgen::closure::Closure::once_into_js(move || -> wasm_bindgen::JsValue {
        update();
        // Mientras la transición espera, el navegador no dibuja cuadros (no
        // corre `requestAnimationFrame`): se espera con un temporizador corto,
        // lo justo para que Leptos termine de armar las tarjetas.
        js_sys::Promise::new(&mut |resolve, _| {
            if let Some(w) = web_sys::window() {
                _ = w.set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, 40);
            }
        })
        .into()
    });
    let Ok(vt) = start.unchecked_into::<js_sys::Function>().call1(&doc, &cb) else {
        return;
    };
    if let Ok(fin) = js_sys::Reflect::get(&vt, &"finished".into()) {
        if let Ok(p) = fin.dyn_into::<js_sys::Promise>() {
            _ = JsFuture::from(p).await;
        }
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
async fn upload_one(
    file: &web_sys::File,
    on_progress: impl Fn(u32) + 'static,
) -> Result<MediaItem, String> {
    let hash = file_hash(file).await;
    if let Some(h) = &hash {
        if let Ok(text) = request("GET", &format!("/api/media/by-hash/{h}"), None).await {
            if let Ok(item) = serde_json::from_str::<MediaItem>(&text) {
                return Ok(item);
            }
        }
    }
    let form = web_sys::FormData::new().map_err(|_| GENERIC.to_string())?;
    on_progress(0);
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
    let text = post_form("/api/media", &form, on_progress).await?;
    serde_json::from_str(&text).map_err(|_| GENERIC.to_string())
}

/// Manda un formulario con `XMLHttpRequest`, que a diferencia de `fetch` avisa
/// cuánto va enviado (porcentaje).
async fn post_form(
    url: &str,
    form: &web_sys::FormData,
    on_progress: impl Fn(u32) + 'static,
) -> Result<String, String> {
    let xhr = web_sys::XmlHttpRequest::new().map_err(|_| GENERIC.to_string())?;
    xhr.open("POST", url).map_err(|_| GENERIC.to_string())?;
    let progress = wasm_bindgen::closure::Closure::<dyn FnMut(web_sys::ProgressEvent)>::new(
        move |e: web_sys::ProgressEvent| {
            if e.length_computable() && e.total() > 0.0 {
                // El 100 % se avisa recién cuando responde el servidor.
                on_progress(((e.loaded() / e.total()) * 100.0).min(99.0) as u32);
            }
        },
    );
    if let Ok(up) = xhr.upload() {
        up.set_onprogress(Some(progress.as_ref().unchecked_ref()));
    }
    let finished = js_sys::Promise::new(&mut |resolve, _| {
        let (r1, r2) = (resolve.clone(), resolve);
        let load = wasm_bindgen::closure::Closure::once_into_js(move || {
            _ = r1.call0(&wasm_bindgen::JsValue::NULL);
        });
        let fail = wasm_bindgen::closure::Closure::once_into_js(move || {
            _ = r2.call0(&wasm_bindgen::JsValue::NULL);
        });
        xhr.set_onload(Some(load.unchecked_ref()));
        xhr.set_onerror(Some(fail.unchecked_ref()));
    });
    xhr.send_with_opt_form_data(Some(form))
        .map_err(|_| GENERIC.to_string())?;
    _ = JsFuture::from(finished).await;
    drop(progress);
    let status = xhr.status().unwrap_or(0);
    let text = xhr.response_text().ok().flatten().unwrap_or_default();
    interpret(status, text)
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
    let bitmap: web_sys::ImageBitmap =
        JsFuture::from(window.create_image_bitmap_with_blob(file).ok()?)
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
        let cb =
            wasm_bindgen::closure::Closure::once_into_js(move |blob: wasm_bindgen::JsValue| {
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
    interpret(res.status(), text)
}

/// La respuesta del servidor: el cuerpo si salió bien; si no, su texto (ya en
/// palabras simples) como mensaje.
fn interpret(status: u16, text: String) -> Result<String, String> {
    if (200..300).contains(&status) {
        Ok(text)
    } else if status == 413 {
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
                "La copiamos a Mis fotos. Podés pegar varios enlaces juntos. Google Fotos, Instagram y Facebook no dejan: bajá la foto y subila."
            </p>
        </div>
    }
}

/// La bandeja de subidas: aparece abajo a la derecha, en cualquier pantalla,
/// mientras hay fotos subiéndose. Cuando una termina, su miniatura vuela hasta
/// su lugar en Mis fotos (ver [`MyPhotos::land`]).
#[leptos::component]
pub fn UploadTray() -> impl IntoView {
    let Some(shared) = use_context::<Shared>() else {
        return ().into_any();
    };
    let queue = shared.queue;
    let any = move || queue.with(|q| !q.is_empty());
    let summary = move || {
        queue.with(|q| {
            let active = q.iter().filter(|e| e.state.is_active()).count();
            let failed = q
                .iter()
                .filter(|e| matches!(e.state, UploadState::Failed(_)))
                .count();
            let new = q
                .iter()
                .filter(|e| matches!(e.state, UploadState::Done(_)))
                .count();
            if active > 0 {
                format!(
                    "Subiendo {active} {}",
                    if active == 1 { "foto" } else { "fotos" }
                )
            } else if failed > 0 {
                format!(
                    "{failed} {} no se pudo subir",
                    if failed == 1 { "foto" } else { "fotos" }
                )
            } else if new > 0 {
                format!(
                    "Listo: {new} {} en Mis fotos",
                    if new == 1 {
                        "foto nueva"
                    } else {
                        "fotos nuevas"
                    }
                )
            } else {
                "Listo".to_string()
            }
        })
    };
    // Cuando todas terminaron sin problemas, la bandeja se cierra sola.
    Effect::new(move |_| {
        let (finished, failed, n) = queue.with(|q| {
            (
                !q.is_empty() && q.iter().all(|e| !e.state.is_active()),
                q.iter().any(|e| matches!(e.state, UploadState::Failed(_))),
                q.len(),
            )
        });
        if finished && !failed {
            set_timeout(
                move || {
                    let still = queue
                        .with_untracked(|q| q.len() == n && q.iter().all(|e| !e.state.is_active()));
                    let failed = queue.with_untracked(|q| {
                        q.iter().any(|e| matches!(e.state, UploadState::Failed(_)))
                    });
                    if still && !failed {
                        queue.set(Vec::new());
                    }
                },
                std::time::Duration::from_millis(2200),
            );
        }
    });
    let keys = move || -> Vec<u32> { queue.with(|q| q.iter().map(|u| u.key).collect()) };
    let has_failed = move || {
        queue.with(|q| q.iter().any(|e| matches!(e.state, UploadState::Failed(_))))
            && queue.with(|q| q.iter().all(|e| !e.state.is_active()))
    };
    view! {
        <div class="pointer-events-none fixed inset-x-4 bottom-20 z-50 flex justify-end sm:bottom-5 sm:right-5 sm:left-auto" class:hidden=move || !any()>
            <section class="pointer-events-auto w-full max-w-72 rounded-box bg-base-100 p-3 shadow-xl ring-1 ring-base-300"
                aria-label="Subida de fotos" role="status" aria-live="polite">
                <div class="mb-1 flex items-center justify-between gap-2">
                    <h2 class="text-sm font-semibold">{summary}</h2>
                    <button type="button" class="btn btn-ghost btn-xs" class:hidden=move || !has_failed()
                        on:click=move |_| queue.set(Vec::new())>"Cerrar"</button>
                </div>
                <ul class="flex max-h-60 flex-col gap-1 overflow-y-auto">
                    <For each=keys key=|k| *k children=move |key| view! { <UploadRow queue key/> }/>
                </ul>
            </section>
        </div>
    }
    .into_any()
}

#[leptos::component]
fn UploadRow(queue: RwSignal<Vec<Upload>>, key: u32) -> impl IntoView {
    let row = move || queue.with(|q| q.iter().find(|e| e.key == key).cloned());
    let name = move || row().map(|r| r.name).unwrap_or_default();
    let state = move || row().map(|r| r.state);
    let preview = queue.with_untracked(|q| {
        q.iter()
            .find(|e| e.key == key)
            .and_then(|e| e.preview.clone())
    });
    let thumb = move || {
        match state() {
        Some(UploadState::Done(_)) => view! {
            <svg viewBox="0 0 24 24" class="h-7 w-7 rounded-md bg-success/15 p-1 text-success" aria-hidden="true">
                <path d="M6 12.5l3.6 3.6L18 8" fill="none" stroke="currentColor" stroke-width="2.4" stroke-linecap="round" stroke-linejoin="round"/>
            </svg>
        }
        .into_any(),
        Some(UploadState::Duplicate(item)) => view! {
            <img src=domain::media_base(&item.url).map(|b| format!("{b}/480.webp")).unwrap_or(item.url) alt="" class="h-7 w-7 rounded-md object-cover"/>
        }
        .into_any(),
        _ => match preview.clone() {
            Some(src) => view! { <img src=src alt="" class="h-7 w-7 rounded-md object-cover"/> }.into_any(),
            None => view! { <span class="block h-7 w-7 rounded-md bg-base-300"></span> }.into_any(),
        },
    }
    };
    let status = move || {
        match state() {
        Some(UploadState::Waiting) | None => view! { <span class="text-xs text-base-content/60">"En espera"</span> }.into_any(),
        Some(UploadState::Preparing) => view! {
            <progress class="progress progress-primary w-full" aria-label=move || format!("Preparando {}", name())></progress>
        }
        .into_any(),
        Some(UploadState::Sending(p)) => view! {
            <progress class="progress progress-primary w-full" value=p.to_string() max="100"
                aria-label=move || format!("Subiendo {}", name())></progress>
        }
        .into_any(),
        Some(UploadState::Optimizing) => view! {
            <progress class="progress progress-primary w-full" aria-label=move || format!("Procesando {}", name())></progress>
        }
        .into_any(),
        Some(UploadState::Done(item)) => {
            let _ = item;
            let text = "Lista";
            view! { <span class="text-xs text-success">{text}</span> }.into_any()
        }
        Some(UploadState::Duplicate(_)) => view! {
            <span class="text-xs text-base-content/70">"Ya estaba en Mis fotos, usamos esa."</span>
        }
        .into_any(),
        Some(UploadState::Failed(msg)) => view! { <span class="text-xs text-error">{msg}</span> }.into_any(),
    }
    };
    view! {
        <li class="grid grid-cols-[1.75rem_1fr] items-center gap-x-2.5 gap-y-0.5 py-0.5">
            <span id=format!("upl-thumb-{key}") class="row-span-2 self-center">{thumb}</span>
            <span class="truncate text-sm">{name}</span>
            <span class="min-w-0">{status}</span>
        </li>
    }
}
