//! Borrador y publicado: guardar no cambia lo que ven los visitantes.

use db::{get_current_page, get_published_page, get_tenant_by_slug, publish_page, save_page};
use domain::{PageDoc, SaveOutcome};
use sqlx::PgPool;

async fn theme(pool: &PgPool, published: bool) -> (String, i32) {
    let t = get_tenant_by_slug(pool, "demo").await.unwrap().unwrap();
    let page = if published {
        get_published_page(pool, t.id, "home").await
    } else {
        get_current_page(pool, t.id, "home").await
    }
    .unwrap()
    .unwrap();
    let doc: PageDoc = serde_json::from_value(page.doc.unwrap()).unwrap();
    (doc.theme, page.published)
}

#[sqlx::test(migrations = "../../migrations")]
async fn saving_is_a_draft_until_published(pool: PgPool) {
    let t = get_tenant_by_slug(&pool, "demo").await.unwrap().unwrap();
    let (before, published) = theme(&pool, true).await;
    assert_eq!(published, 1, "lo que había ya estaba publicado");

    let page = get_current_page(&pool, t.id, "home")
        .await
        .unwrap()
        .unwrap();
    let mut doc: PageDoc = serde_json::from_value(page.doc.unwrap()).unwrap();
    doc.theme = "cupcake".into();
    let outcome = save_page(&pool, t.id, "home", &doc, 1).await.unwrap();
    assert_eq!(outcome, SaveOutcome::Saved { version: 2 });

    assert_eq!(
        theme(&pool, true).await,
        (before.clone(), 1),
        "el visitante sigue viendo lo anterior"
    );
    assert_eq!(
        theme(&pool, false).await,
        ("cupcake".into(), 1),
        "el editor ve el borrador"
    );

    assert!(publish_page(&pool, t.id, "home", 2).await.unwrap());
    assert_eq!(theme(&pool, true).await, ("cupcake".into(), 2));
    assert!(!publish_page(&pool, t.id, "home", 99).await.unwrap());
}
