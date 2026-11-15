//! Guardar en una carpeta: la foto se puede pasar a la carpeta del usuario.

#[tokio::test]
async fn moves_a_photo_into_the_user_folder() {
    let root = std::env::temp_dir().join(format!("mk-media-{}", std::process::id()));
    let store = media::Store::Local { root: root.clone() };
    let img = image::RgbImage::from_fn(300, 200, |x, _| image::Rgb([x as u8, 80, 160]));
    let mut png = Vec::new();
    image::DynamicImage::ImageRgb8(img)
        .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
        .unwrap();
    let p = media::process(&png).unwrap();
    let id = "0b6f6c1e-8d3a-4f0e-9a51-3f1a2b3c4d5e";
    store.put_all(id, &p.variants).await.unwrap();

    let folder = media::photo_folder("7c0e2b9a-1111-4222-8333-444455556666", id);
    assert_eq!(
        folder,
        format!("usuarios/7c0e2b9a-1111-4222-8333-444455556666/fotos/{id}")
    );
    let other = media::photo_folder(
        "7c0e2b9a-1111-4222-8333-444455556666",
        "11111111-2222-4333-8444-555566667777",
    );
    store.put_all(&other, &p.variants).await.unwrap();
    std::fs::write(root.join("nota.txt"), "no es una foto").unwrap();
    let listed: Vec<String> = store
        .list_folders()
        .await
        .unwrap()
        .into_iter()
        .map(|f| f.folder)
        .collect();
    assert_eq!(listed, vec![id.to_string(), other.clone()]);

    let moved = store.move_photo(id, &folder).await.unwrap();
    assert_eq!(
        moved,
        p.variants.iter().map(|v| v.bytes.len() as u64).sum::<u64>()
    );
    assert_eq!(store.get(id, "960.webp").await.unwrap(), None);
    assert!(store.get(&folder, "960.avif").await.unwrap().is_some());
    // Si ya no está en la carpeta vieja, no se puede mover de nuevo.
    assert!(store.move_photo(id, &folder).await.is_err());
    assert!(store.get(&folder, "480.webp").await.unwrap().is_some());
    _ = std::fs::remove_dir_all(root);
}
