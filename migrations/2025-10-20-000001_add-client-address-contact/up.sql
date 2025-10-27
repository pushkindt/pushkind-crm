-- Add address and contact columns to clients and extend FTS indexing

-- Drop existing FTS triggers and tables before recreating them with new columns
DROP TRIGGER IF EXISTS clients_ai;
DROP TRIGGER IF EXISTS clients_au;
DROP TRIGGER IF EXISTS clients_ad;
DROP TABLE IF EXISTS client_fts;
DROP TABLE IF EXISTS client_fts_data;
DROP TABLE IF EXISTS client_fts_idx;
DROP TABLE IF EXISTS client_fts_docsize;
DROP TABLE IF EXISTS client_fts_config;

-- Add the new nullable columns to clients
ALTER TABLE clients ADD COLUMN address TEXT;
ALTER TABLE clients ADD COLUMN contact TEXT;

-- Recreate the FTS5 table including the new columns
CREATE VIRTUAL TABLE client_fts USING fts5(
    name,
    email,
    phone,
    address,
    contact,
    fields,
    content='clients',
    content_rowid='id',
    tokenize = 'unicode61'
);

-- Rebuild the FTS index from existing content
INSERT INTO client_fts(client_fts) VALUES('rebuild');

-- Recreate triggers so FTS stays in sync
CREATE TRIGGER clients_ai AFTER INSERT ON clients BEGIN
  INSERT INTO client_fts(rowid, name, email, phone, address, contact, fields)
  VALUES (new.id, new.name, new.email, new.phone, new.address, new.contact, new.fields);
END;

CREATE TRIGGER clients_ad AFTER DELETE ON clients BEGIN
  INSERT INTO client_fts(client_fts, rowid, name, email, phone, address, contact, fields)
  VALUES('delete', old.id, old.name, old.email, old.phone, old.address, old.contact, old.fields);
END;

CREATE TRIGGER clients_au AFTER UPDATE ON clients BEGIN
  INSERT INTO client_fts(client_fts, rowid, name, email, phone, address, contact, fields)
  VALUES('delete', old.id, old.name, old.email, old.phone, old.address, old.contact, old.fields);
  INSERT INTO client_fts(rowid, name, email, phone, address, contact, fields)
  VALUES (new.id, new.name, new.email, new.phone, new.address, new.contact, new.fields);
END;
