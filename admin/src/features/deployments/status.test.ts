import { describe, expect, it } from "vitest";
import { formatDeploymentDuration } from "./status";

describe("部署耗时格式化", () => {
  it("超过一分钟输出分和秒", () => {
    expect(formatDeploymentDuration("2026-08-13T07:57:33.885Z", "2026-08-13T07:58:55.704Z")).toBe("1 分 22 秒");
  });

  it("整分钟不输出秒", () => {
    expect(formatDeploymentDuration("2026-08-13T07:00:00Z", "2026-08-13T07:03:00Z")).toBe("3 分");
  });

  it("不足一分钟输出秒", () => {
    expect(formatDeploymentDuration("2026-08-13T07:00:00Z", "2026-08-13T07:00:09Z")).toBe("9 秒");
  });

  it("缺少结束时间返回占位符", () => {
    expect(formatDeploymentDuration("2026-08-13T07:00:00Z")).toBe("-");
  });

  it("无效时间返回占位符", () => {
    expect(formatDeploymentDuration("invalid", "2026-08-13T07:00:09Z")).toBe("-");
  });
});
