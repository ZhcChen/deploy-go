-- 控制台会话改为“空闲期滑动续期 + 绝对期上限”：
-- expires_at 仍然表示当前生效的到期时间（续期时取 min(现在 + 空闲期, absolute_expires_at)），
-- 所以所有按 expires_at 判断会话有效性的查询无需修改，绝对期上限天然生效；
-- absolute_expires_at 只在续期时使用。旧会话该列为 NULL：保持原来的固定到期、不续期。
ALTER TABLE sessions ADD COLUMN absolute_expires_at TEXT;
