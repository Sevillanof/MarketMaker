//! Varias tiendas: crearlas, listarlas y pasar del panel a una tienda.

use db::auth::*;
use db::{get_current_page, get_published_page, get_tenant_by_slug, list_products};
use sqlx::PgPool;

fn starter(name: &str) -> serde_json::Value {
    serde_json::to_value(domain::starter_doc(name)).unwrap()
}

async fn user(pool: &PgPool, email: &str) -> uuid::Uuid {
    sqlx::query_scalar("INSERT INTO users (email) VALUES ($1) RETURNING id")
        .bind(email)
        .fetch_one(pool)
        .await
        .unwrap()
}

#[sqlx::test(migrations = "../../migrations")]
async fn a_new_store_starts_unpublished_with_example_products(pool: PgPool) {
    let ana = user(&pool, "ana@correo.com").await;
    assert!(
        create_store(&pool, "La Casa de Ana", "lacasadeana", ana, "demo", &starter("x"))
            .await
            .unwrap()
    );
    assert!(
        !create_store(&pool, "Otra", "lacasadeana", ana, "demo", &starter("x"))
            .await
            .unwrap(),
        "dirección tomada"
    );
    assert!(slug_taken(&pool, "lacasadeana").await.unwrap());

    let t = get_tenant_by_slug(&pool, "lacasadeana")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(t.name, "La Casa de Ana");
    assert_eq!(
        role_in(&pool, t.id, ana).await.unwrap().as_deref(),
        Some("owner")
    );
    let draft = get_current_page(&pool, t.id, "home")
        .await
        .unwrap()
        .unwrap();
    let published = get_published_page(&pool, t.id, "home")
        .await
        .unwrap()
        .unwrap();
    assert_eq!((draft.version, published.version), (1, 0));
    assert!(draft.doc.is_some());
    assert!(published.doc.is_none(), "no se publica sola");
    // Los productos de ejemplo se ven en el editor, nunca para los compradores.
    let all = list_products(&pool, t.id).await.unwrap();
    assert_eq!(all.len(), 8);
    assert!(all.iter().all(|p| p.example));
    assert!(db::products::list_for_buyers(&pool, t.id).await.unwrap().is_empty());
    assert_eq!(db::products::count_real_visible(&pool, t.id).await.unwrap(), 0);
    assert_eq!(db::products::count_examples(&pool, t.id).await.unwrap(), 8);
    // Guardar un producto de ejemplo lo hace propio.
    let mut first = all[0].clone();
    first.name = "Mi taza".into();
    let id = uuid::Uuid::parse_str(&first.id).unwrap();
    assert!(db::products::save(&pool, t.id, id, &first).await.unwrap());
    assert_eq!(db::products::count_real_visible(&pool, t.id).await.unwrap(), 1);
    assert_eq!(db::products::delete_examples(&pool, t.id).await.unwrap(), 7);
    assert_eq!(list_products(&pool, t.id).await.unwrap().len(), 1);
    assert!(!db::orders::delivery_set(&pool, t.id).await.unwrap());

    let stores = stores_for_user(&pool, ana).await.unwrap();
    assert_eq!(stores.len(), 1);
    assert_eq!(stores[0].slug, "lacasadeana");
}

#[sqlx::test(migrations = "../../migrations")]
async fn a_handoff_works_once_and_only_for_its_store(pool: PgPool) {
    let ana = user(&pool, "ana@correo.com").await;
    create_store(&pool, "Ana", "ana", ana, "demo", &starter("x"))
        .await
        .unwrap();
    let ana_store = get_tenant_by_slug(&pool, "ana").await.unwrap().unwrap();
    let demo = get_tenant_by_slug(&pool, "demo").await.unwrap().unwrap();

    let token = create_handoff(&pool, ana, ana_store.id).await.unwrap();
    assert_eq!(
        redeem_handoff(&pool, &token, demo.id).await.unwrap(),
        None,
        "otra tienda"
    );
    assert_eq!(
        redeem_handoff(&pool, &token, ana_store.id).await.unwrap(),
        Some(ana)
    );
    assert_eq!(
        redeem_handoff(&pool, &token, ana_store.id).await.unwrap(),
        None,
        "un solo uso"
    );
}
