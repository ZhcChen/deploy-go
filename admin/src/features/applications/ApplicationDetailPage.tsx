import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Archive, Play, Plus } from "lucide-react";
import { useState, type FormEvent } from "react";
import { Link, useParams } from "react-router-dom";
import { Button } from "../../components/Button";
import { BackLink } from "../../components/BackLink";
import { Field, TextArea, TextInput } from "../../components/form";
import { PageState } from "../../components/PageState";
import { useAuth } from "../auth/AuthContext";
import { toNotice } from "../shared/toNotice";
import { ApiErrorNotice } from "../errors/ApiErrorNotice";
import { TargetEditor } from "../targets/TargetEditor";
import { applicationNodesApi, applicationsApi, deploymentTargetsApi } from "./api";
import { useCursorCollection } from "../shared/useCursorCollection";
import { useUnsavedChanges } from "../shared/useUnsavedChanges";
import { ApplicationSourceSection } from "./ApplicationSourceSection";
import { ApplicationEnvSection } from "../application-envs/ApplicationEnvSection";

export function ApplicationDetailPage() {
  const { id = "" } = useParams();
  const auth = useAuth();
  const queryClient = useQueryClient();
  const isAdministrator = auth.user?.identity === "administrator";
  const [editing, setEditing] = useState(false);
  const [addingTarget, setAddingTarget] = useState(false);
  const app = useQuery({ queryKey: ["application", id], queryFn: () => applicationsApi.applicationsShow({ id }) });
  const targets = useCursorCollection(["deployment-targets", id], (after) => deploymentTargetsApi.deploymentTargetsList({ applicationId: id, limit: 20, after: after ?? undefined }));
  const nodes = useCursorCollection(["nodes", "target-options"], (after) => applicationNodesApi.nodesList({ limit: 200, after: after ?? undefined }));
  const [name, setName] = useState<string | null>(null);
  const [slug, setSlug] = useState<string | null>(null);
  const [description, setDescription] = useState<string | null>(null);
  useUnsavedChanges(editing && (name !== null || slug !== null || description !== null));
  const update = useMutation({ mutationFn: async () => {
    if (!auth.csrfToken || !app.data) throw new Error("缺少必要的安全上下文");
    return applicationsApi.applicationsUpdate({ id, xCSRFToken: auth.csrfToken, saveApplicationRequest: { name: (name ?? app.data.name).trim(), slug: (slug ?? app.data.slug).trim(), description: (description ?? app.data.description).trim(), version: app.data.version } });
  }, onSuccess: (saved) => { queryClient.setQueryData(["application", id], saved); void queryClient.invalidateQueries({ queryKey: ["applications"] }); setEditing(false); } });
  const status = useMutation({ mutationFn: async () => {
    if (!auth.csrfToken || !app.data) throw new Error("缺少必要的安全上下文");
    return applicationsApi.applicationsUpdateStatus({ id, xCSRFToken: auth.csrfToken, applicationStatusRequest: { status: app.data.status === "active" ? "archived" : "active", version: app.data.version } });
  }, onSuccess: (saved) => { queryClient.setQueryData(["application", id], saved); void queryClient.invalidateQueries({ queryKey: ["applications"] }); } });
  async function submit(event: FormEvent) { event.preventDefault(); await update.mutateAsync().catch(() => undefined); }
  function changeStatus() {
    if (app.data?.status === "active" && !window.confirm("归档后将阻止创建和执行新的部署目标，确定继续吗？")) return;
    status.mutate();
  }
  if (app.isLoading) return <PageState kind="loading" />;
  if (app.isError || !app.data) return <div className="state-with-action"><ApiErrorNotice error={toNotice(app.error)} /><Link className="button button--default" to="/apps">返回应用</Link></div>;
  return <section className="workspace detail-page">
    <BackLink to="/apps" parentLabel="应用列表" />
    <div className="detail-title"><div><h2>{app.data.name}</h2><p><code>{app.data.slug}</code> · {app.data.description || "暂无说明"}</p></div><span className={`status-badge status-badge--${app.data.status === "active" ? "online" : "disabled"}`}>{app.data.status === "active" ? "启用" : "已归档"}</span></div>
    {isAdministrator ? <div className="detail-toolbar"><Button onClick={() => setEditing((value) => !value)}>编辑应用</Button><Button tone={app.data.status === "active" ? "danger" : "default"} disabled={status.isPending} onClick={changeStatus}><Archive aria-hidden="true" />{app.data.status === "active" ? "归档应用" : "恢复应用"}</Button></div> : null}
    {status.error ? <ApiErrorNotice error={toNotice(status.error)} /> : null}
    {editing ? <form className="node-form" onSubmit={(event) => void submit(event)}>
      <Field label="名称"><TextInput required value={name ?? app.data.name} onChange={(event) => setName(event.target.value)} /></Field>
      <Field label="Slug"><TextInput required value={slug ?? app.data.slug} onChange={(event) => setSlug(event.target.value)} /></Field>
      <Field label="说明" className="form-span"><TextArea rows={3} value={description ?? app.data.description} onChange={(event) => setDescription(event.target.value)} /></Field>
      {update.error ? <div className="form-span"><ApiErrorNotice error={toNotice(update.error)} /></div> : null}
      <div className="form-actions form-span"><Button type="button" onClick={() => { setEditing(false); setName(null); setSlug(null); setDescription(null); }}>丢弃草稿</Button><Button tone="primary" disabled={update.isPending}>保存</Button></div>
    </form> : null}
    <ApplicationSourceSection applicationId={id} isAdministrator={isAdministrator} applicationActive={app.data.status === "active"} />
    <ApplicationEnvSection applicationId={id} isAdministrator={isAdministrator} />
    <section className="detail-section"><div className="section-heading"><div><h3>部署目标</h3><p>应用部署会一次性固化并发布到全部启用目标。</p></div><div className="section-actions">{app.data.status === "active" && targets.items.some((target) => target.status === "active") ? <Link className="button button--primary" to={`/deployments/new?application=${id}`}><Play aria-hidden="true" />部署应用</Link> : null}{isAdministrator && app.data.status === "active" ? <Button onClick={() => setAddingTarget(true)}><Plus aria-hidden="true" />添加目标</Button> : null}</div></div>
      {addingTarget ? <TargetEditor applicationId={id} nodes={nodes.items} hasMoreNodes={nodes.hasNextPage} loadingMoreNodes={nodes.isFetchingNextPage} onLoadMoreNodes={() => void nodes.fetchNextPage()} onDiscard={() => setAddingTarget(false)} onSaved={() => setAddingTarget(false)} /> : targets.isLoading ? <PageState kind="loading" /> : targets.isError ? <ApiErrorNotice error={toNotice(targets.error)} /> : targets.items.length === 0 ? <PageState kind="empty" /> : <><ul className="resource-list">{targets.items.map((target) => <li key={target.id}><div><strong><code>{target.nodeId}</code></strong><code>{target.scriptPath}</code></div><span className={`status-badge status-badge--${target.status === "active" ? "online" : "disabled"}`}>{target.status === "active" ? "启用" : "停用"}</span><span className="resource-actions"><Link className="text-link" to={`/apps/${id}/targets/${target.id}`}>{isAdministrator ? "配置" : "查看"}</Link></span></li>)}</ul>{targets.hasNextPage ? <div className="pagination-actions"><Button onClick={() => void targets.fetchNextPage()}>加载更多</Button></div> : null}</>}
    </section>
  </section>;
}
