use db::analytics;

#[sqlx::test(migrations = "../../migrations")]
async fn funnels_and_visits(pool: sqlx::PgPool) {
    // Una cuenta sin tienda y una visita de comprador que abandona en Entrega.
    sqlx::query("INSERT INTO users (email) VALUES ('a@x.com')")
        .execute(&pool)
        .await
        .unwrap();
    for (v, step) in [
        ("aaaa", "tienda"),
        ("aaaa", "carrito"),
        ("aaaa", "comprar"),
        ("aaaa", "entrega"),
        ("bbbb", "tienda"),
        ("bbbb", "pedido"),
    ] {
        analytics::record(&pool, v, "buyer", step, Some("demo"), "celular", None)
            .await
            .unwrap();
    }
    let owner = analytics::owner_funnel(&pool, 30).await.unwrap();
    assert_eq!(owner, vec![1, 0, 0, 0, 0, 0, 0]);
    let buyer = analytics::buyer_funnel(&pool, 30).await.unwrap();
    assert_eq!(buyer, vec![2, 0, 1, 1, 1, 0, 0]);
    let all = analytics::recent_visits(&pool, "buyer", 30, false, 10).await.unwrap();
    assert_eq!(all.len(), 2);
    let dropped = analytics::recent_visits(&pool, "buyer", 30, true, 10).await.unwrap();
    assert_eq!(dropped.len(), 1);
    assert_eq!(dropped[0].visita, "aaaa");
    assert_eq!(dropped[0].ultimo, "entrega");
    let steps = analytics::visit_steps(&pool, "aaaa", "buyer").await.unwrap();
    assert_eq!(steps.len(), 4);
    assert_eq!(analytics::purge(&pool).await.unwrap(), 0);
    assert_ne!(
        analytics::visit_code("1.1.1.1", "UA"),
        analytics::visit_code("1.1.1.2", "UA")
    );
}
