//! Tests de integración del guardado contra la base local.
//!
//! `sqlx::test` crea una base temporal por test (a partir de `DATABASE_URL`,
//! leída de `.env`), le aplica las migraciones y la borra al terminar.

use db::{get_current_page, get_tenant_by_slug, save_page, Tenant};
use domain::{PageDoc, SaveOutcome};
use sqlx::PgPool;

async fn demo(pool: &PgPool) -> (Tenant, PageDoc, u32) {
    let tenant = get_tenant_by_slug(pool, "demo").await.unwrap().unwrap();
    let page = get_current_page(pool, tenant.id, "home")
        .await
        .unwrap()
        .unwrap();
    let doc = serde_json::from_value(page.doc.unwrap()).unwrap();
    (tenant, doc, page.version as u32)
}

async fn version_count(pool: &PgPool) -> i64 {
    sqlx::query_scalar("SELECT count(*) FROM page_versions")
        .fetch_one(pool)
        .await
        .unwrap()
}

#[sqlx::test(migrations = "../../migrations")]
async fn valid_save_bumps_version_by_one(pool: PgPool) {
    let (tenant, mut doc, version) = demo(&pool).await;
    assert_eq!(version, 1);
    doc.theme = "cupcake".into();

    let outcome = save_page(&pool, tenant.id, "home", &doc, version)
        .await
        .unwrap();
    assert_eq!(outcome, SaveOutcome::Saved { version: 2 });

    let (_, saved, current) = demo(&pool).await;
    assert_eq!(current, 2);
    assert_eq!(saved, doc);
}

#[sqlx::test(migrations = "../../migrations")]
async fn stale_base_version_is_a_conflict_and_writes_nothing(pool: PgPool) {
    let (tenant, doc, version) = demo(&pool).await;
    save_page(&pool, tenant.id, "home", &doc, version)
        .await
        .unwrap();
    let before = version_count(&pool).await;

    let outcome = save_page(&pool, tenant.id, "home", &doc, version)
        .await
        .unwrap();
    assert_eq!(outcome, SaveOutcome::Conflict { current: 2 });
    assert_eq!(version_count(&pool).await, before);
}

#[sqlx::test(migrations = "../../migrations")]
async fn invalid_doc_is_rejected_and_writes_nothing(pool: PgPool) {
    let (tenant, mut doc, version) = demo(&pool).await;
    doc.theme = "inexistente".into();
    let before = version_count(&pool).await;

    let outcome = save_page(&pool, tenant.id, "home", &doc, version)
        .await
        .unwrap();
    assert!(matches!(outcome, SaveOutcome::Invalid { ref errors } if !errors.is_empty()));
    assert_eq!(version_count(&pool).await, before);
    assert_eq!(demo(&pool).await.2, 1);
}

#[sqlx::test(migrations = "../../migrations")]
async fn only_the_last_20_versions_are_kept(pool: PgPool) {
    let (tenant, doc, mut version) = demo(&pool).await;
    for _ in 0..25 {
        match save_page(&pool, tenant.id, "home", &doc, version)
            .await
            .unwrap()
        {
            SaveOutcome::Saved { version: v } => version = v,
            other => panic!("se esperaba Saved, hubo {other:?}"),
        }
    }
    assert_eq!(version, 26);
    // Las últimas 20, más la 1, que sigue publicada.
    assert_eq!(version_count(&pool).await, 21);
    let (min, max): (i32, i32) =
        sqlx::query_as("SELECT min(number), max(number) FROM page_versions WHERE number > 1")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!((min, max), (7, 26));
}
