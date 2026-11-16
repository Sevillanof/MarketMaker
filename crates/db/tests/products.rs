//! Tests de Productos: cada tienda ve los suyos, la tienda solo los visibles
//! y las etiquetas sirven para encontrarlos.

use db::products;
use domain::Product;
use sqlx::PgPool;
use uuid::Uuid;

async fn tenant(pool: &PgPool, slug: &str) -> Uuid {
    sqlx::query_scalar("INSERT INTO tenants (slug, name) VALUES ($1, $1) RETURNING id")
        .bind(slug)
        .fetch_one(pool)
        .await
        .unwrap()
}

fn taza() -> Product {
    Product {
        name: "Taza".into(),
        price_cents: 500_000,
        compare_at_cents: 650_000,
        photos: vec!["/media/a/960.webp".into(), "/media/b/960.webp".into()],
        image_url: "/media/a/960.webp".into(),
        tags: vec!["cocina".into(), "regalo".into()],
        visible: false,
        photo_fits: [(
            "/media/b/960.webp".to_string(),
            domain::PhotoFit {
                x: 80,
                y: 40,
                zoom: 135,
            },
        )]
        .into(),
        parcel: Some(domain::Parcel {
            weight_g: 350,
            length_mm: 120,
            width_mm: 120,
            height_mm: 105,
        }),
        ..Product::default()
    }
}

#[sqlx::test(migrations = "../../migrations")]
async fn create_edit_hide_and_delete(pool: PgPool) {
    let ana = tenant(&pool, "ana").await;
    let beto = tenant(&pool, "beto").await;
    let id = products::create(&pool, ana, &taza()).await.unwrap();

    let got = products::get(&pool, ana, id).await.unwrap().unwrap();
    assert_eq!(
        got,
        Product {
            id: id.to_string(),
            ..taza()
        }
    );
    assert!(products::get(&pool, beto, id).await.unwrap().is_none());
    // Oculto: el dueño lo ve, la tienda no.
    assert_eq!(products::list_all(&pool, ana).await.unwrap().len(), 1);
    assert!(db::list_products(&pool, ana).await.unwrap().is_empty());

    assert!(products::set_visible(&pool, ana, id, true).await.unwrap());
    assert_eq!(db::list_products(&pool, ana).await.unwrap().len(), 1);

    let changed = Product {
        name: "Taza grande".into(),
        in_stock: false,
        ..taza()
    };
    assert!(!products::save(&pool, beto, id, &changed).await.unwrap());
    assert!(products::save(&pool, ana, id, &changed).await.unwrap());
    let got = products::get(&pool, ana, id).await.unwrap().unwrap();
    assert_eq!((got.name.as_str(), got.in_stock), ("Taza grande", false));

    assert!(!products::delete(&pool, beto, id).await.unwrap());
    assert!(products::delete(&pool, ana, id).await.unwrap());
    assert_eq!(products::count(&pool, ana).await.unwrap(), 0);
}

#[sqlx::test(migrations = "../../migrations")]
async fn tags_list_and_filter(pool: PgPool) {
    let ana = tenant(&pool, "ana").await;
    products::create(&pool, ana, &taza()).await.unwrap();
    let remera = Product {
        name: "Remera".into(),
        tags: vec!["regalo".into(), "ropa".into()],
        ..taza()
    };
    products::create(&pool, ana, &remera).await.unwrap();

    assert_eq!(
        products::tags(&pool, ana).await.unwrap(),
        ["regalo", "cocina", "ropa"]
    );
    let regalos = products::with_tag(&pool, ana, "regalo").await.unwrap();
    assert_eq!(regalos.len(), 2);
    let ropa = products::with_tag(&pool, ana, "ropa").await.unwrap();
    assert_eq!(ropa[0].name, "Remera");
}

#[sqlx::test(migrations = "../../migrations")]
async fn sample_products_come_with_photos_and_tags(pool: PgPool) {
    let demo: Uuid = sqlx::query_scalar("SELECT id FROM tenants WHERE slug = 'demo'")
        .fetch_one(&pool)
        .await
        .unwrap();
    let all = db::list_products(&pool, demo).await.unwrap();
    assert!(!all.is_empty());
    assert!(all
        .iter()
        .all(|p| p.photos == [p.image_url.clone()] && !p.tags.is_empty()));
}
