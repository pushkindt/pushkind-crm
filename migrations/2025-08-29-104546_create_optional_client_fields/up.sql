-- Your SQL goes here
CREATE TABLE IF NOT EXISTS client_fields (
    client_id INTEGER NOT NULL REFERENCES clients(id),
    field VARCHAR(32) NOT NULL,
    value TEXT NOT NULL,
    PRIMARY KEY (client_id, field)
);
