-- 应用模板绑定和通用配置工作区。
-- 配置正文只存在于 application_config_versions 的加密字段；旧 application_env_* 表
-- 保持原语义，既有 Env 登记和同步链路不由本迁移改写。
CREATE TABLE application_template_bindings (
    id TEXT PRIMARY KEY,
    application_id TEXT NOT NULL REFERENCES applications(id) ON DELETE RESTRICT,
    target_id TEXT REFERENCES deployment_targets(id) ON DELETE RESTRICT,
    template_id TEXT NOT NULL,
    template_version TEXT NOT NULL,
    template_digest TEXT NOT NULL,
    source TEXT NOT NULL CHECK (source IN ('template_creation', 'legacy_initialization')),
    status TEXT NOT NULL DEFAULT 'active'
        CHECK (status IN ('draft', 'active', 'deleted')),
    enabled INTEGER NOT NULL DEFAULT 0 CHECK (enabled IN (0, 1)),
    created_by TEXT REFERENCES users(id) ON DELETE RESTRICT,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    version INTEGER NOT NULL DEFAULT 1 CHECK (version > 0),
    UNIQUE (id, application_id),
    UNIQUE (application_id, template_id, template_version)
);

CREATE INDEX application_template_bindings_application
ON application_template_bindings (application_id, status, created_at DESC);

CREATE UNIQUE INDEX application_template_bindings_one_live_per_application
ON application_template_bindings (application_id)
WHERE status <> 'deleted';

CREATE TABLE application_config_files (
    id TEXT PRIMARY KEY,
    binding_id TEXT NOT NULL,
    application_id TEXT NOT NULL REFERENCES applications(id) ON DELETE RESTRICT,
    path TEXT NOT NULL,
    deploy_path TEXT,
    label TEXT NOT NULL DEFAULT '',
    format TEXT NOT NULL CHECK (format IN ('yaml', 'dotenv', 'ini', 'json', 'markdown', 'shell', 'makefile')),
    language TEXT NOT NULL,
    role TEXT NOT NULL CHECK (role IN ('configuration', 'reference', 'platform_managed')),
    delivery TEXT NOT NULL CHECK (delivery IN ('artifact', 'env_lease', 'secret_file_lease', 'reference', 'platform_managed')),
    sensitive INTEGER NOT NULL DEFAULT 0 CHECK (sensitive IN (0, 1)),
    editable INTEGER NOT NULL DEFAULT 0 CHECK (editable IN (0, 1)),
    description TEXT NOT NULL DEFAULT '',
    recommended_changes TEXT NOT NULL DEFAULT '',
    template_source_digest TEXT NOT NULL,
    current_version INTEGER NOT NULL DEFAULT 1 CHECK (current_version > 0),
    current_digest TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'ready' CHECK (status IN ('ready', 'incomplete')),
    deleted_at TEXT,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    version INTEGER NOT NULL DEFAULT 1 CHECK (version > 0),
    UNIQUE (id, application_id),
    UNIQUE (binding_id, path),
    UNIQUE (application_id, path),
    FOREIGN KEY (binding_id, application_id)
        REFERENCES application_template_bindings (id, application_id) ON DELETE RESTRICT
);

CREATE INDEX application_config_files_application
ON application_config_files (application_id, deleted_at, path);

CREATE TABLE application_config_versions (
    id TEXT PRIMARY KEY,
    application_config_file_id TEXT NOT NULL,
    application_id TEXT NOT NULL REFERENCES applications(id) ON DELETE RESTRICT,
    config_version INTEGER NOT NULL CHECK (config_version > 0),
    algorithm TEXT NOT NULL CHECK (algorithm = 'chacha20poly1305-application-config-v1'),
    ciphertext BLOB NOT NULL,
    nonce BLOB NOT NULL,
    key_version INTEGER NOT NULL CHECK (key_version > 0),
    digest TEXT NOT NULL,
    source TEXT NOT NULL CHECK (source IN ('template', 'user', 'restore_version', 'restore_template', 'legacy_initialization')),
    source_version_id TEXT,
    source_template_digest TEXT,
    created_by TEXT REFERENCES users(id) ON DELETE RESTRICT,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    UNIQUE (application_config_file_id, config_version),
    UNIQUE (id, application_id),
    FOREIGN KEY (application_config_file_id, application_id)
        REFERENCES application_config_files (id, application_id) ON DELETE RESTRICT
);

CREATE INDEX application_config_versions_history
ON application_config_versions (application_config_file_id, config_version DESC);

-- 配置版本的身份、来源和内容摘要不可变；换钥只更新等价密文表示。
-- 内容恢复通过插入新版本完成，历史版本不能删除。
CREATE TRIGGER application_config_versions_immutable_update
BEFORE UPDATE OF id, application_config_file_id, application_id, config_version, digest,
    source, source_version_id, source_template_digest, created_by, created_at
ON application_config_versions
BEGIN
    SELECT RAISE(ABORT, 'application config versions are immutable');
END;

CREATE TRIGGER application_config_versions_immutable_delete
BEFORE DELETE ON application_config_versions
BEGIN
    SELECT RAISE(ABORT, 'application config versions are immutable');
END;

CREATE TRIGGER application_template_bindings_immutable_identity
BEFORE UPDATE OF application_id, target_id, template_id, template_version, template_digest, source
ON application_template_bindings
WHEN NEW.application_id IS NOT OLD.application_id
  OR NEW.target_id IS NOT OLD.target_id
  OR NEW.template_id <> OLD.template_id
  OR NEW.template_version <> OLD.template_version
  OR NEW.template_digest <> OLD.template_digest
  OR NEW.source <> OLD.source
BEGIN
    SELECT RAISE(ABORT, 'application template binding identity is immutable');
END;
