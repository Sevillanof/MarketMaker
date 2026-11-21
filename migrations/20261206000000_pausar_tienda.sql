-- Pausar una tienda: los compradores ven "Volvemos pronto" y no se puede
-- comprar, sin despublicar ni perder nada.
ALTER TABLE tenants ADD COLUMN paused boolean NOT NULL DEFAULT false;
