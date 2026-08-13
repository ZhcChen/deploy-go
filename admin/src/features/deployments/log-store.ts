import type { DeploymentLogResponse } from "../../api/generated/models/DeploymentLogResponse";

export const LOG_WINDOW_SIZE = 1000;
export type DeploymentLogKind = "output" | "progress" | "diagnostic" | "error";

const markerPrefix = "DEPLOY_GO_EVENT ";
const progressEvents = new Set([
  "deploy.preflight.started",
  "deploy.preflight.succeeded",
  "deploy.module.started",
  "deploy.module.succeeded",
  "deploy.step.started",
  "deploy.step.succeeded",
  "deploy.verification.started",
  "deploy.verification.succeeded",
]);
const failedEvents = new Set([
  "deploy.preflight.failed",
  "deploy.module.failed",
  "deploy.step.failed",
  "deploy.verification.failed",
]);

export function classifyDeploymentLog(log: Pick<DeploymentLogResponse, "stream" | "content">): DeploymentLogKind {
  let hasProgressEvent = false;
  for (const line of log.content.split("\n")) {
    if (!line.startsWith(markerPrefix)) continue;
    try {
      const event = (JSON.parse(line.slice(markerPrefix.length)) as { event?: unknown }).event;
      if (typeof event !== "string") continue;
      if (failedEvents.has(event)) return "error";
      if (progressEvents.has(event)) hasProgressEvent = true;
    } catch {
      // 无效 marker 保持原始流分类，协议诊断由 Agent 负责。
    }
  }
  if (hasProgressEvent) return "progress";
  return log.stream === "stderr" ? "diagnostic" : "output";
}

export function appendDeploymentLog(logs: DeploymentLogResponse[], incoming: DeploymentLogResponse, limit = LOG_WINDOW_SIZE) {
  if (logs.some((log) => log.sequence === incoming.sequence)) return logs;
  const next = [...logs, { ...incoming, content: sanitizeLogText(incoming.content) }]
    .sort((left, right) => left.sequence - right.sequence);
  return next.length > limit ? next.slice(next.length - limit) : next;
}

export function formatDeploymentLogTime(value: string) {
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return "";
  return date.toLocaleString("zh-CN", {
    hour12: false,
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
  });
}

export function sanitizeLogText(value: string) {
  return Array.from(value, (character) => {
    const code = character.codePointAt(0) ?? 0;
    const unsafeControl = code <= 8 || code === 11 || code === 12 || (code >= 14 && code <= 31) || (code >= 127 && code <= 159);
    const unsafeDirection = (code >= 0x202a && code <= 0x202e) || (code >= 0x2066 && code <= 0x2069);
    return unsafeControl || unsafeDirection ? "�" : character;
  }).join("");
}

const stageLabels: Record<string, string> = {
  prepare: "准备阶段（prepare）",
  release: "发布阶段（release）",
  legacy: "脚本阶段",
};

export function formatDeploymentLogs(logs: DeploymentLogResponse[]) {
  let previousStage: string | undefined;
  const lines: string[] = [];
  for (const log of logs) {
    const stage = log.stage ?? "legacy";
    if (stage !== previousStage) {
      if (lines.length > 0) lines.push("");
      lines.push(stageLabels[stage] ?? stage);
      previousStage = stage;
    }
    lines.push(`${formatDeploymentLogTime(log.createdAt)}\t${log.sequence}\t${log.stream}\t${log.content}${log.truncated ? " [已截断]" : ""}`);
  }
  return lines.join("\n");
}
