-- 应用标签用于跨项目/用途区分应用，支持一个应用关联多个标签。
-- 标签不参与部署身份，slug 仍保持全局唯一。

CREATE TABLE application_tags (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL COLLATE NOCASE UNIQUE,
    created_by TEXT REFERENCES users(id) ON DELETE RESTRICT,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE TABLE application_tag_links (
    application_id TEXT NOT NULL REFERENCES applications(id) ON DELETE RESTRICT,
    tag_id TEXT NOT NULL REFERENCES application_tags(id) ON DELETE RESTRICT,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    PRIMARY KEY (application_id, tag_id)
);

CREATE INDEX application_tag_links_tag_id
ON application_tag_links (tag_id);
