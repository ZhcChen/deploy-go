import { describe, expect, it } from "vitest";
import type { DeploymentEventResponse } from "../api/generated/models/DeploymentEventResponse";
import type { DeploymentResponse } from "../api/generated/models/DeploymentResponse";
import { buildDeploymentFlow } from "../features/deployments/deployment-flow";

const baseDeployment = {
  id: "deployment-1",
  status: "running",
  stageTasks: [],
  protocolComplete: true,
} as unknown as DeploymentResponse;

function event(id: string, eventName: string, stage: string, fields: Partial<DeploymentEventResponse> = {}): DeploymentEventResponse {
  return { id, eventName, stage, createdAt: `2026-08-07T00:00:0${id}Z`, ...fields };
}

describe("部署流程状态聚合", () => {
  it("所有已开始步骤闭合后标记成功", () => {
    const stages = buildDeploymentFlow(baseDeployment, [
      event("1", "deploy.step.started", "prepare", { stepId: "api.build", step: "构建 API" }),
      event("2", "deploy.step.succeeded", "prepare", { stepId: "api.build", step: "构建 API" }),
    ]);
    expect(stages.find((stage) => stage.stage === "prepare")?.status).toBe("succeeded");
  });

  it("存在尚未闭合的步骤时保持执行中", () => {
    const stages = buildDeploymentFlow(baseDeployment, [
      event("1", "deploy.step.succeeded", "prepare", { stepId: "worker.build" }),
      event("2", "deploy.step.started", "prepare", { stepId: "api.build" }),
    ]);
    expect(stages.find((stage) => stage.stage === "prepare")?.status).toBe("running");
  });

  it("失败事件优先于成功事件", () => {
    const stages = buildDeploymentFlow(baseDeployment, [
      event("1", "deploy.step.succeeded", "release", { stepId: "worker.release" }),
      event("2", "deploy.step.failed", "release", { stepId: "api.release" }),
    ]);
    expect(stages.find((stage) => stage.stage === "release")?.status).toBe("failed");
  });

  it("取消且没有明确失败事件时保留取消文案并使用失败视觉", () => {
    const stages = buildDeploymentFlow({ ...baseDeployment, status: "canceled" }, []);
    expect(stages.some((stage) => stage.status === "failed" && stage.statusLabel === "已取消")).toBe(true);
  });

  it("忽略未知协议事件且不生成虚假节点", () => {
    const stages = buildDeploymentFlow(baseDeployment, [event("1", "deploy.future.started", "prepare", { stepId: "future" })]);
    const prepare = stages.find((stage) => stage.stage === "prepare");
    expect(prepare?.status).toBe("idle");
    expect(prepare?.items).toEqual([]);
  });
});
