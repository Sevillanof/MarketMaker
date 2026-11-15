//! Mis fotos: rutas HTTP para listar, subir y borrar las fotos del dueño.
//!
//! Son rutas comunes de Axum (no server functions) porque subir un archivo
//! es un formulario `multipart`. Cada una revisa la sesión igual que
//! [`crate::require_editor`].

use axum::extract::{DefaultBodyLimit, Multipart, Path, State};
use axum::http::{header, HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{delete, get};
use axum::{Json, Router};
use domain::MediaItem;
use uuid::Uuid;

use crate::auth::COOKIE;

/// Lo que necesitan las rutas de fotos.
#[derive(Clone)]
pub struct MediaState {
    pub pool: db::Pool,
    pub store: media::Store,
}

/// `GET/POST /api/media`, `DELETE /api/media/{id}` y los archivos en
/// `GET /media/{id}/{file}`. Los archivos los entrega MarketMaker aunque el
/// almacén esté en otro lado (por ejemplo MinIO en una red privada).
pub fn media_routes<S: Clone + Send + Sync + 'static>(state: MediaState) -> Router<S> {
    Router::new()
        .route("/api/media", get(list).post(upload))
        .route("/api/media/{id}", delete(remove))
        .route("/media/{id}/{file}", get(serve))
        // Un poco más que el máximo, para responder "pesa demasiado" en
        // palabras en lugar de cortar la conexión.
        .layer(DefaultBodyLimit::max(media::MAX_BYTES + 512 * 1024))
        .with_state(state)
}

fn fail(status: StatusCode, msg: impl Into<String>) -> Response {
    (
        status,
        [(header::CONTENT_TYPE, "text/plain; charset=utf-8")],
        msg.into(),
    )
        .into_response()
}

/// La persona que entró, o la respuesta de error para devolver.
#[allow(clippy::result_large_err)]
async fn owner(state: &MediaState, headers: &HeaderMap) -> Result<db::auth::SessionUser, Response> {
    let token = headers
        .get_all(header::COOKIE)
        .iter()
        .filter_map(|v| v.to_str().ok())
        .flat_map(|v| v.split(';'))
        .filter_map(|c| c.trim().split_once('='))
        .find(|(name, _)| *name == COOKIE)
        .map(|(_, v)| v.to_string())
        .filter(|v| !v.is_empty() && v.len() <= 128);
    let denied = || {
        fail(
            StatusCode::UNAUTHORIZED,
            "Tenés que entrar con tu cuenta para usar tus fotos. Abrí /entrar en otra pestaña y volvé a intentar.",
        )
    };
    let Some(token) = token else {
        return Err(denied());
    };
    let user = db::auth::session_user(&state.pool, &token)
        .await
        .map_err(|_| {
            fail(
                StatusCode::INTERNAL_SERVER_ERROR,
                "No pudimos revisar tu sesión. Probá de nuevo.",
            )
        })?;
    user.filter(|u| crate::auth::is_allowed(&u.email))
        .ok_or_else(denied)
}

async fn list(State(state): State<MediaState>, headers: HeaderMap) -> Response {
    let user = match owner(&state, &headers).await {
        Ok(u) => u,
        Err(r) => return r,
    };
    match db::media::list(&state.pool, user.id).await {
        Ok(items) => Json(items).into_response(),
        Err(_) => fail(
            StatusCode::INTERNAL_SERVER_ERROR,
            "No pudimos traer tus fotos. Probá de nuevo.",
        ),
    }
}

/// Recibe una foto en el campo `file`, la convierte y la agrega a Mis fotos.
async fn upload(
    State(state): State<MediaState>,
    headers: HeaderMap,
    mut form: Multipart,
) -> Response {
    let user = match owner(&state, &headers).await {
        Ok(u) => u,
        Err(r) => return r,
    };
    let quota = match db::media::quota(&state.pool, user.id).await {
        Ok(q) => q,
        Err(_) => {
            return fail(
                StatusCode::INTERNAL_SERVER_ERROR,
                "No pudimos guardar la foto. Probá de nuevo.",
            )
        }
    };
    if let Err(msg) = quota.room_for(0) {
        return fail(StatusCode::CONFLICT, msg);
    }

    let too_big = || {
        fail(
            StatusCode::PAYLOAD_TOO_LARGE,
            "Esta foto pesa más de 10 MB, que es el máximo. Probá exportarla con menos calidad.",
        )
    };
    let mut file = None;
    loop {
        match form.next_field().await {
            Ok(Some(field)) if field.name() == Some("file") => {
                let name = domain::media_name(field.file_name().unwrap_or("Foto"));
                match field.bytes().await {
                    Ok(bytes) => file = Some((name, bytes)),
                    Err(_) => return too_big(),
                }
            }
            Ok(Some(_)) => {}
            Ok(None) => break,
            Err(_) => return too_big(),
        }
    }
    let Some((name, bytes)) = file else {
        return fail(
            StatusCode::BAD_REQUEST,
            "No llegó ninguna foto. Elegí una y probá de nuevo.",
        );
    };

    let size = bytes.len();
    // La misma foto que ya subió: se usa esa, sin convertir ni guardar otra.
    let sha256 = db::media::fingerprint(&bytes);
    match db::media::find_by_hash(&state.pool, user.id, &sha256).await {
        Ok(Some(existing)) => {
            return Json(MediaItem {
                duplicate: true,
                ..existing
            })
            .into_response();
        }
        Ok(None) => {}
        Err(_) => {
            return fail(
                StatusCode::INTERNAL_SERVER_ERROR,
                "No pudimos guardar la foto. Probá de nuevo.",
            )
        }
    }
    let processed = match tokio::task::spawn_blocking(move || media::process(&bytes)).await {
        Ok(Ok(p)) => p,
        Ok(Err(e)) => return fail(StatusCode::UNPROCESSABLE_ENTITY, e.to_string()),
        Err(_) => {
            return fail(
                StatusCode::INTERNAL_SERVER_ERROR,
                "No pudimos convertir la foto. Probá con otra.",
            )
        }
    };
    let stored_bytes: usize = processed.variants.iter().map(|v| v.bytes.len()).sum();
    if let Err(msg) = quota.room_for(stored_bytes as u64) {
        return fail(StatusCode::CONFLICT, msg);
    }
    let id = Uuid::new_v4();
    let folder = db::media::media_folder(user.id, id);
    if let Err(e) = state.store.put_all(&folder, &processed.variants).await {
        return fail(StatusCode::INTERNAL_SERVER_ERROR, e.to_string());
    }
    let new = db::media::NewMedia {
        id,
        owner_id: user.id,
        name: &name,
        width: processed.width,
        height: processed.height,
        bytes: size,
        web_bytes: processed.web_bytes(),
        stored_bytes,
        sha256: &sha256,
    };
    match db::media::insert(&state.pool, &new).await {
        Ok(item) => (StatusCode::CREATED, Json(item)).into_response(),
        Err(_) => {
            _ = state.store.delete_all(&folder).await;
            fail(
                StatusCode::INTERNAL_SERVER_ERROR,
                "No pudimos guardar la foto. Probá de nuevo.",
            )
        }
    }
}

async fn remove(
    State(state): State<MediaState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> Response {
    let user = match owner(&state, &headers).await {
        Ok(u) => u,
        Err(r) => return r,
    };
    match db::media::delete(&state.pool, user.id, id).await {
        Ok(Some(folder)) => {
            _ = state.store.delete_all(&folder).await;
            StatusCode::NO_CONTENT.into_response()
        }
        Ok(None) => fail(StatusCode::NOT_FOUND, "Esa foto ya no está en Mis fotos."),
        Err(_) => fail(
            StatusCode::INTERNAL_SERVER_ERROR,
            "No pudimos borrar la foto. Probá de nuevo.",
        ),
    }
}

/// Un archivo de una foto. Cada foto tiene su id y nunca cambia, así que el
/// navegador puede guardarla un año.
async fn serve(
    State(state): State<MediaState>,
    Path((id, file)): Path<(Uuid, String)>,
) -> Response {
    let Some(content_type) = media::content_type(&file) else {
        return StatusCode::NOT_FOUND.into_response();
    };
    let (folder, other) = match db::media::locate(&state.pool, id).await {
        Ok(Some(f)) => f,
        Ok(None) => return StatusCode::NOT_FOUND.into_response(),
        Err(_) => return StatusCode::SERVICE_UNAVAILABLE.into_response(),
    };
    // Si la foto se está pasando a la carpeta del usuario justo ahora, puede
    // estar en la otra.
    let found = match state.store.get(&folder, &file).await {
        Ok(None) => state.store.get(&other, &file).await,
        r => r,
    };
    match found {
        Ok(Some(bytes)) => (
            [
                (header::CONTENT_TYPE, content_type),
                (header::CACHE_CONTROL, "public, max-age=31536000, immutable"),
            ],
            bytes,
        )
            .into_response(),
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Err(_) => StatusCode::SERVICE_UNAVAILABLE.into_response(),
    }
}
