import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { FileKey2, RefreshCw } from "lucide-react";
import { Link } from "react-router-dom";
import type { ApplicationEnvFileResponse } from "../../api/generated/models/ApplicationEnvFileResponse";
import { Button } from "../../components/Button";
import { PageState } from "../../components/PageState";
import { useAuth } from "../auth/AuthContext";
import { ApiErrorNotice } from "../errors/ApiErrorNotice";
import { toNotice } from "../shared/toNotice";
import { applicationEnvsApi } from "./api";

export function ApplicationEnvSection({ applicationId, isAdministrator }: { applicationId: string; isAdministrator: boolean }) {
  const auth = useAuth();
  const queryClient = useQueryClient();
  const envFiles = useQuery({ queryKey: ["application-env-files", applicationId], queryFn: () => applicationEnvsApi.applicationEnvsList({ applicationId }) });
  const retry = useMutation({
    mutationFn: ({ envFileId, targetId }: { envFileId: string; targetId: string }) => {
      if (!auth.csrfToken) throw new Error("缺少必要的安全上下文");
      return applicationEnvsApi.applicationEnvsRetrySync({ envFileId, targetId, xCSRFToken: auth.csrfToken });
    },
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ["application-env-files", applicationId] }),
  });
  const items = envFiles.data?.items ?? [];

  return <section className="detail-section application-envs">
    <div className="section-heading"><div><h3>应用 Env</h3><p>只显示业务应用已经上传登记的文件，内容统一同步到全部目标节点。</p></div></div>
    {envFiles.isLoading ? <PageState kind="loading" /> : envFiles.isError ? <div className="state-with-action"><ApiErrorNotice error={toNotice(envFiles.error)} /><Button onClick={() => void envFiles.refetch()}>重试</Button></div> : items.length === 0 ? <div className="empty-inline"><p>业务应用尚未上传 Env 文件。</p></div> : <ul className="env-file-list">{items.map((file) => <li key={file.id}>
      <div className="env-file-identity"><FileKey2 aria-hidden="true" /><span><strong>{file.fileName}</strong><small>{file.module} · {file.format} · 更新于 {new Date(file.updatedAt).toLocaleString("zh-CN")}</small></span></div>
      <div className="env-file-version"><strong>v{file.currentVersion}</strong><code title={file.currentDigest}>{file.currentDigest.slice(0, 12)}</code></div>
      <div className="env-sync-summary" aria-label={`${file.fileName} 同步状态`}>
        {file.pendingCount > 0 ? <span className="sync-state sync-state--pending">待同步 {file.pendingCount}</span> : null}
        {file.syncingCount > 0 ? <span className="sync-state sync-state--syncing">同步中 {file.syncingCount}</span> : null}
        {file.succeededCount > 0 ? <span className="sync-state sync-state--succeeded">已同步 {file.succeededCount}</span> : null}
        {file.failedCount > 0 ? <span className="sync-state sync-state--failed">失败 {file.failedCount}</span> : null}
        {file.targetCount === 0 ? <span className="sync-state">无目标节点</span> : null}
      </div>
      <div className="env-file-actions">
        {isAdministrator ? <Link className="button button--default" aria-label={`编辑 ${file.fileName}`} to={`/apps/${applicationId}/env/${file.id}`}>编辑</Link> : null}
      </div>
      <EnvSyncDetails file={file} isAdministrator={isAdministrator} retryingTargetId={retry.isPending ? retry.variables?.targetId : undefined} onRetry={(targetId) => retry.mutate({ envFileId: file.id, targetId })} />
    </li>)}</ul>}
    {retry.error ? <ApiErrorNotice error={toNotice(retry.error)} /> : null}
  </section>;
}

export function EnvSyncDetails({ file, isAdministrator, retryingTargetId, onRetry }: { file: ApplicationEnvFileResponse; isAdministrator: boolean; retryingTargetId?: string; onRetry?(targetId: string): void }) {
  if (file.syncs.length === 0) return null;
  return <ul className="env-sync-list" aria-label={`${file.fileName} 节点同步明细`}>{file.syncs.map((sync) => <li key={sync.targetId}>
    <div className="env-sync-node"><strong>{sync.nodeName}</strong><small><code>{sync.nodeId}</code> · 目标 <code>{sync.targetId}</code></small></div>
    <span className={`sync-state sync-state--${sync.status}`}>{syncStatusLabel(sync.status)}</span>
    <div className="env-sync-version"><span>{sync.actualVersion == null ? "尚无实际版本" : `实际版本 v${sync.actualVersion}`}</span><small>{sync.syncedAt ? `同步于 ${formatTime(sync.syncedAt)}` : sync.lastAttemptAt ? `最后尝试 ${formatTime(sync.lastAttemptAt)}` : "尚未尝试"}</small></div>
    <div className="env-sync-error">{sync.errorMessage ? <><strong>{sync.errorMessage}</strong><code>{sync.errorCode}</code></> : <span className="text-muted">-</span>}</div>
    <div className="env-sync-actions">{isAdministrator && sync.status === "failed" && onRetry ? <Button aria-label={`重试 ${sync.nodeName} 的 Env 同步`} disabled={retryingTargetId !== undefined} onClick={() => onRetry(sync.targetId)}><RefreshCw aria-hidden="true" />{retryingTargetId === sync.targetId ? "重试中" : "重试"}</Button> : null}</div>
  </li>)}</ul>;
}

function syncStatusLabel(status: string) {
  return ({ pending: "待同步", syncing: "同步中", succeeded: "已同步", failed: "失败" } as Record<string, string>)[status] ?? status;
}

function formatTime(value: string) {
  return new Date(value).toLocaleString("zh-CN");
}
