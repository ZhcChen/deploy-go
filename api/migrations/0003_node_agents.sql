CREATE TABLE nodes_new (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL COLLATE NOCASE UNIQUE,
    host TEXT,
    port INTEGER CHECK (port IS NULL OR port BETWEEN 1 AND 65535),
    username TEXT,
    ssh_credential_id TEXT REFERENCES ssh_credentials(id) ON DELETE RESTRICT,
    work_root TEXT,
    secrets_root TEXT,
    status TEXT NOT NULL CHECK (status IN ('missing_credential', 'unchecked', 'checking', 'online', 'offline', 'disabled')),
    trusted_host_key TEXT,
    trusted_host_fingerprint TEXT,
    checked_at TEXT,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    version INTEGER NOT NULL DEFAULT 1 CHECK (version > 0),
    CHECK (
        (ssh_credential_id IS NOT NULL AND host IS NOT NULL AND port IS NOT NULL AND username IS NOT NULL AND work_root IS NOT NULL AND secrets_root IS NOT NULL)
        OR (ssh_credential_id IS NULL AND status IN ('missing_credential', 'offline', 'disabled'))
    )
);

INSERT INTO nodes_new (
    id, name, host, port, username, ssh_credential_id, work_root, secrets_root,
    status, trusted_host_key, trusted_host_fingerprint, checked_at,
    created_at, updated_at, version
)
SELECT
    id, name, host, port, username, ssh_credential_id, work_root, secrets_root,
    status, trusted_host_key, trusted_host_fingerprint, checked_at,
    created_at, updated_at, version
FROM nodes;

DROP TABLE nodes;
ALTER TABLE nodes_new RENAME TO nodes;

CREATE INDEX nodes_ssh_credential_id ON nodes (ssh_credential_id);
CREATE INDEX nodes_status ON nodes (status);

CREATE TABLE agents (
    id TEXT PRIMARY KEY,
    node_id TEXT NOT NULL UNIQUE REFERENCES nodes(id) ON DELETE RESTRICT,
    registered_at TEXT,
    last_seen_at TEXT,
    revoked_at TEXT,
    archived_at TEXT,
    agent_version TEXT,
    protocol_version INTEGER CHECK (protocol_version IS NULL OR protocol_version > 0),
    hostname TEXT,
    os_name TEXT,
    architecture TEXT,
    capabilities_json TEXT NOT NULL DEFAULT '{}',
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    version INTEGER NOT NULL DEFAULT 1 CHECK (version > 0)
);

CREATE INDEX agents_last_seen ON agents (last_seen_at);
CREATE INDEX agents_revoked ON agents (revoked_at);

CREATE TABLE agent_enrollment_tokens (
    id TEXT PRIMARY KEY,
    agent_id TEXT NOT NULL REFERENCES agents(id) ON DELETE RESTRICT,
    token_hash BLOB NOT NULL UNIQUE,
    expires_at TEXT NOT NULL,
    consumed_at TEXT,
    revoked_at TEXT,
    created_by TEXT REFERENCES users(id) ON DELETE RESTRICT,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE UNIQUE INDEX agent_enrollment_one_active
ON agent_enrollment_tokens (agent_id)
WHERE consumed_at IS NULL AND revoked_at IS NULL;

CREATE INDEX agent_enrollment_expires ON agent_enrollment_tokens (expires_at);

CREATE TABLE agent_credential_families (
    id TEXT PRIMARY KEY,
    agent_id TEXT NOT NULL REFERENCES agents(id) ON DELETE RESTRICT,
    revoked_at TEXT,
    revoke_reason TEXT,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE UNIQUE INDEX agent_credential_one_active_family
ON agent_credential_families (agent_id)
WHERE revoked_at IS NULL;

CREATE TABLE agent_refresh_credentials (
    id TEXT PRIMARY KEY,
    family_id TEXT NOT NULL REFERENCES agent_credential_families(id) ON DELETE RESTRICT,
    generation INTEGER NOT NULL CHECK (generation > 0),
    token_hash BLOB NOT NULL UNIQUE,
    expires_at TEXT NOT NULL,
    rotation_id TEXT,
    replaced_by_id TEXT REFERENCES agent_refresh_credentials(id) ON DELETE RESTRICT,
    committed_at TEXT,
    revoked_at TEXT,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    UNIQUE (family_id, generation),
    UNIQUE (family_id, rotation_id)
);

CREATE INDEX agent_refresh_expires ON agent_refresh_credentials (expires_at);

CREATE TABLE agent_access_sessions (
    id TEXT PRIMARY KEY,
    agent_id TEXT NOT NULL REFERENCES agents(id) ON DELETE RESTRICT,
    family_id TEXT NOT NULL REFERENCES agent_credential_families(id) ON DELETE RESTRICT,
    token_hash BLOB NOT NULL UNIQUE,
    expires_at TEXT NOT NULL,
    revoked_at TEXT,
    connection_id TEXT,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE INDEX agent_access_agent_expires ON agent_access_sessions (agent_id, expires_at);

CREATE TABLE agent_tasks (
    id TEXT PRIMARY KEY,
    agent_id TEXT NOT NULL REFERENCES agents(id) ON DELETE RESTRICT,
    deployment_id TEXT UNIQUE REFERENCES deployments(id) ON DELETE RESTRICT,
    kind TEXT NOT NULL CHECK (kind IN ('system_inspect', 'deployment_execute', 'health_diagnose')),
    idempotency_key TEXT NOT NULL,
    payload_digest TEXT NOT NULL,
    payload_json TEXT NOT NULL,
    status TEXT NOT NULL CHECK (status IN ('queued', 'delivered', 'accepted', 'running', 'canceling', 'succeeded', 'failed', 'canceled', 'interrupted')),
    deadline_at TEXT NOT NULL,
    lease_expires_at TEXT,
    delivered_at TEXT,
    acknowledged_at TEXT,
    started_at TEXT,
    finished_at TEXT,
    last_sequence INTEGER NOT NULL DEFAULT 0 CHECK (last_sequence >= 0),
    result_json TEXT,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    UNIQUE (agent_id, idempotency_key)
);

CREATE INDEX agent_tasks_dispatch ON agent_tasks (agent_id, status, created_at, id);

CREATE TABLE agent_task_events (
    task_id TEXT NOT NULL REFERENCES agent_tasks(id) ON DELETE RESTRICT,
    sequence INTEGER NOT NULL CHECK (sequence > 0),
    kind TEXT NOT NULL CHECK (kind IN ('output', 'state', 'result', 'diagnostic')),
    payload_json TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    PRIMARY KEY (task_id, sequence)
);

CREATE TABLE migration_0003_foreign_key_guard (
    valid INTEGER NOT NULL CHECK (valid = 1)
);

INSERT INTO migration_0003_foreign_key_guard (valid)
SELECT CASE WHEN EXISTS(SELECT 1 FROM pragma_foreign_key_check) THEN 0 ELSE 1 END;

DROP TABLE migration_0003_foreign_key_guard;
