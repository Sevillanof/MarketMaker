-- Un pedido entra al proceso recién cuando está pagado. Mientras el comprador
-- está en Mercado Pago es solo un intento de compra: no tiene número, no
-- descuenta stock y el dueño no lo ve. El número se le da al aprobarse el pago.
ALTER TABLE orders ALTER COLUMN number DROP NOT NULL;

-- Formas de cobro además de Mercado Pago. Las tiendas que ya existían
-- coordinaban el pago con el comprador: siguen igual.
ALTER TABLE tenants
  ADD COLUMN pay_transfer boolean NOT NULL DEFAULT false,
  ADD COLUMN transfer_info text NOT NULL DEFAULT '',
  ADD COLUMN pay_coordinate boolean NOT NULL DEFAULT false;
UPDATE tenants SET pay_coordinate = true;

-- Cómo paga cada pedido: 'mp', 'transfer' o 'coordinate'.
ALTER TABLE orders ADD COLUMN payment text NOT NULL DEFAULT 'coordinate'
  CHECK (payment IN ('mp', 'transfer', 'coordinate'));
UPDATE orders SET payment = 'mp' WHERE status = 'pending_payment' OR mp_payment_id IS NOT NULL;
