-- Recordatorio de carrito abandonado: lo prende cada tienda (apagado por defecto).
ALTER TABLE tenants ADD COLUMN remind_carts BOOLEAN NOT NULL DEFAULT false;
