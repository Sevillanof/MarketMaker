-- Cuentas de dueño (fase 1): usuarios, códigos para entrar, sesiones y
-- quién es dueño de cada tienda. Los visitantes no tienen cuenta.
-- Qué emails pueden entrar se decide con ALLOWED_EMAILS (ver README).

CREATE TABLE users (
  id         uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  email      text NOT NULL UNIQUE CHECK (email = lower(email)),
  created_at timestamptz NOT NULL DEFAULT now()
);

-- Un rol por persona y tienda. Por ahora solo hay dueños; los colaboradores
-- llegarán con otro valor en `role`.
CREATE TABLE memberships (
  tenant_id  uuid NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
  user_id    uuid NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  role       text NOT NULL CHECK (role IN ('owner')),
  created_at timestamptz NOT NULL DEFAULT now(),
  PRIMARY KEY (tenant_id, user_id)
);

-- Códigos de 6 dígitos enviados por email. Se guarda solo su hash.
CREATE TABLE login_codes (
  id         uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  email      text NOT NULL,
  code_hash  bytea NOT NULL,
  created_at timestamptz NOT NULL DEFAULT now(),
  expires_at timestamptz NOT NULL,
  attempts   int NOT NULL DEFAULT 0,
  used_at    timestamptz
);
CREATE INDEX login_codes_email_idx ON login_codes (email, created_at DESC);

-- Sesiones: la cookie lleva el token y la base solo su hash.
CREATE TABLE sessions (
  token_hash bytea PRIMARY KEY,
  user_id    uuid NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  created_at timestamptz NOT NULL DEFAULT now(),
  expires_at timestamptz NOT NULL
);
CREATE INDEX sessions_user_idx ON sessions (user_id);

ALTER TABLE page_versions ADD COLUMN created_by uuid REFERENCES users(id) ON DELETE SET NULL;
