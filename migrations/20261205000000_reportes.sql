-- Reportes de errores: cualquier persona marca un área de la página, escribe
-- qué pasó y lo manda desde ahí. Los ve solo la cuenta administradora.
-- La captura (JPEG, máx. 1,2 MB) queda en la misma fila.
CREATE TABLE reportes (
  id          uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  num         bigserial UNIQUE,
  creado      timestamptz NOT NULL DEFAULT now(),
  estado      text NOT NULL DEFAULT 'nuevo' CHECK (estado IN ('nuevo', 'en_curso', 'resuelto')),
  descripcion text NOT NULL,
  pagina      text NOT NULL,
  quien       text NOT NULL DEFAULT '',
  pantalla    text NOT NULL DEFAULT '',
  navegador   text NOT NULL DEFAULT '',
  captura     bytea
);
CREATE INDEX reportes_estado_creado ON reportes (estado, creado DESC);
