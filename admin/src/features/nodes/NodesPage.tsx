import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Plus, Server } from "lucide-react";
import { useState, type FormEvent } from "react";
import { Link } from "react-router-dom";
import type { AgentEnrollmentResponse } from "../../api/generated/models/AgentEnrollmentResponse";
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
import { nodesApi } from "./api";

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
  const [creating, setCreating] = useState(false);
  const [name, setName] = useState("");
  const [environment, setEnvironment] = useState("dev");
  const [environmentFilter, setEnvironmentFilter] = useState(initialEnvironmentFilter);
  const [enrollment, setEnrollment] = useState<AgentEnrollmentResponse | null>(null);
  const nodes = useCursorCollection(
    ["nodes"],
    (after) => nodesApi.nodesList({ limit: 50, after: after ?? undefined }),
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
    <div className="workspace-heading"><div><h2>节点</h2><p>{isAdministrator ? "创建服务器节点，安装协同程序后即可接收部署任务。" : "查看已授权应用关联的节点与在线状态。"}</p></div>{isAdministrator ? <Button tone="primary" onClick={() => setCreating(true)}><Plus aria-hidden="true" />创建节点</Button> : null}</div>
    {creating ? <form className="inline-form" onSubmit={(event) => void submit(event)}>
      <Field label="节点名称"><TextInput autoFocus required minLength={1} maxLength={80} disabled={create.isPending} value={name} onChange={(event) => setName(event.target.value)} placeholder="例如：生产节点 01" /></Field>
      <Field label="环境"><Select required disabled={create.isPending} value={environment} onChange={(event) => setEnvironment(event.target.value)}>{AGENT_ENVIRONMENTS.map((item) => <option key={item.value} value={item.value}>{item.label}</option>)}</Select></Field>
      {create.error ? <ApiErrorNotice error={toNotice(create.error)} /> : null}
      <div className="form-actions"><Button type="button" disabled={create.isPending} onClick={() => { setCreating(false); setName(""); setEnvironment("dev"); }}>取消</Button><Button tone="primary" disabled={create.isPending}>{create.isPending ? "正在创建..." : "创建并生成安装命令"}</Button></div>
    </form> : null}
    {enrollment ? <section className="agent-command" aria-live="polite"><div className="section-heading"><div><h3>节点安装命令</h3><p>{enrollment.agent.name} 当前离线。请在 {new Date(enrollment.enrollmentExpiresAt).toLocaleString("zh-CN")} 前到目标 Linux 服务器执行一次性命令。</p></div><Button onClick={() => setEnrollment(null)}>关闭</Button></div><ClipboardFallback value={enrollment.installCommand} label="复制命令" failure="自动复制失败，请选中完整命令后手动复制。" /></section> : null}
    {isAdministrator ? <div className="filter-bar"><label>筛选环境<Select value={environmentFilter} onChange={(event) => {
      const value = event.target.value;
      setEnvironmentFilter(value);
      try {
        window.localStorage.setItem(ENVIRONMENT_FILTER_STORAGE_KEY, value);
      } catch {
        // 无法持久化时仍保留当前页面内的选择。
      }
    }}><option value="all">全部环境</option>{AGENT_ENVIRONMENTS.map((item) => <option key={item.value} value={item.value}>{item.label}</option>)}</Select></label></div> : null}
    {nodes.isLoading || (isAdministrator && agents.isLoading) ? <PageState kind="loading" /> : nodes.isError ? <div className="state-with-action"><ApiErrorNotice error={toNotice(nodes.error)} /><Button onClick={() => void nodes.refetch()}>重试</Button></div> : nodes.items.length === 0 ? <PageState kind="empty" /> : visibleNodes.length === 0 ? <p className="filtered-empty">当前环境没有节点。</p> : <><div className="data-table-wrap"><table className="data-table data-table--priority"><thead><tr><th>节点</th><th>环境</th><th>状态</th><th className="table-column--secondary">协同程序</th><th className="table-column--secondary">最后在线</th><th aria-label="操作"></th></tr></thead><tbody>{visibleNodes.map((node) => { const agent = agentByNode.get(node.id); return <tr key={node.id}><td><Server aria-hidden="true" /><span className="table-entity"><strong>{node.name}</strong><small>{agent?.hostname || node.workRoot || "尚未接入"}</small></span></td><td>{agent ? environmentLabel(agent.environment) : "-"}</td><td><span className={`status-badge status-badge--${node.status === "online" ? "online" : "offline"}`}>{node.status === "online" ? "在线" : "离线"}</span>{agent?.revokedAt ? <small>身份已撤销</small> : null}</td><td className="table-column--secondary">{agent ? <code>v{agent.agentVersion || "-"}</code> : <span className="text-muted">未安装</span>}</td><td className="table-column--secondary">{agent?.lastSeenAt ? new Date(agent.lastSeenAt).toLocaleString("zh-CN") : "从未连接"}</td><td><Link className="table-action" to={`/nodes/${node.id}`}>管理</Link></td></tr>; })}</tbody></table></div>{nodes.hasNextPage ? <div className="pagination-actions"><Button disabled={nodes.isFetchingNextPage} onClick={() => void nodes.fetchNextPage()}>{nodes.isFetchingNextPage ? "正在加载..." : "加载更多"}</Button></div> : null}</>}
  </section>;
}

export function statusLabel(status: string) { return status === "online" ? "在线" : "离线"; }
