-- Your SQL goes here
DELETE FROM clients;
DELETE FROM client_manager;
DELETE FROM client_events;
CREATE UNIQUE INDEX clients_hub_id_email_idx ON clients (hub_id, email);
