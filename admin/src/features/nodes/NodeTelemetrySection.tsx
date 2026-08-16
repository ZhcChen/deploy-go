import { useQuery } from "@tanstack/react-query";
import { Activity, Cpu, Database, Gauge, HardDrive, MemoryStick, Network, RefreshCw } from "lucide-react";
import { useEffect, useState } from "react";
import type { HistoryPoint } from "../../api/generated/models/HistoryPoint";
import type { MetricValue } from "../../api/generated/models/MetricValue";
import { Button } from "../../components/Button";
import { ApiErrorNotice } from "../errors/ApiErrorNotice";
import { toNotice } from "../shared/toNotice";
import { nodesApi } from "./api";

export function NodeTelemetrySection({ nodeId }: { nodeId: string }) {
  const visible = usePageVisibility();
  const telemetry = useQuery({
    queryKey: ["node", nodeId, "telemetry"],
    queryFn: ({ signal }) => nodesApi.nodesTelemetry({ id: nodeId }, { signal }),
    refetchInterval: visible ? 30_000 : false,
    refetchIntervalInBackground: false,
  });

  if (telemetry.isLoading) return <section className="detail-section telemetry-section" aria-busy="true"><h3>节点运行状态</h3><p className="muted">正在读取节点遥测...</p></section>;
  if (!telemetry.data) return <section className="detail-section telemetry-section"><div className="section-head"><div><h3>节点运行状态</h3><p>遥测数据暂时不可用。</p></div><Button onClick={() => void telemetry.refetch()}><RefreshCw aria-hidden="true" />重试</Button></div>{telemetry.error ? <ApiErrorNotice error={toNotice(telemetry.error)} /> : null}</section>;

  const data = telemetry.data;
  const latest = data.latest;
  return <section className="detail-section telemetry-section" aria-labelledby="node-telemetry-title">
    <div className="section-head">
      <div><h3 id="node-telemetry-title">节点运行状态</h3><p>{telemetrySummary(data.capability, data.freshness, data.receivedAt)}</p></div>
      <div className="detail-badges"><span className={`status-badge status-badge--${data.connectivity === "online" ? "online" : "offline"}`}>{connectivityLabel(data.connectivity)}</span><span className={`telemetry-freshness telemetry-freshness--${data.freshness}`}>{freshnessLabel(data.freshness)}</span></div>
    </div>
    {telemetry.isError ? <div className="telemetry-refresh-error"><ApiErrorNotice error={toNotice(telemetry.error)} /><Button onClick={() => void telemetry.refetch()}><RefreshCw aria-hidden="true" />重试</Button></div> : null}
    {data.capability !== "supported" ? <TelemetryEmpty reason={data.capabilityReason} /> : !latest ? <TelemetryEmpty reason="waiting_for_sample" /> : <>
      <div className="telemetry-summary" aria-label="当前资源摘要">
        <Metric icon={<Cpu />} label="CPU" metric={latest.cpuUsageRatio} format={formatPercent} />
        <Metric icon={<MemoryStick />} label="内存" metric={latest.memoryUsedBytes} secondary={latest.memoryTotalBytes} format={formatBytesPair} />
        <Metric icon={<HardDrive />} label="工作盘" metric={latest.workRootUsedBytes} secondary={latest.workRootTotalBytes} format={formatBytesPair} />
        <Metric icon={<Gauge />} label="磁盘忙碌" metric={latest.diskBusyRatio} format={formatPercent} />
        <Metric icon={<Database />} label="磁盘读 / 写" metric={latest.diskReadBytesPerSecond} secondary={latest.diskWriteBytesPerSecond} format={formatRatePair} />
        <Metric icon={<Network />} label="网络下行 / 上行" metric={latest.networkReceiveBytesPerSecond} secondary={latest.networkTransmitBytesPerSecond} format={formatRatePair} />
      </div>
      <div className="telemetry-trends" aria-label="最近 24 小时趋势">
        <Trend title="CPU 使用率" points={data.history} values={(point) => point.cpuUsageRatio} format={formatPercent} />
        <Trend title="内存使用" points={data.history} values={(point) => point.memoryUsedBytes} format={formatBytes} />
        <Trend title="磁盘读写" points={data.history} values={(point) => maxValue(point.diskReadBytesPerSecond, point.diskWriteBytesPerSecond)} format={formatRate} />
        <Trend title="网络吞吐" points={data.history} values={(point) => maxValue(point.networkReceiveBytesPerSecond, point.networkTransmitBytesPerSecond)} format={formatRate} />
      </div>
      <GpuSummary status={latest.gpuStatus} value={latest.gpus} />
    </>}
  </section>;
}

function usePageVisibility() {
  const [visible, setVisible] = useState(() => typeof document === "undefined" || document.visibilityState === "visible");
  useEffect(() => { const update = () => setVisible(document.visibilityState === "visible"); document.addEventListener("visibilitychange", update); return () => document.removeEventListener("visibilitychange", update); }, []);
  return visible;
}

function Metric({ icon, label, metric, secondary, format }: { icon: React.ReactNode; label: string; metric: MetricValue; secondary?: MetricValue; format: (value: number, second?: number) => string }) {
  const available = metric.status === "available" && metric.value != null && (!secondary || secondary.status === "available" && secondary.value != null);
  return <div className="telemetry-metric"><span className="telemetry-metric__icon" aria-hidden="true">{icon}</span><div><span>{label}</span><strong>{available ? format(metric.value!, secondary?.value ?? undefined) : metricStatusLabel(metric.status)}</strong></div></div>;
}

function Trend({ title, points, values, format }: { title: string; points: HistoryPoint[]; values: (point: HistoryPoint) => number | null | undefined; format: (value: number) => string }) {
  const samples = points.map((point) => ({ at: point.receivedAt, value: values(point) })).filter((item): item is { at: string; value: number } => typeof item.value === "number" && Number.isFinite(item.value));
  const max = Math.max(...samples.map((item) => item.value), 1); const width = 320; const height = 88;
  const path = samples.map((item, index) => `${index ? "L" : "M"}${samples.length === 1 ? width / 2 : index * width / (samples.length - 1)},${height - item.value / max * (height - 8)}`).join(" ");
  const latest = samples.at(-1)?.value; const average = samples.length ? samples.reduce((sum, item) => sum + item.value, 0) / samples.length : null;
  return <figure className="telemetry-trend"><figcaption><strong>{title}</strong><span>{latest == null ? "暂无有效数据" : `当前 ${format(latest)} · 平均 ${format(average!)}`}</span></figcaption><div className="telemetry-chart">{path ? <svg viewBox={`0 0 ${width} ${height}`} preserveAspectRatio="none" aria-hidden="true"><path d={path} /></svg> : <span>等待有效样本</span>}</div><ol className="visually-hidden">{samples.map((item) => <li key={item.at}>{new Date(item.at).toLocaleString("zh-CN")}：{format(item.value)}</li>)}</ol></figure>;
}

function GpuSummary({ status, value }: { status: string; value: unknown }) {
  const gpus = Array.isArray(value) ? value.filter((item): item is Record<string, unknown> => Boolean(item) && typeof item === "object") : [];
  return <div className="telemetry-gpu"><div><Activity aria-hidden="true" /><h4>GPU</h4><span>{metricStatusLabel(status)}</span></div>{gpus.length ? <ul>{gpus.map((gpu, index) => <li key={String(gpu.index ?? index)}><strong>{String(gpu.model ?? `GPU ${index}`)}</strong><span>{typeof gpu.utilization_percent === "number" ? `${gpu.utilization_percent.toFixed(0)}%` : "-"}</span></li>)}</ul> : null}</div>;
}

function TelemetryEmpty({ reason }: { reason?: string | null }) { const labels: Record<string,string> = { protocol_v11:"当前 Agent 版本不支持遥测，请升级 Agent。", no_agent:"节点尚未绑定 Agent。", revoked:"Agent 身份已撤销。", archived:"Agent 已归档。", not_connected:"等待 Agent 首次连接。", waiting_for_sample:"等待首个遥测样本。" }; return <div className="telemetry-empty"><Activity aria-hidden="true" /><p>{labels[reason ?? ""] ?? "节点遥测暂不可用。"}</p></div>; }
function telemetrySummary(capability: string, freshness: string, receivedAt?: string | null) { if (capability !== "supported") return "当前节点尚不能提供运行遥测。"; if (!receivedAt) return "等待 Agent 上报首个样本。"; return `${freshness === "stale" ? "最后快照" : "最近更新"} ${new Date(receivedAt).toLocaleString("zh-CN")}`; }
function connectivityLabel(value: string) { return ({ online:"在线", offline:"离线", disabled:"已禁用", unknown:"状态未知" } as Record<string,string>)[value] ?? "状态未知"; }
function freshnessLabel(value: string) { return ({ fresh:"数据正常", stale:"数据已过期", empty:"暂无数据" } as Record<string,string>)[value] ?? "暂无数据"; }
function metricStatusLabel(value: string) { return ({ available:"可用", warming_up:"采集预热中", unsupported:"不支持", collection_error:"采集失败" } as Record<string,string>)[value] ?? "不可用"; }
function formatPercent(value: number) { return `${(value * 100).toFixed(1)}%`; }
function formatBytes(value: number) { const units=["B","KiB","MiB","GiB","TiB"]; let amount=value; let index=0; while(amount>=1024&&index<units.length-1){amount/=1024;index+=1;} return `${amount.toFixed(index ? 1 : 0)} ${units[index]}`; }
function formatBytesPair(value: number, total?: number) { return total == null ? formatBytes(value) : `${formatBytes(value)} / ${formatBytes(total)}`; }
function formatRate(value: number) { return `${formatBytes(value)}/s`; }
function formatRatePair(value: number, second?: number) { return second == null ? formatRate(value) : `${formatRate(value)} / ${formatRate(second)}`; }
function maxValue(first?: number | null, second?: number | null) { const values=[first,second].filter((value): value is number => typeof value === "number"); return values.length ? Math.max(...values) : null; }
