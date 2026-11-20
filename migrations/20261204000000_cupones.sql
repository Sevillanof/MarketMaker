-- Cupones de descuento: el dueño los crea en el panel (Cupones) y el
-- comprador escribe el código al pagar. `uses` cuenta los pedidos que lo
-- usaron (con número, sin cancelar).
CREATE TABLE coupons (
  id         uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  tenant_id  uuid NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
  code       text NOT NULL,
  kind       text NOT NULL CHECK (kind IN ('percent', 'amount')),
  value      bigint NOT NULL CHECK (value > 0),
  min_cents  bigint NOT NULL DEFAULT 0,
  expires    date,
  max_uses   integer,
  uses       integer NOT NULL DEFAULT 0,
  active     boolean NOT NULL DEFAULT true,
  created_at timestamptz NOT NULL DEFAULT now(),
  UNIQUE (tenant_id, code)
);

-- Qué cupón usó cada pedido y cuánto descontó (aparte del descuento por
-- transferencia).
ALTER TABLE orders
  ADD COLUMN coupon_code text NOT NULL DEFAULT '',
  ADD COLUMN coupon_cents bigint NOT NULL DEFAULT 0;
