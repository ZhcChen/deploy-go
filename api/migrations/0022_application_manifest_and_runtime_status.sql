-- 应用类型清单、目标稳定标识与运行时状态。

ALTER TABLE applications
ADD COLUMN app_type TEXT NOT NULL DEFAULT 'binary'
CHECK (app_type IN ('binary', 'redis', 'postgres'));

ALTER TABLE applications
ADD COLUMN type_version TEXT NOT NULL DEFAULT '1'
CHECK (length(type_version) <= 32);

UPDATE applications SET type_version = '1' WHERE app_type = 'binary';

ALTER TABLE deployment_targets
ADD COLUMN target_code TEXT NOT NULL DEFAULT 'prod';

UPDATE deployment_targets SET target_code = environment;

CREATE UNIQUE INDEX deployment_targets_app_node_target_code
ON deployment_targets (application_id, node_id, target_code);

CREATE TABLE application_runtime_statuses (
    runtime_status_id TEXT PRIMARY KEY,
    application_id TEXT NOT NULL REFERENCES applications(id) ON DELETE RESTRICT,
    target_id TEXT NOT NULL REFERENCES deployment_targets(id) ON DELETE RESTRICT,
    status TEXT NOT NULL CHECK (status IN ('pending', 'running', 'succeeded', 'failed')),
    payload_json TEXT,
    error_code TEXT,
    error_message TEXT,
    requested_by TEXT REFERENCES users(id) ON DELETE RESTRICT,
    requested_at TEXT NOT NULL,
    observed_at TEXT,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE INDEX application_runtime_statuses_target_latest
ON application_runtime_statuses (application_id, target_id, created_at, runtime_status_id);

ALTER TABLE agent_tasks
ADD COLUMN runtime_status_id TEXT
REFERENCES application_runtime_statuses(runtime_status_id) ON DELETE RESTRICT;

CREATE UNIQUE INDEX agent_tasks_runtime_status_id
ON agent_tasks (runtime_status_id)
WHERE runtime_status_id IS NOT NULL;
