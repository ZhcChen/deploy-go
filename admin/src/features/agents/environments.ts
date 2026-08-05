export const AGENT_ENVIRONMENTS = [
  { value: "dev", label: "开发环境" },
  { value: "test", label: "测试环境" },
  { value: "staging", label: "预发布环境" },
  { value: "prod", label: "生产环境" },
] as const;

export function environmentLabel(value: string) {
  return AGENT_ENVIRONMENTS.find((item) => item.value === value)?.label ?? value;
}
