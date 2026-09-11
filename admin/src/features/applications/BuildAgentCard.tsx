import { Hammer } from "lucide-react";
import type { AgentResponse } from "../../api/generated/models/AgentResponse";
import { environmentLabel } from "../agents/environments";

export function BuildAgentCard({ name, agentId, agent }: { name?: string | null; agentId: string; agent?: AgentResponse }) {
  const online = agent?.status === "online";
  const statusClass = agent ? (online ? "online" : "offline") : "unknown";
  const statusText = agent ? (online ? "在线" : "离线") : "已配置";
  return <article className="node-card build-agent-card">
    <div className="node-card__head">
      <span className="node-card__icon" aria-hidden="true"><Hammer /></span>
      <div className="node-card__identity"><h3>{name || agentId}</h3><p title={agentId}>构建节点 · <code>{agentId}</code></p></div>
      <span className={`node-card__status node-card__status--${statusClass}`}><span aria-hidden="true" />{statusText}</span>
    </div>
    {agent ? <div className="node-card__meta">
      <span className="environment-badge">{environmentLabel(agent.environment)}</span>
      <span className="node-card__agent"><code>v{agent.agentVersion || "-"}</code><code>协议 v{agent.protocolVersion ?? "-"}</code></span>
    </div> : null}
    {agent?.lastSeenAt ? <div className="node-card__foot"><span>最后在线 · {new Date(agent.lastSeenAt).toLocaleString("zh-CN")}</span></div> : null}
  </article>;
}
