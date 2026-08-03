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
        OR ssh_credential_id IS NULL
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

CREATE TABLE migration_0005_foreign_key_guard (
    valid INTEGER NOT NULL CHECK (valid = 1)
);

INSERT INTO migration_0005_foreign_key_guard (valid)
SELECT CASE WHEN EXISTS(SELECT 1 FROM pragma_foreign_key_check) THEN 0 ELSE 1 END;

DROP TABLE migration_0005_foreign_key_guard;
