import { useMutation, useQueryClient } from "@tanstack/react-query";
import { Plus, UserRound } from "lucide-react";
import { useState, type FormEvent } from "react";
import { Link } from "react-router-dom";
import type { CreateUserRequest } from "../../api/generated/models/CreateUserRequest";
import { Button } from "../../components/Button";
import { PageState } from "../../components/PageState";
import { useAuth } from "../auth/AuthContext";
import { toNotice } from "../credentials/CredentialsPage";
import { ApiErrorNotice } from "../errors/ApiErrorNotice";
import { useCursorCollection } from "../shared/useCursorCollection";
import { useUnsavedChanges } from "../shared/useUnsavedChanges";
import { usersApi } from "./api";

const emptyForm: CreateUserRequest = { username: "", displayName: "", email: "", password: "" };

export function UsersPage() {
  const auth = useAuth();
  const queryClient = useQueryClient();
  const [creating, setCreating] = useState(false);
  const [form, setForm] = useState(emptyForm);
  const users = useCursorCollection(["users"], (after) => usersApi.usersList({ limit: 20, after: after ?? undefined }));
  const dirty = creating && Object.values(form).some(Boolean);
  useUnsavedChanges(dirty);
  const create = useMutation({ mutationFn: async () => {
    if (!auth.csrfToken) throw new Error("缺少 CSRF token");
    return usersApi.usersCreate({ xCSRFToken: auth.csrfToken, createUserRequest: { username: form.username.trim(), password: form.password, displayName: form.displayName?.trim() || undefined, email: form.email?.trim() || undefined } });
  }, onSuccess: async () => { setForm(emptyForm); setCreating(false); await queryClient.invalidateQueries({ queryKey: ["users"] }); } });
  async function submit(event: FormEvent) { event.preventDefault(); await create.mutateAsync().catch(() => undefined); }
  return <section className="workspace">
    <div className="workspace-heading"><div><h2>用户管理</h2><p>管理员分配普通用户账号，不提供注册或邀请入口。</p></div><Button tone="primary" onClick={() => setCreating(true)}><Plus aria-hidden="true" />创建用户</Button></div>
    {creating ? <form className="node-form" onSubmit={(event) => void submit(event)}>
      <label>用户名<input required minLength={3} maxLength={64} disabled={create.isPending} value={form.username} onChange={(event) => setForm({ ...form, username: event.target.value })} /></label>
      <label>显示名称<input maxLength={120} disabled={create.isPending} value={form.displayName ?? ""} onChange={(event) => setForm({ ...form, displayName: event.target.value })} /></label>
      <label>邮箱<input type="email" disabled={create.isPending} value={form.email ?? ""} onChange={(event) => setForm({ ...form, email: event.target.value })} /></label>
      <label>初始密码<input required type="password" minLength={12} autoComplete="new-password" disabled={create.isPending} value={form.password} onChange={(event) => setForm({ ...form, password: event.target.value })} /></label>
      <p className="form-help form-span">密码只在此处设置一次，请通过系统外安全渠道交付给用户。</p>
      {create.error ? <div className="form-span"><ApiErrorNotice error={toNotice(create.error)} /></div> : null}
      <div className="form-actions form-span"><Button type="button" disabled={create.isPending} onClick={() => { setCreating(false); setForm(emptyForm); }}>丢弃草稿</Button><Button tone="primary" disabled={create.isPending}>{create.isPending ? "正在创建..." : "创建用户"}</Button></div>
    </form> : null}
    {users.isLoading ? <PageState kind="loading" /> : users.isError ? <div className="state-with-action"><ApiErrorNotice error={toNotice(users.error)} /><Button onClick={() => void users.refetch()}>重试</Button></div> : users.items.length === 0 ? <PageState kind="empty" /> : <><div className="data-table-wrap"><table className="data-table"><thead><tr><th>用户</th><th>身份</th><th>邮箱</th><th>状态</th><th></th></tr></thead><tbody>{users.items.map((user) => <tr key={user.id}><td><UserRound aria-hidden="true" /><strong>{user.displayName}</strong><small>@{user.username}</small></td><td>{user.identity === "administrator" ? "管理员" : "普通用户"}</td><td>{user.email || "-"}</td><td><span className={`status-badge status-badge--${user.status === "active" ? "online" : "disabled"}`}>{user.status === "active" ? "启用" : "停用"}</span></td><td><Link className="text-link" to={`/settings/users/${user.id}`}>管理</Link></td></tr>)}</tbody></table></div>{users.hasNextPage ? <div className="pagination-actions"><Button disabled={users.isFetchingNextPage} onClick={() => void users.fetchNextPage()}>加载更多</Button></div> : null}</>}
  </section>;
}
