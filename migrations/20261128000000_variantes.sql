-- Colores y talles de cada producto, con el stock de cada combinación
-- (ver domain::Variants). Vacío: el producto se vende como siempre. Con
-- variantes, `stock` es la suma de las combinaciones.
ALTER TABLE products ADD COLUMN variants jsonb NOT NULL DEFAULT '{}';
