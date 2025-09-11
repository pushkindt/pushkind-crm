-- Revert FTS extension and clients.fields column



-- Drop FTS triggers and table
DROP TRIGGER IF EXISTS clients_ai;
DROP TRIGGER IF EXISTS clients_au;
DROP TRIGGER IF EXISTS clients_ad;
DROP TABLE IF EXISTS client_fts;
DROP TABLE IF EXISTS client_fts_data;
DROP TABLE IF EXISTS client_fts_idx;
DROP TABLE IF EXISTS client_fts_docsize;
DROP TABLE IF EXISTS client_fts_config;

-- Remove the denormalized column
ALTER TABLE clients DROP COLUMN fields;

-- Recreate original FTS (without fields)
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
  INSERT INTO client_fts(rowid, name, email, phone)
  VALUES (new.id, new.name, new.email, new.phone);
END;

CREATE TRIGGER clients_ad AFTER DELETE ON clients BEGIN
  DELETE FROM client_fts WHERE rowid = old.id;
END;

CREATE TRIGGER clients_au AFTER UPDATE ON clients BEGIN
  DELETE FROM client_fts WHERE rowid = old.id;
  INSERT INTO client_fts(rowid, name, email, phone)
  VALUES (new.id, new.name, new.email, new.phone);
END;
