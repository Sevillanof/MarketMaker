-- Stock opcional: NULL = no se cuentan las unidades (vale "Hay stock" /
-- "Sin stock"); un número = quedan tantas, y se descuentan al comprar.
ALTER TABLE products ADD COLUMN stock integer CHECK (stock >= 0);
