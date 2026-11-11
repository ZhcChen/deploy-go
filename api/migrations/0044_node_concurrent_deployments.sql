-- 同节点并发部署数：同一个节点（Agent）上允许同时运行多少个部署。
--
-- 默认 1，即保持现状（同一节点一次只运行一个部署）。只有同时满足两个条件才会真正并发：
-- 节点本机的 runner broker 开启了并发（Agent 声明 concurrent_deployments_v1），并且这里大于 1。
-- 范围与协议常量 MAX_CONCURRENT_DEPLOYMENTS_PER_NODE 一致（16）。
ALTER TABLE nodes ADD COLUMN max_concurrent_deployments INTEGER NOT NULL DEFAULT 1
    CHECK (max_concurrent_deployments BETWEEN 1 AND 16);

-- 测试与开发环境的节点：多个项目共同开发、部署频繁，默认设为 10。
-- 只匹配有未撤销、未归档 Agent 的节点；预发布与生产环境保持 1。
-- 即使这里为 10，只要节点本机没有开启并发（没有声明能力），实际仍是 1。
UPDATE nodes
SET max_concurrent_deployments = 10
WHERE archived_at IS NULL
  AND id IN (
      SELECT node_id FROM agents
      WHERE environment IN ('dev', 'test') AND revoked_at IS NULL AND archived_at IS NULL
  );

-- 全平台总并发（系统设置“最大并发部署数”）低于 10 时提到 10，否则它会把多节点、多应用的并发压回去。
-- 只提高、不降低；设置行不存在时由代码默认值生效，不在这里新建。
UPDATE system_settings
SET value_json = json_set(value_json, '$.max_concurrent_deployments', 10),
    updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
    version = version + 1
WHERE key = 'runtime'
  AND json_valid(value_json)
  AND CAST(json_extract(value_json, '$.max_concurrent_deployments') AS INTEGER) < 10;
