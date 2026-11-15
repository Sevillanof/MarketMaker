//! Cuánto pesa cada archivo al convertir una foto:
//! `cargo run -p media --example medir -- foto.jpg`.

fn main() {
    let path = std::env::args().nth(1).expect("falta la foto");
    let bytes = std::fs::read(&path).unwrap();
    let t = std::time::Instant::now();
    let p = media::process(&bytes).unwrap();
    println!(
        "original: {} KB, {}x{}, {:?}",
        bytes.len() / 1024,
        p.width,
        p.height,
        t.elapsed()
    );
    for v in &p.variants {
        println!("  {:10} {:>6} KB", v.name, v.bytes.len() / 1024);
    }
}
