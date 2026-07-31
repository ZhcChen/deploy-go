CREATE TABLE users (
    id TEXT PRIMARY KEY,
    username TEXT NOT NULL COLLATE NOCASE UNIQUE,
    password_hash TEXT NOT NULL,
    identity TEXT NOT NULL CHECK (identity IN ('administrator', 'user')),
    status TEXT NOT NULL CHECK (status IN ('active', 'disabled')),
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    version INTEGER NOT NULL DEFAULT 1 CHECK (version > 0)
);

CREATE UNIQUE INDEX users_single_administrator
ON users (identity)
WHERE identity = 'administrator';

CREATE TABLE sessions (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
    token_hash BLOB NOT NULL UNIQUE,
    csrf_hash BLOB NOT NULL,
    expires_at TEXT NOT NULL,
    revoked_at TEXT,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    last_seen_at TEXT
);

CREATE INDEX sessions_user_id ON sessions (user_id);
CREATE INDEX sessions_expires_at ON sessions (expires_at);

CREATE TABLE ssh_credentials (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL COLLATE NOCASE UNIQUE,
    algorithm TEXT NOT NULL CHECK (algorithm = 'ed25519'),
    public_key TEXT NOT NULL,
    fingerprint TEXT NOT NULL UNIQUE,
    encrypted_private_key BLOB NOT NULL,
    nonce BLOB NOT NULL,
    key_version INTEGER NOT NULL CHECK (key_version > 0),
    created_by TEXT REFERENCES users(id) ON DELETE RESTRICT,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    version INTEGER NOT NULL DEFAULT 1 CHECK (version > 0)
);

CREATE TABLE nodes (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL COLLATE NOCASE UNIQUE,
    host TEXT NOT NULL,
    port INTEGER NOT NULL CHECK (port BETWEEN 1 AND 65535),
    username TEXT NOT NULL,
    ssh_credential_id TEXT REFERENCES ssh_credentials(id) ON DELETE RESTRICT,
    work_root TEXT NOT NULL,
    secrets_root TEXT NOT NULL,
    status TEXT NOT NULL CHECK (status IN ('missing_credential', 'unchecked', 'checking', 'online', 'offline', 'disabled')),
    trusted_host_key TEXT,
    trusted_host_fingerprint TEXT,
    checked_at TEXT,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    version INTEGER NOT NULL DEFAULT 1 CHECK (version > 0),
    CHECK (ssh_credential_id IS NOT NULL OR status IN ('missing_credential', 'disabled'))
);

CREATE INDEX nodes_ssh_credential_id ON nodes (ssh_credential_id);
CREATE INDEX nodes_status ON nodes (status);

CREATE TABLE node_checks (
    id TEXT PRIMARY KEY,
    node_id TEXT NOT NULL REFERENCES nodes(id) ON DELETE RESTRICT,
    status TEXT NOT NULL CHECK (status IN ('pending', 'running', 'succeeded', 'failed')),
    failure_code TEXT,
    failure_message TEXT,
    os_name TEXT,
    architecture TEXT,
    disk_available_bytes INTEGER CHECK (disk_available_bytes >= 0),
    capabilities_json TEXT NOT NULL DEFAULT '{}',
    host_fingerprint TEXT,
    started_at TEXT,
    finished_at TEXT,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE INDEX node_checks_node_created ON node_checks (node_id, created_at DESC);

CREATE TABLE applications (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL COLLATE NOCASE UNIQUE,
    slug TEXT NOT NULL COLLATE NOCASE UNIQUE,
    description TEXT NOT NULL DEFAULT '',
    status TEXT NOT NULL CHECK (status IN ('active', 'archived')),
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    version INTEGER NOT NULL DEFAULT 1 CHECK (version > 0)
);

CREATE TABLE user_application_grants (
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
    application_id TEXT NOT NULL REFERENCES applications(id) ON DELETE RESTRICT,
    granted_by TEXT NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
    granted_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    PRIMARY KEY (user_id, application_id)
);

CREATE INDEX grants_application_id ON user_application_grants (application_id);

CREATE TABLE deployment_targets (
    id TEXT PRIMARY KEY,
    application_id TEXT NOT NULL REFERENCES applications(id) ON DELETE RESTRICT,
    node_id TEXT NOT NULL REFERENCES nodes(id) ON DELETE RESTRICT,
    environment TEXT NOT NULL,
    script_path TEXT NOT NULL,
    parameter_schema TEXT NOT NULL DEFAULT '{}',
    timeout_seconds INTEGER NOT NULL CHECK (timeout_seconds BETWEEN 1 AND 86400),
    verification_config TEXT NOT NULL DEFAULT '{}',
    status TEXT NOT NULL CHECK (status IN ('active', 'disabled')),
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    version INTEGER NOT NULL DEFAULT 1 CHECK (version > 0),
    UNIQUE (application_id, environment, node_id)
);

CREATE INDEX deployment_targets_node_id ON deployment_targets (node_id);

CREATE TABLE secret_file_references (
    id TEXT PRIMARY KEY,
    deployment_target_id TEXT NOT NULL REFERENCES deployment_targets(id) ON DELETE RESTRICT,
    environment_key TEXT NOT NULL,
    file_path TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    version INTEGER NOT NULL DEFAULT 1 CHECK (version > 0),
    UNIQUE (deployment_target_id, environment_key)
);

CREATE TABLE deployments (
    id TEXT PRIMARY KEY,
    target_id TEXT NOT NULL REFERENCES deployment_targets(id) ON DELETE RESTRICT,
    requested_by TEXT NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
    retry_of_id TEXT REFERENCES deployments(id) ON DELETE RESTRICT,
    status TEXT NOT NULL CHECK (status IN ('queued', 'running', 'succeeded', 'failed', 'canceling', 'canceled', 'interrupted')),
    phase TEXT NOT NULL,
    idempotency_key TEXT NOT NULL,
    request_hash TEXT NOT NULL,
    snapshot_hash TEXT NOT NULL,
    snapshot_json TEXT NOT NULL DEFAULT '{}',
    result_summary TEXT,
    exit_code INTEGER,
    protocol_complete INTEGER NOT NULL DEFAULT 0 CHECK (protocol_complete IN (0, 1)),
    queued_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    started_at TEXT,
    finished_at TEXT,
    cancel_requested_at TEXT,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    version INTEGER NOT NULL DEFAULT 1 CHECK (version > 0),
    UNIQUE (requested_by, idempotency_key)
);

CREATE UNIQUE INDEX deployments_one_execution_owner_per_target
ON deployments (target_id)
WHERE status IN ('running', 'canceling');

CREATE INDEX deployments_queue ON deployments (status, queued_at, id);
CREATE INDEX deployments_target_created ON deployments (target_id, created_at DESC);
CREATE INDEX deployments_requested_by ON deployments (requested_by, created_at DESC);

CREATE TABLE deployment_logs (
    deployment_id TEXT NOT NULL REFERENCES deployments(id) ON DELETE RESTRICT,
    sequence INTEGER NOT NULL CHECK (sequence > 0),
    stream TEXT NOT NULL CHECK (stream IN ('stdout', 'stderr', 'system')),
    content TEXT NOT NULL,
    truncated INTEGER NOT NULL DEFAULT 0 CHECK (truncated IN (0, 1)),
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    PRIMARY KEY (deployment_id, sequence)
);

CREATE TABLE deployment_events (
    id TEXT PRIMARY KEY,
    deployment_id TEXT NOT NULL REFERENCES deployments(id) ON DELETE RESTRICT,
    log_sequence INTEGER,
    event_name TEXT NOT NULL,
    status TEXT,
    payload_json TEXT NOT NULL,
    diagnostic_code TEXT,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    FOREIGN KEY (deployment_id, log_sequence) REFERENCES deployment_logs(deployment_id, sequence) ON DELETE RESTRICT
);

CREATE INDEX deployment_events_deployment_created ON deployment_events (deployment_id, created_at);

CREATE TABLE audit_logs (
    id TEXT PRIMARY KEY,
    actor_id TEXT REFERENCES users(id) ON DELETE RESTRICT,
    action TEXT NOT NULL,
    resource_type TEXT NOT NULL,
    resource_id TEXT NOT NULL,
    request_id TEXT NOT NULL,
    summary_json TEXT NOT NULL DEFAULT '{}',
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE INDEX audit_logs_created ON audit_logs (created_at DESC, id DESC);
CREATE INDEX audit_logs_resource ON audit_logs (resource_type, resource_id, created_at DESC);

CREATE TABLE system_settings (
    key TEXT PRIMARY KEY,
    value_json TEXT NOT NULL,
    updated_by TEXT REFERENCES users(id) ON DELETE RESTRICT,
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    version INTEGER NOT NULL DEFAULT 1 CHECK (version > 0)
);
