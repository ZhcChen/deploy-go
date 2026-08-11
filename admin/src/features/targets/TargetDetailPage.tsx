import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { ArrowLeft } from "lucide-react";
import { Link, useParams } from "react-router-dom";
import type { NodeResponse } from "../../api/generated/models/NodeResponse";
import { Button } from "../../components/Button";
import { PageState } from "../../components/PageState";
import { executionModeLabel, privilegedReleaseLabel } from "./labels";
import { useAuth } from "../auth/AuthContext";
import { toNotice } from "../shared/toNotice";
import { ApiErrorNotice } from "../errors/ApiErrorNotice";
import { applicationNodesApi, deploymentTargetsApi } from "../applications/api";
import { useCursorCollection } from "../shared/useCursorCollection";
import { TargetEditor } from "./TargetEditor";

export function TargetDetailPage() {
  const { id = "", targetId = "" } = useParams();
  const auth = useAuth();
  const queryClient = useQueryClient();
  const isAdministrator = auth.user?.identity === "administrator";
  const target = useQuery({ queryKey: ["deployment-target", targetId], queryFn: () => deploymentTargetsApi.deploymentTargetsShow({ id: targetId }) });
  const nodes = useCursorCollection(["nodes", "target-options"], (after) => applicationNodesApi.nodesList({ limit: 200, after: after ?? undefined }));
  const currentNode = useQuery({ queryKey: ["node", target.data?.nodeId], queryFn: () => applicationNodesApi.nodesShow({ id: target.data!.nodeId }), enabled: Boolean(target.data?.nodeId) });
  const status = useMutation({ mutationFn: async () => {
    if (!auth.csrfToken || !target.data) throw new Error("缺少必要的安全上下文");
    return deploymentTargetsApi.deploymentTargetsUpdateStatus({ id: targetId, xCSRFToken: auth.csrfToken, targetStatusRequest: { status: target.data.status === "active" ? "disabled" : "active", version: target.data.version } });
  }, onSuccess: (saved) => { queryClient.setQueryData(["deployment-target", targetId], saved); void queryClient.invalidateQueries({ queryKey: ["deployment-targets", id] }); } });
  if (target.isLoading) return <PageState kind="loading" />;
  if (target.isError || !target.data) return <div className="state-with-action"><ApiErrorNotice error={toNotice(target.error)} /><Link className="button button--default" to={`/apps/${id}`}>返回应用</Link></div>;
  const nodeName = currentNode.data?.name ?? target.data.nodeId;
  return <section className="workspace detail-page"><Link className="back-link" to={`/apps/${id}`}><ArrowLeft aria-hidden="true" />返回应用</Link><div className="detail-title"><div><h2>{nodeName}</h2><p><code>{target.data.nodeId}</code> · {executionModeLabel(target.data.executionMode)}</p></div><span className={`status-badge status-badge--${target.data.status === "active" ? "online" : "disabled"}`}>{target.data.status === "active" ? "启用" : "停用"}</span></div>
    <dl className="definition-grid target-summary"><div><dt>节点</dt><dd>{currentNode.data ? <span className="target-node-summary"><code>{currentNode.data.name}</code><span className={`status-badge status-badge--${currentNode.data.status === "online" ? "online" : "offline"}`}>{currentNode.data.status === "online" ? "在线" : "离线"}</span></span> : <code>{target.data.nodeId}</code>}</dd></div><div><dt>执行模式</dt><dd>{executionModeLabel(target.data.executionMode)}{target.data.executionMode === "two_stage" ? <span className={`privilege-badge target-summary__privilege${target.data.privilegedRelease ? " privilege-badge--enabled" : ""}`}>{privilegedReleaseLabel(target.data.privilegedRelease)}</span> : null}</dd></div><div><dt>兼容环境标识</dt><dd>{target.data.environment}</dd></div><div><dt>发布脚本</dt><dd><code>{target.data.scriptPath}</code></dd></div><div><dt>Snapshot</dt><dd><code>{target.data.snapshotHash}</code></dd></div><div><dt>更新时间</dt><dd>{new Date(target.data.updatedAt).toLocaleString("zh-CN")}</dd></div></dl>
    {target.data.status === "active" ? <div className="detail-toolbar"><Link className="button button--primary" to={`/deployments/new?application=${id}`}>部署应用</Link></div> : null}
    {isAdministrator ? <><div className="detail-toolbar"><Button disabled={status.isPending} onClick={() => { if (target.data.status !== "active" || window.confirm("停用后将阻止该目标的新部署，确定继续吗？")) status.mutate(); }}>{target.data.status === "active" ? "停用目标" : "启用目标"}</Button></div>{status.error ? <ApiErrorNotice error={toNotice(status.error)} /> : null}<TargetEditor key={target.data.version} applicationId={id} nodes={mergeCurrentNode(nodes.items, currentNode.data)} target={target.data} hasMoreNodes={nodes.hasNextPage} loadingMoreNodes={nodes.isFetchingNextPage} onLoadMoreNodes={() => void nodes.fetchNextPage()} onDiscard={() => window.history.back()} onSaved={(saved) => queryClient.setQueryData(["deployment-target", targetId], saved)} /></> : <section className="detail-section"><h3>脚本参数规范</h3><pre className="json-preview">{JSON.stringify(target.data.parameterSchema, null, 2)}</pre><h3>部署后验证</h3><pre className="json-preview">{JSON.stringify(target.data.verificationConfig, null, 2)}</pre></section>}
  </section>;
}

function mergeCurrentNode(nodes: NodeResponse[], current?: NodeResponse) {
  return current && !nodes.some((node) => node.id === current.id) ? [current, ...nodes] : nodes;
}
