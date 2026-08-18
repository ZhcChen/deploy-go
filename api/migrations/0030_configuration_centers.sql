-- 配置中心基础模型：平台/自有 etcd、用途隔离凭据、应用绑定、一次性 reveal、
-- 业务身份、KV 变更事实、切换作业和通用 Secret environment lease。

CREATE TABLE configuration_center_credentials (
    id TEXT PRIMARY KEY,
    purpose TEXT NOT NULL CHECK (purpose IN (
        'platform_admin', 'custom_connection', 'business_identity'
    )),
    algorithm TEXT NOT NULL CHECK (algorithm IN (
        'chacha20poly1305-etcd-admin-v1',
        'chacha20poly1305-etcd-custom-v1',
        'chacha20poly1305-etcd-business-v1'
    )),
    ciphertext BLOB NOT NULL,
    nonce BLOB NOT NULL,
    key_version INTEGER NOT NULL CHECK (key_version > 0),
    status TEXT NOT NULL CHECK (status IN ('active', 'rotating', 'revoked')),
    created_by TEXT REFERENCES users(id) ON DELETE RESTRICT,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    version INTEGER NOT NULL DEFAULT 1 CHECK (version > 0)
);

CREATE INDEX configuration_center_credentials_status
ON configuration_center_credentials (status, purpose, id);

CREATE TABLE configuration_centers (
    id TEXT PRIMARY KEY,
    center_type TEXT NOT NULL CHECK (center_type = 'etcd'),
    scope TEXT NOT NULL CHECK (scope IN ('platform', 'custom')),
    application_id TEXT REFERENCES applications(id) ON DELETE RESTRICT,
    environment TEXT,
    endpoints_json TEXT NOT NULL CHECK (
        json_valid(endpoints_json) AND json_type(endpoints_json) = 'array'
    ),
    username TEXT NOT NULL,
    prefix TEXT NOT NULL DEFAULT '',
    credential_id TEXT NOT NULL REFERENCES configuration_center_credentials(id) ON DELETE RESTRICT,
    status TEXT NOT NULL CHECK (status IN (
        'unconfigured', 'unchecked', 'available', 'unavailable',
        'provisioning', 'retired'
    )),
    last_error_code TEXT,
    last_checked_at TEXT,
    managed_application_id TEXT REFERENCES applications(id) ON DELETE RESTRICT,
    managed_target_id TEXT REFERENCES deployment_targets(id) ON DELETE RESTRICT,
    managed_env_files_json TEXT NOT NULL DEFAULT '[]' CHECK (json_valid(managed_env_files_json)),
    data_volume_fact_json TEXT NOT NULL DEFAULT '{}' CHECK (json_valid(data_volume_fact_json)),
    created_by TEXT REFERENCES users(id) ON DELETE RESTRICT,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    version INTEGER NOT NULL DEFAULT 1 CHECK (version > 0),
    CHECK (
        (scope = 'platform' AND application_id IS NULL AND environment IS NULL)
        OR (scope = 'custom' AND application_id IS NOT NULL AND environment IS NOT NULL AND length(environment) > 0)
    )
);

CREATE UNIQUE INDEX configuration_centers_one_active_platform
ON configuration_centers (scope)
WHERE scope = 'platform' AND status <> 'retired';

CREATE UNIQUE INDEX configuration_centers_custom_application_environment
ON configuration_centers (application_id, environment)
WHERE scope = 'custom' AND status <> 'retired';

CREATE INDEX configuration_centers_status
ON configuration_centers (status, updated_at DESC, id DESC);

CREATE TABLE application_configuration_centers (
    id TEXT PRIMARY KEY,
    application_id TEXT NOT NULL REFERENCES applications(id) ON DELETE RESTRICT,
    environment TEXT NOT NULL,
    mode TEXT NOT NULL CHECK (mode IN ('none', 'platform_etcd', 'custom_etcd')),
    configuration_center_id TEXT REFERENCES configuration_centers(id) ON DELETE RESTRICT,
    prefix TEXT NOT NULL DEFAULT '',
    credential_id TEXT REFERENCES configuration_center_credentials(id) ON DELETE RESTRICT,
    identity_id TEXT,
    status TEXT NOT NULL CHECK (status IN (
        'ready', 'unavailable', 'pending_redeploy', 'provisioning', 'cleanup_pending'
    )),
    last_error_code TEXT,
    last_checked_at TEXT,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    version INTEGER NOT NULL DEFAULT 1 CHECK (version > 0),
    UNIQUE (application_id, environment),
    CHECK (
        (mode = 'none' AND configuration_center_id IS NULL AND credential_id IS NULL AND identity_id IS NULL AND prefix = '')
        OR (mode <> 'none' AND configuration_center_id IS NOT NULL AND credential_id IS NOT NULL AND length(prefix) > 0)
    )
);

CREATE INDEX application_configuration_centers_status
ON application_configuration_centers (status, application_id, environment);

CREATE TABLE configuration_center_reveals (
    id TEXT PRIMARY KEY,
    credential_id TEXT NOT NULL REFERENCES configuration_center_credentials(id) ON DELETE RESTRICT,
    configuration_center_id TEXT NOT NULL REFERENCES configuration_centers(id) ON DELETE RESTRICT,
    deployment_id TEXT REFERENCES deployments(id) ON DELETE RESTRICT,
    purpose TEXT NOT NULL CHECK (purpose = 'platform_admin'),
    status TEXT NOT NULL CHECK (status IN ('pending', 'consumed', 'expired', 'revoked')),
    expires_at TEXT NOT NULL,
    consumed_at TEXT,
    consumed_by TEXT REFERENCES users(id) ON DELETE RESTRICT,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    CHECK ((status = 'consumed') = (consumed_at IS NOT NULL AND consumed_by IS NOT NULL))
);

CREATE UNIQUE INDEX configuration_center_reveals_one_pending
ON configuration_center_reveals (credential_id)
WHERE status = 'pending';

CREATE INDEX configuration_center_reveals_lookup
ON configuration_center_reveals (id, credential_id, status, expires_at);

CREATE TABLE configuration_center_identities (
    id TEXT PRIMARY KEY,
    binding_id TEXT NOT NULL REFERENCES application_configuration_centers(id) ON DELETE RESTRICT,
    username TEXT NOT NULL,
    role_name TEXT NOT NULL,
    credential_id TEXT NOT NULL REFERENCES configuration_center_credentials(id) ON DELETE RESTRICT,
    remote_status TEXT NOT NULL CHECK (remote_status IN ('pending', 'active', 'revoked', 'cleanup_pending')),
    remote_version TEXT,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    version INTEGER NOT NULL DEFAULT 1 CHECK (version > 0),
    UNIQUE (binding_id),
    UNIQUE (username),
    UNIQUE (role_name)
);

CREATE TABLE configuration_center_kv_mutations (
    id TEXT PRIMARY KEY,
    binding_id TEXT NOT NULL REFERENCES application_configuration_centers(id) ON DELETE RESTRICT,
    relative_key TEXT NOT NULL,
    action TEXT NOT NULL CHECK (action IN ('put', 'delete')),
    etcd_revision INTEGER NOT NULL CHECK (etcd_revision > 0),
    value_hmac TEXT,
    value_length INTEGER CHECK (value_length IS NULL OR value_length >= 0),
    actor_id TEXT REFERENCES users(id) ON DELETE RESTRICT,
    request_id TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    UNIQUE (binding_id, relative_key, etcd_revision)
);

CREATE INDEX configuration_center_kv_mutations_lookup
ON configuration_center_kv_mutations (binding_id, relative_key, etcd_revision DESC);

CREATE TABLE configuration_center_switches (
    id TEXT PRIMARY KEY,
    source_center_id TEXT NOT NULL REFERENCES configuration_centers(id) ON DELETE RESTRICT,
    destination_center_id TEXT NOT NULL REFERENCES configuration_centers(id) ON DELETE RESTRICT,
    status TEXT NOT NULL CHECK (status IN (
        'prechecking', 'copying', 'verifying', 'identities',
        'awaiting_confirmation', 'switching', 'succeeded', 'failed', 'canceled'
    )),
    source_revision INTEGER CHECK (source_revision IS NULL OR source_revision > 0),
    destination_revision INTEGER CHECK (destination_revision IS NULL OR destination_revision > 0),
    source_key_count INTEGER CHECK (source_key_count IS NULL OR source_key_count >= 0),
    destination_key_count INTEGER CHECK (destination_key_count IS NULL OR destination_key_count >= 0),
    source_digest TEXT,
    destination_digest TEXT,
    cursor_key TEXT,
    error_code TEXT,
    requested_by TEXT NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
    confirmed_by TEXT REFERENCES users(id) ON DELETE RESTRICT,
    confirmed_at TEXT,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    version INTEGER NOT NULL DEFAULT 1 CHECK (version > 0),
    CHECK (source_center_id <> destination_center_id)
);

CREATE INDEX configuration_center_switches_status
ON configuration_center_switches (status, updated_at, id);

CREATE TABLE secret_environment_leases (
    id TEXT PRIMARY KEY,
    deployment_id TEXT NOT NULL REFERENCES deployments(id) ON DELETE RESTRICT,
    task_id TEXT NOT NULL REFERENCES agent_tasks(id) ON DELETE RESTRICT,
    target_run_id TEXT NOT NULL REFERENCES deployment_target_runs(id) ON DELETE RESTRICT,
    agent_id TEXT NOT NULL REFERENCES agents(id) ON DELETE RESTRICT,
    connection_generation INTEGER NOT NULL CHECK (connection_generation > 0),
    purpose TEXT NOT NULL CHECK (purpose IN ('etcd-init', 'config-center-connection')),
    variable_names_json TEXT NOT NULL CHECK (json_valid(variable_names_json)),
    payload_digest TEXT NOT NULL,
    credential_version INTEGER NOT NULL CHECK (credential_version > 0),
    template_id TEXT NOT NULL,
    template_version TEXT NOT NULL,
    template_digest TEXT NOT NULL,
    release_stage TEXT NOT NULL CHECK (release_stage IN ('prepare', 'release')),
    executor_audience TEXT NOT NULL,
    target_process TEXT NOT NULL,
    status TEXT NOT NULL CHECK (status IN ('issued', 'granted', 'consumed', 'revoked', 'expired')),
    expires_at TEXT NOT NULL,
    granted_at TEXT,
    consumed_at TEXT,
    revoked_at TEXT,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    UNIQUE (task_id, purpose),
    CHECK (json_type(variable_names_json) = 'array')
);

CREATE INDEX secret_environment_leases_lookup
ON secret_environment_leases (id, agent_id, task_id, status, expires_at);
