-- Cuánto pesa la foto que baja un visitante (960 px en AVIF), para mostrarle
-- al dueño cuánto se achicó la foto que subió (`bytes`).
ALTER TABLE media ADD COLUMN web_bytes bigint NOT NULL DEFAULT 0;
