-- Productos: el dueño carga su catálogo. Cada producto suma precio anterior
-- (tachado), si hay stock, si se muestra en la tienda, varias fotos (la
-- primera es la principal y queda también en image_url) y etiquetas para
-- encontrarlo y agruparlo.

ALTER TABLE products
  ADD COLUMN compare_at_cents bigint NOT NULL DEFAULT 0,
  ADD COLUMN in_stock boolean NOT NULL DEFAULT true,
  ADD COLUMN visible boolean NOT NULL DEFAULT true,
  ADD COLUMN photos text[] NOT NULL DEFAULT '{}',
  ADD COLUMN tags text[] NOT NULL DEFAULT '{}',
  ADD COLUMN updated_at timestamptz NOT NULL DEFAULT now();

UPDATE products SET photos = ARRAY[image_url] WHERE image_url <> '' AND photos = '{}';

-- Buscar productos por etiqueta (`tags @> ARRAY['regalo']`).
CREATE INDEX products_tags_idx ON products USING gin (tags);

-- Etiquetas de ejemplo para los productos de muestra.
UPDATE products p
SET tags = x.tags
FROM (VALUES
  ('Auriculares inalámbricos', ARRAY['tecnología', 'audio']),
  ('Mochila urbana',           ARRAY['accesorios', 'viaje']),
  ('Lámpara de escritorio',    ARRAY['hogar', 'oficina']),
  ('Taza de cerámica',         ARRAY['hogar', 'cocina', 'regalo']),
  ('Zapatillas running',       ARRAY['deporte', 'calzado']),
  ('Reloj analógico',          ARRAY['accesorios', 'regalo']),
  ('Cuaderno de tapa dura',    ARRAY['oficina', 'regalo']),
  ('Parlante portátil',        ARRAY['tecnología', 'audio'])
) AS x(name, tags)
WHERE p.name = x.name AND p.tags = '{}';
