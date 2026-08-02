import { useMutation, useQueryClient } from "@tanstack/react-query";
import { Box, Plus } from "lucide-react";
import { useState, type FormEvent } from "react";
import { Link } from "react-router-dom";
import type { SaveApplicationRequest } from "../../api/generated/models/SaveApplicationRequest";
import { Button } from "../../components/Button";
import { PageState } from "../../components/PageState";
import { useAuth } from "../auth/AuthContext";
import { toNotice } from "../credentials/CredentialsPage";
import { ApiErrorNotice } from "../errors/ApiErrorNotice";
import { applicationsApi } from "./api";
import { useCursorCollection } from "../shared/useCursorCollection";
import { useUnsavedChanges } from "../shared/useUnsavedChanges";

const emptyForm: SaveApplicationRequest = { name: "", slug: "", description: "" };

export function ApplicationsPage() {
  const auth = useAuth();
  const queryClient = useQueryClient();
  const isAdministrator = auth.user?.identity === "administrator";
  const [status, setStatus] = useState("");
  const [editing, setEditing] = useState(false);
  const [form, setForm] = useState(emptyForm);
  useUnsavedChanges(editing && (form.name !== "" || form.slug !== "" || form.description !== ""));
  const list = useCursorCollection(["applications", status], (after) => applicationsApi.applicationsList({ limit: 20, after: after ?? undefined, status: status || undefined }));
  const create = useMutation({ mutationFn: async () => {
    if (!auth.csrfToken) throw new Error("缺少 CSRF token");
    return applicationsApi.applicationsCreate({ xCSRFToken: auth.csrfToken, saveApplicationRequest: { ...form, name: form.name.trim(), slug: form.slug.trim(), description: form.description?.trim() } });
  }, onSuccess: async () => { setForm(emptyForm); setEditing(false); await queryClient.invalidateQueries({ queryKey: ["applications"] }); } });
  async function submit(event: FormEvent) { event.preventDefault(); if (!create.isPending) await create.mutateAsync().catch(() => undefined); }
  return <section className="workspace">
    <div className="workspace-heading"><div><h2>应用</h2><p>应用保存业务边界，部署逻辑继续由仓库内受审查脚本负责。</p></div>{isAdministrator ? <Button tone="primary" onClick={() => setEditing(true)}><Plus aria-hidden="true" />创建应用</Button> : null}</div>
    <div className="filter-bar"><label>状态<select value={status} onChange={(event) => setStatus(event.target.value)}><option value="">全部</option><option value="active">启用</option><option value="archived">已归档</option></select></label></div>
    {editing ? <form className="node-form" onSubmit={(event) => void submit(event)}>
      <label>应用名称<input required maxLength={120} value={form.name} onChange={(event) => setForm({ ...form, name: event.target.value })} /></label>
      <label>Slug<input required pattern="[a-z0-9][a-z0-9-]*" value={form.slug} onChange={(event) => setForm({ ...form, slug: event.target.value })} placeholder="voucher-hub" /></label>
      <label className="form-span">说明<textarea rows={3} value={form.description} onChange={(event) => setForm({ ...form, description: event.target.value })} /></label>
      <div className="form-actions form-span"><Button type="button" onClick={() => { setEditing(false); setForm(emptyForm); }}>丢弃草稿</Button><Button tone="primary" disabled={create.isPending}>保存应用</Button></div>
      {create.error ? <div className="form-span"><ApiErrorNotice error={toNotice(create.error)} /></div> : null}
    </form> : null}
    {list.isLoading ? <PageState kind="loading" /> : list.isError ? <div className="state-with-action"><ApiErrorNotice error={toNotice(list.error)} /><Button onClick={() => void list.refetch()}>重试</Button></div> : list.items.length === 0 ? <PageState kind="empty" /> : <><div className="data-table-wrap"><table className="data-table"><thead><tr><th>应用</th><th>Slug</th><th>状态</th><th>说明</th><th></th></tr></thead><tbody>{list.items.map((app) => <tr key={app.id}><td><Box aria-hidden="true" /><strong>{app.name}</strong></td><td><code>{app.slug}</code></td><td><span className={`status-badge status-badge--${app.status === "active" ? "online" : "disabled"}`}>{app.status === "active" ? "启用" : "已归档"}</span></td><td>{app.description || "-"}</td><td><Link className="text-link" to={`/apps/${app.id}`}>{isAdministrator ? "配置" : "查看"}</Link></td></tr>)}</tbody></table></div>{list.hasNextPage ? <div className="pagination-actions"><Button disabled={list.isFetchingNextPage} onClick={() => void list.fetchNextPage()}>{list.isFetchingNextPage ? "正在加载..." : "加载更多"}</Button></div> : null}</>}
  </section>;
}
