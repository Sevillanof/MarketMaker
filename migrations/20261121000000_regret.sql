-- Botón de arrepentimiento: el comprador pide revocar la compra y recibe un código.
ALTER TABLE orders
    ADD COLUMN regret_code text NOT NULL DEFAULT '',
    ADD COLUMN regret_at timestamptz;
