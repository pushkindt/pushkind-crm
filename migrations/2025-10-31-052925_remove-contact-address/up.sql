-- Revert address/contact extension and restore previous FTS definition

DROP TRIGGER IF EXISTS clients_ai;
DROP TRIGGER IF EXISTS clients_au;
DROP TRIGGER IF EXISTS clients_ad;
DROP TABLE IF EXISTS client_fts;
DROP TABLE IF EXISTS client_fts_data;
DROP TABLE IF EXISTS client_fts_idx;
DROP TABLE IF EXISTS client_fts_docsize;
DROP TABLE IF EXISTS client_fts_config;

ALTER TABLE clients DROP COLUMN contact;
ALTER TABLE clients DROP COLUMN address;

CREATE VIRTUAL TABLE client_fts USING fts5(
    name,
    email,
    phone,
    fields,
    content='clients',
    content_rowid='id',
    tokenize = 'unicode61'
);

INSERT INTO client_fts(client_fts) VALUES('rebuild');

CREATE TRIGGER clients_ai AFTER INSERT ON clients BEGIN
  INSERT INTO client_fts(rowid, name, email, phone, fields)
  VALUES (new.id, new.name, new.email, new.phone, new.fields);
END;

CREATE TRIGGER clients_ad AFTER DELETE ON clients BEGIN
  INSERT INTO client_fts(client_fts, rowid, name, email, phone, fields)
  VALUES('delete', old.id, old.name, old.email, old.phone, old.fields);
END;

CREATE TRIGGER clients_au AFTER UPDATE ON clients BEGIN
  INSERT INTO client_fts(client_fts, rowid, name, email, phone, fields)
  VALUES('delete', old.id, old.name, old.email, old.phone, old.fields);
  INSERT INTO client_fts(rowid, name, email, phone, fields)
  VALUES (new.id, new.name, new.email, new.phone, new.fields);
END;
