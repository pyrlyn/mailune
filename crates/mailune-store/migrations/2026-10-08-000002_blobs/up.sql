-- Encrypted message bodies and attachments, addressed by the SHA-256 of
-- the plaintext. `used` is a counter, not a clock, so eviction order does
-- not depend on the device time.
CREATE TABLE blobs (
    hash TEXT PRIMARY KEY NOT NULL,
    data BLOB NOT NULL,
    size BIGINT NOT NULL,
    used BIGINT NOT NULL
);

CREATE INDEX blobs_by_use ON blobs (used);
