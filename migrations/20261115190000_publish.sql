-- Borrador y publicado: `current_version_id` es lo último guardado en el
-- editor (el borrador) y `published_version_id` lo que ven los visitantes.
ALTER TABLE pages
  ADD COLUMN published_version_id uuid REFERENCES page_versions(id);

-- Hasta ahora guardar era publicar: lo vigente ya está publicado.
UPDATE pages SET published_version_id = current_version_id;
