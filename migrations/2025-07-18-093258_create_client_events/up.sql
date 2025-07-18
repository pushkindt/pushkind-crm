-- Create client_events to track various client related events
CREATE TABLE client_events (
    id INTEGER NOT NULL PRIMARY KEY,
    client_id INTEGER NOT NULL REFERENCES clients(id),
    manager_id INTEGER NOT NULL REFERENCES managers(id),
    event_type VARCHAR(255) NOT NULL,
    event_data TEXT NOT NULL, --JSON
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);
