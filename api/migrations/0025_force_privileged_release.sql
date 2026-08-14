-- 特权发布改为平台固定能力：移除目标级 privileged_release 关闭概念。
-- 迁移门禁禁止 DROP COLUMN，列保留但固定为 1；API/Web 不再暴露该配置。
UPDATE deployment_targets
SET privileged_release = 1
WHERE privileged_release = 0;
