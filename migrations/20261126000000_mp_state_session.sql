-- "Vincular mi cuenta" de Mercado Pago queda atado a la sesión que lo
-- empezó: al volver, solo vale en ese mismo navegador. Los que estaban a
-- medio camino se descartan (hay que tocar "Vincular" de nuevo).
DELETE FROM mp_oauth_states;
ALTER TABLE mp_oauth_states ADD COLUMN session_hash bytea NOT NULL;
