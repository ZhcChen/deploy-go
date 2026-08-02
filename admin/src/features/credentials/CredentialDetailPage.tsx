import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { ArrowLeft, Trash2 } from "lucide-react";
import { useState, type FormEvent } from "react";
import { Link, useNavigate, useParams } from "react-router-dom";
import { Button } from "../../components/Button";
import { PageState } from "../../components/PageState";
import { useAuth } from "../auth/AuthContext";
import { ApiErrorNotice } from "../errors/ApiErrorNotice";
import { nodesApi, sshCredentialsApi } from "./api";
import { ClipboardFallback } from "./ClipboardFallback";
import { toNotice } from "./CredentialsPage";

export function CredentialDetailPage() {
  const { id = "" } = useParams();
  const auth = useAuth();
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const detail = useQuery({ queryKey: ["ssh-credential", id], queryFn: () => sshCredentialsApi.sshCredentialsShow({ id }) });
  const nodes = useQuery({ queryKey: ["nodes", "credential-bindings"], queryFn: () => nodesApi.nodesList() });
  const [name, setName] = useState<string | null>(null);
  const rename = useMutation({ mutationFn: async () => {
    if (!auth.csrfToken || !detail.data) throw new Error("缺少必要的安全上下文");
    return sshCredentialsApi.sshCredentialsRename({ id, xCSRFToken: auth.csrfToken, renameCredentialRequest: { name: (name ?? detail.data.name).trim(), version: detail.data.version } });
  }, onSuccess: async (updated) => { setName(updated.name); await Promise.all([queryClient.invalidateQueries({ queryKey: ["ssh-credential", id] }), queryClient.invalidateQueries({ queryKey: ["ssh-credentials"] })]); } });
  const remove = useMutation({ mutationFn: async () => {
    if (!auth.csrfToken) throw new Error("缺少 CSRF token");
    const latestNodes = await nodesApi.nodesList();
    if (latestNodes.items.some((node) => node.sshCredentialId === id)) throw new Error("该密钥仍绑定节点，请先解绑后再删除");
    await sshCredentialsApi.sshCredentialsDeleteCredential({ id, xCSRFToken: auth.csrfToken });
  }, onSuccess: async () => { await queryClient.invalidateQueries({ queryKey: ["ssh-credentials"] }); navigate("/settings/credentials", { replace: true }); } });
  const bindings = nodes.data?.items.filter((node) => node.sshCredentialId === id) ?? [];

  async function submit(event: FormEvent) { event.preventDefault(); await rename.mutateAsync().catch(() => undefined); }
  async function deleteCredential() {
    if (bindings.length || !window.confirm("确定永久删除这个 SSH 密钥吗？")) return;
    await remove.mutateAsync().catch(() => undefined);
  }
  if (detail.isLoading) return <PageState kind="loading" />;
  if (detail.isError || !detail.data) return <div className="state-with-action"><ApiErrorNotice error={toNotice(detail.error)} /><Link className="button button--default" to="/settings/credentials">返回列表</Link></div>;
  return <section className="workspace detail-page">
    <Link className="back-link" to="/settings/credentials"><ArrowLeft aria-hidden="true" />返回 SSH 密钥</Link>
    <div className="detail-title"><div><h2>{detail.data.name}</h2><p><code>{detail.data.fingerprint}</code></p></div><span className="status-badge">{detail.data.algorithm}</span></div>
    <section className="detail-section"><h3>公钥</h3><p>把这段公钥添加到目标服务器部署账号的 <code>authorized_keys</code>。</p><ClipboardFallback value={detail.data.publicKey} /></section>
    <section className="detail-section"><h3>绑定节点</h3>{nodes.isLoading ? <p>正在读取绑定状态...</p> : nodes.isError ? <ApiErrorNotice error={toNotice(nodes.error)} /> : bindings.length ? <ul className="compact-list">{bindings.map((node) => <li key={node.id}><Link to={`/nodes/${node.id}`}>{node.name}</Link><code>{node.host}:{node.port}</code></li>)}</ul> : <p className="muted">当前没有节点使用这把密钥。</p>}</section>
    <section className="detail-section"><h3>重命名</h3><form className="inline-row" onSubmit={(event) => void submit(event)}><input required value={name ?? detail.data.name} onChange={(event) => setName(event.target.value)} /><Button disabled={!(name ?? detail.data.name).trim() || rename.isPending}>保存名称</Button></form>{rename.error ? <ApiErrorNotice error={toNotice(rename.error)} /> : null}</section>
    <section className="danger-zone"><div><h3>删除密钥</h3><p>{bindings.length ? `仍绑定 ${bindings.length} 个节点，当前禁止删除。` : "删除后无法恢复。平台不会再次生成相同密钥。"}</p></div><Button tone="danger" disabled={Boolean(bindings.length) || remove.isPending} onClick={() => void deleteCredential()}><Trash2 aria-hidden="true" />删除</Button>{remove.error ? <ApiErrorNotice error={toNotice(remove.error)} /> : null}</section>
  </section>;
}
