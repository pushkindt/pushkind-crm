-- Your SQL goes here
DELETE FROM client_manager;
DELETE FROM client_events;
DELETE FROM clients;
CREATE UNIQUE INDEX clients_hub_id_email_idx ON clients (hub_id, email);
CREATE UNIQUE INDEX clients_hub_id_phone_idx ON clients (hub_id, phone);
