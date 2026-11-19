-- Cuotas sin interés para mostrar en la tienda y descuento por pagar con
-- transferencia (0 = no se muestra / sin descuento). El pedido guarda el
-- descuento que se aplicó.
ALTER TABLE tenants
    ADD COLUMN installments smallint NOT NULL DEFAULT 0,
    ADD COLUMN transfer_discount smallint NOT NULL DEFAULT 0;
ALTER TABLE orders ADD COLUMN discount_cents bigint NOT NULL DEFAULT 0;
