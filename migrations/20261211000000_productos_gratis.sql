-- Un producto a $ 0 solo se publica si el dueño confirma que es gratis.
ALTER TABLE products ADD COLUMN free BOOLEAN NOT NULL DEFAULT false;
-- Los que ya estaban publicados a $ 0 pasan a ocultos (el dueño decide si los regala).
UPDATE products SET visible = false WHERE price_cents = 0 AND NOT example;
