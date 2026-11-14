-- Schema 3 del panel: la grilla pasa de 12 columnas y filas de 96 px a una
-- grilla fina de 48 columnas y filas de 8 px. Cada celda de antes equivale a
-- 4 columnas por 12 filas, así que se multiplican posición y tamaño y las
-- páginas se ven igual que antes. El servidor también convierte al leer
-- (domain::parse_doc), por si llega algo viejo.

UPDATE page_versions v
SET doc = jsonb_set(
  jsonb_set(v.doc, '{schema}', '3'),
  '{nodes}',
  COALESCE((
    SELECT jsonb_agg(
      n || jsonb_build_object(
        'pos', jsonb_build_object(
          'x', (n #>> '{pos,x}')::int * 4,
          'y', (n #>> '{pos,y}')::int * 12),
        'size', jsonb_build_object(
          'w', (n #>> '{size,w}')::int * 4,
          'h', (n #>> '{size,h}')::int * 12))
      ORDER BY ord)
    FROM jsonb_array_elements(v.doc -> 'nodes') WITH ORDINALITY AS e(n, ord)
  ), '[]'::jsonb)
)
WHERE (v.doc ->> 'schema')::int = 2;
