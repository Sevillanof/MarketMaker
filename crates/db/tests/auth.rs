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
async fn too_many_codes_per_day(pool: PgPool) {
    // Códigos pedidos a lo largo del día (ninguno en la última hora).
    for _ in 0..CODES_PER_DAY {
        sqlx::query(
            "INSERT INTO login_codes (email, code_hash, created_at, expires_at)
             VALUES ('ana@correo.com', '\\x00', now() - interval '3 hours', now() - interval '2 hours')",
        )
        .execute(&pool)
        .await
        .unwrap();
    }
    assert_eq!(
        create_login_code(&pool, "ana@correo.com").await.unwrap(),
        Err(TooMany)
    );
    // A otra persona no le afecta.
    assert!(create_login_code(&pool, "luz@correo.com")
        .await
        .unwrap()
        .is_ok());
}

#[sqlx::test(migrations = "../../migrations")]
async fn link_keys_are_long_and_work_once(pool: PgPool) {
    let key = create_login_link_key(&pool, "ana@correo.com")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(key.len(), 32);
    assert!(key.chars().all(|c| c.is_ascii_hexdigit()));
    assert!(matches!(
        check_login_code(&pool, "ana@correo.com", &key)
            .await
            .unwrap(),
        CodeResult::Ok { .. }
    ));
    assert_eq!(
        check_login_code(&pool, "ana@correo.com", &key)
            .await
            .unwrap(),
        CodeResult::Expired
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn an_invitation_survives_other_links_and_wrong_tries(pool: PgPool) {
    let invite = create_invite_key(&pool, "ana@correo.com").await.unwrap();
    // Alguien pide enlaces desde Entrar y prueba llaves equivocadas.
    let other = create_login_link_key(&pool, "ana@correo.com")
        .await
        .unwrap()
        .unwrap();
    for _ in 0..CODE_ATTEMPTS + 2 {
        check_login_code(&pool, "ana@correo.com", "000000")
            .await
            .unwrap();
    }
    // El enlace pedido se gastó con los intentos; la invitación sigue.
    assert_eq!(
        check_login_code(&pool, "ana@correo.com", &other)
            .await
            .unwrap(),
        CodeResult::Expired
    );
    assert!(matches!(
        check_login_code(&pool, "ana@correo.com", &invite)
            .await
            .unwrap(),
        CodeResult::Ok { .. }
    ));
    // Sirve una sola vez, y no vale para otro email.
    assert_eq!(
        check_login_code(&pool, "ana@correo.com", &invite)
            .await
            .unwrap(),
        CodeResult::Expired
    );
    let luz = create_invite_key(&pool, "luz@correo.com").await.unwrap();
    assert_eq!(
        check_login_code(&pool, "ana@correo.com", &luz)
            .await
            .unwrap(),
        CodeResult::Expired
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn a_new_invitation_replaces_the_old_one_and_expires(pool: PgPool) {
    let who = ["ana@correo.com".to_string(), "luz@correo.com".to_string()];
    assert!(invitations_sent(&pool, &who).await.unwrap().is_empty());
    // Un enlace pedido desde Entrar no cuenta como invitación.
    create_login_link_key(&pool, "luz@correo.com")
        .await
        .unwrap()
        .unwrap();
    let first = create_invite_key(&pool, "ana@correo.com").await.unwrap();
    let sent = invitations_sent(&pool, &who).await.unwrap();
    assert_eq!(sent.len(), 1);
    assert_eq!(sent[0].0, "ana@correo.com");
    let second = create_invite_key(&pool, "ana@correo.com").await.unwrap();
    assert_eq!(
        check_login_code(&pool, "ana@correo.com", &first)
            .await
            .unwrap(),
        CodeResult::Expired
    );
    sqlx::query("UPDATE login_codes SET created_at = created_at - interval '8 days', expires_at = expires_at - interval '8 days' WHERE email = 'ana@correo.com'")
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(
        check_login_code(&pool, "ana@correo.com", &second)
            .await
            .unwrap(),
        CodeResult::Expired
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

    // Una sesión muy vieja se termina aunque se haya usado todos los días.
    let old = create_session(&pool, user_id).await.unwrap();
    sqlx::query("UPDATE sessions SET created_at = now() - make_interval(days => $1)")
        .bind(SESSION_MAX_DAYS + 1)
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(session_user(&pool, &old).await.unwrap(), None);
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

async fn new_account(pool: &PgPool, email: &str) -> uuid::Uuid {
    let code = known_code(pool, email).await;
    let CodeResult::Ok { user_id } = check_login_code(pool, email, &code).await.unwrap() else {
        panic!("debía crear la cuenta");
    };
    user_id
}

#[test]
fn passwords_are_hashed_with_salt() {
    let a = hash_password("mate con facturas");
    let b = hash_password("mate con facturas");
    assert!(a.starts_with("$argon2id$"));
    assert_ne!(a, b, "cada hash lleva su propia sal");
    assert!(verify_password("mate con facturas", &a));
    assert!(!verify_password("mate sin facturas", &a));
    assert!(!verify_password("mate con facturas", "no es un hash"));
}

#[sqlx::test(migrations = "../../migrations")]
async fn accounts_start_without_password_and_get_one(pool: PgPool) {
    let id = new_account(&pool, "ana@correo.com").await;
    let acc = account_by_email(&pool, "ana@correo.com")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        (acc.id, acc.password_hash.clone(), acc.locked_minutes),
        (id, None, 0)
    );
    set_name(&pool, id, "Ana").await.unwrap();
    set_password_hash(&pool, id, &hash_password("mate con facturas"))
        .await
        .unwrap();
    let acc = account_by_id(&pool, id).await.unwrap().unwrap();
    assert_eq!(acc.name, "Ana");
    assert!(verify_password(
        "mate con facturas",
        acc.password_hash.as_deref().unwrap()
    ));
    assert_eq!(
        account_by_email(&pool, "otro@correo.com").await.unwrap(),
        None
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn too_many_wrong_passwords_lock_the_account(pool: PgPool) {
    let id = new_account(&pool, "ana@correo.com").await;
    for _ in 1..PASSWORD_ATTEMPTS {
        assert!(!record_failed_login(&pool, id).await.unwrap());
    }
    assert!(record_failed_login(&pool, id).await.unwrap());
    let acc = account_by_id(&pool, id).await.unwrap().unwrap();
    assert!(acc.locked_minutes > 0 && acc.locked_minutes <= LOCK_MINUTES);
    // Elegir contraseña nueva (con el código) la destraba.
    set_password_hash(&pool, id, &hash_password("otra contraseña"))
        .await
        .unwrap();
    assert_eq!(
        account_by_id(&pool, id)
            .await
            .unwrap()
            .unwrap()
            .locked_minutes,
        0
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn changing_password_closes_other_sessions(pool: PgPool) {
    let id = new_account(&pool, "ana@correo.com").await;
    let mine = create_session(&pool, id).await.unwrap();
    let other = create_session(&pool, id).await.unwrap();
    delete_other_sessions(&pool, id, Some(&mine)).await.unwrap();
    assert!(session_user(&pool, &mine).await.unwrap().is_some());
    assert!(session_user(&pool, &other).await.unwrap().is_none());
    delete_other_sessions(&pool, id, None).await.unwrap();
    assert!(session_user(&pool, &mine).await.unwrap().is_none());
}

#[sqlx::test(migrations = "../../migrations")]
async fn profile_photo_must_be_your_own_and_clears_when_deleted(pool: PgPool) {
    use db::media::{self, NewMedia};
    let ana = new_account(&pool, "ana@correo.com").await;
    let luz = new_account(&pool, "luz@correo.com").await;
    let photo = |id, owner, sha| NewMedia {
        id,
        owner_id: owner,
        name: "Yo",
        width: 800,
        height: 800,
        bytes: 1000,
        web_bytes: 100,
        stored_bytes: 600,
        sha256: sha,
    };
    let mine = uuid::Uuid::new_v4();
    let hers = uuid::Uuid::new_v4();
    media::insert(&pool, &photo(mine, ana, "a")).await.unwrap();
    media::insert(&pool, &photo(hers, luz, "b")).await.unwrap();
    let token = create_session(&pool, ana).await.unwrap();

    assert!(!set_avatar(&pool, ana, Some(hers)).await.unwrap());
    assert!(set_avatar(&pool, ana, Some(mine)).await.unwrap());
    let me = session_user(&pool, &token).await.unwrap().unwrap();
    assert_eq!(me.avatar_id, Some(mine));

    // Si borra la foto de Mis fotos, vuelve a la inicial.
    media::delete(&pool, ana, mine).await.unwrap();
    let me = session_user(&pool, &token).await.unwrap().unwrap();
    assert_eq!(me.avatar_id, None);

    assert!(set_avatar(&pool, ana, None).await.unwrap());
}

#[sqlx::test(migrations = "../../migrations")]
async fn invited_account_is_created_without_code_but_only_once(pool: PgPool) {
    let id = create_account(&pool, "ana@correo.com", "Ana", "hash-1")
        .await
        .unwrap()
        .expect("debía crear la cuenta");
    let account = account_by_email(&pool, "ana@correo.com")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(account.id, id);
    assert_eq!(account.name, "Ana");
    assert_eq!(account.password_hash.as_deref(), Some("hash-1"));
    // Con el mismo email no se crea otra ni se pisa la contraseña.
    assert_eq!(
        create_account(&pool, "ana@correo.com", "Otra", "hash-2")
            .await
            .unwrap(),
        None
    );
    let account = account_by_email(&pool, "ana@correo.com")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(account.password_hash.as_deref(), Some("hash-1"));
    assert_eq!(account.name, "Ana");
}

#[sqlx::test(migrations = "../../migrations")]
async fn whitelist_adds_and_removes_emails(pool: PgPool) {
    let code = known_code(&pool, "admin@correo.com").await;
    let CodeResult::Ok { user_id: admin } = check_login_code(&pool, "admin@correo.com", &code)
        .await
        .unwrap()
    else {
        panic!("debía entrar");
    };
    allow_email(&pool, "lucia@correo.com", admin).await.unwrap();
    disallow_email(&pool, "ana@correo.com", admin)
        .await
        .unwrap();
    let (allowed, removed) = load_allowlist(&pool).await.unwrap();
    assert_eq!(allowed, vec!["lucia@correo.com"]);
    assert_eq!(removed, vec!["ana@correo.com"]);
    // Volver a agregar a quien se quitó lo habilita otra vez.
    allow_email(&pool, "ana@correo.com", admin).await.unwrap();
    let (allowed, removed) = load_allowlist(&pool).await.unwrap();
    assert_eq!(allowed.len(), 2);
    assert!(removed.is_empty());
    // Datos de cada email: con cuenta y sin ella, en el orden pedido.
    let emails = vec![
        "lucia@correo.com".to_string(),
        "admin@correo.com".to_string(),
    ];
    let m = allowlist_members(&pool, &emails).await.unwrap();
    assert_eq!(m[0].email, "lucia@correo.com");
    assert!(!m[0].has_account && m[0].added.is_some());
    assert!(m[1].has_account && m[1].joined.is_some());
}

// ───────────── Equipo de la tienda ─────────────

async fn user(pool: &PgPool, email: &str) -> uuid::Uuid {
    sqlx::query_scalar("INSERT INTO users (email) VALUES ($1) RETURNING id")
        .bind(email)
        .fetch_one(pool)
        .await
        .unwrap()
}

#[sqlx::test(migrations = "../../migrations")]
async fn team_invite_for_new_account_waits_and_then_joins(pool: PgPool) {
    let tenant = get_tenant_by_slug(&pool, "demo").await.unwrap().unwrap();
    let owner = user(&pool, "duena@correo.com").await;
    claim_if_ownerless(&pool, tenant.id, owner).await.unwrap();

    let added = add_team_member(&pool, tenant.id, "sofi@correo.com", "editor", owner)
        .await
        .unwrap();
    assert_eq!(added, Some(TeamAdd::Pending));
    // Queda habilitada para crear la cuenta, pero solo para trabajar en equipo.
    assert_eq!(
        load_team_only(&pool).await.unwrap(),
        vec!["sofi@correo.com"]
    );
    // Repetir la invitación no la duplica.
    assert_eq!(
        add_team_member(&pool, tenant.id, "sofi@correo.com", "editor", owner)
            .await
            .unwrap(),
        Some(TeamAdd::Already)
    );
    let team = team_members(&pool, tenant.id).await.unwrap();
    assert_eq!(team[0].role, "owner");
    assert!(!team[1].joined);

    // Al crear la cuenta y abrir una sesión, entra al equipo con su rol.
    let sofi = user(&pool, "sofi@correo.com").await;
    create_session(&pool, sofi).await.unwrap();
    assert_eq!(
        role_in(&pool, tenant.id, sofi).await.unwrap().as_deref(),
        Some("editor")
    );
    assert!(team_members(&pool, tenant.id)
        .await
        .unwrap()
        .iter()
        .all(|m| m.joined));
}

#[sqlx::test(migrations = "../../migrations")]
async fn team_invite_for_existing_account_joins_at_once(pool: PgPool) {
    let tenant = get_tenant_by_slug(&pool, "demo").await.unwrap().unwrap();
    let owner = user(&pool, "duena@correo.com").await;
    claim_if_ownerless(&pool, tenant.id, owner).await.unwrap();
    let martin = user(&pool, "martin@correo.com").await;
    assert_eq!(
        add_team_member(&pool, tenant.id, "martin@correo.com", "admin", owner)
            .await
            .unwrap(),
        Some(TeamAdd::Joined)
    );
    assert_eq!(
        role_in(&pool, tenant.id, martin).await.unwrap().as_deref(),
        Some("admin")
    );
    // Tenía cuenta: no se la marca como "solo equipo".
    assert!(load_team_only(&pool).await.unwrap().is_empty());
}

#[sqlx::test(migrations = "../../migrations")]
async fn team_roles_change_and_the_owner_is_untouchable(pool: PgPool) {
    let tenant = get_tenant_by_slug(&pool, "demo").await.unwrap().unwrap();
    let owner = user(&pool, "duena@correo.com").await;
    claim_if_ownerless(&pool, tenant.id, owner).await.unwrap();
    let lu = user(&pool, "lu@correo.com").await;
    add_team_member(&pool, tenant.id, "lu@correo.com", "editor", owner)
        .await
        .unwrap();
    assert!(set_team_role(&pool, tenant.id, "lu@correo.com", "admin")
        .await
        .unwrap());
    assert_eq!(
        role_in(&pool, tenant.id, lu).await.unwrap().as_deref(),
        Some("admin")
    );
    // El dueño no cambia ni se quita.
    assert!(
        !set_team_role(&pool, tenant.id, "duena@correo.com", "editor")
            .await
            .unwrap()
    );
    assert!(!remove_team_member(&pool, tenant.id, "duena@correo.com")
        .await
        .unwrap());
    assert_eq!(
        role_in(&pool, tenant.id, owner).await.unwrap().as_deref(),
        Some("owner")
    );
    // Quitarla le saca el acceso.
    assert!(remove_team_member(&pool, tenant.id, "lu@correo.com")
        .await
        .unwrap());
    assert_eq!(role_in(&pool, tenant.id, lu).await.unwrap(), None);
}

#[sqlx::test(migrations = "../../migrations")]
async fn team_has_a_pending_limit_and_respects_removed_emails(pool: PgPool) {
    let tenant = get_tenant_by_slug(&pool, "demo").await.unwrap().unwrap();
    let owner = user(&pool, "duena@correo.com").await;
    claim_if_ownerless(&pool, tenant.id, owner).await.unwrap();
    for i in 0..TEAM_PENDING_MAX {
        let e = format!("p{i}@correo.com");
        assert_eq!(
            add_team_member(&pool, tenant.id, &e, "editor", owner)
                .await
                .unwrap(),
            Some(TeamAdd::Pending)
        );
    }
    assert_eq!(
        add_team_member(&pool, tenant.id, "otra@correo.com", "editor", owner)
            .await
            .unwrap(),
        None
    );
    // Quien el administrador quitó de la Whitelist no se puede invitar.
    disallow_email(&pool, "malo@correo.com", owner)
        .await
        .unwrap();
    remove_team_member(&pool, tenant.id, "p0@correo.com")
        .await
        .unwrap();
    assert_eq!(
        add_team_member(&pool, tenant.id, "malo@correo.com", "editor", owner)
            .await
            .unwrap(),
        Some(TeamAdd::Blocked)
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn whitelist_allow_lets_a_team_only_email_create_stores(pool: PgPool) {
    let tenant = get_tenant_by_slug(&pool, "demo").await.unwrap().unwrap();
    let owner = user(&pool, "duena@correo.com").await;
    claim_if_ownerless(&pool, tenant.id, owner).await.unwrap();
    add_team_member(&pool, tenant.id, "sofi@correo.com", "editor", owner)
        .await
        .unwrap();
    assert_eq!(load_team_only(&pool).await.unwrap().len(), 1);
    allow_email(&pool, "sofi@correo.com", owner).await.unwrap();
    assert!(load_team_only(&pool).await.unwrap().is_empty());
}
