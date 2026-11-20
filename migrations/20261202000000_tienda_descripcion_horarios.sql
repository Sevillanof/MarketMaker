-- Datos de la tienda: una descripción corta (se ve en el pie) y los horarios
-- de atención (en el pie y en el bloque Contacto).
ALTER TABLE tenants
    ADD COLUMN tagline text NOT NULL DEFAULT '',
    ADD COLUMN hours text NOT NULL DEFAULT '';
