-- Datos para transferir en dos campos: alias/CBU/CVU y titular. Lo que había
-- escrito cada tienda en el campo libre pasa entero al alias.
ALTER TABLE tenants
  ADD COLUMN transfer_alias text NOT NULL DEFAULT '',
  ADD COLUMN transfer_holder text NOT NULL DEFAULT '';
UPDATE tenants SET transfer_alias = transfer_info;
ALTER TABLE tenants DROP COLUMN transfer_info;
