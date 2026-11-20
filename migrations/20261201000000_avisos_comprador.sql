-- Emails al comprador cuando el dueño mueve su pedido (pago confirmado,
-- enviado, listo para retirar): cuáles ya se mandaron, para no repetirlos si
-- el pedido va y vuelve en el tablero.
ALTER TABLE orders ADD COLUMN buyer_updates text[] NOT NULL DEFAULT '{}';
