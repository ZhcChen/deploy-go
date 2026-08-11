export function executionModeLabel(mode: string) {
  if (mode === "two_stage") return "两阶段";
  if (mode === "image") return "镜像直连";
  return "单脚本";
}

export function privilegedReleaseLabel(enabled: boolean) {
  return enabled ? "原生特权 release" : "launcher 兼容";
}
