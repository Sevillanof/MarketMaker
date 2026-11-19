//! Cuentas de compradores: enlaces de un solo uso, una cuenta por tienda,
//! sesiones atadas a su tienda y pedidos solo del comprador.

use db::buyers::{self, TooMany};
use db::orders::{self, Created, NewOrder};
use domain::{Delivery, OrderItem, OrderStatus};
use sqlx::PgPool;
use uuid::Uuid;

async fn tenant(pool: &PgPool, slug: &str) -> Uuid {
    sqlx::query_scalar("INSERT INTO tenants (slug, name) VALUES ($1, $1) RETURNING id")
        .bind(slug)
        .fetch_one(pool)
        .await
        .unwrap()
}

async fn place_order(pool: &PgPool, t: Uuid, email: &str, status: OrderStatus) -> Uuid {
    let items = [OrderItem {
        product_id: Uuid::new_v4().to_string(),
        name: "Taza".into(),
        image_url: String::new(),
        unit_cents: 1000,
        qty: 1,
        ..Default::default()
    }];
    let o = NewOrder {
        status,
        customer_name: "Lucía Gómez",
        email,
        phone: "1155551234",
        delivery: Delivery::Pickup,
        address: "",
        postal_code: "",
        city: "",
        notes: "",
        items: &items,
        shipping_cents: 0,
        discount_cents: 0,
        pay: domain::PayMethod::Coordinate,
        shipment: None,
    };
    match orders::create(pool, t, &o).await.unwrap() {
        Created::Saved { id, .. } => id,
        _ => panic!("no se creó el pedido"),
    }
}

#[sqlx::test(migrations = "../../migrations")]
async fn a_link_works_once_and_only_in_its_store(pool: PgPool) {
    let a = tenant(&pool, "tienda-a").await;
    let b = tenant(&pool, "tienda-b").await;
    let key = buyers::create_link(&pool, a, "lucia@correo.com")
        .await
        .unwrap()
        .unwrap();
    // En otra tienda no sirve.
    assert!(buyers::redeem_link(&pool, b, &key).await.unwrap().is_none());
    let id = buyers::redeem_link(&pool, a, &key).await.unwrap();
    assert!(id.is_some());
    // Ya se usó.
    assert!(buyers::redeem_link(&pool, a, &key).await.unwrap().is_none());
    // Una llave inventada tampoco.
    assert!(buyers::redeem_link(&pool, a, &"0".repeat(64))
        .await
        .unwrap()
        .is_none());
}

#[sqlx::test(migrations = "../../migrations")]
async fn a_new_link_cancels_the_previous_one(pool: PgPool) {
    let a = tenant(&pool, "tienda-a").await;
    let first = buyers::create_link(&pool, a, "lucia@correo.com")
        .await
        .unwrap()
        .unwrap();
    let second = buyers::create_link(&pool, a, "lucia@correo.com")
        .await
        .unwrap()
        .unwrap();
    assert!(buyers::redeem_link(&pool, a, &first)
        .await
        .unwrap()
        .is_none());
    assert!(buyers::redeem_link(&pool, a, &second)
        .await
        .unwrap()
        .is_some());
}

#[sqlx::test(migrations = "../../migrations")]
async fn links_per_email_are_capped(pool: PgPool) {
    let a = tenant(&pool, "tienda-a").await;
    for _ in 0..buyers::LINKS_PER_HOUR {
        assert!(buyers::create_link(&pool, a, "lucia@correo.com")
            .await
            .unwrap()
            .is_ok());
    }
    assert_eq!(
        buyers::create_link(&pool, a, "lucia@correo.com")
            .await
            .unwrap(),
        Err(TooMany)
    );
    // Otro email o otra tienda no se ven afectados.
    assert!(buyers::create_link(&pool, a, "otra@correo.com")
        .await
        .unwrap()
        .is_ok());
}

#[sqlx::test(migrations = "../../migrations")]
async fn the_same_person_has_separate_accounts_per_store(pool: PgPool) {
    let a = tenant(&pool, "tienda-a").await;
    let b = tenant(&pool, "tienda-b").await;
    let ka = buyers::create_link(&pool, a, "lucia@correo.com")
        .await
        .unwrap()
        .unwrap();
    let kb = buyers::create_link(&pool, b, "lucia@correo.com")
        .await
        .unwrap()
        .unwrap();
    let ida = buyers::redeem_link(&pool, a, &ka).await.unwrap().unwrap();
    let idb = buyers::redeem_link(&pool, b, &kb).await.unwrap().unwrap();
    assert_ne!(ida, idb);
    let token = buyers::create_session(&pool, ida).await.unwrap();
    // La sesión sirve en su tienda y en ninguna otra.
    assert!(buyers::session_buyer(&pool, a, &token)
        .await
        .unwrap()
        .is_some());
    assert!(buyers::session_buyer(&pool, b, &token)
        .await
        .unwrap()
        .is_none());
    // Cerrar la sesión la invalida.
    buyers::delete_session(&pool, &token).await.unwrap();
    assert!(buyers::session_buyer(&pool, a, &token)
        .await
        .unwrap()
        .is_none());
}

#[sqlx::test(migrations = "../../migrations")]
async fn orders_are_only_the_buyers_own_and_real_ones(pool: PgPool) {
    let a = tenant(&pool, "tienda-a").await;
    let b = tenant(&pool, "tienda-b").await;
    let mine = place_order(&pool, a, "Lucia@Correo.com", OrderStatus::New).await;
    let _other = place_order(&pool, a, "otra@correo.com", OrderStatus::New).await;
    let _elsewhere = place_order(&pool, b, "lucia@correo.com", OrderStatus::New).await;
    // Esperando el pago online: todavía no es un pedido.
    let _unpaid = place_order(&pool, a, "lucia@correo.com", OrderStatus::PendingPayment).await;

    let list = orders::list_for_email(&pool, a, "lucia@correo.com")
        .await
        .unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].id, mine.to_string());
    assert!(orders::get_for_email(&pool, a, "lucia@correo.com", mine)
        .await
        .unwrap()
        .is_some());
    // Un pedido ajeno no se puede abrir, ni con el id correcto.
    assert!(orders::get_for_email(&pool, a, "otra@correo.com", mine)
        .await
        .unwrap()
        .is_none());
    assert!(orders::get_for_email(&pool, b, "lucia@correo.com", mine)
        .await
        .unwrap()
        .is_none());
}

#[sqlx::test(migrations = "../../migrations")]
async fn the_first_sign_in_takes_the_name_from_the_last_order(pool: PgPool) {
    let a = tenant(&pool, "tienda-a").await;
    place_order(&pool, a, "lucia@correo.com", OrderStatus::New).await;
    let key = buyers::create_link(&pool, a, "lucia@correo.com")
        .await
        .unwrap()
        .unwrap();
    let id = buyers::redeem_link(&pool, a, &key).await.unwrap().unwrap();
    let token = buyers::create_session(&pool, id).await.unwrap();
    let me = buyers::session_buyer(&pool, a, &token)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(me.name, "Lucía Gómez");
    assert_eq!(me.phone, "1155551234");
    // Y guardar cambios y la foto funciona.
    buyers::save_profile(&pool, id, "Lucía G.", "11", "Calle 1", "1000", "CABA")
        .await
        .unwrap();
    buyers::set_avatar(&pool, id, Some("data:image/webp;base64,AAAA"))
        .await
        .unwrap();
    let me = buyers::session_buyer(&pool, a, &token)
        .await
        .unwrap()
        .unwrap();
    assert_eq!((me.name.as_str(), me.city.as_str()), ("Lucía G.", "CABA"));
    assert_eq!(me.avatar.as_deref(), Some("data:image/webp;base64,AAAA"));
    // Borrar la cuenta cierra las sesiones; el pedido queda.
    buyers::delete_account(&pool, id).await.unwrap();
    assert!(buyers::session_buyer(&pool, a, &token)
        .await
        .unwrap()
        .is_none());
    assert_eq!(
        orders::list_for_email(&pool, a, "lucia@correo.com")
            .await
            .unwrap()
            .len(),
        1
    );
}

async fn product(pool: &PgPool, t: Uuid, name: &str) -> Uuid {
    sqlx::query_scalar(
        "INSERT INTO products (tenant_id, name, price_cents, image_url, visible)
         VALUES ($1, $2, 1000, '', true) RETURNING id",
    )
    .bind(t)
    .bind(name)
    .fetch_one(pool)
    .await
    .unwrap()
}

async fn buyer(pool: &PgPool, t: Uuid, email: &str) -> Uuid {
    let key = buyers::create_link(pool, t, email).await.unwrap().unwrap();
    buyers::redeem_link(pool, t, &key).await.unwrap().unwrap()
}

#[sqlx::test(migrations = "../../migrations")]
async fn favorites_belong_to_the_buyer_of_that_store(pool: PgPool) {
    let a = tenant(&pool, "tienda-a").await;
    let b = tenant(&pool, "tienda-b").await;
    let taza = product(&pool, a, "Taza").await;
    let remera = product(&pool, b, "Remera").await;
    let lucia_a = buyer(&pool, a, "lucia@correo.com").await;
    let lucia_b = buyer(&pool, b, "lucia@correo.com").await;
    assert_ne!(lucia_a, lucia_b);

    assert!(buyers::set_favorite(&pool, a, lucia_a, taza, true).await.unwrap());
    // Marcar dos veces no duplica.
    assert!(buyers::set_favorite(&pool, a, lucia_a, taza, true).await.unwrap());
    assert_eq!(buyers::favorite_ids(&pool, a, lucia_a).await.unwrap(), vec![taza]);
    // Un producto de otra tienda no se puede guardar en esta cuenta.
    assert!(!buyers::set_favorite(&pool, a, lucia_a, remera, true).await.unwrap());
    // Y la misma persona en la otra tienda no ve los favoritos de esta.
    assert!(buyers::favorite_ids(&pool, b, lucia_b).await.unwrap().is_empty());
    // La lista de la página trae el producto.
    let list = db::products::list_favorites(&pool, a, lucia_a).await.unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].name, "Taza");

    // Un producto que se oculta sale de la lista.
    sqlx::query("UPDATE products SET visible = false WHERE id = $1")
        .bind(taza)
        .execute(&pool)
        .await
        .unwrap();
    assert!(buyers::favorite_ids(&pool, a, lucia_a).await.unwrap().is_empty());
    sqlx::query("UPDATE products SET visible = true WHERE id = $1")
        .bind(taza)
        .execute(&pool)
        .await
        .unwrap();

    assert!(buyers::set_favorite(&pool, a, lucia_a, taza, false).await.unwrap());
    assert!(buyers::favorite_ids(&pool, a, lucia_a).await.unwrap().is_empty());
}

#[sqlx::test(migrations = "../../migrations")]
async fn a_favorite_chosen_before_signing_in_is_saved_on_redeem(pool: PgPool) {
    let a = tenant(&pool, "tienda-a").await;
    let b = tenant(&pool, "tienda-b").await;
    let taza = product(&pool, a, "Taza").await;
    let remera = product(&pool, b, "Remera").await;
    let key = buyers::create_link_for(&pool, a, "lucia@correo.com", Some(taza))
        .await
        .unwrap()
        .unwrap();
    let id = buyers::redeem_link(&pool, a, &key).await.unwrap().unwrap();
    assert_eq!(buyers::favorite_ids(&pool, a, id).await.unwrap(), vec![taza]);
    // Un producto de otra tienda en el enlace se ignora.
    let key = buyers::create_link_for(&pool, a, "otra@correo.com", Some(remera))
        .await
        .unwrap()
        .unwrap();
    let id = buyers::redeem_link(&pool, a, &key).await.unwrap().unwrap();
    assert!(buyers::favorite_ids(&pool, a, id).await.unwrap().is_empty());
}
