//! Tests de pedidos: el stock baja al comprar, nunca se vende de más y
//! vuelve al cancelar.

use db::orders::{self, Created, NewOrder};
use db::products;
use domain::{Delivery, OrderItem, OrderStatus, Product};
use sqlx::PgPool;
use uuid::Uuid;

async fn tenant(pool: &PgPool, slug: &str) -> Uuid {
    sqlx::query_scalar("INSERT INTO tenants (slug, name) VALUES ($1, $1) RETURNING id")
        .bind(slug)
        .fetch_one(pool)
        .await
        .unwrap()
}

fn order<'a>(items: &'a [OrderItem]) -> NewOrder<'a> {
    NewOrder {
        status: OrderStatus::New,
        customer_name: "Ana",
        email: "ana@correo.com",
        phone: "1155551234",
        delivery: Delivery::Pickup,
        address: "",
        postal_code: "",
        city: "",
        notes: "",
        items,
        shipping_cents: 0,
    }
}

fn item(id: Uuid, qty: u32) -> OrderItem {
    OrderItem {
        product_id: id.to_string(),
        name: "Taza".into(),
        image_url: String::new(),
        unit_cents: 1000,
        qty,
    }
}

async fn stock(pool: &PgPool, t: Uuid, id: Uuid) -> Option<u32> {
    products::get(pool, t, id).await.unwrap().unwrap().stock
}

#[sqlx::test(migrations = "../../migrations")]
async fn stock_goes_down_never_below_zero_and_back_on_cancel(pool: PgPool) {
    let t = tenant(&pool, "ana").await;
    let counted = Product {
        name: "Taza".into(),
        stock: Some(3),
        ..Product::default()
    };
    let id = products::create(&pool, t, &counted).await.unwrap();
    let free = products::create(
        &pool,
        t,
        &Product {
            name: "Libre".into(),
            ..Product::default()
        },
    )
    .await
    .unwrap();

    let items = [item(id, 2), item(free, 5)];
    let Created::Saved { id: first, number } =
        orders::create(&pool, t, &order(&items)).await.unwrap()
    else {
        panic!("tenía que alcanzar");
    };
    assert_eq!(number, 1001);
    assert_eq!(stock(&pool, t, id).await, Some(1));
    assert_eq!(stock(&pool, t, free).await, None);

    // Quedan 1: pedir 2 no guarda nada (ni el número).
    let items = [item(id, 2)];
    match orders::create(&pool, t, &order(&items)).await.unwrap() {
        Created::Short { left, .. } => assert_eq!(left, 1),
        Created::Saved { .. } => panic!("vendió de más"),
    }
    assert_eq!(orders::list(&pool, t).await.unwrap().len(), 1);

    let items = [item(id, 1)];
    assert!(matches!(
        orders::create(&pool, t, &order(&items)).await.unwrap(),
        Created::Saved { number: 1002, .. }
    ));
    let p = products::get(&pool, t, id).await.unwrap().unwrap();
    assert_eq!((p.stock, p.in_stock), (Some(0), false));

    // Cancelar devuelve; recuperar vuelve a descontar.
    assert!(orders::set_status(&pool, t, first, OrderStatus::Cancelled)
        .await
        .unwrap());
    assert_eq!(stock(&pool, t, id).await, Some(2));
    assert!(orders::set_status(&pool, t, first, OrderStatus::Paid)
        .await
        .unwrap());
    assert_eq!(stock(&pool, t, id).await, Some(0));
    // Mover entre columnas no toca el stock.
    assert!(orders::set_status(&pool, t, first, OrderStatus::Shipped)
        .await
        .unwrap());
    assert_eq!(stock(&pool, t, id).await, Some(0));
}

#[sqlx::test(migrations = "../../migrations")]
async fn summary_counts_by_column_and_month(pool: PgPool) {
    let t = tenant(&pool, "ana").await;
    let id = products::create(
        &pool,
        t,
        &Product {
            name: "Taza".into(),
            stock: Some(0),
            ..Product::default()
        },
    )
    .await
    .unwrap();
    assert_eq!(products::out_of_stock(&pool, t).await.unwrap(), 1);

    let items = [OrderItem {
        qty: 1,
        ..item(id, 1)
    }];
    let mut ids = vec![];
    for _ in 0..3 {
        // Sin stock contado no se puede comprar; lo liberamos.
        sqlx::query("UPDATE products SET stock = NULL, in_stock = true WHERE id = $1")
            .bind(id)
            .execute(&pool)
            .await
            .unwrap();
        let Created::Saved { id: o, .. } = orders::create(&pool, t, &order(&items)).await.unwrap()
        else {
            panic!("tenía que guardarse");
        };
        ids.push(o);
    }
    orders::set_status(&pool, t, ids[1], OrderStatus::Paid)
        .await
        .unwrap();
    orders::set_status(&pool, t, ids[2], OrderStatus::Cancelled)
        .await
        .unwrap();

    let s = orders::summary(&pool, t).await.unwrap();
    assert_eq!((s.awaiting_payment, s.to_prepare), (1, 1));
    assert_eq!((s.month_orders, s.month_cents), (1, 1000));
    assert_eq!(orders::recent(&pool, t, 2).await.unwrap().len(), 2);
}
