//! Tests de Mis fotos: cada dueño ve y borra solo las suyas.

use db::media;
use sqlx::PgPool;
use uuid::Uuid;

async fn user(pool: &PgPool, email: &str) -> Uuid {
    sqlx::query_scalar("INSERT INTO users (email) VALUES ($1) RETURNING id")
        .bind(email)
        .fetch_one(pool)
        .await
        .unwrap()
}

#[sqlx::test(migrations = "../../migrations")]
async fn each_owner_sees_and_deletes_only_theirs(pool: PgPool) {
    let ana = user(&pool, "ana@correo.com").await;
    let beto = user(&pool, "beto@correo.com").await;
    let id = Uuid::new_v4();
    let item = media::insert(&pool, id, ana, "Remera azul", 1600, 1200, 300_000, 20_000)
        .await
        .unwrap();
    assert_eq!(item.url, format!("/media/{id}/960.webp"));
    assert_eq!((item.original_bytes, item.web_bytes), (300_000, 20_000));

    assert_eq!(media::list(&pool, ana).await.unwrap(), vec![item]);
    assert!(media::list(&pool, beto).await.unwrap().is_empty());
    assert_eq!(media::count(&pool, ana).await.unwrap(), 1);

    assert!(!media::delete(&pool, beto, id).await.unwrap());
    assert!(media::delete(&pool, ana, id).await.unwrap());
    assert!(media::list(&pool, ana).await.unwrap().is_empty());
}

#[sqlx::test(migrations = "../../migrations")]
async fn rename_and_where_each_photo_is_used(pool: PgPool) {
    let ana = user(&pool, "ana@correo.com").await;
    let used = Uuid::new_v4();
    let unused = Uuid::new_v4();
    media::insert(&pool, used, ana, "Taza", 10, 10, 1, 1)
        .await
        .unwrap();
    media::insert(&pool, unused, ana, "Otra", 10, 10, 1, 1)
        .await
        .unwrap();
    // Ana es dueña de la tienda demo y su página de inicio usa la foto.
    let demo = db::get_tenant_by_slug(&pool, "demo")
        .await
        .unwrap()
        .unwrap();
    db::auth::claim_if_ownerless(&pool, demo.id, ana)
        .await
        .unwrap();
    sqlx::query(
        "UPDATE page_versions SET doc = jsonb_set(doc, '{x}', to_jsonb($1::text))
         WHERE id = (SELECT current_version_id FROM pages WHERE tenant_id = $2 AND slug = 'home')",
    )
    .bind(format!("/media/{used}/960.webp"))
    .bind(demo.id)
    .execute(&pool)
    .await
    .unwrap();

    let usage = media::usage(&pool, ana).await.unwrap();
    assert_eq!(usage, vec![(used, demo.name.clone(), "home".to_string())]);

    assert!(media::rename(&pool, ana, unused, "Taza azul")
        .await
        .unwrap());
    let names: Vec<_> = media::list(&pool, ana)
        .await
        .unwrap()
        .into_iter()
        .map(|i| i.name)
        .collect();
    assert!(names.contains(&"Taza azul".to_string()));
    let beto = user(&pool, "beto@correo.com").await;
    assert!(!media::rename(&pool, beto, unused, "Mía").await.unwrap());
}
