-- 节点归档：NULL 表示正常，非 NULL 表示已归档（记录归档时间）。
-- 归档节点不参与部署调度、能力检查与终端连接，但保留历史部署记录。
ALTER TABLE nodes ADD COLUMN archived_at TEXT;
