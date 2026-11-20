-- Categoría y subcategoría de cada producto (una sola de cada una). Las
-- etiquetas siguen aparte: son libres y se pueden poner varias.
ALTER TABLE products
  ADD COLUMN category text NOT NULL DEFAULT '',
  ADD COLUMN subcategory text NOT NULL DEFAULT '';

CREATE INDEX products_category_idx ON products (tenant_id, category) WHERE category <> '';
