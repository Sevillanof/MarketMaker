//! Guardar en un bucket S3. Necesita un servidor compatible: se corre con
//! `S3_ENDPOINT=http://127.0.0.1:9000 R2_BUCKET=fotos R2_ACCESS_KEY_ID=x
//! R2_SECRET_ACCESS_KEY=x cargo test -p media --test s3 -- --ignored`.

#[tokio::test]
#[ignore = "necesita un servidor S3 (ver arriba)"]
async fn saves_reads_and_deletes_in_the_bucket() {
    let store = media::Store::from_env().unwrap();
    assert!(store.describe().contains("bucket"));
    let img = image::RgbImage::from_fn(700, 400, |x, _| image::Rgb([x as u8, 80, 160]));
    let mut png = Vec::new();
    image::DynamicImage::ImageRgb8(img)
        .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
        .unwrap();
    let p = media::process(&png).unwrap();
    let id = "0b6f6c1e-8d3a-4f0e-9a51-3f1a2b3c4d5e";
    store.put_all(id, &p.variants).await.unwrap();
    let got = store.get(id, "960.webp").await.unwrap().unwrap();
    assert_eq!(got, p.variants[3].bytes);
    assert_eq!(store.get(id, "nada.webp").await.unwrap(), None);
    // Pasar la foto a la carpeta del usuario.
    let folder = media::photo_folder("7c0e2b9a-1111-4222-8333-444455556666", id);
    store.move_photo(id, &folder).await.unwrap();
    assert_eq!(store.get(id, "960.webp").await.unwrap(), None);
    assert_eq!(
        store.get(&folder, "960.webp").await.unwrap().unwrap(),
        p.variants[3].bytes
    );
    store.delete_all(&folder).await.unwrap();
    assert_eq!(store.get(&folder, "960.webp").await.unwrap(), None);
}
