-- Encuadre de cada foto en cada producto (punto a la vista y zoom), por
-- dirección de la foto: {"/media/<id>/960.webp": {"x": 80, "y": 50, "zoom": 135}}.
-- La foto guardada no cambia; vacío = todas centradas y sin zoom.
ALTER TABLE products ADD COLUMN photo_fits jsonb NOT NULL DEFAULT '{}';
