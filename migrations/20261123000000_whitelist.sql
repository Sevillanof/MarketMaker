-- Lista de emails que pueden tener cuenta, editable desde la web (Whitelist).
-- Se suma a ALLOWED_EMAILS del servidor, que sigue valiendo como base:
-- 'allowed' agrega un email; 'removed' quita uno que viene de ALLOWED_EMAILS.
CREATE TABLE allowed_emails (
  email      text PRIMARY KEY CHECK (email = lower(email)),
  status     text NOT NULL CHECK (status IN ('allowed', 'removed')),
  added_by   uuid REFERENCES users(id) ON DELETE SET NULL,
  created_at timestamptz NOT NULL DEFAULT now(),
  updated_at timestamptz NOT NULL DEFAULT now()
);
