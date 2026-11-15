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

fn new<'a>(id: Uuid, owner: Uuid, name: &'a str, sha: &'a str) -> media::NewMedia<'a> {
    media::NewMedia {
        id,
        owner_id: owner,
        name,
        width: 1600,
        height: 1200,
        bytes: 300_000,
        web_bytes: 20_000,
        stored_bytes: 150_000,
        sha256: sha,
    }
}

#[sqlx::test(migrations = "../../migrations")]
async fn same_file_quota_and_description(pool: PgPool) {
    let ana = user(&pool, "ana@correo.com").await;
    let beto = user(&pool, "beto@correo.com").await;
    let sha = media::fingerprint(b"foto");
    assert_eq!(sha.len(), 64);
    let id = Uuid::new_v4();
    media::insert(&pool, &new(id, ana, "Remera", &sha))
        .await
        .unwrap();
    // La misma foto: Ana ya la tiene; Beto no.
    assert_eq!(
        media::find_by_hash(&pool, ana, &sha)
            .await
            .unwrap()
            .unwrap()
            .id,
        id.to_string()
    );
    assert!(media::find_by_hash(&pool, beto, &sha)
        .await
        .unwrap()
        .is_none());
    assert!(
        media::insert(&pool, &new(Uuid::new_v4(), ana, "Otra vez", &sha))
            .await
            .is_err()
    );

    let q = media::quota(&pool, ana).await.unwrap();
    assert_eq!((q.photos, q.bytes), (1, 150_000));
    assert_eq!(media::quota(&pool, beto).await.unwrap().photos, 0);

    assert!(media::set_alt(&pool, ana, id, "Remera azul de algodón")
        .await
        .unwrap());
    assert!(!media::set_alt(&pool, beto, id, "x").await.unwrap());
    assert_eq!(
        media::list(&pool, ana).await.unwrap()[0].alt,
        "Remera azul de algodón"
    );
    assert_eq!(
        media::all_folders(&pool).await.unwrap()[&id.to_string()],
        media::media_folder(ana, id)
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn each_owner_sees_and_deletes_only_theirs(pool: PgPool) {
    let ana = user(&pool, "ana@correo.com").await;
    let beto = user(&pool, "beto@correo.com").await;
    let id = Uuid::new_v4();
    let item = media::insert(&pool, &new(id, ana, "Remera azul", "aaa"))
        .await
        .unwrap();
    assert_eq!(item.url, format!("/media/{id}/960.webp"));
    assert_eq!((item.original_bytes, item.web_bytes), (300_000, 20_000));

    assert_eq!(media::list(&pool, ana).await.unwrap(), vec![item]);
    assert!(media::list(&pool, beto).await.unwrap().is_empty());
    assert_eq!(media::count(&pool, ana).await.unwrap(), 1);

    assert_eq!(media::delete(&pool, beto, id).await.unwrap(), None);
    assert_eq!(
        media::locate(&pool, id).await.unwrap(),
        Some((media::media_folder(ana, id), id.to_string()))
    );
    assert!(media::not_in_user_folder(&pool).await.unwrap().is_empty());
    assert_eq!(
        media::delete(&pool, ana, id).await.unwrap(),
        Some(media::media_folder(ana, id))
    );
    assert_eq!(media::locate(&pool, id).await.unwrap(), None);
    assert!(media::list(&pool, ana).await.unwrap().is_empty());
}

#[sqlx::test(migrations = "../../migrations")]
async fn rename_and_where_each_photo_is_used(pool: PgPool) {
    let ana = user(&pool, "ana@correo.com").await;
    let used = Uuid::new_v4();
    let unused = Uuid::new_v4();
    media::insert(&pool, &new(used, ana, "Taza", "b1"))
        .await
        .unwrap();
    media::insert(&pool, &new(unused, ana, "Otra", "b2"))
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
