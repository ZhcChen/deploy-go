-- 应用名称允许重复，slug 保持全局唯一。
--
-- SQLite 不能删除既有 UNIQUE 约束，且迁移门禁禁止 DROP TABLE / DROP COLUMN；
-- 因此保留原 name 列作为内部唯一键（回填为应用 id），新增 display_name
-- 作为面向用户的展示名称。API 与外部读取只使用 display_name，name 不再暴露。

ALTER TABLE applications ADD COLUMN display_name TEXT NOT NULL DEFAULT '';

UPDATE applications SET display_name = name;
UPDATE applications SET name = id;

-- 兼容直接写入 applications 的调用方：未提供 display_name 时自动用 name 回填，
-- 避免展示名出现空值。API 创建应用会显式写入 display_name，不依赖该触发器。
CREATE TRIGGER applications_display_name_backfill_after_insert
AFTER INSERT ON applications
WHEN NEW.display_name = ''
BEGIN
    UPDATE applications SET display_name = NEW.name WHERE id = NEW.id;
END;
