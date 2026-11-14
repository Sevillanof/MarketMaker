-- Esquema inicial: tenants, páginas con versiones del panel y productos.
-- Todas las tablas llevan tenant_id desde ahora, aunque exista un solo tenant.

CREATE TABLE tenants (
  id         uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  slug       text NOT NULL UNIQUE,
  name       text NOT NULL,
  created_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE pages (
  id                 uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  tenant_id          uuid NOT NULL REFERENCES tenants(id),
  slug               text NOT NULL,
  current_version_id uuid,
  UNIQUE (tenant_id, slug)
);

CREATE TABLE page_versions (
  id         uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  page_id    uuid NOT NULL REFERENCES pages(id),
  tenant_id  uuid NOT NULL REFERENCES tenants(id),
  number     int  NOT NULL,
  doc        jsonb NOT NULL,
  created_at timestamptz NOT NULL DEFAULT now(),
  UNIQUE (page_id, number)
);

ALTER TABLE pages
  ADD CONSTRAINT pages_current_version_fk
  FOREIGN KEY (current_version_id) REFERENCES page_versions(id);

CREATE TABLE products (
  id          uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  tenant_id   uuid NOT NULL REFERENCES tenants(id),
  name        text NOT NULL,
  price_cents bigint NOT NULL,
  image_url   text NOT NULL,
  created_at  timestamptz NOT NULL DEFAULT now()
);
