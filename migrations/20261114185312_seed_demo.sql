-- Datos de ejemplo: tenant `demo`, página `home` con su versión 1 y ocho productos.

INSERT INTO tenants (slug, name) VALUES ('demo', 'Demo');

INSERT INTO pages (tenant_id, slug)
SELECT id, 'home' FROM tenants WHERE slug = 'demo';

INSERT INTO page_versions (page_id, tenant_id, number, doc)
SELECT p.id, p.tenant_id, 1, '{
  "schema": 1,
  "theme": "light",
  "nodes": [
    { "id": "n1", "type": "Banner",
      "props": { "title": "Ofertas de la semana", "subtitle": "", "image_url": "https://picsum.photos/seed/marketmaker-hero/1600/600",
                 "button_label": "", "button_href": "" },
      "pos": { "x": 0, "y": 0 }, "size": "full" },
    { "id": "n2", "type": "ProductGrid",
      "props": { "title": "Destacados", "limit": 8 },
      "pos": { "x": 0, "y": 4 }, "size": "one_row" }
  ]
}'::jsonb
FROM pages p JOIN tenants t ON t.id = p.tenant_id
WHERE t.slug = 'demo' AND p.slug = 'home';

UPDATE pages p SET current_version_id = v.id
FROM page_versions v
WHERE v.page_id = p.id AND v.number = 1;

-- created_at escalonado para que el orden de los productos sea estable.
INSERT INTO products (tenant_id, name, price_cents, image_url, created_at)
SELECT t.id, x.name, x.price_cents, x.image_url, now() + (x.ord * interval '1 second')
FROM tenants t,
(VALUES
  (1, 'Auriculares inalámbricos', 4599900, 'https://picsum.photos/seed/mk-product-1/600/400'),
  (2, 'Mochila urbana',           2899000, 'https://picsum.photos/seed/mk-product-2/600/400'),
  (3, 'Lámpara de escritorio',    1850000, 'https://picsum.photos/seed/mk-product-3/600/400'),
  (4, 'Taza de cerámica',          650000, 'https://picsum.photos/seed/mk-product-4/600/400'),
  (5, 'Zapatillas running',       8999900, 'https://picsum.photos/seed/mk-product-5/600/400'),
  (6, 'Reloj analógico',          5420000, 'https://picsum.photos/seed/mk-product-6/600/400'),
  (7, 'Cuaderno de tapa dura',     399000, 'https://picsum.photos/seed/mk-product-7/600/400'),
  (8, 'Parlante portátil',        3275050, 'https://picsum.photos/seed/mk-product-8/600/400')
) AS x(ord, name, price_cents, image_url)
WHERE t.slug = 'demo';
