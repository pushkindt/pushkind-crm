-- Extend clients and client_fts to index optional client_fields
-- 1) Add denormalized column to clients
-- 2) Recreate FTS to include the new column
-- 3) Add triggers to keep FTS and clients.fields in sync

-- Drop existing FTS triggers and table (will recreate)
DROP TRIGGER IF EXISTS clients_ai;
DROP TRIGGER IF EXISTS clients_au;
DROP TRIGGER IF EXISTS clients_ad;
DROP TABLE IF EXISTS client_fts;

-- Add a column to store concatenated optional fields for FTS
ALTER TABLE clients ADD COLUMN fields TEXT;

-- Backfill current values from client_fields
UPDATE clients
SET fields = (
  SELECT trim(COALESCE(group_concat(value, ' '), ''))
  FROM client_fields cf
  WHERE cf.client_id = clients.id
);

-- Recreate the FTS5 table including the new column
CREATE VIRTUAL TABLE client_fts USING fts5(
    name,
    email,
    phone,
    fields,
    content='clients',
    content_rowid='id',
    tokenize = 'unicode61'
);

-- Populate FTS from content table
INSERT INTO client_fts(client_fts) VALUES('rebuild');

-- Recreate triggers on clients to maintain FTS
CREATE TRIGGER clients_ai AFTER INSERT ON clients BEGIN
  INSERT INTO client_fts(rowid, name, email, phone, fields)
  VALUES (new.id, new.name, new.email, new.phone, new.fields);
END;

CREATE TRIGGER clients_ad AFTER DELETE ON clients BEGIN
  DELETE FROM client_fts WHERE rowid = old.id;
END;

CREATE TRIGGER clients_au AFTER UPDATE ON clients BEGIN
  DELETE FROM client_fts WHERE rowid = old.id;
  INSERT INTO client_fts(rowid, name, email, phone, fields)
  VALUES (new.id, new.name, new.email, new.phone, new.fields);
END;

-- Triggers on client_fields to keep clients.fields denormalized value up-to-date
CREATE TRIGGER client_fields_ai AFTER INSERT ON client_fields BEGIN
  UPDATE clients
  SET fields = (
    SELECT trim(COALESCE(group_concat(value, ' '), ''))
    FROM client_fields cf
    WHERE cf.client_id = new.client_id
  )
  WHERE id = new.client_id;
END;

CREATE TRIGGER client_fields_au AFTER UPDATE ON client_fields BEGIN
  UPDATE clients
  SET fields = (
    SELECT trim(COALESCE(group_concat(value, ' '), ''))
    FROM client_fields cf
    WHERE cf.client_id = new.client_id
  )
  WHERE id = new.client_id;
END;

CREATE TRIGGER client_fields_ad AFTER DELETE ON client_fields BEGIN
  UPDATE clients
  SET fields = (
    SELECT trim(COALESCE(group_concat(value, ' '), ''))
    FROM client_fields cf
    WHERE cf.client_id = old.client_id
  )
  WHERE id = old.client_id;
END;
