-- Ficha del producto: cada producto suma una descripción, que muestra el
-- bloque "Descripción del producto" de la ficha que se abre al tocarlo.
-- La ficha en sí viaja en el documento de la página (`product_modal`); los
-- documentos que no la traen usan la ficha por defecto.

ALTER TABLE products ADD COLUMN description text NOT NULL DEFAULT '';

UPDATE products p
SET description = x.description
FROM (VALUES
  ('Auriculares inalámbricos', 'Sonido claro y graves profundos, con hasta 30 horas de batería y estuche de carga. Se conectan por Bluetooth a tu celular o computadora.'),
  ('Mochila urbana',           'Resistente al agua, con compartimento acolchado para notebook de hasta 15 pulgadas y bolsillos para ordenar todo.'),
  ('Lámpara de escritorio',    'Luz LED cálida o fría con tres niveles de intensidad. Brazo flexible para iluminar justo donde la necesitás.'),
  ('Taza de cerámica',         'Hecha a mano, con capacidad de 350 ml. Apta para microondas y lavavajillas.'),
  ('Zapatillas running',       'Livianas y con amortiguación para entrenar todos los días. Capellada de malla que deja respirar el pie.'),
  ('Reloj analógico',          'Caja de acero, malla de cuero y resistencia al agua de 50 metros. Un clásico que combina con todo.'),
  ('Cuaderno de tapa dura',    '192 hojas lisas de 90 gramos, cinta señaladora y bolsillo interno. Ideal para tomar notas o dibujar.'),
  ('Parlante portátil',        'Sonido de 360 grados, resistente a salpicaduras y con 12 horas de batería. Llevalo a donde vayas.')
) AS x(name, description)
WHERE p.name = x.name AND p.description = '';
