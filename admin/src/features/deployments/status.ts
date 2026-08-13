const labels: Record<string, string> = {
  queued: "排队中",
  running: "运行中",
  canceling: "取消中",
  succeeded: "成功",
  failed: "失败",
  canceled: "已取消",
  interrupted: "执行中断",
};

export function deploymentStatusLabel(status: string) {
  return labels[status] ?? status;
}

export function deploymentStatusTone(status: string) {
  if (status === "succeeded") return "online";
  if (status === "queued" || status === "running" || status === "canceling") return "pending";
  return "disabled";
}

export function isTerminalDeployment(status: string) {
  return ["succeeded", "failed", "canceled", "interrupted"].includes(status);
}

export function formatDeploymentDuration(start: string, end?: string | null) {
  const startMs = Date.parse(start);
  const endMs = end ? Date.parse(end) : Number.NaN;
  if (Number.isNaN(startMs) || Number.isNaN(endMs)) return "-";
  const seconds = Math.max(0, Math.round((endMs - startMs) / 1000));
  if (seconds < 60) return `${seconds} 秒`;
  const minutes = Math.floor(seconds / 60);
  const rest = seconds % 60;
  return rest === 0 ? `${minutes} 分` : `${minutes} 分 ${rest} 秒`;
}
