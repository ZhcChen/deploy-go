export function executionModeLabel(mode: string) {
  if (mode === "two_stage") return "两阶段";
  if (mode === "two_stage_script") return "脚本两阶段";
  if (mode === "image") return "镜像直连";
  return "单脚本";
}

export function privilegedReleaseLabel() {
  return "原生特权 release";
}
