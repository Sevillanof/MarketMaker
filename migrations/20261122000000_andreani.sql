-- Envíos con Andreani: peso y medidas de cada producto, y la cuenta de
-- Andreani de cada tienda (conectada con el código de WooCommerce).
ALTER TABLE products ADD COLUMN parcel jsonb;

CREATE TABLE andreani_accounts (
    tenant_id uuid PRIMARY KEY REFERENCES tenants (id) ON DELETE CASCADE,
    -- El "Credential ID" que pega el dueño, y el token que da Andreani al entrar.
    credential text NOT NULL,
    access_token text NOT NULL DEFAULT '',
    client_type text NOT NULL DEFAULT 'pyme',
    holder text NOT NULL DEFAULT '',
    cuit text NOT NULL DEFAULT '',
    email text NOT NULL DEFAULT '',
    -- Los contratos de la cuenta, tal como los devuelve Andreani.
    contracts jsonb NOT NULL DEFAULT '[]',
    -- Lo que elige el dueño (domain::AndreaniSettings), sin la parte de la cuenta.
    settings jsonb,
    connected_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now()
);
