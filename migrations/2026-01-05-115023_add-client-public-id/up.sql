-- Your SQL goes here
ALTER TABLE clients ADD COLUMN public_id BLOB;
UPDATE clients SET public_id = randomblob(16) WHERE public_id IS NULL;
CREATE UNIQUE INDEX clients_hub_id_public_id_idx ON clients (hub_id, public_id);
