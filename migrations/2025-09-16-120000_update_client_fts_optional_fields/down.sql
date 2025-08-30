-- Revert client_fts back to original structure without optional fields
DROP TRIGGER IF EXISTS client_fields_ai;
DROP TRIGGER IF EXISTS client_fields_au;
DROP TRIGGER IF EXISTS client_fields_ad;
DROP TRIGGER IF EXISTS clients_ai;
DROP TRIGGER IF EXISTS clients_au;
DROP TRIGGER IF EXISTS clients_ad;
DROP TABLE IF EXISTS client_fts;

CREATE VIRTUAL TABLE client_fts USING fts5(
    name,
    email,
    phone,
    address,
    content='clients',
    content_rowid='id',
    tokenize = 'unicode61'
);

INSERT INTO client_fts(client_fts) VALUES('rebuild');

CREATE TRIGGER clients_ai AFTER INSERT ON clients BEGIN
  INSERT INTO client_fts(rowid, name, email, phone, address) VALUES (new.id, new.name, new.email, new.phone, new.address);
END;
CREATE TRIGGER clients_ad AFTER DELETE ON clients BEGIN
  INSERT INTO client_fts(client_fts, rowid, name, email, phone, address) VALUES('delete', old.id, old.name, old.email, old.phone, old.address);
END;
CREATE TRIGGER clients_au AFTER UPDATE ON clients BEGIN
  INSERT INTO client_fts(client_fts, rowid, name, email, phone, address) VALUES('delete', old.id, old.name, old.email, old.phone, old.address);
  INSERT INTO client_fts(rowid, name, email, phone, address) VALUES (new.id, new.name, new.email, new.phone, new.address);
END;
