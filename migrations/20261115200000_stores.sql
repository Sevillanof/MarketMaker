-- Varias tiendas: cada una en su dirección (nombre.localhost).
-- Pasar del panel de la plataforma al editor de una tienda usa un pase de un
-- solo uso: la sesión de cada dirección es propia, así que el pase abre una
-- sesión nueva en la tienda sin pedir otro código.
CREATE TABLE login_handoffs (
  token_hash bytea PRIMARY KEY,
  user_id    uuid NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  tenant_id  uuid NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
  expires_at timestamptz NOT NULL,
  used_at    timestamptz
);

ALTER TABLE tenants ADD COLUMN created_by uuid REFERENCES users(id) ON DELETE SET NULL;
