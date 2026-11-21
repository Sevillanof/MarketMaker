-- Equipo de la tienda: además del dueño, la tienda puede tener
-- administradores y editores. Una invitación pendiente se guarda por email
-- hasta que la persona crea su cuenta (o se suma al instante si ya tiene).
ALTER TABLE memberships DROP CONSTRAINT memberships_role_check;
ALTER TABLE memberships
  ADD CONSTRAINT memberships_role_check CHECK (role IN ('owner', 'admin', 'editor'));

CREATE TABLE team_invites (
  tenant_id  uuid NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
  email      text NOT NULL CHECK (email = lower(email)),
  role       text NOT NULL CHECK (role IN ('admin', 'editor')),
  invited_by uuid REFERENCES users(id) ON DELETE SET NULL,
  created_at timestamptz NOT NULL DEFAULT now(),
  sent_at    timestamptz,
  PRIMARY KEY (tenant_id, email)
);
CREATE INDEX team_invites_email_idx ON team_invites (email);

-- Un email habilitado solo por una invitación de equipo puede entrar y
-- trabajar en esa tienda, pero no crear tiendas propias.
ALTER TABLE allowed_emails ADD COLUMN team_only boolean NOT NULL DEFAULT false;
