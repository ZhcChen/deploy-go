-- 镜像直连部署：execution_mode 增加 image，并保存白名单 image_spec。
DROP TRIGGER IF EXISTS deployments_application_matches_target_insert;
DROP TRIGGER IF EXISTS deployments_application_immutable_update;

CREATE TABLE deployment_targets_new (
    id TEXT PRIMARY KEY,
    application_id TEXT NOT NULL REFERENCES applications(id) ON DELETE RESTRICT,
    node_id TEXT NOT NULL REFERENCES nodes(id) ON DELETE RESTRICT,
    environment TEXT NOT NULL,
    execution_mode TEXT NOT NULL DEFAULT 'script'
        CHECK (execution_mode IN ('script', 'two_stage', 'image')),
    script_path TEXT NOT NULL DEFAULT '',
    parameter_schema TEXT NOT NULL DEFAULT '{}',
    timeout_seconds INTEGER NOT NULL CHECK (timeout_seconds BETWEEN 1 AND 86400),
    verification_config TEXT NOT NULL DEFAULT '{}',
    privileged_release INTEGER NOT NULL DEFAULT 0
        CHECK (privileged_release IN (0, 1)),
    image_spec_json TEXT,
    status TEXT NOT NULL CHECK (status IN ('active', 'disabled')),
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    version INTEGER NOT NULL DEFAULT 1 CHECK (version > 0),
    UNIQUE (application_id, environment, node_id),
    CHECK (
        (execution_mode = 'image' AND privileged_release = 1 AND image_spec_json IS NOT NULL AND length(image_spec_json) > 0)
        OR (execution_mode <> 'image' AND image_spec_json IS NULL)
    )
);

INSERT INTO deployment_targets_new (
    id, application_id, node_id, environment, execution_mode, script_path,
    parameter_schema, timeout_seconds, verification_config, privileged_release,
    image_spec_json, status, created_at, updated_at, version
)
SELECT
    id, application_id, node_id, environment, execution_mode, script_path,
    parameter_schema, timeout_seconds, verification_config, privileged_release,
    NULL, status, created_at, updated_at, version
FROM deployment_targets;

DROP TABLE deployment_targets;
ALTER TABLE deployment_targets_new RENAME TO deployment_targets;

CREATE INDEX deployment_targets_node_id ON deployment_targets (node_id);

CREATE TRIGGER deployments_application_matches_target_insert
BEFORE INSERT ON deployments
WHEN NEW.application_id IS NOT NULL
 AND NEW.application_id <> (SELECT application_id FROM deployment_targets WHERE id = NEW.target_id)
BEGIN
    SELECT RAISE(ABORT, 'deployments.application_id must match target');
END;

CREATE TRIGGER deployments_application_immutable_update
BEFORE UPDATE OF application_id, target_id ON deployments
WHEN NOT (NEW.application_id IS OLD.application_id) OR NEW.target_id <> OLD.target_id
BEGIN
    SELECT RAISE(ABORT, 'deployment ownership is immutable');
END;

CREATE TABLE migration_0020_foreign_key_guard (
    valid INTEGER NOT NULL CHECK (valid = 1)
);

INSERT INTO migration_0020_foreign_key_guard (valid)
SELECT CASE WHEN EXISTS(SELECT 1 FROM pragma_foreign_key_check) THEN 0 ELSE 1 END;

DROP TABLE migration_0020_foreign_key_guard;
