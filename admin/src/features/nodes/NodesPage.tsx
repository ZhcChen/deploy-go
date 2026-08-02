import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Plus, Server } from "lucide-react";
import { useState, type FormEvent } from "react";
import { Link } from "react-router-dom";
import type { SaveNodeRequest } from "../../api/generated/models/SaveNodeRequest";
import { Button } from "../../components/Button";
import { PageState } from "../../components/PageState";
import { useAuth } from "../auth/AuthContext";
import { nodesApi, sshCredentialsApi } from "../credentials/api";
import { toNotice } from "../credentials/CredentialsPage";
import { ApiErrorNotice } from "../errors/ApiErrorNotice";
import { useCursorCollection } from "../shared/useCursorCollection";

const initialNode: SaveNodeRequest = { name: "", host: "", port: 22, username: "deploy", workRoot: "/srv/apps", secretsRoot: "/srv/secrets" };

export function NodesPage() {
  const auth = useAuth();
  const queryClient = useQueryClient();
  const [showForm, setShowForm] = useState(false);
  const [form, setForm] = useState<SaveNodeRequest>(initialNode);
  const isAdministrator = auth.user?.identity === "administrator";
  const nodes = useCursorCollection(["nodes"], (after) => nodesApi.nodesList({ limit: 50, after: after ?? undefined }));
  const credentials = useQuery({ queryKey: ["ssh-credentials"], queryFn: () => sshCredentialsApi.sshCredentialsList(), enabled: isAdministrator });
  const create = useMutation({ mutationFn: async () => {
    if (!auth.csrfToken) throw new Error("缺少 CSRF token");
    return nodesApi.nodesCreate({ xCSRFToken: auth.csrfToken, saveNodeRequest: form });
  }, onSuccess: async () => { setForm(initialNode); setShowForm(false); await queryClient.invalidateQueries({ queryKey: ["nodes"] }); } });

  async function submit(event: FormEvent) { event.preventDefault(); await create.mutateAsync().catch(() => undefined); }
  return <section className="workspace">
    <div className="workspace-heading"><div><h2>节点</h2><p>{isAdministrator ? "配置部署账号与目录，然后显式核对 host key 并执行能力检查。" : "查看已授权应用关联的节点与健康状态。"}</p></div>{isAdministrator ? <Button tone="primary" onClick={() => setShowForm((value) => !value)}><Plus aria-hidden="true" />添加节点</Button> : null}</div>
    {isAdministrator && showForm ? <form className="node-form" onSubmit={(event) => void submit(event)}>
      <label>节点名称<input required value={form.name} onChange={(e) => setForm({ ...form, name: e.target.value })} /></label>
      <label>Host<input required value={form.host} onChange={(e) => setForm({ ...form, host: e.target.value })} placeholder="node.internal" /></label>
      <label>端口<input required type="number" min="1" max="65535" value={form.port} onChange={(e) => setForm({ ...form, port: Number(e.target.value) })} /></label>
      <label>部署账号<input required value={form.username} onChange={(e) => setForm({ ...form, username: e.target.value })} /></label>
      <label>工作根目录<input required value={form.workRoot} onChange={(e) => setForm({ ...form, workRoot: e.target.value })} /></label>
      <label>Secrets root<input required value={form.secretsRoot} onChange={(e) => setForm({ ...form, secretsRoot: e.target.value })} /></label>
      <label className="form-span">SSH 密钥<select value={form.sshCredentialId ?? ""} onChange={(e) => setForm({ ...form, sshCredentialId: e.target.value || undefined })}><option value="">稍后绑定</option>{credentials.data?.items.map((item) => <option key={item.id} value={item.id}>{item.name}</option>)}</select></label>
      <div className="form-actions form-span"><Button type="button" onClick={() => setShowForm(false)}>取消</Button><Button tone="primary" disabled={create.isPending}>{create.isPending ? "正在创建..." : "创建节点"}</Button></div>
      {create.error ? <div className="form-span"><ApiErrorNotice error={toNotice(create.error)} /></div> : null}
    </form> : null}
    {nodes.isLoading ? <PageState kind="loading" /> : nodes.isError ? <div className="state-with-action"><ApiErrorNotice error={toNotice(nodes.error)} /><Button onClick={() => void nodes.refetch()}>重试</Button></div> : nodes.items.length === 0 ? <PageState kind="empty" /> : <div className="data-table-wrap"><table className="data-table"><thead><tr><th>节点</th><th>连接地址</th><th>状态</th><th>SSH 密钥</th><th></th></tr></thead><tbody>{nodes.items.map((node) => <tr key={node.id}><td><Server aria-hidden="true" /><strong>{node.name}</strong></td><td><code>{node.username}@{node.host}:{node.port}</code></td><td><span className={`status-badge status-badge--${node.status}`}>{statusLabel(node.status)}</span></td><td>{node.sshCredentialId ? "已绑定" : "未绑定"}</td><td><Link className="text-link" to={`/nodes/${node.id}`}>{isAdministrator ? "接入管理" : "查看"}</Link></td></tr>)}</tbody></table>{nodes.hasNextPage ? <div className="pagination-actions"><Button onClick={() => void nodes.fetchNextPage()}>加载更多</Button></div> : null}</div>}
  </section>;
}

export function statusLabel(status: string) { return ({ online: "在线", offline: "检查失败", unchecked: "待检查", checking: "检查中", missing_credential: "缺少密钥", disabled: "已停用" } as Record<string, string>)[status] ?? status; }
