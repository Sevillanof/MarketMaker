//! Tests de la cuenta de Andreani: conectar, guardar lo elegido, volver a
//! conectar sin perderlo y desconectar.

use db::andreani;
use domain::{AndreaniContract, AndreaniService, AndreaniSettings, Parcel};
use sqlx::PgPool;
use uuid::Uuid;

async fn tenant(pool: &PgPool, slug: &str) -> Uuid {
    sqlx::query_scalar("INSERT INTO tenants (slug, name) VALUES ($1, $1) RETURNING id")
        .bind(slug)
        .fetch_one(pool)
        .await
        .unwrap()
}

#[sqlx::test(migrations = "../../migrations")]
async fn connect_save_reconnect_disconnect(pool: PgPool) {
    let ana = tenant(&pool, "ana").await;
    let contracts = vec![AndreaniContract {
        id: "11".into(),
        number: "400006709".into(),
        service: AndreaniService::Home,
    }];
    let login = |token: &'static str| andreani::Login {
        credential: "cHltZXxhYmM=",
        access_token: token,
        client_type: "pyme",
        holder: "Ana SRL",
        cuit: "30712345678",
        email: "ana@example.com",
        contracts: &[],
    };
    assert!(andreani::account(&pool, ana).await.unwrap().is_none());
    andreani::connect(&pool, ana, &andreani::Login { contracts: &contracts, ..login("t1") })
        .await
        .unwrap();
    let acc = andreani::account(&pool, ana).await.unwrap().unwrap();
    assert_eq!(acc.access_token, "t1");
    assert_eq!(acc.contracts.0, contracts);
    assert!(acc.settings.is_none());

    let settings = AndreaniSettings {
        enabled: true,
        origin: Default::default(),
        services: vec![AndreaniService::Home],
        parcel: Parcel::default(),
    };
    assert!(andreani::save_settings(&pool, ana, &settings).await.unwrap());
    andreani::set_token(&pool, ana, "t2").await.unwrap();

    // Volver a conectar conserva lo elegido.
    andreani::connect(&pool, ana, &login("t3")).await.unwrap();
    let acc = andreani::account(&pool, ana).await.unwrap().unwrap();
    assert_eq!(acc.access_token, "t3");
    assert_eq!(acc.settings.unwrap().0, settings);

    let beto = tenant(&pool, "beto").await;
    assert!(!andreani::save_settings(&pool, beto, &settings).await.unwrap());

    andreani::disconnect(&pool, ana).await.unwrap();
    assert!(andreani::account(&pool, ana).await.unwrap().is_none());
}
