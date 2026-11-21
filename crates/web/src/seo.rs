//! Para los buscadores: `robots.txt`, el mapa del sitio (`sitemap.xml`) de la
//! plataforma y de cada tienda publicada, y `www.<plataforma>` que lleva a la
//! dirección sin `www`.

#[cfg(feature = "ssr")]
pub use http_routes::{routes, www_redirect};

#[cfg(feature = "ssr")]
mod http_routes {
    use axum::extract::{Request, State};
    use axum::http::{header, HeaderMap, StatusCode};
    use axum::middleware::Next;
    use axum::response::{IntoResponse, Redirect, Response};
    use axum::routing::get;
    use axum::Router;

    /// `GET /robots.txt` y `GET /sitemap.xml`, según la dirección pedida.
    pub fn routes<S: Clone + Send + Sync + 'static>(pool: db::Pool) -> Router<S> {
        Router::new()
            .route("/robots.txt", get(robots))
            .route("/sitemap.xml", get(sitemap))
            .with_state(pool)
    }

    /// `www.mkr.com.ar/algo` → `mkr.com.ar/algo` (permanente), para que
    /// Google no vea dos copias de la misma página.
    pub async fn www_redirect(req: Request, next: Next) -> Response {
        let target = host(req.headers())
            .strip_prefix("www.")
            .filter(|rest| *rest == platform())
            .map(|rest| {
                let path = req.uri().path_and_query().map_or("/", |p| p.as_str());
                format!("{}://{rest}{path}", scheme())
            });
        match target {
            Some(url) => Redirect::permanent(&url).into_response(),
            None => next.run(req).await,
        }
    }

    fn host(headers: &HeaderMap) -> String {
        headers
            .get(header::HOST)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .trim()
            .to_lowercase()
    }

    /// `mkr.com.ar` (con puerto si `APP_URL` lo tiene).
    fn platform() -> String {
        let app = crate::email::app_url();
        app.split_once("://")
            .map_or(app.as_str(), |(_, h)| h)
            .to_lowercase()
    }

    fn scheme() -> &'static str {
        if crate::email::app_url().starts_with("https://") {
            "https"
        } else {
            "http"
        }
    }

    fn text(body: String, kind: &'static str) -> Response {
        (
            StatusCode::OK,
            [
                (header::CONTENT_TYPE, kind),
                (header::CACHE_CONTROL, "public, max-age=3600"),
            ],
            body,
        )
            .into_response()
    }

    async fn robots(headers: HeaderMap) -> Response {
        let host = host(&headers);
        let private = match domain::site_from_host(&host) {
            domain::Site::Platform => {
                "Disallow: /tienda/\nDisallow: /editor\nDisallow: /cuenta\nDisallow: /mis-fotos\nDisallow: /admin/\nDisallow: /entrar/pase\nDisallow: /dev/\n"
            }
            domain::Site::Store(_) => {
                "Disallow: /comprar\nDisallow: /pedido/\nDisallow: /mi-cuenta\nDisallow: /vista-previa\nDisallow: /editor\n"
            }
        };
        text(
            format!(
                "User-agent: *\n{private}\nSitemap: {}://{host}/sitemap.xml\n",
                scheme()
            ),
            "text/plain; charset=utf-8",
        )
    }

    fn xml_escape(s: &str) -> String {
        s.replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('"', "&quot;")
            .replace('\'', "&apos;")
    }

    fn urlset(urls: &[String]) -> String {
        let mut out = String::from(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\">\n",
        );
        for u in urls {
            out.push_str("  <url><loc>");
            out.push_str(&xml_escape(u));
            out.push_str("</loc></url>\n");
        }
        out.push_str("</urlset>\n");
        out
    }

    async fn sitemap(State(pool): State<db::Pool>, headers: HeaderMap) -> Response {
        let host = host(&headers);
        let base = format!("{}://{host}", scheme());
        let urls = match domain::site_from_host(&host) {
            domain::Site::Platform => ["/", "/terminos", "/privacidad"]
                .iter()
                .map(|p| format!("{base}{p}"))
                .collect(),
            domain::Site::Store(slug) => match store_urls(&pool, &slug, &base).await {
                Some(u) => u,
                None => return StatusCode::NOT_FOUND.into_response(),
            },
        };
        text(urlset(&urls), "application/xml; charset=utf-8")
    }

    /// Las páginas de una tienda publicada y la ficha de cada producto que se
    /// ve (`/?ver=<id>`, el mismo enlace de "Compartir"). `None` si la tienda
    /// no existe o no está publicada.
    async fn store_urls(pool: &db::Pool, slug: &str, base: &str) -> Option<Vec<String>> {
        let tenant = db::get_tenant_by_slug(pool, slug).await.ok()??;
        let page = db::get_published_page(pool, tenant.id, "home")
            .await
            .ok()??;
        if page.published == 0 {
            return None;
        }
        let doc = domain::parse_doc(page.doc?).ok()?;
        let mut urls = vec![format!("{base}/")];
        urls.extend(doc.pages.iter().map(|p| format!("{base}{}", p.path)));
        urls.push(format!("{base}/arrepentimiento"));
        let products = db::products::list_for_buyers(pool, tenant.id)
            .await
            .unwrap_or_default();
        urls.extend(
            products
                .iter()
                .filter(|p| p.visible)
                .map(|p| format!("{base}/?ver={}", p.id)),
        );
        Some(urls)
    }
}
