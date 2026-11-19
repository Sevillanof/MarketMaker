//! Envíos con Andreani de punta a punta contra un Andreani de mentira que
//! se puede romper a pedido: crear, reintentar, seguir y no duplicar.
#![cfg(feature = "ssr")]

use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::routing::{get, post};
use axum::{Json, Router};
use domain::*;
use serde_json::{json, Value};
use sqlx::PgPool;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

#[derive(Default)]
struct Mock {
    logins: u32,
    registrations: u32,
    /// "ok", "500", "slow", "expired" (el primer token no vale), "nophone"
    reg_mode: String,
    polls: u32,
    track_mode: String,
}
type S = Arc<Mutex<Mock>>;

async fn serve(state: S) -> String {
    let app = Router::new()
        .route(
            "/api/v1/Login",
            post(|State(s): State<S>| async move {
                s.lock().unwrap().logins += 1;
                Json(json!({"response": {"accessToken": "tok", "clientType": "pyme", "razonSocial": "T",
                    "contratos": [{"id": 11, "numeroDeContrato": "1", "modoDeEntregaNombre": "A domicilio", "tipoDeEnvioNombre": "paqueteria"}]}}))
            }),
        )
        .route(
            "/api/v1/Pyme/ShippingRegistration",
            post(|State(s): State<S>, h: HeaderMap, Json(_b): Json<Value>| async move {
                let mode = s.lock().unwrap().reg_mode.clone();
                if mode == "expired" && h.get("x-auth-token").and_then(|v| v.to_str().ok()) == Some("viejo") {
                    return (StatusCode::UNAUTHORIZED, Json(json!({"message": "vencido"})));
                }
                if mode == "500" {
                    return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"message": "Error interno"})));
                }
                if mode == "slow" {
                    tokio::time::sleep(std::time::Duration::from_secs(20)).await;
                }
                let mut m = s.lock().unwrap();
                m.registrations += 1;
                let id = 1000 + m.registrations;
                (StatusCode::OK, Json(json!({"response": {"pedidoId": id.to_string()}})))
            }),
        )
        .route(
            "/api/v1/Shipments/ByOrderNumbers",
            post(|State(s): State<S>, Json(b): Json<Value>| async move {
                let mut m = s.lock().unwrap();
                m.polls += 1;
                let p = m.polls;
                let mode = m.track_mode.clone();
                let out: Vec<Value> = b["salesOrderNumbers"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|id| {
                        let (t, st) = match (mode.as_str(), p) {
                            ("unpaid", _) => ("", ""),
                            (_, 1) => ("360001", "pendiente de ingreso"),
                            (_, 2) => ("360001", "En camino"),
                            _ => ("360001", "Entregado"),
                        };
                        json!({"salesOrderNumber": id, "trackingNumber": t, "trackingStatus": st})
                    })
                    .collect();
                Json(json!({"response": out}))
            }),
        )
        .route("/api/v1/Pyme/ticket", post(|| async { "%PDF-1.4 etiqueta" }))
        .route("/ping", get(|| async { "ok" }))
        .with_state(state);
    let l = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", l.local_addr().unwrap());
    tokio::spawn(async move { axum::serve(l, app).await.unwrap() });
    url
}

async fn setup(pool: &PgPool, slug: &str) -> Uuid {
    let t: Uuid =
        sqlx::query_scalar("INSERT INTO tenants (slug, name) VALUES ($1, $1) RETURNING id")
            .bind(slug)
            .fetch_one(pool)
            .await
            .unwrap();
    db::andreani::connect(
        pool,
        t,
        &db::andreani::Login {
            credential: "cHltZXxhYmM=",
            access_token: "viejo",
            client_type: "pyme",
            holder: "T",
            cuit: "1",
            email: "dueno@x.com",
            contracts: &[AndreaniContract {
                id: "11".into(),
                number: "1".into(),
                service: AndreaniService::Home,
            }],
        },
    )
    .await
    .unwrap();
    db::andreani::save_settings(
        pool,
        t,
        &AndreaniSettings {
            enabled: true,
            origin: AndreaniOrigin {
                street: "Calle".into(),
                number: "1".into(),
                floor: String::new(),
                city: "CABA".into(),
                province: "CABA".into(),
                postal_code: "1425".into(),
                sender: "T".into(),
            },
            services: vec![AndreaniService::Home],
            parcel: Parcel::default(),
        },
    )
    .await
    .unwrap();
    t
}

/// Un pedido por Andreani, ya pagado.
async fn paid_order(pool: &PgPool, t: Uuid) -> Uuid {
    let p = db::products::create(
        pool,
        t,
        &Product {
            name: "Taza".into(),
            stock: Some(5),
            ..Product::default()
        },
    )
    .await
    .unwrap();
    let items = [OrderItem {
        product_id: p.to_string(),
        name: "Taza".into(),
        image_url: String::new(),
        unit_cents: 150000,
        qty: 1,
        ..Default::default()
    }];
    let sh = Shipment {
        service: AndreaniService::Home,
        quote_cents: 500000,
        dni: "30111222".into(),
        branch_code: String::new(),
        branch_name: String::new(),
        branch_address: String::new(),
        state: ShipmentState::Waiting,
        pedido_id: String::new(),
        tracking: String::new(),
        status: String::new(),
        error: String::new(),
    };
    let db::orders::Created::Saved { id, .. } = db::orders::create(
        pool,
        t,
        &db::orders::NewOrder {
            status: OrderStatus::PendingPayment,
            customer_name: "Ana Pérez",
            email: "a@b.com",
            phone: "11 5555-1234",
            delivery: Delivery::Andreani(AndreaniService::Home),
            address: "Av. Santa Fe 3200 piso 2",
            postal_code: "1425",
            city: "CABA",
            notes: "",
            items: &items,
            shipping_cents: 500000,
            discount_cents: 0,
            pay: PayMethod::MercadoPago,
            shipment: Some(&sh),
        },
    )
    .await
    .unwrap() else {
        panic!("stock")
    };
    assert!(db::orders::mark_paid(pool, t, id, "123", 650000)
        .await
        .unwrap());
    id
}

#[sqlx::test(migrations = "../../migrations")]
async fn full_flow(pool: PgPool) {
    let state: S = Default::default();
    let url = serve(state.clone()).await;
    std::env::set_var("ANDREANI_API_URL", &url);
    let t = setup(&pool, "tienda").await;

    // 1) camino feliz con token vencido: relogin y un solo envío.
    state.lock().unwrap().reg_mode = "expired".into();
    let o = paid_order(&pool, t).await;
    let a = web::shipments::ship_order(&pool, t, o);
    let b = web::shipments::ship_order(&pool, t, o);
    let (a, b) = tokio::join!(a, b);
    let sh = a.unwrap().shipment.unwrap();
    println!(
        "A: {:?}  B: {:?}",
        sh.state,
        b.map(|o| o.shipment.unwrap().state)
    );
    assert_eq!(sh.state, ShipmentState::Created, "{sh:?}");
    assert_eq!(state.lock().unwrap().registrations, 1, "duplicó el envío");

    // seguimiento: pendiente → en camino → entregado
    let get = || async { db::orders::get(&pool, t, o).await.unwrap().unwrap() };
    web::shipments::refresh(&pool, t, &[get().await]).await;
    let x = get().await;
    println!(
        "poll1: {:?} {:?}",
        x.status,
        x.shipment.as_ref().unwrap().status
    );
    web::shipments::refresh(&pool, t, &[get().await]).await;
    let x = get().await;
    println!(
        "poll2: {:?} {:?}",
        x.status,
        x.shipment.as_ref().unwrap().status
    );
    assert_eq!(x.status, OrderStatus::Shipped);
    web::shipments::refresh(&pool, t, &[get().await]).await;
    assert_eq!(get().await.status, OrderStatus::Delivered);

    // 2) Andreani con error 500: queda Failed, y reintentar funciona.
    state.lock().unwrap().reg_mode = "500".into();
    let o2 = paid_order(&pool, t).await;
    let sh = web::shipments::ship_order(&pool, t, o2)
        .await
        .unwrap()
        .shipment
        .unwrap();
    println!("500: {:?} {}", sh.state, sh.error);
    assert_eq!(sh.state, ShipmentState::Failed);
    state.lock().unwrap().reg_mode = "ok".into();
    let sh = web::shipments::ship_order(&pool, t, o2)
        .await
        .unwrap()
        .shipment
        .unwrap();
    assert_eq!(sh.state, ShipmentState::Created);
    // reintentar sobre uno ya creado no crea otro
    let before = state.lock().unwrap().registrations;
    web::shipments::ship_order(&pool, t, o2).await;
    assert_eq!(state.lock().unwrap().registrations, before);

    // 3) estado "Creating" huérfano (caída del servidor a mitad de camino).
    let o3 = paid_order(&pool, t).await;
    assert!(db::orders::claim_shipment(&pool, t, o3).await.unwrap());
    let sh = web::shipments::ship_order(&pool, t, o3)
        .await
        .unwrap()
        .shipment
        .unwrap();
    println!("huérfano: {:?}", sh.state);
    assert_eq!(sh.state, ShipmentState::Creating);
    // al arrancar de nuevo, pasa a "falló" y se puede reintentar
    assert_eq!(db::orders::recover_stuck_shipments(&pool).await.unwrap(), 1);
    let sh = web::shipments::ship_order(&pool, t, o3)
        .await
        .unwrap()
        .shipment
        .unwrap();
    assert_eq!(sh.state, ShipmentState::Created, "{sh:?}");
    // pagado que nunca se creó
    let o4 = paid_order(&pool, t).await;
    sqlx::query("UPDATE orders SET paid_at = now() - interval '5 minutes' WHERE id = $1")
        .bind(o4)
        .execute(&pool)
        .await
        .unwrap();
    let w = db::orders::shipments_waiting(&pool).await.unwrap();
    assert!(w.iter().any(|(_, id)| *id == o4));
}
