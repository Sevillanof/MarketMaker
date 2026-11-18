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
        pay: domain::PayMethod::Coordinate,
        shipment: None,
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

#[sqlx::test(migrations = "../../migrations")]
async fn an_online_order_enters_only_when_paid(pool: PgPool) {
    let t = tenant(&pool, "ana").await;
    let counted = Product {
        name: "Taza".into(),
        stock: Some(3),
        ..Product::default()
    };
    let id = products::create(&pool, t, &counted).await.unwrap();
    let items = [item(id, 2)];
    let attempt = NewOrder {
        status: OrderStatus::PendingPayment,
        pay: domain::PayMethod::MercadoPago,
        ..order(&items)
    };
    let Created::Saved { id: o, number } = orders::create(&pool, t, &attempt).await.unwrap() else {
        panic!("alcanza el stock");
    };
    // Mientras no se paga: sin número, sin descontar y el dueño no lo ve.
    assert_eq!(number, 0);
    assert_eq!(stock(&pool, t, id).await, Some(3));
    assert!(orders::list(&pool, t).await.unwrap().is_empty());
    assert_eq!(orders::summary(&pool, t).await.unwrap().awaiting_payment, 0);
    // Pide más de lo que hay: no deja.
    let too_many = [item(id, 4)];
    assert!(matches!(
        orders::create(
            &pool,
            t,
            &NewOrder {
                status: OrderStatus::PendingPayment,
                ..order(&too_many)
            }
        )
        .await
        .unwrap(),
        Created::Short { left: 3, .. }
    ));

    // Se paga de menos: no entra.
    assert!(!orders::mark_paid(&pool, t, o, "1", 100).await.unwrap());
    // Se aprueba el pago: número, stock y aparece para preparar.
    assert!(orders::mark_paid(&pool, t, o, "2", 2000).await.unwrap());
    assert!(
        !orders::mark_paid(&pool, t, o, "2", 2000).await.unwrap(),
        "una sola vez"
    );
    assert_eq!(stock(&pool, t, id).await, Some(1));
    let listed = orders::list(&pool, t).await.unwrap();
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].number, 1001);
    assert_eq!(listed[0].status, OrderStatus::Paid);
    assert_eq!(listed[0].pay, domain::PayMethod::MercadoPago);
}

#[sqlx::test(migrations = "../../migrations")]
async fn unpaid_attempts_are_dropped_and_manual_orders_cancelled(pool: PgPool) {
    let t = tenant(&pool, "ana").await;
    let counted = Product {
        name: "Taza".into(),
        stock: Some(3),
        ..Product::default()
    };
    let id = products::create(&pool, t, &counted).await.unwrap();
    let items = [item(id, 1)];
    let attempt = NewOrder {
        status: OrderStatus::PendingPayment,
        pay: domain::PayMethod::MercadoPago,
        ..order(&items)
    };
    orders::create(&pool, t, &attempt).await.unwrap();
    orders::create(&pool, t, &order(&items)).await.unwrap();
    assert_eq!(
        stock(&pool, t, id).await,
        Some(2),
        "la transferencia guarda la unidad"
    );
    assert_eq!(
        orders::expire_unpaid(&pool, 48).await.unwrap(),
        0,
        "todavía no"
    );
    sqlx::query("UPDATE orders SET updated_at = now() - interval '49 hours'")
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(orders::expire_unpaid(&pool, 48).await.unwrap(), 2);
    let all: i64 = sqlx::query_scalar("SELECT count(*) FROM orders")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(all, 1, "el intento sin pagar se borra");
    let listed = orders::list(&pool, t).await.unwrap();
    assert_eq!(listed[0].status, OrderStatus::Cancelled);
    assert_eq!(stock(&pool, t, id).await, Some(3));
}

#[sqlx::test(migrations = "../../migrations")]
async fn andreani_orders_keep_their_shipment(pool: PgPool) {
    use domain::{AndreaniService, Shipment, ShipmentState};
    let t = tenant(&pool, "ana").await;
    let id = products::create(
        &pool,
        t,
        &Product {
            name: "Taza".into(),
            ..Product::default()
        },
    )
    .await
    .unwrap();
    let items = [item(id, 1)];
    let shipment = Shipment {
        service: AndreaniService::Branch,
        quote_cents: 450_000,
        dni: "30123456".into(),
        branch_code: "S01".into(),
        branch_name: "Andreani Palermo".into(),
        branch_address: "Av. Santa Fe 3200, CABA".into(),
        state: ShipmentState::Waiting,
        pedido_id: String::new(),
        tracking: String::new(),
        status: String::new(),
        error: String::new(),
    };
    let new = NewOrder {
        delivery: Delivery::Andreani(AndreaniService::Branch),
        postal_code: "1425",
        shipping_cents: 450_000,
        shipment: Some(&shipment),
        ..order(&items)
    };
    let Created::Saved { id: o, .. } = orders::create(&pool, t, &new).await.unwrap() else {
        panic!("guardado");
    };
    let got = orders::get(&pool, t, o).await.unwrap().unwrap();
    assert_eq!(got.delivery, Delivery::Andreani(AndreaniService::Branch));
    assert_eq!(got.shipment, Some(shipment));
    assert_eq!(got.total_cents, 1000 + 450_000);

    // Sin pagar no se crea; pagado, se toma una sola vez (y otra si falla).
    assert!(!orders::claim_shipment(&pool, t, o).await.unwrap());
    assert!(orders::set_status(&pool, t, o, OrderStatus::Paid)
        .await
        .unwrap());
    assert!(orders::claim_shipment(&pool, t, o).await.unwrap());
    assert!(!orders::claim_shipment(&pool, t, o).await.unwrap());
    let mut failed = orders::get(&pool, t, o)
        .await
        .unwrap()
        .unwrap()
        .shipment
        .unwrap();
    assert_eq!(failed.state, ShipmentState::Creating);
    failed.state = ShipmentState::Failed;
    orders::save_shipment(&pool, t, o, &failed).await.unwrap();
    assert!(orders::claim_shipment(&pool, t, o).await.unwrap());
    assert_eq!(orders::tenant_of(&pool, o).await.unwrap(), Some(t));
}
