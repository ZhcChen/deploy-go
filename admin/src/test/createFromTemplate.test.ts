import { describe, expect, it, vi } from "vitest";
import {
  defaultScriptPath,
  downloadTemplateFile,
  findTemplate,
  slugify,
  templateDefaults,
  templateDownloadName,
  templateEnvExamples,
  templateParameterSchema,
} from "../features/templates/createFromTemplate";
import postgresComposeEnv from "../../../examples/templates/postgres/compose.env.example?raw";
import postgresSchema from "../../../examples/templates/postgres/parameter-schema.json?raw";
import postgresServiceEnv from "../../../examples/templates/postgres/postgres.env.example?raw";
import redisComposeEnv from "../../../examples/templates/redis/compose.env.example?raw";
import redisSchema from "../../../examples/templates/redis/parameter-schema.json?raw";
import redisServiceEnv from "../../../examples/templates/redis/redis.env.example?raw";

describe("模板创建向导辅助函数", () => {
  it("PostgreSQL 的 Env 示例与 raw 模板文件一致", () => {
    const template = findTemplate("postgres");
    expect(template).toBeDefined();
    const examples = templateEnvExamples(template!);
    expect(examples.composeEnv).toBe(postgresComposeEnv);
    expect(examples.serviceEnv).toBe(postgresServiceEnv);
  });

  it("Redis 的 Env 示例与 raw 模板文件一致", () => {
    const template = findTemplate("redis");
    expect(template).toBeDefined();
    const examples = templateEnvExamples(template!);
    expect(examples.composeEnv).toBe(redisComposeEnv);
    expect(examples.serviceEnv).toBe(redisServiceEnv);
  });

  it("模板默认值包含 slug、Env 文件名与 TCP 验证配置", () => {
    const postgres = templateDefaults(findTemplate("postgres")!);
    expect(postgres.slugSuggestion).toBe("postgres");
    expect(postgres.composeEnvFileName).toBe("compose.env");
    expect(postgres.serviceEnvFileName).toBe("postgres.env");
    expect(postgres.verificationConfig).toEqual({ type: "tcp", port: 5432, timeout_ms: 5000 });

    const redis = templateDefaults(findTemplate("redis")!);
    expect(redis.slugSuggestion).toBe("redis");
    expect(redis.serviceEnvFileName).toBe("redis.env");
    expect(redis.verificationConfig).toEqual({ type: "tcp", port: 6379, timeout_ms: 5000 });
  });

  it("参数 Schema 与 raw 模板文件解析结果一致", () => {
    expect(templateParameterSchema(findTemplate("postgres")!)).toEqual(JSON.parse(postgresSchema));
    expect(templateParameterSchema(findTemplate("redis")!)).toEqual(JSON.parse(redisSchema));
  });

  it("slug 建议符合小写连字符与长度约束", () => {
    expect(slugify("PostgreSQL 16", "postgres")).toBe("postgresql-16");
    expect(slugify("  My App!  ", "app")).toBe("my-app");
    expect(slugify("A", "fallback")).toBe("fallback");
    expect(slugify("中文应用", "fallback")).toBe("fallback");
    expect(slugify("x".repeat(100), "fallback")).toHaveLength(64);
    expect(slugify("x".repeat(100), "fallback")).toMatch(/^[a-z0-9](?:[a-z0-9-]*[a-z0-9])?$/);
  });

  it("脚本占位路径基于节点 work_root 与 slug", () => {
    expect(defaultScriptPath("/srv/apps/", "postgres")).toBe("/srv/apps/postgres/placeholder");
    expect(defaultScriptPath(undefined, "redis")).toBe("/srv/apps/redis/placeholder");
    expect(defaultScriptPath("/srv/apps", "redis")).toBe("/srv/apps/redis/placeholder");
  });

  it("模板下载文件名固定且不包含原始文件路径", () => {
    const template = findTemplate("postgres")!;
    const compose = template.files.find((file) => file.path === "compose.yaml")!;
    expect(templateDownloadName(template, compose)).toBe("deploy-go-postgres-compose.yaml");
    expect(templateDownloadName(template, compose)).not.toContain("/");
  });

  it("下载模板文件使用独立 Blob 与固定文件名", () => {
    const template = findTemplate("redis")!;
    const compose = template.files.find((file) => file.path === "compose.yaml")!;
    const createObjectURL = vi.fn(() => "blob:template-fixture");
    const revokeObjectURL = vi.fn();
    const click = vi.fn();
    const anchor = { href: "", download: "", click };
    vi.spyOn(URL, "createObjectURL").mockImplementation(createObjectURL);
    vi.spyOn(URL, "revokeObjectURL").mockImplementation(revokeObjectURL);
    vi.spyOn(document, "createElement").mockReturnValue(anchor as unknown as HTMLAnchorElement);

    downloadTemplateFile(template, compose);

    expect(createObjectURL).toHaveBeenCalledWith(expect.any(Blob));
    expect(click).toHaveBeenCalled();
    expect(anchor.download).toBe("deploy-go-redis-compose.yaml");
    expect(revokeObjectURL).toHaveBeenCalledWith("blob:template-fixture");
  });
});
