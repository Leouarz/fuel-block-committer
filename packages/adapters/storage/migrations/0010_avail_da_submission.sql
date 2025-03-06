BEGIN;

CREATE TABLE IF NOT EXISTS avail_submission (
    id              SERIAL PRIMARY KEY,
    tx_hash         BYTEA NOT NULL UNIQUE,
    tx_id           INTEGER NOT NULL,
    block_hash      BYTEA NOT NULL,
    block_number    INTEGER NOT NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    status          SMALLINT NOT NULL,
    CONSTRAINT avail_submission_status_check 
        CHECK (status IN (0, 1, 2, 3))
);

CREATE TABLE IF NOT EXISTS avail_submission_fragments (
    id                    SERIAL PRIMARY KEY,
    submission_id         INTEGER NOT NULL REFERENCES avail_submission(id) ON DELETE CASCADE,
    fragment_id           INTEGER NOT NULL,
    UNIQUE(submission_id, fragment_id)
);

COMMIT;
