import { useMutation, useQueryClient } from "@tanstack/react-query";
import { Bot, Plus } from "lucide-react";
import { useState, type FormEvent } from "react";
import { Link } from "react-router-dom";
import type { AgentEnrollmentResponse } from "../../api/generated/models/AgentEnrollmentResponse";
import { Button } from "../../components/Button";
import { PageState } from "../../components/PageState";
import { useAuth } from "../auth/AuthContext";
import { ClipboardFallback } from "../credentials/ClipboardFallback";
import { toNotice } from "../credentials/CredentialsPage";
import { ApiErrorNotice } from "../errors/ApiErrorNotice";
import { useCursorCollection } from "../shared/useCursorCollection";
import { agentsApi } from "./api";

export function AgentsPage() {
  const auth = useAuth();
  const queryClient = useQueryClient();
  const [creating, setCreating] = useState(false);
  const [name, setName] = useState("");
  const [enrollment, setEnrollment] = useState<AgentEnrollmentResponse | null>(null);
  const agents = useCursorCollection(["agents"], (after) => agentsApi.agentsList({ limit: 50, after: after ?? undefined }));
  const create = useMutation({ mutationFn: async () => {
    if (!auth.csrfToken) throw new Error("缺少 CSRF token");
    return agentsApi.agentsCreate({ xCSRFToken: auth.csrfToken, createAgentRequest: { name: name.trim() } });
  }, onSuccess: async (result) => { setEnrollment(result); setName(""); setCreating(false); await queryClient.invalidateQueries({ queryKey: ["agents"] }); } });

  async function submit(event: FormEvent) { event.preventDefault(); await create.mutateAsync().catch(() => undefined); }

  return <section className="workspace">
    <div className="workspace-heading"><div><h2>Agent</h2><p>创建节点身份并通过一次性命令安装协同程序。</p></div><Button tone="primary" onClick={() => setCreating(true)}><Plus aria-hidden="true" />创建 Agent</Button></div>
    {creating ? <form className="inline-form" onSubmit={(event) => void submit(event)}><label>Agent 名称<input autoFocus required minLength={1} maxLength={80} disabled={create.isPending} value={name} onChange={(event) => setName(event.target.value)} placeholder="例如：生产节点 01" /></label>{create.error ? <ApiErrorNotice error={toNotice(create.error)} /> : null}<div className="form-actions"><Button type="button" disabled={create.isPending} onClick={() => { setCreating(false); setName(""); }}>取消</Button><Button tone="primary" disabled={create.isPending}>{create.isPending ? "正在创建..." : "创建并生成命令"}</Button></div></form> : null}
    {enrollment ? <section className="agent-command" aria-live="polite"><div className="section-heading"><div><h3>安装命令</h3><p>{enrollment.agent.name} 当前离线，命令将在 {new Date(enrollment.enrollmentExpiresAt).toLocaleString("zh-CN")} 过期。</p></div><Button onClick={() => setEnrollment(null)}>关闭</Button></div><ClipboardFallback value={enrollment.installCommand} label="复制命令" failure="自动复制失败，请选中上方完整命令后手动复制。" /></section> : null}
    {agents.isLoading ? <PageState kind="loading" /> : agents.isError ? <div className="state-with-action"><ApiErrorNotice error={toNotice(agents.error)} /><Button onClick={() => void agents.refetch()}>重试</Button></div> : agents.items.length === 0 ? <PageState kind="empty" /> : <><div className="data-table-wrap"><table className="data-table"><thead><tr><th>Agent</th><th>状态</th><th>版本</th><th>最后在线</th><th></th></tr></thead><tbody>{agents.items.map((agent) => <tr key={agent.id}><td><Bot aria-hidden="true" /><strong>{agent.name}</strong><small>{agent.hostname || "从未连接"}</small></td><td><span className={`status-badge status-badge--${agent.status}`}>{agent.status === "online" ? "在线" : "离线"}</span>{agent.revokedAt ? <small>已撤销</small> : null}</td><td><code>{agent.agentVersion || "-"}</code></td><td>{agent.lastSeenAt ? new Date(agent.lastSeenAt).toLocaleString("zh-CN") : "从未连接"}</td><td><Link className="text-link" to={`/agents/${agent.id}`}>管理</Link></td></tr>)}</tbody></table></div>{agents.hasNextPage ? <div className="pagination-actions"><Button disabled={agents.isFetchingNextPage} onClick={() => void agents.fetchNextPage()}>加载更多</Button></div> : null}</>}
  </section>;
}
