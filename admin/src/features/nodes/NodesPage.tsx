import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Plus, Server } from "lucide-react";
import { useState, type FormEvent } from "react";
import { Link } from "react-router-dom";
import type { AgentEnrollmentResponse } from "../../api/generated/models/AgentEnrollmentResponse";
import type { AgentResponse } from "../../api/generated/models/AgentResponse";
import type { NodeResponse } from "../../api/generated/models/NodeResponse";
import { Button } from "../../components/Button";
import { Field, Select, TextInput } from "../../components/form";
import { PageState } from "../../components/PageState";
import { agentsApi } from "../agents/api";
import { AGENT_ENVIRONMENTS, environmentLabel } from "../agents/environments";
import { useAuth } from "../auth/AuthContext";
import { ClipboardFallback } from "../shared/ClipboardFallback";
import { toNotice } from "../shared/toNotice";
import { ApiErrorNotice } from "../errors/ApiErrorNotice";
import { useCursorCollection } from "../shared/useCursorCollection";
import { LIST_PAGE_SIZE } from "../shared/pagination";
import { nodesApi } from "./api";
import { formatBytesPair, formatPercent, NodeMetric } from "./NodeMetric";
import { useNodeStatusSocket } from "./useNodeStatusSocket";

const ENVIRONMENT_FILTER_STORAGE_KEY = "deploy-go.nodes.environment-filter";
const NODE_STATUS_REFRESH_INTERVAL_MS = 2_000;

function initialEnvironmentFilter() {
  try {
    const stored = window.localStorage.getItem(ENVIRONMENT_FILTER_STORAGE_KEY);
    if (stored === "all") return stored;
    if (stored && AGENT_ENVIRONMENTS.some((item) => item.value === stored)) return stored;
  } catch {
    // 严格的浏览器策略可能禁用 localStorage。
  }
  return "test";
}

export function NodesPage() {
  const auth = useAuth();
  const isAdministrator = auth.user?.identity === "administrator";
  const queryClient = useQueryClient();
  useNodeStatusSocket(queryClient, auth.csrfToken);
  const [creating, setCreating] = useState(false);
  const [name, setName] = useState("");
  const [environment, setEnvironment] = useState("dev");
  const [environmentFilter, setEnvironmentFilter] = useState(initialEnvironmentFilter);
  const [archived, setArchived] = useState(false);
  const [enrollment, setEnrollment] = useState<AgentEnrollmentResponse | null>(null);
  const nodes = useCursorCollection(
    ["nodes", { archived }],
    (after) => nodesApi.nodesList({ limit: LIST_PAGE_SIZE, after: after ?? undefined, archived: archived ? true : undefined }),
    { intervalMs: NODE_STATUS_REFRESH_INTERVAL_MS },
  );
  const agents = useQuery({
    queryKey: ["agents", "node-links"],
    queryFn: () => agentsApi.agentsList({ limit: 200 }),
    enabled: isAdministrator,
    refetchInterval: NODE_STATUS_REFRESH_INTERVAL_MS,
    refetchIntervalInBackground: false,
    refetchOnWindowFocus: true,
  });
  const agentByNode = new Map(agents.data?.items.map((agent) => [agent.nodeId, agent]));
  const visibleNodes = !isAdministrator || environmentFilter === "all"
    ? nodes.items
    : nodes.items.filter((node) => agentByNode.get(node.id)?.environment === environmentFilter);
  const create = useMutation({
    mutationFn: async () => {
      if (!auth.csrfToken) throw new Error("缺少 CSRF token");
      return agentsApi.agentsCreate({ xCSRFToken: auth.csrfToken, createAgentRequest: { name: name.trim(), environment } });
    },
    onSuccess: async (result) => {
      setEnrollment(result);
      setName("");
      setEnvironment("dev");
      setCreating(false);
      await Promise.all([
        queryClient.invalidateQueries({ queryKey: ["agents"] }),
        queryClient.invalidateQueries({ queryKey: ["nodes"] }),
      ]);
    },
  });

  async function submit(event: FormEvent) {
    event.preventDefault();
    await create.mutateAsync().catch(() => undefined);
  }

  return <section className="workspace">
    <div className="workspace-heading nodes-heading"><div><h2>节点</h2><p>{isAdministrator ? "创建服务器节点，安装协同程序后即可接收部署任务。" : "查看已授权应用关联的节点与在线状态。"}</p></div>{isAdministrator ? <NodeUpgradeOverview agents={agents.data?.items ?? []} nodes={nodes.items} /> : null}{isAdministrator ? <Button tone="primary" onClick={() => setCreating(true)}><Plus aria-hidden="true" />创建节点</Button> : null}</div>
    {creating ? <form className="inline-form" onSubmit={(event) => void submit(event)}>
      <Field label="节点名称"><TextInput autoFocus required minLength={1} maxLength={80} disabled={create.isPending} value={name} onChange={(event) => setName(event.target.value)} placeholder="例如：生产节点 01" /></Field>
      <Field label="环境"><Select required disabled={create.isPending} value={environment} onChange={(event) => setEnvironment(event.target.value)}>{AGENT_ENVIRONMENTS.map((item) => <option key={item.value} value={item.value}>{item.label}</option>)}</Select></Field>
      {create.error ? <ApiErrorNotice error={toNotice(create.error)} /> : null}
      <div className="form-actions"><Button type="button" disabled={create.isPending} onClick={() => { setCreating(false); setName(""); setEnvironment("dev"); }}>取消</Button><Button tone="primary" disabled={create.isPending}>{create.isPending ? "正在创建..." : "创建并生成安装命令"}</Button></div>
    </form> : null}
    {enrollment ? <section className="agent-command" aria-live="polite"><div className="section-heading"><div><h3>节点安装命令</h3><p>{enrollment.agent.name} 当前离线。请在 {new Date(enrollment.enrollmentExpiresAt).toLocaleString("zh-CN")} 前到目标 Linux 服务器执行一次性命令。</p></div><Button onClick={() => setEnrollment(null)}>关闭</Button></div><ClipboardFallback value={enrollment.installCommand} label="复制命令" failure="自动复制失败，请选中完整命令后手动复制。" /></section> : null}
    <div className="filter-bar">{isAdministrator ? <label>筛选环境<Select value={environmentFilter} onChange={(event) => {
      const value = event.target.value;
      setEnvironmentFilter(value);
      try {
        window.localStorage.setItem(ENVIRONMENT_FILTER_STORAGE_KEY, value);
      } catch {
        // 无法持久化时仍保留当前页面内的选择。
      }
    }}><option value="all">全部环境</option>{AGENT_ENVIRONMENTS.map((item) => <option key={item.value} value={item.value}>{item.label}</option>)}</Select></label> : null}<label>节点状态<Select value={archived ? "archived" : "active"} onChange={(event) => setArchived(event.target.value === "archived")}><option value="active">正常</option><option value="archived">已归档</option></Select></label></div>
    {nodes.isLoading || (isAdministrator && agents.isLoading) ? <PageState kind="loading" /> : nodes.isError ? <div className="state-with-action"><ApiErrorNotice error={toNotice(nodes.error)} /><Button onClick={() => void nodes.refetch()}>重试</Button></div> : nodes.items.length === 0 ? <PageState kind="empty" /> : visibleNodes.length === 0 ? <p className="filtered-empty">当前环境没有节点。</p> : <><div className="node-card-grid">{visibleNodes.map((node) => <NodeResourceCard key={node.id} node={node} agent={agentByNode.get(node.id)} />)}</div>{nodes.hasNextPage ? <div className="pagination-actions"><Button disabled={nodes.isFetchingNextPage} onClick={() => void nodes.fetchNextPage()}>{nodes.isFetchingNextPage ? "正在加载..." : "加载更多"}</Button></div> : null}</>}
  </section>;
}

export function statusLabel(status: string) { return status === "online" ? "在线" : "离线"; }

export function upgradeStatusLabel(state: string) {
  return {
    blocked_bootstrap_required: "等待人工安装",
    blocked_unsupported_architecture: "架构不支持",
    waiting_upgrade: "等待升级",
    waiting_for_online: "等待上线",
    waiting_for_idle: "等待空闲",
    downloading: "正在下载",
    installing: "正在安装",
    reconnecting: "等待重连",
    failed: "升级失败",
  }[state] ?? state;
}

export function upgradeStatusTone(state: string) {
  return state === "failed" ? "offline" : state.startsWith("blocked_") ? "unknown" : "checking";
}

function NodeResourceCard({ node, agent }: { node: NodeResponse; agent?: AgentResponse }) {
  const telemetry = useQuery({
    queryKey: ["node", node.id, "telemetry"],
    queryFn: ({ signal }) => nodesApi.nodesTelemetry({ id: node.id }, { signal }),
    enabled: node.status === "online" && !node.archivedAt,
    refetchInterval: node.status === "online" ? 10_000 : false,
    refetchIntervalInBackground: false,
  });
  const latest = telemetry.data?.latest ?? null;
  const online = node.status === "online";
  const archived = Boolean(node.archivedAt);
  const statusClass = archived ? "archived" : online ? "online" : "offline";
  const statusText = archived ? "已归档" : online ? "在线" : "离线";
  const host = agent?.hostname || node.workRoot || "尚未接入";

  return (
    <article className={`node-card node-card--${statusClass}`}>
      <Link className="node-card__link" to={`/nodes/${node.id}`} aria-label={`管理节点 ${node.name}`}>
        <div className="node-card__head">
          <span className="node-card__icon" aria-hidden="true"><Server /></span>
          <div className="node-card__identity">
            <h3>{node.name}</h3>
            <p title={host}>{host}</p>
          </div>
          <span className={`node-card__status node-card__status--${statusClass}`}>
            <span aria-hidden="true" />
            {statusText}
          </span>
        </div>
        <div className="node-card__meta">
          <span className="environment-badge">{agent ? environmentLabel(agent.environment) : "-"}</span>
          {agent ? <span className="node-card__agent"><code>v{agent.agentVersion || "-"}</code><code>协议 v{agent.protocolVersion ?? "-"}</code></span> : <span className="node-card__agent">未安装协同程序</span>}
          {agent?.agentUpgrade && agent.agentUpgrade.state !== "latest" ? <span className={`status-badge status-badge--${upgradeStatusTone(agent.agentUpgrade.state)}`}>{upgradeStatusLabel(agent.agentUpgrade.state)}</span> : null}
          {agent?.revokedAt ? <span className="status-badge status-badge--offline">身份已撤销</span> : null}
        </div>
        <div className="node-card__metrics" aria-label="节点资源">
          <NodeMetric label="CPU" value={latest?.cpuUsageRatio} format={formatPercent} />
          <NodeMetric label="内存" value={latest?.memoryUsedBytes} total={latest?.memoryTotalBytes} format={formatBytesPair} />
          <NodeMetric label="工作盘" value={latest?.workRootUsedBytes} total={latest?.workRootTotalBytes} format={formatBytesPair} />
        </div>
        <div className="node-card__foot">
          <span>最后在线 · {agent?.lastSeenAt ? new Date(agent.lastSeenAt).toLocaleString("zh-CN") : "从未连接"}</span>
          <span className="node-card__manage" aria-hidden="true">管理</span>
        </div>
      </Link>
    </article>
  );
}

function NodeUpgradeOverview({ agents, nodes }: { agents: AgentResponse[]; nodes: NodeResponse[] }) {
  const nodeNames = new Map(nodes.map((node) => [node.id, node.name]));
  const latest = agents.filter((agent) => !agent.agentUpgrade || agent.agentUpgrade.state === "latest").length;
  const upgrading = agents.filter((agent) => ["downloading", "installing", "reconnecting"].includes(agent.agentUpgrade?.state ?? "")).length;
  const waiting = agents.filter((agent) => ["blocked_bootstrap_required", "waiting_upgrade", "waiting_for_online", "waiting_for_idle"].includes(agent.agentUpgrade?.state ?? "")).length;
  const failed = agents.filter((agent) => ["failed", "blocked_unsupported_architecture", "upgrade_state_unavailable"].includes(agent.agentUpgrade?.state ?? "")).length;
  const nodeItems = agents
    .map((agent) => ({ agent, name: nodeNames.get(agent.nodeId) ?? agent.name }))
    .sort((left, right) => left.name.localeCompare(right.name, "zh-CN"));

  return <section className="node-upgrade-overview" aria-label="节点升级进度">
    <div className="node-upgrade-overview__summary">
      <span className="node-upgrade-overview__label">Agent 升级</span>
      <span><strong>{agents.length}</strong> 节点</span>
      <span className="node-upgrade-overview__item node-upgrade-overview__item--latest"><i />{latest} 最新</span>
      <span className="node-upgrade-overview__item node-upgrade-overview__item--active"><i />{upgrading} 进行中</span>
      <span className="node-upgrade-overview__item node-upgrade-overview__item--waiting"><i />{waiting} 待处理</span>
      {failed > 0 ? <span className="node-upgrade-overview__item node-upgrade-overview__item--failed"><i />{failed} 异常</span> : null}
    </div>
    {nodeItems.length > 0 ? <div className="node-upgrade-overview__details">
      {nodeItems.map(({ agent, name }) => {
        const state = agent.agentUpgrade?.state ?? "latest";
        return <span className="node-upgrade-overview__detail" key={agent.id} title={`${name}：${upgradeStatusLabel(state)}`}>
        <b>{name}</b>
        <span className={`status-badge status-badge--${upgradeStatusTone(state)}`}>{upgradeStatusLabel(state)}</span>
        {agent.agentUpgrade?.targetVersion ? <code>v{agent.agentUpgrade.targetVersion}</code> : null}
      </span>;
      })}
    </div> : null}
  </section>;
}
