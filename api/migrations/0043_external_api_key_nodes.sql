-- 外部 API Key 可用节点白名单：限制 Key 通过对外 API 新增或修改 Git 来源、
-- 固定工作区来源和部署目标时可以使用的节点。
-- 不回填任何数据：未配置即空集合，上述写入默认拒绝（fail closed）。
CREATE TABLE external_api_key_nodes (
    api_key_id TEXT NOT NULL REFERENCES external_api_keys(id) ON DELETE CASCADE,
    node_id TEXT NOT NULL REFERENCES nodes(id) ON DELETE RESTRICT,
    granted_by TEXT NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
    granted_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    PRIMARY KEY (api_key_id, node_id)
);

CREATE INDEX external_api_key_nodes_node
ON external_api_key_nodes (node_id);
