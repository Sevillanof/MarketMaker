-- Bienvenida de la primera vez: se muestra una sola vez por cuenta nueva.
-- Las cuentas que ya existen no la ven.
ALTER TABLE users ADD COLUMN welcome_seen boolean NOT NULL DEFAULT false;
UPDATE users SET welcome_seen = true;
