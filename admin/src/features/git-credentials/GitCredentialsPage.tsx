import { useMutation, useQueryClient } from "@tanstack/react-query";
import { Archive, KeyRound, Plus, RotateCcw } from "lucide-react";
import { useState, type FormEvent } from "react";
import { Button } from "../../components/Button";
import { Field, TextInput } from "../../components/form";
import { PageState } from "../../components/PageState";
import { useAuth } from "../auth/AuthContext";
import { ApiErrorNotice } from "../errors/ApiErrorNotice";
import { ClipboardFallback } from "../shared/ClipboardFallback";
import { toNotice } from "../shared/toNotice";
import { useCursorCollection } from "../shared/useCursorCollection";
import { gitCredentialsApi } from "./api";

export function GitCredentialsPage() {
  const auth = useAuth();
  const queryClient = useQueryClient();
  const [creating, setCreating] = useState(false);
  const [name, setName] = useState("");
  const credentials = useCursorCollection(["git-credentials"], (after) => gitCredentialsApi.gitCredentialsList({ limit: 50, after: after ?? undefined }));
  const create = useMutation({ mutationFn: async () => {
    if (!auth.csrfToken) throw new Error("缺少 CSRF token");
    return gitCredentialsApi.gitCredentialsCreate({ xCSRFToken: auth.csrfToken, createGitCredentialRequest: { name: name.trim() } });
  }, onSuccess: async () => { setName(""); setCreating(false); await queryClient.invalidateQueries({ queryKey: ["git-credentials"] }); } });
  const toggleStatus = useMutation({ mutationFn: async (credential: { id: string; status: string; version: number }) => {
    if (!auth.csrfToken) throw new Error("缺少 CSRF token");
    return gitCredentialsApi.gitCredentialsUpdateStatus({ id: credential.id, xCSRFToken: auth.csrfToken, gitCredentialStatusRequest: { status: credential.status === "active" ? "archived" : "active", version: credential.version } });
  }, onSuccess: async () => { await queryClient.invalidateQueries({ queryKey: ["git-credentials"] }); } });

  async function submit(event: FormEvent) {
    event.preventDefault();
    await create.mutateAsync().catch(() => undefined);
  }

  function changeStatus(credential: { id: string; status: string; version: number }) {
    if (credential.status === "active" && !window.confirm("归档后应用来源将不能继续使用该凭证，确定继续吗？")) return;
    toggleStatus.mutate(credential);
  }

  return <section className="workspace">
    <div className="workspace-heading"><div><h2>Git 凭证</h2><p>生成命名 SSH deploy key，公钥可复制到 Git 托管平台；私钥只保存在服务端加密存储。</p></div><Button tone="primary" onClick={() => setCreating(true)}><Plus aria-hidden="true" />创建凭证</Button></div>
    {creating ? <form className="inline-form" onSubmit={(event) => void submit(event)}>
      <Field label="凭证名称"><TextInput autoFocus required minLength={1} maxLength={80} disabled={create.isPending} value={name} onChange={(event) => setName(event.target.value)} placeholder="例如：voucher-hub 只读 key" /></Field>
      {create.error ? <ApiErrorNotice error={toNotice(create.error)} /> : null}
      <div className="form-actions"><Button type="button" disabled={create.isPending} onClick={() => { setCreating(false); setName(""); }}>取消</Button><Button tone="primary" disabled={create.isPending || !name.trim()}>{create.isPending ? "正在创建..." : "生成公钥"}</Button></div>
    </form> : null}
    {credentials.isLoading ? <PageState kind="loading" /> : credentials.isError ? <div className="state-with-action"><ApiErrorNotice error={toNotice(credentials.error)} /><Button onClick={() => void credentials.refetch()}>重试</Button></div> : credentials.items.length === 0 ? <PageState kind="empty" /> : <div className="data-table-wrap"><table className="data-table"><thead><tr><th>凭证</th><th>指纹</th><th>状态</th><th>公钥</th><th></th></tr></thead><tbody>{credentials.items.map((credential) => <tr key={credential.id}><td><KeyRound aria-hidden="true" /><strong>{credential.name}</strong><small>创建于 {new Date(credential.createdAt).toLocaleString("zh-CN")}</small></td><td><code>{credential.fingerprint}</code></td><td><span className={`status-badge status-badge--${credential.status === "active" ? "online" : "disabled"}`}>{credential.status === "active" ? "启用" : "已归档"}</span></td><td><ClipboardFallback value={credential.publicKey} label="复制公钥" /></td><td><Button disabled={toggleStatus.isPending} onClick={() => changeStatus(credential)}>{credential.status === "active" ? <Archive aria-hidden="true" /> : <RotateCcw aria-hidden="true" />}{credential.status === "active" ? "归档" : "恢复"}</Button></td></tr>)}</tbody></table></div>}
    {credentials.hasNextPage ? <div className="pagination-actions"><Button disabled={credentials.isFetchingNextPage} onClick={() => void credentials.fetchNextPage()}>{credentials.isFetchingNextPage ? "正在加载..." : "加载更多"}</Button></div> : null}
  </section>;
}
