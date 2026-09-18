-- Git 来源的源码物化策略：缺省完整检出，显式 sparse 才按 allowlist 检出。
ALTER TABLE application_sources
ADD COLUMN source_materialization_json TEXT NOT NULL
    DEFAULT '{"mode":"full","paths":[]}';
