-- Foto de perfil: una de Mis fotos. Si la borra, la cuenta vuelve a la inicial.
ALTER TABLE users ADD COLUMN avatar_id uuid REFERENCES media(id) ON DELETE SET NULL;
