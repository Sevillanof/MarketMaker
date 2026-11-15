-- Compras: cómo entrega cada tienda y los pedidos que hacen los compradores.

ALTER TABLE tenants
  ADD COLUMN pickup_enabled boolean NOT NULL DEFAULT false,
  ADD COLUMN pickup_address text NOT NULL DEFAULT '',
  ADD COLUMN pickup_hours text NOT NULL DEFAULT '',
  ADD COLUMN shipping_enabled boolean NOT NULL DEFAULT true,
  -- NULL: el costo del envío se coordina con el comprador.
  ADD COLUMN shipping_cents bigint,
  ADD COLUMN shipping_note text NOT NULL DEFAULT 'Lo coordinamos por WhatsApp',
  -- El próximo número de pedido de la tienda (#1001, #1002…).
  ADD COLUMN next_order_number integer NOT NULL DEFAULT 1001;

CREATE TABLE orders (
  -- También es la "llave" de la página del pedido (/pedido/<id>): no se adivina.
  id             uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  tenant_id      uuid NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
  number         integer NOT NULL,
  status         text NOT NULL DEFAULT 'new',
  customer_name  text NOT NULL,
  email          text NOT NULL,
  phone          text NOT NULL,
  delivery       text NOT NULL CHECK (delivery IN ('pickup', 'shipping')),
  address        text NOT NULL DEFAULT '',
  postal_code    text NOT NULL DEFAULT '',
  city           text NOT NULL DEFAULT '',
  notes          text NOT NULL DEFAULT '',
  -- Los productos como estaban al comprar (nombre, foto, precio, cantidad).
  items          jsonb NOT NULL,
  subtotal_cents bigint NOT NULL,
  shipping_cents bigint NOT NULL,
  total_cents    bigint NOT NULL,
  created_at     timestamptz NOT NULL DEFAULT now(),
  updated_at     timestamptz NOT NULL DEFAULT now(),
  UNIQUE (tenant_id, number)
);

CREATE INDEX orders_tenant_created_idx ON orders (tenant_id, created_at DESC);
