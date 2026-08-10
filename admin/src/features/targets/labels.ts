export function executionModeLabel(mode: string) {
  return mode === "two_stage" ? "两阶段" : "单脚本";
}

export function privilegedReleaseLabel(enabled: boolean) {
  return enabled ? "原生特权 release" : "launcher 兼容";
}
