import postgresCompose from "../../../../examples/templates/postgres/compose.yaml?raw";
import postgresComposeEnv from "../../../../examples/templates/postgres/compose.env.example?raw";
import postgresConfig from "../../../../examples/templates/postgres/config/postgresql.conf?raw";
import postgresReadme from "../../../../examples/templates/postgres/README.md?raw";
import postgresSchema from "../../../../examples/templates/postgres/parameter-schema.json?raw";
import postgresServiceEnv from "../../../../examples/templates/postgres/postgres.env.example?raw";
import redisCompose from "../../../../examples/templates/redis/compose.yaml?raw";
import redisComposeEnv from "../../../../examples/templates/redis/compose.env.example?raw";
import redisConfig from "../../../../examples/templates/redis/config/redis.conf?raw";
import redisReadme from "../../../../examples/templates/redis/README.md?raw";
import redisSchema from "../../../../examples/templates/redis/parameter-schema.json?raw";
import redisServiceEnv from "../../../../examples/templates/redis/redis.env.example?raw";

export interface TemplateFile {
  path: string;
  label: string;
  content: string;
}

export interface ApplicationTemplate {
  id: string;
  name: string;
  summary: string;
  files: TemplateFile[];
}

export const applicationTemplates: ApplicationTemplate[] = [
  {
    id: "postgres",
    name: "PostgreSQL 18",
    summary: "Docker Compose 部署 PostgreSQL，数据卷持久化、健康检查与应用配置只读挂载。",
    files: [
      { path: "README.md", label: "说明", content: postgresReadme },
      { path: "compose.yaml", label: "Compose 编排", content: postgresCompose },
      { path: "compose.env.example", label: "Compose Env 字段", content: postgresComposeEnv },
      { path: "postgres.env.example", label: "服务 Env 字段", content: postgresServiceEnv },
      { path: "config/postgresql.conf", label: "应用配置", content: postgresConfig },
      { path: "parameter-schema.json", label: "参数 Schema", content: postgresSchema },
    ],
  },
  {
    id: "redis",
    name: "Redis 7",
    summary: "Docker Compose 部署 Redis，AOF 持久化、健康检查与应用配置只读挂载。",
    files: [
      { path: "README.md", label: "说明", content: redisReadme },
      { path: "compose.yaml", label: "Compose 编排", content: redisCompose },
      { path: "compose.env.example", label: "Compose Env 字段", content: redisComposeEnv },
      { path: "redis.env.example", label: "服务 Env 字段", content: redisServiceEnv },
      { path: "config/redis.conf", label: "应用配置", content: redisConfig },
      { path: "parameter-schema.json", label: "参数 Schema", content: redisSchema },
    ],
  },
];
