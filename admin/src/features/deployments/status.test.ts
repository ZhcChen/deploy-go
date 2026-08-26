import { describe, expect, it } from "vitest";
import { estimateDeploymentProgress, formatDeploymentDuration } from "./status";

describe("部署耗时格式化", () => {
  it("超过一分钟输出分和秒", () => {
    expect(formatDeploymentDuration("2026-08-13T07:57:33.885Z", "2026-08-13T07:58:55.704Z")).toBe("1 分 22 秒");
  });

  it("超过一小时输出小时、分和秒", () => {
    expect(formatDeploymentDuration("2026-08-13T07:00:00Z", "2026-08-13T08:03:15Z")).toBe("1 小时 3 分 15 秒");
  });

  it("整分钟不输出秒", () => {
    expect(formatDeploymentDuration("2026-08-13T07:00:00Z", "2026-08-13T07:03:00Z")).toBe("3 分");
  });

  it("不足一分钟输出秒", () => {
    expect(formatDeploymentDuration("2026-08-13T07:00:00Z", "2026-08-13T07:00:09Z")).toBe("9 秒");
  });

  it("运行中可以根据当前时间计算耗时", () => {
    const now = Date.parse("2026-08-13T07:00:09Z");
    expect(formatDeploymentDuration("2026-08-13T07:00:00Z", null, now)).toBe("9 秒");
  });

  it("缺少结束时间返回占位符", () => {
    expect(formatDeploymentDuration("2026-08-13T07:00:00Z")).toBe("-");
  });

  it("无效时间返回占位符", () => {
    expect(formatDeploymentDuration("invalid", "2026-08-13T07:00:09Z")).toBe("-");
  });

  it("按上一次部署耗时估算进度", () => {
    const now = Date.parse("2026-08-13T07:01:00Z");
    expect(estimateDeploymentProgress(
      { queuedAt: "2026-08-13T07:00:00Z", status: "running", finishedAt: null },
      300,
      now,
    )).toBe("20%");
  });

  it("进度不会超过 100%", () => {
    const now = Date.parse("2026-08-13T07:06:00Z");
    expect(estimateDeploymentProgress(
      { queuedAt: "2026-08-13T07:00:00Z", status: "running", finishedAt: null },
      300,
      now,
    )).toBe("100%");
  });

  it("缺少参考部署耗时返回占位符", () => {
    expect(estimateDeploymentProgress(
      { queuedAt: "2026-08-13T07:00:00Z", status: "running", finishedAt: null },
      null,
    )).toBe("-");
  });

  it("已结束部署显示 100%", () => {
    expect(estimateDeploymentProgress(
      { queuedAt: "2026-08-13T07:00:00Z", status: "succeeded", finishedAt: "2026-08-13T07:05:00Z" },
      300,
    )).toBe("100%");
  });
});
