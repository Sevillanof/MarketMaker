-- Cobrar con Mercado Pago: cada tienda vincula su propia cuenta (OAuth) y
-- cada pedido guarda su preferencia de pago y el pago aprobado.
ALTER TABLE tenants
  ADD COLUMN mp_user_id          bigint,
  ADD COLUMN mp_access_token     text,
  ADD COLUMN mp_refresh_token    text,
  ADD COLUMN mp_public_key       text,
  ADD COLUMN mp_token_expires_at timestamptz,
  ADD COLUMN mp_linked_at        timestamptz,
  ADD COLUMN mp_live             boolean NOT NULL DEFAULT false;

-- El `state` de cada "Vincular mi cuenta": se usa una sola vez.
CREATE TABLE mp_oauth_states (
  state      text PRIMARY KEY,
  tenant_id  uuid NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
  created_at timestamptz NOT NULL DEFAULT now()
);

ALTER TABLE orders
  ADD COLUMN mp_preference_id text,
  ADD COLUMN mp_init_point    text,
  ADD COLUMN mp_payment_id    text,
  ADD COLUMN paid_at          timestamptz;
