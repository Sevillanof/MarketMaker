-- Favoritos de los compradores. Cada favorito pertenece a la cuenta de un
-- comprador, y esa cuenta es de UNA tienda: la misma persona tiene favoritos
-- separados en cada tienda y nunca se mezclan.

CREATE TABLE buyer_favorites (
  buyer_id   uuid NOT NULL REFERENCES buyers(id) ON DELETE CASCADE,
  product_id uuid NOT NULL REFERENCES products(id) ON DELETE CASCADE,
  created_at timestamptz NOT NULL DEFAULT now(),
  PRIMARY KEY (buyer_id, product_id)
);
CREATE INDEX buyer_favorites_product_idx ON buyer_favorites (product_id);

-- Si alguien toca el corazón sin haber entrado, el producto queda anotado en
-- el enlace del email y se guarda al canjearlo (aunque lo abra en otro
-- dispositivo).
ALTER TABLE buyer_links ADD COLUMN pending_favorite uuid;
