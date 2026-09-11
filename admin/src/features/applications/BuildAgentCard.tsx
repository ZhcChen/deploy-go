import { useQuery } from "@tanstack/react-query";
import { Hammer } from "lucide-react";
import { Link } from "react-router-dom";
import type { AgentResponse } from "../../api/generated/models/AgentResponse";
import { environmentLabel } from "../agents/environments";
import { formatBytesPair, formatPercent, NodeMetric } from "../nodes/NodeMetric";
import { nodesApi } from "../nodes/api";

export function BuildAgentCard({ name, agentId, agent }: { name?: string | null; agentId: string; agent?: AgentResponse }) {
  const online = agent?.status === "online";
  const nodeId = agent?.nodeId;
  const statusClass = agent ? (online ? "online" : "offline") : "unknown";
  const statusText = agent ? (online ? "在线" : "离线") : "已配置";
  const telemetry = useQuery({
    queryKey: ["node", nodeId, "telemetry"],
    queryFn: ({ signal }) => nodesApi.nodesTelemetry({ id: nodeId as string }, { signal }),
    enabled: Boolean(nodeId) && online,
    refetchInterval: online ? 10_000 : false,
    refetchIntervalInBackground: false,
  });
  const latest = telemetry.data?.latest ?? null;
  const body = <>
    <div className="node-card__head">
      <span className="node-card__icon" aria-hidden="true"><Hammer /></span>
      <div className="node-card__identity"><h3>{name || agentId}</h3><p title={agentId}>构建节点 · <code>{agentId}</code></p></div>
      <span className={`node-card__status node-card__status--${statusClass}`}><span aria-hidden="true" />{statusText}</span>
    </div>
    {agent ? <div className="node-card__meta">
      <span className="environment-badge">{environmentLabel(agent.environment)}</span>
      <span className="node-card__agent"><code>v{agent.agentVersion || "-"}</code><code>协议 v{agent.protocolVersion ?? "-"}</code></span>
    </div> : null}
    <div className="node-card__metrics" aria-label="节点资源">
      <NodeMetric label="CPU" value={latest?.cpuUsageRatio} format={formatPercent} />
      <NodeMetric label="内存" value={latest?.memoryUsedBytes} total={latest?.memoryTotalBytes} format={formatBytesPair} />
      <NodeMetric label="工作盘" value={latest?.workRootUsedBytes} total={latest?.workRootTotalBytes} format={formatBytesPair} />
    </div>
    <div className="node-card__foot">
      <span>最后在线 · {agent?.lastSeenAt ? new Date(agent.lastSeenAt).toLocaleString("zh-CN") : "从未连接"}</span>
      {nodeId ? <span className="node-card__manage" aria-hidden="true">管理</span> : null}
    </div>
  </>;
  return <article className="node-card">
    {nodeId
      ? <Link className="node-card__link" to={`/nodes/${nodeId}`} aria-label={`管理节点 ${name || agentId}`}>{body}</Link>
      : <div className="node-card__link">{body}</div>}
  </article>;
}
