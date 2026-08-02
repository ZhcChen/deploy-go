import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { ArrowLeft, CheckCircle2, KeyRound, Radar, Unlink } from "lucide-react";
import { useState } from "react";
import { Link, useParams } from "react-router-dom";
import type { HostKeyScanResponse } from "../../api/generated/models/HostKeyScanResponse";
import type { NodeCheckResponse } from "../../api/generated/models/NodeCheckResponse";
import type { NodeResponse } from "../../api/generated/models/NodeResponse";
import { Button } from "../../components/Button";
import { PageState } from "../../components/PageState";
import { useAuth } from "../auth/AuthContext";
import { nodesApi, sshCredentialsApi } from "../credentials/api";
import { toNotice } from "../credentials/CredentialsPage";
import { ApiErrorNotice } from "../errors/ApiErrorNotice";
import { statusLabel } from "./NodesPage";

export function NodeDetailPage() {
  const { id = "" } = useParams();
  const auth = useAuth();
  const isAdministrator = auth.user?.identity === "administrator";
  const queryClient = useQueryClient();
  const detail = useQuery({ queryKey: ["node", id], queryFn: () => nodesApi.nodesShow({ id }) });
  const credentials = useQuery({ queryKey: ["ssh-credentials"], queryFn: () => sshCredentialsApi.sshCredentialsList(), enabled: isAdministrator });
  const [credentialId, setCredentialId] = useState<string | null>(null);
  const [scan, setScan] = useState<HostKeyScanResponse | null>(null);
  const [check, setCheck] = useState<NodeCheckResponse | null>(null);

  function secureContext() { if (!auth.csrfToken) throw new Error("缺少 CSRF token"); return auth.csrfToken; }
  function replaceNode(node: NodeResponse) { queryClient.setQueryData(["node", id], node); void queryClient.invalidateQueries({ queryKey: ["nodes"] }); }
  const bind = useMutation({ mutationFn: () => nodesApi.nodesBindCredential({ id, xCSRFToken: secureContext(), bindCredentialRequest: { credentialId: credentialId!, version: detail.data!.version } }), onSuccess: (node) => { replaceNode(node); setCredentialId(node.sshCredentialId ?? null); setScan(null); setCheck(null); } });
  const unbind = useMutation({ mutationFn: () => nodesApi.nodesUnbindCredential({ id, xCSRFToken: secureContext(), versionRequest: { version: detail.data!.version } }), onSuccess: (node) => { replaceNode(node); setScan(null); setCheck(null); } });
  const scanHost = useMutation({ mutationFn: () => nodesApi.nodesScanHostKey({ id, xCSRFToken: secureContext() }), onSuccess: (result) => { setScan(result); setCheck(null); } });
  const confirm = useMutation({ mutationFn: () => nodesApi.nodesConfirmHostKey({ id, xCSRFToken: secureContext(), confirmHostKeyRequest: { checkId: scan!.checkId, snapshotHash: scan!.snapshotHash, version: detail.data!.version } }), onSuccess: (node) => { replaceNode(node); setScan(null); setCheck(null); } });
  const runCheck = useMutation({ mutationFn: () => nodesApi.nodesRunCheck({ id, xCSRFToken: secureContext() }), onSuccess: (result) => { setCheck(result); void detail.refetch(); } });
  const pending = bind.isPending || unbind.isPending || scanHost.isPending || confirm.isPending || runCheck.isPending;
  const operationError = bind.error || unbind.error || scanHost.error || confirm.error || runCheck.error;
  if (detail.isLoading) return <PageState kind="loading" />;
  if (detail.isError || !detail.data) return <div className="state-with-action"><ApiErrorNotice error={toNotice(detail.error)} /><Link className="button button--default" to="/nodes">返回节点</Link></div>;
  const node = detail.data;
  const selectedCredentialId = credentialId ?? node.sshCredentialId ?? "";
  const confirmed = Boolean(node.trustedHostFingerprint) && !scan;
  return <section className="workspace detail-page">
    <Link className="back-link" to="/nodes"><ArrowLeft aria-hidden="true" />返回节点</Link>
    <div className="detail-title"><div><h2>{node.name}</h2><p><code>{node.username}@{node.host}:{node.port}</code></p></div><span className={`status-badge status-badge--${node.status}`}>{statusLabel(node.status)}</span></div>
    <dl className="definition-grid"><div><dt>工作根目录</dt><dd><code>{node.workRoot}</code></dd></div><div><dt>Secrets root</dt><dd><code>{node.secretsRoot}</code></dd></div></dl>
    {isAdministrator ? <ol className="onboarding-steps">
      <li className={node.sshCredentialId ? "is-complete" : "is-current"}><span>1</span><div><h3>绑定 SSH 密钥</h3><p>选择服务端保管的密钥。更换或解绑后必须重新检查。</p><div className="inline-row"><select aria-label="SSH 密钥" value={selectedCredentialId} onChange={(e) => setCredentialId(e.target.value)}><option value="">选择密钥</option>{credentials.data?.items.map((item) => <option key={item.id} value={item.id}>{item.name}</option>)}</select><Button disabled={!selectedCredentialId || selectedCredentialId === node.sshCredentialId || pending} onClick={() => bind.mutate()}><KeyRound aria-hidden="true" />{node.sshCredentialId ? "更换" : "绑定"}</Button>{node.sshCredentialId ? <Button disabled={pending} onClick={() => unbind.mutate()}><Unlink aria-hidden="true" />解绑</Button> : null}</div></div></li>
      <li className={confirmed ? "is-complete" : node.sshCredentialId ? "is-current" : ""}><span>2</span><div><h3>扫描 host key</h3><p>扫描只读取待核对指纹，不会自动建立信任。</p><Button disabled={!node.sshCredentialId || pending} onClick={() => scanHost.mutate()}><Radar aria-hidden="true" />{scanHost.isPending ? "正在扫描..." : "扫描指纹"}</Button>{scan ? <div className="fingerprint-review"><strong>待核对的 Ed25519 指纹</strong><code>{scan.fingerprint}</code><p>请通过云控制台或节点管理员提供的独立渠道核对。</p></div> : null}</div></li>
      <li className={confirmed ? "is-complete" : scan ? "is-current" : ""}><span>3</span><div><h3>确认 host key</h3>{confirmed && !scan ? <p className="success-text"><CheckCircle2 aria-hidden="true" />已信任 <code>{node.trustedHostFingerprint}</code></p> : <p>只有人工核对扫描指纹后，才能执行确认。</p>}{scan ? <Button tone="primary" disabled={pending} onClick={() => confirm.mutate()}>确认指纹一致</Button> : null}</div></li>
      <li className={node.status === "online" ? "is-complete" : confirmed ? "is-current" : ""}><span>4</span><div><h3>节点能力检查</h3><p>验证认证、系统信息、工作目录与可用磁盘；不会执行部署脚本。</p><Button disabled={!node.sshCredentialId || !confirmed || pending} onClick={() => runCheck.mutate()}>{runCheck.isPending ? "正在检查..." : "执行检查"}</Button>{check ? <CheckResult value={check} /> : null}</div></li>
    </ol> : <section className="detail-section"><h3>节点状态</h3><p>普通用户可查看已授权节点状态；接入配置由管理员维护。</p>{node.trustedHostFingerprint ? <p className="success-text"><CheckCircle2 aria-hidden="true" />host key 已确认</p> : <p className="muted">host key 尚未确认</p>}</section>}
    {operationError ? <ApiErrorNotice error={toNotice(operationError)} /> : null}
  </section>;
}

function CheckResult({ value }: { value: NodeCheckResponse }) {
  if (value.status !== "succeeded") return <div className="check-result check-result--failed" role="alert"><strong>检查失败：{value.failureCode ?? "unknown"}</strong><p>{value.failureMessage ?? "请核对节点配置后重试。"}</p></div>;
  return <dl className="check-result"><div><dt>系统</dt><dd>{value.osName ?? "-"}</dd></div><div><dt>架构</dt><dd>{value.architecture ?? "-"}</dd></div><div><dt>可用磁盘</dt><dd>{formatBytes(value.diskAvailableBytes)}</dd></div></dl>;
}
function formatBytes(value?: number | null) { if (value == null) return "-"; return `${(value / 1024 / 1024 / 1024).toFixed(1)} GiB`; }
