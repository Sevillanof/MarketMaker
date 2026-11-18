//! Con clave, las llaves de Mercado Pago y Andreani quedan cifradas en la
//! base y salen en claro al leerlas. (Archivo aparte: la clave vale para todo
//! el programa de pruebas.)

use db::{andreani, mp, secrets};
use sqlx::PgPool;
use uuid::Uuid;

async fn tenant(pool: &PgPool, slug: &str) -> Uuid {
    sqlx::query_scalar("INSERT INTO tenants (slug, name) VALUES ($1, $1) RETURNING id")
        .bind(slug)
        .fetch_one(pool)
        .await
        .unwrap()
}

async fn stored_mp(pool: &PgPool, id: Uuid) -> (String, String) {
    sqlx::query_as("SELECT mp_access_token, mp_refresh_token FROM tenants WHERE id = $1")
        .bind(id)
        .fetch_one(pool)
        .await
        .unwrap()
}

async fn stored_andreani(pool: &PgPool, id: Uuid) -> (String, String) {
    sqlx::query_as("SELECT credential, access_token FROM andreani_accounts WHERE tenant_id = $1")
        .bind(id)
        .fetch_one(pool)
        .await
        .unwrap()
}

fn login<'a>(credential: &'a str, token: &'a str) -> andreani::Login<'a> {
    andreani::Login {
        credential,
        access_token: token,
        client_type: "pyme",
        holder: "Ana",
        cuit: "20-1-1",
        email: "ana@correo.com",
        contracts: &[],
    }
}

#[sqlx::test(migrations = "../../migrations")]
async fn new_keys_are_stored_sealed(pool: PgPool) {
    secrets::init([3u8; 32]);
    let ana = tenant(&pool, "ana").await;
    mp::save_link(
        &pool,
        ana,
        &mp::Tokens {
            user_id: 7,
            access_token: "APP_USR-acceso",
            refresh_token: "TG-renovar",
            public_key: "APP_USR-publica",
            expires_in_secs: 3600,
            live: true,
        },
    )
    .await
    .unwrap();
    let (access, refresh) = stored_mp(&pool, ana).await;
    assert!(access.starts_with("enc1:") && !access.contains("acceso"));
    assert!(refresh.starts_with("enc1:") && !refresh.contains("renovar"));
    let c = mp::credentials(&pool, ana).await.unwrap().unwrap();
    assert_eq!(c.access_token, "APP_USR-acceso");
    assert_eq!(c.refresh_token, "TG-renovar");

    andreani::connect(&pool, ana, &login("codigo-woo", "t1"))
        .await
        .unwrap();
    andreani::set_token(&pool, ana, "t2").await.unwrap();
    let (credential, token) = stored_andreani(&pool, ana).await;
    assert!(credential.starts_with("enc1:") && !credential.contains("woo"));
    assert!(token.starts_with("enc1:"));
    let acc = andreani::account(&pool, ana).await.unwrap().unwrap();
    assert_eq!(acc.credential, "codigo-woo");
    assert_eq!(acc.access_token, "t2");
}

#[sqlx::test(migrations = "../../migrations")]
async fn keys_from_before_still_work_and_get_sealed(pool: PgPool) {
    secrets::init([3u8; 32]);
    let ana = tenant(&pool, "ana").await;
    let beto = tenant(&pool, "beto").await;
    sqlx::query(
        "UPDATE tenants SET mp_user_id = 7, mp_access_token = 'APP_USR-viejo', mp_refresh_token = 'TG-viejo' WHERE id = $1",
    )
    .bind(ana)
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO andreani_accounts (tenant_id, credential, access_token) VALUES ($1, 'codigo-viejo', 'tok-viejo')",
    )
    .bind(ana)
    .execute(&pool)
    .await
    .unwrap();
    // En claro se leen igual.
    let c = mp::credentials(&pool, ana).await.unwrap().unwrap();
    assert_eq!(c.access_token, "APP_USR-viejo");

    assert_eq!(secrets::seal_stored(&pool).await.unwrap(), 2);
    assert_eq!(secrets::seal_stored(&pool).await.unwrap(), 0);
    let (access, refresh) = stored_mp(&pool, ana).await;
    assert!(access.starts_with("enc1:") && refresh.starts_with("enc1:"));
    let (credential, token) = stored_andreani(&pool, ana).await;
    assert!(credential.starts_with("enc1:") && token.starts_with("enc1:"));
    let c = mp::credentials(&pool, ana).await.unwrap().unwrap();
    assert_eq!(c.access_token, "APP_USR-viejo");
    assert_eq!(c.refresh_token, "TG-viejo");
    let acc = andreani::account(&pool, ana).await.unwrap().unwrap();
    assert_eq!(acc.credential, "codigo-viejo");
    assert_eq!(acc.access_token, "tok-viejo");
    // Una tienda sin Mercado Pago sigue sin nada.
    assert!(mp::credentials(&pool, beto).await.unwrap().is_none());
}
