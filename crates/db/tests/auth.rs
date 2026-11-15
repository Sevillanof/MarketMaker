//! Tests de las cuentas: códigos, sesiones y dueños.

use db::auth::*;
use db::get_tenant_by_slug;
use sqlx::PgPool;

/// Pone a mano un código conocido (el real es al azar).
async fn known_code(pool: &PgPool, email: &str) -> String {
    create_login_code(pool, email).await.unwrap().unwrap();
    let code = "123456".to_string();
    sqlx::query("UPDATE login_codes SET code_hash = sha256(convert_to($1 || ':' || $2, 'UTF8')) WHERE email = $1 AND used_at IS NULL")
        .bind(email)
        .bind(&code)
        .execute(pool)
        .await
        .unwrap();
    code
}

#[sqlx::test(migrations = "../../migrations")]
async fn right_code_creates_the_account_once(pool: PgPool) {
    let code = known_code(&pool, "ana@correo.com").await;
    let CodeResult::Ok { user_id } = check_login_code(&pool, "ana@correo.com", &code)
        .await
        .unwrap()
    else {
        panic!("debía entrar");
    };
    // El código ya no sirve.
    assert_eq!(
        check_login_code(&pool, "ana@correo.com", &code)
            .await
            .unwrap(),
        CodeResult::Expired
    );
    // La segunda vez es la misma cuenta.
    let code = known_code(&pool, "ana@correo.com").await;
    assert_eq!(
        check_login_code(&pool, "ana@correo.com", &code)
            .await
            .unwrap(),
        CodeResult::Ok { user_id }
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn wrong_codes_run_out(pool: PgPool) {
    let code = known_code(&pool, "ana@correo.com").await;
    for left in (1..CODE_ATTEMPTS as u32).rev() {
        assert_eq!(
            check_login_code(&pool, "ana@correo.com", "000000")
                .await
                .unwrap(),
            CodeResult::Wrong { left }
        );
    }
    assert_eq!(
        check_login_code(&pool, "ana@correo.com", "000000")
            .await
            .unwrap(),
        CodeResult::Expired
    );
    // Gastados los intentos, ni el correcto sirve.
    assert_eq!(
        check_login_code(&pool, "ana@correo.com", &code)
            .await
            .unwrap(),
        CodeResult::Expired
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn a_code_is_only_for_its_email(pool: PgPool) {
    let code = known_code(&pool, "ana@correo.com").await;
    assert_eq!(
        check_login_code(&pool, "beto@correo.com", &code)
            .await
            .unwrap(),
        CodeResult::Expired
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn too_many_codes_per_hour(pool: PgPool) {
    for _ in 0..CODES_PER_HOUR {
        assert!(create_login_code(&pool, "ana@correo.com")
            .await
            .unwrap()
            .is_ok());
    }
    assert_eq!(
        create_login_code(&pool, "ana@correo.com").await.unwrap(),
        Err(TooMany)
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn sessions_open_and_close(pool: PgPool) {
    let code = known_code(&pool, "ana@correo.com").await;
    let CodeResult::Ok { user_id } = check_login_code(&pool, "ana@correo.com", &code)
        .await
        .unwrap()
    else {
        panic!();
    };
    let token = create_session(&pool, user_id).await.unwrap();
    let me = session_user(&pool, &token).await.unwrap().unwrap();
    assert_eq!(me.email, "ana@correo.com");
    assert_eq!(session_user(&pool, "otro-token").await.unwrap(), None);
    delete_session(&pool, &token).await.unwrap();
    assert_eq!(session_user(&pool, &token).await.unwrap(), None);
}

#[sqlx::test(migrations = "../../migrations")]
async fn first_person_owns_the_store_and_nobody_else(pool: PgPool) {
    let tenant = get_tenant_by_slug(&pool, "demo").await.unwrap().unwrap();
    let mut ids = vec![];
    for email in ["ana@correo.com", "beto@correo.com"] {
        let code = known_code(&pool, email).await;
        let CodeResult::Ok { user_id } = check_login_code(&pool, email, &code).await.unwrap()
        else {
            panic!();
        };
        claim_if_ownerless(&pool, tenant.id, user_id).await.unwrap();
        ids.push(user_id);
    }
    assert_eq!(
        role_in(&pool, tenant.id, ids[0]).await.unwrap().as_deref(),
        Some("owner")
    );
    assert_eq!(role_in(&pool, tenant.id, ids[1]).await.unwrap(), None);
}
