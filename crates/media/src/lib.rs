//! Fotos del dueño: se convierten a AVIF y WebP en [`WIDTHS`] y se guardan
//! en un [`Store`]. Solo servidor.
//!
//! Al convertir se gira la foto según cómo la sacó el celular y se pierden
//! los datos ocultos del archivo original (incluida la ubicación GPS).

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

const AVIF_QUALITY: f32 = 62.0;
const AVIF_SPEED: u8 = 8;
const WEBP_QUALITY: f32 = 80.0;

#[derive(Debug, thiserror::Error)]
pub enum MediaError {
    #[error("Esta foto pesa {0} MB. El máximo es 10 MB: probá exportarla con menos calidad.")]
    TooBig(usize),
    #[error("Las fotos HEIC del iPhone todavía no se pueden subir. Abrila en Fotos, exportala como JPG y subila de nuevo.")]
    Heic,
    #[error("Ese archivo no es una foto. Podés subir JPG, PNG, WebP o GIF.")]
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

/// Lee la foto, la gira, y genera AVIF + WebP en cada ancho de [`WIDTHS`].
/// Tarda (AVIF es lento de codificar): llamarla con `spawn_blocking`.
pub fn process(bytes: &[u8]) -> Result<Processed, MediaError> {
    if bytes.len() > MAX_BYTES {
        return Err(MediaError::TooBig(bytes.len().div_ceil(1024 * 1024)));
    }
    if is_heic(bytes) {
        return Err(MediaError::Heic);
    }
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
    let rgba = DynamicImage::ImageRgba8(img.to_rgba8());
    let (width, height) = (rgba.width(), rgba.height());

    // Los tres anchos se codifican a la vez, cada uno en su hilo.
    let per_width: Vec<Result<[Variant; 2], MediaError>> = std::thread::scope(|s| {
        let rgba = &rgba;
        let jobs: Vec<_> = WIDTHS
            .iter()
            .map(|&target| s.spawn(move || encode_width(rgba, target)))
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
    Ok(Processed {
        width,
        height,
        variants,
    })
}

/// AVIF y WebP de la foto achicada a `target` de ancho (sin agrandarla).
fn encode_width(rgba: &DynamicImage, target: u32) -> Result<[Variant; 2], MediaError> {
    let (width, height) = (rgba.width(), rgba.height());
    let scaled = if width > target {
        let th = (u64::from(height) * u64::from(target) / u64::from(width)).max(1) as u32;
        rgba.resize_exact(target, th, image::imageops::FilterType::Lanczos3)
    } else {
        rgba.clone()
    };
    let buf = scaled.to_rgba8();
    Ok([
        Variant {
            name: format!("{target}.avif"),
            content_type: "image/avif",
            bytes: encode_avif(&buf)?,
        },
        Variant {
            name: format!("{target}.webp"),
            content_type: "image/webp",
            bytes: encode_webp(&buf),
        },
    ])
}

fn encode_avif(img: &image::RgbaImage) -> Result<Vec<u8>, MediaError> {
    let pixels: Vec<rgb::RGBA8> = img
        .pixels()
        .map(|p| rgb::RGBA8::new(p[0], p[1], p[2], p[3]))
        .collect();
    let opaque = pixels.iter().all(|p| p.a == 255);
    let enc = ravif::Encoder::new()
        .with_quality(AVIF_QUALITY)
        .with_alpha_quality(AVIF_QUALITY)
        .with_speed(AVIF_SPEED);
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
    webp::Encoder::from_rgba(img.as_raw(), img.width(), img.height())
        .encode(WEBP_QUALITY)
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

    /// Guarda los archivos de la foto `id`.
    pub async fn put_all(&self, id: &str, variants: &[Variant]) -> Result<(), MediaError> {
        match self {
            Store::Local { root } => {
                let dir = root.join(id);
                tokio::fs::create_dir_all(&dir).await?;
                for v in variants {
                    tokio::fs::write(dir.join(&v.name), &v.bytes).await?;
                }
                Ok(())
            }
            Store::S3(s) => {
                for v in variants {
                    let key = format!("{id}/{}", v.name);
                    let url =
                        rusty_s3::actions::PutObject::new(&s.bucket, Some(&s.credentials), &key)
                            .sign(SIGN_FOR);
                    let res = s
                        .client
                        .put(url)
                        .header("content-type", v.content_type)
                        .body(v.bytes.clone())
                        .send()
                        .await
                        .map_err(s3_err)?;
                    if !res.status().is_success() {
                        return Err(s3_status(res.status()));
                    }
                }
                Ok(())
            }
        }
    }

    /// El archivo `file` de la foto `id` (`None` si no existe). Quien llama
    /// valida `id` y `file` antes: acá no se arman rutas con texto ajeno.
    pub async fn get(&self, id: &str, file: &str) -> Result<Option<Vec<u8>>, MediaError> {
        match self {
            Store::Local { root } => match tokio::fs::read(root.join(id).join(file)).await {
                Ok(b) => Ok(Some(b)),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
                Err(e) => Err(e.into()),
            },
            Store::S3(s) => {
                let key = format!("{id}/{file}");
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

    /// Borra todos los archivos de la foto `id`.
    pub async fn delete_all(&self, id: &str) -> Result<(), MediaError> {
        match self {
            Store::Local { root } => match tokio::fs::remove_dir_all(root.join(id)).await {
                Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(e.into()),
                _ => Ok(()),
            },
            Store::S3(s) => {
                for w in WIDTHS {
                    for ext in ["avif", "webp"] {
                        let key = format!("{id}/{w}.{ext}");
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

/// La dirección pública del archivo principal de la foto `id`.
pub fn main_url(id: &str) -> String {
    format!("/media/{id}/{MAIN_WIDTH}.webp")
}

#[cfg(test)]
mod tests {
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
