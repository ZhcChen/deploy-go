import type { ImageTemplate } from "../../api/generated/models/ImageTemplate";

export interface ImageTemplateOption {
  value: ImageTemplate;
  label: string;
  image: string;
  hostPort: string;
}

export const imageTemplateOptions: ImageTemplateOption[] = [
  { value: "redis", label: "Redis 7", image: "docker.io/library/redis:7-alpine", hostPort: "6379" },
  { value: "postgres", label: "PostgreSQL 18", image: "docker.io/library/postgres:18-alpine", hostPort: "5432" },
];

export function imageTemplateOption(template: ImageTemplate): ImageTemplateOption {
  return imageTemplateOptions.find((item) => item.value === template) ?? imageTemplateOptions[0];
}

export function imageTemplateLabel(template: ImageTemplate): string {
  return imageTemplateOption(template).label;
}

export function imageTemplateDescription(template: ImageTemplate): string {
  return template === "redis"
    ? "Docker Compose 部署 Redis，AOF 持久化、健康检查与应用配置只读挂载。"
    : "Docker Compose 部署 PostgreSQL，数据卷持久化、健康检查与应用配置只读挂载。";
}

export function isSafeImageReference(value: string): boolean {
  const image = value.trim();
  return image.length > 0
    && image.length <= 512
    && !/^https?:\/\//.test(image)
    && /^[A-Za-z0-9][A-Za-z0-9._:@/+[\]-]*$/.test(image);
}
