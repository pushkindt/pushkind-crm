-- Recreate client_fts to include optional client fields
DROP TRIGGER IF EXISTS clients_ai;
DROP TRIGGER IF EXISTS clients_au;
DROP TRIGGER IF EXISTS clients_ad;
DROP TRIGGER IF EXISTS client_fields_ai;
DROP TRIGGER IF EXISTS client_fields_au;
DROP TRIGGER IF EXISTS client_fields_ad;
DROP TABLE IF EXISTS client_fts;

CREATE VIRTUAL TABLE client_fts USING fts5(
    name,
    email,
    phone,
    address,
    fields,
    tokenize = 'unicode61'
);

INSERT INTO client_fts(rowid, name, email, phone, address, fields)
SELECT
    clients.id,
    clients.name,
    clients.email,
    clients.phone,
    clients.address,
    COALESCE(group_concat(client_fields.value, ' '), '')
FROM clients
LEFT JOIN client_fields ON clients.id = client_fields.client_id
GROUP BY clients.id;

CREATE TRIGGER clients_ai AFTER INSERT ON clients BEGIN
  INSERT INTO client_fts(rowid, name, email, phone, address, fields)
  VALUES (
    new.id,
    new.name,
    new.email,
    new.phone,
    new.address,
    COALESCE((SELECT group_concat(value, ' ') FROM client_fields WHERE client_id = new.id), '')
  );
END;

CREATE TRIGGER clients_ad AFTER DELETE ON clients BEGIN
  INSERT INTO client_fts(client_fts, rowid, name, email, phone, address, fields)
  VALUES (
    'delete',
    old.id,
    old.name,
    old.email,
    old.phone,
    old.address,
    COALESCE((SELECT group_concat(value, ' ') FROM client_fields WHERE client_id = old.id), '')
  );
END;

CREATE TRIGGER clients_au AFTER UPDATE ON clients BEGIN
  INSERT INTO client_fts(client_fts, rowid, name, email, phone, address, fields)
  VALUES (
    'delete',
    old.id,
    old.name,
    old.email,
    old.phone,
    old.address,
    COALESCE((SELECT group_concat(value, ' ') FROM client_fields WHERE client_id = old.id), '')
  );
  INSERT INTO client_fts(rowid, name, email, phone, address, fields)
  VALUES (
    new.id,
    new.name,
    new.email,
    new.phone,
    new.address,
    COALESCE((SELECT group_concat(value, ' ') FROM client_fields WHERE client_id = new.id), '')
  );
END;

