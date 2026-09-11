import type { ReactElement } from "react";

export type ApplicationRuntimeBadge = {
  tone: "online" | "offline" | "checking" | "unknown" | "archived";
  label: string;
  detail: string;
};

export type ApplicationRuntimeFields = {
  status: string;
  runtimeState?: string | null;
  runtimeCheckedAt?: string | null;
  lastDeployedAt?: string | null;
  runtimeProbeStatus?: string | null;
  runtimeProbeErrorCode?: string | null;
  runtimeProbeErrorMessage?: string | null;
};

export function applicationRuntimeBadge(app: ApplicationRuntimeFields): ReactElement {
  const badge = applicationRuntimeState(app);
  return <span className={`status-badge status-badge--${badge.tone}`} title={badge.detail}>{badge.label}</span>;
}

export function applicationRuntimeState(app: ApplicationRuntimeFields): ApplicationRuntimeBadge {
  const checkedAt = app.runtimeCheckedAt ?? app.lastDeployedAt;
  const state = app.status === "archived" ? "archived" : app.runtimeState ?? "unknown";
  return runtimeBadge(state, checkedAt, app.runtimeProbeStatus, app.runtimeProbeErrorCode, app.runtimeProbeErrorMessage);
}

function runtimeBadge(state: string, checkedAt?: string | null, runtimeProbeStatus?: string | null, runtimeProbeErrorCode?: string | null, runtimeProbeErrorMessage?: string | null): ApplicationRuntimeBadge {
  const formattedTime = checkedAt ? new Date(checkedAt).toLocaleString("zh-CN") : null;
  const probeReason = [runtimeProbeErrorCode, runtimeProbeErrorMessage].filter(Boolean).join("：");
  switch (state) {
    case "archived":
      return { tone: "archived", label: "已归档", detail: "应用已归档，不检测运行状态。" };
    case "running":
      if (runtimeProbeStatus === "succeeded") {
        return { tone: "online", label: "运行中", detail: formattedTime ? `平台运行探测通过：${formattedTime}` : "平台运行探测通过。" };
      }
      if (runtimeProbeStatus === "failed") {
        return { tone: "online", label: "运行中", detail: `平台运行探测未完成：${probeReason}${formattedTime ? `；最近部署验证通过：${formattedTime}` : ""}` };
      }
      return { tone: "online", label: "运行中", detail: formattedTime ? `最近一次部署验证通过：${formattedTime}` : "最近一次部署验证通过。" };
    case "failed":
      if (runtimeProbeStatus === "failed") {
        return { tone: "offline", label: "异常", detail: formattedTime ? `平台运行探测异常：${probeReason || "服务不可达"}（${formattedTime}）` : `平台运行探测异常：${probeReason || "服务不可达"}` };
      }
      return { tone: "offline", label: "异常", detail: formattedTime ? `最近一次部署失败：${formattedTime}` : "最近一次部署失败。" };
    case "checking":
      if (runtimeProbeStatus === "pending" || runtimeProbeStatus === "running") {
        return { tone: "checking", label: "检测中", detail: "正在探测本地服务运行状态，请稍候。" };
      }
      return { tone: "checking", label: "部署中", detail: "最近一次部署仍在进行，等待验证结果。" };
    default:
      if (runtimeProbeStatus === "failed") {
        return { tone: "unknown", label: "未部署", detail: `真实运行探测未完成：${probeReason || "无可用结果"}` };
      }
      return { tone: "unknown", label: "未部署", detail: "尚未有可用的部署验证结果。" };
  }
}
