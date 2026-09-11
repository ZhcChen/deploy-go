import type { MetricValue } from "../../api/generated/models/MetricValue";

export function NodeMetric({
  label,
  value,
  total,
  format,
}: {
  label: string;
  value?: MetricValue;
  total?: MetricValue;
  format: (value: number, total?: number) => string;
}) {
  const ratio = metricRatio(value, total);
  const text = ratio == null ? "暂无数据" : format(value!.value!, total?.value ?? undefined);
  return (
    <div className="node-card__metric">
      <div><span>{label}</span><strong>{text}</strong></div>
      <div className="node-card__track" aria-hidden="true"><span style={{ width: ratio == null ? "0%" : `${ratio * 100}%` }} /></div>
    </div>
  );
}

export function formatPercent(value: number) {
  return `${(value * 100).toFixed(1)}%`;
}

export function formatBytesPair(value: number, total?: number) {
  return total == null ? formatBytes(value) : `${formatBytes(value)} / ${formatBytes(total)}`;
}

function metricRatio(value?: MetricValue, total?: MetricValue): number | null {
  if (!value || value.status !== "available" || value.value == null || !Number.isFinite(value.value)) return null;
  if (!total) return clamp(value.value);
  if (total.status !== "available" || total.value == null || total.value <= 0) return null;
  return clamp(value.value / total.value);
}

function clamp(value: number) {
  return Math.min(Math.max(value, 0), 1);
}

function formatBytes(value: number) {
  const units = ["B", "KiB", "MiB", "GiB", "TiB"];
  let amount = value;
  let index = 0;
  while (amount >= 1024 && index < units.length - 1) {
    amount /= 1024;
    index += 1;
  }
  return `${amount.toFixed(index ? 1 : 0)} ${units[index]}`;
}
