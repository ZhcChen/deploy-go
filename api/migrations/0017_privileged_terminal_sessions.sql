ALTER TABLE nodes
ADD COLUMN privileged_execution INTEGER NOT NULL DEFAULT 0
CHECK (privileged_execution IN (0, 1));

CREATE TABLE terminal_sessions (
    id TEXT PRIMARY KEY,
    node_id TEXT NOT NULL REFERENCES nodes(id) ON DELETE RESTRICT,
    agent_id TEXT NOT NULL REFERENCES agents(id) ON DELETE RESTRICT,
    actor_id TEXT NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
    request_id TEXT NOT NULL,
    status TEXT NOT NULL CHECK (status IN ('opening', 'active', 'closing', 'closed', 'failed', 'interrupted')),
    started_at TEXT NOT NULL,
    opened_at TEXT,
    close_requested_at TEXT,
    finished_at TEXT,
    exit_reason TEXT,
    exit_code INTEGER,
    input_bytes INTEGER NOT NULL DEFAULT 0 CHECK (input_bytes >= 0),
    output_bytes INTEGER NOT NULL DEFAULT 0 CHECK (output_bytes >= 0),
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    version INTEGER NOT NULL DEFAULT 1 CHECK (version > 0),
    CHECK ((status IN ('opening', 'active', 'closing') AND finished_at IS NULL)
        OR (status IN ('closed', 'failed', 'interrupted') AND finished_at IS NOT NULL))
);

CREATE UNIQUE INDEX terminal_sessions_one_active_per_node
ON terminal_sessions (node_id)
WHERE status IN ('opening', 'active', 'closing');

CREATE INDEX terminal_sessions_node_created
ON terminal_sessions (node_id, created_at DESC, id DESC);

CREATE INDEX terminal_sessions_actor_created
ON terminal_sessions (actor_id, created_at DESC, id DESC);
