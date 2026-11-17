-- Uso y abandono: recorridos anónimos. Cada fila es un paso de una visita.
-- `visita` es un código al azar que cambia cada día (hash con sal que no se
-- guarda): no hay nombre, email, IP ni nada que se haya escrito.
CREATE TABLE eventos (
  id          bigserial PRIMARY KEY,
  visita      text NOT NULL,
  quien       text NOT NULL CHECK (quien IN ('owner', 'buyer')),
  paso        text NOT NULL,
  tienda      text,
  dispositivo text NOT NULL,
  detalle     text,
  creado      timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX eventos_creado_idx ON eventos (creado);
CREATE INDEX eventos_visita_idx ON eventos (visita, creado);
