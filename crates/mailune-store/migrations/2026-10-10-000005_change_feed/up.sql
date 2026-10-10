-- Change feed (S7). `PRAGMA data_version` says that another connection committed,
-- not what it changed; one counter per topic, bumped by the triggers below, lets a
-- reader name the topics. Child tables count toward their parent's topic. Blobs
-- have no topic: they are addressed by content, and a read bumps their `used`.
CREATE TABLE change_counters (
    topic TEXT PRIMARY KEY NOT NULL,
    version BIGINT NOT NULL
);

INSERT INTO change_counters (topic, version) VALUES
    ('accounts', 0),
    ('mailboxes', 0),
    ('threads', 0),
    ('messages', 0),
    ('embeddings', 0),
    ('sync_state', 0),
    ('ops', 0),
    ('contacts', 0);

CREATE TRIGGER accounts_feed_insert AFTER INSERT ON accounts BEGIN UPDATE change_counters SET version = version + 1 WHERE topic = 'accounts'; END;
CREATE TRIGGER accounts_feed_update AFTER UPDATE ON accounts BEGIN UPDATE change_counters SET version = version + 1 WHERE topic = 'accounts'; END;
CREATE TRIGGER accounts_feed_delete AFTER DELETE ON accounts BEGIN UPDATE change_counters SET version = version + 1 WHERE topic = 'accounts'; END;

CREATE TRIGGER mailboxes_feed_insert AFTER INSERT ON mailboxes BEGIN UPDATE change_counters SET version = version + 1 WHERE topic = 'mailboxes'; END;
CREATE TRIGGER mailboxes_feed_update AFTER UPDATE ON mailboxes BEGIN UPDATE change_counters SET version = version + 1 WHERE topic = 'mailboxes'; END;
CREATE TRIGGER mailboxes_feed_delete AFTER DELETE ON mailboxes BEGIN UPDATE change_counters SET version = version + 1 WHERE topic = 'mailboxes'; END;

CREATE TRIGGER threads_feed_insert AFTER INSERT ON threads BEGIN UPDATE change_counters SET version = version + 1 WHERE topic = 'threads'; END;
CREATE TRIGGER threads_feed_update AFTER UPDATE ON threads BEGIN UPDATE change_counters SET version = version + 1 WHERE topic = 'threads'; END;
CREATE TRIGGER threads_feed_delete AFTER DELETE ON threads BEGIN UPDATE change_counters SET version = version + 1 WHERE topic = 'threads'; END;

CREATE TRIGGER messages_feed_insert AFTER INSERT ON messages BEGIN UPDATE change_counters SET version = version + 1 WHERE topic = 'messages'; END;
CREATE TRIGGER messages_feed_update AFTER UPDATE ON messages BEGIN UPDATE change_counters SET version = version + 1 WHERE topic = 'messages'; END;
CREATE TRIGGER messages_feed_delete AFTER DELETE ON messages BEGIN UPDATE change_counters SET version = version + 1 WHERE topic = 'messages'; END;
CREATE TRIGGER memberships_feed_insert AFTER INSERT ON memberships BEGIN UPDATE change_counters SET version = version + 1 WHERE topic = 'messages'; END;
CREATE TRIGGER memberships_feed_update AFTER UPDATE ON memberships BEGIN UPDATE change_counters SET version = version + 1 WHERE topic = 'messages'; END;
CREATE TRIGGER memberships_feed_delete AFTER DELETE ON memberships BEGIN UPDATE change_counters SET version = version + 1 WHERE topic = 'messages'; END;
CREATE TRIGGER parts_feed_insert AFTER INSERT ON parts BEGIN UPDATE change_counters SET version = version + 1 WHERE topic = 'messages'; END;
CREATE TRIGGER parts_feed_update AFTER UPDATE ON parts BEGIN UPDATE change_counters SET version = version + 1 WHERE topic = 'messages'; END;
CREATE TRIGGER parts_feed_delete AFTER DELETE ON parts BEGIN UPDATE change_counters SET version = version + 1 WHERE topic = 'messages'; END;
CREATE TRIGGER flags_feed_insert AFTER INSERT ON flags BEGIN UPDATE change_counters SET version = version + 1 WHERE topic = 'messages'; END;
CREATE TRIGGER flags_feed_update AFTER UPDATE ON flags BEGIN UPDATE change_counters SET version = version + 1 WHERE topic = 'messages'; END;
CREATE TRIGGER flags_feed_delete AFTER DELETE ON flags BEGIN UPDATE change_counters SET version = version + 1 WHERE topic = 'messages'; END;

CREATE TRIGGER embeddings_feed_insert AFTER INSERT ON embeddings BEGIN UPDATE change_counters SET version = version + 1 WHERE topic = 'embeddings'; END;
CREATE TRIGGER embeddings_feed_update AFTER UPDATE ON embeddings BEGIN UPDATE change_counters SET version = version + 1 WHERE topic = 'embeddings'; END;
CREATE TRIGGER embeddings_feed_delete AFTER DELETE ON embeddings BEGIN UPDATE change_counters SET version = version + 1 WHERE topic = 'embeddings'; END;

CREATE TRIGGER sync_state_feed_insert AFTER INSERT ON sync_state BEGIN UPDATE change_counters SET version = version + 1 WHERE topic = 'sync_state'; END;
CREATE TRIGGER sync_state_feed_update AFTER UPDATE ON sync_state BEGIN UPDATE change_counters SET version = version + 1 WHERE topic = 'sync_state'; END;
CREATE TRIGGER sync_state_feed_delete AFTER DELETE ON sync_state BEGIN UPDATE change_counters SET version = version + 1 WHERE topic = 'sync_state'; END;

CREATE TRIGGER ops_feed_insert AFTER INSERT ON ops BEGIN UPDATE change_counters SET version = version + 1 WHERE topic = 'ops'; END;
CREATE TRIGGER ops_feed_update AFTER UPDATE ON ops BEGIN UPDATE change_counters SET version = version + 1 WHERE topic = 'ops'; END;
CREATE TRIGGER ops_feed_delete AFTER DELETE ON ops BEGIN UPDATE change_counters SET version = version + 1 WHERE topic = 'ops'; END;

CREATE TRIGGER contacts_feed_insert AFTER INSERT ON contacts BEGIN UPDATE change_counters SET version = version + 1 WHERE topic = 'contacts'; END;
CREATE TRIGGER contacts_feed_update AFTER UPDATE ON contacts BEGIN UPDATE change_counters SET version = version + 1 WHERE topic = 'contacts'; END;
CREATE TRIGGER contacts_feed_delete AFTER DELETE ON contacts BEGIN UPDATE change_counters SET version = version + 1 WHERE topic = 'contacts'; END;
