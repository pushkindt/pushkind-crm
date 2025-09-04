-- Your SQL goes here
DROP TRIGGER clients_ai;
DROP TRIGGER clients_au;
DROP TRIGGER clients_ad;
DROP TABLE client_fts;
ALTER TABLE clients DROP COLUMN address;

CREATE VIRTUAL TABLE client_fts USING fts5(
    name,
    email,
    phone,
    content='clients',
    content_rowid='id',
    tokenize = 'unicode61'
);

INSERT INTO client_fts(client_fts) VALUES('rebuild');

CREATE TRIGGER clients_ai AFTER INSERT ON clients BEGIN
  INSERT INTO client_fts(rowid, name, email, phone) VALUES (new.id, new.name, new.email, new.phone);
END;
CREATE TRIGGER clients_ad AFTER DELETE ON clients BEGIN
  DELETE FROM client_fts WHERE rowid = old.id;
END;
CREATE TRIGGER clients_au AFTER UPDATE ON clients BEGIN
  DELETE FROM client_fts WHERE rowid = old.id;
  INSERT INTO client_fts(rowid, name, email, phone) VALUES (new.id, new.name, new.email, new.phone);
END;
