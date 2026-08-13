import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Play, RotateCcw, Server, Settings, X } from "lucide-react";
import { useCallback, useRef, useState } from "react";
import { Link, useNavigate, useParams, useSearchParams } from "react-router-dom";
import { BackLink } from "../../components/BackLink";
import { Button } from "../../components/Button";
import { ConfirmDialog } from "../../components/ConfirmDialog";
import { PageState } from "../../components/PageState";
import { ApiError } from "../../api/http-client";
import { useAuth } from "../auth/AuthContext";
import { toNotice } from "../shared/toNotice";
import { ApiErrorNotice } from "../errors/ApiErrorNotice";
import { createIdempotencyKey, deploymentsApi } from "./api";
import { DeploymentLogPanel } from "./DeploymentLogPanel";
import { DeploymentFlowPanel } from "./DeploymentFlowPanel";
import { deploymentStatusLabel, deploymentStatusTone, formatDeploymentDuration, isTerminalDeployment } from "./status";

export function DeploymentDetailPage() {
  const { id = "" } = useParams();
  const auth = useAuth();
  const navigate = useNavigate();
  const [searchParams, setSearchParams] = useSearchParams();
  const queryClient = useQueryClient();
  const [retryKey] = useState(() => createIdempotencyKey("retry"));
  const [confirmingCancel, setConfirmingCancel] = useState(false);
  const [confirmingRetry, setConfirmingRetry] = useState(false);
  const [confirmingRelease, setConfirmingRelease] = useState(false);
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
  const release = useMutation({ mutationFn: async () => {
    if (!auth.csrfToken) throw new Error("缺少 CSRF token");
    return deploymentsApi.release(id, auth.csrfToken);
  }, onSuccess: (saved) => { queryClient.setQueryData(["deployment", id], saved); void queryClient.invalidateQueries({ queryKey: ["deployments"] }); } });
  if (accessError) return <div className="state-with-action"><ApiErrorNotice error={accessError} /><Link className="button button--default" to="/deployments">返回部署</Link></div>;
  if (deployment.isLoading) return <PageState kind="loading" />;
  if (deployment.isError || !deployment.data) return <div className="state-with-action"><ApiErrorNotice error={toNotice(deployment.error)} /><Link className="button button--default" to="/deployments">返回部署</Link></div>;
  const data = deployment.data;
  const cancelable = data.status === "queued" || data.status === "running";
  const retryTargets = data.targetRuns.filter((run) => !matchesSuccessfulRun(run.status));
  const retryable = (data.status === "failed" || data.status === "canceled" || data.status === "interrupted")
    && !data.targetRuns.some((run) => run.status === "downloading" || run.status === "running");
  const hasActions = data.phase === "awaiting_release" || cancelable || (retryable && retryTargets.length > 0);
  const requestedView = searchParams.get("view");
  const view = requestedView === "details" || requestedView === "logs" ? requestedView : "flow";
  const logStage = searchParams.get("stage") === "prepare" ? "prepare" : searchParams.get("stage") === "release" ? "release" : undefined;
  const selectView = (next: "flow" | "details" | "logs", stage?: "prepare" | "release") => {
    const params = new URLSearchParams();
    params.set("view", next);
    if (stage) params.set("stage", stage);
    setSearchParams(params);
  };
  return <section className="workspace deployment-detail"><BackLink linkRef={backLinkRef} to="/deployments" parentLabel="部署列表" /><div className="detail-title"><div><h2><code>{data.id}</code></h2><p>应用 <code>{data.applicationId}</code> · {data.targetRuns.length} 个目标</p></div><span className={`status-badge status-badge--${deploymentStatusTone(data.status)}`}>{deploymentStatusLabel(data.status)}</span></div>
    {hasActions ? <div className="detail-toolbar">{data.phase === "awaiting_release" ? <><Link className="button button--default" to={`/applications/${data.applicationId}`}><Settings aria-hidden="true" />配置 Env</Link><Button tone="primary" disabled={release.isPending} onClick={() => setConfirmingRelease(true)}><Play aria-hidden="true" />{release.isPending ? "正在开始..." : "开始发布"}</Button></> : null}{cancelable ? <Button tone="danger" disabled={cancel.isPending} onClick={() => setConfirmingCancel(true)}><X aria-hidden="true" />{cancel.isPending ? "正在取消..." : "取消部署"}</Button> : null}{retryable && retryTargets.length > 0 ? <Button tone="primary" disabled={retry.isPending} onClick={() => setConfirmingRetry(true)}><RotateCcw aria-hidden="true" />{retry.isPending ? "正在创建..." : "重试失败目标"}</Button> : null}</div> : null}
    {data.status === "interrupted" ? <p className="notice notice--warning">平台无法证明远端脚本的最终状态。请先核对节点，确认没有冲突执行后再重试。</p> : null}
    {data.phase === "awaiting_release" ? <p className="notice notice--warning">prepare 已完成。请检查应用 Env 已同步到全部目标节点，再开始 release。</p> : null}
    {cancel.error ? <ApiErrorNotice error={toNotice(cancel.error)} /> : null}{retry.error ? <ApiErrorNotice error={toNotice(retry.error)} /> : null}{release.error ? <ApiErrorNotice error={toNotice(release.error)} /> : null}
    <div className="detail-tabs" role="tablist" aria-label="部署详情视图">{([['flow', '流程'], ['details', '详情'], ['logs', '日志']] as const).map(([value, label]) => <button key={value} type="button" role="tab" aria-selected={view === value} onClick={() => selectView(value)}>{label}</button>)}</div>
    <div role="tabpanel" aria-label="流程" hidden={view !== "flow"}>{view === "flow" ? <DeploymentFlowPanel deployment={data} onViewLogs={(stage) => selectView("logs", stage)} /> : null}</div>
    <div role="tabpanel" aria-label="详情" hidden={view !== "details"}>
    <dl className="definition-grid deployment-metadata"><div><dt>阶段</dt><dd>{data.phase}</dd></div><div><dt>执行模式</dt><dd>{data.executionMode === "two_stage" ? "两阶段（prepare + release）" : data.executionMode === "image" ? "镜像直连（固定 Make target）" : "单脚本"}</dd></div>{data.executionMode === "two_stage" ? <><div><dt>固定分支</dt><dd>{data.deploymentBranch ? <code>{data.deploymentBranch}</code> : "-"}</dd></div><div><dt>Commit</dt><dd>{data.resolvedCommitSha ? <code>{data.resolvedCommitSha}</code> : "-"}</dd></div><div><dt>发布版本</dt><dd>{data.releaseVersion || "-"}</dd></div><div><dt>模块</dt><dd>{data.modules?.join(", ") || "-"}</dd></div></> : data.executionMode === "image" && data.imageSpec ? <><div><dt>模板</dt><dd><code>{data.imageSpec.template}</code></dd></div><div><dt>镜像</dt><dd><code>{data.imageSpec.image}</code></dd></div><div><dt>宿主端口</dt><dd><code>{data.imageSpec.host_port}</code></dd></div><div><dt>Env 文件</dt><dd>{data.imageSpec.env_files.join(", ")}</dd></div></> : null}<div><dt>Snapshot</dt><dd><code>{data.snapshotHash}</code></dd></div><div><dt>排队时间</dt><dd>{new Date(data.queuedAt).toLocaleString()}</dd></div><div><dt>开始时间</dt><dd>{data.startedAt ? new Date(data.startedAt).toLocaleString() : "-"}</dd></div><div><dt>结束时间</dt><dd>{data.finishedAt ? new Date(data.finishedAt).toLocaleString() : "-"}</dd></div><div><dt>总耗时</dt><dd>{data.finishedAt ? formatDeploymentDuration(data.queuedAt, data.finishedAt) : "进行中"}</dd></div><div><dt>执行耗时</dt><dd>{data.startedAt && data.finishedAt ? formatDeploymentDuration(data.startedAt, data.finishedAt) : "-"}</dd></div><div><dt>退出码</dt><dd>{data.exitCode ?? "-"}</dd></div><div><dt>协议完整</dt><dd>{data.protocolComplete ? "是" : "否"}</dd></div><div><dt>结果</dt><dd>{data.resultSummary || "-"}</dd></div></dl>
    {data.targetRuns.length > 0 ? <section className="detail-section target-run-summary"><div className="section-heading"><div><h3>逐节点状态</h3><p>整体失败不会覆盖已经成功的节点事实。</p></div></div><ul className="target-run-list">{data.targetRuns.map((run) => <li key={run.id}><div className="target-run-head"><div><Server aria-hidden="true" /><span><strong>{run.nodeId}</strong><code>{run.targetId}</code></span></div><span className={`status-badge status-badge--${runTone(run.status)}`}>{runStatusLabel(run.status)}</span></div><dl className="definition-grid"><div><dt>阶段</dt><dd>{run.phase}</dd></div><div><dt>Env 门禁</dt><dd>{envGateLabel(run.envGateStatus)}</dd></div><div><dt>开始</dt><dd>{run.startedAt ? new Date(run.startedAt).toLocaleString() : "等待执行"}</dd></div><div><dt>结束</dt><dd>{run.finishedAt ? new Date(run.finishedAt).toLocaleString() : "-"}</dd></div><div><dt>结果</dt><dd>{run.resultSummary || "-"}</dd></div><div><dt>错误码</dt><dd>{run.errorCode || "-"}</dd></div></dl></li>)}</ul></section> : null}
    {(data.executionMode === "two_stage" || data.executionMode === "image") && data.stageTasks?.length ? <section className="detail-section stage-summary"><div className="section-heading"><div><h3>阶段任务</h3><p>prepare 与 release 分别持久化，日志已按阶段分组。</p></div></div><ul className="stage-list">{data.stageTasks.map((task) => <li key={task.taskId} className={`stage-card stage-card--${task.stage}`}><div className="stage-card-head"><strong>{task.stage === "prepare" ? "准备 prepare" : "发布 release"}</strong><span className={`status-badge status-badge--${task.status === "succeeded" ? "online" : task.status === "queued" || task.status === "running" || task.status === "accepted" || task.status === "delivered" ? "pending" : "disabled"}`}>{task.status}</span></div><dl className="definition-grid"><div><dt>任务</dt><dd><code>{task.taskId}</code></dd></div><div><dt>退出码</dt><dd>{task.exitCode ?? "-"}</dd></div><div><dt>错误</dt><dd>{task.errorCode || "-"}</dd></div><div><dt>开始</dt><dd>{task.startedAt ? new Date(task.startedAt).toLocaleString() : "-"}</dd></div><div><dt>结束</dt><dd>{task.finishedAt ? new Date(task.finishedAt).toLocaleString() : "-"}</dd></div></dl></li>)}</ul></section> : null}
    </div>
    <div role="tabpanel" aria-label="日志" hidden={view !== "logs"}>{view === "logs" ? <DeploymentLogPanel key={`${id}-${logStage ?? "all"}`} deploymentId={id} initialStage={logStage} onTerminal={() => void deployment.refetch()} onAuthorizationRevoked={revokeAccess} /> : null}</div>
    <ConfirmDialog open={confirmingCancel} title="取消部署" message="取消只会停止脚本，不会自动回滚应用变更。" confirmLabel="确认取消" pending={cancel.isPending} fallbackFocusRef={backLinkRef} onClose={() => setConfirmingCancel(false)} onConfirm={() => { if (cancelLock.current) return; cancelLock.current = true; cancel.mutate(undefined, { onSettled: () => setConfirmingCancel(false) }); }} />
    <ConfirmDialog open={confirmingRetry} title="重试失败目标" message={<><p>将创建新的部署事实，只重新执行以下失败或未执行目标；已成功目标保留为 reused。</p><ul className="confirm-target-list">{retryTargets.map((run) => <li key={run.id}><code>{run.nodeId}</code><span>{runStatusLabel(run.status)} · {run.phase}</span></li>)}</ul></>} confirmLabel={`确认重试 ${retryTargets.length} 个目标`} tone="primary" pending={retry.isPending} fallbackFocusRef={backLinkRef} onClose={() => setConfirmingRetry(false)} onConfirm={() => { if (retryLock.current) return; retryLock.current = true; retry.mutate(undefined, { onSettled: () => setConfirmingRetry(false) }); }} />
    <ConfirmDialog open={confirmingRelease} title="开始发布" message="将使用 prepare 固定的 commit 和制品，在全部目标节点执行 release。" confirmLabel="确认开始发布" tone="primary" pending={release.isPending} fallbackFocusRef={backLinkRef} onClose={() => setConfirmingRelease(false)} onConfirm={() => release.mutate(undefined, { onSettled: () => setConfirmingRelease(false) })} />
  </section>;
}

function matchesSuccessfulRun(status: string) {
  return status === "succeeded" || status === "reused";
}

function runTone(status: string) {
  if (matchesSuccessfulRun(status)) return "online";
  if (["pending", "downloading", "running"].includes(status)) return "pending";
  return "disabled";
}

function runStatusLabel(status: string) {
  const labels: Record<string, string> = { pending: "等待", downloading: "下载制品", running: "执行中", succeeded: "成功", failed: "失败", canceled: "已取消", expired: "等待超时", reused: "复用成功事实" };
  return labels[status] ?? status;
}

function envGateLabel(status: string) {
  const labels: Record<string, string> = { pending: "等待同步", ready: "已就绪", failed: "同步失败", not_required: "无需 Env" };
  return labels[status] ?? status;
}
