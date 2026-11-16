-- Tienda nueva honesta (auditoría, tanda 2).
-- Productos de ejemplo: los copia la tienda nueva para que el diseño no se vea
-- vacío, pero los compradores nunca los ven ni los pueden comprar.
ALTER TABLE products ADD COLUMN example boolean NOT NULL DEFAULT false;

-- Si la persona guardó Entregas alguna vez. Las tiendas que ya existen ya
-- tenían Entregas en uso: cuentan como guardadas.
ALTER TABLE tenants ADD COLUMN delivery_set boolean NOT NULL DEFAULT false;
UPDATE tenants SET delivery_set = true;
