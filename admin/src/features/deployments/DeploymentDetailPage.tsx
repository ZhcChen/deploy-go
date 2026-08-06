import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { ArrowLeft, RotateCcw, X } from "lucide-react";
import { useCallback, useRef, useState } from "react";
import { Link, useNavigate, useParams } from "react-router-dom";
import { Button } from "../../components/Button";
import { ConfirmDialog } from "../../components/ConfirmDialog";
import { PageState } from "../../components/PageState";
import { ApiError } from "../../api/http-client";
import { useAuth } from "../auth/AuthContext";
import { toNotice } from "../shared/toNotice";
import { ApiErrorNotice } from "../errors/ApiErrorNotice";
import { createIdempotencyKey, deploymentsApi } from "./api";
import { DeploymentLogPanel } from "./DeploymentLogPanel";
import { deploymentStatusLabel, deploymentStatusTone, isTerminalDeployment } from "./status";

export function DeploymentDetailPage() {
  const { id = "" } = useParams();
  const auth = useAuth();
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const [retryKey] = useState(() => createIdempotencyKey("retry"));
  const [confirmingCancel, setConfirmingCancel] = useState(false);
  const [accessError, setAccessError] = useState<ApiError | null>(null);
  const cancelLock = useRef(false);
  const retryLock = useRef(false);
  const backLinkRef = useRef<HTMLAnchorElement>(null);
  const revokeAccess = useCallback((error: ApiError) => {
    queryClient.removeQueries({ queryKey: ["deployment", id], exact: true });
    queryClient.removeQueries({ queryKey: ["deployments"] });
    setAccessError(error);
  }, [id, queryClient]);
  const deployment = useQuery({ queryKey: ["deployment", id], queryFn: async () => {
    try {
      return await deploymentsApi.show(id);
    } catch (error) {
      if (error instanceof ApiError && error.status === 403) {
        queueMicrotask(() => revokeAccess(error));
      }
      throw error;
    }
  }, refetchInterval: (query) => query.state.data && !isTerminalDeployment(query.state.data.status) ? 2000 : false });
  const cancel = useMutation({ mutationFn: async () => {
    if (!auth.csrfToken) throw new Error("缺少 CSRF token");
    return deploymentsApi.cancel(id, auth.csrfToken);
  }, onSuccess: (saved) => { queryClient.setQueryData(["deployment", id], saved); void queryClient.invalidateQueries({ queryKey: ["deployments"] }); }, onError: (error) => { if (error instanceof ApiError && error.status === 403) revokeAccess(error); }, onSettled: () => { cancelLock.current = false; } });
  const retry = useMutation({ mutationFn: async () => {
    if (!auth.csrfToken) throw new Error("缺少 CSRF token");
    return deploymentsApi.retry(id, auth.csrfToken, retryKey);
  }, onSuccess: (saved) => navigate(`/deployments/${saved.id}`), onError: (error) => { if (error instanceof ApiError && error.status === 403) revokeAccess(error); }, onSettled: () => { retryLock.current = false; } });
  if (accessError) return <div className="state-with-action"><ApiErrorNotice error={accessError} /><Link className="button button--default" to="/deployments">返回部署</Link></div>;
  if (deployment.isLoading) return <PageState kind="loading" />;
  if (deployment.isError || !deployment.data) return <div className="state-with-action"><ApiErrorNotice error={toNotice(deployment.error)} /><Link className="button button--default" to="/deployments">返回部署</Link></div>;
  const data = deployment.data;
  const cancelable = data.status === "queued" || data.status === "running";
  const retryable = data.status === "failed" || data.status === "canceled" || data.status === "interrupted";
  return <section className="workspace deployment-detail"><Link ref={backLinkRef} className="back-link" to="/deployments"><ArrowLeft aria-hidden="true" />返回部署</Link><div className="detail-title"><div><h2><code>{data.id}</code></h2><p>目标 <code>{data.targetId}</code></p></div><span className={`status-badge status-badge--${deploymentStatusTone(data.status)}`}>{deploymentStatusLabel(data.status)}</span></div>
    <div className="detail-toolbar">{cancelable ? <Button tone="danger" disabled={cancel.isPending} onClick={() => setConfirmingCancel(true)}><X aria-hidden="true" />{cancel.isPending ? "正在取消..." : "取消部署"}</Button> : null}{retryable ? <Button tone="primary" disabled={retry.isPending} onClick={() => { if (retryLock.current) return; retryLock.current = true; retry.mutate(); }}><RotateCcw aria-hidden="true" />{retry.isPending ? "正在创建..." : "重试部署"}</Button> : null}</div>
    {data.status === "interrupted" ? <p className="notice notice--warning">平台无法证明远端脚本的最终状态。请先核对节点，确认没有冲突执行后再重试。</p> : null}
    {cancel.error ? <ApiErrorNotice error={toNotice(cancel.error)} /> : null}{retry.error ? <ApiErrorNotice error={toNotice(retry.error)} /> : null}
    <dl className="definition-grid deployment-metadata"><div><dt>阶段</dt><dd>{data.phase}</dd></div><div><dt>执行模式</dt><dd>{data.executionMode === "two_stage" ? "两阶段（prepare + release）" : "单脚本"}</dd></div>{data.executionMode === "two_stage" ? <><div><dt>固定分支</dt><dd>{data.deploymentBranch ? <code>{data.deploymentBranch}</code> : "-"}</dd></div><div><dt>Commit</dt><dd>{data.resolvedCommitSha ? <code>{data.resolvedCommitSha}</code> : "-"}</dd></div><div><dt>发布版本</dt><dd>{data.releaseVersion || "-"}</dd></div><div><dt>模块</dt><dd>{data.modules?.join(", ") || "-"}</dd></div></> : null}<div><dt>Snapshot</dt><dd><code>{data.snapshotHash}</code></dd></div><div><dt>排队时间</dt><dd>{new Date(data.queuedAt).toLocaleString()}</dd></div><div><dt>开始时间</dt><dd>{data.startedAt ? new Date(data.startedAt).toLocaleString() : "-"}</dd></div><div><dt>结束时间</dt><dd>{data.finishedAt ? new Date(data.finishedAt).toLocaleString() : "-"}</dd></div><div><dt>退出码</dt><dd>{data.exitCode ?? "-"}</dd></div><div><dt>协议完整</dt><dd>{data.protocolComplete ? "是" : "否"}</dd></div><div><dt>结果</dt><dd>{data.resultSummary || "-"}</dd></div></dl>
    {data.executionMode === "two_stage" && data.stageTasks?.length ? <section className="detail-section stage-summary"><div className="section-heading"><div><h3>阶段任务</h3><p>prepare 与 release 分别持久化，日志已按阶段分组。</p></div></div><ul className="stage-list">{data.stageTasks.map((task) => <li key={task.taskId} className={`stage-card stage-card--${task.stage}`}><div className="stage-card-head"><strong>{task.stage === "prepare" ? "准备 prepare" : "发布 release"}</strong><span className={`status-badge status-badge--${task.status === "succeeded" ? "online" : task.status === "queued" || task.status === "running" || task.status === "accepted" || task.status === "delivered" ? "pending" : "disabled"}`}>{task.status}</span></div><dl className="definition-grid"><div><dt>任务</dt><dd><code>{task.taskId}</code></dd></div><div><dt>退出码</dt><dd>{task.exitCode ?? "-"}</dd></div><div><dt>错误</dt><dd>{task.errorCode || "-"}</dd></div><div><dt>开始</dt><dd>{task.startedAt ? new Date(task.startedAt).toLocaleString() : "-"}</dd></div><div><dt>结束</dt><dd>{task.finishedAt ? new Date(task.finishedAt).toLocaleString() : "-"}</dd></div></dl></li>)}</ul></section> : null}
    <DeploymentLogPanel key={id} deploymentId={id} onTerminal={() => void deployment.refetch()} onAuthorizationRevoked={revokeAccess} />
    <ConfirmDialog open={confirmingCancel} title="取消部署" message="取消只会停止脚本，不会自动回滚应用变更。" confirmLabel="确认取消" pending={cancel.isPending} fallbackFocusRef={backLinkRef} onClose={() => setConfirmingCancel(false)} onConfirm={() => { if (cancelLock.current) return; cancelLock.current = true; cancel.mutate(undefined, { onSettled: () => setConfirmingCancel(false) }); }} />
  </section>;
}
