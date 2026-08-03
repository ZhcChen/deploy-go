import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { ArrowLeft, Bot, CheckCircle2 } from "lucide-react";
import { useState } from "react";
import { Link, useParams } from "react-router-dom";
import type { NodeCheckResponse } from "../../api/generated/models/NodeCheckResponse";
import { Button } from "../../components/Button";
import { PageState } from "../../components/PageState";
import { agentsApi } from "../agents/api";
import { useAuth } from "../auth/AuthContext";
import { nodesApi } from "../credentials/api";
import { toNotice } from "../credentials/CredentialsPage";
import { ApiErrorNotice } from "../errors/ApiErrorNotice";
import { statusLabel } from "./NodesPage";

export function NodeDetailPage() {
  const { id = "" } = useParams();
  const auth = useAuth();
  const isAdministrator = auth.user?.identity === "administrator";
  const queryClient = useQueryClient();
  const detail = useQuery({ queryKey: ["node", id], queryFn: () => nodesApi.nodesShow({ id }) });
  const agents = useQuery({
    queryKey: ["agents", "node-links"],
    queryFn: () => agentsApi.agentsList({ limit: 200 }),
    enabled: isAdministrator,
  });
  const [check, setCheck] = useState<NodeCheckResponse | null>(null);

  function secureContext() { if (!auth.csrfToken) throw new Error("缺少 CSRF token"); return auth.csrfToken; }
  const runCheck = useMutation({
    mutationFn: () => nodesApi.nodesRunCheck({ id, xCSRFToken: secureContext() }),
    onSuccess: (result) => {
      setCheck(result);
      void detail.refetch();
      void queryClient.invalidateQueries({ queryKey: ["nodes"] });
    },
  });

  if (detail.isLoading) return <PageState kind="loading" />;
  if (detail.isError || !detail.data) return <div className="state-with-action"><ApiErrorNotice error={toNotice(detail.error)} /><Link className="button button--default" to="/nodes">返回节点</Link></div>;
  const node = detail.data;
  const agent = agents.data?.items.find((item) => item.nodeId === node.id);
  const online = node.status === "online";

  return <section className="workspace detail-page">
    <Link className="back-link" to="/nodes"><ArrowLeft aria-hidden="true" />返回节点</Link>
    <div className="detail-title"><div><h2>{node.name}</h2><p><code>{node.id}</code></p></div><span className={`status-badge status-badge--${online ? "online" : "offline"}`}>{statusLabel(node.status)}</span></div>
    <dl className="definition-grid">
      <div><dt>工作根目录</dt><dd><code>{node.workRoot || "尚未上报"}</code></dd></div>
      <div><dt>Secrets root</dt><dd><code>{node.secretsRoot || "尚未上报"}</code></dd></div>
      <div><dt>最近检查</dt><dd>{node.checkedAt ? new Date(node.checkedAt).toLocaleString("zh-CN") : "尚未检查"}</dd></div>
    </dl>
    {isAdministrator ? <section className="detail-section">
      <div className="section-head"><div><h3>关联 Agent</h3><p>节点身份和在线状态由 Agent 控制连接维护。</p></div>{agent ? <Link className="button button--default" to={`/agents/${agent.id}`}><Bot aria-hidden="true" />查看 Agent</Link> : <Link className="button button--default" to="/agents"><Bot aria-hidden="true" />管理 Agent</Link>}</div>
      {agents.isError ? <ApiErrorNotice error={toNotice(agents.error)} /> : agent ? <dl className="definition-grid"><div><dt>名称</dt><dd>{agent.name}</dd></div><div><dt>版本</dt><dd>{agent.agentVersion ?? "尚未上报"}</dd></div><div><dt>主机</dt><dd>{agent.hostname ?? "尚未上报"}</dd></div><div><dt>架构</dt><dd>{agent.architecture ?? "尚未上报"}</dd></div></dl> : <p className="muted">该节点尚未关联 Agent。</p>}
    </section> : <section className="detail-section"><h3>节点状态</h3><p>普通用户可查看已授权节点状态；Agent 接入和维护由管理员完成。</p></section>}
    {isAdministrator ? <section className="detail-section">
      <div className="section-head"><div><h3>节点能力检查</h3><p>通过关联 Agent 执行 SystemInspect，检查系统、架构、工作目录和可用磁盘，不执行部署脚本。</p></div><Button disabled={!agent || !online || runCheck.isPending} onClick={() => runCheck.mutate()}>{runCheck.isPending ? "正在检查..." : "执行检查"}</Button></div>
      {!agent ? <p className="muted">关联 Agent 后才能执行检查。</p> : !online ? <p className="muted">Agent 离线，恢复连接后才能执行检查。</p> : null}
      {check ? <CheckResult value={check} /> : null}
      {runCheck.error ? <ApiErrorNotice error={toNotice(runCheck.error)} /> : null}
    </section> : null}
  </section>;
}

function CheckResult({ value }: { value: NodeCheckResponse }) {
  if (value.status === "running") return <div className="check-result"><strong>检查任务已下发</strong><p>Agent 正在执行 SystemInspect，完成后节点信息会自动更新。</p></div>;
  if (value.status !== "succeeded") return <div className="check-result check-result--failed" role="alert"><strong>检查失败：{value.failureCode ?? "unknown"}</strong><p>{value.failureMessage ?? "请核对 Agent 状态后重试。"}</p></div>;
  return <dl className="check-result"><div><dt>系统</dt><dd>{value.osName ?? "-"}</dd></div><div><dt>架构</dt><dd>{value.architecture ?? "-"}</dd></div><div><dt>可用磁盘</dt><dd>{formatBytes(value.diskAvailableBytes)}</dd></div><div><dt>结果</dt><dd className="success-text"><CheckCircle2 aria-hidden="true" />检查通过</dd></div></dl>;
}
function formatBytes(value?: number | null) { if (value == null) return "-"; return `${(value / 1024 / 1024 / 1024).toFixed(1)} GiB`; }
