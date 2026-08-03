import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { ArrowLeft, RefreshCw, ShieldX } from "lucide-react";
import { useState } from "react";
import { Link, useParams } from "react-router-dom";
import type { AgentInstallCommandResponse } from "../../api/generated/models/AgentInstallCommandResponse";
import { Button } from "../../components/Button";
import { ConfirmDialog } from "../../components/ConfirmDialog";
import { PageState } from "../../components/PageState";
import { useAuth } from "../auth/AuthContext";
import { ClipboardFallback } from "../shared/ClipboardFallback";
import { toNotice } from "../shared/toNotice";
import { ApiErrorNotice } from "../errors/ApiErrorNotice";
import { agentsApi } from "./api";

export function AgentDetailPage() {
  const { id = "" } = useParams();
  const auth = useAuth();
  const queryClient = useQueryClient();
  const [confirm, setConfirm] = useState<"command" | "revoke" | null>(null);
  const [command, setCommand] = useState<AgentInstallCommandResponse | null>(null);
  const detail = useQuery({ queryKey: ["agent", id], queryFn: () => agentsApi.agentsShow({ agentId: id }) });
  const regenerate = useMutation({ mutationFn: async () => {
    if (!auth.csrfToken) throw new Error("缺少 CSRF token");
    return agentsApi.agentsCreateInstallCommand({ agentId: id, xCSRFToken: auth.csrfToken });
  }, onSuccess: (result) => { setCommand(result); setConfirm(null); } });
  const revoke = useMutation({ mutationFn: async () => {
    if (!auth.csrfToken) throw new Error("缺少 CSRF token");
    await agentsApi.agentsRevoke({ agentId: id, xCSRFToken: auth.csrfToken });
  }, onSuccess: async () => {
    setConfirm(null);
    setCommand(null);
    await Promise.all([
      queryClient.invalidateQueries({ queryKey: ["agent", id] }),
      queryClient.invalidateQueries({ queryKey: ["agents"] }),
      queryClient.invalidateQueries({ queryKey: ["node", detail.data?.nodeId] }),
      queryClient.invalidateQueries({ queryKey: ["nodes"] }),
    ]);
  } });

  if (detail.isLoading) return <PageState kind="loading" />;
  if (detail.isError || !detail.data) return <div className="state-with-action"><ApiErrorNotice error={toNotice(detail.error)} /><Button onClick={() => void detail.refetch()}>重试</Button></div>;
  const agent = detail.data;
  return <article className="detail-page"><Link className="back-link" to="/agents"><ArrowLeft aria-hidden="true" />返回 Agent</Link><div className="detail-title"><div><h2>{agent.name}</h2><p>{agent.id}</p></div><span className={`status-badge status-badge--${agent.status}`}>{agent.status === "online" ? "在线" : "离线"}</span></div>
    <section className="detail-section"><h3>运行信息</h3><dl className="definition-grid"><div><dt>关联节点</dt><dd><Link className="text-link" to={`/nodes/${agent.nodeId}`}>{agent.nodeId}</Link></dd></div><div><dt>主机名</dt><dd>{agent.hostname || "从未连接"}</dd></div><div><dt>版本</dt><dd><code>{agent.agentVersion || "-"}</code></dd></div><div><dt>架构</dt><dd>{agent.architecture || "-"}</dd></div><div><dt>最后在线</dt><dd>{agent.lastSeenAt ? new Date(agent.lastSeenAt).toLocaleString("zh-CN") : "从未连接"}</dd></div><div><dt>身份状态</dt><dd>{agent.revokedAt ? "已撤销" : "有效"}</dd></div></dl></section>
    {command ? <section className="agent-command"><div className="section-heading"><div><h3>新的安装命令</h3><p>旧的未使用 enrollment token 已失效；此命令将在 {new Date(command.enrollmentExpiresAt).toLocaleString("zh-CN")} 过期。执行命令后按提示粘贴 token。</p></div><Button onClick={() => setCommand(null)}>关闭</Button></div><ClipboardFallback value={command.enrollmentToken} label="复制 token" /><ClipboardFallback value={command.installCommand} label="复制命令" /></section> : null}
    <section className="detail-section"><div className="section-heading"><div><h3>安装与修复</h3><p>同一 Agent ID 重跑会保留有效凭证；已撤销身份会显式重新绑定。</p></div><Button onClick={() => setConfirm("command")}><RefreshCw aria-hidden="true" />重新生成命令</Button></div>{regenerate.error ? <ApiErrorNotice error={toNotice(regenerate.error)} /> : null}</section>
    <section className="danger-zone"><div><h3>撤销 Agent</h3><p>立即关闭在线连接并撤销全部 token，节点将转为离线。</p></div><Button tone="danger" disabled={Boolean(agent.revokedAt)} onClick={() => setConfirm("revoke")}><ShieldX aria-hidden="true" />{agent.revokedAt ? "已撤销" : "撤销 Agent"}</Button>{revoke.error ? <ApiErrorNotice error={toNotice(revoke.error)} /> : null}</section>
    <ConfirmDialog open={confirm !== null} title={confirm === "revoke" ? `撤销 ${agent.name}？` : "重新生成安装命令？"} message={confirm === "revoke" ? "在线连接会立即关闭，恢复时必须使用新命令重新绑定。" : "此前尚未使用的安装命令将立即失效。"} confirmLabel={confirm === "revoke" ? "确认撤销" : "确认重新生成"} tone={confirm === "revoke" ? "danger" : "primary"} pending={revoke.isPending || regenerate.isPending} onClose={() => setConfirm(null)} onConfirm={() => { if (confirm === "revoke") revoke.mutate(); else regenerate.mutate(); }} />
  </article>;
}
