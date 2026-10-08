-- v1 mail schema. Raw SQL stays in migration files; queries use Diesel's DSL.
CREATE TABLE accounts (
    id TEXT PRIMARY KEY NOT NULL,
    email TEXT NOT NULL
);

CREATE TABLE mailboxes (
    id TEXT PRIMARY KEY NOT NULL,
    account_id TEXT NOT NULL REFERENCES accounts(id),
    name TEXT NOT NULL,
    role TEXT
);

CREATE TABLE threads (
    id TEXT PRIMARY KEY NOT NULL,
    account_id TEXT NOT NULL REFERENCES accounts(id),
    subject TEXT NOT NULL
);

CREATE TABLE messages (
    id TEXT PRIMARY KEY NOT NULL,
    account_id TEXT NOT NULL REFERENCES accounts(id),
    thread_id TEXT NOT NULL REFERENCES threads(id),
    subject TEXT NOT NULL,
    from_email TEXT NOT NULL,
    to_emails TEXT NOT NULL,
    stamp TEXT NOT NULL,
    received_at BIGINT NOT NULL
);

CREATE INDEX messages_received ON messages (received_at, id);
CREATE INDEX messages_thread ON messages (thread_id, received_at);

CREATE TABLE memberships (
    message_id TEXT NOT NULL REFERENCES messages(id),
    mailbox_id TEXT NOT NULL REFERENCES mailboxes(id),
    PRIMARY KEY (message_id, mailbox_id)
);

CREATE TABLE parts (
    message_id TEXT NOT NULL REFERENCES messages(id),
    ordinal INTEGER NOT NULL,
    content_type TEXT NOT NULL,
    body TEXT NOT NULL,
    PRIMARY KEY (message_id, ordinal)
);

CREATE TABLE flags (
    message_id TEXT PRIMARY KEY NOT NULL REFERENCES messages(id),
    seen INTEGER NOT NULL,
    flagged INTEGER NOT NULL,
    draft INTEGER NOT NULL,
    answered INTEGER NOT NULL,
    deleted INTEGER NOT NULL,
    keywords TEXT NOT NULL
);

CREATE TABLE sync_state (
    account_id TEXT NOT NULL REFERENCES accounts(id),
    mailbox_id TEXT NOT NULL REFERENCES mailboxes(id),
    token TEXT NOT NULL,
    PRIMARY KEY (account_id, mailbox_id)
);

CREATE TABLE ops (
    key TEXT PRIMARY KEY NOT NULL,
    kind TEXT NOT NULL,
    payload TEXT NOT NULL,
    at_ms BIGINT NOT NULL,
    position INTEGER NOT NULL
);

CREATE TABLE contacts (
    account_id TEXT NOT NULL REFERENCES accounts(id),
    email TEXT NOT NULL,
    name TEXT NOT NULL,
    PRIMARY KEY (account_id, email)
);
