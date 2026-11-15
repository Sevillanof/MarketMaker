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
    let item = media::insert(&pool, id, ana, "Remera azul", 1600, 1200, 300_000)
        .await
        .unwrap();
    assert_eq!(item.url, format!("/media/{id}/960.webp"));

    assert_eq!(media::list(&pool, ana).await.unwrap(), vec![item]);
    assert!(media::list(&pool, beto).await.unwrap().is_empty());
    assert_eq!(media::count(&pool, ana).await.unwrap(), 1);

    assert!(!media::delete(&pool, beto, id).await.unwrap());
    assert!(media::delete(&pool, ana, id).await.unwrap());
    assert!(media::list(&pool, ana).await.unwrap().is_empty());
}
