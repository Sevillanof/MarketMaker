-- Mis fotos, más prolijas:
-- * Las fotos nuevas se guardan en usuarios/<dueño>/fotos/<foto>/ (ver
--   media::photo_folder). Las que ya estaban, en <foto>/, quedan marcadas y
--   el servidor las mueve solo al arrancar.
-- * sha256 del archivo original: si el dueño sube la misma foto otra vez,
--   se usa la que ya tiene.
-- * stored_bytes: lo que ocupan todos sus tamaños, para el límite por
--   usuario. Las viejas se miden al moverlas.
-- * alt: qué se ve en la foto, para buscadores y lectores de pantalla.
ALTER TABLE media
  ADD COLUMN in_user_folder boolean NOT NULL DEFAULT false,
  ADD COLUMN sha256 text,
  ADD COLUMN stored_bytes bigint NOT NULL DEFAULT 0,
  ADD COLUMN alt text NOT NULL DEFAULT '';
ALTER TABLE media ALTER COLUMN in_user_folder SET DEFAULT true;
CREATE UNIQUE INDEX media_owner_sha256 ON media (owner_id, sha256) WHERE sha256 IS NOT NULL;
