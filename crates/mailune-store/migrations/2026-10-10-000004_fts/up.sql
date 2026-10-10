-- Full-text index over subject, addresses and body text (S5).
--
-- FTS5 keys its rows by an integer rowid, and VACUUM may renumber the implicit
-- rowid of `messages`, so `search_docs` gives each message a stable one. Deleting
-- by an UNINDEXED message id column instead would scan the whole index.
CREATE TABLE search_docs (
    docid INTEGER PRIMARY KEY NOT NULL,
    account_id TEXT NOT NULL,
    message_id TEXT NOT NULL,
    UNIQUE (account_id, message_id)
);

-- Body text is written by the store once a body is fetched and decoded; subject
-- and addresses follow the messages row through the triggers below.
CREATE VIRTUAL TABLE message_fts USING fts5(
    subject,
    addresses,
    body,
    tokenize = 'unicode61 remove_diacritics 2'
);

-- Recipients are stored as JSON; the index wants their names and addresses as
-- words, not the JSON keys.
CREATE VIEW message_search_source AS
SELECT
    m.account_id,
    m.id,
    m.subject,
    concat_ws(
        ' ',
        m.from_name,
        m.from_email,
        (SELECT group_concat(concat_ws(' ', r.value ->> '$.name', r.value ->> '$.email'), ' ')
            FROM json_each(m.to_addrs) AS r),
        (SELECT group_concat(concat_ws(' ', r.value ->> '$.name', r.value ->> '$.email'), ' ')
            FROM json_each(m.cc_addrs) AS r)
    ) AS addresses
FROM messages AS m;

CREATE TRIGGER messages_fts_insert AFTER INSERT ON messages
BEGIN
    INSERT INTO search_docs (account_id, message_id) VALUES (NEW.account_id, NEW.id);
    INSERT INTO message_fts (rowid, subject, addresses)
    SELECT d.docid, s.subject, s.addresses
    FROM search_docs AS d
    JOIN message_search_source AS s ON s.account_id = d.account_id AND s.id = d.message_id
    WHERE d.account_id = NEW.account_id AND d.message_id = NEW.id;
END;

-- An upsert rewrites every column; only a change to an indexed one should touch
-- the index, so a flag change costs nothing here.
CREATE TRIGGER messages_fts_update
AFTER UPDATE OF subject, from_name, from_email, to_addrs, cc_addrs ON messages
WHEN OLD.subject IS NOT NEW.subject
    OR OLD.from_name IS NOT NEW.from_name
    OR OLD.from_email IS NOT NEW.from_email
    OR OLD.to_addrs IS NOT NEW.to_addrs
    OR OLD.cc_addrs IS NOT NEW.cc_addrs
BEGIN
    UPDATE message_fts
    SET (subject, addresses) = (
        SELECT s.subject, s.addresses
        FROM message_search_source AS s
        WHERE s.account_id = NEW.account_id AND s.id = NEW.id
    )
    WHERE rowid = (
        SELECT docid FROM search_docs WHERE account_id = NEW.account_id AND message_id = NEW.id
    );
END;

CREATE TRIGGER messages_fts_delete AFTER DELETE ON messages
BEGIN
    DELETE FROM message_fts
    WHERE rowid = (
        SELECT docid FROM search_docs WHERE account_id = OLD.account_id AND message_id = OLD.id
    );
    DELETE FROM search_docs WHERE account_id = OLD.account_id AND message_id = OLD.id;
END;

INSERT INTO search_docs (account_id, message_id) SELECT account_id, id FROM messages;

INSERT INTO message_fts (rowid, subject, addresses)
SELECT d.docid, s.subject, s.addresses
FROM search_docs AS d
JOIN message_search_source AS s ON s.account_id = d.account_id AND s.id = d.message_id;
