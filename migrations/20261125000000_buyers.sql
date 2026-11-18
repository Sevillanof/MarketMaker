-- Cuentas de compradores, una por tienda: la misma persona tiene cuentas
-- separadas en tiendas distintas y cada dueño ve solo a sus clientes.
-- Entran sin contraseña, con un enlace que les llega por email. Son
-- independientes de `users` (las cuentas de dueño): una sesión de comprador
-- nunca da acceso al panel ni al editor.

CREATE TABLE buyers (
  id          uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  tenant_id   uuid NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
  email       text NOT NULL CHECK (email = lower(email)),
  name        text NOT NULL DEFAULT '',
  phone       text NOT NULL DEFAULT '',
  -- Dirección guardada, para completar la compra más rápido.
  address     text NOT NULL DEFAULT '',
  postal_code text NOT NULL DEFAULT '',
  city        text NOT NULL DEFAULT '',
  -- Foto de perfil chica (data URL ya reducida en el navegador, máx. 80 KB).
  avatar      text,
  created_at  timestamptz NOT NULL DEFAULT now(),
  UNIQUE (tenant_id, email)
);

-- Enlaces para entrar. Se guarda solo el hash de la llave.
CREATE TABLE buyer_links (
  id         uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  tenant_id  uuid NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
  email      text NOT NULL,
  key_hash   bytea NOT NULL,
  created_at timestamptz NOT NULL DEFAULT now(),
  expires_at timestamptz NOT NULL,
  used_at    timestamptz
);
CREATE INDEX buyer_links_email_idx ON buyer_links (tenant_id, email, created_at DESC);
CREATE INDEX buyer_links_key_idx ON buyer_links (key_hash);

-- Sesiones: la cookie lleva el token y la base solo su hash.
CREATE TABLE buyer_sessions (
  token_hash bytea PRIMARY KEY,
  buyer_id   uuid NOT NULL REFERENCES buyers(id) ON DELETE CASCADE,
  created_at timestamptz NOT NULL DEFAULT now(),
  expires_at timestamptz NOT NULL
);
CREATE INDEX buyer_sessions_buyer_idx ON buyer_sessions (buyer_id);

-- Para encontrar los pedidos de un comprador por su email.
CREATE INDEX orders_buyer_email_idx ON orders (tenant_id, lower(email));
