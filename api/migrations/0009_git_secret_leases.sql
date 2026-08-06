CREATE TABLE git_secret_leases (
    id TEXT PRIMARY KEY,
    task_id TEXT NOT NULL REFERENCES agent_tasks(id) ON DELETE CASCADE,
    git_credential_id TEXT NOT NULL REFERENCES git_credentials(id) ON DELETE RESTRICT,
    payload_digest TEXT NOT NULL,
    purpose TEXT NOT NULL CHECK (purpose = 'git_credential'),
    status TEXT NOT NULL CHECK (status IN ('issued', 'granted', 'rejected', 'expired')),
    expires_at TEXT NOT NULL,
    granted_at TEXT,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE INDEX git_secret_leases_task
ON git_secret_leases (task_id, status);

CREATE INDEX git_secret_leases_lease
ON git_secret_leases (id, status, expires_at);
