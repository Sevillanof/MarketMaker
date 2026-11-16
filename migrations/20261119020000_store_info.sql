-- Datos de la tienda (auditoría, tanda 4): logo y contacto. El menú y el pie
-- del diseño los usan cuando sus campos quedan vacíos.
ALTER TABLE tenants
  ADD COLUMN logo_id uuid REFERENCES media(id) ON DELETE SET NULL,
  ADD COLUMN whatsapp text NOT NULL DEFAULT '',
  ADD COLUMN instagram text NOT NULL DEFAULT '',
  ADD COLUMN contact_email text NOT NULL DEFAULT '';
