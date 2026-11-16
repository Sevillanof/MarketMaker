-- Cuentas con contraseña: la persona entra con email y contraseña y el código
-- por email queda para confirmar el email al crear la cuenta y para cuando se
-- olvida la contraseña. Las cuentas viejas no tienen contraseña hasta que
-- eligen una.
ALTER TABLE users
  ADD COLUMN name          text NOT NULL DEFAULT '',
  ADD COLUMN password_hash text,
  -- Intentos fallidos seguidos; al llegar al límite la cuenta espera un rato.
  ADD COLUMN failed_logins int NOT NULL DEFAULT 0,
  ADD COLUMN locked_until  timestamptz;
