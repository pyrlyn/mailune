-- FTS5 virtual tables cannot be expressed in Diesel's DSL.
CREATE VIRTUAL TABLE message_fts USING fts5(
    message_id UNINDEXED,
    subject,
    addresses,
    body
);
