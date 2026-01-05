-- This file should undo anything in `up.sql`
DROP INDEX clients_hub_id_public_id_idx;
ALTER TABLE clients DROP COLUMN public_id;
