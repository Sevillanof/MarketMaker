-- El orden de las categorías que eligió la tienda (y las creadas con
-- "+ Nueva categoría" que todavía no tienen productos).
ALTER TABLE tenants ADD COLUMN category_order text[] NOT NULL DEFAULT '{}';
