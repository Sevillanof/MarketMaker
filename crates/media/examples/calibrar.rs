//! Genera una foto en varias calidades para medirlas con SSIMULACRA2:
//! `cargo run -p media --example calibrar -- foto.jpg carpeta/`.

fn main() {
    let mut args = std::env::args().skip(1);
    let path = args.next().expect("falta la foto");
    let out = std::path::PathBuf::from(args.next().expect("falta la carpeta"));
    std::fs::create_dir_all(&out).unwrap();
    let img = image::open(&path).unwrap();
    let w = img.width().min(960);
    let img = img
        .resize(w, u32::MAX, image::imageops::FilterType::Lanczos3)
        .to_rgba8();
    img.save(out.join("ref.png")).unwrap();
    for q in [40, 45, 50, 55, 60, 65, 70, 75, 80, 85] {
        for speed in [8u8, 6] {
            let a = media::encode_avif_with(&img, q as f32, speed).unwrap();
            std::fs::write(out.join(format!("a{q}s{speed}.avif")), a).unwrap();
        }
        let wq = q + 5;
        std::fs::write(
            out.join(format!("w{wq}.webp")),
            media::encode_webp_with(&img, wq as f32),
        )
        .unwrap();
    }
}
