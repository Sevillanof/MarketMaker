-- Pedidos que van por Andreani: la forma de entrega "andreani:<servicio>" y
-- los datos del envío (sucursal, DNI, cotización y, después, el alta).
ALTER TABLE orders DROP CONSTRAINT IF EXISTS orders_delivery_check;
ALTER TABLE orders ADD CONSTRAINT orders_delivery_check
    CHECK (delivery IN ('pickup', 'shipping') OR delivery LIKE 'andreani:%');
ALTER TABLE orders ADD COLUMN shipment jsonb;
