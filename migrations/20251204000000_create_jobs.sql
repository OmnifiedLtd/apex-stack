-- Create jobs table for sqlx-mq
CREATE TABLE mq_msgs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    channel_name TEXT NOT NULL,
    channel_args TEXT NOT NULL DEFAULT '',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    attempt_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    attempts INT NOT NULL DEFAULT 0,
    retry_backoff INTERVAL NOT NULL DEFAULT INTERVAL '1 second',
    payload BYTEA NOT NULL
);

-- Index for efficient job fetching with SKIP LOCKED
CREATE INDEX mq_msgs_fetch_idx ON mq_msgs (
    channel_name,
    channel_args,
    attempt_at
);

-- Create payloads table for large payloads
CREATE TABLE mq_payloads (
    id UUID PRIMARY KEY REFERENCES mq_msgs(id) ON DELETE CASCADE,
    payload BYTEA NOT NULL
);
