import type { DeploymentEventResponse } from "../../api/generated/models/DeploymentEventResponse";
import type { DeploymentResponse } from "../../api/generated/models/DeploymentResponse";

export type FlowStatus = "idle" | "running" | "succeeded" | "failed";
export type FlowStageId = "preflight" | "prepare" | "release" | "verification";

export interface FlowItem {
  id: string;
  label: string;
  status: FlowStatus;
  statusLabel: string;
}

export interface FlowStage extends FlowItem {
  stage: FlowStageId;
  logStage: "prepare" | "release";
  items: FlowItem[];
}

const stageLabels: Record<FlowStageId, string> = {
  preflight: "预检",
  prepare: "准备 prepare",
  release: "发布 release",
  verification: "验证",
};

export function buildDeploymentFlow(deployment: DeploymentResponse, events: DeploymentEventResponse[]): FlowStage[] {
  const definitions: Array<{ stage: FlowStageId; matches(event: DeploymentEventResponse): boolean; logStage: "prepare" | "release" }> = [
    { stage: "preflight", logStage: "prepare", matches: (event) => event.eventName.startsWith("deploy.preflight.") },
    { stage: "prepare", logStage: "prepare", matches: (event) => event.stage === "prepare" && !event.eventName.startsWith("deploy.preflight.") },
    { stage: "release", logStage: "release", matches: (event) => event.stage === "release" && !event.eventName.startsWith("deploy.preflight.") && !event.eventName.startsWith("deploy.verification.") },
    { stage: "verification", logStage: "release", matches: (event) => event.eventName.startsWith("deploy.verification.") },
  ];

  const stages = definitions.map(({ stage, matches, logStage }) => {
    const matching = events.filter((event) => isRecognizedFlowEvent(event.eventName) && matches(event));
    const fallback = stage === "prepare" || stage === "release"
      ? deployment.stageTasks.find((task) => task.stage === stage)?.status
      : undefined;
    const status = aggregateStatus(matching, fallback);
    return {
      id: stage,
      stage,
      logStage,
      label: stageLabels[stage],
      status,
      statusLabel: statusLabel(status),
      items: buildItems(matching),
    } satisfies FlowStage;
  });

  if (["failed", "canceled", "interrupted"].includes(deployment.status) && !stages.some((stage) => stage.status === "failed")) {
    const current = [...stages].reverse().find((stage) => stage.status === "running" || stage.status === "idle") ?? stages[0];
    current.status = "failed";
    current.statusLabel = deployment.status === "canceled" ? "已取消" : deployment.status === "interrupted" ? "执行中断" : "失败";
  }
  return stages;
}

function isRecognizedFlowEvent(eventName: string) {
  return /^deploy\.(preflight|module|step|verification)\.(started|succeeded|failed)$/.test(eventName);
}

function aggregateStatus(events: DeploymentEventResponse[], fallback?: string): FlowStatus {
  if (events.some((event) => event.status === "failed" || event.eventName.endsWith(".failed"))) return "failed";
  const lifecycle = new Map<string, "started" | "succeeded">();
  for (const event of events) {
    const key = event.stepId ?? event.module ?? event.eventName.replace(/\.(started|succeeded)$/, "");
    if (event.status === "running" || event.eventName.endsWith(".started")) lifecycle.set(key, "started");
    if (event.status === "succeeded" || event.eventName.endsWith(".succeeded")) lifecycle.set(key, "succeeded");
  }
  if ([...lifecycle.values()].some((value) => value === "started")) return "running";
  if ([...lifecycle.values()].some((value) => value === "succeeded")) return "succeeded";
  if (fallback === "succeeded") return "succeeded";
  if (["queued", "delivered", "accepted", "running"].includes(fallback ?? "")) return "running";
  if (["failed", "canceled", "interrupted", "expired"].includes(fallback ?? "")) return "failed";
  return "idle";
}

function buildItems(events: DeploymentEventResponse[]): FlowItem[] {
  const grouped = new Map<string, DeploymentEventResponse[]>();
  for (const event of events) {
    const id = event.stepId ?? event.module ?? event.eventName;
    const existing = grouped.get(id) ?? [];
    existing.push(event);
    grouped.set(id, existing);
  }
  return [...grouped.entries()].map(([id, entries]) => {
    const last = entries.at(-1)!;
    const status = aggregateStatus(entries);
    return {
      id,
      label: last.step ?? last.moduleName ?? last.module ?? humanizeEvent(last.eventName),
      status,
      statusLabel: statusLabel(status),
    };
  });
}

function humanizeEvent(eventName: string) {
  if (eventName.includes("preflight")) return "部署预检";
  if (eventName.includes("verification")) return "健康验证";
  return eventName;
}

export function statusLabel(status: FlowStatus) {
  return { idle: "未执行", running: "执行中", succeeded: "成功", failed: "失败" }[status];
}
