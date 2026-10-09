-- Schema v1. Provider ids are strings and only unique inside one account,
-- so every mail table keys on (account_id, id).

CREATE TABLE accounts (
    id TEXT PRIMARY KEY NOT NULL,
    email TEXT NOT NULL,
    display_name TEXT,
    provider TEXT NOT NULL
);

CREATE TABLE mailboxes (
    account_id TEXT NOT NULL REFERENCES accounts (id) ON DELETE CASCADE,
    id TEXT NOT NULL,
    name TEXT NOT NULL,
    role TEXT,
    parent_id TEXT,
    PRIMARY KEY (account_id, id)
);

-- Counts and the newest stamp are kept on the row so a list page is one
-- indexed range scan, not an aggregate over messages.
CREATE TABLE threads (
    account_id TEXT NOT NULL REFERENCES accounts (id) ON DELETE CASCADE,
    id TEXT NOT NULL,
    subject TEXT NOT NULL,
    latest_at BIGINT NOT NULL,
    message_count INTEGER NOT NULL,
    unread_count INTEGER NOT NULL,
    PRIMARY KEY (account_id, id)
);

CREATE INDEX threads_by_latest ON threads (account_id, latest_at DESC, id DESC);

CREATE TABLE messages (
    account_id TEXT NOT NULL,
    id TEXT NOT NULL,
    thread_id TEXT NOT NULL,
    from_name TEXT,
    from_email TEXT NOT NULL,
    to_addrs TEXT NOT NULL,
    cc_addrs TEXT NOT NULL,
    subject TEXT NOT NULL,
    snippet TEXT NOT NULL,
    stamp TEXT NOT NULL,
    received_at BIGINT NOT NULL,
    attachment_count INTEGER NOT NULL,
    transport TEXT NOT NULL,
    seen BOOLEAN NOT NULL,
    flagged BOOLEAN NOT NULL,
    draft BOOLEAN NOT NULL,
    answered BOOLEAN NOT NULL,
    deleted BOOLEAN NOT NULL,
    body_hash TEXT,
    PRIMARY KEY (account_id, id),
    FOREIGN KEY (account_id, thread_id) REFERENCES threads (account_id, id)
        ON DELETE CASCADE DEFERRABLE INITIALLY DEFERRED
);

CREATE INDEX messages_by_thread ON messages (account_id, thread_id, received_at);

CREATE TABLE memberships (
    account_id TEXT NOT NULL,
    message_id TEXT NOT NULL,
    mailbox_id TEXT NOT NULL,
    uid BIGINT,
    PRIMARY KEY (account_id, message_id, mailbox_id),
    FOREIGN KEY (account_id, message_id) REFERENCES messages (account_id, id) ON DELETE CASCADE,
    FOREIGN KEY (account_id, mailbox_id) REFERENCES mailboxes (account_id, id) ON DELETE CASCADE
);

CREATE INDEX memberships_by_mailbox ON memberships (account_id, mailbox_id);

CREATE TABLE parts (
    account_id TEXT NOT NULL,
    message_id TEXT NOT NULL,
    part_id TEXT NOT NULL,
    content_type TEXT NOT NULL,
    filename TEXT,
    size BIGINT NOT NULL,
    blob_hash TEXT,
    PRIMARY KEY (account_id, message_id, part_id),
    FOREIGN KEY (account_id, message_id) REFERENCES messages (account_id, id) ON DELETE CASCADE
);

-- Keywords and labels. The five system flags are columns on messages so
-- unread counts need no join.
CREATE TABLE flags (
    account_id TEXT NOT NULL,
    message_id TEXT NOT NULL,
    keyword TEXT NOT NULL,
    PRIMARY KEY (account_id, message_id, keyword),
    FOREIGN KEY (account_id, message_id) REFERENCES messages (account_id, id) ON DELETE CASCADE
);

-- One opaque cursor per sync scope: a JMAP state, a Gmail historyId, a
-- Graph deltaLink, or an IMAP UIDVALIDITY/MODSEQ pair.
CREATE TABLE sync_state (
    account_id TEXT NOT NULL REFERENCES accounts (id) ON DELETE CASCADE,
    scope TEXT NOT NULL,
    state TEXT NOT NULL,
    PRIMARY KEY (account_id, scope)
);

-- Times are nanoseconds since the epoch so a replayed op compares equal to
-- the one that was queued.
CREATE TABLE ops (
    seq INTEGER PRIMARY KEY NOT NULL,
    op_key TEXT NOT NULL UNIQUE,
    kind TEXT NOT NULL,
    thread_id TEXT NOT NULL,
    from_mailbox TEXT,
    to_mailbox TEXT,
    wake_ns BIGINT,
    queued_ns BIGINT NOT NULL
);

CREATE TABLE contacts (
    account_id TEXT NOT NULL REFERENCES accounts (id) ON DELETE CASCADE,
    email TEXT NOT NULL,
    name TEXT,
    last_seen BIGINT NOT NULL,
    times_contacted INTEGER NOT NULL,
    PRIMARY KEY (account_id, email)
);
