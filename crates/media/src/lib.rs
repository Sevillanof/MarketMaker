//! Fotos del dueño: se convierten a AVIF y WebP en [`WIDTHS`] y se guardan
//! en un [`Store`]. Solo servidor.
//!
//! Al convertir se gira la foto según cómo la sacó el celular y se pierden
//! los datos ocultos del archivo original (incluida la ubicación GPS).

pub mod fetch;
pub use fetch::Fetcher;

use std::io::Cursor;
use std::path::PathBuf;

use rusty_s3::S3Action;

use image::{DynamicImage, ImageDecoder, ImageReader};

/// Anchos que se generan. Una foto más chica no se agranda.
pub const WIDTHS: [u32; 3] = [480, 960, 1600];
/// El ancho que guardan los bloques en `image_url`.
pub const MAIN_WIDTH: u32 = 960;
/// Peso máximo de una foto subida.
pub const MAX_BYTES: usize = 10 * 1024 * 1024;
/// Lado máximo aceptado (evita fotos gigantes que llenen la memoria).
pub const MAX_SIDE: u32 = 12_000;

const AVIF_QUALITY: f32 = 85.0;
const AVIF_SPEED: u8 = 6;
const WEBP_QUALITY: f32 = 85.0;

#[derive(Debug, thiserror::Error)]
pub enum MediaError {
    #[error("Esta foto pesa {0} MB. El máximo es 10 MB: probá exportarla con menos calidad.")]
    TooBig(usize),
    #[error("Las fotos HEIC del iPhone todavía no se pueden subir. Abrila en Fotos, exportala como JPG y subila de nuevo.")]
    Heic,
    #[error("Las fotos AVIF todavía no se pueden subir. Guardala como JPG o PNG y subila de nuevo.")]
    Avif,
    #[error("Ese archivo no es una foto. Podés subir JPG, PNG, WebP, GIF o un logo en SVG.")]
    NotAnImage,
    #[error("No pudimos leer esta foto. Puede estar dañada: probá abrirla y guardarla de nuevo.")]
    Broken,
    #[error("Esta foto es demasiado grande ({0} × {1} píxeles). Achicala a menos de 12000 píxeles de lado.")]
    Huge(u32, u32),
    #[error("No pudimos guardar la foto. Probá de nuevo en un rato.")]
    Storage(#[from] std::io::Error),
    #[error("No pudimos convertir la foto. Probá con otra.")]
    Encode(String),
}

/// Un archivo listo para guardar, por ejemplo `960.avif`.
pub struct Variant {
    pub name: String,
    pub content_type: &'static str,
    pub bytes: Vec<u8>,
}

/// Resultado de convertir una foto.
pub struct Processed {
    pub width: u32,
    pub height: u32,
    pub variants: Vec<Variant>,
}

impl Processed {
    /// Lo que baja un visitante: la foto de [`MAIN_WIDTH`] en AVIF.
    pub fn web_bytes(&self) -> usize {
        let main = format!("{MAIN_WIDTH}.avif");
        self.variants
            .iter()
            .find(|v| v.name == main)
            .map_or(0, |v| v.bytes.len())
    }
}

/// Lo que falta de una foto cuando el tamaño más chico ya está listo: los
/// otros anchos ya achicados, esperando a codificarse.
pub struct Pending {
    sizes: Vec<(u32, DynamicImage)>,
}

impl Pending {
    /// Codifica los anchos que faltan (de menor a mayor), cada uno en su hilo.
    /// Tarda: llamarla con `spawn_blocking`.
    pub fn finish(self) -> Result<Vec<Variant>, MediaError> {
        let per_width: Vec<Result<[Variant; 2], MediaError>> = std::thread::scope(|s| {
            let jobs: Vec<_> = self
                .sizes
                .iter()
                .map(|(target, img)| s.spawn(move || encode_width(img, *target)))
                .collect();
            jobs.into_iter()
                .map(|j| {
                    j.join()
                        .unwrap_or_else(|_| Err(MediaError::Encode("falló un hilo".into())))
                })
                .collect()
        });
        let mut variants = Vec::new();
        for pair in per_width {
            variants.extend(pair?);
        }
        Ok(variants)
    }
}

/// Lee la foto, la gira, y genera ya el tamaño más chico (el que alcanza para
/// mostrarla en Mis fotos). Lo demás queda en [`Pending`], para terminarlo
/// por detrás mientras la persona ya puede usar la foto.
pub fn process_fast(bytes: &[u8]) -> Result<(Processed, Pending), MediaError> {
    if bytes.len() > MAX_BYTES {
        return Err(MediaError::TooBig(bytes.len().div_ceil(1024 * 1024)));
    }
    if is_heic(bytes) {
        return Err(MediaError::Heic);
    }
    if is_avif(bytes) {
        return Err(MediaError::Avif);
    }
    let img = if looks_like_svg(bytes) {
        svg_to_image(bytes)?
    } else {
        decode(bytes)?
    };
    let rgba = DynamicImage::ImageRgba8(img.to_rgba8());
    let (width, height) = (rgba.width(), rgba.height());

    // Cada ancho se achica a partir del anterior (más chico), no de la foto
    // entera: achicar una foto de 12 MP tres veces tardaba más de 1 s cada vez.
    // El primer paso, desde una foto muy grande, usa un filtro más rápido.
    let mut scaled: Vec<(u32, DynamicImage)> = Vec::new();
    for &target in WIDTHS.iter().rev() {
        let next = match scaled.last() {
            Some((_, prev)) => resized(prev, target, Quality::Best),
            None if width > 2 * target => resized(&rgba, target, Quality::Fast),
            None => resized(&rgba, target, Quality::Best),
        };
        scaled.push((target, next));
    }
    // `scaled` va de mayor a menor: el último es el que se codifica ahora.
    let (small_w, small) = scaled.pop().expect("hay al menos un ancho");
    let variants = encode_width(&small, small_w)?.into();
    scaled.reverse();
    Ok((
        Processed {
            width,
            height,
            variants,
        },
        Pending { sizes: scaled },
    ))
}

/// Lee la foto, la gira, y genera AVIF + WebP en cada ancho de [`WIDTHS`].
/// Tarda (AVIF es lento de codificar): llamarla con `spawn_blocking`.
pub fn process(bytes: &[u8]) -> Result<Processed, MediaError> {
    let (mut processed, pending) = process_fast(bytes)?;
    processed.variants.extend(pending.finish()?);
    Ok(processed)
}

/// Lee una foto JPG, PNG, WebP o GIF y la gira según cómo se sacó.
fn decode(bytes: &[u8]) -> Result<DynamicImage, MediaError> {
    let reader = ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|_| MediaError::NotAnImage)?;
    if reader.format().is_none() {
        return Err(MediaError::NotAnImage);
    }
    let mut decoder = reader.into_decoder().map_err(|_| MediaError::NotAnImage)?;
    let (w, h) = decoder.dimensions();
    if w > MAX_SIDE || h > MAX_SIDE {
        return Err(MediaError::Huge(w, h));
    }
    let orientation = decoder.orientation().ok();
    let mut img = DynamicImage::from_decoder(decoder).map_err(|_| MediaError::Broken)?;
    if let Some(o) = orientation {
        img.apply_orientation(o);
    }
    Ok(img)
}

/// ¿Es un SVG (un logo, casi siempre)?
fn looks_like_svg(bytes: &[u8]) -> bool {
    let head = String::from_utf8_lossy(&bytes[..bytes.len().min(1024)]).to_lowercase();
    let head = head.trim_start_matches('\u{feff}').trim_start();
    (head.starts_with("<svg") || head.starts_with("<?xml") || head.starts_with("<!--"))
        && head.contains("<svg")
}

/// Lado más largo al que se dibuja un SVG.
const SVG_SIDE: f32 = 1600.0;

/// Dibuja el SVG como imagen (con fondo transparente). Así no se guarda ni
/// se sirve el SVG original: no queda ningún código que traiga adentro, y no
/// se leen archivos ni direcciones que nombre.
fn svg_to_image(bytes: &[u8]) -> Result<DynamicImage, MediaError> {
    use resvg::{tiny_skia, usvg};
    // Las letras de los logos con texto usan las fuentes del servidor (se
    // cargan una sola vez).
    static OPTIONS: std::sync::OnceLock<usvg::Options<'static>> = std::sync::OnceLock::new();
    let options = OPTIONS.get_or_init(|| {
        let mut o = usvg::Options::default();
        o.fontdb_mut().load_system_fonts();
        // Las imágenes que el SVG trae adentro (`data:`) se dibujan; las que
        // nombra por ruta o dirección no se leen nunca: un logo no puede
        // hacer que el servidor abra sus propios archivos.
        o.image_href_resolver = usvg::ImageHrefResolver {
            resolve_data: usvg::ImageHrefResolver::default_data_resolver(),
            resolve_string: Box::new(|_, _| None),
        };
        o
    });
    let tree = usvg::Tree::from_data(bytes, options).map_err(|_| MediaError::Broken)?;
    let size = tree.size();
    let (w, h) = (size.width(), size.height());
    if !(w > 0.0 && h > 0.0) {
        return Err(MediaError::Broken);
    }
    let scale = SVG_SIDE / w.max(h);
    let (pw, ph) = (
        ((w * scale).round() as u32).max(1),
        ((h * scale).round() as u32).max(1),
    );
    let mut pixmap = tiny_skia::Pixmap::new(pw, ph).ok_or(MediaError::Broken)?;
    resvg::render(
        &tree,
        tiny_skia::Transform::from_scale(scale, scale),
        &mut pixmap.as_mut(),
    );
    // tiny-skia guarda los colores premultiplicados por la transparencia.
    let mut rgba = Vec::with_capacity((pw * ph * 4) as usize);
    for p in pixmap.pixels() {
        let c = p.demultiply();
        rgba.extend_from_slice(&[c.red(), c.green(), c.blue(), c.alpha()]);
    }
    image::RgbaImage::from_raw(pw, ph, rgba)
        .map(DynamicImage::ImageRgba8)
        .ok_or(MediaError::Broken)
}

/// Qué filtro se usa al achicar.
#[derive(Clone, Copy)]
enum Quality {
    Best,
    /// Casi el doble de rápido: para el primer paso desde una foto enorme.
    Fast,
}

/// La foto achicada a `target` de ancho (sin agrandarla).
fn resized(img: &DynamicImage, target: u32, quality: Quality) -> DynamicImage {
    let (width, height) = (img.width(), img.height());
    if width > target {
        let th = (u64::from(height) * u64::from(target) / u64::from(width)).max(1) as u32;
        let filter = match quality {
            Quality::Best => image::imageops::FilterType::Lanczos3,
            Quality::Fast => image::imageops::FilterType::CatmullRom,
        };
        img.resize_exact(target, th, filter)
    } else {
        img.clone()
    }
}

/// AVIF y WebP de una foto ya achicada a `target` de ancho.
fn encode_width(scaled: &DynamicImage, target: u32) -> Result<[Variant; 2], MediaError> {
    let buf = scaled.to_rgba8();
    // El tamaño más grande es el que más tarda: velocidad 8 pesa lo mismo
    // y se codifica un 25 % más rápido.
    let speed = if target >= 1600 { 8 } else { AVIF_SPEED };
    Ok([
        Variant {
            name: format!("{target}.avif"),
            content_type: "image/avif",
            bytes: encode_avif_with(&buf, AVIF_QUALITY, speed)?,
        },
        Variant {
            name: format!("{target}.webp"),
            content_type: "image/webp",
            bytes: encode_webp(&buf),
        },
    ])
}

/// Para medir calidades (ver `examples/calibrar.rs`).
#[doc(hidden)]
pub fn encode_avif_with(
    img: &image::RgbaImage,
    quality: f32,
    speed: u8,
) -> Result<Vec<u8>, MediaError> {
    let pixels: Vec<rgb::RGBA8> = img
        .pixels()
        .map(|p| rgb::RGBA8::new(p[0], p[1], p[2], p[3]))
        .collect();
    let opaque = pixels.iter().all(|p| p.a == 255);
    let enc = ravif::Encoder::new()
        .with_quality(quality)
        .with_alpha_quality(quality)
        .with_speed(speed);
    let frame = ravif::Img::new(
        pixels.as_slice(),
        img.width() as usize,
        img.height() as usize,
    );
    let out = if opaque {
        let rgb: Vec<rgb::RGB8> = pixels
            .iter()
            .map(|p| rgb::RGB8::new(p.r, p.g, p.b))
            .collect();
        enc.encode_rgb(ravif::Img::new(
            rgb.as_slice(),
            img.width() as usize,
            img.height() as usize,
        ))
    } else {
        enc.encode_rgba(frame)
    };
    out.map(|e| e.avif_file)
        .map_err(|e| MediaError::Encode(e.to_string()))
}

fn encode_webp(img: &image::RgbaImage) -> Vec<u8> {
    encode_webp_with(img, WEBP_QUALITY)
}

#[doc(hidden)]
pub fn encode_webp_with(img: &image::RgbaImage, quality: f32) -> Vec<u8> {
    webp::Encoder::from_rgba(img.as_raw(), img.width(), img.height())
        .encode(quality)
        .to_vec()
}

/// Las fotos HEIC/HEIF del iPhone: caja `ftyp` con marca `heic`, `heix`, `mif1`…
/// (AVIF también usa `ftyp`, con marca `avif`).
fn is_heic(bytes: &[u8]) -> bool {
    bytes.len() >= 12
        && &bytes[4..8] == b"ftyp"
        && matches!(
            &bytes[8..12],
            b"heic" | b"heix" | b"hevc" | b"heim" | b"heis" | b"mif1" | b"msf1"
        )
}

/// Fotos AVIF (caja `ftyp` con marca `avif` o `avis`): sin decodificador
/// todavía. Antes caían en "no es una foto", que confundía.
fn is_avif(bytes: &[u8]) -> bool {
    bytes.len() >= 12 && &bytes[4..8] == b"ftyp" && matches!(&bytes[8..12], b"avif" | b"avis")
}

/// Dónde se guardan los archivos: una carpeta del servidor o un bucket S3
/// (Cloudflare R2). MarketMaker los entrega siempre en `/media/…`, así el
/// bucket no necesita ser público.
#[derive(Clone, Debug)]
pub enum Store {
    /// Carpeta local (`MEDIA_DIR`, por defecto `media/`).
    Local { root: PathBuf },
    /// Bucket S3 compatible, como Cloudflare R2.
    S3(S3Store),
}

#[derive(Clone, Debug)]
pub struct S3Store {
    bucket: rusty_s3::Bucket,
    credentials: rusty_s3::Credentials,
    client: reqwest::Client,
}

/// Cuánto vale cada firma de un pedido al bucket.
const SIGN_FOR: std::time::Duration = std::time::Duration::from_secs(120);

impl Store {
    /// Cloudflare R2 si están `R2_ACCOUNT_ID`, `R2_BUCKET`,
    /// `R2_ACCESS_KEY_ID` y `R2_SECRET_ACCESS_KEY`; si no, la carpeta
    /// `MEDIA_DIR` (o `media/`). `S3_ENDPOINT` reemplaza la dirección de R2
    /// para usar otro servicio compatible.
    pub fn from_env() -> Result<Self, String> {
        let var = |k: &str| {
            std::env::var(k)
                .ok()
                .map(|v| v.trim().to_string())
                .filter(|v| !v.is_empty())
        };
        let Some(bucket) = var("R2_BUCKET") else {
            let root = std::env::var_os("MEDIA_DIR")
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("media"));
            return Ok(Store::Local { root });
        };
        let missing = |k: &str| format!("falta {k} en .env (para guardar las fotos en R2)");
        let endpoint = match var("S3_ENDPOINT") {
            Some(e) => e,
            None => format!(
                "https://{}.r2.cloudflarestorage.com",
                var("R2_ACCOUNT_ID").ok_or_else(|| missing("R2_ACCOUNT_ID"))?
            ),
        };
        let key = var("R2_ACCESS_KEY_ID").ok_or_else(|| missing("R2_ACCESS_KEY_ID"))?;
        let secret = var("R2_SECRET_ACCESS_KEY").ok_or_else(|| missing("R2_SECRET_ACCESS_KEY"))?;
        let endpoint =
            url::Url::parse(&endpoint).map_err(|e| format!("dirección de R2 inválida: {e}"))?;
        let bucket = rusty_s3::Bucket::new(endpoint, rusty_s3::UrlStyle::Path, bucket, "auto")
            .map_err(|e| format!("bucket de R2 inválido: {e}"))?;
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| e.to_string())?;
        Ok(Store::S3(S3Store {
            bucket,
            credentials: rusty_s3::Credentials::new(key, secret),
            client,
        }))
    }

    /// Pasa los archivos de una foto de la carpeta `from` a `to`: copia
    /// todos y recién después borra los viejos. Si falta alguno, no borra
    /// nada y devuelve error.
    pub async fn move_photo(&self, from: &str, to: &str) -> Result<u64, MediaError> {
        let mut files = Vec::new();
        for w in WIDTHS {
            for ext in ["avif", "webp"] {
                let name = format!("{w}.{ext}");
                let bytes = self.get(from, &name).await?.ok_or_else(|| {
                    MediaError::Storage(std::io::Error::other(format!("falta {from}/{name}")))
                })?;
                let content_type = content_type(&name).unwrap_or("application/octet-stream");
                files.push(Variant {
                    name,
                    content_type,
                    bytes,
                });
            }
        }
        let total = files.iter().map(|f| f.bytes.len() as u64).sum();
        self.put_all(to, &files).await?;
        self.delete_all(from).await?;
        Ok(total)
    }

    /// Las carpetas de fotos que hay en el almacén (`<foto>` o
    /// `usuarios/<dueño>/fotos/<foto>`), con la fecha UTC del archivo más
    /// nuevo (`2026-11-15T21:00:00`). Lo que no tiene esa forma no aparece.
    pub async fn list_folders(&self) -> Result<Vec<StoredFolder>, MediaError> {
        let mut found: std::collections::BTreeMap<String, String> = Default::default();
        let mut note = |key: &str, modified: String| {
            if let Some((folder, file)) = key.rsplit_once('/') {
                if content_type(file).is_some() && parse_folder(folder).is_some() {
                    let e = found.entry(folder.to_string()).or_default();
                    if modified > *e {
                        *e = modified;
                    }
                }
            }
        };
        match self {
            Store::Local { root } => {
                let mut stack = vec![root.clone()];
                while let Some(dir) = stack.pop() {
                    let mut rd = match tokio::fs::read_dir(&dir).await {
                        Ok(rd) => rd,
                        Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
                        Err(e) => return Err(e.into()),
                    };
                    while let Some(entry) = rd.next_entry().await? {
                        let meta = entry.metadata().await?;
                        let path = entry.path();
                        if meta.is_dir() {
                            stack.push(path);
                        } else if let Ok(rel) = path.strip_prefix(root) {
                            let key = rel.to_string_lossy().replace('\\', "/");
                            let secs = meta
                                .modified()
                                .ok()
                                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                                .map_or(0, |d| d.as_secs());
                            note(&key, utc_iso(secs));
                        }
                    }
                }
            }
            Store::S3(s) => {
                let mut token: Option<String> = None;
                loop {
                    let mut action =
                        rusty_s3::actions::ListObjectsV2::new(&s.bucket, Some(&s.credentials));
                    if let Some(t) = &token {
                        action.with_continuation_token(t.clone());
                    }
                    let url = action.sign(SIGN_FOR);
                    let res = s.client.get(url).send().await.map_err(s3_err)?;
                    if !res.status().is_success() {
                        return Err(s3_status(res.status()));
                    }
                    let text = res.text().await.map_err(s3_err)?;
                    let list = rusty_s3::actions::ListObjectsV2::parse_response(&text)
                        .map_err(|e| MediaError::Storage(std::io::Error::other(e.to_string())))?;
                    for obj in &list.contents {
                        let modified = obj.last_modified.clone().unwrap_or_default();
                        note(&obj.key, modified.chars().take(19).collect());
                    }
                    match list.next_continuation_token {
                        Some(t) => token = Some(t),
                        None => break,
                    }
                }
            }
        }
        Ok(found
            .into_iter()
            .map(|(folder, newest)| StoredFolder { folder, newest })
            .collect())
    }

    /// Para el registro al arrancar: dónde se guardan las fotos.
    pub fn describe(&self) -> String {
        match self {
            Store::Local { root } => format!("la carpeta {}", root.display()),
            Store::S3(s) => format!(
                "el bucket {} ({})",
                s.bucket.name(),
                s.bucket.object_url("").host_str().unwrap_or("")
            ),
        }
    }

    /// Guarda los archivos de una foto en la carpeta `folder` (ver
    /// [`photo_folder`]).
    pub async fn put_all(&self, folder: &str, variants: &[Variant]) -> Result<(), MediaError> {
        match self {
            Store::Local { root } => {
                let dir = root.join(folder);
                tokio::fs::create_dir_all(&dir).await?;
                for v in variants {
                    tokio::fs::write(dir.join(&v.name), &v.bytes).await?;
                }
                Ok(())
            }
            Store::S3(s) => {
                // Los 6 archivos viajan a la vez; si uno falla se corta el resto.
                let mut jobs = tokio::task::JoinSet::new();
                for v in variants {
                    let key = format!("{folder}/{}", v.name);
                    let url =
                        rusty_s3::actions::PutObject::new(&s.bucket, Some(&s.credentials), &key)
                            .sign(SIGN_FOR);
                    let (client, ct, body) = (s.client.clone(), v.content_type, v.bytes.clone());
                    jobs.spawn(async move {
                        let res = client
                            .put(url)
                            .header("content-type", ct)
                            .body(body)
                            .send()
                            .await
                            .map_err(s3_err)?;
                        if res.status().is_success() {
                            Ok(())
                        } else {
                            Err(s3_status(res.status()))
                        }
                    });
                }
                while let Some(done) = jobs.join_next().await {
                    done.map_err(|e| MediaError::Storage(std::io::Error::other(e.to_string())))??;
                }
                Ok(())
            }
        }
    }

    /// El archivo `file` de la foto guardada en `folder` (`None` si no
    /// existe). Quien llama valida `file` y arma `folder` con ids: acá no se
    /// arman rutas con texto ajeno.
    pub async fn get(&self, folder: &str, file: &str) -> Result<Option<Vec<u8>>, MediaError> {
        match self {
            Store::Local { root } => match tokio::fs::read(root.join(folder).join(file)).await {
                Ok(b) => Ok(Some(b)),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
                Err(e) => Err(e.into()),
            },
            Store::S3(s) => {
                let key = format!("{folder}/{file}");
                let url = rusty_s3::actions::GetObject::new(&s.bucket, Some(&s.credentials), &key)
                    .sign(SIGN_FOR);
                let res = s.client.get(url).send().await.map_err(s3_err)?;
                match res.status() {
                    st if st.is_success() => Ok(Some(res.bytes().await.map_err(s3_err)?.to_vec())),
                    reqwest::StatusCode::NOT_FOUND => Ok(None),
                    st => Err(s3_status(st)),
                }
            }
        }
    }

    /// Borra todos los archivos de la foto guardada en `folder`.
    pub async fn delete_all(&self, folder: &str) -> Result<(), MediaError> {
        match self {
            Store::Local { root } => match tokio::fs::remove_dir_all(root.join(folder)).await {
                Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(e.into()),
                _ => Ok(()),
            },
            Store::S3(s) => {
                for w in WIDTHS {
                    for ext in ["avif", "webp"] {
                        let key = format!("{folder}/{w}.{ext}");
                        let url = rusty_s3::actions::DeleteObject::new(
                            &s.bucket,
                            Some(&s.credentials),
                            &key,
                        )
                        .sign(SIGN_FOR);
                        let res = s.client.delete(url).send().await.map_err(s3_err)?;
                        if !res.status().is_success()
                            && res.status() != reqwest::StatusCode::NOT_FOUND
                        {
                            return Err(s3_status(res.status()));
                        }
                    }
                }
                Ok(())
            }
        }
    }
}

fn s3_err(e: reqwest::Error) -> MediaError {
    MediaError::Storage(std::io::Error::other(e))
}

fn s3_status(st: reqwest::StatusCode) -> MediaError {
    MediaError::Storage(std::io::Error::other(format!("el bucket respondió {st}")))
}

/// Si `file` es uno de los archivos que se guardan por foto (`960.avif`…),
/// su tipo de contenido.
pub fn content_type(file: &str) -> Option<&'static str> {
    let (w, ext) = file.split_once('.')?;
    let width: u32 = w.parse().ok()?;
    if !WIDTHS.contains(&width) {
        return None;
    }
    match ext {
        "avif" => Some("image/avif"),
        "webp" => Some("image/webp"),
        _ => None,
    }
}

/// Una carpeta de foto en el almacén (ver [`Store::list_folders`]).
#[derive(Debug, Clone, PartialEq)]
pub struct StoredFolder {
    pub folder: String,
    /// Fecha UTC del archivo más nuevo, `2026-11-15T21:00:00`.
    pub newest: String,
}

/// El id de la foto de una carpeta (`<foto>` o
/// `usuarios/<dueño>/fotos/<foto>`); `None` si la carpeta no es de una foto.
pub fn parse_folder(folder: &str) -> Option<&str> {
    let is_id = |s: &str| s.len() == 36 && s.chars().all(|c| c.is_ascii_hexdigit() || c == '-');
    let parts: Vec<&str> = folder.split('/').collect();
    match parts.as_slice() {
        [id] if is_id(id) => Some(id),
        ["usuarios", owner, "fotos", id] if is_id(owner) && is_id(id) => Some(id),
        _ => None,
    }
}

/// Segundos desde 1970 como fecha UTC `2026-11-15T21:00:00`, para comparar
/// con las fechas que da el bucket.
pub fn utc_iso(secs: u64) -> String {
    let days = (secs / 86_400) as i64;
    let rem = secs % 86_400;
    // Días a fecha civil (algoritmo de Howard Hinnant).
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!(
        "{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}",
        rem / 3600,
        rem % 3600 / 60,
        rem % 60
    )
}

/// La carpeta de una foto en el almacén: `usuarios/<dueño>/fotos/<foto>`,
/// así en el panel de R2 se ven ordenadas por usuario. Las direcciones
/// públicas (`/media/<foto>/960.webp`) no cambian.
pub fn photo_folder(owner: &str, id: &str) -> String {
    format!("usuarios/{owner}/fotos/{id}")
}

/// La dirección pública del archivo principal de la foto `id`.
pub fn main_url(id: &str) -> String {
    format!("/media/{id}/{MAIN_WIDTH}.webp")
}

#[cfg(test)]
mod tests {
    #[test]
    fn utc_dates_and_photo_folders() {
        assert_eq!(super::utc_iso(0), "1970-01-01T00:00:00");
        assert_eq!(super::utc_iso(1_794_776_400), "2026-11-15T21:00:00");
        assert_eq!(super::utc_iso(951_782_400), "2000-02-29T00:00:00");
        let id = "0b6f6c1e-8d3a-4f0e-9a51-3f1a2b3c4d5e";
        assert_eq!(super::parse_folder(id), Some(id));
        let f = super::photo_folder("7c0e2b9a-1111-4222-8333-444455556666", id);
        assert_eq!(super::parse_folder(&f), Some(id));
        assert_eq!(super::parse_folder("otra/cosa"), None);
        assert_eq!(super::parse_folder("usuarios/x/fotos/y"), None);
    }

    use super::*;

    fn png(w: u32, h: u32) -> Vec<u8> {
        let img = image::RgbImage::from_fn(w, h, |x, y| {
            image::Rgb([(x % 256) as u8, (y % 256) as u8, 128])
        });
        let mut out = Vec::new();
        DynamicImage::ImageRgb8(img)
            .write_to(&mut Cursor::new(&mut out), image::ImageFormat::Png)
            .unwrap();
        out
    }

    #[test]
    fn an_svg_logo_becomes_a_picture_without_its_code() {
        let svg = br##"<?xml version="1.0"?>
<svg xmlns="http://www.w3.org/2000/svg" width="40" height="20" viewBox="0 0 40 20">
  <script>alert(1)</script>
  <image href="file:///etc/passwd" width="10" height="10"/>
  <rect width="40" height="20" fill="#16a34a"/>
</svg>"##;
        let p = process(svg).unwrap();
        assert_eq!((p.width, p.height), (1600, 800));
        assert!(p.variants.iter().all(|v| v.content_type != "image/svg+xml"));
        assert!(matches!(process(b"<svg"), Err(MediaError::Broken)));
    }

    #[test]
    fn converts_to_all_sizes_without_upscaling() {
        let p = process(&png(1200, 600)).unwrap();
        assert_eq!((p.width, p.height), (1200, 600));
        let names: Vec<_> = p.variants.iter().map(|v| v.name.as_str()).collect();
        assert_eq!(
            names,
            [
                "480.avif",
                "480.webp",
                "960.avif",
                "960.webp",
                "1600.avif",
                "1600.webp"
            ]
        );
        assert!(p.variants.iter().all(|v| !v.bytes.is_empty()));
        // 1600 no se agranda: queda en 1200.
        let big = image::load_from_memory(&p.variants[5].bytes).unwrap();
        assert_eq!(big.width(), 1200);
        let small = image::load_from_memory(&p.variants[1].bytes).unwrap();
        assert_eq!((small.width(), small.height()), (480, 240));
    }

    #[test]
    fn only_known_files_are_served() {
        assert_eq!(content_type("960.avif"), Some("image/avif"));
        assert_eq!(content_type("480.webp"), Some("image/webp"));
        assert_eq!(content_type("500.webp"), None);
        assert_eq!(content_type("../x.webp"), None);
        assert_eq!(content_type("960.png"), None);
    }

    #[test]
    fn rejects_non_images_and_heic() {
        assert!(matches!(
            process(b"hola, no soy una foto"),
            Err(MediaError::NotAnImage)
        ));
        let mut heic = vec![0, 0, 0, 24];
        heic.extend_from_slice(b"ftypheic");
        heic.extend_from_slice(&[0; 16]);
        assert!(matches!(process(&heic), Err(MediaError::Heic)));
        assert!(matches!(
            process(&vec![0; MAX_BYTES + 1]),
            Err(MediaError::TooBig(11))
        ));
    }
}
