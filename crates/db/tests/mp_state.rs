//! "Vincular mi cuenta" de Mercado Pago: el permiso vale una sola vez y solo
//! en la sesión que lo empezó.

use db::mp::{self, StateCheck};
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
async fn the_state_only_works_in_the_session_that_started_it(pool: PgPool) {
    let ana = tenant(&pool, "ana").await;

    // Vuelve a la misma sesión: vale, y una sola vez.
    mp::save_state(&pool, ana, "uno", "sesion-de-ana")
        .await
        .unwrap();
    assert_eq!(
        mp::take_state(&pool, "uno", Some("sesion-de-ana"))
            .await
            .unwrap(),
        StateCheck::Ok(ana)
    );
    assert_eq!(
        mp::take_state(&pool, "uno", Some("sesion-de-ana"))
            .await
            .unwrap(),
        StateCheck::Unknown
    );

    // Lo abre otra persona: no vale, y queda gastado también para la dueña.
    mp::save_state(&pool, ana, "dos", "sesion-de-ana")
        .await
        .unwrap();
    assert_eq!(
        mp::take_state(&pool, "dos", Some("sesion-de-otro"))
            .await
            .unwrap(),
        StateCheck::OtherSession(ana)
    );
    assert_eq!(
        mp::take_state(&pool, "dos", Some("sesion-de-ana"))
            .await
            .unwrap(),
        StateCheck::Unknown
    );

    // Sin sesión tampoco.
    mp::save_state(&pool, ana, "tres", "sesion-de-ana")
        .await
        .unwrap();
    assert_eq!(
        mp::take_state(&pool, "tres", None).await.unwrap(),
        StateCheck::OtherSession(ana)
    );

    // Pasados los 30 minutos ya no sirve.
    mp::save_state(&pool, ana, "cuatro", "sesion-de-ana")
        .await
        .unwrap();
    sqlx::query("UPDATE mp_oauth_states SET created_at = now() - interval '31 minutes'")
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(
        mp::take_state(&pool, "cuatro", Some("sesion-de-ana"))
            .await
            .unwrap(),
        StateCheck::Unknown
    );
}
