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
        discount_cents: 0,
        coupon: None,
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
        ..Default::default()
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
        Created::Saved { .. } | Created::CouponGone => panic!("vendió de más"),
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
        discount_cents: 0,
        coupon: None,
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

#[sqlx::test(migrations = "../../migrations")]
async fn the_transfer_discount_is_saved_and_lowers_the_total(pool: PgPool) {
    let t = tenant(&pool, "eva").await;
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
    let items = [item(id, 2)];
    let new = NewOrder {
        pay: domain::PayMethod::Transfer,
        shipping_cents: 500,
        discount_cents: 200,
        coupon: None,
        ..order(&items)
    };
    let Created::Saved { id: o, .. } = orders::create(&pool, t, &new).await.unwrap() else {
        panic!("guardado");
    };
    let got = orders::get(&pool, t, o).await.unwrap().unwrap();
    assert_eq!((got.subtotal_cents, got.discount_cents), (2000, 200));
    assert_eq!(got.total_cents, 2000 - 200 + 500);

    // Cobros guarda las cuotas y el descuento, y la tienda los muestra
    // solo si puede cobrarlos.
    let mut p = domain::PaymentOptions {
        transfer: true,
        transfer_alias: "mi.alias".into(),
        installments: 3,
        transfer_discount: 10,
        ..Default::default()
    };
    orders::save_payment_options(&pool, t, &p).await.unwrap();
    assert_eq!(orders::payment_options(&pool, t).await.unwrap(), p);
    let info = db::auth::store_info(&pool, t).await.unwrap();
    // Sin Mercado Pago vinculado no hay cuotas para mostrar.
    assert_eq!(
        (info.offers.installments, info.offers.transfer_discount),
        (0, 10)
    );
    p.transfer = false;
    orders::save_payment_options(&pool, t, &p).await.unwrap();
    let info = db::auth::store_info(&pool, t).await.unwrap();
    assert_eq!(info.offers.transfer_discount, 0);
}

/// Con colores y talles, se descuenta la combinación comprada (y el total
/// del producto), nunca se vende de más y vuelve al cancelar.
#[sqlx::test(migrations = "../../migrations")]
async fn variant_stock_goes_down_per_combo_and_back_on_cancel(pool: PgPool) {
    let t = tenant(&pool, "zapas").await;
    let mut p = Product {
        name: "Zapatillas".into(),
        price_cents: 5_000_000,
        ..Product::default()
    };
    p.variants = domain::Variants {
        colors: vec![
            domain::VariantColor {
                name: "Azul".into(),
                ..Default::default()
            },
            domain::VariantColor {
                name: "Rojo".into(),
                ..Default::default()
            },
        ],
        sizes: vec!["40".into(), "41".into()],
        stock: Vec::new(),
    };
    p.variants.set_qty("Azul", "40", 2);
    p.variants.set_qty("Rojo", "41", 5);
    assert!(domain::tidy_product(&mut p).is_empty());
    assert_eq!(p.stock, Some(7));
    let id = products::create(&pool, t, &p).await.unwrap();
    let combo = |color: &str, size: &str, qty: u32| OrderItem {
        color: color.into(),
        size: size.into(),
        ..item(id, qty)
    };

    let items = [combo("Azul", "40", 2)];
    let Created::Saved { id: first, .. } = orders::create(&pool, t, &order(&items)).await.unwrap()
    else {
        panic!("tenía que alcanzar");
    };
    let got = products::get(&pool, t, id).await.unwrap().unwrap();
    assert_eq!(got.variants.qty("Azul", "40"), 0);
    assert_eq!(got.variants.qty("Rojo", "41"), 5);
    assert_eq!(got.stock, Some(5));

    // No quedan Azul 40: no se guarda nada.
    let items = [combo("Rojo", "41", 1), combo("Azul", "40", 1)];
    match orders::create(&pool, t, &order(&items)).await.unwrap() {
        Created::Short { left, .. } => assert_eq!(left, 0),
        Created::Saved { .. } | Created::CouponGone => panic!("no alcanzaba"),
    }
    let got = products::get(&pool, t, id).await.unwrap().unwrap();
    assert_eq!(
        got.variants.qty("Rojo", "41"),
        5,
        "el otro renglón tampoco se descontó"
    );

    // Cancelar devuelve las unidades a su combinación.
    assert!(orders::set_status(&pool, t, first, OrderStatus::Cancelled)
        .await
        .unwrap());
    let got = products::get(&pool, t, id).await.unwrap().unwrap();
    assert_eq!(got.variants.qty("Azul", "40"), 2);
    assert_eq!(got.stock, Some(7));
}

#[sqlx::test(migrations = "../../migrations")]
async fn a_coupon_counts_its_uses_stops_at_the_limit_and_gives_the_use_back(pool: PgPool) {
    use db::coupons::{self, Saved};
    use domain::{Coupon, CouponKind};
    let t = tenant(&pool, "uma").await;
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
    let c = Coupon {
        code: "VERANO10".into(),
        kind: CouponKind::Percent,
        value: 10,
        max_uses: 1,
        ..Coupon::default()
    };
    assert_eq!(coupons::save(&pool, t, &c).await.unwrap(), Saved::Saved);
    assert_eq!(coupons::save(&pool, t, &c).await.unwrap(), Saved::CodeTaken);

    let items = [item(id, 2)];
    let with_coupon = NewOrder {
        coupon: Some(("VERANO10", 200)),
        ..order(&items)
    };
    let Created::Saved { id: first, .. } = orders::create(&pool, t, &with_coupon).await.unwrap()
    else {
        panic!("guardado");
    };
    let got = orders::get(&pool, t, first).await.unwrap().unwrap();
    assert_eq!(
        (got.coupon_code.as_str(), got.coupon_cents, got.total_cents),
        ("VERANO10", 200, 1800)
    );
    assert_eq!(
        coupons::by_code(&pool, t, "VERANO10")
            .await
            .unwrap()
            .unwrap()
            .uses,
        1
    );

    // Se acabaron los usos: el siguiente pedido no se guarda.
    assert!(matches!(
        orders::create(&pool, t, &with_coupon).await.unwrap(),
        Created::CouponGone
    ));

    // Al cancelar el pedido, el uso vuelve.
    assert!(orders::set_status(&pool, t, first, OrderStatus::Cancelled)
        .await
        .unwrap());
    assert_eq!(
        coupons::by_code(&pool, t, "VERANO10")
            .await
            .unwrap()
            .unwrap()
            .uses,
        0
    );
    assert!(matches!(
        orders::create(&pool, t, &with_coupon).await.unwrap(),
        Created::Saved { .. }
    ));
}

#[sqlx::test(migrations = "../../migrations")]
async fn two_payments_for_the_last_unit_are_noted(pool: PgPool) {
    let t = tenant(&pool, "ana").await;
    let last = Product {
        name: "Taza".into(),
        stock: Some(1),
        ..Product::default()
    };
    let id = products::create(&pool, t, &last).await.unwrap();
    let items = [item(id, 1)];
    let attempt = NewOrder {
        status: OrderStatus::PendingPayment,
        pay: domain::PayMethod::MercadoPago,
        ..order(&items)
    };
    // Dos personas llegan a pagar la última: el intento no reserva.
    let Created::Saved { id: first, .. } = orders::create(&pool, t, &attempt).await.unwrap() else {
        panic!("alcanza el stock");
    };
    let Created::Saved { id: second, .. } = orders::create(&pool, t, &attempt).await.unwrap()
    else {
        panic!("alcanza el stock");
    };
    // El primer pago entra limpio.
    let short = orders::mark_paid_noting(&pool, t, first, "1", 1000)
        .await
        .unwrap();
    assert_eq!(short, Some(vec![]));
    // El segundo también entra (ya está cobrado), pero queda anotado qué faltó.
    let short = orders::mark_paid_noting(&pool, t, second, "2", 1000)
        .await
        .unwrap()
        .expect("el pago entra igual");
    assert_eq!(short.len(), 1);
    assert_eq!((short[0].sold, short[0].had), (1, 0));
    assert_eq!(stock(&pool, t, id).await, Some(0));
    // Repetido: no marca nada.
    assert_eq!(
        orders::mark_paid_noting(&pool, t, second, "2", 1000)
            .await
            .unwrap(),
        None
    );
}
