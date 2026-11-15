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
        self.upload_with(files, done, None);
    }

    /// Como [`MyPhotos::upload`], pero pasa a `each` cada foto que subió bien
    /// (para sumar varias a un producto).
    pub fn upload_each(self, files: Vec<web_sys::File>, each: Callback<MediaItem>) {
        self.upload_with(files, Callback::new(|_| ()), Some(each));
    }

    fn upload_with(
        self,
        files: Vec<web_sys::File>,
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
                self.progress.set(Some(if total == 1 {
                    "Subiendo y optimizando la foto…".to_string()
                } else {
                    format!("Subiendo y optimizando {} de {total}…", i + 1)
                }));
                let name = file.name();
                match upload_one(&file).await {
                    Ok(item) => {
                        if let Some(text) = domain::savings_text(&item) {
                            self.saved.update(|s| s.push(text));
                        }
                        self.items
                            .update(|l| l.get_or_insert_with(Vec::new).insert(0, item.clone()));
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

const GENERIC: &str =
    "No pudimos conectarnos con el servidor. Revisá tu conexión y probá de nuevo.";

async fn upload_one(file: &web_sys::File) -> Result<MediaItem, String> {
    let form = web_sys::FormData::new().map_err(|_| GENERIC.to_string())?;
    form.append_with_blob_and_filename("file", file, &file.name())
        .map_err(|_| GENERIC.to_string())?;
    let text = request("POST", "/api/media", Some(form.into())).await?;
    serde_json::from_str(&text).map_err(|_| GENERIC.to_string())
}

/// Hace el pedido y devuelve el cuerpo. Si el servidor responde con error,
/// su texto (ya en palabras simples) es el mensaje.
async fn request(
    method: &str,
    url: &str,
    body: Option<wasm_bindgen::JsValue>,
) -> Result<String, String> {
    let init = web_sys::RequestInit::new();
    init.set_method(method);
    init.set_credentials(web_sys::RequestCredentials::SameOrigin);
    if let Some(b) = body.as_ref() {
        init.set_body(b);
    }
    let req =
        web_sys::Request::new_with_str_and_init(url, &init).map_err(|_| GENERIC.to_string())?;
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
