//! Varias tiendas: crearlas, listarlas y pasar del panel a una tienda.

use db::auth::*;
use db::{get_current_page, get_published_page, get_tenant_by_slug, list_products};
use sqlx::PgPool;

async fn user(pool: &PgPool, email: &str) -> uuid::Uuid {
    sqlx::query_scalar("INSERT INTO users (email) VALUES ($1) RETURNING id")
        .bind(email)
        .fetch_one(pool)
        .await
        .unwrap()
}

#[sqlx::test(migrations = "../../migrations")]
async fn a_new_store_starts_as_a_copy_of_the_demo(pool: PgPool) {
    let ana = user(&pool, "ana@correo.com").await;
    assert!(
        create_store(&pool, "La Casa de Ana", "lacasadeana", ana, "demo")
            .await
            .unwrap()
    );
    assert!(
        !create_store(&pool, "Otra", "lacasadeana", ana, "demo")
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
    assert_eq!((draft.version, published.version), (1, 1));
    assert!(published.doc.is_some());
    assert_eq!(list_products(&pool, t.id).await.unwrap().len(), 8);

    let stores = stores_for_user(&pool, ana).await.unwrap();
    assert_eq!(stores.len(), 1);
    assert_eq!(stores[0].slug, "lacasadeana");
}

#[sqlx::test(migrations = "../../migrations")]
async fn a_handoff_works_once_and_only_for_its_store(pool: PgPool) {
    let ana = user(&pool, "ana@correo.com").await;
    create_store(&pool, "Ana", "ana", ana, "demo")
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
