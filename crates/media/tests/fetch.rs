//! Traer fotos de enlaces contra un servidor de prueba en esta máquina.

use tokio::io::{AsyncReadExt, AsyncWriteExt};

fn png() -> Vec<u8> {
    let img = image::RgbImage::from_fn(40, 30, |x, _| image::Rgb([x as u8, 80, 160]));
    let mut out = Vec::new();
    image::DynamicImage::ImageRgb8(img)
        .write_to(&mut std::io::Cursor::new(&mut out), image::ImageFormat::Png)
        .unwrap();
    out
}

/// Un servidor HTTP mínimo: `/foto.png`, `/pagina`, `/vuelta` (redirige a
/// la foto), `/texto` y `/privada` (403).
async fn server() -> String {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        loop {
            let (mut sock, _) = listener.accept().await.unwrap();
            tokio::spawn(async move {
                let mut buf = vec![0; 4096];
                let n = sock.read(&mut buf).await.unwrap_or(0);
                let req = String::from_utf8_lossy(&buf[..n]).to_string();
                let path = req.split_whitespace().nth(1).unwrap_or("/").to_string();
                let (head, body): (String, Vec<u8>) = match path.as_str() {
                    "/foto.png" => ("200 OK\r\nContent-Type: image/png".into(), png()),
                    "/pagina" => (
                        "200 OK\r\nContent-Type: text/html".into(),
                        br#"<html><head><meta property="og:image" content="/foto.png"></head></html>"#.to_vec(),
                    ),
                    "/vuelta" => ("302 Found\r\nLocation: /foto.png".into(), Vec::new()),
                    "/texto" => ("200 OK\r\nContent-Type: text/plain".into(), b"hola".to_vec()),
                    _ => ("403 Forbidden".into(), Vec::new()),
                };
                let msg = format!(
                    "HTTP/1.1 {head}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    body.len()
                );
                let _ = sock.write_all(msg.as_bytes()).await;
                let _ = sock.write_all(&body).await;
            });
        }
    });
    format!("http://{addr}")
}

#[tokio::test]
async fn brings_photos_from_links_and_pages() {
    let base = server().await;
    let f = media::Fetcher::new(true).unwrap();
    let got = f.fetch(&format!("{base}/foto.png")).await.unwrap();
    assert_eq!(got.name, "foto");
    assert_eq!(got.bytes, png());
    assert_eq!(
        f.fetch(&format!("{base}/pagina")).await.unwrap().bytes,
        png()
    );
    assert_eq!(
        f.fetch(&format!("{base}/vuelta")).await.unwrap().bytes,
        png()
    );
    assert!(f
        .fetch(&format!("{base}/texto"))
        .await
        .unwrap_err()
        .contains("no lleva a una foto"));
    assert!(f
        .fetch(&format!("{base}/privada"))
        .await
        .unwrap_err()
        .contains("privada"));
}

#[tokio::test]
async fn never_goes_into_the_local_network() {
    let base = server().await;
    let f = media::Fetcher::new(false).unwrap();
    let err = f.fetch(&format!("{base}/foto.png")).await.unwrap_err();
    assert!(
        err.contains("puerto") || err.contains("red privada"),
        "{err}"
    );
    let err = f.fetch("https://x.com:8080/foto.png").await.unwrap_err();
    assert!(err.contains("puerto"), "{err}");
    let err = f.fetch("http://127.0.0.1/foto.png").await.unwrap_err();
    assert!(err.contains("red privada"), "{err}");
    let err = f.fetch("http://localhost/foto.png").await.unwrap_err();
    assert!(err.contains("red privada"), "{err}");
}
