import type { ImageTemplate } from "../../api/generated/models/ImageTemplate";

export interface ImageTemplateOption {
  value: ImageTemplate;
  label: string;
  image: string;
  hostPort: string;
  requiredEnvFiles: string[];
}

export const imageTemplateOptions: ImageTemplateOption[] = [
  { value: "etcd", label: "etcd 3.6（单节点）", image: "gcr.io/etcd-development/etcd:v3.6.14", hostPort: "2379", requiredEnvFiles: ["compose.env", "etcd.env"] },
  { value: "redis", label: "Redis 7", image: "docker.io/library/redis:7-alpine", hostPort: "6379", requiredEnvFiles: ["compose.env", "redis.env"] },
  { value: "valkey", label: "Valkey 9", image: "docker.io/valkey/valkey:9-alpine", hostPort: "6379", requiredEnvFiles: ["compose.env", "valkey.env"] },
  { value: "postgres", label: "PostgreSQL 18", image: "docker.io/library/postgres:18-alpine", hostPort: "5432", requiredEnvFiles: ["compose.env", "postgres.env"] },
];

export function imageTemplateOption(template: ImageTemplate): ImageTemplateOption {
  return imageTemplateOptions.find((item) => item.value === template) ?? imageTemplateOptions[0];
}

export function imageTemplateLabel(template: ImageTemplate): string {
  return imageTemplateOption(template).label;
}

export function imageTemplateDescription(template: ImageTemplate): string {
  if (template === "etcd") return "Docker Compose 部署单节点 etcd，仅绑定本机回环地址，适用于开发和测试。";
  if (template === "redis") return "Docker Compose 部署 Redis，AOF 持久化、健康检查与应用配置只读挂载。";
  if (template === "valkey") return "Docker Compose 部署 Valkey 9，AOF 持久化、健康检查与应用配置只读挂载。";
  return "Docker Compose 部署 PostgreSQL，数据卷持久化、健康检查与应用配置只读挂载。";
}

export function imageTemplateRequiredEnvFiles(template: ImageTemplate): string[] {
  return imageTemplateOption(template).requiredEnvFiles;
}

export function hasRequiredImageEnvFiles(template: ImageTemplate, envFiles: string[]): boolean {
  return imageTemplateRequiredEnvFiles(template).every((file) => envFiles.includes(file));
}

export function isSafeImageReference(value: string): boolean {
  const image = value.trim();
  return image.length > 0
    && image.length <= 512
    && !/^https?:\/\//.test(image)
    && /^[A-Za-z0-9][A-Za-z0-9._:@/+[\]-]*$/.test(image);
}
