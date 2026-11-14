-- Schema 2 del panel: el tamaño pasa de una opción fija ("size": "full") a
-- ancho y alto libres ("size": {"w": 12, "h": 4}). Se convierten todas las
-- versiones guardadas con la tabla de opciones de entonces. El servidor
-- también convierte al leer (domain::parse_doc), por si llega algo viejo.
--
-- Además, las imágenes de ejemplo dejan de venir de picsum.photos y se sirven
-- desde public/img (AVIF con respaldo WebP).

WITH legacy(type, preset, w, h) AS (VALUES
  ('Banner', 'full', 12, 4), ('Banner', 'half', 6, 4),
  ('Text', 'small', 4, 2), ('Text', 'medium', 6, 3), ('Text', 'full', 12, 2),
  ('ProductGrid', 'one_row', 12, 5), ('ProductGrid', 'two_rows', 12, 9), ('ProductGrid', 'half', 6, 9),
  ('FeaturedProduct', 'small', 3, 4), ('FeaturedProduct', 'medium', 4, 5), ('FeaturedProduct', 'large', 6, 5)
)
UPDATE page_versions v
SET doc = jsonb_set(
  jsonb_set(v.doc, '{schema}', '2'),
  '{nodes}',
  COALESCE((
    SELECT jsonb_agg(
      jsonb_set(n, '{size}', jsonb_build_object('w', COALESCE(l.w, 12), 'h', COALESCE(l.h, 4)))
      ORDER BY ord)
    FROM jsonb_array_elements(v.doc -> 'nodes') WITH ORDINALITY AS e(n, ord)
    LEFT JOIN legacy l ON l.type = n ->> 'type' AND l.preset = n ->> 'size'
  ), '[]'::jsonb)
)
WHERE (v.doc ->> 'schema')::int = 1;

UPDATE page_versions
SET doc = replace(replace(doc::text,
  'https://picsum.photos/seed/marketmaker-hero/1600/600', '/img/hero.webp'),
  'https://picsum.photos/seed/marketmaker-banner/1600/600', '/img/banner.webp')::jsonb
WHERE doc::text LIKE '%picsum.photos/seed/marketmaker-%';

UPDATE products
SET image_url = regexp_replace(image_url,
  '^https://picsum\.photos/seed/mk-product-([0-9]+)/600/400$', '/img/product-\1.webp')
WHERE image_url LIKE 'https://picsum.photos/seed/mk-product-%';
