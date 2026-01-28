CREATE INDEX clients_hub_id_id_idx ON clients (hub_id, id);
CREATE INDEX managers_hub_id_is_user_idx ON managers (hub_id, is_user);
CREATE INDEX client_manager_manager_id_idx ON client_manager (manager_id);
CREATE INDEX client_events_client_id_created_at_idx ON client_events (client_id, created_at DESC);
