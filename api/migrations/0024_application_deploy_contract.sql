-- 应用级部署契约：参数 Schema 与验证配置从部署目标上移到应用。
--
-- 设计约束（迁移门禁）：
--   新增 migration 不允许 DROP TABLE / DROP COLUMN，因此不重建 deployment_targets。
--   目标表旧列 parameter_schema / verification_config 保留但弃用：
--   API 创建/更新目标不再接受这两个字段，部署快照、preview、confirm
--   一律读取 applications.parameter_schema / applications.verification_config。
--
-- 回填规则：
--   - 同应用多目标值不一致时取最近更新目标（updated_at DESC, id DESC）。
--   - 镜像目标遗留的 '{}' 视为未配置，回填有效默认值。
--   - 无目标的应用回填有效默认值。
--
-- 该文件只新增应用级列并回填数据，不改动部署目标表的既有行和触发器。

ALTER TABLE applications
ADD COLUMN parameter_schema TEXT NOT NULL DEFAULT '{"type":"object","properties":{},"required":[],"additionalProperties":false}';

ALTER TABLE applications
ADD COLUMN verification_config TEXT NOT NULL DEFAULT '{"type":"http","path":"/healthz","expected_status":200,"timeout_ms":5000}';

-- 从目标回填参数 Schema：非空非 '{}' 的目标值优先，否则使用空对象默认值。
UPDATE applications
SET parameter_schema = COALESCE((
        SELECT CASE
            WHEN target.parameter_schema IN ('{}', '')
                THEN '{"type":"object","properties":{},"required":[],"additionalProperties":false}'
            ELSE target.parameter_schema
        END
        FROM deployment_targets target
        WHERE target.application_id = applications.id
        ORDER BY target.updated_at DESC, target.id DESC
        LIMIT 1
    ), '{"type":"object","properties":{},"required":[],"additionalProperties":false}');

-- 从目标回填验证配置：非空非 '{}' 的目标值优先，否则使用 HTTP healthz 默认值。
UPDATE applications
SET verification_config = COALESCE((
        SELECT CASE
            WHEN target.verification_config IN ('{}', '')
                THEN '{"type":"http","path":"/healthz","expected_status":200,"timeout_ms":5000}'
            ELSE target.verification_config
        END
        FROM deployment_targets target
        WHERE target.application_id = applications.id
        ORDER BY target.updated_at DESC, target.id DESC
        LIMIT 1
    ), '{"type":"http","path":"/healthz","expected_status":200,"timeout_ms":5000}');
