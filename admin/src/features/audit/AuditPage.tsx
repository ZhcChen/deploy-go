import { FileClock } from "lucide-react";
import { useState } from "react";
import { Button } from "../../components/Button";
import { PageState } from "../../components/PageState";
import { toNotice } from "../credentials/CredentialsPage";
import { ApiErrorNotice } from "../errors/ApiErrorNotice";
import { useCursorCollection } from "../shared/useCursorCollection";
import { auditApi } from "./api";

export function AuditPage() {
  const [action, setAction] = useState("");
  const [resourceType, setResourceType] = useState("");
  const logs = useCursorCollection(["audit-logs", action, resourceType], (after) => auditApi.auditList({ limit: 30, after: after ?? undefined, action: action || undefined, resourceType: resourceType || undefined }));
  return <section className="workspace">
    <div className="workspace-heading"><div><h2>审计记录</h2><p>按时间倒序查看系统状态变更，不显示密码、凭证或脚本 secret。</p></div></div>
    <div className="filter-bar audit-filters"><label>动作<input value={action} onChange={(event) => setAction(event.target.value.trim())} placeholder="user.create" /></label><label>资源类型<input value={resourceType} onChange={(event) => setResourceType(event.target.value.trim())} placeholder="user" /></label></div>
    {logs.isLoading ? <PageState kind="loading" /> : logs.isError ? <div className="state-with-action"><ApiErrorNotice error={toNotice(logs.error)} /><Button onClick={() => void logs.refetch()}>重试</Button></div> : logs.items.length === 0 ? <PageState kind="empty" /> : <><div className="data-table-wrap"><table className="data-table audit-table"><thead><tr><th>时间</th><th>动作</th><th>资源</th><th>操作者</th><th>Request ID</th></tr></thead><tbody>{logs.items.map((log) => <tr key={log.id}><td>{new Date(log.createdAt).toLocaleString()}</td><td><FileClock aria-hidden="true" /><code>{log.action}</code></td><td><code>{log.resourceType}:{log.resourceId}</code></td><td>{log.actorId || "系统"}</td><td><code>{log.requestId}</code></td></tr>)}</tbody></table></div>{logs.hasNextPage ? <div className="pagination-actions"><Button disabled={logs.isFetchingNextPage} onClick={() => void logs.fetchNextPage()}>加载更多</Button></div> : null}</>}
  </section>;
}
