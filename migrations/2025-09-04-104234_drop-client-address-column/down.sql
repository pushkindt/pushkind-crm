-- This file should undo anything in `up.sql`
DROP TRIGGER clients_ai;
DROP TRIGGER clients_au;
DROP TRIGGER clients_ad;
DROP TABLE client_fts;
ALTER TABLE clients ADD COLUMN address VARCHAR;

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
  DELETE FROM client_fts WHERE rowid = old.id;
END;
CREATE TRIGGER clients_au AFTER UPDATE ON clients BEGIN
  DELETE FROM client_fts WHERE rowid = old.id;
  INSERT INTO client_fts(rowid, name, email, phone, address) VALUES (new.id, new.name, new.email, new.phone, new.address);
END;
