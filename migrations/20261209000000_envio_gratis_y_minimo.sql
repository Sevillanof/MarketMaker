-- Envío gratis desde un monto y pedido mínimo, por tienda (centavos; NULL = sin).
ALTER TABLE tenants
    ADD COLUMN free_from_cents BIGINT CHECK (free_from_cents > 0),
    ADD COLUMN min_order_cents BIGINT CHECK (min_order_cents > 0);
