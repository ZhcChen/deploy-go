-- 应用类型枚举扩展：valkey / etcd 模板已加入平台，0022 的 CHECK 未同步放宽。
--
-- SQLite 不能直接修改既有 CHECK；按迁移门禁不得 DROP COLUMN，因此把旧列
-- 重命名为 deprecated 的 app_type_legacy，新增同名 app_type 列并回填。
-- 应用读写全部使用新列，旧列仅保留迁移兼容性。

ALTER TABLE applications RENAME COLUMN app_type TO app_type_legacy;

ALTER TABLE applications
ADD COLUMN app_type TEXT NOT NULL DEFAULT 'binary'
CHECK (app_type IN ('binary', 'redis', 'valkey', 'postgres', 'etcd'));

UPDATE applications SET app_type = app_type_legacy;
