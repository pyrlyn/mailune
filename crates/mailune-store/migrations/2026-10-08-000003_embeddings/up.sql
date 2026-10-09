-- One vector per message chunk and embedding model. Vectors are
-- little-endian f32; nearest neighbours are a cosine scan in Rust.
CREATE TABLE embeddings (
    account_id TEXT NOT NULL,
    message_id TEXT NOT NULL,
    chunk INTEGER NOT NULL,
    model TEXT NOT NULL,
    dim INTEGER NOT NULL,
    vector BLOB NOT NULL,
    PRIMARY KEY (account_id, message_id, chunk, model),
    FOREIGN KEY (account_id, message_id) REFERENCES messages (account_id, id) ON DELETE CASCADE
);

CREATE INDEX embeddings_by_model ON embeddings (account_id, model);
