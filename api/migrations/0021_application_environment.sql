-- 应用环境标识：applications 增加环境字段，并回填目标环境跟随应用环境。
ALTER TABLE applications
ADD COLUMN environment TEXT NOT NULL DEFAULT 'prod'
CHECK (environment IN ('dev', 'test', 'staging', 'prod'));

-- 应用环境优先取自目标节点上未吊销、未归档且最近活跃的 Agent 环境。
-- 无明确依据的应用保持 prod，避免破坏现有生产部署。
UPDATE applications
SET environment = COALESCE((
    SELECT agent.environment
    FROM deployment_targets target
    JOIN agents agent ON agent.node_id = target.node_id
    WHERE target.application_id = applications.id
      AND agent.revoked_at IS NULL
      AND agent.archived_at IS NULL
      AND agent.environment IN ('dev', 'test', 'staging', 'prod')
    ORDER BY agent.last_seen_at DESC, agent.id
    LIMIT 1
), 'prod');

-- 目标环境跟随应用环境。为避免 UNIQUE(application_id, environment, node_id)
-- 在历史多环境目标上冲突，只回填“同应用同节点唯一目标”以及已与应用环境一致的目标；
-- 存在历史冲突组合时保留原值，不阻塞线上迁移。
UPDATE deployment_targets
SET environment = (
        SELECT application.environment
        FROM applications application
        WHERE application.id = deployment_targets.application_id
    ),
    version = version + 1,
    updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
WHERE environment <> (
        SELECT application.environment
        FROM applications application
        WHERE application.id = deployment_targets.application_id
    )
  AND (
        (
            SELECT COUNT(*)
            FROM deployment_targets other
            WHERE other.application_id = deployment_targets.application_id
              AND other.node_id = deployment_targets.node_id
        ) = 1
        OR environment = (
            SELECT application.environment
            FROM applications application
            WHERE application.id = deployment_targets.application_id
        )
    );

CREATE TABLE migration_0021_foreign_key_guard (
    valid INTEGER NOT NULL CHECK (valid = 1)
);

INSERT INTO migration_0021_foreign_key_guard (valid)
SELECT CASE WHEN EXISTS(SELECT 1 FROM pragma_foreign_key_check) THEN 0 ELSE 1 END;

DROP TABLE migration_0021_foreign_key_guard;
