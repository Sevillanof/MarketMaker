-- Mis fotos: la biblioteca de fotos de cada dueño. Los archivos (AVIF y
-- WebP en 480, 960 y 1600 px) viven en el almacén de fotos, no en la base.

CREATE TABLE media (
  id         uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  owner_id   uuid NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  name       text NOT NULL,
  width      int  NOT NULL,
  height     int  NOT NULL,
  bytes      bigint NOT NULL,
  created_at timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX media_owner_idx ON media (owner_id, created_at DESC);
