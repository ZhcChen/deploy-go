import { applicationTemplates, type ApplicationTemplate, type TemplateFile } from "./applicationTemplates";

export interface TemplateWizardDefaults {
  appName: string;
  slugSuggestion: string;
  description: string;
  composeEnvFileName: string;
  serviceEnvFileName: string;
  verificationConfig: Record<string, unknown>;
}

const defaultsByTemplate: Record<string, TemplateWizardDefaults> = {
  postgres: {
    appName: "PostgreSQL 18",
    slugSuggestion: "postgres",
    description: "Docker Compose 部署 PostgreSQL，数据卷持久化、健康检查与应用配置只读挂载。",
    composeEnvFileName: "compose.env",
    serviceEnvFileName: "postgres.env",
    verificationConfig: { type: "tcp", port: 5432, timeout_ms: 5000 },
  },
  redis: {
    appName: "Redis 7",
    slugSuggestion: "redis",
    description: "Docker Compose 部署 Redis，AOF 持久化、健康检查与应用配置只读挂载。",
    composeEnvFileName: "compose.env",
    serviceEnvFileName: "redis.env",
    verificationConfig: { type: "tcp", port: 6379, timeout_ms: 5000 },
  },
};

export function findTemplate(id: string): ApplicationTemplate | undefined {
  return applicationTemplates.find((template) => template.id === id);
}

export function templateDefaults(template: ApplicationTemplate): TemplateWizardDefaults {
  return defaultsByTemplate[template.id] ?? {
    appName: template.name,
    slugSuggestion: template.id,
    description: template.summary,
    composeEnvFileName: "compose.env",
    serviceEnvFileName: "service.env",
    verificationConfig: { type: "tcp", port: 5432, timeout_ms: 5000 },
  };
}

export function templateParameterSchema(template: ApplicationTemplate): unknown {
  const file = template.files.find((item) => item.path === "parameter-schema.json");
  if (!file) return { type: "object", properties: {}, required: [], additionalProperties: false };
  try {
    return JSON.parse(file.content) as unknown;
  } catch {
    return { type: "object", properties: {}, required: [], additionalProperties: false };
  }
}

export function templateEnvExamples(template: ApplicationTemplate) {
  const defaults = templateDefaults(template);
  const file = (path: string) => template.files.find((item) => item.path === path)?.content ?? "";
  return {
    composeEnv: file(`${defaults.composeEnvFileName}.example`),
    serviceEnv: file(`${defaults.serviceEnvFileName}.example`),
    localEnv: file(".env.example"),
  };
}

export function slugify(name: string, fallback: string): string {
  const slug = name
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-+|-+$/g, "")
    .slice(0, 64);
  return slug.length >= 3 ? slug : fallback;
}

export function defaultScriptPath(workRoot: string | null | undefined, slug: string): string {
  const base = (workRoot ?? "/srv/apps").replace(/\/+$/, "");
  return `${base}/${slug}/placeholder`;
}

export function templateDownloadName(template: ApplicationTemplate, file: TemplateFile): string {
  return `deploy-go-${template.id}-${file.path.replaceAll("/", "-")}`;
}

export function downloadTemplateFile(template: ApplicationTemplate, file: TemplateFile): void {
  const blob = new Blob([file.content], { type: "text/plain;charset=utf-8" });
  const url = URL.createObjectURL(blob);
  const anchor = document.createElement("a");
  anchor.href = url;
  anchor.download = templateDownloadName(template, file);
  anchor.click();
  URL.revokeObjectURL(url);
}
