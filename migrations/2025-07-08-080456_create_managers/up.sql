-- Your SQL goes here
CREATE TABLE managers (
    id INTEGER NOT NULL PRIMARY KEY,
    hub_id INTEGER NOT NULL,
    name VARCHAR NOT NULL,
    email VARCHAR NOT NULL
);

CREATE TABLE client_manager (
    client_id INTEGER NOT NULL REFERENCES clients(id),
    manager_id INTEGER NOT NULL REFERENCES managers(id),
    PRIMARY KEY (client_id, manager_id)
);
