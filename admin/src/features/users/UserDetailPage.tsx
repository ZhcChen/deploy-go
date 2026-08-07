import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { KeyRound, Power } from "lucide-react";
import { useState, type FormEvent } from "react";
import { Link, useParams } from "react-router-dom";
import { Button } from "../../components/Button";
import { BackLink } from "../../components/BackLink";
import { Field, TextInput } from "../../components/form";
import { PageState } from "../../components/PageState";
import { useAuth } from "../auth/AuthContext";
import { toNotice } from "../shared/toNotice";
import { ApiErrorNotice } from "../errors/ApiErrorNotice";
import { useUnsavedChanges } from "../shared/useUnsavedChanges";
import { usersApi } from "./api";

export function UserDetailPage() {
  const { id = "" } = useParams();
  const auth = useAuth();
  const queryClient = useQueryClient();
  const [resetting, setResetting] = useState(false);
  const [password, setPassword] = useState("");
  useUnsavedChanges(resetting && password !== "");
  const user = useQuery({ queryKey: ["user", id], queryFn: () => usersApi.usersShow({ id }) });
  const reset = useMutation({ mutationFn: async () => {
    if (!auth.csrfToken || !user.data) throw new Error("缺少必要的安全上下文");
    await usersApi.usersResetPassword({ id, xCSRFToken: auth.csrfToken, resetPasswordRequest: { password, version: user.data.version } });
  }, onSuccess: async () => { setPassword(""); setResetting(false); await queryClient.invalidateQueries({ queryKey: ["user", id] }); } });
  const status = useMutation({ mutationFn: async () => {
    if (!auth.csrfToken || !user.data) throw new Error("缺少必要的安全上下文");
    return usersApi.usersUpdateStatus({ id, xCSRFToken: auth.csrfToken, updateStatusRequest: { status: user.data.status === "active" ? "disabled" : "active", version: user.data.version } });
  }, onSuccess: (saved) => { queryClient.setQueryData(["user", id], saved); void queryClient.invalidateQueries({ queryKey: ["users"] }); } });
  async function submit(event: FormEvent) { event.preventDefault(); await reset.mutateAsync().catch(() => undefined); }
  function changeStatus() {
    if (user.data?.status === "active" && !window.confirm("停用后该用户的现有会话会被撤销，确定继续吗？")) return;
    status.mutate();
  }
  if (user.isLoading) return <PageState kind="loading" />;
  if (user.isError || !user.data) return <div className="state-with-action"><ApiErrorNotice error={toNotice(user.error)} /><Link className="button" to="/settings/users">返回用户</Link></div>;
  const isAdministrator = user.data.identity === "administrator";
  return <section className="workspace detail-page">
    <BackLink to="/settings/users" parentLabel="用户列表" />
    <div className="detail-title"><div><h2>{user.data.displayName}</h2><p>@{user.data.username} · {user.data.email || "未设置邮箱"}</p></div><span className={`status-badge status-badge--${user.data.status === "active" ? "online" : "disabled"}`}>{user.data.status === "active" ? "启用" : "停用"}</span></div>
    <div className="detail-toolbar"><Button disabled={isAdministrator} onClick={() => setResetting(true)}><KeyRound aria-hidden="true" />重置密码</Button><Button tone={user.data.status === "active" ? "danger" : "default"} disabled={isAdministrator || status.isPending} onClick={changeStatus}><Power aria-hidden="true" />{user.data.status === "active" ? "停用用户" : "启用用户"}</Button></div>
    {isAdministrator ? <p className="notice">唯一管理员账号不能停用或由此页面重置密码。</p> : null}
    {status.error ? <ApiErrorNotice error={toNotice(status.error)} /> : null}
    {resetting ? <form className="inline-form" onSubmit={(event) => void submit(event)}>
      <Field label="新密码" hint="重置成功后，该用户全部既有会话会立即失效；新密码请通过系统外安全渠道交付。"><TextInput required type="password" minLength={12} autoComplete="new-password" disabled={reset.isPending} value={password} onChange={(event) => setPassword(event.target.value)} /></Field>
      {reset.error ? <ApiErrorNotice error={toNotice(reset.error)} /> : null}
      <div className="form-actions"><Button type="button" disabled={reset.isPending} onClick={() => { setResetting(false); setPassword(""); }}>丢弃草稿</Button><Button tone="primary" disabled={reset.isPending}>确认重置</Button></div>
    </form> : null}
  </section>;
}
