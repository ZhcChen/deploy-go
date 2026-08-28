import { describe, expect, it } from "vitest";
import { moduleDefaults, schemaDefaults } from "../features/deployments/ParameterEditor";

const schema = {
  type: "object",
  properties: {
    modules: {
      type: "string",
      maxLength: 512,
      "x-options": ["worker", "api", "admin"],
    },
  },
  required: ["modules"],
  additionalProperties: false,
};

describe("参数 Schema 默认模块选择", () => {
  it("未配置默认选择时保留默认全选行为", () => {
    expect(moduleDefaults(schema)).toEqual(["worker", "api", "admin"]);
    expect(schemaDefaults(schema).modules).toBe("worker,api,admin");
  });

  it("按 x-default-selected 初始化并保持 x-options 顺序", () => {
    const configured = {
      ...schema,
      properties: {
        modules: {
          ...schema.properties.modules,
          "x-default-selected": ["admin", "worker"],
        },
      },
    };

    expect(moduleDefaults(configured)).toEqual(["worker", "admin"]);
    expect(schemaDefaults(configured).modules).toBe("worker,admin");
  });

  it("空数组表示默认不选模块", () => {
    const empty = {
      ...schema,
      properties: {
        modules: {
          ...schema.properties.modules,
          "x-default-selected": [],
        },
      },
    };

    expect(moduleDefaults(empty)).toEqual([]);
    expect(schemaDefaults(empty).modules).toBe("");
  });
});
